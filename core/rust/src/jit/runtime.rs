use std::collections::{HashMap, HashSet};

#[cfg(all(feature = "native-jit", not(target_arch = "wasm32")))]
use super::{native::NativeTrace, NativeBackend};
#[cfg(any(not(feature = "native-jit"), target_arch = "wasm32"))]
use super::{CheckedBackend, Trace};
use super::{
    ExitReason, ExitSnapshot, Hotness, JitConfig, LoopKey, TraceBackend, TraceOutcome,
    TraceRecorder, TraceValue,
};
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
    pub recording_starts: u64,
    pub recording_completed: u64,
    pub recording_aborts: u64,
    pub trace_paths: u64,
    pub branch_exits: u64,
    pub type_exits: u64,
    pub error_exits: u64,
    pub disabled_loops: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TracePathKey {
    loop_key: LoopKey,
    path: Vec<u32>,
}

struct CachedTrace<T> {
    path: Vec<u32>,
    compiled: T,
}

#[derive(Default)]
struct LoopProfile {
    iterations: u64,
    branch_exits: u32,
}

pub(crate) struct JitRuntime {
    hotness: Hotness,
    recorder: TraceRecorder,
    #[cfg(any(not(feature = "native-jit"), target_arch = "wasm32"))]
    backend: CheckedBackend,
    #[cfg(any(not(feature = "native-jit"), target_arch = "wasm32"))]
    traces: HashMap<LoopKey, Vec<CachedTrace<Trace>>>,
    #[cfg(all(feature = "native-jit", not(target_arch = "wasm32")))]
    backend: NativeBackend,
    #[cfg(all(feature = "native-jit", not(target_arch = "wasm32")))]
    traces: HashMap<LoopKey, Vec<CachedTrace<NativeTrace>>>,
    candidates: HashMap<TracePathKey, u32>,
    rejected: HashSet<TracePathKey>,
    disabled: HashSet<LoopKey>,
    profiles: HashMap<LoopKey, LoopProfile>,
    config: JitConfig,
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
            candidates: HashMap::new(),
            rejected: HashSet::new(),
            disabled: HashSet::new(),
            profiles: HashMap::new(),
            config,
            batch_iterations: 1024,
            telemetry: JitTelemetry::default(),
        }
    }

    pub(crate) fn backedge(
        &mut self,
        program: &Program,
        function: u16,
        _from: u32,
        header: u32,
        path: &[u32],
        recording_locals: &[TraceValue],
        locals: &mut [TraceValue],
    ) -> Option<ExitSnapshot> {
        let key = LoopKey { function, header };
        let path_key = TracePathKey {
            loop_key: key,
            path: path.to_vec(),
        };
        self.telemetry.backedges += 1;
        if self.disabled.contains(&key) {
            return None;
        }
        if self.rejected.contains(&path_key) {
            return None;
        }
        let existing = self
            .traces
            .get(&key)
            .and_then(|traces| traces.iter().position(|trace| trace.path == path));
        let trace_count = self.traces.get(&key).map_or(0, Vec::len);
        let should_compile = if existing.is_some() || trace_count >= self.config.max_traces_per_loop
        {
            false
        } else if trace_count == 0 {
            let hot = self.hotness.backedge(key);
            hot || (self.hotness.count(key) == 1 && program.function_has_i64_parameters(function))
        } else {
            let count = self.candidates.entry(path_key.clone()).or_default();
            *count = count.saturating_add(1);
            *count == self.config.side_trace_threshold
        };
        if should_compile {
            self.telemetry.compile_attempts += 1;
            self.telemetry.recording_starts += 1;
            match self
                .recorder
                .record_path(program, function, header, path, recording_locals)
            {
                Ok(trace) => match self.backend.compile(&trace) {
                    Ok(compiled) => {
                        self.traces.entry(key).or_default().push(CachedTrace {
                            path: path.to_vec(),
                            compiled,
                        });
                        self.candidates.remove(&path_key);
                        self.telemetry.compiled += 1;
                        self.telemetry.recording_completed += 1;
                        self.telemetry.trace_paths += 1;
                    }
                    Err(_) => {
                        self.rejected.insert(path_key);
                        self.telemetry.rejected += 1;
                        self.telemetry.recording_aborts += 1;
                        return None;
                    }
                },
                Err(_) => {
                    self.rejected.insert(path_key);
                    self.telemetry.rejected += 1;
                    self.telemetry.recording_aborts += 1;
                    return None;
                }
            }
        }
        let Some(traces) = self.traces.get_mut(&key) else {
            return None;
        };
        let entry = locals.to_vec();
        let preferred = traces.iter().position(|trace| trace.path == path);
        let order = preferred
            .into_iter()
            .chain((0..traces.len()).filter(|index| Some(*index) != preferred))
            .collect::<Vec<_>>();
        for index in order {
            locals.clone_from_slice(&entry);
            self.telemetry.entries += 1;
            match self
                .backend
                .enter(&mut traces[index].compiled, locals, self.batch_iterations)
            {
                TraceOutcome::Completed { iterations } => {
                    self.telemetry.completed_iterations += u64::from(iterations);
                    self.profiles.entry(key).or_default().iterations += u64::from(iterations);
                    return Some(ExitSnapshot {
                        function,
                        instruction: header,
                        locals: locals.to_vec(),
                        stack: Vec::new(),
                    });
                }
                TraceOutcome::SideExit {
                    reason,
                    iterations,
                    snapshot,
                } => {
                    self.telemetry.side_exits += 1;
                    self.telemetry.completed_iterations += u64::from(iterations);
                    let profile = self.profiles.entry(key).or_default();
                    profile.iterations += u64::from(iterations);
                    match reason {
                        ExitReason::BranchChanged => {
                            self.telemetry.branch_exits += 1;
                            profile.branch_exits = profile.branch_exits.saturating_add(1);
                        }
                        ExitReason::WrongTag => self.telemetry.type_exits += 1,
                        _ => self.telemetry.error_exits += 1,
                    }
                    if profile.branch_exits >= self.config.max_branch_exits_before_bailout
                        && profile.iterations
                            < u64::from(profile.branch_exits)
                                * u64::from(self.config.min_iterations_per_branch_exit)
                    {
                        self.disabled.insert(key);
                        self.telemetry.disabled_loops += 1;
                        return Some(snapshot);
                    }
                    if reason == ExitReason::BranchChanged && snapshot.locals == entry {
                        continue;
                    }
                    return Some(snapshot);
                }
            }
        }
        locals.clone_from_slice(&entry);
        None
    }

    #[cfg(test)]
    pub(crate) fn compiled_count(&self) -> usize {
        self.traces.values().map(Vec::len).sum()
    }

    pub(crate) fn telemetry(&self) -> JitTelemetry {
        self.telemetry
    }

    pub(crate) fn is_disabled(&self, function: u16, header: u32) -> bool {
        self.disabled.contains(&LoopKey { function, header })
    }
}
