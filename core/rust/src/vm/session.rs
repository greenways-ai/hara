//! Live, browser-safe ownership of one observed bytecode execution.
//!
//! The session compiles or decodes a validated program once, retains the real
//! [`Machine`], and projects only the three established Hara evidence schemas:
//! metrics, compact events, and full trace steps. Runtime values and promises
//! never enter those documents; they remain owned by the session until reset or
//! deterministic disposal.

use std::cell::Cell;
use std::collections::VecDeque;
use std::rc::Rc;

use crate::core::{Promise, Value};
use crate::kernel::NamespaceRegistry;

use super::machine::observation::ObservationLimits;
use super::{compile_source_with, decode_program, validate, Machine, Program, VmError};

#[path = "session/evidence.rs"]
mod evidence;
#[path = "session/control.rs"]
mod control;
#[cfg(target_arch = "wasm32")]
#[path = "session/browser.rs"]
mod browser;
#[cfg(target_arch = "wasm32")]
pub use browser::BrowserBytecodeObservationSession;
#[cfg(test)]
#[path = "session/tests.rs"]
mod tests;

pub use evidence::{BYTECODE_EVENTS_SCHEMA, BYTECODE_METRICS_SCHEMA, BYTECODE_TRACE_SCHEMA};
use evidence::{CompactEventRecord, SessionMetrics, TraceStepRecord};

thread_local! {
    static NEXT_SESSION_ID: Cell<u64> = const { Cell::new(1) };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BytecodeSessionStatus {
    Ready,
    Running,
    Paused,
    Suspended,
    Returned,
    Failed,
    Disposed,
}

impl BytecodeSessionStatus {
    pub const fn as_keyword(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Suspended => "suspended",
            Self::Returned => "returned",
            Self::Failed => "failed",
            Self::Disposed => "disposed",
        }
    }

    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Returned | Self::Failed | Self::Disposed)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionRetentionLimits {
    pub events: usize,
    pub trace: usize,
}

impl Default for SessionRetentionLimits {
    fn default() -> Self {
        Self {
            events: 512,
            trace: 128,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BytecodeSessionError {
    message: String,
}

impl BytecodeSessionError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for BytecodeSessionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for BytecodeSessionError {}

pub struct BytecodeObservationSession {
    session_id: String,
    source_id: String,
    source: Option<String>,
    program: Option<Rc<Program>>,
    machine: Option<Machine>,
    registry: NamespaceRegistry<Value>,
    status: BytecodeSessionStatus,
    paused_from: Option<BytecodeSessionStatus>,
    observation_limits: ObservationLimits,
    retention_limits: SessionRetentionLimits,
    trace_generation: u64,
    trace_id: String,
    next_sequence: u64,
    metrics: SessionMetrics,
    events: VecDeque<CompactEventRecord>,
    trace_steps: VecDeque<TraceStepRecord>,
    dropped_events: u64,
    omitted_trace_steps: u64,
    result: Option<Value>,
    error: Option<VmError>,
    suspension: Option<Promise>,
}

impl BytecodeObservationSession {
    pub fn compile(source: impl Into<String>) -> Result<Self, BytecodeSessionError> {
        let session_id = next_session_id();
        let source_id = format!("{session_id}.hal");
        Self::compile_named(session_id, source_id, source)
    }

    pub fn compile_named(
        session_id: impl Into<String>,
        source_id: impl Into<String>,
        source: impl Into<String>,
    ) -> Result<Self, BytecodeSessionError> {
        let source = source.into();
        let registry = fresh_registry();
        let program = compile_source_with(&source, &registry)
            .map_err(|error| BytecodeSessionError::new(error.to_string()))?;
        Self::from_validated_program(
            session_id,
            source_id,
            Some(source),
            program,
            registry,
        )
    }

    pub fn from_artifact(bytes: &[u8]) -> Result<Self, BytecodeSessionError> {
        let session_id = next_session_id();
        let source_id = format!("{session_id}.hbc");
        Self::from_artifact_named(session_id, source_id, bytes)
    }

    pub fn from_artifact_named(
        session_id: impl Into<String>,
        source_id: impl Into<String>,
        bytes: &[u8],
    ) -> Result<Self, BytecodeSessionError> {
        let program = decode_program(bytes).map_err(BytecodeSessionError::new)?;
        Self::from_validated_program(
            session_id,
            source_id,
            None,
            program,
            fresh_registry(),
        )
    }

    pub fn from_program(
        session_id: impl Into<String>,
        source_id: impl Into<String>,
        program: Program,
    ) -> Result<Self, BytecodeSessionError> {
        validate(&program).map_err(|error| BytecodeSessionError::new(error.to_string()))?;
        Self::from_validated_program(
            session_id,
            source_id,
            None,
            program,
            fresh_registry(),
        )
    }

    fn from_validated_program(
        session_id: impl Into<String>,
        source_id: impl Into<String>,
        source: Option<String>,
        program: Program,
        registry: NamespaceRegistry<Value>,
    ) -> Result<Self, BytecodeSessionError> {
        let session_id = required_id(session_id.into(), "session id")?;
        let source_id = required_id(source_id.into(), "source id")?;
        let trace_generation = 0;
        let trace_id = trace_id(&session_id, trace_generation);
        let program = Rc::new(program);
        let machine = Machine::entry(program.clone());
        Ok(Self {
            session_id,
            source_id,
            source,
            program: Some(program),
            machine: Some(machine),
            registry,
            status: BytecodeSessionStatus::Ready,
            paused_from: None,
            observation_limits: ObservationLimits::default(),
            retention_limits: SessionRetentionLimits::default(),
            trace_generation,
            trace_id,
            next_sequence: 0,
            metrics: SessionMetrics::default(),
            events: VecDeque::new(),
            trace_steps: VecDeque::new(),
            dropped_events: 0,
            omitted_trace_steps: 0,
            result: None,
            error: None,
            suspension: None,
        })
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }

    pub fn trace_id(&self) -> &str {
        &self.trace_id
    }

    pub fn status(&self) -> BytecodeSessionStatus {
        self.status
    }

    pub fn sequence(&self) -> u64 {
        self.next_sequence
    }

    pub fn observation_limits(&self) -> ObservationLimits {
        self.observation_limits
    }

    pub fn set_observation_limits(&mut self, limits: ObservationLimits) {
        self.observation_limits = limits;
    }

    pub fn retention_limits(&self) -> SessionRetentionLimits {
        self.retention_limits
    }

    pub fn set_retention_limits(&mut self, limits: SessionRetentionLimits) {
        self.retention_limits = limits;
        while self.events.len() > limits.events {
            self.events.pop_front();
            self.dropped_events = self.dropped_events.saturating_add(1);
        }
        while self.trace_steps.len() > limits.trace {
            self.trace_steps.pop_front();
            self.omitted_trace_steps = self.omitted_trace_steps.saturating_add(1);
        }
    }

}

fn next_session_id() -> String {
    NEXT_SESSION_ID.with(|next| {
        let value = next.get();
        next.set(value.saturating_add(1));
        format!("bytecode/session-{value}")
    })
}

fn trace_id(session_id: &str, generation: u64) -> String {
    format!("{session_id}/trace-{generation}")
}

fn required_id(value: String, label: &str) -> Result<String, BytecodeSessionError> {
    if value.trim().is_empty() {
        return Err(BytecodeSessionError::new(format!(
            "bytecode session {label} must not be empty"
        )));
    }
    Ok(value)
}

fn fresh_registry() -> NamespaceRegistry<Value> {
    crate::embedding_namespace_registry()
}
