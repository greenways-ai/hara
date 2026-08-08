use super::{InstructionEvent, TerminalEvent, TransitionEvent, VmProbe, BYTECODE_EVENTS_SCHEMA};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VmEvent {
    Instruction(InstructionEvent),
    Transition(TransitionEvent),
    Terminal(TerminalEvent),
}

pub struct EventRing {
    slots: Box<[Option<VmEvent>]>,
    next: usize,
    len: usize,
    dropped: u64,
}

impl EventRing {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            slots: vec![None; capacity].into_boxed_slice(),
            next: 0,
            len: 0,
            dropped: 0,
        }
    }

    pub fn schema(&self) -> &'static str {
        BYTECODE_EVENTS_SCHEMA
    }

    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    pub fn iter(&self) -> impl Iterator<Item = &VmEvent> {
        let capacity = self.slots.len();
        let start = if self.len == capacity { self.next } else { 0 };
        (0..self.len).filter_map(move |offset| {
            let index = if capacity == 0 {
                0
            } else {
                (start + offset) % capacity
            };
            self.slots.get(index).and_then(Option::as_ref)
        })
    }

    fn push(&mut self, event: VmEvent) {
        if self.slots.is_empty() {
            self.dropped = self.dropped.saturating_add(1);
            return;
        }
        if self.len == self.slots.len() {
            self.dropped = self.dropped.saturating_add(1);
        } else {
            self.len += 1;
        }
        self.slots[self.next] = Some(event);
        self.next = (self.next + 1) % self.slots.len();
    }
}

impl VmProbe for EventRing {
    #[inline(always)]
    fn on_instruction(&mut self, event: InstructionEvent) {
        self.push(VmEvent::Instruction(event));
    }

    #[inline(always)]
    fn on_transition(&mut self, event: TransitionEvent) {
        self.push(VmEvent::Transition(event));
    }

    #[inline(always)]
    fn on_terminal(&mut self, event: TerminalEvent) {
        self.push(VmEvent::Terminal(event));
    }
}

pub struct SampledProbe<P> {
    inner: P,
    every: u64,
    seen: u64,
}

impl<P> SampledProbe<P> {
    pub fn new(inner: P, every: u64) -> Self {
        Self {
            inner,
            every: every.max(1),
            seen: 0,
        }
    }

    pub fn inner(&self) -> &P {
        &self.inner
    }

    pub fn into_inner(self) -> P {
        self.inner
    }
}

impl<P: VmProbe> VmProbe for SampledProbe<P> {
    #[inline(always)]
    fn on_instruction(&mut self, event: InstructionEvent) {
        let emit = self.seen % self.every == 0;
        self.seen = self.seen.saturating_add(1);
        if emit {
            self.inner.on_instruction(event);
        }
    }

    #[inline(always)]
    fn on_transition(&mut self, event: TransitionEvent) {
        self.inner.on_transition(event);
    }

    #[inline(always)]
    fn on_terminal(&mut self, event: TerminalEvent) {
        self.inner.on_terminal(event);
    }
}
