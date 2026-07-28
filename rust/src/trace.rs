//! Development-only structured trace contract.
//!
//! This module deliberately owns no evaluator state.  The evaluator will add
//! hooks in the next slice; keeping collection separate makes it possible to
//! prove that the schema and its limits do not affect normal evaluation.

use std::fmt;

pub const SCHEMA: &str = "hara.trace/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TraceId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EventId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OperationId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceLimits {
    pub max_events: usize,
    pub max_depth: usize,
    pub max_value_chars: usize,
}

impl Default for TraceLimits {
    fn default() -> Self {
        Self {
            max_events: 10_000,
            max_depth: 100,
            max_value_chars: 1_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValuePreview {
    pub type_name: String,
    pub display: String,
    pub truncated: bool,
}

impl ValuePreview {
    pub fn new(type_name: impl Into<String>, display: impl AsRef<str>, limit: usize) -> Self {
        let display = display.as_ref();
        let mut chars = display.chars();
        let bounded: String = chars.by_ref().take(limit).collect();
        let truncated = chars.next().is_some();
        Self {
            type_name: type_name.into(),
            display: if truncated {
                format!("{bounded}…")
            } else {
                bounded
            },
            truncated,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceEventKind {
    EvaluationStart,
    MacroExpand,
    OperationEnter,
    OperationReturn,
    Error,
    TraceTruncated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceEvent {
    pub id: EventId,
    pub sequence: u64,
    pub kind: TraceEventKind,
    pub operation: Option<OperationId>,
    pub parent_operation: Option<OperationId>,
    pub depth: usize,
    pub function: Option<String>,
    pub values: Vec<ValuePreview>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceStatus {
    Ok,
    Error,
    Truncated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trace {
    pub schema: &'static str,
    pub trace_id: TraceId,
    pub status: TraceStatus,
    pub events: Vec<TraceEvent>,
    pub result: Option<ValuePreview>,
    pub error: Option<String>,
}

/// Bounded event collector used by development evaluator hooks.
#[derive(Debug)]
pub struct TraceCollector {
    trace: Trace,
    limits: TraceLimits,
    next_event: u64,
    next_operation: u64,
    truncated: bool,
}

impl TraceCollector {
    pub fn new(trace_id: TraceId, limits: TraceLimits) -> Self {
        Self {
            trace: Trace {
                schema: SCHEMA,
                trace_id,
                status: TraceStatus::Ok,
                events: Vec::new(),
                result: None,
                error: None,
            },
            limits,
            next_event: 1,
            next_operation: 1,
            truncated: false,
        }
    }

    pub fn next_operation_id(&mut self) -> OperationId {
        let id = OperationId(self.next_operation);
        self.next_operation += 1;
        id
    }

    pub fn preview_value(
        &self,
        type_name: impl Into<String>,
        display: impl AsRef<str>,
    ) -> ValuePreview {
        ValuePreview::new(type_name, display, self.limits.max_value_chars)
    }

    pub fn record(&mut self, mut event: TraceEvent) {
        if self.truncated {
            return;
        }
        if event.depth > self.limits.max_depth || self.trace.events.len() >= self.limits.max_events
        {
            self.truncated = true;
            self.trace.status = TraceStatus::Truncated;
            self.push_truncation_event();
            return;
        }
        event.id = EventId(self.next_event);
        event.sequence = self.next_event;
        self.next_event += 1;
        self.trace.events.push(event);
    }

    pub fn finish(mut self, result: ValuePreview) -> Trace {
        self.trace.result = Some(result);
        self.trace
    }

    pub fn fail(mut self, error: impl Into<String>) -> Trace {
        let error = error.into();
        self.record(TraceEvent::error(error.clone()));
        self.trace.status = if self.truncated {
            TraceStatus::Truncated
        } else {
            TraceStatus::Error
        };
        self.trace.error = Some(error);
        self.trace
    }

    fn push_truncation_event(&mut self) {
        // Reserve no extra capacity: the diagnostic replaces further detail.
        let id = EventId(self.next_event);
        self.next_event += 1;
        self.trace.events.push(TraceEvent {
            id,
            sequence: id.0,
            kind: TraceEventKind::TraceTruncated,
            operation: None,
            parent_operation: None,
            depth: 0,
            function: None,
            values: Vec::new(),
            message: Some("trace limit reached; evaluation continued".into()),
        });
    }
}

impl TraceEvent {
    pub fn new(kind: TraceEventKind) -> Self {
        Self {
            id: EventId(0),
            sequence: 0,
            kind,
            operation: None,
            parent_operation: None,
            depth: 0,
            function: None,
            values: Vec::new(),
            message: None,
        }
    }

    pub fn error(error: impl Into<String>) -> Self {
        let mut event = Self::new(TraceEventKind::Error);
        event.message = Some(error.into());
        event
    }
}

impl fmt::Display for TraceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "trace-{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collector_assigns_parent_linkable_operation_and_event_ids() {
        let mut collector = TraceCollector::new(TraceId(17), TraceLimits::default());
        let parent = collector.next_operation_id();
        let child = collector.next_operation_id();
        let mut enter = TraceEvent::new(TraceEventKind::OperationEnter);
        enter.operation = Some(parent);
        enter.function = Some("app.main/run".into());
        collector.record(enter);
        let mut nested = TraceEvent::new(TraceEventKind::OperationEnter);
        nested.operation = Some(child);
        nested.parent_operation = Some(parent);
        nested.depth = 1;
        nested.function = Some("app.math/calculate".into());
        collector.record(nested);

        let trace = collector.finish(ValuePreview::new("number", "12", 10));
        assert_eq!(trace.schema, SCHEMA);
        assert_eq!(trace.events[0].id, EventId(1));
        assert_eq!(trace.events[1].parent_operation, Some(parent));
        assert_eq!(trace.result.unwrap().display, "12");
    }

    #[test]
    fn collector_truncates_recording_without_losing_final_result() {
        let mut collector = TraceCollector::new(
            TraceId(1),
            TraceLimits {
                max_events: 1,
                ..TraceLimits::default()
            },
        );
        collector.record(TraceEvent::new(TraceEventKind::EvaluationStart));
        collector.record(TraceEvent::new(TraceEventKind::OperationEnter));
        let trace = collector.finish(ValuePreview::new("number", "12", 10));

        assert_eq!(trace.status, TraceStatus::Truncated);
        assert!(matches!(
            trace.events.last().unwrap().kind,
            TraceEventKind::TraceTruncated
        ));
        assert_eq!(trace.result.unwrap().display, "12");
    }

    #[test]
    fn value_previews_bound_unicode_without_invalid_utf8() {
        let value = ValuePreview::new("string", "λabcdef", 2);
        assert_eq!(value.display, "λa…");
        assert!(value.truncated);
    }
}
