//! One execution frame: the local slot array plus its operand-stack base.

use super::slot::VmSlot;

#[derive(Debug)]
pub struct Frame {
    locals: Vec<VmSlot>,
    base: usize,
}

impl Frame {
    /// The frame the machine starts with: all slots initialized to `nil`.
    pub(crate) fn entry(local_count: usize) -> Frame {
        Frame {
            locals: vec![VmSlot::Nil; local_count],
            base: 0,
        }
    }

    /// The frame for a function call: `args` fill the parameter slots
    /// `0..arity`, `captures` the capture slots directly above them, and
    /// the remaining slots start as `nil`. Out-of-range writes are dropped
    /// rather than panicking (the validator guarantees they fit; this
    /// defends hand-built programs).
    pub(crate) fn call(
        local_count: usize,
        arity: usize,
        args: Vec<VmSlot>,
        captures: Vec<VmSlot>,
        base: usize,
    ) -> Frame {
        Self::call_reusing(Vec::new(), local_count, arity, args, captures, base)
    }

    pub(crate) fn call_reusing(
        mut locals: Vec<VmSlot>,
        local_count: usize,
        arity: usize,
        args: Vec<VmSlot>,
        captures: Vec<VmSlot>,
        base: usize,
    ) -> Frame {
        locals.clear();
        locals.resize(local_count, VmSlot::Nil);
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
        Frame { locals, base }
    }

    pub(crate) fn into_locals(self) -> Vec<VmSlot> {
        self.locals
    }

    pub(crate) fn local(&self, slot: u16) -> Option<&VmSlot> {
        self.locals.get(usize::from(slot))
    }

    /// Clones the `count` slots starting at `start`; `None` when the range
    /// exceeds the frame (rejected by validation, defended here).
    pub(crate) fn slot_range(&self, start: usize, count: usize) -> Option<Vec<VmSlot>> {
        let end = start.checked_add(count)?;
        if end > self.locals.len() {
            return None;
        }
        Some(self.locals[start..end].to_vec())
    }

    /// Stores a value into a slot; false when the slot is out of range
    /// (rejected by validation, defended here so the machine never
    /// panics on malformed programs).
    pub(crate) fn store(&mut self, slot: u16, value: VmSlot) -> bool {
        match self.locals.get_mut(usize::from(slot)) {
            Some(cell) => {
                *cell = value;
                true
            }
            None => false,
        }
    }

    /// Operand-stack base at which this frame was entered.
    pub(crate) fn base(&self) -> usize {
        self.base
    }
}
