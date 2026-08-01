#![allow(clippy::missing_safety_doc)]

#[path = "../../../rust/src/core.rs"]
mod core;
#[path = "../../../rust/src/hta.rs"]
mod hta;
#[path = "../../../rust/src/json.rs"]
mod json;
#[path = "../../../rust/src/kernel.rs"]
mod kernel;
#[path = "../../../rust/src/lang.rs"]
mod lang;
#[path = "../../../rust/src/task.rs"]
mod task;

use core::{EvalFiber, EvalFiberState, Promise, PromiseRejection, PromiseState, Value};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;
use std::rc::Rc;

const ABI_VERSION: u32 = 1;
const REQUEST_BINDING: &str = "__openhal_request";

type HostCall = (u64, Promise, String, String, Vec<Value>);

#[repr(C)]
pub struct OpenHalBuffer {
    pub data: *mut u8,
    pub len: usize,
}

pub struct OpenHalRuntime {
    env: HashMap<String, Value>,
    namespaces: kernel::NamespaceRegistry<Value>,
    protocols: core::ProtocolRegistry,
    resources: Rc<RefCell<HashMap<String, String>>>,
    next_task: u64,
    next_call: u64,
    events: Rc<RefCell<VecDeque<Vec<u8>>>>,
    ready: Rc<RefCell<VecDeque<(u64, PromiseState)>>>,
    calls: HashMap<u64, (u64, Promise)>,
    fibers: HashMap<u64, EvalFiber>,
    tasks: HashMap<u64, Promise>,
}

impl OpenHalRuntime {
    fn new() -> Self {
        let namespaces = kernel::NamespaceRegistry::new("user");
        let foundation = namespaces.find_or_create("std.foundation");

        for (name, value) in core::exception_function_values() {
            foundation.intern(name, value);
        }
        for (name, protocol) in core::foundation_protocol_values() {
            foundation.intern(&name, protocol.clone());
            namespaces
                .find_or_create(core::builtin_protocol_namespace(&name))
                .intern(name, protocol);
        }
        for (namespace, name, method) in core::builtin_protocol_method_values() {
            namespaces.find_or_create(namespace).intern(name, method);
        }
        for (name, descriptor) in core::native_type_values() {
            let canonical_name = format!("std.native.{name}");
            let var = foundation.intern(&canonical_name, descriptor);
            foundation.map_var(lang::data::Symbol::parse(&name), var);
            namespaces.find_or_create(canonical_name);
        }

        core::refer_startup_defaults(&namespaces, "user");
        let mut env = HashMap::new();
        core::select_namespace_environment(&namespaces, &mut env, "user");

        let resources = Rc::new(RefCell::new(HashMap::new()));
        resources.borrow_mut().extend([
            (
                "std.foundation".into(),
                include_str!("../../../lib/src/std/foundation.hal").into(),
            ),
            (
                "std.foundation.promise".into(),
                include_str!("../../../lib/src/std/foundation/promise.hal").into(),
            ),
            (
                "std.foundation.coroutine".into(),
                include_str!("../../../lib/src/std/foundation/coroutine.hal").into(),
            ),
            (
                "std.foundation.host".into(),
                include_str!("../../../lib/src/std/foundation/host.hal").into(),
            ),
        ]);

        Self {
            env,
            namespaces,
            protocols: core::ProtocolRegistry::core(),
            resources,
            next_task: 1,
            next_call: 1,
            events: Rc::new(RefCell::new(VecDeque::new())),
            ready: Rc::new(RefCell::new(VecDeque::new())),
            calls: HashMap::new(),
            fibers: HashMap::new(),
            tasks: HashMap::new(),
        }
    }

    fn allocate_task(&mut self) -> u64 {
        let task = self.next_task;
        self.next_task = self.next_task.saturating_add(1);
        task
    }

    fn host_handler(
        &mut self,
        _task: u64,
    ) -> (
        Rc<dyn Fn(String, String, Vec<Value>) -> Result<Value, String>>,
        Rc<RefCell<Vec<HostCall>>>,
        Rc<RefCell<u64>>,
    ) {
        let pending = Rc::new(RefCell::new(Vec::new()));
        let queue = pending.clone();
        let next = Rc::new(RefCell::new(self.next_call));
        let ids = next.clone();
        let handler = Rc::new(move |service: String, method: String, args: Vec<Value>| {
            let call = *ids.borrow();
            *ids.borrow_mut() = call.saturating_add(1);
            let promise = Promise::new();
            queue
                .borrow_mut()
                .push((call, promise.clone(), service, method, args));
            Ok(Value::Promise(promise))
        });
        (handler, pending, next)
    }

    fn collect_calls(
        &mut self,
        task: u64,
        pending: Rc<RefCell<Vec<HostCall>>>,
        next: Rc<RefCell<u64>>,
    ) {
        self.next_call = *next.borrow();
        for (call, promise, service, method, args) in pending.borrow_mut().drain(..) {
            let value = Value::Vector(
                vec![
                    Value::Number(2),
                    Value::Number(call as i64),
                    Value::Number(task as i64),
                    Value::String("OPENHAL".into()),
                    Value::Nil,
                    Value::String(service),
                    Value::String(method),
                    Value::Vector(args.into()),
                ]
                .into(),
            );
            match hta::encode(&value) {
                Ok(bytes) => {
                    self.calls.insert(call, (task, promise));
                    self.events.borrow_mut().push_back(bytes);
                }
                Err(error) => {
                    promise.reject(format!("hta/value-unsupported: {error}"));
                }
            }
        }
    }

    fn start(&mut self, source: &str, binding: Option<Value>) -> u64 {
        let task = self.allocate_task();
        let mut environment = self.env.clone();
        if let Some(binding) = binding {
            environment.insert(REQUEST_BINDING.into(), binding);
        }

        let (handler, pending, next) = self.host_handler(task);
        let namespaces = self.namespaces.clone();
        let protocols = self.protocols.clone();
        let resources = self.resources.clone();
        let provider = Rc::new(move |name: &str| resources.borrow().get(name).cloned());

        let result = core::with_capability_providers(None, None, || {
            core::with_namespace_registry(&namespaces, || {
                core::with_namespace_source(provider, || {
                    core::with_protocols(&protocols, || {
                        core::with_host_calls(handler, || EvalFiber::start(source, environment))
                    })
                })
            })
        });

        self.collect_calls(task, pending, next);
        match result {
            Ok(fiber) => self.drive(task, fiber),
            Err(error) => self.push_event(event(1, task, error_value("eval/error", error))),
        }
        task
    }

    fn resume(&mut self, task: u64, state: PromiseState) {
        let Some(mut fiber) = self.fibers.remove(&task) else {
            return;
        };
        let (handler, pending, next) = self.host_handler(task);
        let namespaces = self.namespaces.clone();
        let protocols = self.protocols.clone();
        let resources = self.resources.clone();
        let provider = Rc::new(move |name: &str| resources.borrow().get(name).cloned());

        core::with_capability_providers(None, None, || {
            core::with_namespace_registry(&namespaces, || {
                core::with_namespace_source(provider, || {
                    core::with_protocols(&protocols, || {
                        core::with_host_calls(handler, || fiber.resume(state));
                    });
                });
            });
        });

        self.collect_calls(task, pending, next);
        self.drive(task, fiber);
    }

    fn drive(&mut self, task: u64, fiber: EvalFiber) {
        match fiber.state() {
            EvalFiberState::Suspended => {
                let promise = fiber.pending().expect("suspended fiber promise");
                let ready = self.ready.clone();
                promise.on_settle(Rc::new(move |state| {
                    ready.borrow_mut().push_back((task, state));
                }));
                self.fibers.insert(task, fiber);
            }
            EvalFiberState::Completed(Value::Promise(promise)) => {
                let events = self.events.clone();
                promise.on_settle(Rc::new(move |state| {
                    emit_settlement(&events, task, state)
                }));
                self.tasks.insert(task, promise);
            }
            EvalFiberState::Completed(value) => {
                self.env = fiber.environment();
                core::save_namespace_environment(&self.namespaces, &mut self.env);
                core::refresh_namespace_environment(&self.namespaces, &mut self.env);
                self.push_event(event(0, task, value));
            }
            EvalFiberState::Failed(error) => {
                self.push_event(event(1, task, error_value("eval/error", error)));
            }
            EvalFiberState::Cancelled => {
                self.push_event(event(
                    1,
                    task,
                    error_value("task/cancelled", "cancelled".into()),
                ));
            }
            EvalFiberState::Running => {
                self.push_event(event(
                    1,
                    task,
                    error_value("fiber/invalid-state", "running fiber escaped".into()),
                ));
            }
        }
    }

    fn drain_ready(&mut self) {
        loop {
            let next = self.ready.borrow_mut().pop_front();
            match next {
                Some((task, state)) => self.resume(task, state),
                None => break,
            }
        }
    }

    fn deliver(&mut self, call: u64, success: bool, payload: Value) -> Result<(), ()> {
        let Some((_task, promise)) = self.calls.remove(&call) else {
            return Err(());
        };
        if success {
            promise.resolve(payload);
        } else {
            promise.reject_value(payload);
        }
        self.drain_ready();
        Ok(())
    }

    fn cancel(&mut self, task: u64) -> bool {
        self.calls.retain(|_, (owner, _)| *owner != task);
        if let Some(mut fiber) = self.fibers.remove(&task) {
            fiber.cancel();
            self.push_event(event(
                1,
                task,
                error_value("task/cancelled", "cancelled".into()),
            ));
            return true;
        }
        if let Some(promise) = self.tasks.remove(&task) {
            promise.reject("cancelled");
            return true;
        }
        false
    }

    fn drop_task(&mut self, task: u64) {
        self.fibers.remove(&task);
        self.tasks.remove(&task);
        self.calls.retain(|_, (owner, _)| *owner != task);
    }

    fn push_event(&self, value: Value) {
        enqueue_event(&self.events, value);
    }
}

fn event(kind: i64, id: u64, value: Value) -> Value {
    Value::Vector(vec![Value::Number(kind), Value::Number(id as i64), value].into())
}

fn error_value(code: &str, message: String) -> Value {
    Value::Map(
        vec![
            (Value::Keyword("code".into()), Value::Keyword(code.into())),
            (Value::Keyword("message".into()), Value::String(message)),
            (Value::Keyword("origin".into()), Value::Keyword("openhal".into())),
            (Value::Keyword("retryable".into()), Value::Bool(false)),
        ]
        .into_iter()
        .collect(),
    )
}

fn emit_settlement(events: &Rc<RefCell<VecDeque<Vec<u8>>>>, task: u64, state: PromiseState) {
    let value = match state {
        PromiseState::Pending => return,
        PromiseState::Fulfilled(value) => event(0, task, value),
        PromiseState::Rejected(PromiseRejection::Value(value)) => event(1, task, value),
        PromiseState::Rejected(PromiseRejection::Message(message)) => {
            event(1, task, error_value("promise/rejected", message))
        }
    };
    enqueue_event(events, value);
}

fn enqueue_event(events: &Rc<RefCell<VecDeque<Vec<u8>>>>, value: Value) {
    let encoded = hta::encode(&value).or_else(|error| {
        hta::encode(&event(
            1,
            0,
            error_value("hta/value-unsupported", error),
        ))
    });
    if let Ok(bytes) = encoded {
        events.borrow_mut().push_back(bytes);
    }
}

fn bytes<'a>(pointer: *const u8, len: usize) -> Result<&'a [u8], ()> {
    if pointer.is_null() {
        if len == 0 {
            return Ok(&[]);
        }
        return Err(());
    }
    Ok(unsafe { std::slice::from_raw_parts(pointer, len) })
}

fn source<'a>(pointer: *const u8, len: usize) -> Result<&'a str, ()> {
    std::str::from_utf8(bytes(pointer, len)?).map_err(|_| ())
}

unsafe fn runtime_mut<'a>(runtime: *mut OpenHalRuntime) -> Result<&'a mut OpenHalRuntime, ()> {
    runtime.as_mut().ok_or(())
}

#[no_mangle]
pub extern "C" fn openhal_abi_version() -> u32 {
    ABI_VERSION
}

#[no_mangle]
pub extern "C" fn openhal_runtime_new() -> *mut OpenHalRuntime {
    match catch_unwind(AssertUnwindSafe(OpenHalRuntime::new)) {
        Ok(runtime) => Box::into_raw(Box::new(runtime)),
        Err(_) => ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn openhal_runtime_free(runtime: *mut OpenHalRuntime) {
    if !runtime.is_null() {
        drop(Box::from_raw(runtime));
    }
}

#[no_mangle]
pub unsafe extern "C" fn openhal_start(
    runtime: *mut OpenHalRuntime,
    source_ptr: *const u8,
    source_len: usize,
    binding_ptr: *const u8,
    binding_len: usize,
) -> u64 {
    catch_unwind(AssertUnwindSafe(|| {
        let runtime = runtime_mut(runtime)?;
        let source = source(source_ptr, source_len)?;
        let binding = if binding_len == 0 {
            None
        } else {
            Some(hta::decode(bytes(binding_ptr, binding_len)?).map_err(|_| ())?)
        };
        Ok::<u64, ()>(runtime.start(source, binding))
    }))
    .ok()
    .and_then(Result::ok)
    .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn openhal_poll(runtime: *mut OpenHalRuntime) -> usize {
    catch_unwind(AssertUnwindSafe(|| {
        let runtime = runtime_mut(runtime)?;
        runtime.drain_ready();
        Ok::<usize, ()>(runtime.events.borrow().len())
    }))
    .ok()
    .and_then(Result::ok)
    .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn openhal_next_event(
    runtime: *mut OpenHalRuntime,
    output: *mut OpenHalBuffer,
) -> i32 {
    if output.is_null() {
        return 1;
    }
    (*output).data = ptr::null_mut();
    (*output).len = 0;

    catch_unwind(AssertUnwindSafe(|| {
        let runtime = runtime_mut(runtime)?;
        runtime.drain_ready();
        let Some(bytes) = runtime.events.borrow_mut().pop_front() else {
            return Ok::<i32, ()>(1);
        };
        let boxed = bytes.into_boxed_slice();
        let len = boxed.len();
        let data = Box::into_raw(boxed) as *mut u8;
        (*output).data = data;
        (*output).len = len;
        Ok(0)
    }))
    .ok()
    .and_then(Result::ok)
    .unwrap_or(2)
}

#[no_mangle]
pub unsafe extern "C" fn openhal_buffer_free(data: *mut u8, len: usize) {
    if data.is_null() {
        return;
    }
    let slice = ptr::slice_from_raw_parts_mut(data, len);
    drop(Box::from_raw(slice));
}

#[no_mangle]
pub unsafe extern "C" fn openhal_deliver(
    runtime: *mut OpenHalRuntime,
    call: u64,
    success: i32,
    payload_ptr: *const u8,
    payload_len: usize,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        let runtime = runtime_mut(runtime)?;
        let payload = if payload_len == 0 {
            Value::Nil
        } else {
            hta::decode(bytes(payload_ptr, payload_len)?).map_err(|_| ())?
        };
        runtime
            .deliver(call, success != 0, payload)
            .map_err(|_| ())?;
        Ok::<i32, ()>(0)
    }))
    .ok()
    .and_then(Result::ok)
    .unwrap_or(1)
}

#[no_mangle]
pub unsafe extern "C" fn openhal_cancel(runtime: *mut OpenHalRuntime, task: u64) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        let runtime = runtime_mut(runtime)?;
        Ok::<i32, ()>(if runtime.cancel(task) { 0 } else { 1 })
    }))
    .ok()
    .and_then(Result::ok)
    .unwrap_or(1)
}

#[no_mangle]
pub unsafe extern "C" fn openhal_drop_task(runtime: *mut OpenHalRuntime, task: u64) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        let runtime = runtime_mut(runtime)?;
        runtime.drop_task(task);
        Ok::<i32, ()>(0)
    }))
    .ok()
    .and_then(Result::ok)
    .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn take_event(runtime: &mut OpenHalRuntime) -> Value {
        runtime.drain_ready();
        let bytes = runtime.events.borrow_mut().pop_front().unwrap();
        hta::decode(&bytes).unwrap()
    }

    #[test]
    fn synchronous_handler_returns_response_map() {
        let mut runtime = OpenHalRuntime::new();
        runtime.start(
            "{:status 200 :headers {\"content-type\" \"text/plain\"} :body \"hello\"}",
            None,
        );
        let Value::Vector(event) = take_event(&mut runtime) else {
            panic!("event vector")
        };
        assert!(matches!(event.get(0), Some(Value::Number(0))));
    }

    #[test]
    fn host_call_suspends_and_resumes_the_fiber() {
        let mut runtime = OpenHalRuntime::new();
        let task = runtime.start(
            "(do (std.foundation.coroutine/await (std.native.Host/call \"nginx\" \"sleep\" [1])) {:status 200 :body \"done\"})",
            None,
        );
        let Value::Vector(call) = take_event(&mut runtime) else {
            panic!("host event")
        };
        assert!(matches!(call.get(0), Some(Value::Number(2))));
        let call_id = match call.get(1) {
            Some(Value::Number(value)) => *value as u64,
            _ => panic!("call id"),
        };
        runtime.deliver(call_id, true, Value::Nil).unwrap();
        let Value::Vector(done) = take_event(&mut runtime) else {
            panic!("completion event")
        };
        assert!(matches!(done.get(0), Some(Value::Number(0))));
        assert!(matches!(done.get(1), Some(Value::Number(value)) if *value == task as i64));
    }
}
