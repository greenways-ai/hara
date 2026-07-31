//! Compiler: `Form` trees (with parser spans) to a validated `Program`.
//!
//! Supports the milestone-2 synchronous subset: literals, lexical locals,
//! the ten shared primitives, `if`, `do`, `let`, `loop`/`recur`, `fn`
//! closures with capture-by-value upvalues, direct calls, and top-level
//! `defn` lowered to direct slot bindings (the var-analysis pass; see
//! `specs/runtime/draft/hal-bytecode-vm.edn` `:vm/defn-lowering`).
//! Anything else is a typed [`CompileError`] with source context; the
//! compiler never emits fallback calls into the tree-walking evaluator.
//!
//! Structure: shared state (constants, finished prototypes) plus a stack
//! of function contexts. Each context owns a code buffer, scope stack,
//! loop stack, and capture list. Slot layout per function: parameters at
//! `0..arity-1`, captures at `arity..arity+capture_count-1`, body locals
//! above. Captures are discovered by a free-variable pre-pass over the
//! body, so their slots are reserved (and pre-declared in the function's
//! base scope) before any body-local slot is allocated.

use crate::core::{Primitive, Value};
use crate::kernel::{Form, Position, Span, SpannedForm};
use std::collections::HashMap;

use super::error::{CompileError, CompileErrorKind};
use super::opcode::Instruction;
use super::program::{FunctionPrototype, Program, MAX_CONSTANTS, MAX_PRIMITIVE_ARGUMENTS};
use super::source_map::SourceMap;
use super::validate::{self, stack_heights};

#[path = "compiler/scope.rs"]
mod scope;
#[path = "compiler/functions.rs"]
mod functions;
use scope::ScopeStack;

/// Operators that name language forms the VM does not implement. In
/// operator position they report as unsupported rather than as unbound
/// symbols; everything else unbound reports as an unbound symbol,
/// matching the evaluator.
const UNSUPPORTED_OPERATORS: &[&str] = &[
    "def", "var", "quote", "set!", "ns", "in-ns", "require", "try", "throw", "await",
];

/// Compiles source text into a validated program. Multiple top-level
/// forms compile as an implicit `do`.
pub fn compile_source(source: &str) -> Result<Program, CompileError> {
    let forms = crate::kernel::read_forms(source)?;
    let mut compiler = Compiler::new();
    let children = compiler.children(&forms);
    compiler.compile_sequence(&children, true)?;
    compiler.finish()
}

/// A form paired with its span and (when the parser provided matching
/// children) the spans of its elements.
struct Child<'a> {
    form: &'a Form,
    span: &'a Span,
    children: Option<&'a [SpannedForm]>,
}

#[derive(Clone)]
struct LoopContext {
    header: usize,
    slots: Vec<u16>,
}

/// One in-progress function body: code, scopes, loops, and captures.
/// The entry function is context 0 with arity and captures 0.
struct FnContext {
    /// Reserved index into `Compiler::functions`.
    proto_id: usize,
    name: Option<String>,
    /// The `defn` self name: compiles to `CallStatic` in operator
    /// position, rejected in value position.
    self_name: Option<String>,
    /// Parameter count; params occupy slots `0..params-1`.
    params: u16,
    /// Captured free variables in slot order (slots `params..`); each
    /// entry carries the first-occurrence position for diagnostics.
    captures: Vec<(String, Option<Position>)>,
    code: Vec<Instruction>,
    source_map: SourceMap,
    scopes: ScopeStack,
    loops: Vec<LoopContext>,
    /// Whether control can reach the next emitted instruction. `recur`
    /// clears it; the compiler emits no dead code.
    fallthrough: bool,
}

struct Compiler {
    constants: Vec<Value>,
    constant_index: HashMap<Value, u32>,
    functions: Vec<FunctionPrototype>,
    contexts: Vec<FnContext>,
    /// Names introduced by top-level `(declare ...)`: an explicit opt-in
    /// to replacing a std.foundation builtin through `defn`.
    declared: Vec<String>,
    /// True while compiling a direct child of the top-level sequence;
    /// `defn` is only legal there, and never in tail position.
    top_level: bool,
}

/// The reservation placed in `functions` while a body is compiled: the
/// prototype index, arity, and capture count are known up front, the
/// code is filled in when the context closes.
fn placeholder(name: Option<String>, arity: u16, capture_count: u16) -> FunctionPrototype {
    FunctionPrototype {
        name,
        arity,
        capture_count,
        local_count: 0,
        max_stack: 0,
        code: Vec::new(),
        source_map: SourceMap::default(),
    }
}

impl Compiler {
    fn new() -> Compiler {
        let mut scopes = ScopeStack::new();
        scopes.push_scope();
        Compiler {
            constants: Vec::new(),
            constant_index: HashMap::new(),
            functions: vec![placeholder(None, 0, 0)],
            contexts: vec![FnContext {
                proto_id: 0,
                name: None,
                self_name: None,
                params: 0,
                captures: Vec::new(),
                code: Vec::new(),
                source_map: SourceMap::default(),
                scopes,
                loops: Vec::new(),
                fallthrough: true,
            }],
            declared: Vec::new(),
            top_level: true,
        }
    }

    fn ctx(&self) -> &FnContext {
        self.contexts.last().expect("function context is open")
    }

    fn ctx_mut(&mut self) -> &mut FnContext {
        self.contexts.last_mut().expect("function context is open")
    }

    /// Pairs parsed forms with their spans. When a node's children do not
    /// match its element count (reader macros expand to synthetic lists),
    /// elements inherit the parent span.
    fn children<'a>(&self, nodes: &'a [SpannedForm]) -> Vec<Child<'a>> {
        nodes
            .iter()
            .map(|node| Child {
                form: &node.form,
                span: &node.span,
                children: Some(&node.children),
            })
            .collect()
    }

    fn list_children<'a>(
        &self,
        elements: &'a [Form],
        span: &'a Span,
        children: Option<&'a [SpannedForm]>,
    ) -> Vec<Child<'a>> {
        let usable = children.filter(|nodes| nodes.len() == elements.len());
        elements
            .iter()
            .enumerate()
            .map(|(index, form)| match usable.and_then(|nodes| nodes.get(index)) {
                Some(node) => Child {
                    form: &node.form,
                    span: &node.span,
                    children: Some(&node.children),
                },
                None => Child {
                    form,
                    span,
                    children: None,
                },
            })
            .collect()
    }

    fn emit(&mut self, instruction: Instruction, position: Option<Position>) -> usize {
        let context = self.ctx_mut();
        debug_assert!(context.fallthrough, "no emission after control terminates");
        let index = context.code.len();
        context.code.push(instruction);
        context.source_map.record(position);
        index
    }

    fn patch_jump(&mut self, at: usize, target: usize) {
        let target = target as u32;
        match &mut self.ctx_mut().code[at] {
            Instruction::Jump(operand) | Instruction::JumpIfFalse(operand) => *operand = target,
            other => unreachable!("patching non-jump instruction: {other:?}"),
        }
    }

    fn constant(&mut self, value: Value, span: &Span) -> Result<(), CompileError> {
        let index = match self.constant_index.get(&value) {
            Some(index) => *index,
            None => {
                if self.constants.len() >= MAX_CONSTANTS {
                    return Err(CompileError::new(
                        CompileErrorKind::Limit,
                        format!("constant pool exceeds limit of {MAX_CONSTANTS}"),
                        Some(span.start),
                    ));
                }
                let index = self.constants.len() as u32;
                self.constants.push(value.clone());
                self.constant_index.insert(value, index);
                index
            }
        };
        self.emit(Instruction::Constant(index), Some(span.start));
        Ok(())
    }

    fn unsupported(&self, form: &Form, span: &Span) -> CompileError {
        let message = match form {
            Form::List(elements) => match elements.first() {
                Some(Form::Symbol(name)) => format!("unsupported operator: {name}"),
                _ => format!("unsupported form: {form}"),
            },
            _ => format!("unsupported form: {form}"),
        };
        CompileError::new(
            CompileErrorKind::UnsupportedForm,
            message,
            Some(span.start),
        )
    }

    /// Compiles a sequence of forms as an implicit `do`: every non-final
    /// result is popped. Dead forms after a terminating `recur` are not
    /// analyzed, matching the evaluator, which never reaches them.
    fn compile_sequence(&mut self, children: &[Child<'_>], tail: bool) -> Result<(), CompileError> {
        if children.is_empty() {
            self.emit(Instruction::Nil, None);
            return Ok(());
        }
        let top = self.top_level && self.contexts.len() == 1;
        let last = children.len() - 1;
        for (index, child) in children.iter().enumerate() {
            if !self.ctx().fallthrough {
                break;
            }
            self.top_level = top;
            self.compile_form(child.form, child.span, child.children, tail && index == last)?;
            if index != last && self.ctx().fallthrough {
                self.emit(Instruction::Pop, Some(child.span.start));
            }
        }
        Ok(())
    }

    /// `(declare name ...)` marks names for explicit replacement: a later
    /// top-level `defn` of a declared std.foundation builtin lowers to a
    /// slot binding instead of erroring (issue #202 ruling). Top-level
    /// statements only; evaluates to nil, matching the evaluator.
    fn compile_declare(
        &mut self,
        children: &[Child<'_>],
        span: &Span,
        top: bool,
    ) -> Result<(), CompileError> {
        if !top {
            return Err(CompileError::new(
                CompileErrorKind::UnsupportedForm,
                "declare is only supported as a top-level statement",
                Some(span.start),
            ));
        }
        for child in &children[1..] {
            let Form::Symbol(name) = child.form else {
                return Err(CompileError::new(
                    CompileErrorKind::Arity,
                    "declare expects name symbols",
                    Some(child.span.start),
                ))
            };
            if !self.declared.iter().any(|n| n == name) {
                self.declared.push(name.clone());
            }
        }
        self.emit(Instruction::Nil, Some(span.start));
        Ok(())
    }

    fn compile_form(
        &mut self,
        form: &Form,
        span: &Span,
        children: Option<&[SpannedForm]>,
        tail: bool,
    ) -> Result<(), CompileError> {
        let top = self.top_level;
        self.top_level = false;
        if !self.ctx().fallthrough {
            // Dead code (e.g. after a nested infinite loop): not analyzed,
            // matching the evaluator, which never reaches it.
            return Ok(());
        }
        match form {
            Form::Nil => {
                self.emit(Instruction::Nil, Some(span.start));
                Ok(())
            }
            Form::Bool(true) => {
                self.emit(Instruction::True, Some(span.start));
                Ok(())
            }
            Form::Bool(false) => {
                self.emit(Instruction::False, Some(span.start));
                Ok(())
            }
            Form::Number(value) => self.constant(Value::Number(*value), span),
            Form::Float(value) => self.constant(Value::Float(*value), span),
            Form::String(value) => self.constant(Value::String(value.clone()), span),
            Form::Keyword(value) => self.constant(Value::Keyword(value.clone().into()), span),
            Form::Character(value) => self.constant(Value::Character(*value), span),
            Form::BigInteger(value) => self.constant(Value::BigInteger(value.clone()), span),
            Form::Decimal(value) => self.constant(Value::Decimal(value.clone()), span),
            Form::Regex(value) => self.constant(Value::Regex(value.clone()), span),
            Form::Metadata(_, value) => self.compile_form(value, span, None, tail),
            Form::Symbol(name) => {
                if self.ctx().self_name.as_deref() == Some(name.as_str()) {
                    return Err(CompileError::new(
                        CompileErrorKind::UnsupportedForm,
                        "defn self-reference in value position is not supported",
                        Some(span.start),
                    ));
                }
                match self.ctx().scopes.resolve(name) {
                    Some(slot) => {
                        self.emit(Instruction::LoadLocal(slot), Some(span.start));
                        Ok(())
                    }
                    None => Err(CompileError::new(
                        CompileErrorKind::UnboundSymbol,
                        format!("unbound symbol: {name}"),
                        Some(span.start),
                    )),
                }
            }
            Form::List(elements) if elements.is_empty() => {
                self.emit(Instruction::Nil, Some(span.start));
                Ok(())
            }
            Form::List(elements) => {
                let children = self.list_children(elements, span, children);
                match &elements[0] {
                    Form::Symbol(name) if name == "if" => self.compile_if(&children, span, tail),
                    Form::Symbol(name) if name == "do" => {
                        // A top-level `do` is transparent: its statements
                        // keep top-level position, so `defn` lowering works
                        // inside `(do (defn ...) ...)`.
                        self.top_level = top;
                        self.compile_sequence(&children[1..], tail)
                    }
                    Form::Symbol(name) if name == "let" => {
                        self.compile_let(&children, span, tail)
                    }
                    Form::Symbol(name) if name == "loop" => {
                        self.compile_loop(&children, span, tail)
                    }
                    Form::Symbol(name) if name == "recur" => {
                        self.compile_recur(&children, span, tail)
                    }
                    Form::Symbol(name) if name == "fn" => self.compile_fn_form(&children, span),
                    Form::Symbol(name) if name == "defn" => {
                        self.compile_defn(&children, span, top, tail)
                    }
                    Form::Symbol(name) if name == "declare" => {
                        self.compile_declare(&children, span, top)
                    }
                    Form::Symbol(name) if UNSUPPORTED_OPERATORS.contains(&name.as_str()) => {
                        Err(self.unsupported(form, span))
                    }
                    Form::Symbol(name) => match Primitive::from_symbol(name) {
                        Some(op) => self.compile_primitive(&children, span, op),
                        None => self.compile_named_call(name, &children, span),
                    },
                    _ => self.compile_expression_call(&children, span),
                }
            }
            _ => Err(self.unsupported(form, span)),
        }
    }

    fn compile_primitive(
        &mut self,
        children: &[Child<'_>],
        span: &Span,
        op: Primitive,
    ) -> Result<(), CompileError> {
        let argc = children.len() - 1;
        if argc > MAX_PRIMITIVE_ARGUMENTS {
            return Err(CompileError::new(
                CompileErrorKind::Limit,
                format!("primitive calls support at most {MAX_PRIMITIVE_ARGUMENTS} arguments"),
                Some(span.start),
            ));
        }
        for argument in &children[1..] {
            self.compile_form(argument.form, argument.span, argument.children, false)?;
        }
        if !self.ctx().fallthrough {
            return Ok(());
        }
        self.emit(
            Instruction::Primitive {
                op,
                argc: argc as u8,
            },
            Some(span.start),
        );
        Ok(())
    }

    /// Compiles the argument forms of a call (callee already compiled).
    fn compile_call_arguments(
        &mut self,
        children: &[Child<'_>],
        span: &Span,
    ) -> Result<(), CompileError> {
        let argc = children.len() - 1;
        if argc > MAX_PRIMITIVE_ARGUMENTS {
            return Err(CompileError::new(
                CompileErrorKind::Limit,
                format!("calls support at most {MAX_PRIMITIVE_ARGUMENTS} arguments"),
                Some(span.start),
            ));
        }
        for argument in &children[1..] {
            self.compile_form(argument.form, argument.span, argument.children, false)?;
        }
        Ok(())
    }

    fn compile_if(
        &mut self,
        children: &[Child<'_>],
        span: &Span,
        tail: bool,
    ) -> Result<(), CompileError> {
        // The condition is never a tail position; the branches inherit the
        // `if`'s own tail context.
        if children.len() != 3 && children.len() != 4 {
            return Err(CompileError::new(
                CompileErrorKind::Arity,
                "if expects 2 or 3 arguments",
                Some(span.start),
            ));
        }
        let condition = &children[1];
        self.compile_form(condition.form, condition.span, condition.children, false)?;
        if !self.ctx().fallthrough {
            // The condition cannot produce a value (e.g. an infinite inner
            // loop); the branches are dead code.
            return Ok(());
        }
        let jump_else = self.emit(Instruction::JumpIfFalse(0), Some(condition.span.start));
        let then = &children[2];
        self.compile_form(then.form, then.span, then.children, tail)?;
        let then_fell = self.ctx().fallthrough;
        let jump_end = if then_fell {
            Some(self.emit(Instruction::Jump(0), Some(then.span.start)))
        } else {
            None
        };
        // The else branch starts fresh at its label.
        self.ctx_mut().fallthrough = true;
        let else_target = self.ctx().code.len();
        if let Some(else_form) = children.get(3) {
            self.compile_form(else_form.form, else_form.span, else_form.children, tail)?;
        } else {
            self.emit(Instruction::Nil, Some(span.start));
        }
        let else_fell = self.ctx().fallthrough;
        let end = self.ctx().code.len();
        self.patch_jump(jump_else, else_target);
        if let Some(jump_end) = jump_end {
            self.patch_jump(jump_end, end);
        }
        self.ctx_mut().fallthrough = then_fell || else_fell;
        Ok(())
    }

    /// Compiles `let`-style ordered bindings into fresh slots, returns the
    /// bound slots, and leaves the scope open for the body.
    fn compile_bindings(
        &mut self,
        children: &[Child<'_>],
        form_name: &str,
    ) -> Result<Vec<u16>, CompileError> {
        let bindings = &children[1];
        let pairs: &[Form] = match bindings.form {
            Form::List(values) | Form::Vector(values) => values,
            _ => {
                return Err(CompileError::new(
                    CompileErrorKind::Arity,
                    format!("{form_name} expects a binding list or vector"),
                    Some(bindings.span.start),
                ))
            }
        };
        if pairs.len() % 2 != 0 {
            return Err(CompileError::new(
                CompileErrorKind::Arity,
                format!("{form_name} bindings require name/value pairs"),
                Some(bindings.span.start),
            ));
        }
        // Binding-pair children keep their own spans when available.
        let pair_children = self.list_children(pairs, bindings.span, bindings.children);
        let mut slots = Vec::with_capacity(pairs.len() / 2);
        for pair in pair_children.chunks(2) {
            let (name, initializer) = (&pair[0], &pair[1]);
            // Binding names are structural: validate before compiling the
            // initializer so destructuring reports on the name.
            let Form::Symbol(symbol) = name.form else {
                return Err(CompileError::new(
                    CompileErrorKind::UnsupportedForm,
                    format!("{form_name} destructuring is not supported"),
                    Some(name.span.start),
                ))
            };
            self.compile_form(initializer.form, initializer.span, initializer.children, false)?;
            if !self.ctx().fallthrough {
                return Ok(slots);
            }
            let slot = self.ctx_mut().scopes.declare(symbol).map_err(|error| {
                CompileError::new(error.kind(), error.message(), Some(name.span.start))
            })?;
            self.emit(Instruction::StoreLocal(slot), Some(name.span.start));
            slots.push(slot);
        }
        Ok(slots)
    }

    fn compile_let(
        &mut self,
        children: &[Child<'_>],
        span: &Span,
        tail: bool,
    ) -> Result<(), CompileError> {
        if children.len() < 3 {
            return Err(CompileError::new(
                CompileErrorKind::Arity,
                "let expects bindings and a body",
                Some(span.start),
            ));
        }
        self.ctx_mut().scopes.push_scope();
        let result = self
            .compile_bindings(children, "let")
            .and_then(|_| self.compile_sequence(&children[2..], tail));
        self.ctx_mut().scopes.pop_scope();
        result
    }

    fn compile_loop(
        &mut self,
        children: &[Child<'_>],
        span: &Span,
        _tail: bool,
    ) -> Result<(), CompileError> {
        if children.len() < 3 {
            return Err(CompileError::new(
                CompileErrorKind::Arity,
                "loop expects bindings and a body",
                Some(span.start),
            ));
        }
        self.ctx_mut().scopes.push_scope();
        let result = self
            .compile_bindings(children, "loop")
            .and_then(|slots| {
                let header = self.ctx().code.len();
                self.ctx_mut().loops.push(LoopContext { header, slots });
                // Multiple body forms sequence like `do`; the last one is
                // the loop's tail (recur) position.
                let result = self.compile_sequence(&children[2..], true);
                self.ctx_mut().loops.pop();
                result
            });
        self.ctx_mut().scopes.pop_scope();
        result
    }

    fn compile_recur(
        &mut self,
        children: &[Child<'_>],
        span: &Span,
        tail: bool,
    ) -> Result<(), CompileError> {
        if children.len() < 2 {
            return Err(CompileError::new(
                CompileErrorKind::Recur,
                "recur expects values",
                Some(span.start),
            ));
        }
        let Some(context) = self.ctx().loops.last().cloned() else {
            return Err(CompileError::new(
                CompileErrorKind::Recur,
                "recur must be inside loop",
                Some(span.start),
            ));
        };
        if !tail {
            return Err(CompileError::new(
                CompileErrorKind::Recur,
                "recur must be in tail position",
                Some(span.start),
            ));
        }
        if children.len() - 1 != context.slots.len() {
            return Err(CompileError::new(
                CompileErrorKind::Recur,
                "loop recur arity mismatch",
                Some(span.start),
            ));
        }
        // Every argument is evaluated before any store, then stored into
        // the loop slots in reverse order: simultaneous recurrence.
        for argument in &children[1..] {
            self.compile_form(argument.form, argument.span, argument.children, false)?;
        }
        if !self.ctx().fallthrough {
            return Ok(());
        }
        for &slot in context.slots.iter().rev() {
            self.emit(Instruction::StoreLocal(slot), Some(span.start));
        }
        self.emit(Instruction::Jump(context.header as u32), Some(span.start));
        self.ctx_mut().fallthrough = false;
        Ok(())
    }

    fn finish(mut self) -> Result<Program, CompileError> {
        if self.ctx().fallthrough {
            self.emit(Instruction::Return, None);
        }
        self.close_context();
        let mut program = Program {
            constants: self.constants,
            functions: self.functions,
            entry: 0,
        };
        // The shared analysis computes each operand-stack high-water mark;
        // full validation then runs over the whole program before it is
        // returned.
        for index in 0..program.functions.len() {
            let heights = stack_heights(&program, &program.functions[index])
                .map_err(|error| internal(error.to_string()))?;
            program.functions[index].max_stack = heights.iter().copied().max().unwrap_or(0);
        }
        validate::validate(&program).map_err(|error| internal(error.to_string()))?;
        Ok(program)
    }
}

fn internal(message: String) -> CompileError {
    CompileError::new(CompileErrorKind::Internal, message, None)
}
