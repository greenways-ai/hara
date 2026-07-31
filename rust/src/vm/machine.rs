//! The synchronous stack machine.
//!
//! The machine executes validated programs: validation (see
//! `vm::validate`) is the safety gate, and every indexing operation here
//! still converts impossible states into [`VmError`] instead of
//! panicking. The dispatch loop performs no per-instruction heap
//! allocation — primitive and call arguments reuse a scratch buffer —
//! and never looks up locals by name or clones forms.
//!
//! VM closures and static calls stay inside one machine. Call frames and
//! compact scalar slots avoid native callback recursion and boxed integer
//! traffic on the hot path; closures are converted to shared runtime values
//! only when they escape through the public value boundary.
//!
//! Exceptions (milestone 3): every failure routes through
//! [`Machine::raise`], which unwinds to the innermost covering try-table
//! entry. Catch dispatch and binding identity come from the shared
//! `core::catch_matches`/`core::caught_error` boundary, so thrown values
//! and runtime-error strings behave exactly as in the tree evaluator.

use std::collections::HashMap;
use std::rc::Rc;

use super::error::VmError;
use super::frame::Frame;
use super::opcode::Instruction;
use super::program::{FunctionPrototype, Program};
use super::slot::{VmClosure, VmMultiArity, VmSlot};
use crate::core::{
    apply_binary_numbers, apply_binary_primitive, apply_primitive, call_value,
    native_fixed_variadic_function, native_function, with_namespace_registry, Value,
};

#[path = "machine/globals.rs"]
mod globals;

/// Terminal state of a machine run. Suspension variants belong to the
/// later async milestone; adding them does not change instruction
/// dispatch, only the set of exit points.
pub enum VmOutcome {
    Returned(Value),
    Failed(VmError),
}

/// Result of executing one instruction. Call actions only carry their
/// collected operands: the nested machine runs from the thin `run` loop
/// after the fat `dispatch` frame has exited, keeping the native stack
/// cost per guest call level small (issue #223).
enum Dispatch {
    Next(usize),
    Call {
        callee: VmSlot,
        args: Vec<VmSlot>,
    },
    CallStatic {
        prototype: u16,
        args: Vec<VmSlot>,
        captures: Vec<VmSlot>,
    },
    Returned(VmSlot),
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
    free_locals: Vec<Vec<VmSlot>>,
    free_args: Vec<Vec<VmSlot>>,
    vm_globals: HashMap<usize, VmSlot>,
    next_closure_identity: u64,
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
            free_locals: Vec::new(),
            free_args: Vec::new(),
            vm_globals: HashMap::new(),
            next_closure_identity: 0,
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
        mut args: Vec<VmSlot>,
        captures: Vec<VmSlot>,
    ) -> Machine {
        let index = usize::from(prototype);
        let proto = &program.functions[index];
        let mut arity = usize::from(proto.arity);
        if proto.variadic {
            // The rest parameter occupies the slot directly above the
            // fixed parameters (captures sit above it): pack the
            // remaining arguments into a list there, exactly like
            // `call_function` binds `& rest`.
            let fixed = arity.min(args.len());
            let rest = args
                .split_off(fixed)
                .into_iter()
                .map(|value| Machine::into_value(program.clone(), value))
                .collect();
            args.push(Value::List(rest).into());
            arity = fixed + 1;
        }
        Machine {
            frame: Frame::call(usize::from(proto.local_count), arity, args, captures, 0),
            stack: Vec::with_capacity(usize::from(proto.max_stack)),
            program,
            function: index,
            scratch: Vec::new(),
            calls: Vec::new(),
            free_locals: Vec::new(),
            free_args: Vec::new(),
            vm_globals: HashMap::new(),
            next_closure_identity: 0,
            ip: 0,
        }
    }

    fn into_value(program: Rc<Program>, slot: VmSlot) -> Value {
        match slot {
            VmSlot::Number(value) => Value::Number(value),
            VmSlot::Bool(value) => Value::Bool(value),
            VmSlot::Nil => Value::Nil,
            VmSlot::Value(value) => *value,
            VmSlot::InlineClosure { prototype, .. } => Self::closure_value(
                program,
                Rc::new(VmClosure {
                    prototype,
                    captures: Vec::new(),
                }),
            ),
            VmSlot::Closure(closure) => Self::closure_value(program, closure),
            VmSlot::MultiArity(dispatch) => {
                let functions = dispatch
                    .clauses
                    .iter()
                    .cloned()
                    .map(
                        |closure| match Self::closure_value(program.clone(), closure) {
                            Value::Function(function) => function,
                            _ => unreachable!(),
                        },
                    )
                    .collect();
                crate::core::arity_dispatcher(&dispatch.name, functions, false)
            }
        }
    }

    fn callable_key(value: &Value) -> Option<usize> {
        match value {
            Value::Function(function) => Some(Rc::as_ptr(function) as usize),
            _ => None,
        }
    }

    fn remember_vm_global(&mut self, value: &Value, slot: VmSlot) {
        if let Some(key) = Self::callable_key(value) {
            self.vm_globals.insert(key, slot);
        }
    }

    fn closure_value(program: Rc<Program>, closure: Rc<VmClosure>) -> Value {
        let proto = &program.functions[usize::from(closure.prototype)];
        let arity = usize::from(proto.arity);
        let variadic = proto.variadic;
        let name = proto.name.clone();
        let registry = crate::core::namespace_registry().ok();
        let callback = move |args: Vec<Value>| {
            let run = || match Machine::call_slots(
                program.clone(),
                closure.prototype,
                args.into_iter().map(VmSlot::from).collect(),
                closure.captures.clone(),
            )
            .run()
            {
                VmOutcome::Returned(value) => Ok(value),
                VmOutcome::Failed(error) => Err(error.message),
            };
            match &registry {
                Some(registry) => with_namespace_registry(registry, run),
                None => run(),
            }
        };
        if variadic {
            native_fixed_variadic_function(name.as_deref().unwrap_or("fn"), arity, callback)
        } else {
            native_function(name.as_deref().unwrap_or("fn"), arity, callback)
        }
    }

    fn enter_callable(
        &mut self,
        program: &Rc<Program>,
        callee: VmSlot,
        mut args: Vec<VmSlot>,
    ) -> Result<(), String> {
        match callee {
            VmSlot::InlineClosure { prototype, .. } => {
                self.check_arity(program, prototype, args.len())?;
                self.enter_prototype(program, prototype, args, Vec::new());
                Ok(())
            }
            VmSlot::Closure(closure) => {
                self.check_arity(program, closure.prototype, args.len())?;
                self.enter_prototype(program, closure.prototype, args, closure.captures.clone());
                Ok(())
            }
            VmSlot::MultiArity(dispatch) => {
                let closure = dispatch
                    .clauses
                    .iter()
                    .find(|closure| {
                        let proto = &program.functions[usize::from(closure.prototype)];
                        (!proto.variadic && usize::from(proto.arity) == args.len())
                            || (proto.variadic && args.len() >= usize::from(proto.arity))
                    })
                    .cloned()
                    .ok_or_else(|| format!("{} has no arity {}", dispatch.name, args.len()))?;
                self.enter_prototype(program, closure.prototype, args, closure.captures.clone());
                Ok(())
            }
            value => {
                let callee = Self::into_value(program.clone(), value);
                let runtime_args = args
                    .drain(..)
                    .map(|value| Self::into_value(program.clone(), value))
                    .collect();
                self.free_args.push(args);
                let value = call_value(callee, runtime_args)?;
                self.stack.push(value.into());
                self.ip += 1;
                Ok(())
            }
        }
    }

    fn check_arity(&self, program: &Program, prototype: u16, argc: usize) -> Result<(), String> {
        let proto = &program.functions[usize::from(prototype)];
        let arity = usize::from(proto.arity);
        if (!proto.variadic && argc != arity) || (proto.variadic && argc < arity) {
            let expectation = if proto.variadic {
                format!("at least {arity}")
            } else {
                arity.to_string()
            };
            return Err(format!("function expects {expectation} arguments"));
        }
        Ok(())
    }

    fn enter_prototype(
        &mut self,
        program: &Program,
        prototype: u16,
        mut args: Vec<VmSlot>,
        captures: Vec<VmSlot>,
    ) {
        let proto = &program.functions[usize::from(prototype)];
        let mut frame_arity = usize::from(proto.arity);
        if proto.variadic {
            let fixed = frame_arity.min(args.len());
            let rest = args
                .split_off(fixed)
                .into_iter()
                .map(|value| Self::into_value(self.program.clone(), value))
                .collect();
            args.push(Value::List(rest).into());
            frame_arity = fixed + 1;
        }
        let locals = self.free_locals.pop().unwrap_or_default();
        let frame = Frame::call_reusing(
            locals,
            usize::from(proto.local_count),
            frame_arity,
            &mut args,
            captures,
            self.stack.len(),
        );
        self.free_args.push(args);
        let caller = std::mem::replace(&mut self.frame, frame);
        self.calls.push(SavedFrame {
            function: self.function,
            frame: caller,
            call_ip: self.ip,
        });
        self.function = usize::from(prototype);
        self.ip = 0;
    }

    /// Runs the function to completion or failure.
    pub fn run(&mut self) -> VmOutcome {
        let program = self.program.clone();
        // Guest calls use the explicit frame stack below, so instruction
        // dispatch may be inlined without increasing native recursion depth.
        loop {
            let Some(function) = program.functions.get(self.function) else {
                return VmOutcome::Failed(VmError::new("function index out of range", 0, None));
            };
            let Some(instruction) = function.code.get(self.ip) else {
                return VmOutcome::Failed(self.error(function, "instruction pointer out of range"));
            };
            match self.dispatch(&program, function, instruction) {
                Dispatch::Next(ip) => self.ip = ip,
                Dispatch::Call { callee, args } => {
                    if let Err(message) = self.enter_callable(&program, callee, args) {
                        match self.raise(function, message) {
                            Ok(target) => self.ip = target,
                            Err(error) => return VmOutcome::Failed(error),
                        }
                    }
                }
                Dispatch::CallStatic {
                    prototype,
                    args,
                    captures,
                } => self.enter_prototype(&program, prototype, args, captures),
                Dispatch::Returned(value) => {
                    self.stack.truncate(self.frame.base());
                    if let Some(caller) = self.calls.pop() {
                        self.function = caller.function;
                        let completed = std::mem::replace(&mut self.frame, caller.frame);
                        self.free_locals.push(completed.into_locals());
                        self.ip = caller.call_ip + 1;
                        self.stack.push(value);
                    } else {
                        return VmOutcome::Returned(Self::into_value(program.clone(), value));
                    }
                }
                Dispatch::Failed(error) => return VmOutcome::Failed(error),
            }
        }
    }

    /// Executes one instruction, returning where the `run` loop
    /// continues. Call instructions only collect their operands into a
    /// [`Dispatch`] action: the actual call happens in `run` after this
    /// frame has exited. The hot dispatch is inlined into the run loop now
    /// that guest calls no longer recurse through the native stack.
    #[inline(always)]
    fn dispatch(
        &mut self,
        program: &Rc<Program>,
        function: &FunctionPrototype,
        instruction: &Instruction,
    ) -> Dispatch {
        macro_rules! guarded {
            ($expr:expr) => {
                match $expr {
                    Ok(()) => {}
                    Err(message) => match self.raise(function, message) {
                        Ok(target) => return Dispatch::Next(target),
                        Err(error) => return Dispatch::Failed(error),
                    },
                }
            };
        }
        let mut next_ip = self.ip + 1;
        match instruction {
            Instruction::Constant(index) => {
                let Some(value) = program.constants.get(*index as usize) else {
                    match self.raise(function, format!("constant index {index} out of range")) {
                        Ok(target) => return Dispatch::Next(target),
                        Err(error) => return Dispatch::Failed(error),
                    }
                };
                self.stack.push(value.clone().into());
            }
            Instruction::Nil => self.stack.push(VmSlot::Nil),
            Instruction::True => self.stack.push(VmSlot::Bool(true)),
            Instruction::False => self.stack.push(VmSlot::Bool(false)),
            Instruction::LoadLocal(slot) => {
                let Some(value) = self.frame.local(*slot) else {
                    match self.raise(function, format!("local slot {slot} out of range")) {
                        Ok(target) => return Dispatch::Next(target),
                        Err(error) => return Dispatch::Failed(error),
                    }
                };
                self.stack.push(value.clone());
            }
            Instruction::StoreLocal(slot) => {
                let Some(value) = self.stack.pop() else {
                    match self.raise(function, "stack underflow") {
                        Ok(target) => return Dispatch::Next(target),
                        Err(error) => return Dispatch::Failed(error),
                    }
                };
                if !self.frame.store(*slot, value) {
                    match self.raise(function, format!("local slot {slot} out of range")) {
                        Ok(target) => return Dispatch::Next(target),
                        Err(error) => return Dispatch::Failed(error),
                    }
                }
            }
            Instruction::Pop => {
                if self.stack.pop().is_none() {
                    match self.raise(function, "stack underflow") {
                        Ok(target) => return Dispatch::Next(target),
                        Err(error) => return Dispatch::Failed(error),
                    }
                }
            }
            Instruction::Primitive { op, argc } => {
                let argc = usize::from(*argc);
                if self.stack.len() < argc {
                    match self.raise(function, "stack underflow") {
                        Ok(target) => return Dispatch::Next(target),
                        Err(error) => return Dispatch::Failed(error),
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
                            _ if matches!(op, crate::core::Primitive::Equal) => {
                                Ok(VmSlot::Bool(match (&left, &right) {
                                    (VmSlot::Closure(left), VmSlot::Closure(right)) => {
                                        Rc::ptr_eq(left, right)
                                    }
                                    (
                                        VmSlot::InlineClosure { identity: left, .. },
                                        VmSlot::InlineClosure { identity: right, .. },
                                    ) => left == right,
                                    (VmSlot::MultiArity(left), VmSlot::MultiArity(right)) => {
                                        Rc::ptr_eq(left, right)
                                    }
                                    _ => false,
                                }))
                            }
                            _ => Err(format!("{} expects values", op.operator())),
                        },
                    }
                } else {
                    self.scratch.clear();
                    for value in self.stack.split_off(self.stack.len() - argc) {
                        let Some(value) = value.into_runtime_value() else {
                            return Dispatch::Failed(
                                self.error(function, format!("{} expects values", op.operator())),
                            );
                        };
                        self.scratch.push(value);
                    }
                    apply_primitive(*op, &self.scratch).map(VmSlot::from)
                };
                match result {
                    Ok(value) => self.stack.push(value),
                    Err(message) => match self.raise(function, message) {
                        Ok(target) => return Dispatch::Next(target),
                        Err(error) => return Dispatch::Failed(error),
                    },
                }
            }
            Instruction::PrimitiveLocalConst {
                op,
                local,
                constant,
            } => {
                let Some(left) = self.frame.local(*local) else {
                    return Dispatch::Failed(
                        self.error(function, format!("local slot {local} out of range")),
                    );
                };
                let Some(right) = program.constants.get(*constant as usize) else {
                    return Dispatch::Failed(
                        self.error(function, format!("constant index {constant} out of range")),
                    );
                };
                let result = match (left, right) {
                    (VmSlot::Number(left), Value::Number(right)) => {
                        apply_binary_numbers(*op, *left, *right).map(VmSlot::from)
                    }
                    _ => {
                        let Some(left) = left.runtime_value() else {
                            return Dispatch::Failed(
                                self.error(function, format!("{} expects values", op.operator())),
                            );
                        };
                        apply_binary_primitive(*op, &left, right).map(VmSlot::from)
                    }
                };
                match result {
                    Ok(value) => self.stack.push(value),
                    Err(message) => match self.raise(function, message) {
                        Ok(target) => return Dispatch::Next(target),
                        Err(error) => return Dispatch::Failed(error),
                    },
                }
            }
            Instruction::Jump(target) => next_ip = *target as usize,
            Instruction::JumpIfFalse(target) => {
                let Some(condition) = self.stack.pop() else {
                    match self.raise(function, "stack underflow") {
                        Ok(target) => return Dispatch::Next(target),
                        Err(error) => return Dispatch::Failed(error),
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
                guarded!(self.exec_closure(program, *prototype, *captures));
            }
            Instruction::Call { argc } => match self.collect_call(*argc) {
                Ok((callee, args)) => return Dispatch::Call { callee, args },
                Err(message) => match self.raise(function, message) {
                    Ok(target) => return Dispatch::Next(target),
                    Err(error) => return Dispatch::Failed(error),
                },
            },
            Instruction::CallStatic { prototype, argc } => {
                match self.collect_call_static(program, function, *prototype, *argc) {
                    Ok((prototype, args, captures)) => {
                        return Dispatch::CallStatic {
                            prototype,
                            args,
                            captures,
                        };
                    }
                    Err(message) => match self.raise(function, message) {
                        Ok(target) => return Dispatch::Next(target),
                        Err(error) => return Dispatch::Failed(error),
                    },
                }
            }
            Instruction::Throw => {
                let Some(value) = self.stack.pop() else {
                    match self.raise(function, "stack underflow") {
                        Ok(target) => return Dispatch::Next(target),
                        Err(error) => return Dispatch::Failed(error),
                    }
                };
                let message = crate::core::thrown_error(Self::into_value(program.clone(), value));
                match self.raise(function, message) {
                    Ok(target) => return Dispatch::Next(target),
                    Err(error) => return Dispatch::Failed(error),
                }
            }
            Instruction::Rethrow => {
                let Some(value) = self.stack.pop() else {
                    match self.raise(function, "stack underflow") {
                        Ok(target) => return Dispatch::Next(target),
                        Err(error) => return Dispatch::Failed(error),
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
                    Ok(target) => return Dispatch::Next(target),
                    Err(error) => return Dispatch::Failed(error),
                }
            }
            Instruction::GetGlobal(index) => {
                guarded!(self.exec_get_global(program, *index));
            }
            Instruction::DefGlobal { name, metadata } => {
                guarded!(self.exec_def_global(program, *name, *metadata));
            }
            Instruction::SetGlobal(index) => {
                guarded!(self.exec_set_global(program, *index));
            }
            Instruction::VarGlobal(index) => {
                guarded!(self.exec_var_global(program, *index));
            }
            Instruction::DeclareGlobal(index) => {
                guarded!(self.exec_declare_global(program, *index));
            }
            Instruction::DefStruct { name, fields } => {
                guarded!(self.exec_def_struct(program, *name, *fields));
            }
            Instruction::StructField(index) => {
                guarded!(self.exec_struct_field(program, *index));
            }
            Instruction::InstanceOf => {
                guarded!(self.exec_instance_of());
            }
            Instruction::MakeMultiArity { name, count } => {
                guarded!(self.exec_make_multi_arity(program, *name, *count));
            }
            Instruction::Return => {
                return match self.stack.pop() {
                    Some(value) => Dispatch::Returned(value),
                    None => Dispatch::Failed(self.error(function, "stack underflow")),
                };
            }
        }
        Dispatch::Next(next_ip)
    }

    /// Pops the callee and arguments for a Call instruction.
    fn collect_call(&mut self, argc: u8) -> Result<(VmSlot, Vec<VmSlot>), String> {
        let argc = usize::from(argc);
        if self.stack.len() < argc + 1 {
            return Err("stack underflow".to_string());
        }
        let mut args = self.free_args.pop().unwrap_or_default();
        args.extend(self.stack.drain(self.stack.len() - argc..));
        let callee = self.stack.pop().expect("callee checked above");
        Ok((callee, args))
    }

    /// Collects the arguments and capture slots for a CallStatic
    /// instruction; the nested machine runs from the thin `run` loop.
    fn collect_call_static(
        &mut self,
        program: &Program,
        function: &FunctionPrototype,
        prototype: u16,
        argc: u8,
    ) -> Result<(u16, Vec<VmSlot>, Vec<VmSlot>), String> {
        let argc = usize::from(argc);
        if self.stack.len() < argc {
            return Err("stack underflow".to_string());
        }
        let Some(proto) = program.functions.get(usize::from(prototype)) else {
            return Err(format!("callstatic target {prototype} out of range"));
        };
        let capture_count = usize::from(proto.capture_count);
        let mut args = self.free_args.pop().unwrap_or_default();
        args.extend(self.stack.drain(self.stack.len() - argc..));
        let capture_base = usize::from(function.arity) + usize::from(function.variadic);
        let Some(captures) = self.frame.slot_range(capture_base, capture_count) else {
            return Err("capture slots out of range".to_string());
        };
        Ok((prototype, args, captures))
    }

    /// Executes the Closure instruction: builds a plain
    /// `core::Value::Function` whose native callback re-enters
    /// [`Machine::call`]. Kept out of the dispatch loop
    /// (`#[inline(never)]`) so the hot `run` frame stays small — guest
    /// recursion maps onto native stack depth through Call/CallStatic,
    /// and this arm carries large transient locals.
    #[inline(never)]
    fn exec_closure(
        &mut self,
        program: &Rc<Program>,
        prototype: u16,
        captures: u8,
    ) -> Result<(), String> {
        let captures = usize::from(captures);
        if self.stack.len() < captures {
            return Err("stack underflow".to_string());
        }
        let Some(_proto) = program.functions.get(usize::from(prototype)) else {
            return Err(format!("closure prototype {prototype} out of range"));
        };
        if captures == 0 {
            let identity = self.next_closure_identity;
            self.next_closure_identity = self.next_closure_identity.wrapping_add(1);
            self.stack.push(VmSlot::InlineClosure {
                prototype,
                identity,
            });
        } else {
            let captured = self.stack.split_off(self.stack.len() - captures);
            self.stack.push(VmSlot::Closure(Rc::new(VmClosure {
                prototype,
                captures: captured,
            })));
        }
        Ok(())
    }

    /// Routes a failure through the static handler table: the innermost
    /// entry covering the failing instruction gets first catch dispatch,
    /// then outer entries. Returns the instruction to continue at, or the
    /// terminal error when no entry handles it.
    #[cold]
    #[inline(never)]
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
                    if !self
                        .frame
                        .store(value_slot, Value::String(message.clone()).into())
                        || !self.frame.store(flag_slot, Value::Bool(true).into())
                    {
                        return Err(self.error(function, "pending slot out of range"));
                    }
                    return Ok(finally as usize);
                }
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
            let completed = std::mem::replace(&mut self.frame, caller.frame);
            self.free_locals.push(completed.into_locals());
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

/// Reads a string constant (the global-name operands).
fn constant_string(program: &Program, index: u32) -> Option<&str> {
    match program.constants.get(index as usize) {
        Some(Value::String(value)) => Some(value.as_str()),
        _ => None,
    }
}

/// Reads a string-vector constant (defstruct field names).
fn constant_string_vector(program: &Program, index: u32) -> Option<Vec<String>> {
    match program.constants.get(index as usize) {
        Some(Value::Vector(fields)) => fields
            .iter()
            .map(|field| match field {
                Value::String(field) => Some(field.clone()),
                _ => None,
            })
            .collect(),
        _ => None,
    }
}

fn run_entry(program: Rc<Program>) -> Result<Value, VmError> {
    match Machine::entry(program).run() {
        VmOutcome::Returned(value) => Ok(value),
        VmOutcome::Failed(error) => Err(error),
    }
}

/// Executes a validated program's entry function.
///
/// Programs produced by [`crate::vm::compile_source`] are already
/// validated; callers constructing programs by hand must run
/// [`crate::vm::validate`] first. Either way the machine reports
/// [`VmError`] rather than panicking on malformed state. When no
/// namespace registry is active the program runs against a throwaway
/// `user` registry, so same-program `def`/`defn`/`defstruct` can intern
/// without touching caller state (issue #223).
pub fn execute_program(program: Rc<Program>) -> Result<Value, VmError> {
    if crate::core::namespace_registry().is_ok() {
        return run_entry(program);
    }
    let registry = crate::kernel::NamespaceRegistry::new("user");
    with_namespace_registry(&registry, || run_entry(program))
}

/// Executes a program against a caller's namespace registry: globals
/// intern into it and resolve from it, with no env bridge, snapshot, or
/// refresh (issue #223).
pub fn execute_program_with_globals(
    program: Rc<Program>,
    globals: &crate::kernel::NamespaceRegistry<Value>,
) -> Result<Value, VmError> {
    with_namespace_registry(globals, || run_entry(program))
}
