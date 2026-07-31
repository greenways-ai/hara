//! Compile-time lexical scope stack: symbol-to-slot mappings with slot
//! reuse across sibling scopes. Runtime frames are plain `Vec<Value>`
//! indexed by slot; no string-keyed lookup survives compilation.

use crate::vm::error::{CompileError, CompileErrorKind};
use crate::vm::program::MAX_LOCALS;
use std::collections::HashMap;

struct Scope {
    names: HashMap<String, u16>,
    /// `next_slot` when the scope was pushed; restored on pop so sibling
    /// scopes reuse slots.
    saved_next: u16,
}

#[derive(Default)]
pub struct ScopeStack {
    scopes: Vec<Scope>,
    next_slot: u16,
    high_water: u16,
}

impl ScopeStack {
    pub fn new() -> ScopeStack {
        ScopeStack::default()
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(Scope {
            names: HashMap::new(),
            saved_next: self.next_slot,
        });
    }

    pub fn pop_scope(&mut self) {
        let scope = self.scopes.pop().expect("balanced scope stack");
        self.next_slot = scope.saved_next;
    }

    /// Allocates a fresh slot for `name` in the innermost scope. A repeated
    /// name in the same scope shadows its earlier binding (matching the
    /// evaluator's sequential `let`).
    pub fn declare(&mut self, name: &str) -> Result<u16, CompileError> {
        let scope = self.scopes.last_mut().expect("function scope is open");
        let slot = self.next_slot;
        if usize::from(slot) >= MAX_LOCALS {
            return Err(CompileError::new(
                CompileErrorKind::Limit,
                format!("local slots exceed limit of {MAX_LOCALS}"),
                None,
            ));
        }
        self.next_slot += 1;
        self.high_water = self.high_water.max(self.next_slot);
        scope.names.insert(name.to_string(), slot);
        Ok(slot)
    }

    /// Allocates a fresh slot in the innermost scope without registering a
    /// name: internal compiler state (try pending slots) that no user
    /// symbol can resolve or shadow.
    pub fn declare_hidden(&mut self) -> Result<u16, CompileError> {
        let slot = self.next_slot;
        if usize::from(slot) >= MAX_LOCALS {
            return Err(CompileError::new(
                CompileErrorKind::Limit,
                format!("local slots exceed limit of {MAX_LOCALS}"),
                None,
            ));
        }
        self.next_slot += 1;
        self.high_water = self.high_water.max(self.next_slot);
        Ok(slot)
    }

    /// Resolves a symbol innermost-first.
    pub fn resolve(&self, name: &str) -> Option<u16> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.names.get(name).copied())
    }

    /// Maximum simultaneously live slots; becomes the frame's
    /// `local_count`.
    pub fn high_water(&self) -> u16 {
        self.high_water
    }
}
