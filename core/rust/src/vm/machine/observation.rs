//! Opt-in, bounded observations of the validated bytecode machine.
//!
//! The ordinary [`Machine::run`] loop remains the allocation-free production
//! path. Enabling `bytecode-observation` adds a separate stepping API that
//! executes exactly one instruction or one documented call, return, unwind,
//! suspension, resume, or terminal boundary and projects the resulting state
//! into owned scalar/string/vector data suitable for `hal.bytecode-trace/v1`.

use super::{Dispatch, Machine, VmSlot};
use crate::core::{Promise, PromiseState, Value};
use crate::kernel::Position;
use crate::vm::error::VmError;
use crate::vm::opcode::Instruction;

/// Portable schema consumed by `code.vm.bytecode` and Hodos.
pub const BYTECODE_TRACE_SCHEMA: &str = "hal.bytecode-trace/v1";

/// Bounded projection limits. Stack and call projections retain the most
/// recent values/frames; locals and handlers retain their leading entries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObservationLimits {
    pub stack: usize,
    pub locals: usize,
    pub calls: usize,
    pub handlers: usize,
    pub display_chars: usize,
}

impl Default for ObservationLimits {
    fn default() -> Self {
        Self {
            stack: 64,
            locals: 64,
            calls: 32,
            handlers: 32,
            display_chars: 160,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MachineObservationStatus {
    Ready,
    Running,
    Suspended,
    Returned,
    Failed,
}

impl MachineObservationStatus {
    pub const fn as_keyword(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Running => "running",
            Self::Suspended => "suspended",
            Self::Returned => "returned",
            Self::Failed => "failed",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationEventKind {
    InstructionExecute,
    CallEnter,
    CallReturn,
    ExceptionUnwind,
    MachineSuspend,
    MachineResume,
    MachineReturn,
    MachineFail,
}

impl ObservationEventKind {
    pub const fn as_keyword(self) -> &'static str {
        match self {
            Self::InstructionExecute => "instruction/execute",
            Self::CallEnter => "call/enter",
            Self::CallReturn => "call/return",
            Self::ExceptionUnwind => "exception/unwind",
            Self::MachineSuspend => "machine/suspend",
            Self::MachineResume => "machine/resume",
            Self::MachineReturn => "machine/return",
            Self::MachineFail => "machine/fail",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationEventStatus {
    Ok,
    Error,
    Suspended,
}

impl ObservationEventStatus {
    pub const fn as_keyword(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Error => "error",
            Self::Suspended => "suspended",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InstructionOperand {
    Unsigned(u64),
    Text(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstructionSnapshot {
    pub opcode: &'static str,
    pub operands: Vec<InstructionOperand>,
    pub display: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourcePositionSnapshot {
    pub offset: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValueSnapshot {
    pub kind: &'static str,
    pub display: String,
    pub truncated: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProgramSnapshot {
    pub entry: usize,
    pub constants: usize,
    pub functions: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallFrameSnapshot {
    pub function: usize,
    pub name: Option<String>,
    pub call_ip: usize,
    pub stack_base: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HandlerSnapshot {
    pub start: usize,
    pub end: usize,
    pub depth: usize,
    pub catches: Vec<String>,
    pub finally: Option<usize>,
}

/// Fully owned, bounded state. It deliberately contains no `Rc`, `Promise`,
/// executable `Value`, closure, or host handle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MachineSnapshot {
    pub program: ProgramSnapshot,
    pub status: MachineObservationStatus,
    pub function: usize,
    pub function_name: Option<String>,
    pub function_arity: usize,
    pub function_variadic: bool,
    pub function_captures: usize,
    pub ip: usize,
    pub instruction: Option<InstructionSnapshot>,
    pub source: Option<SourcePositionSnapshot>,
    pub stack_base: usize,
    pub stack: Vec<ValueSnapshot>,
    pub stack_omitted: usize,
    pub locals: Vec<ValueSnapshot>,
    pub locals_omitted: usize,
    pub calls: Vec<CallFrameSnapshot>,
    pub calls_omitted: usize,
    pub handlers: Vec<HandlerSnapshot>,
    pub handlers_omitted: usize,
    pub result: Option<ValueSnapshot>,
    pub error: Option<String>,
}

/// Live outcome is kept separate from the serializable snapshots. Consumers can
/// persist `before`/`after` while the runtime retains the actual result/promise.
pub enum ObservedStepOutcome {
    Continue,
    Suspended(Promise),
    Returned(Value),
    Failed(VmError),
}

pub struct ObservedStep {
    pub schema: &'static str,
    pub kind: ObservationEventKind,
    pub status: ObservationEventStatus,
    pub before: MachineSnapshot,
    pub after: MachineSnapshot,
    pub instruction: Option<InstructionSnapshot>,
    pub source: Option<SourcePositionSnapshot>,
    pub outcome: ObservedStepOutcome,
}

impl Machine {
    /// Projects the current executable state with default bounds. Terminal and
    /// suspension status belongs to the `after` snapshot returned by the
    /// observed boundary that produced it.
    pub fn snapshot(&self) -> MachineSnapshot {
        self.snapshot_with_limits(ObservationLimits::default())
    }

    pub fn snapshot_with_limits(&self, limits: ObservationLimits) -> MachineSnapshot {
        self.snapshot_for(self.live_observation_status(), limits, None, None)
    }

    /// Executes one instruction or one documented VM boundary. This path is
    /// opt-in and intentionally bypasses tracing-JIT recording; JIT internals
    /// are not part of the portable observation contract.
    pub fn step_observed(&mut self) -> ObservedStep {
        self.step_observed_with_limits(ObservationLimits::default())
    }

    pub fn step_observed_with_limits(&mut self, limits: ObservationLimits) -> ObservedStep {
        let before = self.snapshot_for(self.live_observation_status(), limits, None, None);
        let instruction_snapshot = before.instruction.clone();
        let source_snapshot = before.source.clone();
        let program = self.program.clone();
        let Some(function) = program.functions.get(self.function) else {
            let error = VmError::new("function index out of range", 0, None);
            return self.failed_observation(
                before,
                instruction_snapshot,
                source_snapshot,
                error,
                limits,
            );
        };
        let Some(instruction) = function.code.get(self.ip).cloned() else {
            let error = self.error(function, "instruction pointer out of range");
            return self.failed_observation(
                before,
                instruction_snapshot,
                source_snapshot,
                error,
                limits,
            );
        };

        match self.dispatch(&program, function, &instruction) {
            Dispatch::Next(ip) => {
                self.ip = ip;
                self.continue_observation(
                    ObservationEventKind::InstructionExecute,
                    before,
                    instruction_snapshot,
                    source_snapshot,
                    limits,
                )
            }
            Dispatch::Unwound(ip) => {
                self.clear_observed_jit_boundary();
                self.ip = ip;
                self.continue_observation(
                    ObservationEventKind::ExceptionUnwind,
                    before,
                    instruction_snapshot,
                    source_snapshot,
                    limits,
                )
            }
            Dispatch::Call { callee, args } => {
                self.clear_observed_jit_boundary();
                if let Err(message) = self.enter_callable(&program, callee, args) {
                    match self.raise(function, message) {
                        Ok(target) => {
                            self.ip = target;
                            return self.continue_observation(
                                ObservationEventKind::ExceptionUnwind,
                                before,
                                instruction_snapshot,
                                source_snapshot,
                                limits,
                            );
                        }
                        Err(error) => {
                            return self.failed_observation(
                                before,
                                instruction_snapshot,
                                source_snapshot,
                                error,
                                limits,
                            );
                        }
                    }
                }
                self.continue_observation(
                    ObservationEventKind::CallEnter,
                    before,
                    instruction_snapshot,
                    source_snapshot,
                    limits,
                )
            }
            Dispatch::CallStatic {
                prototype,
                args,
                captures,
            } => {
                self.clear_observed_jit_boundary();
                self.enter_or_spawn(&program, prototype, args, captures);
                self.continue_observation(
                    ObservationEventKind::CallEnter,
                    before,
                    instruction_snapshot,
                    source_snapshot,
                    limits,
                )
            }
            Dispatch::CallStaticDirect { prototype, argc } => {
                self.clear_observed_jit_boundary();
                self.enter_static_direct(&program, prototype, argc);
                self.continue_observation(
                    ObservationEventKind::CallEnter,
                    before,
                    instruction_snapshot,
                    source_snapshot,
                    limits,
                )
            }
            Dispatch::Returned(value) => {
                self.clear_observed_jit_boundary();
                self.stack.truncate(self.frame.base());
                if let Some(caller) = self.calls.pop() {
                    self.function = caller.function;
                    let completed = std::mem::replace(&mut self.frame, caller.frame);
                    self.free_locals.push(completed.into_locals());
                    self.ip = caller.call_ip + 1;
                    self.stack.push(value);
                    self.continue_observation(
                        ObservationEventKind::CallReturn,
                        before,
                        instruction_snapshot,
                        source_snapshot,
                        limits,
                    )
                } else {
                    let value = Self::into_value(program, value);
                    let result = value_snapshot(&value, limits.display_chars);
                    let after = self.snapshot_for(
                        MachineObservationStatus::Returned,
                        limits,
                        Some(result),
                        None,
                    );
                    ObservedStep {
                        schema: BYTECODE_TRACE_SCHEMA,
                        kind: ObservationEventKind::MachineReturn,
                        status: ObservationEventStatus::Ok,
                        before,
                        after,
                        instruction: instruction_snapshot,
                        source: source_snapshot,
                        outcome: ObservedStepOutcome::Returned(value),
                    }
                }
            }
            Dispatch::Suspended(promise) => {
                let after =
                    self.snapshot_for(MachineObservationStatus::Suspended, limits, None, None);
                ObservedStep {
                    schema: BYTECODE_TRACE_SCHEMA,
                    kind: ObservationEventKind::MachineSuspend,
                    status: ObservationEventStatus::Suspended,
                    before,
                    after,
                    instruction: instruction_snapshot,
                    source: source_snapshot,
                    outcome: ObservedStepOutcome::Suspended(promise),
                }
            }
            Dispatch::Failed(error) => self.failed_observation(
                before,
                instruction_snapshot,
                source_snapshot,
                error,
                limits,
            ),
        }
    }

    /// Applies one settlement at a suspended `Await` without automatically
    /// running subsequent instructions. Callers can continue with
    /// `step_observed`, preserving one event boundary per call.
    pub fn resume_observed(&mut self, state: PromiseState) -> ObservedStep {
        self.resume_observed_with_limits(state, ObservationLimits::default())
    }

    pub fn resume_observed_with_limits(
        &mut self,
        state: PromiseState,
        limits: ObservationLimits,
    ) -> ObservedStep {
        let before = self.snapshot_for(MachineObservationStatus::Suspended, limits, None, None);
        let instruction_snapshot = before.instruction.clone();
        let source_snapshot = before.source.clone();
        let Some(function) = self.program.functions.get(self.function).cloned() else {
            let error = VmError::new("function index out of range", 0, None);
            return self.failed_observation(
                before,
                instruction_snapshot,
                source_snapshot,
                error,
                limits,
            );
        };
        if !matches!(function.code.get(self.ip), Some(Instruction::Await)) {
            let error = self.error(&function, "VM is not suspended at await");
            return self.failed_observation(
                before,
                instruction_snapshot,
                source_snapshot,
                error,
                limits,
            );
        }

        match state {
            PromiseState::Pending => {
                let promise = match self.stack.last().and_then(VmSlot::runtime_value) {
                    Some(Value::Promise(promise)) => promise,
                    _ => {
                        let error = self.error(&function, "await expects a promise");
                        return self.failed_observation(
                            before,
                            instruction_snapshot,
                            source_snapshot,
                            error,
                            limits,
                        );
                    }
                };
                let after =
                    self.snapshot_for(MachineObservationStatus::Suspended, limits, None, None);
                ObservedStep {
                    schema: BYTECODE_TRACE_SCHEMA,
                    kind: ObservationEventKind::MachineSuspend,
                    status: ObservationEventStatus::Suspended,
                    before,
                    after,
                    instruction: instruction_snapshot,
                    source: source_snapshot,
                    outcome: ObservedStepOutcome::Suspended(promise),
                }
            }
            PromiseState::Fulfilled(value) => {
                self.stack.pop();
                self.stack.push(value.into());
                self.ip += 1;
                self.continue_observation(
                    ObservationEventKind::MachineResume,
                    before,
                    instruction_snapshot,
                    source_snapshot,
                    limits,
                )
            }
            PromiseState::Rejected(error) => {
                self.stack.pop();
                match self.raise(&function, crate::core::promise_rejection_error(error)) {
                    Ok(target) => {
                        self.ip = target;
                        self.continue_observation(
                            ObservationEventKind::ExceptionUnwind,
                            before,
                            instruction_snapshot,
                            source_snapshot,
                            limits,
                        )
                    }
                    Err(error) => self.failed_observation(
                        before,
                        instruction_snapshot,
                        source_snapshot,
                        error,
                        limits,
                    ),
                }
            }
        }
    }

    fn continue_observation(
        &self,
        kind: ObservationEventKind,
        before: MachineSnapshot,
        instruction: Option<InstructionSnapshot>,
        source: Option<SourcePositionSnapshot>,
        limits: ObservationLimits,
    ) -> ObservedStep {
        ObservedStep {
            schema: BYTECODE_TRACE_SCHEMA,
            kind,
            status: ObservationEventStatus::Ok,
            before,
            after: self.snapshot_for(MachineObservationStatus::Running, limits, None, None),
            instruction,
            source,
            outcome: ObservedStepOutcome::Continue,
        }
    }

    fn failed_observation(
        &self,
        before: MachineSnapshot,
        instruction: Option<InstructionSnapshot>,
        source: Option<SourcePositionSnapshot>,
        error: VmError,
        limits: ObservationLimits,
    ) -> ObservedStep {
        let message = error.message.clone();
        ObservedStep {
            schema: BYTECODE_TRACE_SCHEMA,
            kind: ObservationEventKind::MachineFail,
            status: ObservationEventStatus::Error,
            before,
            after: self.snapshot_for(
                MachineObservationStatus::Failed,
                limits,
                None,
                Some(message),
            ),
            instruction,
            source,
            outcome: ObservedStepOutcome::Failed(error),
        }
    }

    fn snapshot_for(
        &self,
        status: MachineObservationStatus,
        limits: ObservationLimits,
        result: Option<ValueSnapshot>,
        error: Option<String>,
    ) -> MachineSnapshot {
        let function = self.program.functions.get(self.function);
        let instruction = function
            .and_then(|function| function.code.get(self.ip))
            .map(instruction_snapshot);
        let source = function
            .and_then(|function| function.source_map.position(self.ip))
            .map(position_snapshot);
        let (stack, stack_omitted) = slot_tail(&self.stack, limits.stack, limits.display_chars);
        let (locals, locals_omitted) =
            slot_head(self.frame.locals(), limits.locals, limits.display_chars);
        let call_start = self.calls.len().saturating_sub(limits.calls);
        let calls = self.calls[call_start..]
            .iter()
            .map(|frame| CallFrameSnapshot {
                function: frame.function,
                name: self
                    .program
                    .functions
                    .get(frame.function)
                    .and_then(|function| function.name.clone()),
                call_ip: frame.call_ip,
                stack_base: frame.frame.base(),
            })
            .collect();
        let handlers_all = function.map_or(&[][..], |function| function.handlers.as_slice());
        let handler_count = handlers_all.len().min(limits.handlers);
        let handlers = handlers_all[..handler_count]
            .iter()
            .map(|handler| HandlerSnapshot {
                start: handler.start as usize,
                end: handler.end as usize,
                depth: handler.depth as usize,
                catches: handler
                    .catches
                    .iter()
                    .map(|catch| catch.class.clone())
                    .collect(),
                finally: handler.finally.map(|value| value as usize),
            })
            .collect();

        MachineSnapshot {
            program: ProgramSnapshot {
                entry: self.program.entry as usize,
                constants: self.program.constants.len(),
                functions: self.program.functions.len(),
            },
            status,
            function: self.function,
            function_name: function.and_then(|function| function.name.clone()),
            function_arity: function.map_or(0, |function| function.arity as usize),
            function_variadic: function.is_some_and(|function| function.variadic),
            function_captures: function.map_or(0, |function| function.capture_count as usize),
            ip: self.ip,
            instruction,
            source,
            stack_base: self.frame.base(),
            stack,
            stack_omitted,
            locals,
            locals_omitted,
            calls,
            calls_omitted: call_start,
            handlers,
            handlers_omitted: handlers_all.len().saturating_sub(handler_count),
            result,
            error,
        }
    }

    fn live_observation_status(&self) -> MachineObservationStatus {
        if self.function == self.program.entry as usize
            && self.ip == 0
            && self.calls.is_empty()
            && self.stack.is_empty()
        {
            MachineObservationStatus::Ready
        } else {
            MachineObservationStatus::Running
        }
    }

    #[cfg(feature = "tracing-jit")]
    fn clear_observed_jit_boundary(&mut self) {
        self.jit_path.clear();
        self.jit_loop_entries.clear();
    }

    #[cfg(not(feature = "tracing-jit"))]
    fn clear_observed_jit_boundary(&mut self) {}
}

fn slot_head(values: &[VmSlot], limit: usize, display_chars: usize) -> (Vec<ValueSnapshot>, usize) {
    let retained = values.len().min(limit);
    (
        values[..retained]
            .iter()
            .map(|value| slot_snapshot(value, display_chars))
            .collect(),
        values.len().saturating_sub(retained),
    )
}

fn slot_tail(values: &[VmSlot], limit: usize, display_chars: usize) -> (Vec<ValueSnapshot>, usize) {
    let start = values.len().saturating_sub(limit);
    (
        values[start..]
            .iter()
            .map(|value| slot_snapshot(value, display_chars))
            .collect(),
        start,
    )
}

fn slot_snapshot(value: &VmSlot, display_chars: usize) -> ValueSnapshot {
    match value {
        VmSlot::Number(value) => bounded_value("number", value.to_string(), display_chars),
        VmSlot::Bool(value) => bounded_value("boolean", value.to_string(), display_chars),
        VmSlot::Nil => bounded_value("nil", "nil".to_string(), display_chars),
        VmSlot::Value(value) => value_snapshot(value, display_chars),
        VmSlot::InlineClosure {
            prototype,
            identity,
        } => bounded_value(
            "closure",
            format!("<closure prototype={prototype} identity={identity}>"),
            display_chars,
        ),
        VmSlot::Closure(closure) => bounded_value(
            "closure",
            format!(
                "<closure prototype={} captures={}",
                closure.prototype,
                closure.captures.len()
            ) + ">",
            display_chars,
        ),
        VmSlot::MultiArity(dispatch) => bounded_value(
            "multi-arity",
            format!(
                "<multi-arity {} clauses={}",
                dispatch.name,
                dispatch.clauses.len()
            ) + ">",
            display_chars,
        ),
    }
}

fn value_snapshot(value: &Value, display_chars: usize) -> ValueSnapshot {
    let kind = match value {
        Value::Number(_) => "number",
        Value::Bool(_) => "boolean",
        Value::Nil => "nil",
        Value::String(_) => "string",
        Value::Promise(_) => "promise",
        _ => "value",
    };
    bounded_value(kind, value.display(), display_chars)
}

fn bounded_value(kind: &'static str, display: String, limit: usize) -> ValueSnapshot {
    let mut chars = display.chars();
    let mut bounded: String = chars.by_ref().take(limit).collect();
    let truncated = chars.next().is_some();
    if truncated {
        bounded.push('…');
    }
    ValueSnapshot {
        kind,
        display: bounded,
        truncated,
    }
}

fn position_snapshot(position: Position) -> SourcePositionSnapshot {
    SourcePositionSnapshot {
        offset: position.offset,
        line: position.line,
        column: position.column,
    }
}

fn instruction_snapshot(instruction: &Instruction) -> InstructionSnapshot {
    use InstructionOperand::{Text, Unsigned};

    let (opcode, operands) = match instruction {
        Instruction::Constant(index) => ("constant", vec![Unsigned(*index as u64)]),
        Instruction::Nil => ("nil", vec![]),
        Instruction::True => ("true", vec![]),
        Instruction::False => ("false", vec![]),
        Instruction::LoadLocal(slot) => ("load-local", vec![Unsigned(*slot as u64)]),
        Instruction::StoreLocal(slot) => ("store-local", vec![Unsigned(*slot as u64)]),
        Instruction::Pop => ("pop", vec![]),
        Instruction::Dup => ("dup", vec![]),
        Instruction::Primitive { op, argc } => (
            "primitive",
            vec![Text(op.operator().to_string()), Unsigned(*argc as u64)],
        ),
        Instruction::PrimitiveLocalConst {
            op,
            local,
            constant,
        } => (
            "primitive-local-const",
            vec![
                Text(op.operator().to_string()),
                Unsigned(*local as u64),
                Unsigned(*constant as u64),
            ],
        ),
        Instruction::Jump(target) => ("jump", vec![Unsigned(*target as u64)]),
        Instruction::JumpIfFalse(target) => ("jump-if-false", vec![Unsigned(*target as u64)]),
        Instruction::Closure {
            prototype,
            captures,
        } => (
            "closure",
            vec![Unsigned(*prototype as u64), Unsigned(*captures as u64)],
        ),
        Instruction::Call { argc } => ("call", vec![Unsigned(*argc as u64)]),
        Instruction::CallStatic { prototype, argc } => (
            "call-static",
            vec![Unsigned(*prototype as u64), Unsigned(*argc as u64)],
        ),
        Instruction::Throw => ("throw", vec![]),
        Instruction::Rethrow => ("rethrow", vec![]),
        Instruction::GetGlobal(index) => ("get-global", vec![Unsigned(*index as u64)]),
        Instruction::DefGlobal { name, metadata } => {
            let mut operands = vec![Unsigned(*name as u64)];
            if let Some(metadata) = metadata {
                operands.push(Unsigned(*metadata as u64));
            }
            ("def-global", operands)
        }
        Instruction::SetGlobal(index) => ("set-global", vec![Unsigned(*index as u64)]),
        Instruction::VarGlobal(index) => ("var-global", vec![Unsigned(*index as u64)]),
        Instruction::DeclareGlobal(index) => ("declare-global", vec![Unsigned(*index as u64)]),
        Instruction::DefStruct { name, fields } => (
            "def-struct",
            vec![Unsigned(*name as u64), Unsigned(*fields as u64)],
        ),
        Instruction::StructField(index) => ("struct-field", vec![Unsigned(*index as u64)]),
        Instruction::InstanceOf => ("instance-of", vec![]),
        Instruction::MakeMultiArity { name, count } => (
            "make-multi-arity",
            vec![Unsigned(*name as u64), Unsigned(*count as u64)],
        ),
        Instruction::BuildVector(count) => ("build-vector", vec![Unsigned(*count as u64)]),
        Instruction::BuildMap(pairs) => ("build-map", vec![Unsigned(*pairs as u64)]),
        Instruction::BuildSet(count) => ("build-set", vec![Unsigned(*count as u64)]),
        Instruction::BuildList(count) => ("build-list", vec![Unsigned(*count as u64)]),
        Instruction::ConcatList(count) => ("concat-list", vec![Unsigned(*count as u64)]),
        Instruction::ToVector => ("to-vector", vec![]),
        Instruction::DefMacro { name, metadata } => {
            let mut operands = vec![Unsigned(*name as u64)];
            if let Some(metadata) = metadata {
                operands.push(Unsigned(*metadata as u64));
            }
            ("def-macro", operands)
        }
        Instruction::Await => ("await", vec![]),
        Instruction::HostCall => ("host-call", vec![]),
        Instruction::Return => ("return", vec![]),
    };

    InstructionSnapshot {
        opcode,
        operands,
        display: instruction.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::program::{FunctionPrototype, Program};
    use crate::vm::source_map::SourceMap;
    use crate::vm::{compile_source, validate};
    use std::rc::Rc;

    fn machine(source: &str) -> Machine {
        Machine::entry(Rc::new(
            compile_source(source).expect("source must compile"),
        ))
    }

    fn run_observed(machine: &mut Machine) -> (Vec<ObservationEventKind>, Value) {
        let mut kinds = Vec::new();
        for _ in 0..256 {
            let step = machine.step_observed();
            kinds.push(step.kind);
            match step.outcome {
                ObservedStepOutcome::Continue => {}
                ObservedStepOutcome::Returned(value) => return (kinds, value),
                ObservedStepOutcome::Suspended(_) => panic!("unexpected suspension"),
                ObservedStepOutcome::Failed(error) => panic!("unexpected failure: {error}"),
            }
        }
        panic!("observed execution did not terminate");
    }

    #[test]
    fn arithmetic_steps_project_instructions_and_return_value() {
        let mut machine = machine("(+ 1 (* 2 3))");
        let initial = machine.snapshot();
        assert_eq!(initial.status, MachineObservationStatus::Ready);
        assert_eq!(initial.program.entry, 0);

        // The production compiler folds the nested multiplication.
        // Observation must report only instructions the VM actually executes.
        let mut saw_add = false;
        let mut final_after = None;
        for _ in 0..64 {
            let step = machine.step_observed();
            if let Some(instruction) = &step.instruction {
                if instruction.opcode == "primitive"
                    && instruction
                        .operands
                        .contains(&InstructionOperand::Text("+".into()))
                {
                    saw_add = true;
                }
            }
            match step.outcome {
                ObservedStepOutcome::Continue => {}
                ObservedStepOutcome::Returned(value) => {
                    assert_eq!(value, Value::Number(7));
                    final_after = Some(step.after);
                    break;
                }
                ObservedStepOutcome::Suspended(_) => panic!("unexpected suspension"),
                ObservedStepOutcome::Failed(error) => panic!("unexpected failure: {error}"),
            }
        }
        assert!(saw_add);
        let after = final_after.expect("return snapshot");
        assert_eq!(after.status, MachineObservationStatus::Returned);
        assert_eq!(after.result.expect("result").display, "7");
    }

    #[test]
    fn static_calls_report_enter_and_return_boundaries() {
        let mut machine = machine("(do (defn f [x] (+ x 1)) (f 41))");
        let registry = crate::embedding_namespace_registry();
        let (kinds, value) =
            crate::core::with_namespace_registry(&registry, || run_observed(&mut machine));
        assert_eq!(value, Value::Number(42));
        assert!(kinds.contains(&ObservationEventKind::CallEnter));
        assert!(kinds.contains(&ObservationEventKind::CallReturn));
    }

    #[test]
    fn caught_runtime_errors_report_an_exact_unwind_boundary() {
        let mut machine = machine("(try (/ 1 0) (catch Exception error 42))");
        let (kinds, value) = run_observed(&mut machine);
        assert_eq!(value, Value::Number(42));
        assert!(kinds.contains(&ObservationEventKind::ExceptionUnwind));
    }

    #[test]
    fn uncaught_runtime_errors_keep_source_and_terminal_diagnostics() {
        let mut machine = machine("(/ 1 0)");
        for _ in 0..32 {
            let step = machine.step_observed();
            match step.outcome {
                ObservedStepOutcome::Continue => {}
                ObservedStepOutcome::Failed(error) => {
                    assert_eq!(step.kind, ObservationEventKind::MachineFail);
                    assert_eq!(step.status, ObservationEventStatus::Error);
                    assert_eq!(step.after.status, MachineObservationStatus::Failed);
                    assert_eq!(step.after.error.as_deref(), Some("division by zero"));
                    assert_eq!(error.message, "division by zero");
                    let source = step.source.expect("source position");
                    assert_eq!((source.line, source.column), (1, 1));
                    return;
                }
                ObservedStepOutcome::Returned(value) => {
                    panic!("unexpected return: {}", value.display())
                }
                ObservedStepOutcome::Suspended(_) => panic!("unexpected suspension"),
            }
        }
        panic!("failure was not observed");
    }

    #[test]
    fn pending_await_resumes_as_one_boundary() {
        let promise = Promise::new();
        let mut source_map = SourceMap::default();
        for _ in 0..3 {
            source_map.record(None);
        }
        let program = Program {
            namespace: None,
            constants: vec![Value::Promise(promise.clone())],
            var_metadata: Vec::new(),
            schema_types: Default::default(),
            function_types: Default::default(),
            inferred_function_types: Default::default(),
            functions: vec![FunctionPrototype {
                name: Some("await-demo".into()),
                async_function: false,
                arity: 0,
                variadic: false,
                capture_count: 0,
                local_count: 0,
                max_stack: 1,
                code: vec![
                    Instruction::Constant(0),
                    Instruction::Await,
                    Instruction::Return,
                ],
                source_map,
                handlers: Vec::new(),
            }],
            entry: 0,
        };
        validate(&program).expect("program must validate");
        let mut machine = Machine::entry(Rc::new(program));

        assert!(matches!(
            machine.step_observed().outcome,
            ObservedStepOutcome::Continue
        ));
        let suspended = machine.step_observed();
        assert_eq!(suspended.kind, ObservationEventKind::MachineSuspend);
        assert_eq!(suspended.after.status, MachineObservationStatus::Suspended);
        assert!(matches!(
            suspended.outcome,
            ObservedStepOutcome::Suspended(_)
        ));

        assert!(promise.resolve(Value::Number(42)));
        let resumed = machine.resume_observed(promise.state());
        assert_eq!(resumed.kind, ObservationEventKind::MachineResume);
        assert_eq!(resumed.after.ip, 2);
        assert!(matches!(resumed.outcome, ObservedStepOutcome::Continue));

        let returned = machine.step_observed();
        match returned.outcome {
            ObservedStepOutcome::Returned(value) => assert_eq!(value, Value::Number(42)),
            _ => panic!("expected return after resume"),
        }
    }

    #[test]
    fn snapshot_limits_bound_stack_and_preserve_the_top() {
        let mut machine = machine("[1 2 3 4]");
        for _ in 0..4 {
            assert!(matches!(
                machine.step_observed().outcome,
                ObservedStepOutcome::Continue
            ));
        }
        let snapshot = machine.snapshot_with_limits(ObservationLimits {
            stack: 2,
            ..ObservationLimits::default()
        });
        assert_eq!(snapshot.stack_omitted, 2);
        assert_eq!(
            snapshot
                .stack
                .iter()
                .map(|value| value.display.as_str())
                .collect::<Vec<_>>(),
            vec!["3", "4"]
        );
    }
}
