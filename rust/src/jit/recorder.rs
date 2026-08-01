use super::trace_ir::{Trace, TraceOp};
use crate::core::{Primitive, Value};
use crate::vm::{Instruction, Program};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordError {
    InvalidRange,
    TooLong,
    UnsupportedInstruction(u32),
    UnsupportedConstant(u32),
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
    ) -> Result<Trace, RecordError> {
        let prototype = program.functions.get(function as usize).ok_or(RecordError::InvalidRange)?;
        if header > backedge || backedge as usize >= prototype.code.len() {
            return Err(RecordError::InvalidRange);
        }
        let mut operations = Vec::new();
        for (ip, instruction) in prototype.code[header as usize..=backedge as usize].iter().enumerate() {
            let absolute = header + ip as u32;
            match instruction {
                Instruction::LoadLocal(local) => {
                    operations.push(TraceOp::GuardLocalI64 { local: *local });
                    operations.push(TraceOp::LoadLocal { local: *local });
                }
                Instruction::StoreLocal(local) => operations.push(TraceOp::StoreLocal { local: *local }),
                Instruction::Constant(index) => match program.constants.get(*index as usize) {
                    Some(Value::Number(value)) => operations.push(TraceOp::ConstantI64(*value)),
                    _ => return Err(RecordError::UnsupportedConstant(*index)),
                },
                Instruction::Primitive { op, argc: 2 }
                    if matches!(op, Primitive::Add | Primitive::Subtract | Primitive::Multiply | Primitive::Remainder | Primitive::Less | Primitive::LessOrEqual | Primitive::Greater | Primitive::GreaterOrEqual | Primitive::Equal) =>
                {
                    operations.push(TraceOp::BinaryI64(*op));
                }
                Instruction::PrimitiveLocalConst { op, local, constant } => {
                    let value = match program.constants.get(*constant as usize) {
                        Some(Value::Number(value)) => *value,
                        _ => return Err(RecordError::UnsupportedConstant(*constant)),
                    };
                    operations.push(TraceOp::GuardLocalI64 { local: *local });
                    operations.push(TraceOp::LoadLocal { local: *local });
                    operations.push(TraceOp::ConstantI64(value));
                    operations.push(TraceOp::BinaryI64(*op));
                }
                Instruction::JumpIfFalse(_) => operations.push(TraceOp::GuardTruthy { expected: true }),
                Instruction::Pop => operations.push(TraceOp::Pop),
                Instruction::Jump(target) if *target == header => operations.push(TraceOp::LoopBackedge),
                _ => return Err(RecordError::UnsupportedInstruction(absolute)),
            }
            if operations.len() > self.max_operations {
                return Err(RecordError::TooLong);
            }
        }
        if !matches!(operations.last(), Some(TraceOp::LoopBackedge)) {
            return Err(RecordError::InvalidRange);
        }
        Ok(Trace { function, header, resume_ip: header, operations })
    }
}
