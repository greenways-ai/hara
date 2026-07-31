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
use crate::core::{
    apply_binary_primitive, apply_primitive, call_value, native_fixed_variadic_function,
    native_function, with_namespace_registry, Value,
};
use crate::lang::data::Symbol;

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
    Call { callee: Value, args: Vec<Value> },
    CallStatic {
        prototype: u16,
        args: Vec<Value>,
        captures: Vec<Value>,
    },
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
        mut args: Vec<Value>,
        captures: Vec<Value>,
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
            let rest = Value::List(args.split_off(fixed).into_iter().collect());
            args.push(rest);
            arity = fixed + 1;
        }
        Machine {
            frame: Frame::call(usize::from(proto.local_count), arity, args, captures),
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
        // This loop stays thin on purpose: every instruction executes in
        // [`Machine::dispatch`], whose frame has exited before any nested
        // machine runs. Guest recursion maps onto native stack depth
        // through the call actions below, so the locals of the fat
        // dispatch match must not live in this frame (issue #223).
        loop {
            let Some(instruction) = function.code.get(self.ip) else {
                return VmOutcome::Failed(self.error(function, "instruction pointer out of range"));
            };
            match self.dispatch(&program, function, instruction) {
                Dispatch::Next(ip) => self.ip = ip,
                Dispatch::Call { callee, args } => match call_value(callee, args) {
                    Ok(value) => {
                        self.stack.push(value);
                        self.ip += 1;
                    }
                    Err(message) => match self.raise(function, message) {
                        Ok(target) => self.ip = target,
                        Err(error) => return VmOutcome::Failed(error),
                    },
                },
                Dispatch::CallStatic {
                    prototype,
                    args,
                    captures,
                } => match Machine::call(program.clone(), prototype, args, captures).run() {
                    VmOutcome::Returned(value) => {
                        self.stack.push(value);
                        self.ip += 1;
                    }
                    VmOutcome::Failed(error) => match self.raise(function, error.message) {
                        Ok(target) => self.ip = target,
                        Err(error) => return VmOutcome::Failed(error),
                    },
                },
                Dispatch::Returned(value) => return VmOutcome::Returned(value),
                Dispatch::Failed(error) => return VmOutcome::Failed(error),
            }
        }
    }

    /// Executes one instruction, returning where the `run` loop
    /// continues. Call instructions only collect their operands into a
    /// [`Dispatch`] action: the actual call happens in `run` after this
    /// (fat) frame has exited. `#[inline(never)]` guarantees that split
    /// in every build profile.
    #[inline(never)]
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
                self.stack.push(value.clone());
            }
            Instruction::Nil => self.stack.push(Value::Nil),
            Instruction::True => self.stack.push(Value::Bool(true)),
            Instruction::False => self.stack.push(Value::Bool(false)),
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
            Instruction::Closure { prototype, captures } => {
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
                let message = crate::core::thrown_error(value);
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
                    Value::String(message) => message,
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
    fn collect_call(&mut self, argc: u8) -> Result<(Value, Vec<Value>), String> {
        let argc = usize::from(argc);
        if self.stack.len() < argc + 1 {
            return Err("stack underflow".to_string());
        }
        self.scratch.clear();
        self.scratch.extend(self.stack.drain(self.stack.len() - argc..));
        let callee = self.stack.pop().expect("callee checked above");
        Ok((callee, std::mem::take(&mut self.scratch)))
    }

    /// Collects the arguments and capture slots for a CallStatic
    /// instruction; the nested machine runs from the thin `run` loop.
    fn collect_call_static(
        &mut self,
        program: &Program,
        function: &FunctionPrototype,
        prototype: u16,
        argc: u8,
    ) -> Result<(u16, Vec<Value>, Vec<Value>), String> {
        let argc = usize::from(argc);
        if self.stack.len() < argc {
            return Err("stack underflow".to_string());
        }
        let Some(proto) = program.functions.get(usize::from(prototype)) else {
            return Err(format!("callstatic target {prototype} out of range"));
        };
        let capture_count = usize::from(proto.capture_count);
        self.scratch.clear();
        self.scratch.extend(self.stack.drain(self.stack.len() - argc..));
        let capture_base = usize::from(function.arity) + usize::from(function.variadic);
        let Some(captures) = self.frame.slot_range(capture_base, capture_count) else {
            return Err("capture slots out of range".to_string());
        };
        let args = std::mem::take(&mut self.scratch);
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
        let Some(proto) = program.functions.get(usize::from(prototype)) else {
            return Err(format!("closure prototype {prototype} out of range"));
        };
        let arity = usize::from(proto.arity);
        let variadic = proto.variadic;
        let name = proto.name.clone();
        let captured = Rc::new(self.stack.split_off(self.stack.len() - captures));
        let program = program.clone();
        // Globals resolve at call time: the closure carries the
        // registry it was created under, like evaluator closures
        // carry their captured environment (issue #223).
        let registry = crate::core::namespace_registry().ok();
        let callback = move |args: Vec<Value>| {
            if variadic && args.len() < arity {
                return Err(format!("function expects at least {arity} arguments"));
            }
            let run = || {
                match Machine::call(program.clone(), prototype, args, (*captured).clone()).run() {
                    VmOutcome::Returned(value) => Ok(value),
                    // The raw message crosses the call boundary:
                    // catch identity (prefix matching against the
                    // thrown-value side channel) depends on it.
                    VmOutcome::Failed(error) => Err(error.message),
                }
            };
            match &registry {
                Some(registry) => with_namespace_registry(registry, run),
                None => run(),
            }
        };
        let closure = if variadic {
            native_fixed_variadic_function(name.as_deref().unwrap_or("fn"), arity, callback)
        } else {
            native_function(name.as_deref().unwrap_or("fn"), arity, callback)
        };
        self.stack.push(closure);
        Ok(())
    }

    /// Executes the GetGlobal instruction: resolves the var in the
    /// current namespace registry and pushes its value.
    #[inline(never)]
    fn exec_get_global(&mut self, program: &Program, index: u32) -> Result<(), String> {
        let Some(name) = constant_string(program, index) else {
            return Err(format!("constant index {index} out of range"));
        };
        let var = crate::core::vm_resolve_global(name)?;
        self.stack.push(var.deref_value());
        Ok(())
    }

    /// Executes the DefGlobal instruction: interns (or reuses) the var
    /// cell, attaches optional metadata, and leaves the value on the
    /// stack.
    #[inline(never)]
    fn exec_def_global(
        &mut self,
        program: &Program,
        name: u32,
        metadata: Option<u16>,
    ) -> Result<(), String> {
        let Some(name) = constant_string(program, name) else {
            return Err(format!("constant index {name} out of range"));
        };
        let Some(value) = self.stack.pop() else {
            return Err("stack underflow".to_string());
        };
        let metadata = metadata.map(|index| program.var_metadata[usize::from(index)].clone());
        crate::core::vm_def_global(name, value.clone(), metadata)?;
        self.stack.push(value);
        Ok(())
    }

    /// Executes the SetGlobal instruction: resets an existing var's root
    /// value and leaves the value on the stack.
    #[inline(never)]
    fn exec_set_global(&mut self, program: &Program, index: u32) -> Result<(), String> {
        let Some(name) = constant_string(program, index) else {
            return Err(format!("constant index {index} out of range"));
        };
        let Some(value) = self.stack.pop() else {
            return Err("stack underflow".to_string());
        };
        let var = crate::core::namespace_registry().and_then(|registry| {
            registry
                .resolve(&Symbol::parse(name))
                .ok_or_else(|| format!("unbound var: {name}"))
        })?;
        var.reset_value(value.clone());
        self.stack.push(value);
        Ok(())
    }

    /// Executes the VarGlobal instruction: pushes the var itself.
    /// Display is namespace-qualified on every path, matching the JVM
    /// runtime and the fixed tree evaluator (issue #223).
    #[inline(never)]
    fn exec_var_global(&mut self, program: &Program, index: u32) -> Result<(), String> {
        let Some(name) = constant_string(program, index) else {
            return Err(format!("constant index {index} out of range"));
        };
        let var = crate::core::vm_resolve_global(name)?;
        self.stack.push(Value::Var(var));
        Ok(())
    }

    /// Executes the DeclareGlobal instruction: interns a nil var when the
    /// name is unbound, never resetting an existing var; pushes nil.
    #[inline(never)]
    fn exec_declare_global(&mut self, program: &Program, index: u32) -> Result<(), String> {
        let Some(name) = constant_string(program, index) else {
            return Err(format!("constant index {index} out of range"));
        };
        crate::core::vm_declare_global(name)?;
        self.stack.push(Value::Nil);
        Ok(())
    }

    /// Executes the DefStruct instruction: interns the type and its
    /// constructor vars, pushing the evaluator's result (nil).
    #[inline(never)]
    fn exec_def_struct(
        &mut self,
        program: &Program,
        name: u32,
        fields: u32,
    ) -> Result<(), String> {
        let Some(name) = constant_string(program, name) else {
            return Err(format!("constant index {name} out of range"));
        };
        let Some(field_names) = constant_string_vector(program, fields) else {
            return Err(format!("constant index {fields} out of range"));
        };
        let value = crate::core::vm_defstruct(name, field_names)?;
        self.stack.push(value);
        Ok(())
    }

    /// Executes the StructField instruction.
    #[inline(never)]
    fn exec_struct_field(&mut self, program: &Program, index: u32) -> Result<(), String> {
        let Some(field) = constant_string(program, index) else {
            return Err(format!("constant index {index} out of range"));
        };
        let Some(value) = self.stack.pop() else {
            return Err("stack underflow".to_string());
        };
        let value = crate::core::struct_field_value(&value, field)?;
        self.stack.push(value);
        Ok(())
    }

    /// Executes the InstanceOf instruction.
    #[inline(never)]
    fn exec_instance_of(&mut self) -> Result<(), String> {
        let Some(value) = self.stack.pop() else {
            return Err("stack underflow".to_string());
        };
        let Some(ty) = self.stack.pop() else {
            return Err("stack underflow".to_string());
        };
        let value = crate::core::struct_instance_of(&ty, &value)?;
        self.stack.push(value);
        Ok(())
    }

    /// Executes the MakeMultiArity instruction: packs the top `count`
    /// function values into the shared arity dispatcher.
    #[inline(never)]
    fn exec_make_multi_arity(
        &mut self,
        program: &Program,
        name: u32,
        count: u8,
    ) -> Result<(), String> {
        let count = usize::from(count);
        if self.stack.len() < count {
            return Err("stack underflow".to_string());
        }
        let Some(name) = constant_string(program, name).map(str::to_owned) else {
            return Err(format!("constant index {name} out of range"));
        };
        let start = self.stack.len() - count;
        if self.stack[start..]
            .iter()
            .any(|value| !matches!(value, Value::Function(_)))
        {
            return Err("multi-arity clauses must be functions".to_string());
        }
        let functions = self
            .stack
            .drain(start..)
            .map(|value| match value {
                Value::Function(function) => function,
                _ => unreachable!("checked above"),
            })
            .collect();
        self.stack
            .push(crate::core::arity_dispatcher(&name, functions, false));
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
