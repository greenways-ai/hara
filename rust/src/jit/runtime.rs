use std::collections::{HashMap, HashSet};

use super::{CheckedBackend, Hotness, JitConfig, LoopKey, Trace, TraceBackend, TraceOutcome, TraceRecorder, TraceValue};
use crate::vm::Program;

pub(crate) struct JitRuntime {
    hotness: Hotness,
    recorder: TraceRecorder,
    backend: CheckedBackend,
    traces: HashMap<LoopKey, Trace>,
    rejected: HashSet<LoopKey>,
    batch_iterations: u32,
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
            backend: CheckedBackend,
            traces: HashMap::new(),
            rejected: HashSet::new(),
            batch_iterations: 1024,
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
        if self.rejected.contains(&key) {
            return false;
        }
        if !self.traces.contains_key(&key) && self.hotness.backedge(key) {
            match self.recorder.record_loop(program, function, header, from) {
                Ok(trace) => {
                    let compiled = self.backend.compile(&trace).expect("checked backend compilation");
                    self.traces.insert(key, compiled);
                }
                Err(_) => {
                    self.rejected.insert(key);
                    return false;
                }
            }
        }
        let Some(trace) = self.traces.get(&key) else { return false };
        match self.backend.enter(trace, locals, self.batch_iterations) {
            TraceOutcome::Completed { .. } | TraceOutcome::SideExit { .. } => true,
        }
    }

    #[cfg(test)]
    pub(crate) fn compiled_count(&self) -> usize {
        self.traces.len()
    }
}
