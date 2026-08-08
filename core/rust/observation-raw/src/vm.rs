#[path = "../../src/vm/artifact.rs"]
pub mod artifact;
#[path = "../../src/vm/bundle.rs"]
pub mod bundle;
#[path = "../../src/vm/compiler.rs"]
pub mod compiler;
#[path = "../../src/vm/disassemble.rs"]
pub mod disassemble;
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
#[path = "../../src/vm/prepared.rs"]
pub mod prepared;
#[path = "../../src/vm/program.rs"]
pub mod program;
#[path = "session.rs"]
pub mod session;
#[path = "../../src/vm/slot.rs"]
mod slot;
#[path = "../../src/vm/source_map.rs"]
pub mod source_map;
#[path = "../../src/vm/validate.rs"]
pub mod validate;

pub use artifact::decode_program;
pub use compiler::{compile_source, compile_source_with};
pub use error::VmError;
pub use fiber::VmFiber;
pub use machine::{execute_program, Machine};
pub use program::Program;
pub use validate::validate;
