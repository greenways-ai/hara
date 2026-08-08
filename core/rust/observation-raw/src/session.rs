//! Live observed bytecode ownership for the plain-C browser module.
//!
//! The main `hara-wasm` crate adds a wasm-bindgen shell around the same native
//! session. This on-demand raw module deliberately omits that shell and reuses
//! the shared control, evidence and machine implementation behind a compact
//! JSON ABI that can be instantiated without generated JavaScript glue.

use std::cell::Cell;
use std::collections::VecDeque;
use std::rc::Rc;

use crate::core::{Promise, Value};
use crate::kernel::NamespaceRegistry;

use super::machine::observation::ObservationLimits;
use super::{compile_source_with, decode_program, validate, Machine, Program, VmError};

#[path = "../../src/vm/session/evidence.rs"]
mod evidence;
#[path = "../../src/vm/session/control.rs"]
mod control;

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
    pub(super) fn new(message: impl Into<String>) -> Self {
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
    pub(super) session_id: String,
    pub(super) source_id: String,
    pub(super) source: Option<String>,
    pub(super) program: Option<Rc<Program>>,
    pub(super) machine: Option<Machine>,
    pub(super) registry: NamespaceRegistry<Value>,
    pub(super) status: BytecodeSessionStatus,
    pub(super) paused_from: Option<BytecodeSessionStatus>,
    pub(super) observation_limits: ObservationLimits,
    pub(super) retention_limits: SessionRetentionLimits,
    pub(super) trace_generation: u64,
    pub(super) trace_id: String,
    pub(super) next_sequence: u64,
    pub(super) metrics: SessionMetrics,
    pub(super) events: VecDeque<CompactEventRecord>,
    pub(super) trace_steps: VecDeque<TraceStepRecord>,
    pub(super) dropped_events: u64,
    pub(super) omitted_trace_steps: u64,
    pub(super) result: Option<Value>,
    pub(super) error: Option<VmError>,
    pub(super) suspension: Option<Promise>,
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

pub(super) fn trace_id(session_id: &str, generation: u64) -> String {
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

pub(super) fn fresh_registry() -> NamespaceRegistry<Value> {
    crate::embedding_namespace_registry()
}
