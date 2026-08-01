use super::trace_ir::{ExitReason, ExitSnapshot, Trace, TraceOp, TraceOutcome, TraceValue};
use crate::core::Primitive;

pub trait TraceBackend {
    type Compiled;
    fn compile(&mut self, trace: &Trace) -> Result<Self::Compiled, String>;
    fn enter(&mut self, trace: &Self::Compiled, locals: &mut [TraceValue], max_iterations: u32) -> TraceOutcome;
}

#[derive(Default)]
pub struct CheckedBackend;

impl TraceBackend for CheckedBackend {
    type Compiled = Trace;

    fn compile(&mut self, trace: &Trace) -> Result<Trace, String> {
        Ok(trace.clone())
    }

    fn enter(&mut self, trace: &Trace, locals: &mut [TraceValue], max_iterations: u32) -> TraceOutcome {
        let mut iterations = 0;
        let mut stack = Vec::with_capacity(8);
        while iterations < max_iterations {
            stack.clear();
            for operation in &trace.operations {
                let exit = |reason, stack: &Vec<TraceValue>, locals: &[TraceValue]| TraceOutcome::SideExit {
                    reason,
                    snapshot: ExitSnapshot {
                        function: trace.function,
                        instruction: trace.resume_ip,
                        locals: locals.to_vec(),
                        stack: stack.clone(),
                    },
                };
                match *operation {
                    TraceOp::GuardLocalI64 { local } => {
                        if !matches!(locals.get(local as usize), Some(TraceValue::I64(_))) {
                            return exit(ExitReason::WrongTag, &stack, locals);
                        }
                    }
                    TraceOp::LoadLocal { local } => match locals.get(local as usize).copied() {
                        Some(value) => stack.push(value),
                        None => return exit(ExitReason::WrongTag, &stack, locals),
                    },
                    TraceOp::ConstantI64(value) => stack.push(TraceValue::I64(value)),
                    TraceOp::StoreLocal { local } => {
                        let Some(value) = stack.pop() else { return exit(ExitReason::Unsupported, &stack, locals) };
                        let Some(slot) = locals.get_mut(local as usize) else { return exit(ExitReason::Unsupported, &stack, locals) };
                        *slot = value;
                    }
                    TraceOp::Pop => { stack.pop(); }
                    TraceOp::GuardTruthy { expected } => {
                        let Some(value) = stack.pop() else { return exit(ExitReason::Unsupported, &stack, locals) };
                        let truthy = !matches!(value, TraceValue::Bool(false) | TraceValue::Nil);
                        if truthy != expected { return exit(ExitReason::BranchChanged, &stack, locals); }
                    }
                    TraceOp::BinaryI64(op) => {
                        let (Some(TraceValue::I64(right)), Some(TraceValue::I64(left))) = (stack.pop(), stack.pop()) else {
                            return exit(ExitReason::WrongTag, &stack, locals);
                        };
                        let value = match op {
                            Primitive::Add => left.checked_add(right).map(TraceValue::I64),
                            Primitive::Subtract => left.checked_sub(right).map(TraceValue::I64),
                            Primitive::Multiply => left.checked_mul(right).map(TraceValue::I64),
                            Primitive::Remainder if right == 0 => return exit(ExitReason::DivisionByZero, &stack, locals),
                            Primitive::Remainder => left.checked_rem(right).map(TraceValue::I64),
                            Primitive::Less => Some(TraceValue::Bool(left < right)),
                            Primitive::LessOrEqual => Some(TraceValue::Bool(left <= right)),
                            Primitive::Greater => Some(TraceValue::Bool(left > right)),
                            Primitive::GreaterOrEqual => Some(TraceValue::Bool(left >= right)),
                            Primitive::Equal => Some(TraceValue::Bool(left == right)),
                            _ => return exit(ExitReason::Unsupported, &stack, locals),
                        };
                        let Some(value) = value else { return exit(ExitReason::Overflow, &stack, locals) };
                        stack.push(value);
                    }
                    TraceOp::LoopBackedge => iterations += 1,
                }
            }
        }
        TraceOutcome::Completed { iterations }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn increment_trace() -> Trace {
        Trace {
            function: 0,
            header: 2,
            resume_ip: 2,
            operations: vec![
                TraceOp::GuardLocalI64 { local: 0 },
                TraceOp::LoadLocal { local: 0 },
                TraceOp::ConstantI64(1),
                TraceOp::BinaryI64(Primitive::Add),
                TraceOp::StoreLocal { local: 0 },
                TraceOp::LoopBackedge,
            ],
        }
    }

    #[test]
    fn checked_backend_executes_and_guards() {
        let trace = increment_trace();
        let mut backend = CheckedBackend;
        let compiled = backend.compile(&trace).unwrap();
        let mut locals = [TraceValue::I64(0)];
        assert_eq!(backend.enter(&compiled, &mut locals, 5), TraceOutcome::Completed { iterations: 5 });
        assert_eq!(locals[0], TraceValue::I64(5));
        let mut wrong = [TraceValue::Bool(false)];
        assert!(matches!(backend.enter(&compiled, &mut wrong, 1), TraceOutcome::SideExit { reason: ExitReason::WrongTag, .. }));
    }
}
