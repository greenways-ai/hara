//! Program representation for the experimental bytecode VM.
//!
//! Constants reuse `core::Value` directly: the VM does not duplicate the
//! Hara value model. Programs are in-memory only; there is no persistent
//! bytecode artifact in this milestone (HIR remains the form serializer).

use super::opcode::Instruction;
use super::source_map::SourceMap;
use crate::core::Value;

/// Maximum number of entries in the constant pool.
pub const MAX_CONSTANTS: usize = 1 << 24;
/// Maximum number of instructions per function prototype.
pub const MAX_INSTRUCTIONS: usize = 1 << 24;
/// Maximum number of local slots per frame (inherent to the `u16` operands).
pub const MAX_LOCALS: usize = u16::MAX as usize;
/// Maximum computed operand-stack depth for any function.
pub const MAX_OPERAND_STACK: usize = 4096;
/// Maximum number of arguments in one primitive call (`u8` operand).
pub const MAX_PRIMITIVE_ARGUMENTS: usize = u8::MAX as usize;

/// Index of a function prototype inside [`Program::functions`].
pub type FunctionId = u16;

/// A compiled function. Milestone 1 only ever emits the entry function;
/// the structure already matches what the closure milestone will need.
#[derive(Debug, Clone)]
pub struct FunctionPrototype {
    pub name: Option<String>,
    /// Required argument count. Always 0 for the entry function today.
    pub arity: u16,
    /// Number of local slots the frame allocates.
    pub local_count: u16,
    /// Declared operand-stack high-water mark; the validator recomputes
    /// and verifies it.
    pub max_stack: u16,
    pub code: Vec<Instruction>,
    pub source_map: SourceMap,
}

/// A compiled program: a constant pool plus function prototypes.
#[derive(Debug, Clone)]
pub struct Program {
    pub constants: Vec<Value>,
    pub functions: Vec<FunctionPrototype>,
    pub entry: FunctionId,
}

impl Program {
    /// The prototype execution starts from.
    pub fn entry_function(&self) -> &FunctionPrototype {
        &self.functions[self.entry as usize]
    }
}
