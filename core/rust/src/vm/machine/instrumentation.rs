//! Low-overhead, opt-in instrumentation for the production bytecode machine.
//!
//! Unlike `machine::observation`, this module does not project values, locals,
//! handlers, or source strings on every instruction. It emits copy-only scalar
//! events through a monomorphized probe and keeps the ordinary `Machine::run`
//! path unchanged.

use super::{Dispatch, Machine, VmOutcome, VmSlot};
use crate::core::{PromiseState, Value};
use crate::vm::error::VmError;
use crate::vm::opcode::Instruction;

pub const BYTECODE_METRICS_SCHEMA: &str = "hal.bytecode-metrics/v1";
pub const BYTECODE_EVENTS_SCHEMA: &str = "hal.bytecode-events/v1";

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Opcode {
    Constant,
    Nil,
    True,
    False,
    LoadLocal,
    StoreLocal,
    Pop,
    Dup,
    Primitive,
    PrimitiveLocalConst,
    Jump,
    JumpIfFalse,
    Closure,
    Call,
    CallStatic,
    Throw,
    Rethrow,
    GetGlobal,
    DefGlobal,
    SetGlobal,
    VarGlobal,
    DeclareGlobal,
    DefStruct,
    StructField,
    InstanceOf,
    MakeMultiArity,
    BuildVector,
    BuildMap,
    BuildSet,
    BuildList,
    ConcatList,
    ToVector,
    DefMacro,
    Await,
    HostCall,
    Return,
}

impl Opcode {
    pub const COUNT: usize = 36;

    pub const fn index(self) -> usize {
        self as usize
    }

    pub const fn as_keyword(self) -> &'static str {
        match self {
            Self::Constant => "constant",
            Self::Nil => "nil",
            Self::True => "true",
            Self::False => "false",
            Self::LoadLocal => "load-local",
            Self::StoreLocal => "store-local",
            Self::Pop => "pop",
            Self::Dup => "dup",
            Self::Primitive => "primitive",
            Self::PrimitiveLocalConst => "primitive-local-const",
            Self::Jump => "jump",
            Self::JumpIfFalse => "jump-if-false",
            Self::Closure => "closure",
            Self::Call => "call",
            Self::CallStatic => "call-static",
            Self::Throw => "throw",
            Self::Rethrow => "rethrow",
            Self::GetGlobal => "get-global",
            Self::DefGlobal => "def-global",
            Self::SetGlobal => "set-global",
            Self::VarGlobal => "var-global",
            Self::DeclareGlobal => "declare-global",
            Self::DefStruct => "def-struct",
            Self::StructField => "struct-field",
            Self::InstanceOf => "instance-of",
            Self::MakeMultiArity => "make-multi-arity",
            Self::BuildVector => "build-vector",
            Self::BuildMap => "build-map",
            Self::BuildSet => "build-set",
            Self::BuildList => "build-list",
            Self::ConcatList => "concat-list",
            Self::ToVector => "to-vector",
            Self::DefMacro => "def-macro",
            Self::Await => "await",
            Self::HostCall => "host-call",
            Self::Return => "return",
        }
    }

    fn from_instruction(instruction: &Instruction) -> Self {
        match instruction {
            Instruction::Constant(_) => Self::Constant,
            Instruction::Nil => Self::Nil,
            Instruction::True => Self::True,
            Instruction::False => Self::False,
            Instruction::LoadLocal(_) => Self::LoadLocal,
            Instruction::StoreLocal(_) => Self::StoreLocal,
            Instruction::Pop => Self::Pop,
            Instruction::Dup => Self::Dup,
            Instruction::Primitive { .. } => Self::Primitive,
            Instruction::PrimitiveLocalConst { .. } => Self::PrimitiveLocalConst,
            Instruction::Jump(_) => Self::Jump,
            Instruction::JumpIfFalse(_) => Self::JumpIfFalse,
            Instruction::Closure { .. } => Self::Closure,
            Instruction::Call { .. } => Self::Call,
            Instruction::CallStatic { .. } => Self::CallStatic,
            Instruction::Throw => Self::Throw,
            Instruction::Rethrow => Self::Rethrow,
            Instruction::GetGlobal(_) => Self::GetGlobal,
            Instruction::DefGlobal { .. } => Self::DefGlobal,
            Instruction::SetGlobal(_) => Self::SetGlobal,
            Instruction::VarGlobal(_) => Self::VarGlobal,
            Instruction::DeclareGlobal(_) => Self::DeclareGlobal,
            Instruction::DefStruct { .. } => Self::DefStruct,
            Instruction::StructField(_) => Self::StructField,
            Instruction::InstanceOf => Self::InstanceOf,
            Instruction::MakeMultiArity { .. } => Self::MakeMultiArity,
            Instruction::BuildVector(_) => Self::BuildVector,
            Instruction::BuildMap(_) => Self::BuildMap,
            Instruction::BuildSet(_) => Self::BuildSet,
            Instruction::BuildList(_) => Self::BuildList,
            Instruction::ConcatList(_) => Self::ConcatList,
            Instruction::ToVector => Self::ToVector,
            Instruction::DefMacro { .. } => Self::DefMacro,
            Instruction::Await => Self::Await,
            Instruction::HostCall => Self::HostCall,
            Instruction::Return => Self::Return,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InstructionEvent {
    pub function: u16,
    pub ip: u32,
    pub opcode: Opcode,
    pub stack_depth: u32,
    pub call_depth: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitionKind {
    CallEnter,
    CallReturn,
    ExceptionUnwind,
    MachineSuspend,
    MachineResume,
}

impl TransitionKind {
    pub const fn as_keyword(self) -> &'static str {
        match self {
            Self::CallEnter => "call/enter",
            Self::CallReturn => "call/return",
            Self::ExceptionUnwind => "exception/unwind",
            Self::MachineSuspend => "machine/suspend",
            Self::MachineResume => "machine/resume",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransitionEvent {
    pub kind: TransitionKind,
    pub from_function: u16,
    pub from_ip: u32,
    pub to_function: u16,
    pub to_ip: u32,
    pub stack_depth: u32,
    pub call_depth: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalKind {
    Return,
    Fail,
}

impl TerminalKind {
    pub const fn as_keyword(self) -> &'static str {
        match self {
            Self::Return => "machine/return",
            Self::Fail => "machine/fail",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerminalEvent {
    pub kind: TerminalKind,
    pub function: u16,
    pub ip: u32,
    pub stack_depth: u32,
    pub call_depth: u16,
}

pub trait VmProbe {
    #[inline(always)]
    fn on_instruction(&mut self, _event: InstructionEvent) {}

    #[inline(always)]
    fn on_transition(&mut self, _event: TransitionEvent) {}

    #[inline(always)]
    fn on_terminal(&mut self, _event: TerminalEvent) {}
}

#[derive(Default)]
pub struct NoProbe;

impl VmProbe for NoProbe {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BytecodeMetrics {
    pub schema: &'static str,
    pub instructions: u64,
    pub opcode_counts: [u64; Opcode::COUNT],
    pub calls: u64,
    pub returns: u64,
    pub unwinds: u64,
    pub suspensions: u64,
    pub resumptions: u64,
    pub terminal_returns: u64,
    pub failures: u64,
    pub max_stack_depth: u32,
    pub max_call_depth: u16,
}

impl Default for BytecodeMetrics {
    fn default() -> Self {
        Self {
            schema: BYTECODE_METRICS_SCHEMA,
            instructions: 0,
            opcode_counts: [0; Opcode::COUNT],
            calls: 0,
            returns: 0,
            unwinds: 0,
            suspensions: 0,
            resumptions: 0,
            terminal_returns: 0,
            failures: 0,
            max_stack_depth: 0,
            max_call_depth: 0,
        }
    }
}

#[derive(Default)]
pub struct CounterProbe {
    metrics: BytecodeMetrics,
}

impl CounterProbe {
    pub fn metrics(&self) -> &BytecodeMetrics {
        &self.metrics
    }

    pub fn into_metrics(self) -> BytecodeMetrics {
        self.metrics
    }

    pub fn opcode_count(&self, opcode: Opcode) -> u64 {
        self.metrics.opcode_counts[opcode.index()]
    }

    fn observe_depths(&mut self, stack_depth: u32, call_depth: u16) {
        self.metrics.max_stack_depth = self.metrics.max_stack_depth.max(stack_depth);
        self.metrics.max_call_depth = self.metrics.max_call_depth.max(call_depth);
    }
}

impl VmProbe for CounterProbe {
    #[inline(always)]
    fn on_instruction(&mut self, event: InstructionEvent) {
        self.metrics.instructions = self.metrics.instructions.saturating_add(1);
        self.metrics.opcode_counts[event.opcode.index()] =
            self.metrics.opcode_counts[event.opcode.index()].saturating_add(1);
        self.observe_depths(event.stack_depth, event.call_depth);
    }

    #[inline(always)]
    fn on_transition(&mut self, event: TransitionEvent) {
        match event.kind {
            TransitionKind::CallEnter => {
                self.metrics.calls = self.metrics.calls.saturating_add(1)
            }
            TransitionKind::CallReturn => {
                self.metrics.returns = self.metrics.returns.saturating_add(1)
            }
            TransitionKind::ExceptionUnwind => {
                self.metrics.unwinds = self.metrics.unwinds.saturating_add(1)
            }
            TransitionKind::MachineSuspend => {
                self.metrics.suspensions = self.metrics.suspensions.saturating_add(1)
            }
            TransitionKind::MachineResume => {
                self.metrics.resumptions = self.metrics.resumptions.saturating_add(1)
            }
        }
        self.observe_depths(event.stack_depth, event.call_depth);
    }

    #[inline(always)]
    fn on_terminal(&mut self, event: TerminalEvent) {
        match event.kind {
            TerminalKind::Return => {
                self.metrics.terminal_returns = self.metrics.terminal_returns.saturating_add(1)
            }
            TerminalKind::Fail => {
                self.metrics.failures = self.metrics.failures.saturating_add(1)
            }
        }
        self.observe_depths(event.stack_depth, event.call_depth);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VmEvent {
    Instruction(InstructionEvent),
    Transition(TransitionEvent),
    Terminal(TerminalEvent),
}

pub struct EventRing {
    slots: Box<[Option<VmEvent>]>,
    next: usize,
    len: usize,
    dropped: u64,
}

impl EventRing {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            slots: vec![None; capacity].into_boxed_slice(),
            next: 0,
            len: 0,
            dropped: 0,
        }
    }

    pub fn schema(&self) -> &'static str {
        BYTECODE_EVENTS_SCHEMA
    }

    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    pub fn iter(&self) -> impl Iterator<Item = &VmEvent> {
        let capacity = self.slots.len();
        let start = if self.len == capacity { self.next } else { 0 };
        (0..self.len).filter_map(move |offset| {
            let index = if capacity == 0 {
                0
            } else {
                (start + offset) % capacity
            };
            self.slots.get(index).and_then(Option::as_ref)
        })
    }

    fn push(&mut self, event: VmEvent) {
        if self.slots.is_empty() {
            self.dropped = self.dropped.saturating_add(1);
            return;
        }
        if self.len == self.slots.len() {
            self.dropped = self.dropped.saturating_add(1);
        } else {
            self.len += 1;
        }
        self.slots[self.next] = Some(event);
        self.next = (self.next + 1) % self.slots.len();
    }
}

impl VmProbe for EventRing {
    #[inline(always)]
    fn on_instruction(&mut self, event: InstructionEvent) {
        self.push(VmEvent::Instruction(event));
    }

    #[inline(always)]
    fn on_transition(&mut self, event: TransitionEvent) {
        self.push(VmEvent::Transition(event));
    }

    #[inline(always)]
    fn on_terminal(&mut self, event: TerminalEvent) {
        self.push(VmEvent::Terminal(event));
    }
}

pub struct SampledProbe<P> {
    inner: P,
    every: u64,
    seen: u64,
}

impl<P> SampledProbe<P> {
    pub fn new(inner: P, every: u64) -> Self {
        Self {
            inner,
            every: every.max(1),
            seen: 0,
        }
    }

    pub fn inner(&self) -> &P {
        &self.inner
    }

    pub fn into_inner(self) -> P {
        self.inner
    }
}

impl<P: VmProbe> VmProbe for SampledProbe<P> {
    #[inline(always)]
    fn on_instruction(&mut self, event: InstructionEvent) {
        let emit = self.seen % self.every == 0;
        self.seen = self.seen.saturating_add(1);
        if emit {
            self.inner.on_instruction(event);
        }
    }

    #[inline(always)]
    fn on_transition(&mut self, event: TransitionEvent) {
        self.inner.on_transition(event);
    }

    #[inline(always)]
    fn on_terminal(&mut self, event: TerminalEvent) {
        self.inner.on_terminal(event);
    }
}

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

#[cfg(test)]
mod tests;
