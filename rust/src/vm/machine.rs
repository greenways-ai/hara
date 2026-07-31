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
//!
//! Exceptions (milestone 3): every failure routes through
//! [`Machine::raise`], which unwinds to the innermost covering try-table
//! entry. Catch dispatch and binding identity come from the shared
//! `core::catch_matches`/`core::caught_error` boundary, so thrown values
//! and runtime-error strings behave exactly as in the tree evaluator.

use std::rc::Rc;

use super::error::VmError;
use super::frame::Frame;
use super::opcode::Instruction;
use super::program::{FunctionPrototype, Program};
use crate::core::{apply_binary_primitive, apply_primitive, call_function, native_function, Value};

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
                return VmOutcome::Failed(self.error(function, "instruction pointer out of range"));
            };
            let mut next_ip = self.ip + 1;
            match instruction {
                Instruction::Constant(index) => {
                    let Some(value) = program.constants.get(*index as usize) else {
                        match self.raise(function, format!("constant index {index} out of range")) {
                            Ok(target) => {
                                self.ip = target;
                                continue;
                            }
                            Err(error) => return VmOutcome::Failed(error),
                        }
                    };
                    self.stack.push(value.clone());
                }
                Instruction::Nil => self.stack.push(Value::Nil),
                Instruction::True => self.stack.push(Value::Bool(true)),
                Instruction::False => self.stack.push(Value::Bool(false)),
                Instruction::LoadLocal(slot) => {
                    let Some(value) = self.frame.local(*slot) else {
                        match self.raise(function, format!("local slot {slot} out of range")) {
                            Ok(target) => {
                                self.ip = target;
                                continue;
                            }
                            Err(error) => return VmOutcome::Failed(error),
                        }
                    };
                    self.stack.push(value.clone());
                }
                Instruction::StoreLocal(slot) => {
                    let Some(value) = self.stack.pop() else {
                        match self.raise(function, "stack underflow") {
                            Ok(target) => {
                                self.ip = target;
                                continue;
                            }
                            Err(error) => return VmOutcome::Failed(error),
                        }
                    };
                    if !self.frame.store(*slot, value) {
                        match self.raise(function, format!("local slot {slot} out of range")) {
                            Ok(target) => {
                                self.ip = target;
                                continue;
                            }
                            Err(error) => return VmOutcome::Failed(error),
                        }
                    }
                }
                Instruction::Pop => {
                    if self.stack.pop().is_none() {
                        match self.raise(function, "stack underflow") {
                            Ok(target) => {
                                self.ip = target;
                                continue;
                            }
                            Err(error) => return VmOutcome::Failed(error),
                        }
                    }
                }
                Instruction::Primitive { op, argc } => {
                    let argc = usize::from(*argc);
                    if self.stack.len() < argc {
                        match self.raise(function, "stack underflow") {
                            Ok(target) => {
                                self.ip = target;
                                continue;
                            }
                            Err(error) => return VmOutcome::Failed(error),
                        }
                    }
                    let result = if argc == 2 {
                        let right = self.stack.pop().expect("primitive arity checked above");
                        let left = self.stack.pop().expect("primitive arity checked above");
                        apply_binary_primitive(*op, &left, &right)
                    } else {
                        self.scratch.clear();
                        self.scratch
                            .extend(self.stack.drain(self.stack.len() - argc..));
                        apply_primitive(*op, &self.scratch)
                    };
                    match result {
                        Ok(value) => self.stack.push(value),
                        Err(message) => match self.raise(function, message) {
                            Ok(target) => {
                                self.ip = target;
                                continue;
                            }
                            Err(error) => return VmOutcome::Failed(error),
                        },
                    }
                }
                Instruction::Jump(target) => next_ip = *target as usize,
                Instruction::JumpIfFalse(target) => {
                    let Some(condition) = self.stack.pop() else {
                        match self.raise(function, "stack underflow") {
                            Ok(target) => {
                                self.ip = target;
                                continue;
                            }
                            Err(error) => return VmOutcome::Failed(error),
                        }
                    };
                    if !condition.truthy() {
                        next_ip = *target as usize;
                    }
                }
                Instruction::Closure {
                    prototype,
                    captures,
                } => {
                    let captures = usize::from(*captures);
                    if self.stack.len() < captures {
                        match self.raise(function, "stack underflow") {
                            Ok(target) => {
                                self.ip = target;
                                continue;
                            }
                            Err(error) => return VmOutcome::Failed(error),
                        }
                    }
                    let Some(proto) = program.functions.get(usize::from(*prototype)) else {
                        match self.raise(
                            function,
                            format!("closure prototype {prototype} out of range"),
                        ) {
                            Ok(target) => {
                                self.ip = target;
                                continue;
                            }
                            Err(error) => return VmOutcome::Failed(error),
                        }
                    };
                    let prototype = *prototype;
                    let arity = usize::from(proto.arity);
                    let name = proto.name.clone();
                    let captured = Rc::new(self.stack.split_off(self.stack.len() - captures));
                    let program = program.clone();
                    let closure =
                        native_function(name.as_deref().unwrap_or("fn"), arity, move |args| {
                            match Machine::call(
                                program.clone(),
                                prototype,
                                args,
                                (*captured).clone(),
                            )
                            .run()
                            {
                                VmOutcome::Returned(value) => Ok(value),
                                // The raw message crosses the call boundary:
                                // catch identity (prefix matching against the
                                // thrown-value side channel) depends on it.
                                VmOutcome::Failed(error) => Err(error.message),
                            }
                        });
                    self.stack.push(closure);
                }
                Instruction::Call { argc } => {
                    let argc = usize::from(*argc);
                    if self.stack.len() < argc + 1 {
                        match self.raise(function, "stack underflow") {
                            Ok(target) => {
                                self.ip = target;
                                continue;
                            }
                            Err(error) => return VmOutcome::Failed(error),
                        }
                    }
                    self.scratch.clear();
                    self.scratch
                        .extend(self.stack.drain(self.stack.len() - argc..));
                    let callee = self.stack.pop().expect("callee checked above");
                    match callee {
                        Value::Function(callee) => {
                            match call_function(&callee, std::mem::take(&mut self.scratch)) {
                                Ok(value) => self.stack.push(value),
                                Err(message) => match self.raise(function, message) {
                                    Ok(target) => {
                                        self.ip = target;
                                        continue;
                                    }
                                    Err(error) => return VmOutcome::Failed(error),
                                },
                            }
                        }
                        _ => match self.raise(function, "value is not callable") {
                            Ok(target) => {
                                self.ip = target;
                                continue;
                            }
                            Err(error) => return VmOutcome::Failed(error),
                        },
                    }
                }
                Instruction::CallStatic { prototype, argc } => {
                    let argc = usize::from(*argc);
                    if self.stack.len() < argc {
                        match self.raise(function, "stack underflow") {
                            Ok(target) => {
                                self.ip = target;
                                continue;
                            }
                            Err(error) => return VmOutcome::Failed(error),
                        }
                    }
                    let Some(proto) = program.functions.get(usize::from(*prototype)) else {
                        match self.raise(
                            function,
                            format!("callstatic target {prototype} out of range"),
                        ) {
                            Ok(target) => {
                                self.ip = target;
                                continue;
                            }
                            Err(error) => return VmOutcome::Failed(error),
                        }
                    };
                    let capture_count = usize::from(proto.capture_count);
                    self.scratch.clear();
                    self.scratch
                        .extend(self.stack.drain(self.stack.len() - argc..));
                    let Some(captures) = self
                        .frame
                        .slot_range(usize::from(function.arity), capture_count)
                    else {
                        match self.raise(function, "capture slots out of range") {
                            Ok(target) => {
                                self.ip = target;
                                continue;
                            }
                            Err(error) => return VmOutcome::Failed(error),
                        }
                    };
                    let args = std::mem::take(&mut self.scratch);
                    match Machine::call(program.clone(), *prototype, args, captures).run() {
                        VmOutcome::Returned(value) => self.stack.push(value),
                        VmOutcome::Failed(error) => match self.raise(function, error.message) {
                            Ok(target) => {
                                self.ip = target;
                                continue;
                            }
                            Err(error) => return VmOutcome::Failed(error),
                        },
                    }
                }
                Instruction::Throw => {
                    let Some(value) = self.stack.pop() else {
                        match self.raise(function, "stack underflow") {
                            Ok(target) => {
                                self.ip = target;
                                continue;
                            }
                            Err(error) => return VmOutcome::Failed(error),
                        }
                    };
                    let message = crate::core::thrown_error(value);
                    match self.raise(function, message) {
                        Ok(target) => {
                            self.ip = target;
                            continue;
                        }
                        Err(error) => return VmOutcome::Failed(error),
                    }
                }
                Instruction::Rethrow => {
                    let Some(value) = self.stack.pop() else {
                        match self.raise(function, "stack underflow") {
                            Ok(target) => {
                                self.ip = target;
                                continue;
                            }
                            Err(error) => return VmOutcome::Failed(error),
                        }
                    };
                    let message = match value {
                        Value::String(message) => message,
                        // Defensive: the compiler only emits Rethrow behind a
                        // pending-error flag it set to a message string.
                        _ => "rethrow expects a string message".to_string(),
                    };
                    match self.raise(function, message) {
                        Ok(target) => {
                            self.ip = target;
                            continue;
                        }
                        Err(error) => return VmOutcome::Failed(error),
                    }
                }
                Instruction::Return => {
                    return match self.stack.pop() {
                        Some(value) => VmOutcome::Returned(value),
                        None => VmOutcome::Failed(self.error(function, "stack underflow")),
                    };
                }
            }
            self.ip = next_ip;
        }
    }

    /// Routes a failure through the static handler table: the innermost
    /// entry covering the failing instruction gets first catch dispatch,
    /// then outer entries. Returns the instruction to continue at, or the
    /// terminal error when no entry handles it.
    fn raise(
        &mut self,
        function: &FunctionPrototype,
        message: impl Into<String>,
    ) -> Result<usize, VmError> {
        let message = message.into();
        let error_ip = self.ip;
        for entry in function.handlers.iter().rev() {
            let (start, end) = (entry.start as usize, entry.end as usize);
            if error_ip < start || error_ip >= end {
                continue;
            }
            let depth = usize::from(entry.depth);
            if self.stack.len() < depth {
                return Err(self.error(function, "handler stack depth out of range"));
            }
            for catch in &entry.catches {
                if crate::core::catch_matches(&message, &catch.class) {
                    self.stack.truncate(depth);
                    // The side channel is consumed only now that a match is
                    // decided, exactly like the evaluator's finish_try.
                    let value = crate::core::caught_error(&message);
                    if !self.frame.store(catch.binding, value) {
                        return Err(self.error(function, "catch binding slot out of range"));
                    }
                    return Ok(catch.target as usize);
                }
            }
            if let Some(finally) = entry.finally {
                let (Some(value_slot), Some(flag_slot)) =
                    (entry.pending_value, entry.pending_error)
                else {
                    return Err(self.error(function, "handler pending slots missing"));
                };
                self.stack.truncate(depth);
                // The original message parks in the pending slot; the side
                // channel stays intact so outer catches match the original
                // class and bind the original value after Rethrow.
                if !self.frame.store(value_slot, Value::String(message.clone()))
                    || !self.frame.store(flag_slot, Value::Bool(true))
                {
                    return Err(self.error(function, "pending slot out of range"));
                }
                return Ok(finally as usize);
            }
            // No catch matched and there is no finally: an outer entry may
            // still cover the failure, so keep searching.
        }
        Err(VmError::new(
            message,
            error_ip as u32,
            function.source_map.position(error_ip),
        ))
    }

    fn error(&self, function: &FunctionPrototype, message: impl Into<String>) -> VmError {
        VmError::new(
            message,
            self.ip as u32,
            function.source_map.position(self.ip),
        )
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
