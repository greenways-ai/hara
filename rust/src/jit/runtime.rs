use std::collections::{HashMap, HashSet};

#[cfg(all(feature = "native-jit", not(target_arch = "wasm32")))]
use super::{native::NativeTrace, NativeBackend};
#[cfg(any(not(feature = "native-jit"), target_arch = "wasm32"))]
use super::{CheckedBackend, Trace};
use super::{Hotness, JitConfig, LoopKey, TraceBackend, TraceOutcome, TraceRecorder, TraceValue};
use crate::vm::Program;

/// Per-program tracing-JIT counters. They make it possible to distinguish a
/// cold loop, an unsupported trace, and a trace that is compiled but exits.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct JitTelemetry {
    pub backedges: u64,
    pub compile_attempts: u64,
    pub compiled: u64,
    pub rejected: u64,
    pub entries: u64,
    pub completed_iterations: u64,
    pub side_exits: u64,
}

pub(crate) struct JitRuntime {
    hotness: Hotness,
    recorder: TraceRecorder,
    #[cfg(any(not(feature = "native-jit"), target_arch = "wasm32"))]
    backend: CheckedBackend,
    #[cfg(any(not(feature = "native-jit"), target_arch = "wasm32"))]
    traces: HashMap<LoopKey, Trace>,
    #[cfg(all(feature = "native-jit", not(target_arch = "wasm32")))]
    backend: NativeBackend,
    #[cfg(all(feature = "native-jit", not(target_arch = "wasm32")))]
    traces: HashMap<LoopKey, NativeTrace>,
    rejected: HashSet<LoopKey>,
    batch_iterations: u32,
    telemetry: JitTelemetry,
}

impl Default for JitRuntime {
    fn default() -> Self {
        Self::new(JitConfig::default())
    }
}

impl JitRuntime {
    pub(crate) fn new(config: JitConfig) -> Self {
        Self {
            hotness: Hotness::new(config),
            recorder: TraceRecorder::new(config.max_trace_operations),
            backend: Default::default(),
            traces: HashMap::new(),
            rejected: HashSet::new(),
            batch_iterations: 1024,
            telemetry: JitTelemetry::default(),
        }
    }

    pub(crate) fn backedge(
        &mut self,
        program: &Program,
        function: u16,
        from: u32,
        header: u32,
        locals: &mut [TraceValue],
    ) -> bool {
        let key = LoopKey { function, header };
        self.telemetry.backedges += 1;
        if self.rejected.contains(&key) {
            return false;
        }
        if !self.traces.contains_key(&key) && self.hotness.backedge(key) {
            self.telemetry.compile_attempts += 1;
            match self
                .recorder
                .record_loop(program, function, header, from, locals)
            {
                Ok(trace) => match self.backend.compile(&trace) {
                    Ok(compiled) => {
                        self.traces.insert(key, compiled);
                        self.telemetry.compiled += 1;
                    }
                    Err(_) => {
                        self.rejected.insert(key);
                        self.telemetry.rejected += 1;
                        return false;
                    }
                },
                Err(_) => {
                    self.rejected.insert(key);
                    self.telemetry.rejected += 1;
                    return false;
                }
            }
        }
        let Some(trace) = self.traces.get_mut(&key) else {
            return false;
        };
        self.telemetry.entries += 1;
        match self.backend.enter(trace, locals, self.batch_iterations) {
            TraceOutcome::Completed { iterations } => {
                self.telemetry.completed_iterations += u64::from(iterations);
                true
            }
            TraceOutcome::SideExit { .. } => {
                self.telemetry.side_exits += 1;
                true
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn compiled_count(&self) -> usize {
        self.traces.len()
    }

    pub(crate) fn telemetry(&self) -> JitTelemetry {
        self.telemetry
    }
}
