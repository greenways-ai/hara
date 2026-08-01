use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::time::{Duration, Instant};

use crate::core::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum PromiseRejection {
    Message(String),
    Value(Value),
}

impl PromiseRejection {
    pub fn value(&self) -> Value {
        match self {
            Self::Message(message) => Value::String(message.clone()),
            Self::Value(value) => value.clone(),
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::Message(message) => message.clone(),
            Self::Value(value) => value.display(),
        }
    }

    pub fn is_cancelled(&self) -> bool {
        matches!(self, Self::Message(message) if message == "cancelled")
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
        }
        self.state()
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
        self.reject("cancelled")
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
            std::mem::take(&mut inner.continuations)
        };
        for continuation in continuations {
            continuation(next.clone());
        }
        true
    }

    pub fn on_settle(&self, continuation: Rc<dyn Fn(PromiseState)>) {
        let state = self.state();
        if matches!(state, PromiseState::Pending) {
            self.inner.borrow_mut().continuations.push(continuation);
        } else {
            continuation(state);
        }
    }

    pub fn adopt(&self, other: &Promise) -> bool {
        match other.state() {
            PromiseState::Pending => {
                if !matches!(self.state(), PromiseState::Pending) {
                    return false;
                }
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
