//! Program representation for the experimental bytecode VM.
//!
//! Constants reuse `core::Value` directly: the VM does not duplicate the
//! Hara value model. The versioned `vm::artifact` codec persists validated
//! programs for packaging and browser startup without reparsing source.

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
/// Maximum number of captured values in one closure (`u8` operand).
pub const MAX_CAPTURES: usize = u8::MAX as usize;

/// Index of a function prototype inside [`Program::functions`].
pub type FunctionId = u16;

/// One `catch` clause of a [`TryEntry`]: the machine stores the caught
/// value into `binding` and jumps to `target` when `class` matches.
#[derive(Debug, Clone)]
pub struct CatchEntry {
    /// The dispatch class; `Exception` for the implicit 3-element shape.
    pub class: String,
    pub binding: u16,
    pub target: u32,
}

/// A static handler table entry: the protected range `[start, end)` and
/// its catch/finally regions. Registered outermost-first, so the machine's
/// reverse-order search finds the innermost covering entry.
#[derive(Debug, Clone)]
pub struct TryEntry {
    pub start: u32,
    pub end: u32,
    /// Operand-stack height at try entry; the machine truncates to this on
    /// unwind. Patched in after stack analysis and verified by validation.
    pub depth: u16,
    pub catches: Vec<CatchEntry>,
    pub finally: Option<u32>,
    /// Hidden slots holding the pending result (a value or an error
    /// message string) and the error flag; present exactly when `finally`
    /// is present.
    pub pending_value: Option<u16>,
    pub pending_error: Option<u16>,
}

/// A compiled function. The entry function has arity and capture count 0;
/// `fn`/`defn` forms contribute the remaining prototypes.
#[derive(Debug, Clone)]
pub struct FunctionPrototype {
    pub name: Option<String>,
    /// Calling this prototype creates a child async execution and returns
    /// its stable result promise instead of the direct body value.
    pub async_function: bool,
    /// Required argument count. Always 0 for the entry function. When
    /// `variadic` is set this counts only the fixed parameters.
    pub arity: u16,
    /// Whether the last parameter binds the remaining arguments as a
    /// `Value::List` (`[a b & rest]`).
    pub variadic: bool,
    /// Number of captured values the frame expects in the slots directly
    /// above the parameters. Always 0 for the entry function.
    pub capture_count: u16,
    /// Number of local slots the frame allocates.
    pub local_count: u16,
    /// Declared operand-stack high-water mark; the validator recomputes
    /// and verifies it.
    pub max_stack: u16,
    pub code: Vec<Instruction>,
    pub source_map: SourceMap,
    /// Static handler table for `try`/`catch`/`finally`; empty for
    /// functions without protected regions.
    pub handlers: Vec<TryEntry>,
}

/// A compiled program: a constant pool plus function prototypes.
#[derive(Debug, Clone)]
pub struct Program {
    pub constants: Vec<Value>,
    /// Hara metadata tables for `DefGlobal` (docstrings, attr maps,
    /// computed arglists), assembled at compile time from the source
    /// forms. Empty for programs without global definitions.
    pub var_metadata: Vec<std::rc::Rc<crate::lang::data::Metadata>>,
    pub functions: Vec<FunctionPrototype>,
    pub entry: FunctionId,
}

impl Program {
    /// The prototype execution starts from.
    pub fn entry_function(&self) -> &FunctionPrototype {
        &self.functions[self.entry as usize]
    }
}
