//! Whole-function WebAssembly compiler.
//!
//! Unlike the tracing experiment, this tier consumes a complete validated
//! bytecode [`Program`](crate::vm::Program), constructs typed basic-block IR,
//! and emits a deterministic portable Wasm module. HNW1 retains the HBC4
//! program as the semantic fallback and binds it to the generated module.

#[path = "whole_wasm/artifact.rs"]
mod artifact;
#[path = "whole_wasm/codegen.rs"]
mod codegen;
#[path = "whole_wasm/handles.rs"]
mod handles;
#[path = "whole_wasm/ir.rs"]
pub mod ir;
#[path = "whole_wasm/reps.rs"]
mod reps;
#[cfg(not(target_arch = "wasm32"))]
#[path = "whole_wasm/runtime.rs"]
mod runtime;

pub use artifact::{compile_artifact, decode_artifact, NativeArtifact, HNW_ABI_VERSION};
pub use codegen::compile_program;
pub use ir::{lower_program, MirBlock, MirFunction, MirOp, MirProgram, MirTerminator, Rep};
#[cfg(not(target_arch = "wasm32"))]
pub use runtime::NativeModule;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::{compile_artifact, decode_artifact, NativeModule};
    use crate::vm::compile_source;

    fn module(source: &str) -> NativeModule {
        let program = compile_source(source).expect("source must compile to bytecode");
        let bytes = compile_artifact(&program).expect("bytecode must compile to HNW1");
        NativeModule::load(&bytes).expect("HNW1 must load")
    }

    #[test]
    fn whole_function_loop_executes_without_vm_dispatch() {
        let mut native = module("(loop [i 0 acc 0] (if (< i 5000) (recur (+ i 1) (+ acc i)) acc))");
        assert_eq!(native.call_entry_i64(), Ok(12_497_500));
    }

    #[test]
    fn artifact_is_deterministic_authenticated_and_retains_hbc_fallback() {
        let program = compile_source("(+ 19 23)").unwrap();
        let first = compile_artifact(&program).unwrap();
        assert_eq!(first, compile_artifact(&program).unwrap());
        let decoded = decode_artifact(&first).unwrap();
        assert_eq!(decoded.program.entry, program.entry);
        assert!(decoded.wasm.starts_with(b"\0asm"));
        let mut corrupt = first;
        let index = corrupt.len() / 2;
        corrupt[index] ^= 1;
        assert_eq!(
            decode_artifact(&corrupt).unwrap_err(),
            "native artifact checksum mismatch"
        );
    }

    #[test]
    fn arithmetic_errors_match_hara_semantics() {
        assert_eq!(
            module("(/ 1 0)").call_entry_i64(),
            Err("division by zero".into())
        );
        assert_eq!(
            module("(+ 9223372036854775807 1)").call_entry_i64(),
            Err("integer overflow".into())
        );
        assert_eq!(
            module("(* -9223372036854775808 -1)").call_entry_i64(),
            Err("integer overflow".into())
        );
    }

    #[test]
    fn point_sensitive_representations_preserve_numeric_truthiness() {
        assert_eq!(module("(if 0 19 23)").call_entry_i64(), Ok(19));
        assert_eq!(module("(if false 19 23)").call_entry_i64(), Ok(23));
    }

    #[test]
    fn non_escaping_mutable_arrays_use_wasm_linear_memory() {
        let source = "(let [a (std.native.Arr/new 1 2 3)]
                        (std.native.Arr/set-index a 1 40)
                        (+ (std.native.Arr/get-index a 0)
                           (+ (std.native.Arr/get-index a 1) 1)))";
        assert_eq!(module(source).call_entry_i64(), Ok(42));
    }

    #[test]
    fn wasm_linear_arrays_preserve_bounds_errors() {
        assert_eq!(
            module("(std.native.Arr/get-index (std.native.Arr/new 1 2) 2)").call_entry_i64(),
            Err("array index out of bounds".into())
        );
        assert_eq!(
            module("(std.native.Arr/get-index (std.native.Arr/new 1 2) -1)").call_entry_i64(),
            Err("array index out of bounds".into())
        );
    }
}
