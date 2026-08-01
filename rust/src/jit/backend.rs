use super::trace_ir::{ExitReason, ExitSnapshot, Trace, TraceOp, TraceOutcome, TraceValue};
use crate::core::{Primitive, Value};

pub trait TraceBackend {
    type Compiled;
    fn compile(&mut self, trace: &Trace) -> Result<Self::Compiled, String>;
    fn enter(
        &mut self,
        trace: &mut Self::Compiled,
        locals: &mut [TraceValue],
        max_iterations: u32,
    ) -> TraceOutcome;
}

#[derive(Default)]
pub struct CheckedBackend;

impl TraceBackend for CheckedBackend {
    type Compiled = Trace;

    fn compile(&mut self, trace: &Trace) -> Result<Trace, String> {
        Ok(trace.clone())
    }

    fn enter(
        &mut self,
        trace: &mut Trace,
        locals: &mut [TraceValue],
        max_iterations: u32,
    ) -> TraceOutcome {
        for operation in &trace.operations {
            let valid = match operation {
                TraceOp::GuardLocalI64 { local } => {
                    matches!(locals.get(usize::from(*local)), Some(TraceValue::I64(_)))
                }
                TraceOp::GuardLocalVectorI64 { local } => {
                    locals.get(usize::from(*local)).is_some_and(numeric_vector)
                }
                _ => true,
            };
            if !valid {
                return TraceOutcome::SideExit {
                    reason: ExitReason::WrongTag,
                    snapshot: ExitSnapshot {
                        function: trace.function,
                        instruction: trace.resume_ip,
                        locals: locals.to_vec(),
                        stack: Vec::new(),
                    },
                };
            }
        }
        let mut iterations = 0;
        let mut stack = Vec::with_capacity(8);
        while iterations < max_iterations {
            stack.clear();
            for operation in &trace.operations {
                let exit = |reason, stack: &Vec<TraceValue>, locals: &[TraceValue]| {
                    TraceOutcome::SideExit {
                        reason,
                        snapshot: ExitSnapshot {
                            function: trace.function,
                            instruction: trace.resume_ip,
                            locals: locals.to_vec(),
                            stack: stack.clone(),
                        },
                    }
                };
                match *operation {
                    TraceOp::GuardLocalI64 { .. } | TraceOp::GuardLocalVectorI64 { .. } => {}
                    TraceOp::LoadLocal { local } => match locals.get(local as usize).cloned() {
                        Some(value) => stack.push(value),
                        None => return exit(ExitReason::WrongTag, &stack, locals),
                    },
                    TraceOp::ConstantI64(value) => stack.push(TraceValue::I64(value)),
                    TraceOp::ConstantVectorI64 { vector } => {
                        let Some(values) = trace.vectors.get(usize::from(vector)) else {
                            return exit(ExitReason::Unsupported, &stack, locals);
                        };
                        stack.push(TraceValue::Indexed(Box::new(Value::Vector(
                            values.iter().copied().map(Value::Number).collect(),
                        ))));
                    }
                    TraceOp::StoreLocal { local } => {
                        let Some(value) = stack.pop() else {
                            return exit(ExitReason::Unsupported, &stack, locals);
                        };
                        let Some(slot) = locals.get_mut(local as usize) else {
                            return exit(ExitReason::Unsupported, &stack, locals);
                        };
                        *slot = value;
                    }
                    TraceOp::Pop => {
                        stack.pop();
                    }
                    TraceOp::GuardTruthy { expected } => {
                        let Some(value) = stack.pop() else {
                            return exit(ExitReason::Unsupported, &stack, locals);
                        };
                        let truthy = !matches!(value, TraceValue::Bool(false) | TraceValue::Nil);
                        if truthy != expected {
                            return exit(ExitReason::BranchChanged, &stack, locals);
                        }
                    }
                    TraceOp::BinaryI64(op) => {
                        let (Some(TraceValue::I64(right)), Some(TraceValue::I64(left))) =
                            (stack.pop(), stack.pop())
                        else {
                            return exit(ExitReason::WrongTag, &stack, locals);
                        };
                        let value = match op {
                            Primitive::Add => left.checked_add(right).map(TraceValue::I64),
                            Primitive::Subtract => left.checked_sub(right).map(TraceValue::I64),
                            Primitive::Multiply => left.checked_mul(right).map(TraceValue::I64),
                            Primitive::Remainder if right == 0 => {
                                return exit(ExitReason::DivisionByZero, &stack, locals)
                            }
                            Primitive::Remainder => left.checked_rem(right).map(TraceValue::I64),
                            Primitive::Less => Some(TraceValue::Bool(left < right)),
                            Primitive::LessOrEqual => Some(TraceValue::Bool(left <= right)),
                            Primitive::Greater => Some(TraceValue::Bool(left > right)),
                            Primitive::GreaterOrEqual => Some(TraceValue::Bool(left >= right)),
                            Primitive::Equal => Some(TraceValue::Bool(left == right)),
                            _ => return exit(ExitReason::Unsupported, &stack, locals),
                        };
                        let Some(value) = value else {
                            return exit(ExitReason::Overflow, &stack, locals);
                        };
                        stack.push(value);
                    }
                    TraceOp::VectorNthI64 => {
                        let Some(TraceValue::I64(index)) = stack.pop() else {
                            return exit(ExitReason::WrongTag, &stack, locals);
                        };
                        let Some(vector) = stack.pop() else {
                            return exit(ExitReason::WrongTag, &stack, locals);
                        };
                        let Some(index) = usize::try_from(index).ok() else {
                            return exit(ExitReason::IndexOutOfBounds, &stack, locals);
                        };
                        let TraceValue::Indexed(value) = vector else {
                            return exit(ExitReason::WrongTag, &stack, locals);
                        };
                        let value = match value.as_ref() {
                            Value::Tuple(values) => values.get(index),
                            Value::Vector(values) => values.get(index),
                            _ => None,
                        };
                        match value {
                            Some(Value::Number(value)) => stack.push(TraceValue::I64(*value)),
                            Some(_) => return exit(ExitReason::WrongTag, &stack, locals),
                            None => return exit(ExitReason::IndexOutOfBounds, &stack, locals),
                        }
                    }
                    TraceOp::LoopBackedge => iterations += 1,
                }
            }
        }
        TraceOutcome::Completed { iterations }
    }
}

fn numeric_vector(value: &TraceValue) -> bool {
    let TraceValue::Indexed(value) = value else {
        return false;
    };
    match value.as_ref() {
        Value::Tuple(values) => values.iter().all(|value| matches!(value, Value::Number(_))),
        Value::Vector(values) => values.iter().all(|value| matches!(value, Value::Number(_))),
        _ => false,
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
            vectors: Vec::new(),
        }
    }

    #[test]
    fn checked_backend_executes_and_guards() {
        let trace = increment_trace();
        let mut backend = CheckedBackend;
        let mut compiled = backend.compile(&trace).unwrap();
        let mut locals = [TraceValue::I64(0)];
        assert_eq!(
            backend.enter(&mut compiled, &mut locals, 5),
            TraceOutcome::Completed { iterations: 5 }
        );
        assert_eq!(locals[0], TraceValue::I64(5));
        let mut wrong = [TraceValue::Bool(false)];
        assert!(matches!(
            backend.enter(&mut compiled, &mut wrong, 1),
            TraceOutcome::SideExit {
                reason: ExitReason::WrongTag,
                ..
            }
        ));
    }

    #[test]
    fn checked_backend_indexes_numeric_vector_constants_and_exits_on_bounds() {
        let trace = Trace {
            function: 0,
            header: 0,
            resume_ip: 0,
            operations: vec![
                TraceOp::ConstantVectorI64 { vector: 0 },
                TraceOp::LoadLocal { local: 0 },
                TraceOp::VectorNthI64,
                TraceOp::StoreLocal { local: 1 },
                TraceOp::LoopBackedge,
            ],
            vectors: vec![vec![10, 20, 30]],
        };
        let mut backend = CheckedBackend;
        let mut compiled = backend.compile(&trace).unwrap();
        let mut locals = [TraceValue::I64(1), TraceValue::Nil];
        assert_eq!(
            backend.enter(&mut compiled, &mut locals, 1),
            TraceOutcome::Completed { iterations: 1 }
        );
        assert_eq!(locals[1], TraceValue::I64(20));

        locals[0] = TraceValue::I64(3);
        assert!(matches!(
            backend.enter(&mut compiled, &mut locals, 1),
            TraceOutcome::SideExit {
                reason: ExitReason::IndexOutOfBounds,
                ..
            }
        ));
    }
}
