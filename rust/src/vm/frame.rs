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

    pub fn local(&self, slot: u16) -> Option<&Value> {
        self.locals.get(usize::from(slot))
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
