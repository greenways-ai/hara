use super::super::{Dispatch, Machine, VmOutcome, VmSlot};
use super::{
    InstructionEvent, Opcode, TerminalEvent, TerminalKind, TransitionEvent, TransitionKind, VmProbe,
};
use crate::core::{PromiseState, Value};
use crate::vm::error::VmError;
use crate::vm::opcode::Instruction;

impl Machine {
    /// Runs through the same production dispatch implementation while emitting
    /// compact scalar events. This path deliberately does not execute tracing
    /// JIT recordings; JIT internals have a separate telemetry contract.
    pub fn run_instrumented<P: VmProbe>(&mut self, probe: &mut P) -> VmOutcome {
        let program = self.program.clone();
        self.clear_instrumented_jit_state();
        loop {
            let Some(function) = program.functions.get(self.function) else {
                let error = VmError::new("function index out of range", 0, None);
                probe.on_terminal(self.terminal_event(TerminalKind::Fail));
                return VmOutcome::Failed(error);
            };
            let Some(instruction) = function.code.get(self.ip) else {
                let error = self.error(function, "instruction pointer out of range");
                probe.on_terminal(self.terminal_event(TerminalKind::Fail));
                return VmOutcome::Failed(error);
            };
            probe.on_instruction(self.instruction_event(instruction));
            let from_function = self.function;
            let from_ip = self.ip;
            match self.dispatch(&program, function, instruction) {
                Dispatch::Next(ip) => self.ip = ip,
                Dispatch::Unwound(ip) => {
                    self.ip = ip;
                    probe.on_transition(self.transition_event(
                        TransitionKind::ExceptionUnwind,
                        from_function,
                        from_ip,
                    ));
                }
                Dispatch::Call { callee, args } => {
                    if let Err(message) = self.enter_callable(&program, callee, args) {
                        match self.raise(function, message) {
                            Ok(target) => {
                                self.ip = target;
                                probe.on_transition(self.transition_event(
                                    TransitionKind::ExceptionUnwind,
                                    from_function,
                                    from_ip,
                                ));
                            }
                            Err(error) => {
                                probe.on_terminal(self.terminal_event(TerminalKind::Fail));
                                return VmOutcome::Failed(error);
                            }
                        }
                    } else {
                        probe.on_transition(self.transition_event(
                            TransitionKind::CallEnter,
                            from_function,
                            from_ip,
                        ));
                    }
                }
                Dispatch::CallStatic {
                    prototype,
                    args,
                    captures,
                } => {
                    self.enter_or_spawn(&program, prototype, args, captures);
                    probe.on_transition(self.transition_event(
                        TransitionKind::CallEnter,
                        from_function,
                        from_ip,
                    ));
                }
                Dispatch::CallStaticDirect { prototype, argc } => {
                    self.enter_static_direct(&program, prototype, argc);
                    probe.on_transition(self.transition_event(
                        TransitionKind::CallEnter,
                        from_function,
                        from_ip,
                    ));
                }
                Dispatch::Returned(value) => {
                    self.stack.truncate(self.frame.base());
                    if let Some(caller) = self.calls.pop() {
                        self.function = caller.function;
                        let completed = std::mem::replace(&mut self.frame, caller.frame);
                        self.free_locals.push(completed.into_locals());
                        self.ip = caller.call_ip + 1;
                        self.stack.push(value);
                        probe.on_transition(self.transition_event(
                            TransitionKind::CallReturn,
                            from_function,
                            from_ip,
                        ));
                    } else {
                        probe.on_terminal(self.terminal_event(TerminalKind::Return));
                        return VmOutcome::Returned(Self::into_value(program.clone(), value));
                    }
                }
                Dispatch::Suspended(promise) => {
                    probe.on_transition(self.transition_event(
                        TransitionKind::MachineSuspend,
                        from_function,
                        from_ip,
                    ));
                    return VmOutcome::Suspended(promise);
                }
                Dispatch::Failed(error) => {
                    probe.on_terminal(self.terminal_event(TerminalKind::Fail));
                    return VmOutcome::Failed(error);
                }
            }
        }
    }

    /// Applies one settlement to a machine stopped at `Await`, emits the
    /// resume or unwind boundary, and continues through `run_instrumented`.
    pub fn resume_instrumented<P: VmProbe>(
        &mut self,
        state: PromiseState,
        probe: &mut P,
    ) -> VmOutcome {
        let from_function = self.function;
        let from_ip = self.ip;
        let Some(function) = self.program.functions.get(self.function).cloned() else {
            let error = VmError::new("function index out of range", 0, None);
            probe.on_terminal(self.terminal_event(TerminalKind::Fail));
            return VmOutcome::Failed(error);
        };
        if !matches!(function.code.get(self.ip), Some(Instruction::Await)) {
            let error = self.error(&function, "VM is not suspended at await");
            probe.on_terminal(self.terminal_event(TerminalKind::Fail));
            return VmOutcome::Failed(error);
        }
        match state {
            PromiseState::Pending => {
                let promise = match self.stack.last().and_then(VmSlot::runtime_value) {
                    Some(Value::Promise(promise)) => promise,
                    _ => {
                        let error = self.error(&function, "await expects a promise");
                        probe.on_terminal(self.terminal_event(TerminalKind::Fail));
                        return VmOutcome::Failed(error);
                    }
                };
                probe.on_transition(self.transition_event(
                    TransitionKind::MachineSuspend,
                    from_function,
                    from_ip,
                ));
                return VmOutcome::Suspended(promise);
            }
            PromiseState::Fulfilled(value) => {
                self.stack.pop();
                self.stack.push(value.into());
                self.ip += 1;
                probe.on_transition(self.transition_event(
                    TransitionKind::MachineResume,
                    from_function,
                    from_ip,
                ));
            }
            PromiseState::Rejected(error) => {
                self.stack.pop();
                match self.raise(&function, crate::core::promise_rejection_error(error)) {
                    Ok(target) => {
                        self.ip = target;
                        probe.on_transition(self.transition_event(
                            TransitionKind::ExceptionUnwind,
                            from_function,
                            from_ip,
                        ));
                    }
                    Err(error) => {
                        probe.on_terminal(self.terminal_event(TerminalKind::Fail));
                        return VmOutcome::Failed(error);
                    }
                }
            }
        }
        self.run_instrumented(probe)
    }

    #[inline(always)]
    fn instruction_event(&self, instruction: &Instruction) -> InstructionEvent {
        InstructionEvent {
            function: saturating_u16(self.function),
            ip: saturating_u32(self.ip),
            opcode: Opcode::from_instruction(instruction),
            stack_depth: saturating_u32(self.stack.len()),
            call_depth: saturating_u16(self.calls.len()),
        }
    }

    #[inline(always)]
    fn transition_event(
        &self,
        kind: TransitionKind,
        from_function: usize,
        from_ip: usize,
    ) -> TransitionEvent {
        TransitionEvent {
            kind,
            from_function: saturating_u16(from_function),
            from_ip: saturating_u32(from_ip),
            to_function: saturating_u16(self.function),
            to_ip: saturating_u32(self.ip),
            stack_depth: saturating_u32(self.stack.len()),
            call_depth: saturating_u16(self.calls.len()),
        }
    }

    #[inline(always)]
    fn terminal_event(&self, kind: TerminalKind) -> TerminalEvent {
        TerminalEvent {
            kind,
            function: saturating_u16(self.function),
            ip: saturating_u32(self.ip),
            stack_depth: saturating_u32(self.stack.len()),
            call_depth: saturating_u16(self.calls.len()),
        }
    }

    #[cfg(feature = "tracing-jit")]
    fn clear_instrumented_jit_state(&mut self) {
        self.jit_path.clear();
        self.jit_loop_entries.clear();
        self.jit_suppressed_range = None;
    }

    #[cfg(not(feature = "tracing-jit"))]
    fn clear_instrumented_jit_state(&mut self) {}
}

#[inline(always)]
fn saturating_u16(value: usize) -> u16 {
    value.min(u16::MAX as usize) as u16
}

#[inline(always)]
fn saturating_u32(value: usize) -> u32 {
    value.min(u32::MAX as usize) as u32
}
