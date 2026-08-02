use super::trace_ir::{Trace, TraceOp, TraceValue};
use crate::core::{Primitive, Value};
use crate::vm::{Instruction, Program};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordError {
    InvalidRange,
    InvalidStack,
    TooLong,
    UnsupportedInstruction(u32),
    UnsupportedConstant(u32),
    UnsupportedLocal(u16),
}

pub struct TraceRecorder {
    max_operations: usize,
}

impl TraceRecorder {
    pub fn new(max_operations: usize) -> Self {
        Self { max_operations }
    }

    pub fn record_loop(
        &self,
        program: &Program,
        function: u16,
        header: u32,
        backedge: u32,
        locals: &[TraceValue],
    ) -> Result<Trace, RecordError> {
        let path = (header..=backedge).collect::<Vec<_>>();
        self.record_path(program, function, header, &path, locals)
    }

    /// Lowers the concrete instruction path observed by the VM. Forward
    /// branches disappear into the linear trace; their observed direction is
    /// retained as a guard.
    pub fn record_path(
        &self,
        program: &Program,
        function: u16,
        header: u32,
        path: &[u32],
        locals: &[TraceValue],
    ) -> Result<Trace, RecordError> {
        let prototype = program
            .functions
            .get(function as usize)
            .ok_or(RecordError::InvalidRange)?;
        if path.first() != Some(&header) || path.is_empty() {
            return Err(RecordError::InvalidRange);
        }
        let mut operations = Vec::new();
        let mut vectors = Vec::new();
        for (index, absolute) in path.iter().copied().enumerate() {
            let instruction = prototype
                .code
                .get(absolute as usize)
                .ok_or(RecordError::InvalidRange)?;
            let next = path.get(index + 1).copied().unwrap_or(header);
            match instruction {
                Instruction::LoadLocal(local) => {
                    operations.push(match locals.get(usize::from(*local)) {
                        Some(TraceValue::I64(_)) => TraceOp::GuardLocalI64 { local: *local },
                        Some(TraceValue::Bool(_)) => TraceOp::GuardLocalBool { local: *local },
                        Some(TraceValue::Nil) => TraceOp::GuardLocalNil { local: *local },
                        Some(TraceValue::Indexed(value)) if numeric_vector(value).is_some() => {
                            TraceOp::GuardLocalVectorI64 { local: *local }
                        }
                        _ => return Err(RecordError::UnsupportedLocal(*local)),
                    });
                    operations.push(TraceOp::LoadLocal { local: *local });
                }
                Instruction::StoreLocal(local) => {
                    operations.push(TraceOp::StoreLocal { local: *local })
                }
                Instruction::Constant(index) => match program.constants.get(*index as usize) {
                    Some(Value::Number(value)) => operations.push(TraceOp::ConstantI64(*value)),
                    Some(Value::Bool(value)) => operations.push(TraceOp::ConstantBool(*value)),
                    Some(Value::Nil) => operations.push(TraceOp::ConstantNil),
                    Some(value @ (Value::Tuple(_) | Value::Vector(_))) => {
                        let values = numeric_vector(value)
                            .ok_or(RecordError::UnsupportedConstant(*index))?;
                        let vector =
                            u16::try_from(vectors.len()).map_err(|_| RecordError::TooLong)?;
                        vectors.push(values);
                        operations.push(TraceOp::ConstantVectorI64 { vector });
                    }
                    _ => return Err(RecordError::UnsupportedConstant(*index)),
                },
                Instruction::Nil => operations.push(TraceOp::ConstantNil),
                Instruction::True => operations.push(TraceOp::ConstantBool(true)),
                Instruction::False => operations.push(TraceOp::ConstantBool(false)),
                Instruction::BuildVector(count) => {
                    let count = usize::from(*count);
                    if operations.len() < count {
                        return Err(RecordError::InvalidStack);
                    }
                    let start = operations.len() - count;
                    let values = operations[start..]
                        .iter()
                        .map(|operation| match operation {
                            TraceOp::ConstantI64(value) => Some(*value),
                            _ => None,
                        })
                        .collect::<Option<Vec<_>>>()
                        .ok_or(RecordError::UnsupportedInstruction(absolute))?;
                    operations.truncate(start);
                    let vector = u16::try_from(vectors.len()).map_err(|_| RecordError::TooLong)?;
                    vectors.push(values);
                    operations.push(TraceOp::ConstantVectorI64 { vector });
                }
                Instruction::Primitive { op, argc: 2 }
                    if matches!(
                        op,
                        Primitive::Add
                            | Primitive::Subtract
                            | Primitive::Multiply
                            | Primitive::Divide
                            | Primitive::Remainder
                            | Primitive::Less
                            | Primitive::LessOrEqual
                            | Primitive::Greater
                            | Primitive::GreaterOrEqual
                            | Primitive::Equal
                    ) =>
                {
                    operations.push(TraceOp::BinaryI64(*op));
                }
                Instruction::Primitive { op, argc: 1 }
                    if matches!(
                        op,
                        Primitive::Count | Primitive::First | Primitive::Rest | Primitive::Second
                    ) =>
                {
                    operations.push(match op {
                        Primitive::Count => TraceOp::VectorCountI64,
                        Primitive::First => TraceOp::VectorFirstI64,
                        Primitive::Rest => TraceOp::VectorRestI64,
                        Primitive::Second => TraceOp::VectorSecondI64,
                        _ => unreachable!(),
                    });
                }
                Instruction::Primitive {
                    op: Primitive::Nth,
                    argc: 2,
                } => {
                    operations.push(TraceOp::VectorNthI64);
                }
                Instruction::PrimitiveLocalConst {
                    op,
                    local,
                    constant,
                } => {
                    let value = match program.constants.get(*constant as usize) {
                        Some(Value::Number(value)) => *value,
                        _ => return Err(RecordError::UnsupportedConstant(*constant)),
                    };
                    operations.push(match (op, locals.get(usize::from(*local))) {
                        (Primitive::Nth, Some(TraceValue::Indexed(vector)))
                            if numeric_vector(vector).is_some() =>
                        {
                            TraceOp::GuardLocalVectorI64 { local: *local }
                        }
                        (_, Some(TraceValue::I64(_))) if binary_i64(*op) => {
                            TraceOp::GuardLocalI64 { local: *local }
                        }
                        _ => return Err(RecordError::UnsupportedLocal(*local)),
                    });
                    operations.push(TraceOp::LoadLocal { local: *local });
                    operations.push(TraceOp::ConstantI64(value));
                    operations.push(if *op == Primitive::Nth {
                        TraceOp::VectorNthI64
                    } else {
                        TraceOp::BinaryI64(*op)
                    });
                }
                Instruction::JumpIfFalse(target) => {
                    let expected = next != *target;
                    if next != absolute + 1 && next != *target {
                        return Err(RecordError::InvalidRange);
                    }
                    operations.push(TraceOp::GuardTruthy { expected })
                }
                Instruction::Pop => operations.push(TraceOp::Pop),
                Instruction::Jump(target) if *target == next => {
                    if *target == header {
                        operations.push(TraceOp::LoopBackedge)
                    }
                }
                _ => return Err(RecordError::UnsupportedInstruction(absolute)),
            }
            if operations.len() > self.max_operations {
                return Err(RecordError::TooLong);
            }
        }
        if !matches!(operations.last(), Some(TraceOp::LoopBackedge)) {
            return Err(RecordError::InvalidRange);
        }
        if !valid_types(&operations, locals) {
            return Err(RecordError::InvalidStack);
        }
        Ok(Trace {
            function,
            header,
            resume_ip: header,
            operations,
            vectors,
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TraceType {
    I64,
    Bool,
    Nil,
    Vector,
    Slice,
}

fn valid_types(operations: &[TraceOp], entry_locals: &[TraceValue]) -> bool {
    use TraceType::*;
    let mut locals = entry_locals
        .iter()
        .enumerate()
        .filter_map(|(index, value)| {
            let kind = match value {
                TraceValue::I64(_) => I64,
                TraceValue::Bool(_) => Bool,
                TraceValue::Nil => Nil,
                TraceValue::Indexed(value) if numeric_vector(value).is_some() => Vector,
                _ => return None,
            };
            Some((index as u16, kind))
        })
        .collect::<std::collections::HashMap<_, _>>();
    let entry_types = locals.clone();
    let mut stack = Vec::new();
    for operation in operations {
        match *operation {
            TraceOp::GuardLocalI64 { local } => {
                locals.insert(local, I64);
            }
            TraceOp::GuardLocalBool { local } => {
                locals.insert(local, Bool);
            }
            TraceOp::GuardLocalNil { local } => {
                locals.insert(local, Nil);
            }
            TraceOp::GuardLocalVectorI64 { local } => {
                locals.insert(local, Vector);
            }
            TraceOp::LoadLocal { local } => {
                let Some(kind) = locals.get(&local).copied() else {
                    return false;
                };
                stack.push(kind);
            }
            TraceOp::ConstantI64(_) => stack.push(I64),
            TraceOp::ConstantBool(_) => stack.push(Bool),
            TraceOp::ConstantNil => stack.push(Nil),
            TraceOp::ConstantVectorI64 { .. } => stack.push(Vector),
            TraceOp::BinaryI64(op) => {
                if stack.pop() != Some(I64) || stack.pop() != Some(I64) {
                    return false;
                }
                stack.push(
                    if matches!(
                        op,
                        Primitive::Equal
                            | Primitive::Less
                            | Primitive::LessOrEqual
                            | Primitive::Greater
                            | Primitive::GreaterOrEqual
                    ) {
                        Bool
                    } else {
                        I64
                    },
                );
            }
            TraceOp::VectorCountI64 => {
                if !matches!(stack.pop(), Some(Vector | Slice)) {
                    return false;
                }
                stack.push(I64);
            }
            TraceOp::VectorFirstI64 | TraceOp::VectorSecondI64 => {
                if !matches!(stack.pop(), Some(Vector | Slice)) {
                    return false;
                }
                stack.push(I64);
            }
            TraceOp::VectorRestI64 => {
                if !matches!(stack.pop(), Some(Vector | Slice)) {
                    return false;
                }
                stack.push(Slice);
            }
            TraceOp::VectorNthI64 => {
                if stack.pop() != Some(I64) || !matches!(stack.pop(), Some(Vector | Slice)) {
                    return false;
                }
                stack.push(I64);
            }
            TraceOp::StoreLocal { local } => {
                let Some(kind) = stack.pop() else {
                    return false;
                };
                if matches!(kind, Vector | Slice)
                    || entry_types.get(&local).is_some_and(|entry| *entry != kind)
                    || !entry_types.contains_key(&local)
                {
                    return false;
                }
                locals.insert(local, kind);
            }
            TraceOp::GuardTruthy { .. } => {
                if !matches!(stack.pop(), Some(Bool | Nil)) {
                    return false;
                }
            }
            TraceOp::Pop => {
                if stack.pop().is_none() {
                    return false;
                }
            }
            TraceOp::LoopBackedge => {
                if !stack.is_empty() {
                    return false;
                }
            }
        }
    }
    stack.is_empty()
}

fn binary_i64(op: Primitive) -> bool {
    matches!(
        op,
        Primitive::Add
            | Primitive::Subtract
            | Primitive::Multiply
            | Primitive::Divide
            | Primitive::Remainder
            | Primitive::Less
            | Primitive::LessOrEqual
            | Primitive::Greater
            | Primitive::GreaterOrEqual
            | Primitive::Equal
    )
}

fn numeric_vector(value: &Value) -> Option<Vec<i64>> {
    let values: Box<dyn Iterator<Item = &Value> + '_> = match value {
        Value::Tuple(values) => Box::new(values.iter()),
        Value::Vector(values) => Box::new(values.iter()),
        _ => return None,
    };
    values
        .map(|value| match value {
            Value::Number(value) => Some(*value),
            _ => None,
        })
        .collect()
}
