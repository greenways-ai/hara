//! Experimental staged bytecode VM for the Rust runtime (issue #195).
//!
//! Milestone 1 is synchronous and closure-free: it compiles literals,
//! lexical locals, arithmetic, comparisons, `if`, `do`, `let`, and
//! `loop`/`recur` into a typed instruction program and executes it on a
//! stack machine. See `notes/rust-bytecode-vm.md` for the design.
//!
//! Everything in this module is gated behind the non-default `bytecode-vm`
//! Cargo feature. The VM never replaces `Runtime::eval_native` and never
//! falls back to the tree-walking evaluator: unsupported forms are typed
//! compile errors.

#[path = "vm/opcode.rs"]
pub mod opcode;
#[path = "vm/program.rs"]
pub mod program;
#[path = "vm/source_map.rs"]
pub mod source_map;
#[path = "vm/error.rs"]
pub mod error;
#[path = "vm/validate.rs"]
pub mod validate;
#[path = "vm/disassemble.rs"]
pub mod disassemble;

#[cfg(test)]
#[path = "vm/tests.rs"]
mod tests;

pub use disassemble::disassemble;
pub use error::{CompileError, CompileErrorKind, ValidationError, VmError};
pub use opcode::Instruction;
pub use program::{FunctionId, FunctionPrototype, Program};
pub use validate::validate;
