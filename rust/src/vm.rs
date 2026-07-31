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
#[path = "vm/compiler.rs"]
pub mod compiler;
#[path = "vm/frame.rs"]
pub mod frame;
#[path = "vm/machine.rs"]
pub mod machine;

#[cfg(test)]
#[path = "vm/tests.rs"]
mod tests;
#[cfg(test)]
#[path = "vm/execution_tests.rs"]
mod execution_tests;
#[cfg(test)]
#[path = "vm/differential_tests.rs"]
mod differential_tests;

pub use compiler::compile_source;
pub use disassemble::disassemble;
pub use error::{CompileError, CompileErrorKind, ValidationError, VmError};
pub use machine::{execute_program, Machine, VmOutcome};
pub use opcode::Instruction;
pub use program::{FunctionId, FunctionPrototype, Program};
pub use validate::validate;

/// Compiles, validates, and executes a closed source string in one step.
/// Errors from either stage flatten to their display form (which carries
/// source positions). No fallback to the tree-walking evaluator.
pub fn eval_source(source: &str) -> Result<crate::core::Value, String> {
    let program = compile_source(source).map_err(|error| error.to_string())?;
    execute_program(&program).map_err(|error| error.to_string())
}
