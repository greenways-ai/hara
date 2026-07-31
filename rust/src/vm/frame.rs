//! One execution frame: the local slot array plus the operand-stack base.
//! The base is reserved for the multi-frame call stack of the closure
//! milestone; milestone 1 runs exactly one frame.

use crate::core::Value;

#[derive(Debug)]
pub struct Frame {
    locals: Vec<Value>,
    base: usize,
}

impl Frame {
    /// The frame the machine starts with: all slots initialized to `nil`.
    pub fn entry(local_count: usize) -> Frame {
        Frame {
            locals: vec![Value::Nil; local_count],
            base: 0,
        }
    }

    /// The frame for a function call: `args` fill the parameter slots
    /// `0..arity`, `captures` the capture slots directly above them, and
    /// the remaining slots start as `nil`. Out-of-range writes are dropped
    /// rather than panicking (the validator guarantees they fit; this
    /// defends hand-built programs).
    pub fn call(
        local_count: usize,
        arity: usize,
        args: Vec<Value>,
        captures: Vec<Value>,
    ) -> Frame {
        let mut locals = vec![Value::Nil; local_count];
        for (index, value) in args.into_iter().enumerate() {
            if let Some(cell) = locals.get_mut(index) {
                *cell = value;
            }
        }
        for (index, value) in captures.into_iter().enumerate() {
            if let Some(cell) = locals.get_mut(arity + index) {
                *cell = value;
            }
        }
        Frame { locals, base: 0 }
    }

    pub fn local(&self, slot: u16) -> Option<&Value> {
        self.locals.get(usize::from(slot))
    }

    /// Clones the `count` slots starting at `start`; `None` when the range
    /// exceeds the frame (rejected by validation, defended here).
    pub fn slot_range(&self, start: usize, count: usize) -> Option<Vec<Value>> {
        let end = start.checked_add(count)?;
        if end > self.locals.len() {
            return None;
        }
        Some(self.locals[start..end].to_vec())
    }

    /// Stores a value into a slot; false when the slot is out of range
    /// (rejected by validation, defended here so the machine never
    /// panics on malformed programs).
    pub fn store(&mut self, slot: u16, value: Value) -> bool {
        match self.locals.get_mut(usize::from(slot)) {
            Some(cell) => {
                *cell = value;
                true
            }
            None => false,
        }
    }

    /// Operand-stack base of this frame; always 0 until function calls
    /// arrive with the closure milestone.
    pub fn base(&self) -> usize {
        self.base
    }
}
