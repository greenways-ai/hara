//! Compiler: `Form` trees (with parser spans) to a validated `Program`.
//!
//! Supports the milestone-4 synchronous subset: literals, lexical locals,
//! the ten shared primitives, `if`, `do`, `let`, `loop`/`recur`, `fn`
//! closures with capture-by-value upvalues (including variadic
//! parameters), direct calls, exceptions, and the registry-direct global
//! forms — `def`, `defn`/`defn-` (single- and multi-arity, interning real
//! late-bound vars), `var`, `set!`, `declare`, `defstruct`, `field`, and
//! `instance?` (issue #223; see
//! `specs/01-lang/010-bytecode/draft/hal-bytecode-vm.edn` `:vm/namespaces`).
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
use crate::lang::data::List as PList;
use std::collections::HashMap;

use super::error::{CompileError, CompileErrorKind};
use super::opcode::Instruction;
use super::program::{
    FunctionPrototype, Program, TryEntry, MAX_CONSTANTS, MAX_PRIMITIVE_ARGUMENTS,
};
use super::source_map::SourceMap;
use super::validate::{self, stack_heights};

#[path = "compiler/exceptions.rs"]
mod exceptions;
#[path = "compiler/functions.rs"]
mod functions;
#[path = "compiler/scope.rs"]
mod scope;
use exceptions::TryContext;
#[path = "compiler/globals.rs"]
mod globals;
#[path = "compiler/recur.rs"]
mod recur;
use recur::LoopContext;
use scope::ScopeStack;

/// Operators that name language forms the VM does not implement. In
/// operator position they report as unsupported rather than as unbound
/// symbols; everything else unbound reports as an unbound symbol,
/// matching the evaluator.
const UNSUPPORTED_OPERATORS: &[&str] = &["quote", "ns", "in-ns", "require", "await"];

/// Compiles source text into a validated program. Multiple top-level
/// forms compile as an implicit `do`. Without a namespace registry the
/// program must be closed: only the names it declares itself are
/// visible as globals (issue #223).
pub fn compile_source(source: &str) -> Result<Program, CompileError> {
    let forms = crate::kernel::read_forms(source)?;
    let mut compiler = Compiler::new();
    let children = compiler.children(&forms);
    compiler.compile_sequence(&children, true)?;
    compiler.finish()
}

/// Compiles against a caller's namespace registry: registry vars
/// (std.foundation and anything already interned) are visible to the
/// two-phase global check, exactly as they will resolve at execution
/// time through `execute_program_with_globals` (issue #223).
pub fn compile_source_with(
    source: &str,
    registry: &crate::kernel::NamespaceRegistry<crate::core::Value>,
) -> Result<Program, CompileError> {
    crate::core::with_namespace_registry(registry, || compile_source(source))
}

/// A form paired with its span and (when the parser provided matching
/// children) the spans of its elements.
#[derive(Clone, Copy)]
struct Child<'a> {
    form: &'a Form,
    span: &'a Span,
    children: Option<&'a [SpannedForm]>,
}

/// One in-progress function body: code, scopes, loops, and captures.
/// The entry function is context 0 with arity and captures 0.
struct FnContext {
    /// Reserved index into `Compiler::functions`.
    proto_id: usize,
    name: Option<String>,
    /// Fixed parameter count; params occupy slots `0..params-1`.
    params: u16,
    /// Whether the function has a `& rest` parameter (occupying the slot
    /// directly above the fixed params, below the captures).
    variadic: bool,
    /// Whether `std.foundation.coroutine/await` may be emitted in this
    /// function. Every function context resets this flag.
    suspend_allowed: bool,
    /// Whether calls to this prototype return a result promise.
    async_function: bool,
    /// Captured free variables in slot order (slots `params..`); each
    /// entry carries the first-occurrence position for diagnostics.
    captures: Vec<(String, Option<Position>)>,
    code: Vec<Instruction>,
    source_map: SourceMap,
    scopes: ScopeStack,
    loops: Vec<LoopContext>,
    tries: Vec<TryContext>,
    /// Finished handler table entries for this function; entry depths are
    /// patched in after stack analysis in `finish`.
    handlers: Vec<TryEntry>,
    /// Whether control can reach the next emitted instruction. `recur`
    /// clears it; the compiler emits no dead code.
    fallthrough: bool,
}

struct Compiler {
    constants: Vec<Value>,
    constant_index: HashMap<Value, u32>,
    functions: Vec<FunctionPrototype>,
    contexts: Vec<FnContext>,
    /// Names this program defines (`def`/`defn`/`declare`/`defstruct`):
    /// visible to global references compiled after their defining form
    /// (issue #223 two-phase visibility).
    globals: Vec<String>,
    /// Var metadata table indexed by `DefGlobal` operands.
    var_metadata: Vec<std::rc::Rc<crate::lang::data::Metadata>>,
    /// True while compiling a direct child of the top-level sequence;
    /// `defn` and `declare` are only legal there.
    top_level: bool,
}

/// The reservation placed in `functions` while a body is compiled: the
/// prototype index, arity, and capture count are known up front, the
/// code is filled in when the context closes.
fn placeholder(
    name: Option<String>,
    arity: u16,
    capture_count: u16,
    variadic: bool,
    async_function: bool,
) -> FunctionPrototype {
    FunctionPrototype {
        name,
        async_function,
        arity,
        variadic,
        capture_count,
        local_count: 0,
        max_stack: 0,
        code: Vec::new(),
        source_map: SourceMap::default(),
        handlers: Vec::new(),
    }
}

impl Compiler {
    fn new() -> Compiler {
        let mut scopes = ScopeStack::new();
        scopes.push_scope();
        Compiler {
            constants: Vec::new(),
            constant_index: HashMap::new(),
            functions: vec![placeholder(None, 0, 0, false, false)],
            contexts: vec![FnContext {
                proto_id: 0,
                name: None,
                params: 0,
                variadic: false,
                suspend_allowed: false,
                async_function: false,
                captures: Vec::new(),
                code: Vec::new(),
                source_map: SourceMap::default(),
                scopes,
                loops: Vec::new(),
                tries: Vec::new(),
                handlers: Vec::new(),
                fallthrough: true,
            }],
            globals: Vec::new(),
            var_metadata: Vec::new(),
            top_level: true,
        }
    }

    fn ctx(&self) -> &FnContext {
        self.contexts.last().expect("function context is open")
    }

    fn ctx_mut(&mut self) -> &mut FnContext {
        self.contexts.last_mut().expect("function context is open")
    }

    /// Resolves coroutine forms by canonical Var identity so aliases and
    /// referred names behave exactly like fully-qualified source.
    fn is_coroutine_var(&self, name: &str, member: &str) -> bool {
        let canonical = format!("std.foundation.coroutine/{member}");
        if name == canonical {
            return true;
        }
        crate::core::namespace_registry()
            .ok()
            .and_then(|registry| {
                let source = registry.resolve(&crate::lang::data::Symbol::parse(name))?;
                let target = registry.resolve(&crate::lang::data::Symbol::parse(&canonical))?;
                Some(source.same_identity(&target))
            })
            .unwrap_or(false)
    }

    fn is_host_call_var(&self, name: &str) -> bool {
        let canonical = "std.native.Host/call";
        if name == canonical {
            return true;
        }
        crate::core::namespace_registry()
            .ok()
            .and_then(|registry| {
                let source = registry.resolve(&crate::lang::data::Symbol::parse(name))?;
                let target = registry.resolve(&crate::lang::data::Symbol::parse(canonical))?;
                Some(source.same_identity(&target))
            })
            .unwrap_or(false)
    }

    fn compile_host_call(
        &mut self,
        children: &[Child<'_>],
        span: &Span,
    ) -> Result<(), CompileError> {
        if children.len() != 4 {
            return Err(CompileError::new(
                CompileErrorKind::Arity,
                "std.native.Host/call expects service, method, and an argument vector",
                Some(span.start),
            ));
        }
        self.compile_call_arguments(children, span)?;
        if self.ctx().fallthrough {
            self.emit(Instruction::HostCall, Some(span.start));
        }
        Ok(())
    }

    fn compile_await(&mut self, children: &[Child<'_>], span: &Span) -> Result<(), CompileError> {
        if children.len() != 2 {
            return Err(CompileError::new(
                CompileErrorKind::Arity,
                "co/await expects one promise",
                Some(span.start),
            ));
        }
        if !self.ctx().suspend_allowed {
            return Err(CompileError::new(
                CompileErrorKind::InvalidEffect,
                "co/await requires ^:async or co/create",
                Some(span.start),
            ));
        }
        let promise = &children[1];
        self.compile_form(promise.form, promise.span, promise.children, false)?;
        if self.ctx().fallthrough {
            self.emit(Instruction::Await, Some(span.start));
        }
        Ok(())
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
            .map(
                |(index, form)| match usable.and_then(|nodes| nodes.get(index)) {
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
                },
            )
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
        let index = self.constant_index_of(value, span)?;
        self.emit(Instruction::Constant(index), Some(span.start));
        Ok(())
    }

    /// The pool index for a constant, interning it if new. Used directly
    /// for instruction operands (global names, struct fields); `constant`
    /// additionally emits the load.
    fn constant_index_of(&mut self, value: Value, span: &Span) -> Result<u32, CompileError> {
        match self.constant_index.get(&value) {
            Some(index) => Ok(*index),
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
                Ok(index)
            }
        }
    }

    fn unsupported(&self, form: &Form, span: &Span) -> CompileError {
        let message = match form {
            Form::List(elements) => match elements.first() {
                Some(Form::Symbol(name)) => format!("unsupported operator: {name}"),
                _ => format!("unsupported form: {form}"),
            },
            _ => format!("unsupported form: {form}"),
        };
        CompileError::new(CompileErrorKind::UnsupportedForm, message, Some(span.start))
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
            self.compile_form(
                child.form,
                child.span,
                child.children,
                tail && index == last,
            )?;
            if index != last && self.ctx().fallthrough {
                self.emit(Instruction::Pop, Some(child.span.start));
            }
        }
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
        if let Form::List(values) = crate::core::form_without_metadata(form) {
            let protected = matches!(
                values.first(),
                Some(Form::Symbol(name)) if name == "quote" || name == "syntax-quote"
            );
            if !protected {
                let expanded = crate::core::vm_macroexpand(form).map_err(|message| {
                    CompileError::new(
                        CompileErrorKind::UnsupportedForm,
                        message,
                        Some(span.start),
                    )
                })?;
                if expanded != *form {
                    self.top_level = top;
                    return self.compile_form(&expanded, span, None, tail);
                }
            }
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
            Form::Vector(_) | Form::Map(_) | Form::Set(_) if constant_form(form) => {
                let value = crate::core::form_to_value(form).map_err(|message| {
                    CompileError::new(CompileErrorKind::UnsupportedForm, message, Some(span.start))
                })?;
                self.constant(value, span)
            }
            Form::Vector(values) => {
                self.compile_collection_values(values.iter(), span)?;
                self.emit(
                    Instruction::BuildVector(self.collection_count(values.len(), span)?),
                    Some(span.start),
                );
                Ok(())
            }
            Form::Map(entries) => {
                if entries.len() > usize::from(u16::MAX) {
                    return Err(CompileError::new(
                        CompileErrorKind::Limit,
                        "map literal exceeds 65535 entries",
                        Some(span.start),
                    ));
                }
                for (key, value) in entries {
                    self.compile_form(key, span, None, false)?;
                    self.compile_form(value, span, None, false)?;
                }
                self.emit(
                    Instruction::BuildMap(entries.len() as u16),
                    Some(span.start),
                );
                Ok(())
            }
            Form::Set(values) => {
                self.compile_collection_values(values.iter(), span)?;
                self.emit(
                    Instruction::BuildSet(self.collection_count(values.len(), span)?),
                    Some(span.start),
                );
                Ok(())
            }
            Form::Metadata(_, value) => self.compile_form(value, span, None, tail),
            Form::Symbol(name) => match self.ctx().scopes.resolve(name) {
                Some(slot) => {
                    self.emit(Instruction::LoadLocal(slot), Some(span.start));
                    Ok(())
                }
                None if self.visible_global(name) => self.emit_get_global(name, span),
                None => Err(CompileError::new(
                    CompileErrorKind::UnboundSymbol,
                    format!("unbound symbol: {name}"),
                    Some(span.start),
                )),
            },
            Form::List(elements) if elements.is_empty() => {
                self.constant(Value::List(PList::new()), span)
            }
            Form::List(elements) => {
                let children = self.list_children(elements, span, children);
                match &elements[0] {
                    Form::Symbol(name) if self.is_coroutine_var(name, "await") => {
                        self.compile_await(&children, span)
                    }
                    Form::Symbol(name) if self.is_host_call_var(name) => {
                        self.compile_host_call(&children, span)
                    }
                    Form::Symbol(name) if name == "if" => self.compile_if(&children, span, tail),
                    Form::Symbol(name) if name == "and" => self.compile_and(&children, span, tail),
                    Form::Symbol(name) if name == "or" => self.compile_or(&children, span, tail),
                    Form::Symbol(name) if name == "cond" => {
                        self.compile_cond(&children, span, tail)
                    }
                    Form::Symbol(name) if name == "quote" => self.compile_quote(&children, span),
                    Form::Symbol(name) if name == "syntax-quote" => {
                        self.compile_syntax_quote(&children, span)
                    }
                    Form::Symbol(name) if name == "do" => {
                        // A top-level `do` is transparent: its statements
                        // keep top-level position, so `defn` lowering works
                        // inside `(do (defn ...) ...)`.
                        self.top_level = top;
                        self.compile_sequence(&children[1..], tail)
                    }
                    Form::Symbol(name) if name == "let" => self.compile_let(&children, span, tail),
                    Form::Symbol(name) if name == "loop" => {
                        self.compile_loop(&children, span, tail)
                    }
                    Form::Symbol(name) if name == "recur" => {
                        self.compile_recur(&children, span, tail)
                    }
                    Form::Symbol(name) if name == "fn" => self.compile_fn_form(&children, span),
                    Form::Symbol(name) if name == "def" => self.compile_def(&children, span),
                    Form::Symbol(name) if name == "defn" || name == "defn-" => {
                        self.compile_defn(&children, span, top, name == "defn-")
                    }
                    Form::Symbol(name) if name == "defmacro" => {
                        self.compile_defmacro(&children, span, top)
                    }
                    Form::Symbol(name) if name == "declare" => {
                        self.compile_declare(&children, span, top)
                    }
                    Form::Symbol(name) if name == "var" => self.compile_var(&children, span),
                    Form::Symbol(name) if name == "set!" => self.compile_set(&children, span),
                    Form::Symbol(name) if name == "defstruct" => {
                        self.compile_defstruct(&children, span)
                    }
                    Form::Symbol(name) if name == "field" => self.compile_field(&children, span),
                    Form::Symbol(name) if name == "instance?" => {
                        self.compile_instance_of(&children, span)
                    }
                    Form::Symbol(name) if name == "try" => self.compile_try(&children, span, tail),
                    Form::Symbol(name) if name == "throw" => self.compile_throw(&children, span),
                    Form::Symbol(name) if UNSUPPORTED_OPERATORS.contains(&name.as_str()) => {
                        Err(self.unsupported(form, span))
                    }
                    // Precedence mirrors the evaluator (core.rs operator
                    // dispatch): a bound var wins over the structural
                    // builtin arms, so a program-declared or registry
                    // global compiles to GetGlobal+Call even when it names
                    // a primitive; only otherwise-unbound operator names
                    // lower to Primitive instructions (issue #223).
                    Form::Symbol(name) if self.visible_global(name) => {
                        self.compile_named_call(name, &children, span)
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

    fn collection_count(&self, count: usize, span: &Span) -> Result<u16, CompileError> {
        u16::try_from(count).map_err(|_| {
            CompileError::new(
                CompileErrorKind::Limit,
                "collection literal exceeds 65535 items",
                Some(span.start),
            )
        })
    }

    fn compile_collection_values<'a>(
        &mut self,
        values: impl Iterator<Item = &'a Form>,
        span: &Span,
    ) -> Result<(), CompileError> {
        for value in values {
            self.compile_form(value, span, None, false)?;
        }
        Ok(())
    }

    fn compile_quote(&mut self, children: &[Child<'_>], span: &Span) -> Result<(), CompileError> {
        if children.len() != 2 {
            return Err(CompileError::new(
                CompileErrorKind::Arity,
                "quote expects one argument",
                Some(span.start),
            ));
        }
        let value = crate::core::form_to_value(children[1].form).map_err(|message| {
            CompileError::new(CompileErrorKind::UnsupportedForm, message, Some(span.start))
        })?;
        self.constant(value, span)
    }

    fn compile_syntax_quote(
        &mut self,
        children: &[Child<'_>],
        span: &Span,
    ) -> Result<(), CompileError> {
        if children.len() != 2 {
            return Err(CompileError::new(
                CompileErrorKind::Arity,
                "syntax-quote expects one argument",
                Some(span.start),
            ));
        }
        let lowered = lower_syntax_quote(children[1].form, false).map_err(|message| {
            CompileError::new(CompileErrorKind::UnsupportedForm, message, Some(span.start))
        })?;
        self.compile_form(&lowered, span, None, false)
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
        // Mutable conversion creates/consumes runtime identity and must run on
        // every execution. Folding it would place a one-shot transient in the
        // constant pool, so the second execution would observe a frozen value.
        if !matches!(op, Primitive::ToMutable | Primitive::ToPersistent)
            && children[1..]
                .iter()
                .all(|argument| constant_form(argument.form))
        {
            let arguments = children[1..]
                .iter()
                .map(|argument| crate::core::form_to_value(argument.form))
                .collect::<Result<Vec<_>, _>>();
            if let Ok(arguments) = arguments {
                if let Ok(value) = crate::core::apply_primitive(op, &arguments) {
                    return self.constant(value, span);
                }
            }
        }
        if op == Primitive::First && argc == 1 {
            if let Form::List(elements) = children[1].form {
                if matches!(elements.as_slice(), [Form::Symbol(name), _] if name == "rest") {
                    let nested =
                        self.list_children(elements, children[1].span, children[1].children);
                    if constant_form(nested[1].form) {
                        if let Ok(argument) = crate::core::form_to_value(nested[1].form) {
                            if let Ok(value) =
                                crate::core::apply_primitive(Primitive::Second, &[argument])
                            {
                                return self.constant(value, span);
                            }
                        }
                    }
                    self.compile_form(nested[1].form, nested[1].span, nested[1].children, false)?;
                    if self.ctx().fallthrough {
                        self.emit(
                            Instruction::Primitive {
                                op: Primitive::Second,
                                argc: 1,
                            },
                            Some(span.start),
                        );
                    }
                    return Ok(());
                }
            }
        }
        if argc == 2 {
            if let (Form::Symbol(name), Form::Number(value)) = (children[1].form, children[2].form)
            {
                if let Some(local) = self.ctx().scopes.resolve(name) {
                    let constant =
                        self.constant_index_of(Value::Number(*value), children[2].span)?;
                    self.emit(
                        Instruction::PrimitiveLocalConst {
                            op,
                            local,
                            constant,
                        },
                        Some(span.start),
                    );
                    return Ok(());
                }
            }
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

    fn compile_and(
        &mut self,
        children: &[Child<'_>],
        span: &Span,
        tail: bool,
    ) -> Result<(), CompileError> {
        if children.len() == 1 {
            self.emit(Instruction::True, Some(span.start));
            return Ok(());
        }
        let mut false_jumps = Vec::new();
        for child in &children[1..children.len() - 1] {
            self.compile_form(child.form, child.span, child.children, false)?;
            self.emit(Instruction::Dup, Some(child.span.start));
            false_jumps.push(self.emit(Instruction::JumpIfFalse(0), Some(child.span.start)));
            self.emit(Instruction::Pop, Some(child.span.start));
        }
        let last = children.last().expect("and has an argument");
        self.compile_form(last.form, last.span, last.children, tail)?;
        let end = self.ctx().code.len();
        for jump in false_jumps {
            self.patch_jump(jump, end);
        }
        Ok(())
    }

    fn compile_or(
        &mut self,
        children: &[Child<'_>],
        span: &Span,
        tail: bool,
    ) -> Result<(), CompileError> {
        if children.len() == 1 {
            self.emit(Instruction::Nil, Some(span.start));
            return Ok(());
        }
        let mut end_jumps = Vec::new();
        for child in &children[1..children.len() - 1] {
            self.compile_form(child.form, child.span, child.children, false)?;
            self.emit(Instruction::Dup, Some(child.span.start));
            let false_jump = self.emit(Instruction::JumpIfFalse(0), Some(child.span.start));
            end_jumps.push(self.emit(Instruction::Jump(0), Some(child.span.start)));
            let next = self.ctx().code.len();
            self.patch_jump(false_jump, next);
            self.emit(Instruction::Pop, Some(child.span.start));
        }
        let last = children.last().expect("or has an argument");
        self.compile_form(last.form, last.span, last.children, tail)?;
        let end = self.ctx().code.len();
        for jump in end_jumps {
            self.patch_jump(jump, end);
        }
        Ok(())
    }

    fn compile_cond(
        &mut self,
        children: &[Child<'_>],
        span: &Span,
        tail: bool,
    ) -> Result<(), CompileError> {
        let clauses = &children[1..];
        if clauses.len() % 2 != 0 {
            return Err(CompileError::new(
                CompileErrorKind::Arity,
                "cond expects test/expression pairs",
                Some(span.start),
            ));
        }
        if clauses.is_empty() {
            self.emit(Instruction::Nil, Some(span.start));
            return Ok(());
        }
        let mut end_jumps = Vec::new();
        for pair in clauses.chunks(2) {
            self.compile_form(pair[0].form, pair[0].span, pair[0].children, false)?;
            let next_jump = self.emit(Instruction::JumpIfFalse(0), Some(pair[0].span.start));
            self.compile_form(pair[1].form, pair[1].span, pair[1].children, tail)?;
            end_jumps.push(self.emit(Instruction::Jump(0), Some(pair[1].span.start)));
            let next = self.ctx().code.len();
            self.patch_jump(next_jump, next);
        }
        self.emit(Instruction::Nil, Some(span.start));
        let end = self.ctx().code.len();
        for jump in end_jumps {
            self.patch_jump(jump, end);
        }
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
                ));
            };
            self.compile_form(
                initializer.form,
                initializer.span,
                initializer.children,
                false,
            )?;
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
        let result = self.compile_bindings(children, "loop").and_then(|slots| {
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

    fn finish(mut self) -> Result<Program, CompileError> {
        if self.ctx().fallthrough {
            self.emit(Instruction::Return, None);
        }
        self.close_context();
        let mut program = Program {
            var_metadata: self.var_metadata,
            constants: self.constants,
            functions: self.functions,
            entry: 0,
        };
        // The shared analysis computes each operand-stack high-water mark;
        // full validation then runs over the whole program before it is
        // returned. Handler entry depths are patched from the same pass.
        for index in 0..program.functions.len() {
            let heights = stack_heights(&program, &program.functions[index])
                .map_err(|error| internal(error.to_string()))?;
            program.functions[index].max_stack = heights.iter().copied().max().unwrap_or(0);
            for entry_index in 0..program.functions[index].handlers.len() {
                let start = program.functions[index].handlers[entry_index].start as usize;
                program.functions[index].handlers[entry_index].depth = heights[start];
            }
        }
        validate::validate(&program).map_err(|error| internal(error.to_string()))?;
        Ok(program)
    }
}

fn constant_form(form: &Form) -> bool {
    match form {
        Form::Nil
        | Form::Bool(_)
        | Form::Number(_)
        | Form::Float(_)
        | Form::BigInteger(_)
        | Form::Decimal(_)
        | Form::Character(_)
        | Form::Regex(_)
        | Form::Keyword(_)
        | Form::String(_) => true,
        Form::Tagged(_, value) | Form::Metadata(_, value) => constant_form(value),
        Form::Vector(values) | Form::Set(values) => values.iter().all(constant_form),
        Form::Map(entries) => entries
            .iter()
            .all(|(key, value)| constant_form(key) && constant_form(value)),
        Form::Symbol(_) | Form::List(_) => false,
    }
}

fn quoted(form: &Form) -> Form {
    Form::List(vec![Form::Symbol("quote".into()), form.clone()])
}

fn unquote_argument(form: &Form, operator: &str) -> Option<Result<Form, String>> {
    let Form::List(parts) = crate::core::form_without_metadata(form) else {
        return None;
    };
    if !matches!(parts.first(), Some(Form::Symbol(name)) if name == operator) {
        return None;
    }
    Some(if parts.len() == 2 {
        Ok(parts[1].clone())
    } else {
        Err(format!("{operator} expects one argument"))
    })
}

fn lower_syntax_sequence(values: &[Form], vector: bool) -> Result<Form, String> {
    let mut expressions = Vec::with_capacity(values.len());
    let mut segments = Vec::with_capacity(values.len());
    let mut spliced = false;
    for value in values {
        if let Some(argument) = unquote_argument(value, "unquote-splicing") {
            spliced = true;
            segments.push(argument?);
        } else {
            let expression = lower_syntax_quote(value, true)?;
            expressions.push(expression.clone());
            segments.push(Form::Vector(vec![expression]));
        }
    }
    if !spliced {
        return if vector {
            Ok(Form::Vector(expressions))
        } else {
            Ok(Form::List(
                std::iter::once(Form::Symbol("list".into()))
                    .chain(expressions)
                    .collect(),
            ))
        };
    }
    let concatenated = Form::List(
        std::iter::once(Form::Symbol("concat".into()))
            .chain(segments)
            .collect(),
    );
    Ok(if vector {
        Form::List(vec![Form::Symbol("vec".into()), concatenated])
    } else {
        Form::List(vec![
            Form::Symbol("apply".into()),
            Form::Symbol("list".into()),
            concatenated,
        ])
    })
}

fn lower_syntax_quote(form: &Form, nested: bool) -> Result<Form, String> {
    if let Some(argument) = unquote_argument(form, "unquote") {
        return argument;
    }
    if unquote_argument(form, "unquote-splicing").is_some() {
        return Err(if nested {
            "unquote-splicing is only valid inside a collection".into()
        } else {
            "unquote-splicing is not valid at the root of syntax-quote".into()
        });
    }
    Ok(match crate::core::form_without_metadata(form) {
        Form::List(values) => lower_syntax_sequence(values, false)?,
        Form::Vector(values) => lower_syntax_sequence(values, true)?,
        Form::Map(entries) => Form::Map(
            entries
                .iter()
                .map(|(key, value)| {
                    Ok((lower_syntax_quote(key, true)?, lower_syntax_quote(value, true)?))
                })
                .collect::<Result<Vec<_>, String>>()?,
        ),
        Form::Set(values) => Form::Set(
            values
                .iter()
                .map(|value| lower_syntax_quote(value, true))
                .collect::<Result<Vec<_>, String>>()?,
        ),
        _ => quoted(form),
    })
}

fn internal(message: String) -> CompileError {
    CompileError::new(CompileErrorKind::Internal, message, None)
}
