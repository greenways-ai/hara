//! Experimental staged bytecode VM for the Rust runtime (issue #195).
//!
//! Milestone 4 compiles literals, lexical locals, arithmetic,
//! comparisons, `if`, `do`, `let`, `loop`/`recur`, `fn` closures
//! (including variadic), exceptions, and the registry-direct global
//! forms (`def`, `defn`/`defn-` single- and multi-arity, `var`, `set!`,
//! `declare`, `defstruct`, `field`, `instance?`) into a typed
//! instruction program and executes it on a stack machine (issue #223).
//! See `notes/rust-bytecode-vm.md` for the design.
//!
//! Everything in this module is gated behind the non-default `bytecode-vm`
//! Cargo feature. The VM never replaces `Runtime::eval_native` and never
//! falls back to the tree-walking evaluator: unsupported forms are typed
//! compile errors.

#[path = "vm/opcode.rs"]
pub mod opcode;
#[path = "vm/program.rs"]
pub mod program;
#[path = "vm/artifact.rs"]
pub mod artifact;
#[path = "vm/source_map.rs"]
pub mod source_map;
#[path = "vm/slot.rs"]
mod slot;
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
#[cfg(test)]
#[path = "vm/conformance_tests.rs"]
mod conformance_tests;

/// Normalizes an error message to a coarse category for comparison. The
/// fiber and the synchronous fallback phrase some shape errors
/// differently ("let expects bindings" vs "let expects a binding list or
/// vector"); each bucket covers every phrasing of one failure class.
/// Shared by the differential tests and the corpus-driven conformance
/// tests; the bucket names are pinned by
/// `specs/01-lang/010-bytecode/draft/conformance/bytecode-vm.edn`.
#[cfg(test)]
pub(crate) fn error_category(message: &str) -> &'static str {
    let buckets: &[(&[&str], &str)] = &[
        (&["division by zero"], "division by zero"),
        (&["integer overflow"], "integer overflow"),
        (&["expects numbers"], "expects numbers"),
        (&["expects at least", "expects arguments"], "primitive arity"),
        (&["expects 2 or 3 arguments"], "if arity"),
        (
            &["expects bindings and a body", "expects bindings and body"],
            "binding body shape",
        ),
        (
            &["expects a binding list or vector", "expects bindings"],
            "binding bindings shape",
        ),
        (&["require name/value pairs"], "binding pairs"),
        (&["function expects"], "function arity"),
        (&["value is not callable"], "not callable"),
        (&["function parameters must be a vector"], "fn params shape"),
        (&["throw expects one value"], "throw arity"),
        (&["thrown: "], "thrown"),
        // "unbound var" is checked first: its message contains "unbound
        // var", not "unbound symbol", so order is safe either way.
        (&["unbound var"], "unbound var"),
        (&["unbound symbol"], "unbound symbol"),
        (&["recur"], "recur"),
        (&["Invalid number"], "reader"),
        (&["EOF while reading"], "reader"),
    ];
    for (markers, bucket) in buckets {
        if markers.iter().any(|marker| message.contains(marker)) {
            return bucket;
        }
    }
    panic!("unclassified error message: {message}")
}

pub use compiler::{compile_source, compile_source_with};
pub use disassemble::disassemble;
pub use error::{CompileError, CompileErrorKind, ValidationError, VmError};
pub use machine::{execute_program, execute_program_with_globals, Machine, VmOutcome};
pub use opcode::Instruction;
pub use program::{FunctionId, FunctionPrototype, Program};
pub use artifact::{decode_program, encode_program};
pub use validate::validate;

/// Compiles, validates, and executes a closed source string in one step.
/// Errors from either stage flatten to their display form (which carries
/// source positions). No fallback to the tree-walking evaluator.
pub fn eval_source(source: &str) -> Result<crate::core::Value, String> {
    let program = compile_source(source).map_err(|error| error.to_string())?;
    execute_program(std::rc::Rc::new(program)).map_err(|error| error.to_string())
}
