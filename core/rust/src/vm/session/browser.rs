use js_sys::{JSON, Reflect};
use wasm_bindgen::prelude::*;

use crate::core::{PromiseState, Value};
use crate::task::PromiseRejection;

use super::{
    BytecodeObservationSession as NativeSession, BytecodeSessionError, ObservationLimits,
    SessionRetentionLimits,
};

#[wasm_bindgen(js_name = BytecodeObservationSession)]
pub struct BrowserBytecodeObservationSession {
    inner: NativeSession,
}

#[wasm_bindgen]
impl BrowserBytecodeObservationSession {
    #[wasm_bindgen(constructor)]
    pub fn new(source: &str) -> Result<BrowserBytecodeObservationSession, JsValue> {
        NativeSession::compile(source)
            .map(|inner| Self { inner })
            .map_err(js_session_error)
    }

    #[wasm_bindgen(js_name = compileNamed)]
    pub fn compile_named(
        session_id: &str,
        source_id: &str,
        source: &str,
    ) -> Result<BrowserBytecodeObservationSession, JsValue> {
        NativeSession::compile_named(session_id, source_id, source)
            .map(|inner| Self { inner })
            .map_err(js_session_error)
    }

    #[wasm_bindgen(js_name = fromArtifact)]
    pub fn from_artifact(bytes: &[u8]) -> Result<BrowserBytecodeObservationSession, JsValue> {
        NativeSession::from_artifact(bytes)
            .map(|inner| Self { inner })
            .map_err(js_session_error)
    }

    #[wasm_bindgen(js_name = fromNamedArtifact)]
    pub fn from_named_artifact(
        session_id: &str,
        source_id: &str,
        bytes: &[u8],
    ) -> Result<BrowserBytecodeObservationSession, JsValue> {
        NativeSession::from_artifact_named(session_id, source_id, bytes)
            .map(|inner| Self { inner })
            .map_err(js_session_error)
    }

    #[wasm_bindgen(getter, js_name = sessionId)]
    pub fn session_id(&self) -> String {
        self.inner.session_id().into()
    }

    #[wasm_bindgen(getter, js_name = sourceId)]
    pub fn source_id(&self) -> String {
        self.inner.source_id().into()
    }

    #[wasm_bindgen(getter, js_name = traceId)]
    pub fn trace_id(&self) -> String {
        self.inner.trace_id().into()
    }

    #[wasm_bindgen(getter)]
    pub fn status(&self) -> String {
        self.inner.status().as_keyword().into()
    }

    #[wasm_bindgen(getter)]
    pub fn sequence(&self) -> f64 {
        self.inner.sequence().min(9_007_199_254_740_991) as f64
    }

    pub fn snapshot(&self) -> Result<JsValue, JsValue> {
        self.inner
            .snapshot_value()
            .map_err(js_session_error)
            .and_then(to_js)
    }

    pub fn step(&mut self) -> Result<JsValue, JsValue> {
        self.inner
            .step()
            .map_err(js_session_error)
            .and_then(to_js)
    }

    pub fn run(&mut self, step_limit: u32) -> Result<JsValue, JsValue> {
        self.inner
            .run(step_limit as usize)
            .map_err(js_session_error)
            .and_then(to_js)
    }

    pub fn pause(&mut self) -> bool {
        self.inner.pause()
    }

    /// Resumes a paused session when `settlement` is `undefined`/`null`, or a
    /// suspended promise session from one of these JSON-safe shapes:
    /// `{status:"pending"}`, `{status:"fulfilled", value:...}`, or
    /// `{status:"rejected", error:...}`. With no shape, a suspended session
    /// reads the retained promise's current settlement.
    pub fn resume(&mut self, settlement: JsValue) -> Result<JsValue, JsValue> {
        let settlement = if settlement.is_undefined() || settlement.is_null() {
            None
        } else {
            Some(settlement_state(&settlement)?)
        };
        self.inner
            .resume(settlement)
            .map_err(js_session_error)
            .and_then(to_js)
    }

    #[wasm_bindgen(js_name = resolveSuspension)]
    pub fn resolve_suspension(&self, value: JsValue) -> Result<bool, JsValue> {
        self.inner
            .resolve_suspension(from_js(&value)?)
            .map_err(js_session_error)
    }

    #[wasm_bindgen(js_name = rejectSuspension)]
    pub fn reject_suspension(&self, error: JsValue) -> Result<bool, JsValue> {
        self.inner
            .reject_suspension(from_js(&error)?)
            .map_err(js_session_error)
    }

    #[wasm_bindgen(js_name = suspensionState)]
    pub fn suspension_state(&self) -> Option<String> {
        self.inner
            .suspended_promise()
            .map(|promise| match promise.state() {
                PromiseState::Pending => "pending".into(),
                PromiseState::Fulfilled(_) => "fulfilled".into(),
                PromiseState::Rejected(_) => "rejected".into(),
            })
    }

    pub fn reset(&mut self) -> Result<JsValue, JsValue> {
        self.inner
            .reset()
            .map_err(js_session_error)
            .and_then(to_js)
    }

    pub fn metrics(&self) -> Result<JsValue, JsValue> {
        to_js(self.inner.metrics())
    }

    pub fn events(&self) -> Result<JsValue, JsValue> {
        to_js(self.inner.events())
    }

    pub fn trace(&self) -> Result<JsValue, JsValue> {
        to_js(self.inner.trace())
    }

    #[wasm_bindgen(js_name = resultDisplay)]
    pub fn result_display(&self) -> Option<String> {
        self.inner.result().map(Value::display)
    }

    #[wasm_bindgen(js_name = errorMessage)]
    pub fn error_message(&self) -> Option<String> {
        self.inner.error().map(|error| error.to_string())
    }

    #[wasm_bindgen(js_name = setObservationLimits)]
    pub fn set_observation_limits(
        &mut self,
        stack: u32,
        locals: u32,
        calls: u32,
        handlers: u32,
        display_chars: u32,
    ) {
        self.inner.set_observation_limits(ObservationLimits {
            stack: stack as usize,
            locals: locals as usize,
            calls: calls as usize,
            handlers: handlers as usize,
            display_chars: display_chars as usize,
        });
    }

    #[wasm_bindgen(js_name = setRetentionLimits)]
    pub fn set_retention_limits(&mut self, events: u32, trace: u32) {
        self.inner.set_retention_limits(SessionRetentionLimits {
            events: events as usize,
            trace: trace as usize,
        });
    }

    pub fn dispose(&mut self) -> bool {
        self.inner.dispose()
    }
}

fn settlement_state(value: &JsValue) -> Result<PromiseState, JsValue> {
    let status = Reflect::get(value, &JsValue::from_str("status"))?
        .as_string()
        .ok_or_else(|| js_error("bytecode settlement status must be a string"))?;
    match status.as_str() {
        "pending" => Ok(PromiseState::Pending),
        "fulfilled" => {
            let value = Reflect::get(value, &JsValue::from_str("value"))?;
            Ok(PromiseState::Fulfilled(from_js(&value)?))
        }
        "rejected" => {
            let error = Reflect::get(value, &JsValue::from_str("error"))?;
            Ok(PromiseState::Rejected(PromiseRejection::Value(from_js(
                &error,
            )?)))
        }
        other => Err(js_error(&format!(
            "unsupported bytecode settlement status: {other}"
        ))),
    }
}

fn from_js(value: &JsValue) -> Result<Value, JsValue> {
    let json = JSON::stringify(value)?
        .as_string()
        .ok_or_else(|| js_error("bytecode settlement is not JSON serializable"))?;
    crate::json::read(&json).map_err(|error| js_error(&error))
}

fn to_js(value: Value) -> Result<JsValue, JsValue> {
    let json = crate::json::write(&value).map_err(|error| js_error(&error))?;
    JSON::parse(&json)
}

fn js_session_error(error: BytecodeSessionError) -> JsValue {
    js_error(error.message())
}

fn js_error(message: &str) -> JsValue {
    JsValue::from_str(message)
}

