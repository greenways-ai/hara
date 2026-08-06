use super::{reps, NativeModule, Rep};
use crate::core::Value;
use crate::vm::FunctionId;

const INVOCATION_ARGUMENTS_ERROR: &str =
    "hta/invocation-malformed: expected an HTA sequence of arguments";
const INVOCATION_ABI_ERROR: &str =
    "hta/invocation-abi: whole-Wasm function must declare handle-backed arguments and result";

impl NativeModule {
    /// Calls a whole-Wasm function through the portable HTA1 value boundary.
    ///
    /// `request` is one HTA1-encoded Hara list, tuple, or vector containing
    /// the function arguments. The result is returned as one HTA1 frame.
    /// Internally, decoded values use the process-local scoped arena so calls
    /// between compiled Hara functions do not repeatedly encode or decode.
    ///
    /// This first portable adapter accepts functions whose declared arguments
    /// and result use the dynamic handle representation. Scalar kernels retain
    /// the existing `call_i64` fast path.
    pub fn call_hta(&mut self, function: FunctionId, request: &[u8]) -> Result<Vec<u8>, String> {
        ensure_hta_value_abi(self, function)?;
        let arguments = decode_arguments(request)?;
        let result = self.call_value(function, &arguments)?;
        crate::hta::encode(&result)
    }
}

fn decode_arguments(request: &[u8]) -> Result<Vec<Value>, String> {
    match crate::hta::decode(request)? {
        Value::List(values) => Ok(values.iter().cloned().collect()),
        Value::Tuple(values) => Ok(values.iter().cloned().collect()),
        Value::Vector(values) => Ok(values.iter().cloned().collect()),
        _ => Err(INVOCATION_ARGUMENTS_ERROR.into()),
    }
}

fn ensure_hta_value_abi(module: &NativeModule, function: FunctionId) -> Result<(), String> {
    let program = &module.artifact().program;
    let prototype = program
        .functions
        .get(usize::from(function))
        .ok_or_else(|| format!("unknown whole-Wasm function {function}"))?;

    let parameters_are_handles = (0..usize::from(prototype.arity)).all(|parameter| {
        reps::declared_parameter_rep(program, function, prototype, parameter)
            == Some(Rep::TruthyHandle)
    });
    let result_is_handle = reps::declared_result_rep(program, function) == Some(Rep::TruthyHandle);

    if parameters_are_handles && result_is_handle {
        Ok(())
    } else {
        Err(format!("{INVOCATION_ABI_ERROR}: {function}"))
    }
}
