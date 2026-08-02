use wasmtime::{Engine, Instance, Module, Store, Val};

use crate::vm::FunctionId;

use super::artifact::{decode_artifact, NativeArtifact};
use super::codegen::{ERROR_DIVISION_BY_ZERO, ERROR_INTEGER_OVERFLOW};

/// A validated HNW1 module instantiated by Wasmtime. Calls enter a generated
/// whole Wasm function directly; the bytecode program is retained as fallback
/// metadata, not interpreted on this path.
pub struct NativeModule {
    artifact: NativeArtifact,
    store: Store<()>,
    instance: Instance,
}

impl NativeModule {
    pub fn load(bytes: &[u8]) -> Result<Self, String> {
        let artifact = decode_artifact(bytes)?;
        let engine = Engine::default();
        let module = Module::new(&engine, &artifact.wasm).map_err(|error| error.to_string())?;
        let mut store = Store::new(&engine, ());
        let instance =
            Instance::new(&mut store, &module, &[]).map_err(|error| error.to_string())?;
        Ok(Self {
            artifact,
            store,
            instance,
        })
    }

    pub fn artifact(&self) -> &NativeArtifact {
        &self.artifact
    }

    pub fn call_i64(&mut self, function: FunctionId, arguments: &[i64]) -> Result<i64, String> {
        let (_, arity) = self
            .artifact
            .functions
            .get(usize::from(function))
            .ok_or_else(|| format!("unknown whole-Wasm function {function}"))?;
        if arguments.len() != usize::from(*arity) {
            return Err(format!(
                "whole-Wasm function {function} expects {arity} arguments, got {}",
                arguments.len()
            ));
        }
        let error = self
            .instance
            .get_global(&mut self.store, "hara_error")
            .ok_or("whole-Wasm module has no hara_error global")?;
        error
            .set(&mut self.store, Val::I32(0))
            .map_err(|error| error.to_string())?;
        let callable = self
            .instance
            .get_func(&mut self.store, &format!("hara_fn_{function}"))
            .ok_or_else(|| format!("whole-Wasm module has no function {function}"))?;
        let inputs = arguments.iter().copied().map(Val::I64).collect::<Vec<_>>();
        let mut outputs = [Val::I64(0)];
        match callable.call(&mut self.store, &inputs, &mut outputs) {
            Ok(()) => outputs[0]
                .i64()
                .ok_or_else(|| "whole-Wasm function returned a non-i64 result".into()),
            Err(trap) => {
                let code = error.get(&mut self.store).i32().unwrap_or_default();
                match code {
                    ERROR_INTEGER_OVERFLOW => Err("integer overflow".into()),
                    ERROR_DIVISION_BY_ZERO => Err("division by zero".into()),
                    _ => Err(format!("whole-Wasm trap: {trap}")),
                }
            }
        }
    }

    /// Calls the zero-arity entry through the initial scalar ABI. Returning a
    /// raw i64 is intentional: MIR result-representation metadata must exist
    /// before this boundary can faithfully construct a dynamic Hara `Value`.
    pub fn call_entry_i64(&mut self) -> Result<i64, String> {
        let entry = self.artifact.program.entry;
        self.call_i64(entry, &[])
    }
}
