#[path = "../../src/core.rs"]
mod core;
#[path = "../../src/hta.rs"]
mod hta;
#[path = "../../src/json.rs"]
mod json;
#[path = "../../src/kernel.rs"]
mod kernel;
#[path = "../../src/lang.rs"]
mod lang;
#[path = "../../src/task.rs"]
mod task;

use core::{EvalFiber, EvalFiberState, Promise, PromiseState, Value};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

#[no_mangle]
pub extern "C" fn version() -> i32 {
    1
}
#[no_mangle]
pub extern "C" fn add(left: i32, right: i32) -> i32 {
    left.wrapping_add(right)
}
#[no_mangle]
pub extern "C" fn alloc(size: usize) -> *mut u8 {
    unsafe { std::alloc::alloc(std::alloc::Layout::from_size_align(size.max(1), 1).unwrap()) }
}
#[no_mangle]
pub extern "C" fn hta_alloc(size: usize) -> *mut u8 {
    alloc(size)
}
#[no_mangle]
pub extern "C" fn hta_dealloc(pointer: *mut u8, size: usize) {
    if !pointer.is_null() {
        unsafe {
            std::alloc::dealloc(
                pointer,
                std::alloc::Layout::from_size_align(size.max(1), 1).unwrap(),
            )
        }
    }
}
#[no_mangle]
pub extern "C" fn hta_abi_version() -> i32 {
    1
}

struct Runtime {
    env: HashMap<String, Value>,
    namespaces: kernel::NamespaceRegistry<Value>,
    /// Guest protocol declarations and extensions must survive across HTA
    /// evaluations just like namespace bindings.  The native runtime owns the
    /// same registry; without this raw WASM kernels could load frame helpers
    /// but not the concrete `std.substrate` node.
    protocols: core::ProtocolRegistry,
    next_task: u64,
    next_call: u64,
    events: Rc<RefCell<VecDeque<Vec<u8>>>>,
    ready: Rc<RefCell<VecDeque<(u64, PromiseState)>>>,
    calls: HashMap<u64, (u64, Promise)>,
    fibers: HashMap<u64, EvalFiber>,
    tasks: HashMap<u64, Promise>,
    resources: Rc<RefCell<HashMap<String, String>>>,
}
impl Runtime {
    fn new() -> Self {
        let namespaces = kernel::NamespaceRegistry::new("user");
        let json_namespace = namespaces.find_or_create("std.foundation.json");
        json_namespace.intern(
            "read",
            core::native_function("json/read", 1, |arguments| match arguments.as_slice() {
                [Value::String(source)] => json::read(source),
                _ => Err("json/read expects a string".into()),
            }),
        );
        json_namespace.intern(
            "write",
            core::native_function("json/write", 1, |arguments| {
                json::write(&arguments[0]).map(Value::String)
            }),
        );
        json_namespace.intern(
            "write-pp",
            core::native_function("json/write-pp", 1, |arguments| {
                json::write_pretty(&arguments[0]).map(Value::String)
            }),
        );
        let mut env = HashMap::new();
        core::refresh_namespace_environment(&namespaces, &mut env);
        Self {
            env,
            namespaces,
            protocols: core::ProtocolRegistry::core(),
            next_task: 1,
            next_call: 1,
            events: Rc::new(RefCell::new(VecDeque::new())),
            ready: Rc::new(RefCell::new(VecDeque::new())),
            calls: HashMap::new(),
            fibers: HashMap::new(),
            tasks: HashMap::new(),
            resources: Rc::new(RefCell::new(HashMap::new())),
        }
    }
    fn event(&self, value: Value) {
        enqueue_event(&self.events, value);
    }
    fn host_handler(
        &mut self,
        _task: u64,
    ) -> (
        Rc<dyn Fn(String, String, Vec<Value>) -> Result<Value, String>>,
        Rc<RefCell<Vec<(u64, Promise, String, String, Vec<Value>)>>>,
        Rc<RefCell<u64>>,
    ) {
        let pending = Rc::new(RefCell::new(Vec::new()));
        let queue = pending.clone();
        let next = Rc::new(RefCell::new(self.next_call));
        let ids = next.clone();
        let handler = Rc::new(move |service: String, method: String, args: Vec<Value>| {
            let call = *ids.borrow();
            *ids.borrow_mut() += 1;
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
        pending: Rc<RefCell<Vec<(u64, Promise, String, String, Vec<Value>)>>>,
        next: Rc<RefCell<u64>>,
    ) {
        self.next_call = *next.borrow();
        for (call, promise, service, method, args) in pending.borrow_mut().drain(..) {
            let value = Value::Vector(
                vec![
                    Value::Number(2),
                    Value::Number(call as i64),
                    Value::Number(task as i64),
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
    fn start_fiber(&mut self, task: u64, source: &str) -> Result<(), String> {
        self.start_fiber_with_bindings(task, source, Vec::new())
    }
    fn start_fiber_with_bindings(
        &mut self,
        task: u64,
        source: &str,
        bindings: Vec<Value>,
    ) -> Result<(), String> {
        let (handler, pending, next) = self.host_handler(task);
        let namespaces = self.namespaces.clone();
        let protocols = self.protocols.clone();
        let resources = self.resources.clone();
        let provider = Rc::new(move |name: &str| resources.borrow().get(name).cloned());
        let mut environment = self.env.clone();
        for (index, value) in bindings.into_iter().enumerate() {
            environment.insert(format!("__hta_arg_{index}"), value);
        }
        let fiber = core::with_namespace_registry(&namespaces, || {
            core::with_namespace_source(provider, || {
                core::with_protocols(&protocols, || {
                    core::with_host_calls(handler, || EvalFiber::start(source, environment))
                })
            })
        })?;
        self.collect_calls(task, pending, next);
        self.drive(task, fiber);
        Ok(())
    }
    fn resume_fiber(&mut self, task: u64, state: PromiseState) {
        let Some(mut fiber) = self.fibers.remove(&task) else {
            return;
        };
        let (handler, pending, next) = self.host_handler(task);
        let namespaces = self.namespaces.clone();
        let protocols = self.protocols.clone();
        let resources = self.resources.clone();
        let provider = Rc::new(move |name: &str| resources.borrow().get(name).cloned());
        core::with_namespace_registry(&namespaces, || {
            core::with_namespace_source(provider, || {
                core::with_protocols(&protocols, || {
                    core::with_host_calls(handler, || {
                        fiber.resume(state);
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
                    ready.borrow_mut().push_back((task, state))
                }));
                self.fibers.insert(task, fiber);
            }
            EvalFiberState::Completed(Value::Promise(promise)) => {
                let events = self.events.clone();
                promise.on_settle(Rc::new(move |state| emit_settlement(&events, task, state)));
                self.tasks.insert(task, promise);
            }
            EvalFiberState::Completed(value) => {
                self.env = fiber.environment();
                core::save_namespace_environment(&self.namespaces, &mut self.env);
                core::refresh_namespace_environment(&self.namespaces, &mut self.env);
                self.event(event(0, task, value));
            }
            EvalFiberState::Failed(error) => {
                self.event(event(1, task, error_value("eval/error", error)))
            }
            EvalFiberState::Cancelled => self.event(event(
                1,
                task,
                error_value("task/cancelled", "cancelled".into()),
            )),
            EvalFiberState::Running => self.event(event(
                1,
                task,
                error_value("fiber/invalid-state", "running fiber escaped".into()),
            )),
        }
    }
    fn drain_ready(&mut self) {
        loop {
            let next = { self.ready.borrow_mut().pop_front() };
            match next {
                Some((task, state)) => self.resume_fiber(task, state),
                None => break,
            }
        }
    }
}
thread_local! {static RUNTIME:RefCell<Runtime>=RefCell::new(Runtime::new());}
fn event(kind: i64, id: u64, value: Value) -> Value {
    Value::Vector(vec![Value::Number(kind), Value::Number(id as i64), value].into())
}
fn error_value(code: &str, message: String) -> Value {
    Value::Map(
        vec![
            (Value::Keyword("code".into()), Value::Keyword(code.into())),
            (Value::Keyword("message".into()), Value::String(message)),
            (
                Value::Keyword("origin".into()),
                Value::Keyword("wasm".into()),
            ),
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
        PromiseState::Rejected(error) => event(1, task, error_value("promise/rejected", error)),
    };
    enqueue_event(events, value);
}
fn enqueue_event(events: &Rc<RefCell<VecDeque<Vec<u8>>>>, value: Value) {
    match hta::encode(&value) {
        Ok(bytes) => events.borrow_mut().push_back(bytes),
        Err(error) => {
            let id = match &value {
                Value::Vector(values) => match values.get(1) {
                    Some(Value::Number(id)) => *id as u64,
                    _ => 0,
                },
                _ => 0,
            };
            let fallback = event(1, id, error_value("hta/value-unsupported", error));
            if let Ok(bytes) = hta::encode(&fallback) {
                events.borrow_mut().push_back(bytes);
            }
        }
    }
}
fn request(bytes: &[u8]) -> Result<(String, Vec<Value>), String> {
    match hta::decode(bytes)? {
        Value::Vector(values) if values.len() == 2 => {
            let target = match &values[0] {
                Value::String(value) => value.clone(),
                _ => return Err("hta/start target must be a string".into()),
            };
            let arguments = match &values[1] {
                Value::Vector(value) => value.iter().cloned().collect(),
                _ => return Err("hta/start arguments must be a vector".into()),
            };
            Ok((target, arguments))
        }
        _ => Err("hta/start expects [target arguments]".into()),
    }
}
#[no_mangle]
pub extern "C" fn hta_start(pointer: *const u8, size: usize) -> i64 {
    let bytes = if pointer.is_null() {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(pointer, size) }
    };
    RUNTIME.with(|cell| {
        let mut runtime = cell.borrow_mut();
        let task = runtime.next_task;
        runtime.next_task += 1;
        let result = match request(bytes) {
            Ok((target, args)) if target == "eval" => match args.as_slice() {
                [Value::String(source)] => runtime.start_fiber(task, source),
                _ => Err("hta eval expects one source string".into()),
            },
            Ok((target, args)) if target == "eval-bound" => match args.as_slice() {
                [Value::String(source), Value::Vector(bindings)] => runtime
                    .start_fiber_with_bindings(task, source, bindings.iter().cloned().collect()),
                _ => Err("hta eval-bound expects a source string and binding vector".into()),
            },
            Ok((target, args)) if target == "register-resource" => match args.as_slice() {
                [Value::String(name), Value::String(source)] => {
                    runtime
                        .resources
                        .borrow_mut()
                        .insert(name.clone(), source.clone());
                    runtime.event(event(0, task, Value::Bool(true)));
                    Ok(())
                }
                _ => Err("hta register-resource expects name and source strings".into()),
            },
            Ok((target, _)) => Err(format!("hta/target-unknown: {target}")),
            Err(error) => Err(error),
        };
        if let Err(error) = result {
            runtime.event(event(1, task, error_value("eval/error", error)));
        }
        runtime.drain_ready();
        task as i64
    })
}
fn output(bytes: Vec<u8>) -> i64 {
    let size = bytes.len();
    let pointer = alloc(size);
    if pointer.is_null() {
        return 0;
    }
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), pointer, size) };
    ((pointer as u64) << 32 | size as u64) as i64
}
#[no_mangle]
pub extern "C" fn hta_next_event() -> i64 {
    RUNTIME.with(|cell| {
        let mut runtime = cell.borrow_mut();
        runtime.drain_ready();
        let output_value = runtime
            .events
            .borrow_mut()
            .pop_front()
            .map(output)
            .unwrap_or(0);
        output_value
    })
}
#[no_mangle]
pub extern "C" fn hta_poll() -> i32 {
    RUNTIME.with(|cell| {
        let mut runtime = cell.borrow_mut();
        runtime.drain_ready();
        let count = runtime.events.borrow().len() as i32;
        count
    })
}
#[no_mangle]
pub extern "C" fn hta_deliver(pointer: *const u8, size: usize) -> i32 {
    let bytes = if pointer.is_null() {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(pointer, size) }
    };
    RUNTIME.with(|cell| {
        let mut runtime = cell.borrow_mut();
        let values = match hta::decode(bytes) {
            Ok(Value::Vector(values)) if values.len() == 3 => values,
            _ => return 1,
        };
        let call = match values[0] {
            Value::Number(v) if v >= 0 => v as u64,
            _ => return 1,
        };
        let state = match values[1] {
            Value::Number(v) => v,
            _ => return 1,
        };
        let payload = values[2].clone();
        let Some((_task, promise)) = runtime.calls.remove(&call) else {
            return 2;
        };
        if state == 0 {
            promise.resolve(payload);
        } else {
            promise.reject(match payload {
                Value::String(v) => v,
                v => v.display(),
            });
        }
        runtime.drain_ready();
        0
    })
}
#[no_mangle]
pub extern "C" fn hta_cancel(task: i64) -> i32 {
    RUNTIME.with(|cell| {
        let mut runtime = cell.borrow_mut();
        let task = task as u64;
        runtime.calls.retain(|_, (owner, _)| *owner != task);
        if let Some(mut fiber) = runtime.fibers.remove(&task) {
            fiber.cancel();
            runtime.event(event(
                1,
                task,
                error_value("task/cancelled", "cancelled".into()),
            ));
            return 0;
        }
        if let Some(promise) = runtime.tasks.remove(&task) {
            promise.reject("cancelled");
            return 0;
        }
        1
    })
}
#[no_mangle]
pub extern "C" fn hta_drop_task(task: i64) -> i32 {
    RUNTIME.with(|runtime| {
        let mut runtime = runtime.borrow_mut();
        let task = task as u64;
        runtime.fibers.remove(&task);
        runtime.tasks.remove(&task);
        runtime.calls.retain(|_, (owner, _)| *owner != task);
        0
    })
}
fn source_text(source_ptr: *const u8, source_len: usize) -> Result<&'static str, i32> {
    if source_ptr.is_null() {
        return Err(1);
    }
    let bytes = unsafe { std::slice::from_raw_parts(source_ptr, source_len) };
    std::str::from_utf8(bytes).map_err(|_| 1)
}

fn error_code(error: &str) -> i32 {
    let message = error.to_ascii_lowercase();
    if message.contains("division by zero") {
        return 5;
    }
    if message.contains("unbound symbol") || message.contains("unbound var") {
        return 2;
    }
    if message.contains("arity")
        || message.contains("at least")
        || message.contains("argument") && message.contains("expects")
    {
        return 3;
    }
    if message.contains("index") || message.contains("out of bounds") {
        return 6;
    }
    if message.contains("unknown") || message.contains("unsupported") {
        return 7;
    }
    if message.contains("parse") || message.contains("unexpected") || message.contains("unclosed") {
        return 1;
    }
    4
}

fn evaluate(source: &str) -> Result<i64, i32> {
    kernel::parse_forms(source).map_err(|_| 1)?;
    let mut env = HashMap::new();
    let namespaces = kernel::NamespaceRegistry::new("user");
    let protocols = core::ProtocolRegistry::core();
    let value = core::with_namespace_registry(&namespaces, || {
        core::with_protocols(&protocols, || core::eval_text(source, &mut env))
    })
    .map_err(|error| error_code(&error))?;
    value.parse::<i64>().map_err(|_| 4)
}

#[no_mangle]
pub extern "C" fn eval_i64(source_ptr: *const u8, source_len: usize) -> i64 {
    match source_text(source_ptr, source_len).and_then(evaluate) {
        Ok(value) => value,
        Err(_) => i64::MIN,
    }
}

/// Returns zero for a successful evaluation, otherwise a stable core.v1 error code.
#[no_mangle]
pub extern "C" fn eval_error_code(source_ptr: *const u8, source_len: usize) -> i32 {
    match source_text(source_ptr, source_len) {
        Ok(source) => {
            if kernel::parse_forms(source).is_err() {
                return 1;
            }
            let mut env = HashMap::new();
            let namespaces = kernel::NamespaceRegistry::new("user");
            let protocols = core::ProtocolRegistry::core();
            match core::with_namespace_registry(&namespaces, || {
                core::with_protocols(&protocols, || core::eval_text(source, &mut env))
            }) {
                Ok(_) => 0,
                Err(error) => error_code(&error),
            }
        }
        Err(code) => code,
    }
}

#[cfg(test)]
mod tests {
    use super::{eval_error_code, evaluate, Runtime};
    use crate::core::Value;
    use crate::lang::data::Symbol;
    use crate::lang::protocol::IDeref;

    #[test]
    fn parser_failures_have_the_stable_parse_code() {
        for source in [")", "[1", "123a", "\"unterminated"] {
            assert_eq!(evaluate(source), Err(1), "{source}");
            assert_eq!(
                eval_error_code(source.as_ptr(), source.len()),
                1,
                "{source}"
            );
        }
        assert_eq!(eval_error_code(b"(+ 1 2)".as_ptr(), 7), 0);
    }

    #[test]
    fn portable_type_descriptors_are_available_in_raw_wasm() {
        for source in [
            "(if (= (type nil) :hara.type/nil) 42 0)",
            "(if (= (type :key) :hara.type/keyword) 42 0)",
            "(if (= (type (symbol \"hara/name\")) :hara.type/symbol) 42 0)",
            "(if (= (type []) :hara.type/tuple) 42 0)",
            "(if (= (type (vector)) :hara.type/vector) 42 0)",
            "(if (= (type {}) :hara.type/ordered-map) 42 0)",
            "(if (= (type (ns:create (quote example))) :hara.type/namespace) 42 0)",
        ] {
            assert_eq!(evaluate(source), Ok(42), "{source}");
        }
    }

    #[test]
    fn iterator_lifecycle_matches_native_core_in_raw_wasm() {
        for source in [
            "(let (it (iter-cycle [1 2])) (do (iter-next it) (iter-close it) (if (iter-has? it) 0 42)))",
            "(let (it (iter-zip [1 2] [3 4])) (do (iter-close it) (if (iter-has? it) 0 42)))",
            "(let (it (iter-map (fn [x] x) [1 2])) (do (iter-close it) (if (iter-has? it) 0 42)))",
        ] {
            assert_eq!(evaluate(source), Ok(42), "{source}");
        }
    }

    fn completion_value(runtime: &mut Runtime, task: u64) -> crate::core::Value {
        let frame = runtime
            .events
            .borrow_mut()
            .pop_front()
            .expect("completion event");
        match super::hta::decode(&frame).unwrap() {
            crate::core::Value::Vector(values) => {
                assert_eq!(values[0], crate::core::Value::Number(0), "eval failed");
                assert_eq!(values[1], crate::core::Value::Number(task as i64));
                values[2].clone()
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn require_loads_registered_resource_and_binds_alias() {
        let mut runtime = Runtime::new();
        runtime.resources.borrow_mut().insert(
            "chrome.api".to_string(),
            "(ns chrome.api) (defn answer [] 42)".to_string(),
        );
        runtime
            .start_fiber(1, "(require [chrome.api :as api]) (api/answer)")
            .unwrap();
        assert_eq!(
            completion_value(&mut runtime, 1),
            crate::core::Value::Number(42)
        );
    }

    #[test]
    fn require_supports_ns_form_clauses_and_qualified_access() {
        let mut runtime = Runtime::new();
        runtime.resources.borrow_mut().insert(
            "acme.tools".to_string(),
            "(ns acme.tools) (defn seven [] 7)".to_string(),
        );
        runtime
            .start_fiber(
                2,
                "(ns demo (:require [acme.tools :as tools])) (tools/seven)",
            )
            .unwrap();
        assert_eq!(
            completion_value(&mut runtime, 2),
            crate::core::Value::Number(7)
        );
        runtime.start_fiber(3, "(acme.tools/seven)").unwrap();
        assert_eq!(
            completion_value(&mut runtime, 3),
            crate::core::Value::Number(7)
        );
    }

    #[test]
    fn fibers_preserve_guest_protocol_extensions() {
        let mut runtime = Runtime::new();
        runtime
            .start_fiber(
                1,
                "(defstruct Box [value]) (defprotocol ReadBox (read-box [self])) \
                 (extend-type Box ReadBox (read-box [self] (field self :value))) :ok",
            )
            .unwrap();
        assert!(matches!(
            completion_value(&mut runtime, 1),
            Value::Keyword(_)
        ));

        runtime
            .start_fiber(2, "(protocol-call ReadBox read-box (Box 42))")
            .unwrap();
        assert_eq!(completion_value(&mut runtime, 2), Value::Number(42));
    }

    #[test]
    fn bound_fibers_receive_hta_values_without_serializing_source() {
        let mut runtime = Runtime::new();
        runtime
            .start_fiber_with_bindings(
                1,
                "(get __hta_arg_0 :answer)",
                vec![Value::Map(
                    vec![(Value::Keyword("answer".into()), Value::Number(42))]
                        .into_iter()
                        .collect(),
                )],
            )
            .unwrap();
        assert_eq!(completion_value(&mut runtime, 1), Value::Number(42));
    }

    #[test]
    fn raw_kernels_expose_the_foundation_json_namespace() {
        let mut runtime = Runtime::new();
        runtime
            .start_fiber(
                1,
                "(ns example.json) (std.foundation.json/write {\"answer\" 42})",
            )
            .unwrap();
        assert_eq!(
            completion_value(&mut runtime, 1),
            Value::String("{\"answer\":42}".into())
        );
    }

    #[test]
    fn raw_kernels_run_the_shared_substrate_frame_fixture() {
        let mut runtime = Runtime::new();
        runtime.resources.borrow_mut().insert(
            "std.substrate.frame".into(),
            include_str!("../../../lib/src/std/substrate/frame.hal").into(),
        );
        runtime
            .start_fiber(
                1,
                include_str!("../../../lib/test-fixtures/std/substrate/frame_conformance.hal"),
            )
            .unwrap();
        assert_eq!(
            completion_value(&mut runtime, 1),
            Value::String(
                "{\"version\":\"substrate.v1\",\"kind\":\"request\",\"id\":\"req-1\",\"source\":\"client/a\",\"target\":\"server/b\",\"space\":\"workspace/main\",\"meta\":{\"trace\":\"trace-1\"},\"action\":\"math/add\",\"args\":[19,23],\"reply_to\":null,\"status\":null,\"data\":null,\"error\":null,\"signal\":null,\"cause\":null}".into(),
            )
        );
    }

    #[test]
    fn raw_kernels_run_atom_backed_substrate_request_stream_and_cancellation_lifecycle() {
        let mut runtime = Runtime::new();
        runtime.resources.borrow_mut().extend([
            (
                "std.substrate.protocol".into(),
                include_str!("../../../lib/src/std/substrate/protocol.hal").into(),
            ),
            (
                "std.substrate".into(),
                include_str!("../../../lib/src/std/substrate.hal").into(),
            ),
        ]);
        runtime
            .start_fiber(
                1,
                include_str!(
                    "../../../lib/test-fixtures/std/substrate/node_lifecycle_conformance.hal"
                ),
            )
            .unwrap();
        assert_eq!(
            completion_value(&mut runtime, 1),
            Value::Vector(
                vec![
                    Value::Number(84),
                    Value::Number(42),
                    Value::Keyword("rejected".into()),
                ]
                .into()
            ),
        );
    }

    #[test]
    fn require_missing_namespace_is_a_clean_error() {
        let mut runtime = Runtime::new();
        runtime.start_fiber(4, "(require [no.such.ns])").unwrap();
        let frame = runtime
            .events
            .borrow_mut()
            .pop_front()
            .expect("error event");
        match super::hta::decode(&frame).unwrap() {
            crate::core::Value::Vector(values) => {
                assert_eq!(values[0], crate::core::Value::Number(1), "expected failure");
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn fibers_persist_namespace_selection_defs_and_var_identity() {
        let mut runtime = Runtime::new();
        runtime
            .start_fiber(1, "(ns example.lib) (def answer 42)")
            .unwrap();

        assert_eq!(runtime.namespaces.current().name().as_str(), "example.lib");
        let namespace = runtime.namespaces.find("example.lib").unwrap();
        let answer = namespace.resolve(&Symbol::parse("answer")).unwrap();
        assert_eq!(answer.symbol().as_str(), "example.lib/answer");
        assert_eq!(answer.deref(), Value::Number(42));
        assert!(
            matches!(runtime.env.get("answer"), Some(Value::Var(var)) if var.same_identity(&answer))
        );

        runtime.start_fiber(2, "(ns user) (def local 7)").unwrap();
        assert_eq!(runtime.namespaces.current().name().as_str(), "user");
        assert_eq!(answer.deref(), Value::Number(42));
        assert!(runtime
            .namespaces
            .find("example.lib")
            .unwrap()
            .resolve(&Symbol::parse("answer"))
            .unwrap()
            .same_identity(&answer));
    }
}

#[no_mangle]
pub extern "C" fn hta_release(pointer: *const u8, size: usize) -> i32 {
    let bytes = if pointer.is_null() {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(pointer, size) }
    };
    match hta::decode(bytes) {
        Ok(Value::Extension(_)) => 0,
        _ => 1,
    }
}
