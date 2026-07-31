//! The synchronous stack machine.
//!
//! The machine executes validated programs: validation (see
//! `vm::validate`) is the safety gate, and every indexing operation here
//! still converts impossible states into [`VmError`] instead of
//! panicking. The dispatch loop performs no per-instruction heap
//! allocation — primitive and call arguments reuse a scratch buffer —
//! and never looks up locals by name or clones forms.
//!
//! Function calls run a nested machine per call (closure milestone):
//! `Closure` builds a plain `core::Value::Function` whose native callback
//! re-enters [`Machine::call`], so arity errors, `<fn>` display, and
//! pointer identity all come from the shared value model. An in-machine
//! frame stack is reserved as future performance work.

use std::rc::Rc;

use super::error::VmError;
use super::frame::Frame;
use super::opcode::Instruction;
use super::program::{FunctionPrototype, Program};
use crate::core::{apply_primitive, call_function, native_function, Value};

/// Terminal state of a machine run. Suspension variants belong to the
/// later async milestone; adding them does not change instruction
/// dispatch, only the set of exit points.
pub enum VmOutcome {
    Returned(Value),
    Failed(VmError),
}

/// A synchronous interpreter for one function of a validated [`Program`].
pub struct Machine {
    program: Rc<Program>,
    function: usize,
    frame: Frame,
    stack: Vec<Value>,
    scratch: Vec<Value>,
    ip: usize,
}

impl Machine {
    /// The machine for the program's entry function.
    pub fn entry(program: Rc<Program>) -> Machine {
        let index = usize::from(program.entry);
        let local_count = usize::from(program.functions[index].local_count);
        let max_stack = usize::from(program.functions[index].max_stack);
        Machine {
            program,
            function: index,
            frame: Frame::entry(local_count),
            stack: Vec::with_capacity(max_stack),
            scratch: Vec::new(),
            ip: 0,
        }
    }

    /// The machine for a function call: `args` fill the parameter slots,
    /// `captures` the capture slots directly above them.
    pub fn call(
        program: Rc<Program>,
        prototype: u16,
        args: Vec<Value>,
        captures: Vec<Value>,
    ) -> Machine {
        let index = usize::from(prototype);
        let proto = &program.functions[index];
        Machine {
            frame: Frame::call(
                usize::from(proto.local_count),
                usize::from(proto.arity),
                args,
                captures,
            ),
            stack: Vec::with_capacity(usize::from(proto.max_stack)),
            program,
            function: index,
            scratch: Vec::new(),
            ip: 0,
        }
    }

    /// Runs the function to completion or failure.
    pub fn run(&mut self) -> VmOutcome {
        let program = self.program.clone();
        let Some(function) = program.functions.get(self.function) else {
            return VmOutcome::Failed(VmError::new("function index out of range", 0, None));
        };
        loop {
            let Some(instruction) = function.code.get(self.ip) else {
                return self.failure(function, "instruction pointer out of range");
            };
            let mut next_ip = self.ip + 1;
            match instruction {
                Instruction::Constant(index) => {
                    let Some(value) = program.constants.get(*index as usize) else {
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
                Instruction::Closure { prototype, captures } => {
                    let captures = usize::from(*captures);
                    if self.stack.len() < captures {
                        return self.failure(function, "stack underflow");
                    }
                    let Some(proto) = program.functions.get(usize::from(*prototype)) else {
                        return self
                            .failure(function, format!("closure prototype {prototype} out of range"));
                    };
                    let prototype = *prototype;
                    let arity = usize::from(proto.arity);
                    let name = proto.name.clone();
                    let captured = Rc::new(self.stack.split_off(self.stack.len() - captures));
                    let program = program.clone();
                    let closure = native_function(
                        name.as_deref().unwrap_or("fn"),
                        arity,
                        move |args| {
                            match Machine::call(
                                program.clone(),
                                prototype,
                                args,
                                (*captured).clone(),
                            )
                            .run()
                            {
                                VmOutcome::Returned(value) => Ok(value),
                                VmOutcome::Failed(error) => Err(error.to_string()),
                            }
                        },
                    );
                    self.stack.push(closure);
                }
                Instruction::Call { argc } => {
                    let argc = usize::from(*argc);
                    if self.stack.len() < argc + 1 {
                        return self.failure(function, "stack underflow");
                    }
                    self.scratch.clear();
                    self.scratch.extend(self.stack.drain(self.stack.len() - argc..));
                    let callee = self.stack.pop().expect("callee checked above");
                    match callee {
                        Value::Function(callee) => {
                            match call_function(&callee, std::mem::take(&mut self.scratch)) {
                                Ok(value) => self.stack.push(value),
                                Err(message) => return self.failure(function, message),
                            }
                        }
                        _ => return self.failure(function, "value is not callable"),
                    }
                }
                Instruction::CallStatic { prototype, argc } => {
                    let argc = usize::from(*argc);
                    if self.stack.len() < argc {
                        return self.failure(function, "stack underflow");
                    }
                    let Some(proto) = program.functions.get(usize::from(*prototype)) else {
                        return self
                            .failure(function, format!("callstatic target {prototype} out of range"));
                    };
                    let capture_count = usize::from(proto.capture_count);
                    self.scratch.clear();
                    self.scratch.extend(self.stack.drain(self.stack.len() - argc..));
                    let Some(captures) = self
                        .frame
                        .slot_range(usize::from(function.arity), capture_count)
                    else {
                        return self.failure(function, "capture slots out of range");
                    };
                    let args = std::mem::take(&mut self.scratch);
                    match Machine::call(program.clone(), *prototype, args, captures).run() {
                        VmOutcome::Returned(value) => self.stack.push(value),
                        VmOutcome::Failed(error) => return self.failure(function, error.message),
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
pub fn execute_program(program: Rc<Program>) -> Result<Value, VmError> {
    match Machine::entry(program).run() {
        VmOutcome::Returned(value) => Ok(value),
        VmOutcome::Failed(error) => Err(error),
    }
}
