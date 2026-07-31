//! `loop`/`recur` compilation: simultaneous recurrence into loop
//! slots plus the tail-position and finally-crossing checks. Split
//! from `compiler.rs` to stay under the repository's per-file line cap.

use crate::kernel::Span;

use super::{Child, Compiler};
use crate::vm::error::{CompileError, CompileErrorKind};
use crate::vm::opcode::Instruction;

#[derive(Clone)]
pub(super) struct LoopContext {
    pub(super) header: usize,
    pub(super) slots: Vec<u16>,
}

impl Compiler {
    pub(super) fn compile_recur(
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
        // A recur targeting a loop opened before a try-with-finally must
        // leave the region, which would skip the finally. The evaluator
        // runs the finally per crossing; the general resume protocol is
        // frame-stack work, so the crossing is rejected (notes §18.6).
        // Checked before tail position: the try itself suppresses tail
        // propagation, and the crossing is the actionable error.
        let loop_index = self.ctx().loops.len() - 1;
        if self
            .ctx()
            .tries
            .iter()
            .any(|entry| entry.has_finally && loop_index < entry.loop_depth)
        {
            return Err(CompileError::new(
                CompileErrorKind::Recur,
                "recur cannot cross a finally boundary",
                Some(span.start),
            ));
        }
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
}
