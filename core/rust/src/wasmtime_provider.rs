#![cfg(not(target_arch = "wasm32"))]

use std::cell::RefCell;

use wasmtime::{Engine, Instance, Module, Store, Val};

use crate::core::Value;
use crate::extension::{ExtensionManifest, WasmAbi, WasmExtensionProvider};

struct Session {
    store: Store<()>,
    instance: Instance,
}

/// Process-shareable compiled code. Hosts can store one of these per artifact
/// digest and creates a fresh provider/store for every session that loads it.
#[derive(Clone)]
pub struct CompiledWasmModule {
    engine: Engine,
    module: Module,
}

impl CompiledWasmModule {
    pub fn compile(bytes: &[u8]) -> Result<Self, String> {
        let engine = Engine::default();
        let module = Module::new(&engine, bytes)
            .map_err(|error| format!("extension/module-invalid: {error}"))?;
        if module.imports().next().is_some() {
            return Err("extension/module-invalid: extension modules must be import-free".into());
        }
        Ok(Self { engine, module })
    }

    pub fn provider(&self) -> WasmtimeExtensionProvider {
        WasmtimeExtensionProvider {
            engine: self.engine.clone(),
            module: self.module.clone(),
            session: RefCell::new(None),
        }
    }
}

/// Import-free Wasmtime host for the direct scalar core.v1 ABI.
pub struct WasmtimeExtensionProvider {
    engine: Engine,
    module: Module,
    session: RefCell<Option<Session>>,
}

impl WasmtimeExtensionProvider {
    pub fn compile(bytes: &[u8]) -> Result<Self, String> {
        Ok(CompiledWasmModule::compile(bytes)?.provider())
    }
}

impl WasmExtensionProvider for WasmtimeExtensionProvider {
    fn supports(&self, abi: WasmAbi) -> bool {
        abi == WasmAbi::CoreV1
    }

    fn start(&self, manifest: &ExtensionManifest) -> Result<(), String> {
        if !manifest.capabilities.is_empty() {
            return Err(format!(
                "extension/capability-denied: {:?} for {}",
                manifest.capabilities, manifest.namespace
            ));
        }
        let mut store = Store::new(&self.engine, ());
        let instance = Instance::new(&mut store, &self.module, &[])
            .map_err(|error| format!("extension/module-invalid: {error}"))?;
        for (name, _) in &manifest.exports {
            let function = instance
                .get_func(&mut store, name)
                .ok_or_else(|| format!("extension/malformed: module has no export {name}"))?;
            if function.ty(&store).results().len() > 1 {
                return Err(format!(
                    "extension/abi-type-unsupported: {name} has multiple results"
                ));
            }
        }
        *self.session.borrow_mut() = Some(Session { store, instance });
        Ok(())
    }

    fn invoke(
        &self,
        manifest: &ExtensionManifest,
        export: &str,
        arguments: &[Value],
    ) -> Result<Value, String> {
        let specification = manifest
            .exports
            .iter()
            .find(|(name, _)| name == export)
            .map(|(_, specification)| specification)
            .ok_or_else(|| format!("extension/export-missing: {export}"))?;
        let mut session = self.session.borrow_mut();
        let session = session
            .as_mut()
            .ok_or_else(|| format!("extension/not-started: {}", manifest.namespace))?;
        let function = session
            .instance
            .get_func(&mut session.store, export)
            .ok_or_else(|| format!("extension/export-missing: {export}"))?;
        let values = specification
            .arguments
            .iter()
            .zip(arguments)
            .map(|(wire_type, value)| argument(export, wire_type, value))
            .collect::<Result<Vec<_>, _>>()?;
        let mut results = if specification.returns == "void" {
            Vec::new()
        } else {
            vec![default_result(&specification.returns)?]
        };
        function
            .call(&mut session.store, &values, &mut results)
            .map_err(|error| {
                format!(
                    "extension/invoke-failed: {}/{} ({error})",
                    manifest.namespace, export
                )
            })?;
        result(export, &specification.returns, results.into_iter().next())
    }

    fn cancel(&self, _manifest: &ExtensionManifest, _request: u64) -> Result<(), String> {
        Err("extension/cancel-unsupported: core.v1 calls are synchronous".into())
    }

    fn shutdown(&self, _manifest: &ExtensionManifest) {
        self.session.borrow_mut().take();
    }
}

fn argument(export: &str, wire_type: &str, value: &Value) -> Result<Val, String> {
    let type_error = || format!("extension/type-error: {export} expects {wire_type}");
    match (wire_type, value) {
        ("i32", Value::Number(value)) => i32::try_from(*value)
            .map(Val::I32)
            .map_err(|_| type_error()),
        ("i64", Value::Number(value)) => Ok(Val::I64(*value)),
        ("f32", Value::Float(value)) => Ok(Val::F32((*value as f32).to_bits())),
        ("f32", Value::Number(value)) => Ok(Val::F32((*value as f32).to_bits())),
        ("f64", Value::Float(value)) => Ok(Val::F64(value.to_bits())),
        ("f64", Value::Number(value)) => Ok(Val::F64((*value as f64).to_bits())),
        ("boolean", Value::Bool(value)) => Ok(Val::I32(i32::from(*value))),
        _ => Err(type_error()),
    }
}

fn default_result(wire_type: &str) -> Result<Val, String> {
    match wire_type {
        "i32" | "boolean" => Ok(Val::I32(0)),
        "i64" => Ok(Val::I64(0)),
        "f32" => Ok(Val::F32(0)),
        "f64" => Ok(Val::F64(0)),
        _ => Err(format!("extension/abi-type-unsupported: {wire_type}")),
    }
}

fn result(export: &str, wire_type: &str, value: Option<Val>) -> Result<Value, String> {
    match (wire_type, value) {
        ("void", None) => Ok(Value::Nil),
        ("i32", Some(Val::I32(value))) => Ok(Value::Number(i64::from(value))),
        ("i64", Some(Val::I64(value))) => Ok(Value::Number(value)),
        ("f32", Some(Val::F32(value))) => Ok(Value::Float(f32::from_bits(value) as f64)),
        ("f64", Some(Val::F64(value))) => Ok(Value::Float(f64::from_bits(value))),
        ("boolean", Some(Val::I32(value))) => Ok(Value::Bool(value != 0)),
        _ => Err(format!(
            "extension/abi-type-unsupported: {export} -> {wire_type}"
        )),
    }
}
