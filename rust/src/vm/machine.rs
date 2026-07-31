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
use super::slot::{VmClosure, VmSlot};
use crate::core::{
    apply_binary_numbers, apply_binary_primitive, apply_primitive, call_function, native_function,
    Value,
};

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
    stack: Vec<VmSlot>,
    scratch: Vec<Value>,
    calls: Vec<SavedFrame>,
    ip: usize,
}

struct SavedFrame {
    function: usize,
    frame: Frame,
    call_ip: usize,
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
            calls: Vec::new(),
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
        Machine::call_slots(
            program,
            prototype,
            args.into_iter().map(VmSlot::from).collect(),
            captures.into_iter().map(VmSlot::from).collect(),
        )
    }

    fn call_slots(
        program: Rc<Program>,
        prototype: u16,
        args: Vec<VmSlot>,
        captures: Vec<VmSlot>,
    ) -> Machine {
        let index = usize::from(prototype);
        let proto = &program.functions[index];
        Machine {
            frame: Frame::call(
                usize::from(proto.local_count),
                usize::from(proto.arity),
                args,
                captures,
                0,
            ),
            stack: Vec::with_capacity(usize::from(proto.max_stack)),
            program,
            function: index,
            scratch: Vec::new(),
            calls: Vec::new(),
            ip: 0,
        }
    }

    fn into_value(program: Rc<Program>, slot: VmSlot) -> Value {
        match slot {
            VmSlot::Number(value) => Value::Number(value),
            VmSlot::Bool(value) => Value::Bool(value),
            VmSlot::Nil => Value::Nil,
            VmSlot::Value(value) => *value,
            VmSlot::Closure(closure) => {
                let proto = &program.functions[usize::from(closure.prototype)];
                let arity = usize::from(proto.arity);
                let name = proto.name.clone();
                native_function(name.as_deref().unwrap_or("fn"), arity, move |args| {
                    match Machine::call_slots(
                        program.clone(),
                        closure.prototype,
                        args.into_iter().map(VmSlot::from).collect(),
                        closure.captures.clone(),
                    )
                    .run()
                    {
                        VmOutcome::Returned(value) => Ok(value),
                        VmOutcome::Failed(error) => Err(error.message),
                    }
                })
            }
        }
    }

    /// Runs the function to completion or failure.
    pub fn run(&mut self) -> VmOutcome {
        let program = self.program.clone();
        loop {
            let Some(function) = program.functions.get(self.function) else {
                return VmOutcome::Failed(VmError::new("function index out of range", 0, None));
            };
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
                    self.stack.push(value.clone().into());
                }
                Instruction::Nil => self.stack.push(Value::Nil.into()),
                Instruction::True => self.stack.push(Value::Bool(true).into()),
                Instruction::False => self.stack.push(Value::Bool(false).into()),
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
                        match (&left, &right) {
                            (VmSlot::Number(left), VmSlot::Number(right)) => {
                                apply_binary_numbers(*op, *left, *right).map(VmSlot::from)
                            }
                            _ => match (left.runtime_value(), right.runtime_value()) {
                            (Some(left), Some(right)) => {
                                apply_binary_primitive(*op, &left, &right).map(VmSlot::from)
                            }
                            _ if matches!(op, crate::core::Primitive::Equal) => Ok(VmSlot::Bool(
                                matches!(
                                    (&left, &right),
                                    (VmSlot::Closure(left), VmSlot::Closure(right))
                                        if Rc::ptr_eq(left, right)
                                ),
                            )),
                            _ => Err(format!("{} expects numbers", op.operator())),
                            },
                        }
                    } else {
                        self.scratch.clear();
                        let arguments = self.stack.split_off(self.stack.len() - argc);
                        for argument in arguments {
                            let Some(value) = argument.runtime_value() else {
                                return VmOutcome::Failed(self.error(
                                    function,
                                    format!("{} expects values", op.operator()),
                                ));
                            };
                            self.scratch.push(value);
                        }
                        apply_primitive(*op, &self.scratch).map(VmSlot::from)
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
                    let Some(_proto) = program.functions.get(usize::from(*prototype)) else {
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
                    let captured = self.stack.split_off(self.stack.len() - captures);
                    self.stack.push(VmSlot::Closure(Rc::new(VmClosure {
                        prototype,
                        captures: captured,
                    })));
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
                    let args = self.stack.split_off(self.stack.len() - argc);
                    let callee = self.stack.pop().expect("callee checked above");
                    match callee {
                        VmSlot::Closure(closure) => {
                            let proto = &program.functions[usize::from(closure.prototype)];
                            if argc != usize::from(proto.arity) {
                                match self.raise(
                                    function,
                                    format!(
                                        "function expects {} arguments",
                                        proto.arity
                                    ),
                                ) {
                                    Ok(target) => {
                                        self.ip = target;
                                        continue;
                                    }
                                    Err(error) => return VmOutcome::Failed(error),
                                }
                            }
                            let base = self.stack.len();
                            let frame = Frame::call(
                                usize::from(proto.local_count),
                                usize::from(proto.arity),
                                args,
                                closure.captures.clone(),
                                base,
                            );
                            let caller = std::mem::replace(&mut self.frame, frame);
                            self.calls.push(SavedFrame {
                                function: self.function,
                                frame: caller,
                                call_ip: self.ip,
                            });
                            self.function = usize::from(closure.prototype);
                            self.ip = 0;
                            continue;
                        }
                        VmSlot::Value(value) if matches!(&*value, Value::Function(_)) => {
                            let Value::Function(callee) = *value else {
                                unreachable!()
                            };
                            let args = args
                                .into_iter()
                                .map(|slot| Self::into_value(program.clone(), slot))
                                .collect();
                            match call_function(&callee, args) {
                                Ok(value) => self.stack.push(value.into()),
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
                    let args = self.stack.split_off(self.stack.len() - argc);
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
                    let base = self.stack.len();
                    let callee = Frame::call(
                        usize::from(proto.local_count),
                        usize::from(proto.arity),
                        args,
                        captures,
                        base,
                    );
                    let caller = std::mem::replace(&mut self.frame, callee);
                    self.calls.push(SavedFrame {
                        function: self.function,
                        frame: caller,
                        call_ip: self.ip,
                    });
                    self.function = usize::from(*prototype);
                    self.ip = 0;
                    continue;
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
                    let message = crate::core::thrown_error(Self::into_value(program.clone(), value));
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
                        VmSlot::Value(value) => match *value {
                            Value::String(message) => message,
                            _ => "rethrow expects a string message".to_string(),
                        },
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
                    let Some(value) = self.stack.pop() else {
                        return VmOutcome::Failed(self.error(function, "stack underflow"));
                    };
                    self.stack.truncate(self.frame.base());
                    if let Some(caller) = self.calls.pop() {
                        self.function = caller.function;
                        self.frame = caller.frame;
                        self.ip = caller.call_ip + 1;
                        self.stack.push(value);
                        continue;
                    }
                    return VmOutcome::Returned(Self::into_value(program.clone(), value));
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
        _function: &FunctionPrototype,
        message: impl Into<String>,
    ) -> Result<usize, VmError> {
        let message = message.into();
        loop {
            let function = &self.program.functions[self.function];
            let error_ip = self.ip;
            for entry in function.handlers.iter().rev() {
                let (start, end) = (entry.start as usize, entry.end as usize);
                if error_ip < start || error_ip >= end {
                    continue;
                }
                let depth = self.frame.base() + usize::from(entry.depth);
                if self.stack.len() < depth {
                    return Err(self.error(function, "handler stack depth out of range"));
                }
                for catch in &entry.catches {
                    if crate::core::catch_matches(&message, &catch.class) {
                        self.stack.truncate(depth);
                        // The side channel is consumed only now that a match is
                        // decided, exactly like the evaluator's finish_try.
                        let value = crate::core::caught_error(&message);
                        if !self.frame.store(catch.binding, value.into()) {
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
                    if !self
                        .frame
                        .store(value_slot, Value::String(message.clone()).into())
                        || !self.frame.store(flag_slot, Value::Bool(true).into())
                    {
                        return Err(self.error(function, "pending slot out of range"));
                    }
                    return Ok(finally as usize);
                }
                // No catch matched and there is no finally: an outer entry may
                // still cover the failure, so keep searching.
            }
            let Some(caller) = self.calls.pop() else {
                return Err(VmError::new(
                    message,
                    error_ip as u32,
                    function.source_map.position(error_ip),
                ));
            };
            self.stack.truncate(self.frame.base());
            self.function = caller.function;
            self.frame = caller.frame;
            self.ip = caller.call_ip;
        }
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
