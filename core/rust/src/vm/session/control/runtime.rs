use crate::core::{PromiseState, Value};
use crate::kernel::NamespaceRegistry;

use crate::vm::machine::observation::{ObservedStep, ObservedStepOutcome};
use super::super::{
    evidence, BytecodeObservationSession, BytecodeSessionError, BytecodeSessionStatus,
    CompactEventRecord, SessionMetrics, TraceStepRecord,
};
use super::cancel_pending;

impl BytecodeObservationSession {
    pub(super) fn ensure_runnable(&self) -> Result<(), BytecodeSessionError> {
        if matches!(
            self.status,
            BytecodeSessionStatus::Ready | BytecodeSessionStatus::Running
        ) {
            return Ok(());
        }
        Err(BytecodeSessionError::new(format!(
            "bytecode session cannot run from {}",
            self.status.as_keyword()
        )))
    }

    pub(super) fn execute_step(&mut self) -> Result<TraceStepRecord, BytecodeSessionError> {
        let limits = self.observation_limits;
        let registry = &self.registry;
        let machine = self
            .machine
            .as_mut()
            .ok_or_else(|| BytecodeSessionError::new("bytecode session is disposed"))?;
        let step = crate::core::with_namespace_registry(registry, || {
            machine.step_observed_with_limits(limits)
        });
        Ok(self.record_step(step, true))
    }

    pub(super) fn execute_resume(
        &mut self,
        settlement: PromiseState,
    ) -> Result<TraceStepRecord, BytecodeSessionError> {
        let limits = self.observation_limits;
        let registry = &self.registry;
        let machine = self
            .machine
            .as_mut()
            .ok_or_else(|| BytecodeSessionError::new("bytecode session is disposed"))?;
        let step = crate::core::with_namespace_registry(registry, || {
            machine.resume_observed_with_limits(settlement, limits)
        });
        Ok(self.record_step(step, false))
    }

    fn record_step(&mut self, step: ObservedStep, counted_instruction: bool) -> TraceStepRecord {
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.saturating_add(1);
        let ObservedStep {
            kind,
            status,
            before,
            after,
            instruction,
            source,
            outcome,
            ..
        } = step;
        let record = TraceStepRecord::new(
            &self.trace_id,
            sequence,
            kind,
            status,
            before,
            after,
            instruction,
            source,
        );
        self.metrics.observe(&record, counted_instruction);
        self.retain_event(CompactEventRecord::from_trace(&self.trace_id, &record));
        self.retain_trace(record.clone());
        self.paused_from = None;
        match outcome {
            ObservedStepOutcome::Continue => {
                self.status = BytecodeSessionStatus::Running;
                self.suspension = None;
            }
            ObservedStepOutcome::Suspended(promise) => {
                self.status = BytecodeSessionStatus::Suspended;
                self.suspension = Some(promise);
            }
            ObservedStepOutcome::Returned(value) => {
                self.status = BytecodeSessionStatus::Returned;
                self.result = Some(value);
                self.suspension = None;
            }
            ObservedStepOutcome::Failed(error) => {
                self.status = BytecodeSessionStatus::Failed;
                self.error = Some(error);
                self.suspension = None;
            }
        }
        record
    }

    fn retain_event(&mut self, event: CompactEventRecord) {
        if self.retention_limits.events == 0 {
            self.dropped_events = self.dropped_events.saturating_add(1);
            return;
        }
        if self.events.len() == self.retention_limits.events {
            self.events.pop_front();
            self.dropped_events = self.dropped_events.saturating_add(1);
        }
        self.events.push_back(event);
    }

    fn retain_trace(&mut self, step: TraceStepRecord) {
        if self.retention_limits.trace == 0 {
            self.omitted_trace_steps = self.omitted_trace_steps.saturating_add(1);
            return;
        }
        if self.trace_steps.len() == self.retention_limits.trace {
            self.trace_steps.pop_front();
            self.omitted_trace_steps = self.omitted_trace_steps.saturating_add(1);
        }
        self.trace_steps.push_back(step);
    }

    pub(super) fn delta_trace(&self, steps: &[TraceStepRecord]) -> Value {
        evidence::trace_document(
            &self.session_id,
            &self.trace_id,
            &self.source_id,
            self.next_sequence,
            self.status.as_keyword(),
            steps.iter(),
            0,
        )
    }

    pub(super) fn dispose_inner(&mut self) {
        cancel_pending(self.suspension.take());
        self.machine = None;
        self.program = None;
        // Disposal must remain safe during thread-local teardown. Rebuilding the
        // full embedding registry can touch other TLS caches after destruction
        // has begun, while an empty registry releases the retained namespace
        // graph without invoking any runtime bootstrap path.
        self.registry = NamespaceRegistry::new("user");
        self.source = None;
        self.result = None;
        self.error = None;
        self.metrics = SessionMetrics::default();
        self.events.clear();
        self.trace_steps.clear();
        self.dropped_events = 0;
        self.omitted_trace_steps = 0;
        self.paused_from = None;
        self.status = BytecodeSessionStatus::Disposed;
    }
}

impl Drop for BytecodeObservationSession {
    fn drop(&mut self) {
        if self.status != BytecodeSessionStatus::Disposed {
            self.dispose_inner();
        }
    }
}
