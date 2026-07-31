//! The synchronous stack machine.
//!
//! The machine executes validated programs: validation (see
//! `vm::validate`) is the safety gate, and every indexing operation here
//! still converts impossible states into [`VmError`] instead of
//! panicking. The dispatch loop performs no per-instruction heap
//! allocation — primitive arguments reuse a scratch buffer — and never
//! looks up locals by name or clones forms.

use super::error::VmError;
use super::frame::Frame;
use super::opcode::Instruction;
use super::program::{FunctionPrototype, Program};
use crate::core::{apply_primitive, Value};

/// Terminal state of a machine run. Suspension variants belong to the
/// later async milestone; adding them does not change instruction
/// dispatch, only the set of exit points.
pub enum VmOutcome {
    Returned(Value),
    Failed(VmError),
}

/// A synchronous interpreter for one validated [`Program`].
pub struct Machine<'a> {
    program: &'a Program,
    frame: Frame,
    stack: Vec<Value>,
    scratch: Vec<Value>,
    ip: usize,
}

impl<'a> Machine<'a> {
    pub fn new(program: &'a Program) -> Machine<'a> {
        let entry = program.entry_function();
        Machine {
            program,
            frame: Frame::entry(usize::from(entry.local_count)),
            stack: Vec::with_capacity(usize::from(entry.max_stack)),
            scratch: Vec::new(),
            ip: 0,
        }
    }

    /// Runs the entry function to completion or failure.
    pub fn run(&mut self) -> VmOutcome {
        let function = self.program.entry_function();
        loop {
            let Some(instruction) = function.code.get(self.ip) else {
                return self.failure(function, "instruction pointer out of range");
            };
            let mut next_ip = self.ip + 1;
            match instruction {
                Instruction::Constant(index) => {
                    let Some(value) = self.program.constants.get(*index as usize) else {
                        return self.failure(function, format!("constant index {index} out of range"));
                    };
                    self.stack.push(value.clone());
                }
                Instruction::Nil => self.stack.push(Value::Nil),
                Instruction::True => self.stack.push(Value::Bool(true)),
                Instruction::False => self.stack.push(Value::Bool(false)),
                Instruction::LoadLocal(slot) => {
                    let Some(value) = self.frame.local(*slot) else {
                        return self.failure(function, format!("local slot {slot} out of range"));
                    };
                    self.stack.push(value.clone());
                }
                Instruction::StoreLocal(slot) => {
                    let Some(value) = self.stack.pop() else {
                        return self.failure(function, "stack underflow");
                    };
                    if !self.frame.store(*slot, value) {
                        return self.failure(function, format!("local slot {slot} out of range"));
                    }
                }
                Instruction::Pop => {
                    if self.stack.pop().is_none() {
                        return self.failure(function, "stack underflow");
                    }
                }
                Instruction::Primitive { op, argc } => {
                    let argc = usize::from(*argc);
                    if self.stack.len() < argc {
                        return self.failure(function, "stack underflow");
                    }
                    self.scratch.clear();
                    self.scratch.extend(self.stack.drain(self.stack.len() - argc..));
                    match apply_primitive(*op, &self.scratch) {
                        Ok(value) => self.stack.push(value),
                        Err(message) => return self.failure(function, message),
                    }
                }
                Instruction::Jump(target) => next_ip = *target as usize,
                Instruction::JumpIfFalse(target) => {
                    let Some(condition) = self.stack.pop() else {
                        return self.failure(function, "stack underflow");
                    };
                    if !condition.truthy() {
                        next_ip = *target as usize;
                    }
                }
                Instruction::Return => {
                    return match self.stack.pop() {
                        Some(value) => VmOutcome::Returned(value),
                        None => self.failure(function, "stack underflow"),
                    };
                }
            }
            self.ip = next_ip;
        }
    }

    fn failure(&self, function: &FunctionPrototype, message: impl Into<String>) -> VmOutcome {
        VmOutcome::Failed(VmError::new(
            message,
            self.ip as u32,
            function.source_map.position(self.ip),
        ))
    }
}

/// Executes a validated program's entry function.
///
/// Programs produced by [`crate::vm::compile_source`] are already
/// validated; callers constructing programs by hand must run
/// [`crate::vm::validate`] first. Either way the machine reports
/// [`VmError`] rather than panicking on malformed state.
pub fn execute_program(program: &Program) -> Result<Value, VmError> {
    match Machine::new(program).run() {
        VmOutcome::Returned(value) => Ok(value),
        VmOutcome::Failed(error) => Err(error),
    }
}
