//! Full Hara compilation surface, kept outside VM-only deployments.

pub use hara_vm::{Program, Value};
pub use hara_wasm::vm::{compile_halc_module, compile_source, compile_source_with, CompileError};

pub fn compile_bytecode(source: &str) -> Result<Vec<u8>, String> {
    let program = compile_source(source).map_err(|error| error.to_string())?;
    hara_wasm::vm::encode_program(&program)
}

#[cfg(feature = "full-wasm")]
pub fn compile_wasm(source: &str) -> Result<Vec<u8>, String> {
    let program = compile_source(source).map_err(|error| error.to_string())?;
    hara_wasm::whole_wasm::compile_artifact(&program)
}

#[cfg(test)]
mod tests {
    #[test]
    fn compiler_output_executes_in_vm_only_crate() {
        let artifact = super::compile_bytecode("(+ 19 23)").unwrap();
        assert_eq!(hara_vm::execute(&artifact).unwrap().display(), "42");
    }
}
