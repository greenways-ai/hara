//! Hoplite's test-free embedding facade over the shared Hara VM sources.

#[path = "../../../src/vm/artifact.rs"]
pub mod artifact;
#[path = "../../../src/vm/compiler.rs"]
pub mod compiler;
#[path = "../../../src/vm/disassemble.rs"]
pub mod disassemble;
#[path = "../../../src/vm/error.rs"]
pub mod error;
#[path = "../../../src/vm/fiber.rs"]
pub mod fiber;
#[path = "../../../src/vm/frame.rs"]
pub mod frame;
#[path = "../../../src/vm/machine.rs"]
pub mod machine;
#[path = "../../../src/vm/opcode.rs"]
pub mod opcode;
#[path = "../../../src/vm/program.rs"]
pub mod program;
#[path = "../../../src/vm/slot.rs"]
mod slot;
#[path = "../../../src/vm/source_map.rs"]
pub mod source_map;
#[path = "../../../src/vm/validate.rs"]
pub mod validate;

pub use compiler::{compile_source, compile_source_with};
pub use disassemble::disassemble;
pub use fiber::{VmFiber, VmFiberState};
pub use machine::execute_program;
