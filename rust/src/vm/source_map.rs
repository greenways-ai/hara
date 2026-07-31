//! Per-instruction source positions for diagnostics and the disassembler.

use crate::kernel::Position;

/// Maps instruction indexes to the source position of the form that
/// produced them. Stored as a parallel vector so lookup during error
/// construction is a single index.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SourceMap {
    positions: Vec<Option<Position>>,
}

impl SourceMap {
    pub(crate) fn record(&mut self, position: Option<Position>) {
        self.positions.push(position);
    }

    pub(crate) fn pop(&mut self) {
        self.positions.pop();
    }

    /// The source position recorded for an instruction, when available.
    pub fn position(&self, instruction: usize) -> Option<Position> {
        self.positions.get(instruction).copied().flatten()
    }

    pub fn len(&self) -> usize {
        self.positions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }
}
