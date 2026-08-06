use std::cell::{Cell, RefCell};
use std::collections::{HashSet, VecDeque};
use std::rc::{Rc, Weak};
use std::time::{Duration, Instant};

use crate::core::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum PromiseRejection {
    Message(String),
    Value(Value),
    Cancelled(Value),
}

impl PromiseRejection {
    pub fn value(&self) -> Value {
        match self {
            Self::Message(message) => Value::String(message.clone()),
            Self::Value(value) | Self::Cancelled(value) => value.clone(),
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::Message(message) => message.clone(),
            Self::Value(value) | Self::Cancelled(value) => value.display(),
        }
    }

    pub fn is_cancelled(&self) -> bool {
        match self {
            Self::Cancelled(_) => true,
            Self::Message(message) => message == "cancelled",
            Self::Value(_) => false,
        }
    }

    pub fn cancelled() -> Self {
        Self::Cancelled(Value::Map(
            [
                (
                    Value::Keyword("code".into()),
                    Value::Keyword("task/cancelled".into()),
                ),
                (
                    Value::Keyword("message".into()),
                    Value::String("cancelled".into()),
                ),
                (
                    Value::Keyword("origin".into()),
                    Value::Keyword("runtime".into()),
                ),
                (Value::Keyword("retryable".into()), Value::Bool(false)),
            ]
            .into_iter()
            .collect(),
        ))
    }
}

impl From<String> for PromiseRejection {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}

impl From<&str> for PromiseRejection {
    fn from(value: &str) -> Self {
        Self::Message(value.into())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PromiseState {
    Pending,
    Fulfilled(Value),
    Rejected(PromiseRejection),
}

#[derive(Default)]
struct PromiseHooks {
    poller: Option<Rc<dyn Fn()>>,
    waiter: Option<Rc<dyn Fn()>>,
    cancel: Option<Rc<dyn Fn()>>,
}

struct PromiseInner {
    state: PromiseState,
    continuations: Vec<Rc<dyn Fn(PromiseState)>>,
    deferred: Option<(Instant, Rc<dyn Fn() -> Result<Value, String>>)>,
    hooks: PromiseHooks,
    adopted_from: Option<Weak<RefCell<PromiseInner>>>,
}

type ContinuationJob = (Rc<dyn Fn(PromiseState)>, PromiseState);

thread_local! {
    static CONTINUATION_QUEUE: RefCell<VecDeque<ContinuationJob>> = RefCell::new(VecDeque::new());
    static DRAINING_CONTINUATIONS: Cell<bool> = const { Cell::new(false) };
}

fn enqueue_continuation(continuation: Rc<dyn Fn(PromiseState)>, state: PromiseState) {
    CONTINUATION_QUEUE.with(|queue| queue.borrow_mut().push_back((continuation, state)));
    DRAINING_CONTINUATIONS.with(|draining| {
        if draining.replace(true) {
            return;
        }
        loop {
            let job = CONTINUATION_QUEUE.with(|queue| queue.borrow_mut().pop_front());
            let Some((continuation, state)) = job else {
                break;
            };
            continuation(state);
        }
        draining.set(false);
    });
}

#[derive(Clone)]
pub struct Promise {
    inner: Rc<RefCell<PromiseInner>>,
}

#[derive(Clone)]
pub(crate) struct WeakPromise {
    inner: Weak<RefCell<PromiseInner>>,
}

impl WeakPromise {
    pub(crate) fn upgrade(&self) -> Option<Promise> {
        self.inner.upgrade().map(|inner| Promise { inner })
    }
}

impl std::fmt::Debug for Promise {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Promise")
            .field("state", &self.state())
            .finish()
    }
}

impl Default for Promise {
    fn default() -> Self {
        Self::new()
    }
}

impl Promise {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(PromiseInner {
                state: PromiseState::Pending,
                continuations: Vec::new(),
                deferred: None,
                hooks: PromiseHooks::default(),
                adopted_from: None,
            })),
        }
    }

    pub fn state(&self) -> PromiseState {
        self.run_deferred_if_ready();
        let poller = self.inner.borrow().hooks.poller.clone();
        if let Some(poller) = poller {
            poller();
        }
        self.inner.borrow().state.clone()
    }

    fn run_deferred_if_ready(&self) {
        let task = {
            let mut inner = self.inner.borrow_mut();
            if !inner
                .deferred
                .as_ref()
                .is_some_and(|(at, _)| Instant::now() >= *at)
            {
                return;
            }
            inner.deferred.take().map(|(_, task)| task)
        };
        if let Some(task) = task {
            settle_result(self, task());
        }
    }

    pub fn set_poller(&self, poller: Rc<dyn Fn()>) {
        self.inner.borrow_mut().hooks.poller = Some(poller);
    }

    pub fn set_waiter(&self, waiter: Rc<dyn Fn()>) {
        self.inner.borrow_mut().hooks.waiter = Some(waiter);
    }

    pub fn set_cancel_hook(&self, cancel: Rc<dyn Fn()>) {
        self.inner.borrow_mut().hooks.cancel = Some(cancel);
    }

    pub fn wait_state(&self) -> PromiseState {
        let waiter = self.inner.borrow().hooks.waiter.clone();
        if let Some(waiter) = waiter {
            waiter();
        } else {
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(deadline) = self
                .inner
                .borrow()
                .deferred
                .as_ref()
                .map(|(deadline, _)| *deadline)
            {
                if let Some(delay) = deadline.checked_duration_since(Instant::now()) {
                    std::thread::sleep(delay);
                }
            }
        }
        loop {
            let state = self.state();
            if !matches!(state, PromiseState::Pending) {
                return state;
            }
            let delay = {
                let inner = self.inner.borrow();
                inner
                    .deferred
                    .as_ref()
                    .map(|(at, _)| at.saturating_duration_since(Instant::now()))
            };
            let Some(delay) = delay else {
                return state;
            };
            if !delay.is_zero() {
                std::thread::sleep(delay);
            } else {
                std::thread::yield_now();
            }
        }
    }

    pub fn wait_state_timeout(&self, timeout: Duration) -> PromiseState {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = timeout;
            return self.state();
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let deadline = Instant::now() + timeout;
            loop {
                let state = self.state();
                if !matches!(state, PromiseState::Pending) || Instant::now() >= deadline {
                    return state;
                }
                let remaining = deadline.saturating_duration_since(Instant::now());
                std::thread::sleep(remaining.min(Duration::from_millis(1)));
            }
        }
    }

    pub(crate) fn notify_cancel(&self) {
        let cancel = self.inner.borrow().hooks.cancel.clone();
        if let Some(cancel) = cancel {
            cancel();
        }
    }

    pub fn cancel(&self) -> bool {
        if !matches!(self.inner.borrow().state, PromiseState::Pending) {
            return false;
        }
        self.notify_cancel();
        self.reject_rejection(PromiseRejection::cancelled())
    }

    pub fn schedule(&self, delay: Duration, task: Rc<dyn Fn() -> Result<Value, String>>) {
        if delay.is_zero() {
            settle_result(self, task());
        } else {
            self.inner.borrow_mut().deferred = Some((Instant::now() + delay, task));
        }
    }

    pub fn resolve(&self, value: Value) -> bool {
        self.settle(PromiseState::Fulfilled(value))
    }

    pub fn reject(&self, error: impl Into<String>) -> bool {
        self.reject_rejection(PromiseRejection::Message(error.into()))
    }

    pub fn reject_value(&self, error: Value) -> bool {
        self.reject_rejection(PromiseRejection::Value(error))
    }

    pub fn reject_rejection(&self, error: PromiseRejection) -> bool {
        self.settle(PromiseState::Rejected(error))
    }

    fn settle(&self, next: PromiseState) -> bool {
        let continuations = {
            let mut inner = self.inner.borrow_mut();
            if !matches!(inner.state, PromiseState::Pending) {
                return false;
            }
            inner.state = next.clone();
            inner.deferred = None;
            inner.hooks = PromiseHooks::default();
            inner.adopted_from = None;
            std::mem::take(&mut inner.continuations)
        };
        for continuation in continuations {
            enqueue_continuation(continuation, next.clone());
        }
        true
    }

    pub fn on_settle(&self, continuation: Rc<dyn Fn(PromiseState)>) {
        let state = self.state();
        if matches!(state, PromiseState::Pending) {
            self.inner.borrow_mut().continuations.push(continuation);
        } else {
            enqueue_continuation(continuation, state);
        }
    }

    pub fn adopt(&self, other: &Promise) -> bool {
        if self.same_identity(other) || self.adoption_would_cycle(other) {
            return self.reject("promise adoption cycle");
        }
        match other.state() {
            PromiseState::Pending => {
                if !matches!(self.state(), PromiseState::Pending) {
                    return false;
                }
                self.inner.borrow_mut().adopted_from = Some(Rc::downgrade(&other.inner));
                let source = other.clone();
                self.set_poller(Rc::new(move || {
                    source.state();
                }));
                let source = other.clone();
                self.set_waiter(Rc::new(move || {
                    source.wait_state();
                }));
                let destination = self.clone();
                other.on_settle(Rc::new(move |state| match state {
                    PromiseState::Fulfilled(value) => {
                        destination.resolve(value);
                    }
                    PromiseState::Rejected(error) => {
                        destination.reject_rejection(error);
                    }
                    PromiseState::Pending => {}
                }));
                true
            }
            PromiseState::Fulfilled(value) => self.resolve(value),
            PromiseState::Rejected(error) => self.reject_rejection(error),
        }
    }

    fn adoption_would_cycle(&self, source: &Promise) -> bool {
        let mut current = Some(source.inner.clone());
        let mut seen = HashSet::new();
        while let Some(inner) = current {
            if Rc::ptr_eq(&self.inner, &inner) {
                return true;
            }
            let address = Rc::as_ptr(&inner) as usize;
            if !seen.insert(address) {
                return true;
            }
            current = inner.borrow().adopted_from.as_ref().and_then(Weak::upgrade);
        }
        false
    }

    pub fn same_identity(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }

    pub(crate) fn downgrade(&self) -> WeakPromise {
        WeakPromise {
            inner: Rc::downgrade(&self.inner),
        }
    }

    pub fn identity_address(&self) -> usize {
        Rc::as_ptr(&self.inner) as usize
    }
}

pub fn settle_result(destination: &Promise, result: Result<Value, String>) {
    match result {
        Ok(Value::Promise(source)) => {
            destination.adopt(&source);
        }
        Ok(value) => {
            destination.resolve(value);
        }
        Err(error) => {
            destination.reject(error);
        }
    }
}

pub trait PromiseProvider {
    fn native(&self) -> bool;
    fn run(&self, task: Rc<dyn Fn() -> Result<Value, String>>) -> Promise;
    fn delay(&self, duration: Duration, task: Rc<dyn Fn() -> Result<Value, String>>) -> Promise;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LocalPromiseProvider;

impl PromiseProvider for LocalPromiseProvider {
    fn native(&self) -> bool {
        true
    }

    fn run(&self, task: Rc<dyn Fn() -> Result<Value, String>>) -> Promise {
        let promise = Promise::new();
        settle_result(&promise, task());
        promise
    }

    fn delay(&self, duration: Duration, task: Rc<dyn Fn() -> Result<Value, String>>) -> Promise {
        let promise = Promise::new();
        promise.schedule(duration, task);
        promise
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancellation_is_a_structured_rejection() {
        let promise = Promise::new();
        assert!(promise.cancel());
        let PromiseState::Rejected(rejection) = promise.state() else {
            panic!("cancelled promise was not rejected");
        };
        assert!(rejection.is_cancelled());
        let Value::Map(fields) = rejection.value() else {
            panic!("cancellation was not represented by a map");
        };
        assert_eq!(
            fields.get(&Value::Keyword("code".into())),
            Some(&Value::Keyword("task/cancelled".into()))
        );
        assert_eq!(
            fields.get(&Value::Keyword("retryable".into())),
            Some(&Value::Bool(false))
        );
    }

    #[test]
    fn adoption_cycles_are_rejected() {
        let direct = Promise::new();
        assert!(direct.adopt(&direct));
        assert!(matches!(direct.state(), PromiseState::Rejected(_)));

        let first = Promise::new();
        let second = Promise::new();
        assert!(first.adopt(&second));
        assert!(second.adopt(&first));
        assert!(matches!(second.state(), PromiseState::Rejected(_)));
        assert!(matches!(first.state(), PromiseState::Rejected(_)));
    }

    #[test]
    fn continuation_chains_are_trampolined() {
        let promises: Vec<_> = (0..20_000).map(|_| Promise::new()).collect();
        for pair in promises.windows(2) {
            let next = pair[1].clone();
            pair[0].on_settle(Rc::new(move |state| match state {
                PromiseState::Fulfilled(value) => {
                    next.resolve(value);
                }
                PromiseState::Rejected(error) => {
                    next.reject_rejection(error);
                }
                PromiseState::Pending => {}
            }));
        }
        promises[0].resolve(Value::Number(42));
        assert_eq!(
            promises.last().unwrap().state(),
            PromiseState::Fulfilled(Value::Number(42))
        );
    }
}
