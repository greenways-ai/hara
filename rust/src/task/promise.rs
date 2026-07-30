use std::cell::RefCell;
use std::rc::Rc;
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

#[derive(Clone)]
pub struct Promise {
    pub(crate) state: Rc<RefCell<PromiseState>>,
    continuations: Rc<RefCell<Vec<Rc<dyn Fn(PromiseState)>>>>,
    deferred: Rc<RefCell<Option<(Instant, Rc<dyn Fn() -> Result<Value, String>>)>>>,
    hooks: Rc<RefCell<PromiseHooks>>,
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
            state: Rc::new(RefCell::new(PromiseState::Pending)),
            continuations: Rc::new(RefCell::new(Vec::new())),
            deferred: Rc::new(RefCell::new(None)),
            hooks: Rc::new(RefCell::new(PromiseHooks::default())),
        }
    }

    pub fn state(&self) -> PromiseState {
        self.run_deferred_if_ready();
        let poller = self.hooks.borrow().poller.clone();
        if let Some(poller) = poller {
            poller();
        }
        self.state.borrow().clone()
    }

    fn run_deferred_if_ready(&self) {
        let ready = self
            .deferred
            .borrow()
            .as_ref()
            .is_some_and(|(at, _)| Instant::now() >= *at);
        if !ready {
            return;
        }
        let task = self.deferred.borrow_mut().take().map(|(_, task)| task);
        if let Some(task) = task {
            settle_result(self, task());
        }
    }

    pub fn set_poller(&self, poller: Rc<dyn Fn()>) {
        self.hooks.borrow_mut().poller = Some(poller);
    }

    pub fn set_waiter(&self, waiter: Rc<dyn Fn()>) {
        self.hooks.borrow_mut().waiter = Some(waiter);
    }

    pub fn set_cancel_hook(&self, cancel: Rc<dyn Fn()>) {
        self.hooks.borrow_mut().cancel = Some(cancel);
    }

    pub fn wait_state(&self) -> PromiseState {
        let waiter = self.hooks.borrow().waiter.clone();
        if let Some(waiter) = waiter {
            waiter();
        }
        self.state()
    }

    pub(crate) fn notify_cancel(&self) {
        let cancel = self.hooks.borrow().cancel.clone();
        if let Some(cancel) = cancel {
            cancel();
        }
    }

    pub fn cancel(&self) -> bool {
        if !matches!(*self.state.borrow(), PromiseState::Pending) {
            return false;
        }
        self.notify_cancel();
        self.reject("cancelled")
    }

    pub fn schedule(&self, delay: Duration, task: Rc<dyn Fn() -> Result<Value, String>>) {
        if delay.is_zero() {
            settle_result(self, task());
        } else {
            *self.deferred.borrow_mut() = Some((Instant::now() + delay, task));
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
        self.deferred.borrow_mut().take();
        let continuations = {
            let mut state = self.state.borrow_mut();
            if !matches!(*state, PromiseState::Pending) {
                return false;
            }
            *state = next.clone();
            std::mem::take(&mut *self.continuations.borrow_mut())
        };
        *self.hooks.borrow_mut() = PromiseHooks::default();
        for continuation in continuations {
            continuation(next.clone());
        }
        true
    }

    pub fn on_settle(&self, continuation: Rc<dyn Fn(PromiseState)>) {
        let state = self.state();
        if matches!(state, PromiseState::Pending) {
            self.continuations.borrow_mut().push(continuation);
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
        Rc::ptr_eq(&self.state, &other.state)
    }

    pub fn identity_address(&self) -> *const RefCell<PromiseState> {
        Rc::as_ptr(&self.state)
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
