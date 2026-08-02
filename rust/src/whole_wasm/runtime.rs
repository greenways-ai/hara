use std::cell::RefCell;
use std::rc::Rc;

use wasmtime::{Engine, FuncType, Instance, Linker, Module, Store, Val, ValType};

use crate::core::Value;
use crate::vm::FunctionId;

use super::artifact::{decode_artifact, NativeArtifact};
use super::codegen::{ERROR_DIVISION_BY_ZERO, ERROR_INTEGER_OVERFLOW};
use super::handles::{Handle, HandleScope};

#[derive(Default)]
struct HostState {
    handles: HandleScope,
}

/// A validated HNW1 module instantiated by Wasmtime. Calls enter a generated
/// whole Wasm function directly; the bytecode program is retained as fallback
/// metadata, not interpreted on this path.
pub struct NativeModule {
    artifact: NativeArtifact,
    store: Store<HostState>,
    instance: Instance,
}

impl NativeModule {
    pub fn load(bytes: &[u8]) -> Result<Self, String> {
        let artifact = decode_artifact(bytes)?;
        let engine = Engine::default();
        let module = Module::new(&engine, &artifact.wasm).map_err(|error| error.to_string())?;
        let mut store = Store::new(&engine, HostState::default());
        let mut linker = Linker::new(&engine);
        define_array_imports(&mut linker)?;
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|error| error.to_string())?;
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
        self.store.data_mut().handles.begin_call();
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

fn define_array_imports(linker: &mut Linker<HostState>) -> Result<(), String> {
    linker
        .func_new(
            "hara",
            "array_empty",
            FuncType::new([], [ValType::I64]),
            |mut caller, _, outputs| {
                let value = Value::Array(Rc::new(RefCell::new(Vec::new())));
                outputs[0] = Val::I64(
                    caller
                        .data_mut()
                        .handles
                        .insert(value)
                        .map_err(host_error)?
                        .to_abi(),
                );
                Ok(())
            },
        )
        .map_err(|error| error.to_string())?;
    linker
        .func_new(
            "hara",
            "array_push_i64",
            FuncType::new([ValType::I64, ValType::I64], [ValType::I64]),
            |caller, inputs, outputs| {
                let handle = Handle::from_abi(inputs[0].i64().unwrap());
                let value = inputs[1].i64().unwrap();
                match caller.data().handles.get(handle).map_err(host_error)? {
                    Value::Array(values) => values.borrow_mut().push(Value::Number(value)),
                    _ => return Err(host_error("whole-Wasm array handle expected".into())),
                }
                outputs[0] = Val::I64(handle.to_abi());
                Ok(())
            },
        )
        .map_err(|error| error.to_string())?;
    linker
        .func_new(
            "hara",
            "array_get_i64",
            FuncType::new([ValType::I64, ValType::I64], [ValType::I64]),
            |caller, inputs, outputs| {
                let handle = Handle::from_abi(inputs[0].i64().unwrap());
                let index = array_index(inputs[1].i64().unwrap())?;
                let result = match caller.data().handles.get(handle).map_err(host_error)? {
                    Value::Array(values) => values
                        .borrow()
                        .get(index)
                        .cloned()
                        .ok_or_else(|| host_error("array/get index out of bounds".into()))?,
                    _ => return Err(host_error("whole-Wasm array handle expected".into())),
                };
                let Value::Number(result) = result else {
                    return Err(host_error(
                        "whole-Wasm array element is not an integer".into(),
                    ));
                };
                outputs[0] = Val::I64(result);
                Ok(())
            },
        )
        .map_err(|error| error.to_string())?;
    linker
        .func_new(
            "hara",
            "array_set_i64",
            FuncType::new([ValType::I64, ValType::I64, ValType::I64], [ValType::I64]),
            |caller, inputs, outputs| {
                let handle = Handle::from_abi(inputs[0].i64().unwrap());
                let index = array_index(inputs[1].i64().unwrap())?;
                let value = inputs[2].i64().unwrap();
                match caller.data().handles.get(handle).map_err(host_error)? {
                    Value::Array(values) => {
                        let mut values = values.borrow_mut();
                        let slot = values
                            .get_mut(index)
                            .ok_or_else(|| host_error("array/set index out of bounds".into()))?;
                        *slot = Value::Number(value);
                    }
                    _ => return Err(host_error("whole-Wasm array handle expected".into())),
                }
                outputs[0] = Val::I64(handle.to_abi());
                Ok(())
            },
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn array_index(value: i64) -> Result<usize, wasmtime::Error> {
    usize::try_from(value).map_err(|_| host_error("array index must be non-negative".into()))
}

fn host_error(message: String) -> wasmtime::Error {
    wasmtime::Error::msg(message)
}
