use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LoopKey {
    pub function: u16,
    pub header: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JitConfig {
    pub hot_threshold: u32,
    pub side_trace_threshold: u32,
    pub max_traces_per_loop: usize,
    pub max_branch_exits_before_bailout: u32,
    pub min_iterations_per_branch_exit: u32,
    pub max_trace_operations: usize,
}

impl Default for JitConfig {
    fn default() -> Self {
        Self {
            hot_threshold: 16,
            side_trace_threshold: 16,
            max_traces_per_loop: 4,
            max_branch_exits_before_bailout: 32,
            min_iterations_per_branch_exit: 8,
            max_trace_operations: 4096,
        }
    }
}

#[derive(Debug)]
pub struct Hotness {
    config: JitConfig,
    counters: HashMap<LoopKey, u32>,
}

impl Hotness {
    pub fn new(config: JitConfig) -> Self {
        Self {
            config,
            counters: HashMap::new(),
        }
    }

    pub fn backedge(&mut self, key: LoopKey) -> bool {
        let counter = self.counters.entry(key).or_default();
        *counter = counter.saturating_add(1);
        *counter == self.config.hot_threshold
    }

    pub fn count(&self, key: LoopKey) -> u32 {
        self.counters.get(&key).copied().unwrap_or(0)
    }

    pub fn config(&self) -> JitConfig {
        self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_are_per_loop_and_saturating() {
        let key = LoopKey {
            function: 2,
            header: 7,
        };
        let mut hotness = Hotness::new(JitConfig {
            hot_threshold: 2,
            side_trace_threshold: 2,
            max_traces_per_loop: 4,
            max_branch_exits_before_bailout: 32,
            min_iterations_per_branch_exit: 8,
            max_trace_operations: 10,
        });
        assert!(!hotness.backedge(key));
        assert!(hotness.backedge(key));
        assert!(!hotness.backedge(key));
        assert_eq!(hotness.count(key), 3);
    }
}
