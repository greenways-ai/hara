#[path = "../../src/vm/artifact.rs"]
pub mod artifact;
#[path = "../../src/vm/compile.rs"]
pub mod compile;
#[path = "../../src/vm/error.rs"]
pub mod error;
#[path = "../../src/vm/fiber.rs"]
pub mod fiber;
#[path = "../../src/vm/frame.rs"]
pub mod frame;
#[path = "../../src/vm/machine.rs"]
pub mod machine;
#[path = "../../src/vm/opcode.rs"]
pub mod opcode;
#[path = "../../src/vm/program.rs"]
pub mod program;
#[path = "../../src/vm/slot.rs"]
pub mod slot;
#[path = "../../src/vm/validate.rs"]
pub mod validate;
#[path = "session.rs"]
pub mod session;

pub use artifact::decode_program;
pub use compile::{compile_source, compile_source_with, disassemble_source};
pub use error::VmError;
pub use fiber::VmFiber;
pub use machine::{execute_program, Machine};
pub use program::Program;
pub use validate::validate;
