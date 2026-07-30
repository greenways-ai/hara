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
    2
}

struct Runtime {
    name: String,
    env: HashMap<String, Value>,
    namespaces: kernel::NamespaceRegistry<Value>,
    /// Guest protocol declarations and extensions must survive across HTA
    /// evaluations just like namespace bindings.  The native runtime owns the
    /// same registry; without this raw WASM kernels could load frame helpers
    /// but not the concrete `std.lib.substrate` node.
    protocols: core::ProtocolRegistry,
    next_call: u64,
    events: Rc<RefCell<VecDeque<Vec<u8>>>>,
    ready: Rc<RefCell<VecDeque<(u64, PromiseState)>>>,
    calls: HashMap<u64, (u64, Promise)>,
    fibers: HashMap<u64, EvalFiber>,
    tasks: HashMap<u64, Promise>,
    resources: Rc<RefCell<HashMap<String, String>>>,
    mount_id: Option<u64>,
}
impl Runtime {
    fn new() -> Self {
        Self::shared(
            "ROOT",
            Rc::new(RefCell::new(HashMap::new())),
            Rc::new(RefCell::new(VecDeque::new())),
        )
    }

    fn shared(
        name: &str,
        resources: Rc<RefCell<HashMap<String, String>>>,
        events: Rc<RefCell<VecDeque<Vec<u8>>>>,
    ) -> Self {
        let namespaces = kernel::NamespaceRegistry::new("user");
        let foundation_namespace = namespaces.find_or_create("std.foundation");
        for (name, value) in core::exception_function_values() {
            foundation_namespace.intern(name, value);
        }
        for (name, protocol) in core::foundation_protocol_values() {
            foundation_namespace.intern(&name, protocol.clone());
            namespaces
                .find_or_create(core::builtin_protocol_namespace(&name))
                .intern(name, protocol);
        }
        for (namespace, name, method) in core::builtin_protocol_method_values() {
            namespaces.find_or_create(namespace).intern(name, method);
        }
        let native_namespace = namespaces.find_or_create("std.native");
        for (name, descriptor) in core::native_type_values() {
            let var = native_namespace.intern(&name, descriptor);
            foundation_namespace.map_var(lang::data::Symbol::parse(&name), var);
        }
        let native_json = namespaces.find_or_create("std.native.Json");
        let json_read = native_json.intern(
            "read",
            core::native_function(
                "std.native.Json/read",
                1,
                |arguments| match arguments.as_slice() {
                    [Value::String(source)] => json::read(source),
                    _ => Err("json/read expects a string".into()),
                },
            ),
        );
        let json_write = native_json.intern(
            "write",
            core::native_function("std.native.Json/write", 1, |arguments| {
                json::write(&arguments[0]).map(Value::String)
            }),
        );
        let json_pretty = native_json.intern(
            "pretty",
            core::native_function("std.native.Json/pretty", 2, |arguments| {
                if core::map_entries(&arguments[1]).is_none() {
                    return Err("json/pretty expects an options map".into());
                }
                json::write_pretty(&arguments[0]).map(Value::String)
            }),
        );
        let json_namespace = namespaces.find_or_create("std.foundation.json");
        json_namespace.map_var(lang::data::Symbol::parse("read"), json_read);
        json_namespace.map_var(lang::data::Symbol::parse("write"), json_write);
        json_namespace.map_var(lang::data::Symbol::parse("pretty"), json_pretty);
        let native_edn = namespaces.find_or_create("std.native.Edn");
        let edn_read = native_edn.intern(
            "read",
            core::native_function(
                "std.native.Edn/read",
                1,
                |arguments| match arguments.as_slice() {
                    [Value::String(source)] => core::read_edn(source),
                    _ => Err("edn/read expects a string".into()),
                },
            ),
        );
        let edn_namespace = namespaces.find_or_create("std.foundation.edn");
        edn_namespace.map_var(lang::data::Symbol::parse("read"), edn_read);
        edn_namespace.intern(
            "write",
            core::native_function("edn/write", 1, |arguments| {
                Ok(Value::String(arguments[0].display()))
            }),
        );
        edn_namespace.intern(
            "pretty",
            core::native_function("edn/pretty", 2, |arguments| {
                if core::map_entries(&arguments[1]).is_none() {
                    return Err("edn/pretty expects an options map".into());
                }
                Ok(Value::String(arguments[0].display()))
            }),
        );
        core::refer_startup_defaults(&namespaces, "user");
        let mut env = HashMap::new();
        core::select_namespace_environment(&namespaces, &mut env, "user");
        Self {
            name: name.into(),
            env,
            namespaces,
            protocols: core::ProtocolRegistry::core(),
            next_call: 1,
            events,
            ready: Rc::new(RefCell::new(VecDeque::new())),
            calls: HashMap::new(),
            fibers: HashMap::new(),
            tasks: HashMap::new(),
            resources,
            mount_id: None,
        }
    }

    fn busy(&self) -> bool {
        !self.fibers.is_empty() || !self.tasks.is_empty() || !self.calls.is_empty()
    }

    fn complete(&self, prefix: &str) -> Value {
        let mut names = self.namespaces.visible_symbol_names();
        names.extend(self.env.keys().cloned());
        names.extend(
            core::completion_symbols()
                .iter()
                .map(|name| (*name).to_owned()),
        );
        names.sort();
        names.dedup();
        Value::Vector(
            names
                .into_iter()
                .filter(|name| name.starts_with(prefix))
                .map(Value::String)
                .collect::<Vec<_>>()
                .into(),
        )
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
                    Value::String(self.name.clone()),
                    self.mount_id
                        .map(|mount| Value::Number(mount as i64))
                        .unwrap_or(Value::Nil),
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
        let file_provider = self.mount_id.map(|_| {
            Rc::new(HostFileProvider {
                handler: handler.clone(),
            }) as Rc<dyn core::FileProvider>
        });
        let namespaces = self.namespaces.clone();
        let protocols = self.protocols.clone();
        let resources = self.resources.clone();
        let provider = Rc::new(move |name: &str| resources.borrow().get(name).cloned());
        let mut environment = self.env.clone();
        for (index, value) in bindings.into_iter().enumerate() {
            environment.insert(format!("__hta_arg_{index}"), value);
        }
        let fiber = core::with_capability_providers(file_provider, None, || {
            core::with_namespace_registry(&namespaces, || {
                core::with_namespace_source(provider, || {
                    core::with_protocols(&protocols, || {
                        core::with_host_calls(handler, || EvalFiber::start(source, environment))
                    })
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
        let file_provider = self.mount_id.map(|_| {
            Rc::new(HostFileProvider {
                handler: handler.clone(),
            }) as Rc<dyn core::FileProvider>
        });
        let namespaces = self.namespaces.clone();
        let protocols = self.protocols.clone();
        let resources = self.resources.clone();
        let provider = Rc::new(move |name: &str| resources.borrow().get(name).cloned());
        core::with_capability_providers(file_provider, None, || {
            core::with_namespace_registry(&namespaces, || {
                core::with_namespace_source(provider, || {
                    core::with_protocols(&protocols, || {
                        core::with_host_calls(handler, || {
                            fiber.resume(state);
                        });
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

struct HostFileProvider {
    handler: Rc<dyn Fn(String, String, Vec<Value>) -> Result<Value, String>>,
}

impl HostFileProvider {
    fn promise(&self, method: &str, arguments: Vec<Value>) -> Result<Promise, core::FileError> {
        match (self.handler)("file".into(), method.into(), arguments) {
            Ok(Value::Promise(promise)) => Ok(promise),
            Ok(_) => Err(core::FileError::Invalid(
                "file host call did not return a promise".into(),
            )),
            Err(error) => Err(core::FileError::Invalid(error)),
        }
    }
}

impl core::FileProvider for HostFileProvider {
    fn resolve(&self, root: &str, path: &str) -> Result<String, core::FileError> {
        if path.contains('\0') {
            return Err(core::FileError::Invalid("path contains NUL".into()));
        }
        let combined = format!("{}/{}", root.trim_end_matches('/'), path);
        let mut segments = Vec::new();
        for segment in combined.split('/') {
            match segment {
                "" | "." => {}
                ".." => {
                    if segments.pop().is_none() {
                        return Err(core::FileError::Denied);
                    }
                }
                value => segments.push(value),
            }
        }
        Ok(format!("/{}", segments.join("/")))
    }

    fn read(&self, path: &str) -> Result<Promise, core::FileError> {
        self.promise("read", vec![Value::String(path.into())])
    }

    fn write(&self, path: &str, bytes: Vec<u8>) -> Result<Promise, core::FileError> {
        self.promise(
            "write",
            vec![Value::String(path.into()), Value::Bytes(bytes)],
        )
    }

    fn exists(&self, path: &str) -> Result<Promise, core::FileError> {
        self.promise("exists", vec![Value::String(path.into())])
    }

    fn list(&self, path: &str) -> Result<Promise, core::FileError> {
        self.promise("list", vec![Value::String(path.into())])
    }

    fn mkdir(&self, path: &str) -> Result<Promise, core::FileError> {
        self.promise("mkdir", vec![Value::String(path.into())])
    }

    fn delete(&self, path: &str) -> Result<Promise, core::FileError> {
        self.promise("delete", vec![Value::String(path.into())])
    }
}

struct FilesystemMount {
    provider: String,
    key: Option<String>,
    attachments: usize,
}

struct KernelRuntime {
    next_task: u64,
    resources: Rc<RefCell<HashMap<String, String>>>,
    events: Rc<RefCell<VecDeque<Vec<u8>>>>,
    sessions: HashMap<String, Runtime>,
    task_sessions: HashMap<u64, String>,
    mounts: HashMap<u64, FilesystemMount>,
    next_mount_id: u64,
}

impl KernelRuntime {
    fn new() -> Self {
        let resources = Rc::new(RefCell::new(HashMap::new()));
        resources.borrow_mut().insert(
            "std.foundation.file".into(),
            include_str!("../../../lib/src/std/foundation/file.hal").into(),
        );
        resources.borrow_mut().insert(
            "std.foundation.host".into(),
            include_str!("../../../lib/src/std/foundation/host.hal").into(),
        );
        let events = Rc::new(RefCell::new(VecDeque::new()));
        let mut sessions = HashMap::new();
        sessions.insert(
            "ROOT".into(),
            Runtime::shared("ROOT", resources.clone(), events.clone()),
        );
        Self {
            next_task: 1,
            resources,
            events,
            sessions,
            task_sessions: HashMap::new(),
            mounts: HashMap::new(),
            next_mount_id: 1,
        }
    }

    fn task(&mut self) -> u64 {
        let task = self.next_task;
        self.next_task += 1;
        task
    }

    fn session(&self, name: &str) -> Result<&Runtime, String> {
        self.sessions
            .get(name)
            .ok_or_else(|| format!("NO_SESSION {name}"))
    }

    fn session_mut(&mut self, name: &str) -> Result<&mut Runtime, String> {
        self.sessions
            .get_mut(name)
            .ok_or_else(|| format!("NO_SESSION {name}"))
    }

    fn create_session(&mut self, name: &str) -> Result<(), String> {
        validate_session_name(name)?;
        if self.sessions.contains_key(name) {
            return Err(format!("SESSION_EXISTS {name}"));
        }
        self.sessions.insert(
            name.into(),
            Runtime::shared(name, self.resources.clone(), self.events.clone()),
        );
        Ok(())
    }

    fn create_filesystem(&mut self, descriptor: &Value) -> Result<u64, String> {
        let entries = core::map_entries(descriptor)
            .ok_or_else(|| "filesystem/create expects a provider descriptor map".to_string())?;
        let field = |name: &str| {
            entries.iter().find_map(|(key, value)| match key {
                Value::String(key) if key == name => Some(value.clone()),
                Value::Keyword(key) if key.as_str() == name => Some(value.clone()),
                _ => None,
            })
        };
        let provider = match field("provider") {
            Some(Value::String(provider)) => provider,
            Some(Value::Keyword(provider)) => provider.as_str().to_owned(),
            _ => return Err("filesystem/create requires a provider".into()),
        };
        if !matches!(provider.as_str(), "memory" | "indexeddb") {
            return Err(format!("FILESYSTEM_PROVIDER_UNSUPPORTED {provider}"));
        }
        let key = match field("key") {
            Some(Value::String(key)) if !key.is_empty() => Some(key),
            Some(Value::Nil) | None if provider == "memory" => None,
            None => return Err("filesystem/create indexeddb requires a key".into()),
            _ => return Err("filesystem/create key must be a non-empty string".into()),
        };
        let mount_id = self.next_mount_id;
        self.next_mount_id = self
            .next_mount_id
            .checked_add(1)
            .filter(|value| *value <= i64::MAX as u64)
            .ok_or_else(|| "FILESYSTEM_IDS_EXHAUSTED".to_string())?;
        self.mounts.insert(
            mount_id,
            FilesystemMount {
                provider,
                key,
                attachments: 0,
            },
        );
        Ok(mount_id)
    }

    fn attach_filesystem(&mut self, name: &str, mount_id: u64) -> Result<(), String> {
        let current = self.session(name)?;
        if current.busy() {
            return Err(format!("SESSION_BUSY {name}"));
        }
        if !self.mounts.contains_key(&mount_id) {
            return Err(format!("NO_FILESYSTEM {mount_id}"));
        }
        if self.session(name)?.mount_id == Some(mount_id) {
            return Ok(());
        }
        self.detach_filesystem(name)?;
        self.mounts.get_mut(&mount_id).unwrap().attachments += 1;
        self.session_mut(name)?.mount_id = Some(mount_id);
        Ok(())
    }

    fn detach_filesystem(&mut self, name: &str) -> Result<(), String> {
        let current = self.session(name)?;
        if current.busy() {
            return Err(format!("SESSION_BUSY {name}"));
        }
        let mount_id = current.mount_id;
        self.session_mut(name)?.mount_id = None;
        if let Some(mount_id) = mount_id {
            if let Some(mount) = self.mounts.get_mut(&mount_id) {
                mount.attachments = mount.attachments.saturating_sub(1);
            }
        }
        Ok(())
    }

    fn close_filesystem(&mut self, mount_id: u64) -> Result<(), String> {
        let mount = self
            .mounts
            .get(&mount_id)
            .ok_or_else(|| format!("NO_FILESYSTEM {mount_id}"))?;
        if mount.attachments != 0 {
            return Err(format!("FILESYSTEM_ATTACHED {mount_id}"));
        }
        self.mounts.remove(&mount_id);
        Ok(())
    }

    fn close_session(&mut self, name: &str) -> Result<(), String> {
        validate_session_name(name)?;
        if name == "ROOT" {
            return Err("ROOT_CANNOT_CLOSE".into());
        }
        if !self.sessions.contains_key(name) {
            return Err(format!("NO_SESSION {name}"));
        }
        let owned = self
            .task_sessions
            .iter()
            .filter_map(|(task, session)| (session == name).then_some(*task))
            .collect::<Vec<_>>();
        for task in owned {
            self.task_sessions.remove(&task);
            enqueue_event(
                &self.events,
                event(
                    1,
                    task,
                    error_value("session/closed", format!("session closed: {name}")),
                ),
            );
        }
        let mount_id = self.sessions.remove(name).and_then(|runtime| runtime.mount_id);
        if let Some(mount_id) = mount_id {
            if let Some(mount) = self.mounts.get_mut(&mount_id) {
                mount.attachments = mount.attachments.saturating_sub(1);
            }
        }
        Ok(())
    }

    fn drain_ready(&mut self) {
        for session in self.sessions.values_mut() {
            session.drain_ready();
        }
    }
}

fn validate_session_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
    {
        return Err("INVALID_SESSION_NAME".into());
    }
    Ok(())
}

thread_local! {static KERNEL:RefCell<KernelRuntime>=RefCell::new(KernelRuntime::new());}
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
    KERNEL.with(|cell| {
        let mut kernel = cell.borrow_mut();
        let task = kernel.task();
        let result = match request(bytes) {
            Ok((target, args)) => dispatch(&mut kernel, task, &target, args),
            Err(error) => Err(error),
        };
        if let Err(error) = result {
            enqueue_event(
                &kernel.events,
                event(1, task, error_value("eval/error", error)),
            );
        }
        kernel.drain_ready();
        task as i64
    })
}

fn dispatch(
    kernel: &mut KernelRuntime,
    task: u64,
    target: &str,
    args: Vec<Value>,
) -> Result<(), String> {
    match target {
        "eval" => dispatch_eval(kernel, task, "ROOT", &args, false),
        "eval-bound" => dispatch_eval(kernel, task, "ROOT", &args, true),
        "complete" => dispatch_complete(kernel, task, "ROOT", &args),
        "session/eval" => match args.as_slice() {
            [Value::String(session), Value::String(source)] => {
                dispatch_eval_values(kernel, task, session, source, None)
            }
            _ => Err("hta session/eval expects session and source strings".into()),
        },
        "session/eval-bound" => match args.as_slice() {
            [Value::String(session), Value::String(source), Value::Vector(bindings)] => {
                dispatch_eval_values(
                    kernel,
                    task,
                    session,
                    source,
                    Some(bindings.iter().cloned().collect()),
                )
            }
            _ => Err("hta session/eval-bound expects session, source, and binding vector".into()),
        },
        "session/complete" => match args.as_slice() {
            [Value::String(session), Value::String(prefix)] => {
                dispatch_complete_values(kernel, task, session, prefix)
            }
            _ => Err("hta session/complete expects session and prefix strings".into()),
        },
        "session/create" => match args.as_slice() {
            [Value::String(session)] => {
                kernel.create_session(session)?;
                enqueue_event(
                    &kernel.events,
                    event(0, task, Value::String(session.clone())),
                );
                Ok(())
            }
            _ => Err("hta session/create expects one session string".into()),
        },
        "session/list" => {
            if !args.is_empty() {
                return Err("hta session/list expects no arguments".into());
            }
            let mut sessions = kernel.sessions.keys().cloned().collect::<Vec<_>>();
            sessions.sort();
            enqueue_event(
                &kernel.events,
                event(
                    0,
                    task,
                    Value::Vector(
                        sessions
                            .into_iter()
                            .map(Value::String)
                            .collect::<Vec<_>>()
                            .into(),
                    ),
                ),
            );
            Ok(())
        }
        "session/info" => match args.as_slice() {
            [Value::String(session)] => {
                let runtime = kernel.session(session)?;
                let value = Value::Map(
                    vec![
                        (
                            Value::Keyword("session/id".into()),
                            Value::String(session.clone()),
                        ),
                        (
                            Value::Keyword("session/state".into()),
                            Value::Keyword(if runtime.busy() { "busy" } else { "idle" }.into()),
                        ),
                        (
                            Value::Keyword("session/filesystem".into()),
                            runtime
                                .mount_id
                                .map(|mount| Value::Number(mount as i64))
                                .unwrap_or(Value::Nil),
                        ),
                    ]
                    .into_iter()
                    .collect(),
                );
                enqueue_event(&kernel.events, event(0, task, value));
                Ok(())
            }
            _ => Err("hta session/info expects one session string".into()),
        },
        "session/attach-filesystem" => match args.as_slice() {
            [Value::String(session), Value::Number(mount_id)] if *mount_id > 0 => {
                kernel.attach_filesystem(session, *mount_id as u64)?;
                enqueue_event(&kernel.events, event(0, task, Value::Bool(true)));
                Ok(())
            }
            _ => Err("hta session/attach-filesystem expects a session string and mount id".into()),
        },
        "session/detach-filesystem" => match args.as_slice() {
            [Value::String(session)] => {
                kernel.detach_filesystem(session)?;
                enqueue_event(&kernel.events, event(0, task, Value::Bool(true)));
                Ok(())
            }
            _ => Err("hta session/detach-filesystem expects one session string".into()),
        },
        "filesystem/create" => match args.as_slice() {
            [descriptor] => {
                let mount_id = kernel.create_filesystem(descriptor)?;
                enqueue_event(
                    &kernel.events,
                    event(0, task, Value::Number(mount_id as i64)),
                );
                Ok(())
            }
            _ => Err("hta filesystem/create expects one provider descriptor".into()),
        },
        "filesystem/info" => match args.as_slice() {
            [Value::Number(mount_id)] if *mount_id > 0 => {
                let mount = kernel
                    .mounts
                    .get(&(*mount_id as u64))
                    .ok_or_else(|| format!("NO_FILESYSTEM {mount_id}"))?;
                let value = Value::Map(
                    vec![
                        (
                            Value::Keyword("filesystem/id".into()),
                            Value::Number(*mount_id),
                        ),
                        (
                            Value::Keyword("filesystem/provider".into()),
                            Value::Keyword(mount.provider.clone().into()),
                        ),
                        (
                            Value::Keyword("filesystem/key".into()),
                            mount
                                .key
                                .clone()
                                .map(Value::String)
                                .unwrap_or(Value::Nil),
                        ),
                        (
                            Value::Keyword("filesystem/attachments".into()),
                            Value::Number(mount.attachments as i64),
                        ),
                    ]
                    .into_iter()
                    .collect(),
                );
                enqueue_event(&kernel.events, event(0, task, value));
                Ok(())
            }
            _ => Err("hta filesystem/info expects one mount id".into()),
        },
        "filesystem/close" => match args.as_slice() {
            [Value::Number(mount_id)] if *mount_id > 0 => {
                kernel.close_filesystem(*mount_id as u64)?;
                enqueue_event(&kernel.events, event(0, task, Value::Bool(true)));
                Ok(())
            }
            _ => Err("hta filesystem/close expects one mount id".into()),
        },
        "session/close" => match args.as_slice() {
            [Value::String(session)] => {
                kernel.close_session(session)?;
                enqueue_event(&kernel.events, event(0, task, Value::Bool(true)));
                Ok(())
            }
            _ => Err("hta session/close expects one session string".into()),
        },
        "register-resource" => match args.as_slice() {
            [Value::String(name), Value::String(source)] => {
                kernel
                    .resources
                    .borrow_mut()
                    .insert(name.clone(), source.clone());
                enqueue_event(&kernel.events, event(0, task, Value::Bool(true)));
                Ok(())
            }
            _ => Err("hta register-resource expects name and source strings".into()),
        },
        "register-resources" => match args.as_slice() {
            [Value::Vector(resources)] => {
                let mut staged = Vec::with_capacity(resources.len());
                for entry in resources.iter() {
                    let Value::Vector(entry) = entry else {
                        return Err(
                            "hta register-resources expects string pairs".into(),
                        );
                    };
                    let (Some(Value::String(name)), Some(Value::String(source))) =
                        (entry.get(0), entry.get(1))
                    else {
                        return Err("hta register-resources expects string pairs".into());
                    };
                    if entry.len() != 2 {
                        return Err("hta register-resources expects string pairs".into());
                    }
                    staged.push((name.clone(), source.clone()));
                }
                kernel.resources.borrow_mut().extend(staged);
                enqueue_event(&kernel.events, event(0, task, Value::Bool(true)));
                Ok(())
            }
            _ => Err("hta register-resources expects one vector of string pairs".into()),
        },
        _ => Err(format!("hta/target-unknown: {target}")),
    }
}

fn dispatch_eval(
    kernel: &mut KernelRuntime,
    task: u64,
    session: &str,
    args: &[Value],
    bound: bool,
) -> Result<(), String> {
    if bound {
        match args {
            [Value::String(source), Value::Vector(bindings)] => dispatch_eval_values(
                kernel,
                task,
                session,
                source,
                Some(bindings.iter().cloned().collect()),
            ),
            _ => Err("hta eval-bound expects a source string and binding vector".into()),
        }
    } else {
        match args {
            [Value::String(source)] => dispatch_eval_values(kernel, task, session, source, None),
            _ => Err("hta eval expects one source string".into()),
        }
    }
}

fn dispatch_eval_values(
    kernel: &mut KernelRuntime,
    task: u64,
    session: &str,
    source: &str,
    bindings: Option<Vec<Value>>,
) -> Result<(), String> {
    validate_session_name(session)?;
    kernel.session(session)?;
    kernel.task_sessions.insert(task, session.into());
    let runtime = kernel.session_mut(session)?;
    match bindings {
        Some(bindings) => runtime.start_fiber_with_bindings(task, source, bindings),
        None => runtime.start_fiber(task, source),
    }
}

fn dispatch_complete(
    kernel: &mut KernelRuntime,
    task: u64,
    session: &str,
    args: &[Value],
) -> Result<(), String> {
    match args {
        [Value::String(prefix)] => dispatch_complete_values(kernel, task, session, prefix),
        _ => Err("hta complete expects one prefix string".into()),
    }
}

fn dispatch_complete_values(
    kernel: &mut KernelRuntime,
    task: u64,
    session: &str,
    prefix: &str,
) -> Result<(), String> {
    let value = kernel.session(session)?.complete(prefix);
    enqueue_event(&kernel.events, event(0, task, value));
    Ok(())
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
    KERNEL.with(|cell| {
        let mut kernel = cell.borrow_mut();
        kernel.drain_ready();
        let output_value = kernel
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
    KERNEL.with(|cell| {
        let mut kernel = cell.borrow_mut();
        kernel.drain_ready();
        let count = kernel.events.borrow().len() as i32;
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
    KERNEL.with(|cell| {
        let mut kernel = cell.borrow_mut();
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
        let Some(runtime) = kernel
            .sessions
            .values_mut()
            .find(|runtime| runtime.calls.contains_key(&call))
        else {
            return 2;
        };
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
        kernel.drain_ready();
        0
    })
}
#[no_mangle]
pub extern "C" fn hta_cancel(task: i64) -> i32 {
    KERNEL.with(|cell| {
        let mut kernel = cell.borrow_mut();
        let task = task as u64;
        let Some(session) = kernel.task_sessions.get(&task).cloned() else {
            return 1;
        };
        let Some(runtime) = kernel.sessions.get_mut(&session) else {
            return 1;
        };
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
    KERNEL.with(|kernel| {
        let mut kernel = kernel.borrow_mut();
        let task = task as u64;
        if let Some(session) = kernel.task_sessions.remove(&task) {
            if let Some(runtime) = kernel.sessions.get_mut(&session) {
                runtime.fibers.remove(&task);
                runtime.tasks.remove(&task);
                runtime.calls.retain(|_, (owner, _)| *owner != task);
            }
        }
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
    use super::{dispatch, eval_error_code, evaluate, KernelRuntime, Runtime};
    use crate::core::Value;
    use crate::lang::data::Symbol;
    use crate::lang::protocol::IDeref;

    fn result(kernel: &mut KernelRuntime) -> Vec<Value> {
        kernel.drain_ready();
        let bytes = kernel
            .events
            .borrow_mut()
            .pop_front()
            .expect("result event");
        match crate::hta::decode(&bytes).expect("valid HTA event") {
            Value::Vector(values) => values.iter().cloned().collect(),
            value => panic!("expected result vector, got {}", value.display()),
        }
    }

    #[test]
    fn kernel_sessions_isolate_namespaces_in_one_runtime() {
        let mut kernel = KernelRuntime::new();
        kernel.create_session("alpha").unwrap();
        kernel.create_session("beta").unwrap();

        dispatch(
            &mut kernel,
            1,
            "session/eval",
            vec![
                Value::String("alpha".into()),
                Value::String("(def answer 41) (+ answer 1)".into()),
            ],
        )
        .unwrap();
        assert!(matches!(
            result(&mut kernel).as_slice(),
            [Value::Number(0), Value::Number(1), Value::Number(42)]
        ));

        dispatch(
            &mut kernel,
            2,
            "session/eval",
            vec![
                Value::String("beta".into()),
                Value::String("(def answer 6) (* answer 7)".into()),
            ],
        )
        .unwrap();
        assert!(matches!(
            result(&mut kernel).as_slice(),
            [Value::Number(0), Value::Number(2), Value::Number(42)]
        ));

        dispatch(
            &mut kernel,
            3,
            "session/eval",
            vec![
                Value::String("alpha".into()),
                Value::String("answer".into()),
            ],
        )
        .unwrap();
        assert!(matches!(
            result(&mut kernel).as_slice(),
            [Value::Number(0), Value::Number(3), Value::Number(41)]
        ));
    }

    #[test]
    fn filesystem_reattachment_preserves_idle_session_state() {
        let mut kernel = KernelRuntime::new();
        kernel.create_session("example").unwrap();
        dispatch(
            &mut kernel,
            1,
            "session/eval",
            vec![
                Value::String("example".into()),
                Value::String("(def stale-value 42)".into()),
            ],
        )
        .unwrap();
        result(&mut kernel);

        let mount_id = kernel
            .create_filesystem(&Value::Map(
                vec![(
                    Value::Keyword("provider".into()),
                    Value::String("memory".into()),
                )]
                .into_iter()
                .collect(),
            ))
            .unwrap();
        kernel.attach_filesystem("example", mount_id).unwrap();
        assert_eq!(
            kernel
                .session("example")
                .unwrap()
                .complete("stale")
                .display(),
            "[\"stale-value\"]"
        );
        assert_eq!(
            kernel.session("example").unwrap().mount_id,
            Some(mount_id)
        );
        assert_eq!(
            kernel.close_filesystem(mount_id).unwrap_err(),
            format!("FILESYSTEM_ATTACHED {mount_id}")
        );
        kernel.detach_filesystem("example").unwrap();
        kernel.close_filesystem(mount_id).unwrap();
    }

    #[test]
    fn filesystem_reattachment_rejects_busy_session() {
        let mut kernel = KernelRuntime::new();
        kernel.create_session("busy").unwrap();
        dispatch(
            &mut kernel,
            1,
            "session/eval",
            vec![
                Value::String("busy".into()),
                Value::String(
                    "(deref (std.native.Host/call \"wait\" \"forever\" []))".into(),
                ),
            ],
        )
        .unwrap();
        let mount_id = kernel
            .create_filesystem(&Value::Map(
                vec![(
                    Value::Keyword("provider".into()),
                    Value::String("memory".into()),
                )]
                .into_iter()
                .collect(),
            ))
            .unwrap();
        assert_eq!(kernel.attach_filesystem("busy", mount_id).unwrap_err(), "SESSION_BUSY busy");
    }

    #[test]
    fn mounted_file_calls_use_the_canonical_hal_module_and_v2_identity() {
        let mut kernel = KernelRuntime::new();
        kernel.create_session("files").unwrap();
        let mount_id = kernel
            .create_filesystem(&Value::Map(
                vec![(
                    Value::Keyword("provider".into()),
                    Value::String("memory".into()),
                )]
                .into_iter()
                .collect(),
            ))
            .unwrap();
        kernel.attach_filesystem("files", mount_id).unwrap();
        dispatch(
            &mut kernel,
            1,
            "session/eval",
            vec![
                Value::String("files".into()),
                Value::String(
                    "(do (require [std.foundation.file :as file]) \
                     (file/write \"/note.bin\" (bytes 1 2 3)))"
                        .into(),
                ),
            ],
        )
        .unwrap();
        let call = result(&mut kernel);
        assert!(matches!(
            call.as_slice(),
            [
                Value::Number(2),
                Value::Number(_),
                Value::Number(1),
                Value::String(session),
                Value::Number(found_mount),
                Value::String(service),
                Value::String(method),
                Value::Vector(_)
            ] if session == "files"
                && *found_mount == mount_id as i64
                && service == "file"
                && method == "write"
        ));
    }

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
                assert_eq!(
                    values[0],
                    crate::core::Value::Number(0),
                    "eval failed for task {}: {}",
                    values[1].display(),
                    values[2].display()
                );
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
    fn raw_kernels_expose_the_foundation_data_namespaces() {
        assert_eq!(crate::core::NATIVE_TYPES.len(), 19);
        assert_eq!(
            crate::core::NATIVE_TYPES
                .iter()
                .map(|(_, methods)| methods.len())
                .sum::<usize>(),
            120
        );
        let mut runtime = Runtime::new();
        assert!(runtime.env.contains_key("edn/write"));
        assert!(runtime.env.contains_key("ICount"));
        for native_type in [
            "Maths", "Numbers", "Bits", "String", "Bytes", "File", "Socket", "Promise",
            "Coroutine", "Arr", "Obj", "Runtime", "Printer", "Edn", "Json", "Host", "Regex",
            "UUID", "Error",
        ] {
            assert!(runtime.env.contains_key(native_type), "{native_type}");
        }
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
        runtime
            .start_fiber(2, "(std.foundation.edn/read \"{:answer 42}\")")
            .unwrap();
        assert_eq!(
            completion_value(&mut runtime, 2).display(),
            "{:answer 42}"
        );
        runtime
            .start_fiber(
                3,
                "(std.foundation.json/pretty {\"answer\" 42} {})",
            )
            .unwrap();
        assert_eq!(
            completion_value(&mut runtime, 3),
            Value::String("{\n  \"answer\": 42\n}".into())
        );
        runtime
            .start_fiber(4, "(std.foundation.edn/pretty {:answer 42} {})")
            .unwrap();
        assert_eq!(
            completion_value(&mut runtime, 4),
            Value::String("{:answer 42}".into())
        );
        runtime
            .start_fiber(
                5,
                "(try \
                   (throw (std.foundation/ex-info \"bad input\" {:kind :invalid})) \
                   (catch Throwable error \
                     [(std.foundation/ex-message error) \
                      (std.foundation/ex-data error)]))",
            )
            .unwrap();
        assert_eq!(
            completion_value(&mut runtime, 5).display(),
            "[\"bad input\" {:kind :invalid}]"
        );
        runtime
            .start_fiber(6, "(edn/write {:answer 42})")
            .unwrap();
        assert_eq!(
            completion_value(&mut runtime, 6),
            Value::String("{:answer 42}".into())
        );
        runtime.start_fiber(7, "(= Maths std.native/Maths)").unwrap();
        assert_eq!(completion_value(&mut runtime, 7), Value::Bool(true));
        runtime
            .start_fiber(
                8,
                "[(= Edn std.native/Edn std.foundation/Edn) \
                  (= Json std.native/Json std.foundation/Json) \
                  (= Maths std.native/Maths std.foundation/Maths)]",
            )
            .unwrap();
        assert_eq!(
            completion_value(&mut runtime, 8).display(),
            "[true true true]"
        );
        runtime.start_fiber(9, "(ICount/count [1 2 3])").unwrap();
        assert_eq!(completion_value(&mut runtime, 9), Value::Number(3));
    }

    #[test]
    fn raw_kernel_keeps_the_three_bang_name_compatibility_operations() {
        let mut runtime = Runtime::new();
        runtime
            .start_fiber(
                1,
                "(let (reference (atom 1)) \
                   [(reset! reference 2) \
                    (cas! reference 2 3) \
                    (cas! reference 2 4) \
                    (swap! reference (fn [value amount] (+ value amount)) 39) \
                    (deref reference)])",
            )
            .unwrap();
        assert_eq!(
            completion_value(&mut runtime, 1).display(),
            "[2 true false 42 42]"
        );
    }

    #[test]
    fn raw_kernels_run_the_shared_substrate_frame_fixture() {
        let mut runtime = Runtime::new();
        runtime.resources.borrow_mut().insert(
            "std.lib.substrate.frame".into(),
            include_str!("../../../lib/src/std/lib/substrate/frame.hal").into(),
        );
        runtime
            .start_fiber(
                1,
                include_str!("../../../lib/test-fixtures/std/lib/substrate/frame_conformance.hal"),
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
                "std.lib.substrate.protocol".into(),
                include_str!("../../../lib/src/std/lib/substrate/protocol.hal").into(),
            ),
            (
                "std.lib.substrate".into(),
                include_str!("../../../lib/src/std/lib/substrate.hal").into(),
            ),
        ]);
        runtime
            .start_fiber(
                1,
                include_str!(
                    "../../../lib/test-fixtures/std/lib/substrate/node_lifecycle_conformance.hal"
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
