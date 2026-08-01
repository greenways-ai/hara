//! Global-form compilation (issue #223): `def`, `defn`/`defn-`, `var`,
//! `set!`, `declare`, `defstruct`, `field`, and `instance?` lower to the
//! registry-direct global instructions. Names resolve in two phases:
//! compile-time visibility (program-declared globals plus the
//! compilation namespace registry) decides whether a symbol may compile
//! to a global reference at all; the emitted instructions resolve the
//! var through the registry again at execution time, so globals are
//! late-bound through the shared var cell — the same semantics the JVM
//! runtime and the fixed tree evaluator exhibit. Split from
//! `compiler.rs` to stay under the repository's per-file line cap.

use std::rc::Rc;

use crate::core::{binding_symbol, definition_metadata};
use crate::kernel::{Form, Span};
use crate::lang::data::Metadata;
use crate::lang::protocol::INamespaced;
use crate::vm::error::{CompileError, CompileErrorKind};
use crate::vm::opcode::Instruction;

use super::{Child, Compiler};

impl Compiler {
    fn require_owned_global(&self, name: &str, span: &Span) -> Result<(), CompileError> {
        let referred = crate::core::namespace_registry()
            .ok()
            .and_then(|registry| {
                let current = registry.current();
                current
                    .resolve(&crate::lang::data::Symbol::parse(name))
                    .map(|var| var.symbol().get_namespace() != Some(current.name().as_str()))
            })
            .unwrap_or(false);
        if referred {
            return Err(unsupported(
                format!("Cannot replace referred Var without ns omission: {name}"),
                span.start,
            ));
        }
        Ok(())
    }

    /// A name constant's pool index (no instruction emitted). Global
    /// operands are string names resolved at execution time.
    pub(super) fn name_constant(&mut self, name: &str, span: &Span) -> Result<u32, CompileError> {
        self.constant_index_of(crate::core::Value::String(name.to_string()), span)
    }

    /// Registers a name as program-declared: visible to global
    /// references compiled from this point on.
    pub(super) fn declare_program_global(&mut self, name: &str) {
        if !self.globals.iter().any(|global| global == name) {
            self.globals.push(name.to_string());
        }
    }

    /// Whether `name` may compile to a global reference: declared by
    /// this program, or resolvable through the compilation namespace
    /// registry (the Runtime path; the free `compile_source` path has
    /// none and only sees program-declared names).
    pub(super) fn visible_global(&self, name: &str) -> bool {
        self.globals.iter().any(|global| global == name)
            || crate::core::namespace_registry()
                .map(|registry| {
                    registry
                        .resolve(&crate::lang::data::Symbol::parse(name))
                        .is_some()
                })
                .unwrap_or(false)
    }

    /// Emits a `GetGlobal` for a visible name.
    pub(super) fn emit_get_global(&mut self, name: &str, span: &Span) -> Result<(), CompileError> {
        let index = self.name_constant(name, span)?;
        self.emit(Instruction::GetGlobal(index), Some(span.start));
        Ok(())
    }

    fn var_metadata(&mut self, metadata: Option<Rc<Metadata>>) -> Option<u16> {
        metadata.map(|metadata| {
            let index = self.var_metadata.len() as u16;
            self.var_metadata.push(metadata);
            index
        })
    }

    /// `(def name init)`: interns the value in the current namespace and
    /// evaluates to it (matching the evaluator's `def` arm). The name
    /// becomes visible only after the initializer compiles, so an
    /// initializer cannot self-reference — matching the evaluator's
    /// "unbound symbol" for `(def x x)` on a fresh name.
    pub(super) fn compile_def(
        &mut self,
        children: &[Child<'_>],
        span: &Span,
    ) -> Result<(), CompileError> {
        if children.len() != 3 {
            return Err(CompileError::new(
                CompileErrorKind::Arity,
                "def expects a name and value",
                Some(span.start),
            ));
        }
        let (name, metadata) = binding_symbol(children[1].form, "def name")
            .map_err(|message| unsupported(message, children[1].span.start))?;
        self.require_owned_global(&name, children[1].span)?;
        let metadata = self.var_metadata(metadata);
        let initializer = &children[2];
        self.compile_form(
            initializer.form,
            initializer.span,
            initializer.children,
            false,
        )?;
        if !self.ctx().fallthrough {
            return Ok(());
        }
        self.declare_program_global(&name);
        let name_index = self.name_constant(&name, children[1].span)?;
        self.emit(
            Instruction::DefGlobal {
                name: name_index,
                metadata,
            },
            Some(span.start),
        );
        Ok(())
    }

    /// `(set! name value)`: resets a global var's root and evaluates to
    /// the value. A lexical name is a compile error: the VM does not
    /// cell-wrap lexical bindings, unlike the tree evaluator (a
    /// documented divergence). An invisible name is "unbound var".
    pub(super) fn compile_set(
        &mut self,
        children: &[Child<'_>],
        span: &Span,
    ) -> Result<(), CompileError> {
        if children.len() != 3 {
            return Err(CompileError::new(
                CompileErrorKind::Arity,
                "set! expects a name and value",
                Some(span.start),
            ));
        }
        let Form::Symbol(name) = children[1].form else {
            return Err(unsupported(
                "set! expects a name symbol",
                children[1].span.start,
            ));
        };
        if self.ctx().scopes.resolve(name).is_some() {
            return Err(unsupported(
                format!("set! targets a global var: {name} is a lexical binding"),
                children[1].span.start,
            ));
        }
        if !self.visible_global(name) {
            return Err(CompileError::new(
                CompileErrorKind::UnboundSymbol,
                format!("unbound var: {name}"),
                Some(children[1].span.start),
            ));
        }
        self.require_owned_global(name, children[1].span)?;
        let value = &children[2];
        self.compile_form(value.form, value.span, value.children, false)?;
        if !self.ctx().fallthrough {
            return Ok(());
        }
        let index = self.name_constant(name, children[1].span)?;
        self.emit(Instruction::SetGlobal(index), Some(span.start));
        Ok(())
    }

    /// `(var name)` (also the `#'name` reader form): pushes the var
    /// itself. Lexical bindings are not vars; an invisible name is
    /// "unbound var".
    pub(super) fn compile_var(
        &mut self,
        children: &[Child<'_>],
        span: &Span,
    ) -> Result<(), CompileError> {
        if children.len() != 2 {
            return Err(CompileError::new(
                CompileErrorKind::Arity,
                "var expects a name symbol",
                Some(span.start),
            ));
        }
        let Form::Symbol(name) = children[1].form else {
            return Err(unsupported(
                "var expects a name symbol",
                children[1].span.start,
            ));
        };
        if !self.visible_global(name) {
            return Err(CompileError::new(
                CompileErrorKind::UnboundSymbol,
                format!("unbound var: {name}"),
                Some(children[1].span.start),
            ));
        }
        let index = self.name_constant(name, children[1].span)?;
        self.emit(Instruction::VarGlobal(index), Some(span.start));
        Ok(())
    }

    /// `(declare name ...)`: interns a nil var per name without resetting
    /// any existing binding. It supplies forward visibility only; namespace
    /// omission, not declare, controls whether a referred name may be replaced.
    /// Top-level statements only
    /// (stricter than the evaluator, documented); evaluates to nil.
    pub(super) fn compile_declare(
        &mut self,
        children: &[Child<'_>],
        span: &Span,
        top: bool,
    ) -> Result<(), CompileError> {
        if !top {
            return Err(unsupported(
                "declare is only supported as a top-level statement",
                span.start,
            ));
        }
        let names = &children[1..];
        if names.is_empty() {
            self.emit(Instruction::Nil, Some(span.start));
            return Ok(());
        }
        for (index, child) in names.iter().enumerate() {
            let Form::Symbol(name) = child.form else {
                return Err(CompileError::new(
                    CompileErrorKind::Arity,
                    "declare expects name symbols",
                    Some(child.span.start),
                ));
            };
            self.require_owned_global(name, child.span)?;
            self.declare_program_global(name);
            let constant = self.name_constant(name, child.span)?;
            self.emit(Instruction::DeclareGlobal(constant), Some(child.span.start));
            if index + 1 != names.len() {
                self.emit(Instruction::Pop, Some(child.span.start));
            }
        }
        Ok(())
    }

    /// `(defstruct Name [field ...])`: interns the type and its
    /// constructor vars and evaluates to nil, matching the evaluator.
    pub(super) fn compile_defstruct(
        &mut self,
        children: &[Child<'_>],
        span: &Span,
    ) -> Result<(), CompileError> {
        if children.len() < 3 {
            return Err(CompileError::new(
                CompileErrorKind::Arity,
                "defstruct expects a name and field vector",
                Some(span.start),
            ));
        }
        let Form::Symbol(name) = children[1].form else {
            return Err(unsupported(
                "defstruct name must be an unqualified symbol",
                children[1].span.start,
            ));
        };
        if name.contains('/') {
            return Err(unsupported(
                "defstruct name must be an unqualified symbol",
                children[1].span.start,
            ));
        }
        let fields: &[Form] = match children[2].form {
            Form::Vector(fields) => fields,
            _ => {
                return Err(CompileError::new(
                    CompileErrorKind::Arity,
                    "defstruct expects a field vector",
                    Some(children[2].span.start),
                ))
            }
        };
        let mut names: Vec<crate::core::Value> = Vec::with_capacity(fields.len());
        for field in fields {
            let Form::Symbol(field) = field else {
                return Err(unsupported(
                    "defstruct field names must be unqualified symbols",
                    children[2].span.start,
                ));
            };
            if field.contains('/') {
                return Err(unsupported(
                    "defstruct field names must be unqualified symbols",
                    children[2].span.start,
                ));
            }
            if names
                .iter()
                .any(|candidate| matches!(candidate, crate::core::Value::String(c) if c == field))
            {
                return Err(unsupported(
                    "Duplicate defstruct field",
                    children[2].span.start,
                ));
            }
            names.push(crate::core::Value::String(field.clone()));
        }
        let name_index = self.name_constant(name, children[1].span)?;
        let fields_index = self.constant_index_of(
            crate::core::Value::Vector(names.into_iter().collect()),
            span,
        )?;
        self.declare_program_global(name);
        self.declare_program_global(&format!("->{name}"));
        self.declare_program_global(&format!("map->{name}"));
        self.emit(
            Instruction::DefStruct {
                name: name_index,
                fields: fields_index,
            },
            Some(span.start),
        );
        Ok(())
    }

    /// `(field instance :name)`: positional struct field access; the
    /// field name is a literal keyword or symbol.
    pub(super) fn compile_field(
        &mut self,
        children: &[Child<'_>],
        span: &Span,
    ) -> Result<(), CompileError> {
        if children.len() != 3 {
            return Err(CompileError::new(
                CompileErrorKind::Arity,
                "field expects a struct and field name",
                Some(span.start),
            ));
        }
        let field = match children[2].form {
            Form::Keyword(field) | Form::Symbol(field) if !field.contains('/') => field,
            _ => {
                return Err(unsupported(
                    "field name must be an unqualified keyword or symbol",
                    children[2].span.start,
                ))
            }
        };
        let instance = &children[1];
        self.compile_form(instance.form, instance.span, instance.children, false)?;
        if !self.ctx().fallthrough {
            return Ok(());
        }
        let index = self.name_constant(field, children[2].span)?;
        self.emit(Instruction::StructField(index), Some(span.start));
        Ok(())
    }

    /// `(instance? type value)`: struct type membership.
    pub(super) fn compile_instance_of(
        &mut self,
        children: &[Child<'_>],
        span: &Span,
    ) -> Result<(), CompileError> {
        if children.len() != 3 {
            return Err(CompileError::new(
                CompileErrorKind::Arity,
                "instance? expects a struct type and value",
                Some(span.start),
            ));
        }
        for argument in &children[1..] {
            self.compile_form(argument.form, argument.span, argument.children, false)?;
        }
        if !self.ctx().fallthrough {
            return Ok(());
        }
        self.emit(Instruction::InstanceOf, Some(span.start));
        Ok(())
    }

    /// `(defn name ...)` / `(defn- name ...)`: interns a real var holding
    /// the function (single arity) or arity dispatcher (multiple
    /// clauses) and evaluates to the var, matching the evaluator.
    /// Top-level statements only; the name is visible before the bodies
    /// compile, so self- and mutual-recursion within the form resolve
    /// through the var (late binding). Referred Vars remain owned by their
    /// defining namespace and must be omitted before a local definition.
    pub(super) fn compile_defn(
        &mut self,
        children: &[Child<'_>],
        span: &Span,
        top: bool,
        private: bool,
    ) -> Result<(), CompileError> {
        if !top {
            return Err(unsupported(
                "defn is only supported as a top-level statement",
                span.start,
            ));
        }
        if children.len() < 4 {
            return Err(CompileError::new(
                CompileErrorKind::Arity,
                "defn expects a name, parameters, and a body",
                Some(span.start),
            ));
        }
        let (name, metadata) = binding_symbol(children[1].form, "defn name")
            .map_err(|message| unsupported(format!("{message}"), children[1].span.start))?;
        self.require_owned_global(&name, children[1].span)?;
        // `definition_metadata` works on the raw forms; the surviving
        // `rest` is a suffix of the elements, so the matching children
        // (with spans) are the same suffix of `children`.
        let raw: Vec<Form> = children.iter().map(|child| child.form.clone()).collect();
        let (metadata, rest) = definition_metadata(metadata, &raw[2..], private, false)
            .map_err(|message| unsupported(format!("{name}: {message}"), children[1].span.start))?;
        let offset = children.len() - rest.len();
        let rest_children = &children[offset..];
        if rest_children.is_empty() {
            return Err(CompileError::new(
                CompileErrorKind::Arity,
                "defn expects a name, parameters, and a body",
                Some(span.start),
            ));
        }
        let async_function = metadata.as_ref().is_some_and(|value| value.flag("async"));
        let metadata = self.var_metadata(metadata);
        self.declare_program_global(&name);
        let single_arity = matches!(
            crate::core::form_without_metadata(rest_children[0].form),
            Form::Vector(_)
        );
        if single_arity {
            self.compile_function(
                Some(&name),
                &rest_children[0],
                &rest_children[1..],
                span,
                async_function,
                async_function,
            )?;
        } else {
            // Multi-arity: each clause is a list `(params body...)`.
            let mut count = 0usize;
            for clause in rest_children {
                let clause_forms: &[Form] = match clause.form {
                    Form::List(forms) => forms,
                    _ => {
                        return Err(unsupported(
                            "defn multi-arity clauses must be lists",
                            clause.span.start,
                        ))
                    }
                };
                let clause_children =
                    self.list_children(clause_forms, clause.span, clause.children);
                if clause_children.len() < 2 {
                    return Err(CompileError::new(
                        CompileErrorKind::Arity,
                        "defn clause expects parameters and a body",
                        Some(clause.span.start),
                    ));
                }
                self.compile_function(
                    None,
                    &clause_children[0],
                    &clause_children[1..],
                    span,
                    async_function,
                    async_function,
                )?;
                count += 1;
                if count > u8::MAX as usize {
                    return Err(CompileError::new(
                        CompileErrorKind::Limit,
                        "defn supports at most 255 arity clauses",
                        Some(span.start),
                    ));
                }
            }
            let name_constant = self.name_constant(&name, children[1].span)?;
            self.emit(
                Instruction::MakeMultiArity {
                    name: name_constant,
                    count: count as u8,
                },
                Some(span.start),
            );
        }
        if !self.ctx().fallthrough {
            return Ok(());
        }
        let name_index = self.name_constant(&name, children[1].span)?;
        self.emit(
            Instruction::DefGlobal {
                name: name_index,
                metadata,
            },
            Some(span.start),
        );
        self.emit(Instruction::Pop, Some(span.start));
        self.emit(Instruction::VarGlobal(name_index), Some(span.start));
        Ok(())
    }
}

fn unsupported(message: impl Into<String>, position: crate::kernel::Position) -> CompileError {
    CompileError::new(
        CompileErrorKind::UnsupportedForm,
        message.into(),
        Some(position),
    )
}
