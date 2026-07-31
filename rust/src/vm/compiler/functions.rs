//! Function and call compilation: `fn` closures with by-value captures
//! (including variadic parameters) and direct calls. Global forms live
//! in `globals.rs` (issue #223). Split from `compiler.rs` to stay under
//! the repository's per-file line cap.

use crate::core::Primitive;
use crate::kernel::{Form, Position, Span};
use crate::vm::error::{CompileError, CompileErrorKind};
use crate::vm::opcode::Instruction;
use crate::vm::program::{FunctionPrototype, MAX_CAPTURES};
use crate::vm::source_map::SourceMap;

use super::scope::ScopeStack;
use super::{placeholder, Child, Compiler, FnContext};

impl Compiler {
    /// A call whose operator is a symbol: a lexical slot holding a
    /// function value, a visible global (late-bound through the var,
    /// issue #223), or an unbound symbol.
    pub(super) fn compile_named_call(
        &mut self,
        name: &str,
        children: &[Child<'_>],
        span: &Span,
    ) -> Result<(), CompileError> {
        let argc = (children.len() - 1) as u8;
        match self.ctx().scopes.resolve(name) {
            Some(slot) => self.emit(Instruction::LoadLocal(slot), Some(span.start)),
            None if self.visible_global(name) => {
                let index = self.name_constant(name, span)?;
                self.emit(Instruction::GetGlobal(index), Some(span.start))
            }
            None => {
                return Err(CompileError::new(
                    CompileErrorKind::UnboundSymbol,
                    format!("unbound symbol: {name}"),
                    Some(span.start),
                ))
            }
        };
        self.compile_call_arguments(children, span)?;
        if !self.ctx().fallthrough {
            return Ok(());
        }
        self.emit(Instruction::Call { argc }, Some(span.start));
        Ok(())
    }

    /// A call whose operator is itself an expression, e.g. `((fn [x] x) 1)`.
    pub(super) fn compile_expression_call(
        &mut self,
        children: &[Child<'_>],
        span: &Span,
    ) -> Result<(), CompileError> {
        let callee = &children[0];
        self.compile_form(callee.form, callee.span, callee.children, false)?;
        if !self.ctx().fallthrough {
            return Ok(());
        }
        let argc = (children.len() - 1) as u8;
        self.compile_call_arguments(children, span)?;
        if !self.ctx().fallthrough {
            return Ok(());
        }
        self.emit(Instruction::Call { argc }, Some(span.start));
        Ok(())
    }

    pub(super) fn compile_fn_form(
        &mut self,
        children: &[Child<'_>],
        span: &Span,
    ) -> Result<(), CompileError> {
        if children.len() < 3 {
            return Err(CompileError::new(
                CompileErrorKind::Arity,
                "fn expects parameters and a body",
                Some(span.start),
            ));
        }
        self.compile_function(None, &children[1], &children[2..], span)
    }

    /// Compiles a `fn`/`defn` body into a new function context and emits
    /// the closure creation (capture loads + `Closure`) into the
    /// enclosing context. `name` names the function value (display and
    /// errors); self-references resolve as globals, not captures
    /// (issue #223).
    pub(super) fn compile_function(
        &mut self,
        name: Option<&str>,
        params: &Child<'_>,
        body: &[Child<'_>],
        span: &Span,
    ) -> Result<(), CompileError> {
        let elements: &[Form] = match params.form {
            Form::Vector(elements) => elements,
            Form::List(_) => {
                return Err(CompileError::new(
                    CompileErrorKind::UnsupportedForm,
                    "fn multi-arity is not supported",
                    Some(params.span.start),
                ))
            }
            _ => {
                return Err(CompileError::new(
                    CompileErrorKind::Arity,
                    "function parameters must be a vector",
                    Some(params.span.start),
                ))
            }
        };
        let param_children = self.list_children(elements, params.span, params.children);
        let mut names: Vec<String> = Vec::with_capacity(elements.len());
        let mut rest_at: Option<usize> = None;
        for param in &param_children {
            match param.form {
                Form::Symbol(name) if name == "&" => {
                    if rest_at.is_some() {
                        return Err(CompileError::new(
                            CompileErrorKind::Arity,
                            "function parameters support a single & rest parameter",
                            Some(param.span.start),
                        ));
                    }
                    rest_at = Some(names.len());
                }
                Form::Symbol(name) => names.push(name.clone()),
                _ => {
                    return Err(CompileError::new(
                        CompileErrorKind::UnsupportedForm,
                        "fn destructuring is not supported",
                        Some(param.span.start),
                    ))
                }
            }
        }
        // With `& rest`, the fixed arity is the count before `&` and the
        // rest parameter occupies the slot directly above the fixed
        // params (captures sit above it), matching `call_function`.
        let (arity, variadic) = match rest_at {
            Some(fixed) => {
                if names.len() != fixed + 1 {
                    return Err(CompileError::new(
                        CompileErrorKind::Arity,
                        "the & rest parameter must be the last parameter",
                        Some(params.span.start),
                    ));
                }
                (fixed as u16, true)
            }
            None => (names.len() as u16, false),
        };
        // Free variables become capture slots directly above the params.
        let mut free: Vec<(String, Option<Position>)> = Vec::new();
        {
            let mut bound = names.clone();
            for child in body {
                self.collect_free(child, &mut bound, &mut free);
            }
        }
        if free.len() > MAX_CAPTURES {
            return Err(CompileError::new(
                CompileErrorKind::Limit,
                format!("closures support at most {MAX_CAPTURES} captures"),
                Some(span.start),
            ));
        }
        let proto_id = self.functions.len();
        self.functions.push(placeholder(
            name.map(str::to_string),
            arity,
            free.len() as u16,
            variadic,
        ));
        let mut scopes = ScopeStack::new();
        scopes.push_scope();
        // `names` includes the rest parameter; the param-child zip only
        // covers the fixed params plus `&`, so positions come from the
        // name symbols where possible.
        for (name, param) in names.iter().zip(&param_children) {
            scopes.declare(name).map_err(|error| {
                CompileError::new(error.kind(), error.message(), Some(param.span.start))
            })?;
        }
        for (name, position) in &free {
            scopes.declare(name).map_err(|error| {
                CompileError::new(error.kind(), error.message(), *position)
            })?;
        }
        self.contexts.push(FnContext {
            proto_id,
            name: name.map(str::to_string),
            params: arity,
            variadic,
            captures: free,
            code: Vec::new(),
            source_map: SourceMap::default(),
            scopes,
            loops: Vec::new(),
            tries: Vec::new(),
            handlers: Vec::new(),
            fallthrough: true,
        });
        let compiled = self.compile_sequence(body, true);
        if compiled.is_ok() && self.ctx().fallthrough {
            self.emit(Instruction::Return, None);
        }
        let context = self.close_context();
        compiled?;
        // The closure is created in the enclosing context: load each
        // capture (pre-declared as a binding in every intermediate
        // function, so one resolution step suffices), then the function
        // value itself.
        for (name, position) in &context.captures {
            let Some(slot) = self.ctx().scopes.resolve(name) else {
                return Err(CompileError::new(
                    CompileErrorKind::UnboundSymbol,
                    format!("unbound symbol: {name}"),
                    *position,
                ));
            };
            self.emit(Instruction::LoadLocal(slot), *position);
        }
        self.emit(
            Instruction::Closure {
                prototype: proto_id as u16,
                captures: context.captures.len() as u8,
            },
            Some(span.start),
        );
        Ok(())
    }

    /// Pops the current function context and fills in its reserved
    /// prototype. Returns the context for its capture list.
    pub(super) fn close_context(&mut self) -> FnContext {
        let context = self.contexts.pop().expect("balanced context stack");
        self.functions[context.proto_id] = FunctionPrototype {
            name: context.name.clone(),
            arity: context.params,
            variadic: context.variadic,
            capture_count: context.captures.len() as u16,
            local_count: context.scopes.high_water(),
            max_stack: 0,
            code: context.code.clone(),
            source_map: context.source_map.clone(),
            handlers: context.handlers.clone(),
        };
        context
    }

    /// Free-variable pre-pass: collects symbol references inside `child`
    /// that are not bound within the current function (params, let's,
    /// loops, and nested fn params are bound; special-form and primitive
    /// operators are not references). First-occurrence order, deduped.
    fn collect_free(
        &self,
        child: &Child<'_>,
        bound: &mut Vec<String>,
        free: &mut Vec<(String, Option<Position>)>,
    ) {
        match child.form {
            Form::Symbol(name) => {
                if bound.iter().any(|b| b == name)
                    || free.iter().any(|(f, _)| f == name)
                    || self.visible_global(name)
                {
                    // Bound locally, already collected, or a global:
                    // globals compile to GetGlobal (late binding through
                    // the shared var cell, issue #223), never captures.
                } else {
                    free.push((name.clone(), Some(child.span.start)));
                }
            }
            Form::List(elements) if !elements.is_empty() => {
                let children = self.list_children(elements, child.span, child.children);
                match &elements[0] {
                    Form::Symbol(head) => match head.as_str() {
                        "if" | "do" | "recur" | "try" | "throw" | "finally" => {
                            for c in &children[1..] {
                                self.collect_free(c, bound, free);
                            }
                        }
                        // Catch clauses bind their name symbol over the
                        // clause body; the class symbol is not a
                        // reference. Shapes mirror the compiler:
                        // (catch name body) or (catch Class name body...).
                        "catch" => {
                            let marked = bound.len();
                            if children.len() == 3 {
                                if let Form::Symbol(name) = children[1].form {
                                    bound.push(name.clone());
                                }
                                for c in &children[2..] {
                                    self.collect_free(c, bound, free);
                                }
                            } else if children.len() >= 4 {
                                if let Form::Symbol(name) = children[2].form {
                                    bound.push(name.clone());
                                }
                                for c in &children[3..] {
                                    self.collect_free(c, bound, free);
                                }
                            } else {
                                // Malformed: rejected by the compiler
                                // later; walk conservatively.
                                for c in &children[1..] {
                                    self.collect_free(c, bound, free);
                                }
                            }
                            bound.truncate(marked);
                        }
                        "let" | "loop" => {
                            if children.len() >= 2 {
                                let bindings = &children[1];
                                match bindings.form {
                                    Form::Vector(pair_forms) | Form::List(pair_forms) => {
                                        let pairs = self.list_children(
                                            pair_forms,
                                            bindings.span,
                                            bindings.children,
                                        );
                                        // Initializers see only earlier
                                        // bindings (sequential `let`);
                                        // each name binds right after
                                        // its initializer.
                                        let marked = bound.len();
                                        for pair in pairs.chunks(2) {
                                            if let [name, initializer] = pair {
                                                self.collect_free(initializer, bound, free);
                                                if let Form::Symbol(name) = name.form {
                                                    bound.push(name.clone());
                                                }
                                            }
                                        }
                                        for c in &children[2..] {
                                            self.collect_free(c, bound, free);
                                        }
                                        bound.truncate(marked);
                                    }
                                    _ => {
                                        for c in &children[1..] {
                                            self.collect_free(c, bound, free);
                                        }
                                    }
                                }
                            }
                        }
                        "fn" => {
                            let marked = bound.len();
                            match children.get(1).map(|c| c.form) {
                                Some(Form::Vector(params)) => {
                                    for param in params {
                                        if let Form::Symbol(name) = param {
                                            bound.push(name.clone());
                                        }
                                    }
                                    for c in &children[2..] {
                                        self.collect_free(c, bound, free);
                                    }
                                }
                                _ => {
                                    for c in &children[1..] {
                                        self.collect_free(c, bound, free);
                                    }
                                }
                            }
                            bound.truncate(marked);
                        }
                        // Rejected by the compiler later; nothing to collect.
                        "defn" | "var" => {}
                        _ if Primitive::from_symbol(head).is_some()
                            && !self.visible_global(head) =>
                        {
                            for c in &children[1..] {
                                self.collect_free(c, bound, free);
                            }
                        }
                        _ => {
                            // A function call: the operator is a
                            // reference unless it is a visible global.
                            if !bound.iter().any(|b| b == head)
                                && !free.iter().any(|(f, _)| f == head)
                                && !self.visible_global(head)
                            {
                                free.push((head.clone(), Some(children[0].span.start)));
                            }
                            for c in &children[1..] {
                                self.collect_free(c, bound, free);
                            }
                        }
                    },
                    _ => {
                        for c in &children {
                            self.collect_free(c, bound, free);
                        }
                    }
                }
            }
            Form::Metadata(_, value) => {
                let wrapped = Child {
                    form: value,
                    span: child.span,
                    children: None,
                };
                self.collect_free(&wrapped, bound, free);
            }
            _ => {}
        }
    }
}
