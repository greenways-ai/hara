use std::collections::BTreeMap;

use crate::core::Value;
use crate::vm::machine::observation::{
    InstructionSnapshot, MachineSnapshot, ObservationEventKind, ObservationEventStatus,
    SourcePositionSnapshot,
};

pub const BYTECODE_METRICS_SCHEMA: &str = "hal.bytecode-metrics/v1";
pub const BYTECODE_EVENTS_SCHEMA: &str = "hal.bytecode-events/v1";
pub const BYTECODE_TRACE_SCHEMA: &str = "hal.bytecode-trace/v1";

const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

#[derive(Clone, Debug)]
pub(super) struct TraceStepRecord {
    pub id: String,
    pub sequence: u64,
    pub kind: ObservationEventKind,
    pub status: ObservationEventStatus,
    pub before: MachineSnapshot,
    pub after: MachineSnapshot,
    pub instruction: Option<InstructionSnapshot>,
    pub source: Option<SourcePositionSnapshot>,
    pub error: Option<String>,
}

impl TraceStepRecord {
    pub fn new(
        trace_id: &str,
        sequence: u64,
        kind: ObservationEventKind,
        status: ObservationEventStatus,
        before: MachineSnapshot,
        after: MachineSnapshot,
        instruction: Option<InstructionSnapshot>,
        source: Option<SourcePositionSnapshot>,
    ) -> Self {
        let error = after.error.clone();
        Self {
            id: format!("{trace_id}/step/{sequence}"),
            sequence,
            kind,
            status,
            before,
            after,
            instruction,
            source,
            error,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct EvidenceLabel(&'static str);

impl From<&EvidenceLabel> for String {
    fn from(value: &EvidenceLabel) -> Self {
        value.0.into()
    }
}

#[derive(Clone, Debug)]
pub(super) enum CompactEventRecord {
    Instruction {
        id: String,
        sequence: u64,
        function: usize,
        ip: usize,
        opcode: String,
        stack_depth: usize,
        call_depth: usize,
    },
    Transition {
        id: String,
        sequence: u64,
        transition: EvidenceLabel,
        from_function: usize,
        from_ip: usize,
        to_function: usize,
        to_ip: usize,
        stack_depth: usize,
        call_depth: usize,
    },
    Terminal {
        id: String,
        sequence: u64,
        terminal: EvidenceLabel,
        function: usize,
        ip: usize,
        stack_depth: usize,
        call_depth: usize,
    },
}

impl CompactEventRecord {
    pub fn from_trace(trace_id: &str, step: &TraceStepRecord) -> Self {
        let id = format!("{trace_id}/event/{}", step.sequence);
        match step.kind {
            ObservationEventKind::InstructionExecute => Self::Instruction {
                id,
                sequence: step.sequence,
                function: step.before.function,
                ip: step.before.ip,
                opcode: step
                    .instruction
                    .as_ref()
                    .map_or_else(|| "unknown".into(), |instruction| instruction.opcode.into()),
                stack_depth: stack_depth(&step.before),
                call_depth: call_depth(&step.before),
            },
            ObservationEventKind::CallEnter
            | ObservationEventKind::CallReturn
            | ObservationEventKind::ExceptionUnwind
            | ObservationEventKind::MachineSuspend
            | ObservationEventKind::MachineResume => Self::Transition {
                id,
                sequence: step.sequence,
                transition: EvidenceLabel(step.kind.as_keyword()),
                from_function: step.before.function,
                from_ip: step.before.ip,
                to_function: step.after.function,
                to_ip: step.after.ip,
                stack_depth: stack_depth(&step.after),
                call_depth: call_depth(&step.after),
            },
            ObservationEventKind::MachineReturn | ObservationEventKind::MachineFail => {
                Self::Terminal {
                    id,
                    sequence: step.sequence,
                    terminal: EvidenceLabel(step.kind.as_keyword()),
                    function: step.after.function,
                    ip: step.after.ip,
                    stack_depth: stack_depth(&step.after),
                    call_depth: call_depth(&step.after),
                }
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct SessionMetrics {
    pub instructions: u64,
    pub opcode_counts: BTreeMap<String, u64>,
    pub calls: u64,
    pub returns: u64,
    pub unwinds: u64,
    pub suspensions: u64,
    pub resumptions: u64,
    pub terminal_returns: u64,
    pub failures: u64,
    pub max_stack_depth: usize,
    pub max_call_depth: usize,
}

impl SessionMetrics {
    pub fn observe(&mut self, step: &TraceStepRecord, counted_instruction: bool) {
        if counted_instruction {
            if let Some(instruction) = &step.instruction {
                self.instructions = self.instructions.saturating_add(1);
                let count = self
                    .opcode_counts
                    .entry(instruction.opcode.into())
                    .or_default();
                *count = (*count).saturating_add(1);
                self.observe_depths(stack_depth(&step.before), call_depth(&step.before));
            }
        }
        match step.kind {
            ObservationEventKind::InstructionExecute => {}
            ObservationEventKind::CallEnter => {
                self.calls = self.calls.saturating_add(1);
                self.observe_after(step);
            }
            ObservationEventKind::CallReturn => {
                self.returns = self.returns.saturating_add(1);
                self.observe_after(step);
            }
            ObservationEventKind::ExceptionUnwind => {
                self.unwinds = self.unwinds.saturating_add(1);
                self.observe_after(step);
            }
            ObservationEventKind::MachineSuspend => {
                self.suspensions = self.suspensions.saturating_add(1);
                self.observe_after(step);
            }
            ObservationEventKind::MachineResume => {
                self.resumptions = self.resumptions.saturating_add(1);
                self.observe_after(step);
            }
            ObservationEventKind::MachineReturn => {
                self.terminal_returns = self.terminal_returns.saturating_add(1);
                self.observe_after(step);
            }
            ObservationEventKind::MachineFail => {
                self.failures = self.failures.saturating_add(1);
                self.observe_after(step);
            }
        }
    }

    fn observe_after(&mut self, step: &TraceStepRecord) {
        self.observe_depths(stack_depth(&step.after), call_depth(&step.after));
    }

    fn observe_depths(&mut self, stack_depth: usize, call_depth: usize) {
        self.max_stack_depth = self.max_stack_depth.max(stack_depth);
        self.max_call_depth = self.max_call_depth.max(call_depth);
    }
}

#[path = "evidence/document.rs"]
mod document;

pub(super) fn metrics_document(
    session_id: &str,
    trace_id: &str,
    sequence: u64,
    status: &str,
    metrics: &SessionMetrics,
) -> Value {
    document::metrics_document(session_id, trace_id, sequence, status, metrics)
}

pub(super) fn events_document<'a>(
    session_id: &str,
    trace_id: &str,
    sequence: u64,
    status: &str,
    events: impl IntoIterator<Item = &'a CompactEventRecord>,
    dropped: u64,
) -> Value {
    document::events_document(session_id, trace_id, sequence, status, events, dropped)
}

pub(super) fn trace_document<'a>(
    session_id: &str,
    trace_id: &str,
    source_id: &str,
    sequence: u64,
    status: &str,
    steps: impl IntoIterator<Item = &'a TraceStepRecord>,
    dropped: u64,
) -> Value {
    document::trace_document(
        session_id,
        trace_id,
        source_id,
        sequence,
        status,
        steps,
        dropped,
    )
}

pub(super) fn snapshot_value(snapshot: &MachineSnapshot, source_id: &str) -> Value {
    document::snapshot_value(snapshot, source_id)
}

fn stack_depth(snapshot: &MachineSnapshot) -> usize {
    snapshot.stack.len().saturating_add(snapshot.stack_omitted)
}

fn call_depth(snapshot: &MachineSnapshot) -> usize {
    snapshot.calls.len().saturating_add(snapshot.calls_omitted)
}
