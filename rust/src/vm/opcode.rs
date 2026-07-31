//! Typed instruction set for the experimental bytecode VM.
//!
//! Instructions are a typed enum, not packed bytes: the milestone
//! prioritises exact validation and disassembly over encoding density.
//! Jump operands are absolute instruction indexes.

use crate::core::Primitive;

/// One VM instruction.
///
/// Stack effects (validated before execution):
///
/// - `Constant`, `Nil`, `True`, `False`, `LoadLocal`: push 1.
/// - `StoreLocal`, `Pop`, `JumpIfFalse`: pop 1.
/// - `Primitive`: pops `argc`, pushes 1 (net `1 - argc`).
/// - `Closure`: pops `captures`, pushes 1 (net `1 - captures`).
/// - `Call`: pops `argc` arguments plus the callee, pushes 1 (net `-argc`).
/// - `CallStatic`: pops `argc`, pushes 1 (net `1 - argc`).
/// - `Jump`: no change.
/// - `Throw`, `Rethrow`: pop 1; terminal (unwind).
/// - `Return`: terminal; requires stack height exactly 1.
#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    /// Pushes `constants[index]` onto the operand stack.
    Constant(u32),
    /// Pushes `Value::Nil`.
    Nil,
    /// Pushes `Value::Bool(true)`.
    True,
    /// Pushes `Value::Bool(false)`.
    False,
    /// Pushes the value of local slot `slot`.
    LoadLocal(u16),
    /// Pops the top of the stack into local slot `slot`.
    StoreLocal(u16),
    /// Discards the top of the stack.
    Pop,
    /// Pops `argc` arguments, applies the shared value-level primitive,
    /// and pushes the result.
    Primitive { op: Primitive, argc: u8 },
    /// Unconditional jump to an absolute instruction index.
    Jump(u32),
    /// Pops the condition and jumps when it is not truthy
    /// (`Value::truthy`: only `nil` and `false` are false).
    JumpIfFalse(u32),
    /// Pops `captures` captured values and pushes a function value for
    /// `prototype`.
    Closure { prototype: u16, captures: u8 },
    /// Pops `argc` arguments and then the callee, invokes the function
    /// value through the shared `call_function` boundary, and pushes the
    /// result.
    Call { argc: u8 },
    /// Pops `argc` arguments and calls `prototype` directly, copying the
    /// current frame's capture slots as the callee's captures (`defn`
    /// self-recursion).
    CallStatic { prototype: u16, argc: u8 },
    /// Pops one value and raises it as a guest exception through the
    /// shared `core::thrown_error` boundary. Terminal: unwinds to the
    /// innermost covering try entry or fails the run.
    Throw,
    /// Pops a string and raises that exact message without touching the
    /// thrown-value side channel, preserving error identity across an
    /// unmatched finally boundary. Terminal; only emitted in finally
    /// resume sequences.
    Rethrow,
    /// Returns the top of the stack as the function result.
    Return,
}

impl Instruction {
    /// The jump target, when the instruction transfers control.
    pub fn jump_target(&self) -> Option<u32> {
        match self {
            Instruction::Jump(target) | Instruction::JumpIfFalse(target) => Some(*target),
            _ => None,
        }
    }

    /// Whether control falls through to the next instruction.
    pub(crate) fn falls_through(&self) -> bool {
        !matches!(
            self,
            Instruction::Jump(_) | Instruction::Return | Instruction::Throw | Instruction::Rethrow
        )
    }

    /// Static stack effect of the instruction; `None` for the terminals
    /// (`Return` requires height exactly 1, `Throw`/`Rethrow` pop 1), which
    /// are validated separately.
    pub(crate) fn stack_effect(&self) -> Option<i32> {
        Some(match self {
            Instruction::Constant(_)
            | Instruction::Nil
            | Instruction::True
            | Instruction::False
            | Instruction::LoadLocal(_) => 1,
            Instruction::StoreLocal(_) | Instruction::Pop | Instruction::JumpIfFalse(_) => -1,
            Instruction::Primitive { argc, .. } | Instruction::CallStatic { argc, .. } => {
                1 - i32::from(*argc)
            }
            Instruction::Closure { captures, .. } => 1 - i32::from(*captures),
            Instruction::Call { argc } => -i32::from(*argc),
            Instruction::Jump(_) => 0,
            Instruction::Return | Instruction::Throw | Instruction::Rethrow => return None,
        })
    }
}

impl std::fmt::Display for Instruction {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::Constant(index) => write!(formatter, "Constant {index}"),
            Instruction::Nil => formatter.write_str("Nil"),
            Instruction::True => formatter.write_str("True"),
            Instruction::False => formatter.write_str("False"),
            Instruction::LoadLocal(slot) => write!(formatter, "LoadLocal {slot}"),
            Instruction::StoreLocal(slot) => write!(formatter, "StoreLocal {slot}"),
            Instruction::Pop => formatter.write_str("Pop"),
            Instruction::Primitive { op, argc } => {
                write!(formatter, "Primitive {} {argc}", op.operator())
            }
            Instruction::Jump(target) => write!(formatter, "Jump {target:04}"),
            Instruction::JumpIfFalse(target) => write!(formatter, "JumpIfFalse {target:04}"),
            Instruction::Closure { prototype, captures } => {
                write!(formatter, "Closure {prototype:04} captures {captures}")
            }
            Instruction::Call { argc } => write!(formatter, "Call {argc}"),
            Instruction::CallStatic { prototype, argc } => {
                write!(formatter, "CallStatic {prototype:04} {argc}")
            }
            Instruction::Throw => formatter.write_str("Throw"),
            Instruction::Rethrow => formatter.write_str("Rethrow"),
            Instruction::Return => formatter.write_str("Return"),
        }
    }
}
