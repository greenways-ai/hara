//! Rust-only tracing JIT implementation details.
//!
//! This module consumes VM execution observations directly. It neither reads
//! Evaluation Journal events nor changes HALC.

#[path = "jit/backend.rs"]
pub mod backend;
#[path = "jit/hotness.rs"]
pub mod hotness;
#[path = "jit/recorder.rs"]
pub mod recorder;
#[path = "jit/runtime.rs"]
pub(crate) mod runtime;
#[path = "jit/trace_ir.rs"]
pub mod trace_ir;

pub use backend::{CheckedBackend, TraceBackend};
pub use hotness::{Hotness, JitConfig, LoopKey};
pub use recorder::{RecordError, TraceRecorder};
pub use trace_ir::{ExitReason, ExitSnapshot, Trace, TraceOp, TraceOutcome, TraceValue};
