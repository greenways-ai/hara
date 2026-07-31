#![allow(clippy::too_many_lines)] // Temporary compatibility facade during Java-port split.
#[cfg(not(target_arch = "wasm32"))]
pub mod asset;
#[cfg(not(target_arch = "wasm32"))]
pub mod cli_app;
mod core;
pub mod extension;
#[cfg(not(target_arch = "wasm32"))]
pub mod extension_tool;
pub mod hta;
mod json;
pub mod kernel;
pub mod lang;
#[cfg(not(target_arch = "wasm32"))]
pub mod identity_tool;
#[cfg(not(target_arch = "wasm32"))]
pub mod native_cli;
#[cfg(not(target_arch = "wasm32"))]
mod native_extension;
#[cfg(not(target_arch = "wasm32"))]
pub mod package;
#[cfg(not(target_arch = "wasm32"))]
mod process_extension;
#[cfg(not(target_arch = "wasm32"))]
pub mod project;
#[cfg(not(target_arch = "wasm32"))]
pub mod resp;
#[cfg(not(target_arch = "wasm32"))]
pub mod tap;
pub mod task;
#[cfg(feature = "dev-trace")]
pub mod trace;
// Experimental staged bytecode VM (issue #195). Non-default feature; the
// default evaluator is untouched.
#[cfg(feature = "bytecode-vm")]
pub mod vm;
#[cfg(not(target_arch = "wasm32"))]
pub mod wasmtime_provider;
use crate::kernel::Form;
use crate::lang::protocol::INamespaced;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use wasm_bindgen::prelude::*;

include!(concat!(env!("OUT_DIR"), "/embedded_hal.rs"));

const EAGER_HAL_RESOURCES: &[&str] = &[
    "std.foundation.string",
    "std.foundation.promise",
    "std.foundation.bytes",
    "std.foundation.coroutine",
    "std.foundation.file",
    "std.foundation.host",
    "std.foundation.socket",
    "std.foundation.edn",
    "std.foundation.json",
];

fn ignore_socket_event(_event: core::SocketEvent) {}

#[wasm_bindgen(start)]
pub fn init_wasm() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub struct PromiseHandle {
    promise: core::Promise,
}

#[wasm_bindgen]
impl PromiseHandle {
    fn from_promise(promise: core::Promise) -> PromiseHandle {
        PromiseHandle { promise }
    }

    #[wasm_bindgen(constructor)]
    pub fn new() -> PromiseHandle {
        PromiseHandle {
            promise: core::Promise::new(),
        }
    }

    pub fn state(&self) -> String {
        match self.promise.state() {
            core::PromiseState::Pending => "pending".into(),
            core::PromiseState::Fulfilled(_) => "fulfilled".into(),
            core::PromiseState::Rejected(_) => "rejected".into(),
        }
    }

    pub fn resolve(&self, value: &str) -> bool {
        self.promise.resolve(core::Value::String(value.into()))
    }

    pub fn reject(&self, error: &str) -> bool {
        self.promise.reject(error)
    }

    pub fn adopt(&self, other: &PromiseHandle) -> bool {
        self.promise.adopt(&other.promise)
    }

    pub fn value(&self) -> Result<String, JsValue> {
        match self.promise.state() {
            core::PromiseState::Pending => Err(JsValue::from_str("promise is pending")),
            core::PromiseState::Fulfilled(value) => Ok(value.display()),
            core::PromiseState::Rejected(error) => Err(JsValue::from_str(&error.message())),
        }
    }
}

#[wasm_bindgen]
pub struct Runtime {
    env: HashMap<String, core::Value>,
    protocols: core::ProtocolRegistry,
    extensions: core::ExtensionRegistry,
    wasm_extensions: HashMap<String, extension::WasmExtension>,
    providers: core::ProviderRegistry,
    resources: HashMap<String, String>,
    loaded_resources: HashSet<String>,
    namespace_registry: kernel::NamespaceRegistry<core::Value>,
    macros: Rc<RefCell<HashMap<(String, String), Rc<core::Function>>>>,
    generated_configs: HashMap<String, kernel::GeneratedNamespaceConfig>,
    #[cfg(feature = "dev-trace")]
    next_trace_id: u64,
    #[cfg(target_arch = "wasm32")]
    host_handler: Option<js_sys::Function>,
    #[cfg(not(target_arch = "wasm32"))]
    extension_roots: Vec<std::path::PathBuf>,
}

/// A process-local kernel that multiplexes isolated evaluator sessions.
///
/// Raw HTA exposes the same lifecycle over its wire targets; this native
/// facade keeps embedding hosts from treating a `Runtime` as the process
/// boundary when several independent sessions can share one kernel.
pub struct SessionKernel {
    sessions: HashMap<String, Runtime>,
    resources: HashMap<String, String>,
    mounts: HashMap<u64, FilesystemMount>,
    session_mounts: HashMap<String, u64>,
    next_mount_id: u64,
}

struct FilesystemMount {
    provider: Rc<dyn core::FileProvider>,
    kind: &'static str,
    key: String,
    attachments: usize,
}

impl Default for SessionKernel {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionKernel {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::from([("ROOT".into(), Runtime::new())]),
            resources: HashMap::new(),
            mounts: HashMap::new(),
            session_mounts: HashMap::new(),
            next_mount_id: 1,
        }
    }

    pub fn create_session(&mut self, name: &str) -> Result<(), String> {
        validate_session_name(name)?;
        if self.sessions.contains_key(name) {
            return Err(format!("SESSION_EXISTS {name}"));
        }
        let mut runtime = Runtime::new();
        for (resource, source) in &self.resources {
            runtime.register_resource(resource, source);
        }
        self.sessions.insert(name.into(), runtime);
        Ok(())
    }

    pub fn session_names(&self) -> Vec<String> {
        let mut names = self.sessions.keys().cloned().collect::<Vec<_>>();
        names.sort();
        names
    }

    pub fn session_namespace(&self, session: &str) -> Result<String, String> {
        self.sessions
            .get(session)
            .map(Runtime::current_namespace)
            .ok_or_else(|| format!("NO_SESSION {session}"))
    }

    pub fn eval(&mut self, session: &str, source: &str) -> Result<String, String> {
        self.sessions
            .get_mut(session)
            .ok_or_else(|| format!("NO_SESSION {session}"))?
            .eval_transfer_text(source)
    }

    pub fn register_resource(&mut self, name: &str, source: &str) {
        self.resources.insert(name.into(), source.into());
        for runtime in self.sessions.values_mut() {
            runtime.register_resource(name, source);
        }
    }

    pub fn create_memory_filesystem(&mut self, root: &str) -> u64 {
        self.create_filesystem(Rc::new(core::MemoryFileProvider::new(root)), "memory", root)
    }

    #[cfg(any(not(target_arch = "wasm32"), target_os = "wasi"))]
    pub fn create_native_filesystem(&mut self, root: &str) -> u64 {
        self.create_filesystem(Rc::new(core::NativeFileProvider::new(root)), "native", root)
    }

    fn create_filesystem(
        &mut self,
        provider: Rc<dyn core::FileProvider>,
        kind: &'static str,
        key: &str,
    ) -> u64 {
        let id = self.next_mount_id;
        self.next_mount_id = self
            .next_mount_id
            .checked_add(1)
            .expect("filesystem mount identifiers exhausted");
        self.mounts.insert(
            id,
            FilesystemMount {
                provider,
                kind,
                key: key.into(),
                attachments: 0,
            },
        );
        id
    }

    pub fn attach_filesystem(&mut self, session: &str, mount_id: u64) -> Result<(), String> {
        if !self.sessions.contains_key(session) {
            return Err(format!("NO_SESSION {session}"));
        }
        let provider = self
            .mounts
            .get(&mount_id)
            .ok_or_else(|| format!("NO_FILESYSTEM {mount_id}"))?
            .provider
            .clone();
        if self.session_mounts.get(session) == Some(&mount_id) {
            return Ok(());
        }
        self.detach_filesystem(session)?;
        self.mounts.get_mut(&mount_id).unwrap().attachments += 1;
        self.session_mounts.insert(session.into(), mount_id);
        self.sessions
            .get_mut(session)
            .unwrap()
            .providers
            .set_file(Some(provider));
        Ok(())
    }

    pub fn detach_filesystem(&mut self, session: &str) -> Result<(), String> {
        let runtime = self
            .sessions
            .get_mut(session)
            .ok_or_else(|| format!("NO_SESSION {session}"))?;
        runtime.providers.set_file(None);
        if let Some(mount_id) = self.session_mounts.remove(session) {
            if let Some(mount) = self.mounts.get_mut(&mount_id) {
                mount.attachments = mount.attachments.saturating_sub(1);
            }
        }
        Ok(())
    }

    pub fn filesystem(&self, session: &str) -> Option<u64> {
        self.session_mounts.get(session).copied()
    }

    pub fn filesystem_info(&self, mount_id: u64) -> Result<(&str, &str, usize), String> {
        self.mounts
            .get(&mount_id)
            .map(|mount| (mount.kind, mount.key.as_str(), mount.attachments))
            .ok_or_else(|| format!("NO_FILESYSTEM {mount_id}"))
    }

    pub fn close_filesystem(&mut self, mount_id: u64) -> Result<(), String> {
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

    pub fn close_session(&mut self, name: &str) -> Result<(), String> {
        validate_session_name(name)?;
        if name == "ROOT" {
            return Err("ROOT_CANNOT_CLOSE".into());
        }
        if !self.sessions.contains_key(name) {
            return Err(format!("NO_SESSION {name}"));
        }
        self.detach_filesystem(name)?;
        self.sessions.remove(name);
        Ok(())
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

/// The root Foundation surface deliberately contains only the iterator core.
/// Native iterator mechanics must enter through the `Iter/*` type alias, so
/// reject legacy unqualified call heads before namespace rewriting canonicalizes
/// an alias to its backing method name.
fn reject_legacy_iterator_calls(form: &Form) -> Result<(), String> {
    const LEGACY: &[&str] = &[
        "iter-has?", "iter-finite?", "iter-materialize", "iter-close", "iter-map",
        "iter-filter", "iter-take-while", "iter-drop-while", "iter-mapcat", "iter-keep",
        "iter-interpose", "iter-interleave", "iter-every?", "iter-any?", "iter-take",
        "iter-drop", "iter-zip", "iter-cycle", "iter-partition-pair", "iter-partition-all",
        "iter-partition", "iter-range", "iter-constantly", "iter-repeatedly", "iter-iterate",
    ];
    match form {
        Form::List(values) => {
            if let Some(Form::Symbol(name)) = values.first() {
                if LEGACY.contains(&name.as_str()) {
                    return Err(format!("unbound symbol: {name}"));
                }
                if name == "quote" {
                    return Ok(());
                }
            }
            for value in values {
                reject_legacy_iterator_calls(value)?;
            }
        }
        Form::Vector(values) | Form::Set(values) => {
            for value in values {
                reject_legacy_iterator_calls(value)?;
            }
        }
        Form::Map(entries) => {
            for (key, value) in entries {
                reject_legacy_iterator_calls(key)?;
                reject_legacy_iterator_calls(value)?;
            }
        }
        Form::Tagged(_, value) | Form::Metadata(_, value) => reject_legacy_iterator_calls(value)?,
        _ => {}
    }
    Ok(())
}

#[wasm_bindgen]
impl Runtime {
    fn empty() -> Runtime {
        let namespace_registry = kernel::NamespaceRegistry::new("user");
        let foundation = namespace_registry.find_or_create("std.foundation");
        foundation.intern_with_origin(
            "list",
            core::native_variadic_function("list", |values| Ok(core::Value::List(values.into()))),
            kernel::VarOrigin::RustLibrary,
        );
        for (name, value) in core::exception_function_values() {
            foundation.intern_with_origin(name, value, kernel::VarOrigin::RustLibrary);
        }
        for (name, value) in core::basic_function_values() {
            foundation.intern_with_origin(name, value, kernel::VarOrigin::RustLibrary);
        }
        for (name, protocol) in core::foundation_protocol_values() {
            foundation.intern(&name, protocol.clone());
            let namespace =
                namespace_registry.find_or_create(core::builtin_protocol_namespace(&name));
            namespace.intern(name, protocol);
        }
        for (namespace, name, method) in core::builtin_protocol_method_values() {
            namespace_registry
                .find_or_create(namespace)
                .intern(name, method);
        }
        let native = namespace_registry.find_or_create("std.native");
        for (name, descriptor) in core::native_type_values() {
            let canonical_name = format!("std.native.{name}");
            let var = foundation.intern(&canonical_name, descriptor);
            foundation.map_var(crate::lang::data::Symbol::parse(&name), var.clone());
            native.map_var(crate::lang::data::Symbol::parse(&name), var);
            namespace_registry.find_or_create(canonical_name);
        }
        for (native_type, methods) in core::NATIVE_TYPES {
            let namespace_name = format!("std.native.{native_type}");
            let namespace = namespace_registry.find_or_create(&namespace_name);
            for method in *methods {
                let dispatch_name = if *native_type == "Iter" {
                    (*method).to_owned()
                } else {
                    format!("{namespace_name}/{method}")
                };
                namespace.intern_with_origin(
                    *method,
                    core::structural_function_value(dispatch_name),
                    kernel::VarOrigin::RuntimePrimitive,
                );
            }
        }
        Runtime {
            env: HashMap::new(),
            protocols: core::ProtocolRegistry::core(),
            extensions: core::ExtensionRegistry::new(),
            wasm_extensions: HashMap::new(),
            providers: core::ProviderRegistry::new(),
            resources: HashMap::new(),
            loaded_resources: HashSet::new(),
            namespace_registry,
            macros: Rc::new(RefCell::new(HashMap::new())),
            generated_configs: HashMap::from([(
                "user".into(),
                kernel::GeneratedNamespaceConfig::defaults(),
            )]),
            #[cfg(feature = "dev-trace")]
            next_trace_id: 1,
            #[cfg(target_arch = "wasm32")]
            host_handler: None,
            #[cfg(not(target_arch = "wasm32"))]
            extension_roots: native_extension::configured_roots(),
        }
    }

    #[wasm_bindgen(constructor)]
    pub fn new() -> Runtime {
        let mut runtime = Runtime::empty();
        runtime
            .bootstrap_foundation()
            .expect("embedded std.foundation fallback must be valid");
        runtime
    }

    /// Creates the portable L0 evaluator without loading the language-level
    /// foundation. This is useful for small embedded surfaces whose commands
    /// only require core forms and should become interactive immediately.
    pub fn core() -> Runtime {
        Runtime::empty()
    }

    fn refer_native_types_into(&mut self, namespace: &str) {
        let target = self.namespace_registry.find_or_create(namespace);
        for (protocol, _) in core::FOUNDATION_PROTOCOLS {
            let protocol_namespace = core::builtin_protocol_namespace(protocol);
            if let Some(source) = self.namespace_registry.find(&protocol_namespace) {
                target.alias(protocol, source);
            }
        }
        for (native_type, _) in core::NATIVE_TYPES {
            let native_namespace = format!("std.native.{native_type}");
            if let Some(source) = self.namespace_registry.find(&native_namespace) {
                target.alias(*native_type, source);
            }
        }
        if let Some(native) = self.namespace_registry.find("std.native") {
            for (name, var) in native.mappings() {
                if target.resolve(&name).is_none() {
                    target.map_var(name.clone(), var.clone());
                }
                let canonical = crate::lang::data::Symbol::parse(&format!(
                    "std.native.{}",
                    name.as_str()
                ));
                if target.resolve(&canonical).is_none() {
                    target.map_var(canonical, var);
                }
            }
        }
    }

    fn refer_foundation_into(&mut self, namespace: &str) {
        self.refer_native_types_into(namespace);
        let target = self.namespace_registry.find_or_create(namespace);
        if namespace == "std.foundation" {
            return;
        }
        let Some(foundation) = self.namespace_registry.find("std.foundation") else {
            return;
        };
        for (name, var) in foundation.mappings() {
            if target.resolve(&name).is_none() {
                target.map_var(name, var);
            }
        }
    }

    fn bootstrap_foundation(&mut self) -> Result<(), String> {
        for &(name, _, source) in EMBEDDED_HAL_RESOURCES {
            self.register_resource(name, source);
        }
        let foundation = self
            .resources
            .get("std.foundation")
            .cloned()
            .ok_or_else(|| "embedded HAL catalog is missing std.foundation".to_owned())?;
        core::with_definition_origin(kernel::VarOrigin::HalFallback, || {
            self.eval_text(&foundation)
        })?;
        let foundation_namespace = self.namespace_registry.find_or_create("std.foundation");
        for name in core::structural_callable_names() {
            let symbol = crate::lang::data::Symbol::parse(name);
            if foundation_namespace.resolve(&symbol).is_none() {
                foundation_namespace.intern_with_origin(
                    name,
                    core::structural_function_value(name),
                    kernel::VarOrigin::RuntimePrimitive,
                );
            }
        }
        self.loaded_resources.insert("std.foundation".into());
        let json = self.namespace_registry.find_or_create("std.native.Json");
        json.intern(
            "read",
            core::native_function("std.native.Json/read", 1, |arguments| {
                match arguments.as_slice() {
                    [core::Value::String(source)] => json::read(source),
                    _ => Err("json/read expects a string".into()),
                }
            }),
        );
        json.intern(
            "write",
            core::native_function("std.native.Json/write", 1, |arguments| {
                json::write(&arguments[0]).map(core::Value::String)
            }),
        );
        json.intern(
            "pretty",
            core::native_function("std.native.Json/pretty", 2, |arguments| {
                if core::map_entries(&arguments[1]).is_none() {
                    return Err("json/pretty expects an options map".into());
                }
                json::write_pretty(&arguments[0]).map(core::Value::String)
            }),
        );
        for &name in EAGER_HAL_RESOURCES {
            let source = self
                .resources
                .get(name)
                .cloned()
                .ok_or_else(|| format!("embedded HAL catalog is missing {name}"))?;
            core::with_definition_origin(kernel::VarOrigin::HalFallback, || {
                self.eval_text(&source)
            })?;
            self.loaded_resources.insert(name.into());
        }
        self.use_namespace("std.foundation");
        self.refer_foundation_into("user");
        self.use_namespace("user");
        Ok(())
    }

    fn eval_text_mode(&mut self, source: &str, traced: bool) -> Result<String, String> {
        self.refresh_qualified_bindings();
        let forms = kernel::parse_forms(source)?;
        let result = self.eval_forms(forms, traced)?;
        self.save_namespace();
        self.refresh_qualified_bindings();
        Ok(result.display())
    }

    fn eval_transfer_text(&mut self, source: &str) -> Result<String, String> {
        self.refresh_qualified_bindings();
        let forms = kernel::parse_forms(source)?;
        let result = self.eval_forms(forms, false)?;
        self.save_namespace();
        self.refresh_qualified_bindings();
        if !core::session_transferable(&result) {
            return Err(format!(
                "SESSION_TRANSFER_REJECTED {}",
                core::portable_type_name(&result)
            ));
        }
        Ok(result.display())
    }

    pub fn eval_hir(&mut self, bytes: &[u8]) -> Result<String, String> {
        self.refresh_qualified_bindings();
        let module = kernel::hir::decode_hir(bytes)?;
        let result = self.eval_forms(module.forms, false)?;
        self.save_namespace();
        self.refresh_qualified_bindings();
        Ok(result.display())
    }

    fn eval_forms(&mut self, forms: Vec<Form>, traced: bool) -> Result<core::Value, String> {
        let mut result = core::Value::Nil;
        for form in forms {
            if let Form::List(values) = &form {
                if matches!(values.first(), Some(Form::Symbol(name)) if name == "ns") {
                    let name = match values.get(1) {
                        Some(Form::Symbol(name)) if !name.contains('/') => name.clone(),
                        _ => return Err("ns expects an unqualified namespace symbol".into()),
                    };
                    #[cfg(not(target_arch = "wasm32"))]
                    let roots = self.extension_roots.clone();
                    let config =
                        kernel::GeneratedNamespaceConfig::configure_with(&values[2..], |target| {
                            if self.namespace_registry.find(target).is_some()
                                || self.resources.contains_key(target)
                                || self.wasm_extensions.contains_key(target)
                            {
                                return true;
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                return native_extension::package_exists(target, &roots);
                            }
                            #[cfg(target_arch = "wasm32")]
                            false
                        })?;
                    for target in config.required_namespaces() {
                        if self.resources.contains_key(target)
                            || self.loaded_resources.contains(target)
                        {
                            continue;
                        }
                        if target == "std.foundation"
                            || target.starts_with("std.lib.")
                            || target.starts_with("std.foundation.")
                        {
                            continue;
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        self.install_discovered_extension(target)?;
                        self.load_wasm_extension_namespace(target)?;
                    }

                    let registry_before = self.namespace_registry.snapshot();
                    let environment_before = self.env.clone();
                    let macros_before = self.macros.borrow().clone();
                    let configs_before = self.generated_configs.clone();
                    let loaded_before = self.loaded_resources.clone();
                    self.generated_configs.insert(name.clone(), config);
                    self.use_namespace(&name);
                    let foundation_bootstrap_child = name.starts_with("std.foundation.");
                    let require_specs = values[2..]
                        .iter()
                        .flat_map(|clause| match clause {
                            Form::List(items)
                                if matches!(items.first(), Some(Form::Keyword(key)) if key == "require") =>
                            {
                                items[1..].to_vec()
                            }
                            Form::List(items)
                                if matches!(items.first(), Some(Form::Keyword(key)) if key == "use") =>
                            {
                                items[1..]
                                    .iter()
                                    .cloned()
                                    .map(|target| Form::Vector(vec![target]))
                                    .collect()
                            }
                            _ => Vec::new(),
                        })
                        // std.foundation is the host bootstrap namespace. Its
                        // child HAL libraries are rewritten against the
                        // catalog while it is still being assembled, so they
                        // must not recursively require the partially-built
                        // namespace through the ordinary module loader.
                        .filter(|spec| {
                            !foundation_bootstrap_child
                                || !matches!(spec,
                                Form::Vector(items)
                                    if matches!(items.first(), Some(Form::Symbol(target)) if target == "std.foundation"))
                        })
                        .collect::<Vec<_>>();
                    if !require_specs.is_empty() {
                        let require_form = Form::List(
                            std::iter::once(Form::Symbol("require".into()))
                                .chain(require_specs)
                                .collect(),
                        );
                        if let Err(error) = self.eval_form(require_form, traced) {
                            self.namespace_registry.restore(registry_before);
                            self.env = environment_before;
                            *self.macros.borrow_mut() = macros_before;
                            self.generated_configs = configs_before;
                            self.loaded_resources = loaded_before;
                            return Err(error);
                        }
                        let config = self
                            .generated_configs
                            .get(&name)
                            .expect("ns config was installed");
                        self.sync_generated_aliases(config);
                    }
                    result = core::Value::Nil;
                    continue;
                }
            }
            if let Form::List(values) = &form {
                if matches!(values.first(), Some(Form::Symbol(name)) if name == "require") {
                    let current = self.current_namespace();
                    let mut config = self
                        .generated_configs
                        .get(&current)
                        .cloned()
                        .unwrap_or_else(kernel::GeneratedNamespaceConfig::defaults);
                    {
                        #[cfg(not(target_arch = "wasm32"))]
                        let roots = self.extension_roots.clone();
                        let available = |target: &str| {
                            if self.namespace_registry.find(target).is_some()
                                || self.resources.contains_key(target)
                                || self.wasm_extensions.contains_key(target)
                            {
                                return true;
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                return native_extension::package_exists(target, &roots);
                            }
                            #[cfg(target_arch = "wasm32")]
                            false
                        };
                        for spec in &values[1..] {
                            config.apply_require(spec, &available)?;
                        }
                    }
                    self.sync_generated_aliases(&config);
                    self.generated_configs.insert(current, config);
                }
            }
            let config = self
                .generated_configs
                .get(&self.current_namespace())
                .cloned()
                .unwrap_or_else(kernel::GeneratedNamespaceConfig::defaults);
            reject_legacy_iterator_calls(&form)?;
            let resolved = config.rewrite(form);
            result = self.eval_form(resolved, traced)?;
            if matches!(result, core::Value::Recur(_)) {
                return Err("recur must be inside loop".into());
            }
            self.save_namespace();
            self.refresh_qualified_bindings();
        }
        self.save_namespace();
        self.refresh_qualified_bindings();
        Ok(result)
    }

    fn eval_text(&mut self, source: &str) -> Result<String, String> {
        self.eval_text_mode(source, false)
    }

    fn eval_form(&mut self, form: Form, traced: bool) -> Result<core::Value, String> {
        let namespace_source = self.namespace_source();
        if traced {
            return core::with_capability_providers(
                self.providers.file(),
                self.providers.socket(),
                || {
                    core::with_promise_provider(self.providers.promise(), || {
                        core::with_macros(self.macros.clone(), || {
                            core::with_namespace_registry(&self.namespace_registry, || {
                                core::with_namespace_source(namespace_source, || {
                                    core::with_protocols(&self.protocols, || {
                                        #[cfg(target_arch = "wasm32")]
                                        if let Some(handler) = &self.host_handler {
                                            let handler = handler.clone();
                                            return core::with_host_calls(
                                                host_call_bridge(handler),
                                                || core::eval_traced(&form, &mut self.env),
                                            );
                                        }
                                        core::eval_traced(&form, &mut self.env)
                                    })
                                })
                            })
                        })
                    })
                },
            );
        }
        let env = self.env.clone();
        let (result, fiber) = core::with_capability_providers(
            self.providers.file(),
            self.providers.socket(),
            || {
                core::with_promise_provider(self.providers.promise(), || {
                    core::with_macros(self.macros.clone(), || {
                        core::with_namespace_registry(&self.namespace_registry, || {
                            core::with_namespace_source(namespace_source, || {
                                core::with_protocols(&self.protocols, || -> Result<(Result<core::Value, String>, core::EvalFiber), String> {
                                    let mut fiber =
                                        core::EvalFiber::start_forms(vec![form], env)?;
                                    #[cfg(target_arch = "wasm32")]
                                    if let Some(handler) = &self.host_handler {
                                        let handler = handler.clone();
                                        let result = core::with_host_calls(
                                            host_call_bridge(handler),
                                            || fiber.drive_sync(),
                                        );
                                        return Ok((result, fiber));
                                    }
                                    Ok((fiber.drive_sync(), fiber))
                                })
                            })
                        })
                    })
                })
            },
        )?;
        self.env = fiber.environment();
        result
    }

    fn refresh_qualified_bindings(&mut self) {
        core::refresh_namespace_environment(&self.namespace_registry, &mut self.env);
    }

    fn save_namespace(&mut self) {
        core::save_namespace_environment(&self.namespace_registry, &mut self.env);
    }

    pub fn create_namespace(&mut self, name: &str) -> bool {
        if name.is_empty() || self.namespace_registry.find(name).is_some() {
            return false;
        }
        self.namespace_registry.find_or_create(name);
        true
    }

    pub fn use_namespace(&mut self, name: &str) -> bool {
        if name.is_empty() {
            return false;
        }
        let config = self
            .generated_configs
            .get(name)
            .cloned()
            .unwrap_or_else(kernel::GeneratedNamespaceConfig::defaults);
        if config.blank() {
            let target = self.namespace_registry.find_or_create(name);
            for (local, var) in target.mappings() {
                if var.symbol().get_namespace() != Some(name) {
                    target.unmap(&local);
                }
            }
            self.refer_native_types_into(name);
        } else {
            self.refer_foundation_into(name);
        }
        core::select_namespace_environment(&self.namespace_registry, &mut self.env, name);
        self.sync_generated_aliases(&config);
        self.refresh_qualified_bindings();
        true
    }

    fn sync_generated_aliases(&self, config: &kernel::GeneratedNamespaceConfig) {
        let target = self.namespace_registry.current();
        for (alias, namespace) in config.aliases() {
            if let Some(source) = self.namespace_registry.find(&namespace) {
                target.alias(alias, source);
            }
        }
        for namespace in config.used_namespaces() {
            if let Some(source) = self.namespace_registry.find(namespace) {
                for (symbol, var) in source.mappings() {
                    target.map_var(symbol, var);
                }
                let source_name = source.name().as_str().to_owned();
                let target_name = target.name().as_str().to_owned();
                let referred = self
                    .macros
                    .borrow()
                    .iter()
                    .filter_map(|((namespace, name), function)| {
                        (namespace == &source_name).then(|| (name.clone(), function.clone()))
                    })
                    .collect::<Vec<_>>();
                let mut macros = self.macros.borrow_mut();
                for (name, function) in referred {
                    macros.insert((target_name.clone(), name), function);
                }
            }
        }
    }

    pub fn visible_symbols(&self) -> Vec<String> {
        self.namespace_registry.visible_symbol_names()
    }

    pub fn current_namespace(&self) -> String {
        self.namespace_registry.current().name().as_str().to_owned()
    }

    pub fn alias_namespace(&mut self, alias: &str, target: &str) -> bool {
        if alias.is_empty() || target.is_empty() {
            return false;
        }
        let Some(target) = self.namespace_registry.find(target) else {
            return false;
        };
        self.namespace_registry.current().alias(alias, target);
        self.refresh_qualified_bindings();
        true
    }

    pub fn resolve_namespace(&self, name: &str) -> String {
        self.namespace_registry
            .current()
            .aliases()
            .into_iter()
            .find(|(alias, _)| alias.as_str() == name)
            .map(|(_, namespace)| namespace.name().as_str().to_owned())
            .unwrap_or_else(|| name.into())
    }

    /// Evaluates source after selecting a namespace.
    pub fn eval_in_namespace(&mut self, name: &str, source: &str) -> Result<String, JsValue> {
        let name = self.resolve_namespace(name);
        self.use_namespace(&name);
        self.eval_text(source)
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn require_resource_in_namespace(
        &mut self,
        resource: &str,
        namespace: &str,
    ) -> Result<String, JsValue> {
        let namespace = self.resolve_namespace(namespace);
        self.use_namespace(&namespace);
        self.require_resource(resource)
    }

    pub fn install_memory_file_provider(&mut self, root: &str) {
        self.providers
            .install_file(core::MemoryFileProvider::new(root));
    }

    #[cfg(any(not(target_arch = "wasm32"), target_os = "wasi"))]
    pub fn install_native_file_provider(&mut self, root: &str) {
        self.providers
            .install_file(core::NativeFileProvider::new(root));
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn install_native_socket_provider(&mut self) {
        self.providers
            .install_socket(core::NativeSocketProvider::default());
    }

    pub fn install_loopback_socket_provider(&mut self) {
        self.providers
            .install_socket(core::LoopbackSocketProvider::default());
    }

    /// Installs the JS host handler that backs `std.native.Host/call`.
    #[cfg(target_arch = "wasm32")]
    pub fn install_host_handler(&mut self, handler: js_sys::Function) {
        self.host_handler = Some(handler);
    }

    pub fn file_resolve(&self, root: &str, path: &str) -> Result<String, JsValue> {
        let provider = self
            .providers
            .file()
            .ok_or_else(|| JsValue::from_str("file/unsupported"))?;
        provider
            .resolve(root, path)
            .map_err(|error| JsValue::from_str(&format!("file/{}", error.code())))
    }

    pub fn file_read(&self, path: &str) -> Result<PromiseHandle, JsValue> {
        let provider = self
            .providers
            .file()
            .ok_or_else(|| JsValue::from_str("file/unsupported"))?;
        provider
            .read(path)
            .map(PromiseHandle::from_promise)
            .map_err(|error| JsValue::from_str(&format!("file/{}", error.code())))
    }

    pub fn file_write(&self, path: &str, bytes: Vec<u8>) -> Result<PromiseHandle, JsValue> {
        let provider = self
            .providers
            .file()
            .ok_or_else(|| JsValue::from_str("file/unsupported"))?;
        provider
            .write(path, bytes)
            .map(PromiseHandle::from_promise)
            .map_err(|error| JsValue::from_str(&format!("file/{}", error.code())))
    }

    pub fn file_exists(&self, path: &str) -> Result<PromiseHandle, JsValue> {
        let provider = self
            .providers
            .file()
            .ok_or_else(|| JsValue::from_str("file/unsupported"))?;
        provider
            .exists(path)
            .map(PromiseHandle::from_promise)
            .map_err(|error| JsValue::from_str(&format!("file/{}", error.code())))
    }

    pub fn file_list(&self, path: &str) -> Result<PromiseHandle, JsValue> {
        let provider = self
            .providers
            .file()
            .ok_or_else(|| JsValue::from_str("file/unsupported"))?;
        provider
            .list(path)
            .map(PromiseHandle::from_promise)
            .map_err(|error| JsValue::from_str(&format!("file/{}", error.code())))
    }

    pub fn file_mkdir(&self, path: &str) -> Result<PromiseHandle, JsValue> {
        let provider = self
            .providers
            .file()
            .ok_or_else(|| JsValue::from_str("file/unsupported"))?;
        provider
            .mkdir(path)
            .map(PromiseHandle::from_promise)
            .map_err(|error| JsValue::from_str(&format!("file/{}", error.code())))
    }

    pub fn file_delete(&self, path: &str) -> Result<PromiseHandle, JsValue> {
        let provider = self
            .providers
            .file()
            .ok_or_else(|| JsValue::from_str("file/unsupported"))?;
        provider
            .delete(path)
            .map(PromiseHandle::from_promise)
            .map_err(|error| JsValue::from_str(&format!("file/{}", error.code())))
    }

    pub fn extension_available(&self, name: &str) -> bool {
        self.extensions.contains(name) || self.wasm_extensions.contains_key(name)
    }

    pub fn require_extension(&mut self, name: &str) -> Result<String, JsValue> {
        if self.wasm_extensions.contains_key(name) {
            return self
                .load_wasm_extension_namespace(name)
                .map_err(|error| JsValue::from_str(&error));
        }
        self.extensions
            .require(name, &mut self.protocols)
            .map_err(|error| JsValue::from_str(&error))
    }

    /// Registers a host-supplied Hara resource. Resources are source text, not executable host code.
    pub fn register_resource(&mut self, name: &str, source: &str) {
        let changed = self
            .resources
            .get(name)
            .is_some_and(|existing| existing != source);
        self.resources.insert(name.into(), source.into());
        if changed {
            self.loaded_resources.remove(name);
        }
    }

    /// Evaluates a registered resource in the current lexical namespace.
    pub fn load_resource(&mut self, name: &str) -> Result<String, JsValue> {
        let source = self
            .resources
            .get(name)
            .cloned()
            .ok_or_else(|| JsValue::from_str("module/not-found"))?;
        self.eval_text(&source)
            .map_err(|error| JsValue::from_str(&error))
    }

    /// Loads a resource once; subsequent requires return the current loaded marker.
    pub fn require_resource(&mut self, name: &str) -> Result<String, JsValue> {
        if self.loaded_resources.contains(name) {
            return Ok(":loaded".into());
        }
        if self.resources.contains_key(name) {
            let result = self.load_resource(name)?;
            self.loaded_resources.insert(name.into());
            return Ok(result);
        }
        if self.extensions.contains(name) {
            let result = self.require_extension(name)?;
            self.loaded_resources.insert(name.into());
            return Ok(result);
        }
        if self.wasm_extensions.contains_key(name) {
            let result = self
                .load_wasm_extension_namespace(name)
                .map_err(|error| JsValue::from_str(&error))?;
            self.loaded_resources.insert(name.into());
            return Ok(result);
        }
        Err(JsValue::from_str("module/not-found"))
    }

    pub fn file_supported(&self) -> bool {
        self.providers.capabilities().file
    }

    pub fn socket_supported(&self) -> bool {
        self.providers.capabilities().socket
    }

    /// Opens a callback-based socket and returns its provider-owned handle.
    pub fn socket_connect(&self, host: &str, port: u16) -> Result<u64, JsValue> {
        let provider = self
            .providers
            .socket()
            .ok_or_else(|| JsValue::from_str("socket/unsupported"))?;
        provider
            .connect(host, port, Rc::new(ignore_socket_event))
            .map_err(|error| JsValue::from_str(&format!("socket/{}", error.code())))
    }

    pub fn socket_send(&self, socket: u64, bytes: Vec<u8>) -> Result<usize, JsValue> {
        let provider = self
            .providers
            .socket()
            .ok_or_else(|| JsValue::from_str("socket/unsupported"))?;
        provider
            .send(socket, &bytes)
            .map_err(|error| JsValue::from_str(&format!("socket/{}", error.code())))
    }

    pub fn socket_close(&self, socket: u64) -> Result<(), JsValue> {
        let provider = self
            .providers
            .socket()
            .ok_or_else(|| JsValue::from_str("socket/unsupported"))?;
        provider
            .close(socket)
            .map_err(|error| JsValue::from_str(&format!("socket/{}", error.code())))
    }

    /// Returns whether a protocol method is registered in this runtime context.
    pub fn has_protocol_method(&self, protocol: &str, method: &str) -> bool {
        self.protocols.contains(protocol, method)
    }

    pub fn eval(&mut self, source: &str) -> Result<String, JsValue> {
        self.eval_text(source)
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn eval_traced(&mut self, source: &str) -> Result<String, JsValue> {
        self.eval_text_mode(source, true)
            .map_err(|error| JsValue::from_str(&error))
    }

    #[cfg(feature = "bytecode-vm")]
    #[wasm_bindgen(js_name = compileBytecodeArtifact)]
    pub fn compile_bytecode_artifact_js(&self, source: &str) -> Result<Vec<u8>, JsValue> {
        self.compile_bytecode_artifact(source)
            .map_err(|error| JsValue::from_str(&error))
    }

    #[cfg(feature = "bytecode-vm")]
    #[wasm_bindgen(js_name = evalBytecodeArtifact)]
    pub fn eval_bytecode_artifact_js(&mut self, bytes: &[u8]) -> Result<String, JsValue> {
        self.eval_bytecode_artifact(bytes)
            .map_err(|error| JsValue::from_str(&error))
    }

    #[cfg(any(not(target_arch = "wasm32"), target_os = "wasi"))]
    pub fn eval_native(&mut self, source: &str) -> Result<String, String> {
        self.eval_text(source)
    }

    #[cfg(any(not(target_arch = "wasm32"), target_os = "wasi"))]
    pub fn eval_native_traced(&mut self, source: &str) -> Result<String, String> {
        self.eval_text_mode(source, true)
    }
}

/// Experimental bytecode VM entry points (issue #195), gated behind the
/// non-default `bytecode-vm` feature. These accept only closed,
/// namespace-independent forms in the supported synchronous subset;
/// anything else fails as a typed compile error. There is no fallback to
/// the default evaluator, and `Runtime::eval_native` is unaffected.
///
/// Programs are returned inside `Rc` because compiled closures share the
/// program with their executing machines; `Rc::clone` is the cheap way to
/// pass one around.
#[cfg(feature = "bytecode-vm")]
pub fn compile_bytecode(source: &str) -> Result<std::rc::Rc<vm::Program>, String> {
    vm::compile_source(source)
        .map(std::rc::Rc::new)
        .map_err(|error| error.to_string())
}

/// Executes a previously compiled and validated program.
#[cfg(feature = "bytecode-vm")]
pub fn execute_bytecode(program: &std::rc::Rc<vm::Program>) -> Result<String, String> {
    vm::execute_program(program.clone())
        .map(|value| value.display())
        .map_err(|error| error.to_string())
}

/// Compiles source into a checksummed, versioned bytecode artifact.
#[cfg(feature = "bytecode-vm")]
pub fn compile_bytecode_artifact(source: &str) -> Result<Vec<u8>, String> {
    let program = compile_bytecode(source)?;
    vm::encode_program(program.as_ref())
}

/// Decodes, validates, and executes a bytecode artifact.
#[cfg(feature = "bytecode-vm")]
pub fn execute_bytecode_artifact(bytes: &[u8]) -> Result<String, String> {
    let program = std::rc::Rc::new(vm::decode_program(bytes)?);
    execute_bytecode(&program)
}

/// Compiles and executes a source string through the experimental VM.
#[cfg(feature = "bytecode-vm")]
pub fn eval_bytecode_native(source: &str) -> Result<String, String> {
    execute_bytecode(&compile_bytecode(source)?)
}

#[cfg(feature = "bytecode-vm")]
impl Runtime {
    /// Compiles source against this runtime's namespace registry:
    /// std.foundation vars and anything already interned are visible to
    /// the compiler's two-phase global check (issue #223). The program
    /// is validated but not executed; globals intern only at execution.
    pub fn compile_bytecode(&self, source: &str) -> Result<std::rc::Rc<vm::Program>, String> {
        vm::compile_source_with(source, &self.namespace_registry)
            .map(std::rc::Rc::new)
            .map_err(|error| error.to_string())
    }

    /// Compiles and executes through the experimental VM against this
    /// runtime's registry, then syncs the flat env so later `eval_native`
    /// calls see the vars the program interned. No fallback: unsupported
    /// forms fail as compile errors. `eval_native` is unaffected.
    pub fn eval_bytecode_native(&mut self, source: &str) -> Result<String, String> {
        let program = self.compile_bytecode(source)?;
        let result = vm::execute_program_with_globals(program, &self.namespace_registry)
            .map(|value| value.display())
            .map_err(|error| error.to_string());
        // Rebuild the env from the registry so mixed evaluator/VM usage
        // on one Runtime observes the same globals.
        let current = self.namespace_registry.current().name().as_str().to_owned();
        core::select_namespace_environment(&self.namespace_registry, &mut self.env, &current);
        result
    }

    /// Compiles against this runtime's namespaces and persists the validated
    /// program for later native or browser execution.
    pub fn compile_bytecode_artifact(&self, source: &str) -> Result<Vec<u8>, String> {
        let program = self.compile_bytecode(source)?;
        vm::encode_program(program.as_ref())
    }

    /// Executes a persisted artifact against this runtime's namespaces.
    pub fn eval_bytecode_artifact(&mut self, bytes: &[u8]) -> Result<String, String> {
        let program = std::rc::Rc::new(vm::decode_program(bytes)?);
        let result = vm::execute_program_with_globals(program, &self.namespace_registry)
            .map(|value| value.display())
            .map_err(|error| error.to_string());
        let current = self.namespace_registry.current().name().as_str().to_owned();
        core::select_namespace_environment(&self.namespace_registry, &mut self.env, &current);
        result
    }
}

impl Runtime {
    /// Evaluates once through the existing evaluator and returns a
    /// development-only structured trace.
    #[cfg(feature = "dev-trace")]
    pub fn eval_native_trace(&mut self, source: &str) -> Result<trace::Trace, String> {
        let trace_id = trace::TraceId(self.next_trace_id);
        self.next_trace_id += 1;
        let (result, trace) = core::with_development_trace(
            trace_id,
            trace::TraceLimits::default(),
            || self.eval_text_mode(source, true),
            |value, collector| collector.preview_value("result", value),
        );
        result.map(|_| trace)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn add_extension_root(&mut self, root: impl Into<std::path::PathBuf>) {
        self.extension_roots.push(root.into());
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn install_discovered_extension(&mut self, namespace: &str) -> Result<(), String> {
        if self.wasm_extensions.contains_key(namespace) {
            return Ok(());
        }
        let package =
            native_extension::ExtensionPackage::discover(namespace, &self.extension_roots)?
                .ok_or_else(|| format!("extension/not-found: {namespace}"))?;
        if package.manifest.provider == "hta" {
            let target = package
                .manifest
                .targets
                .get("node")
                .ok_or_else(|| format!("extension/target-unsupported: node for {namespace}"))?;
            if target.runtime != "process" {
                return Err(format!(
                    "extension/target-unsupported: node for {namespace}"
                ));
            }
            let module = package.resolve(&target.module)?;
            let provider = process_extension::ProcessExtensionProvider::new(module);
            return self.install_wasm_extension(
                &package.source,
                &package.descriptor.display().to_string(),
                provider,
            );
        }
        if package.manifest.provider != "wasm" {
            return Err(format!(
                "extension/provider-unsupported: {} for {namespace}",
                package.manifest.provider
            ));
        }
        let bytes = package.module_bytes()?;
        let provider = wasmtime_provider::WasmtimeExtensionProvider::compile(&bytes)?;
        self.install_wasm_extension(
            &package.source,
            &package.descriptor.display().to_string(),
            provider,
        )
    }

    pub fn install_wasm_extension<P: extension::WasmExtensionProvider + 'static>(
        &mut self,
        manifest_source: &str,
        origin: &str,
        provider: P,
    ) -> Result<(), String> {
        let manifest = extension::ExtensionManifest::parse(manifest_source, origin)?;
        let namespace = manifest.namespace.clone();
        if self.wasm_extensions.contains_key(&namespace)
            || self.extensions.contains(&namespace)
            || self.resources.contains_key(&namespace)
        {
            return Err(format!(
                "extension/ambiguous: namespace already registered: {namespace}"
            ));
        }
        let extension = extension::WasmExtension::new(manifest, provider)?;
        self.wasm_extensions.insert(namespace, extension);
        Ok(())
    }

    pub fn cancel_wasm_extension(&self, name: &str, request: u64) -> Result<(), String> {
        self.wasm_extensions
            .get(name)
            .ok_or_else(|| format!("extension/not-found: {name}"))?
            .cancel(request)
    }

    /// Invokes an installed WASM extension without routing the call through
    /// source text. Service hosts use this binary-safe boundary for HTA1
    /// arguments and results.
    pub fn invoke_wasm_extension(
        &mut self,
        namespace: &str,
        export: &str,
        arguments: &[extension::Value],
    ) -> Result<extension::Value, String> {
        let binding = self
            .wasm_extensions
            .get_mut(namespace)
            .ok_or_else(|| format!("extension/not-found: {namespace}"))?
            .require()?
            .into_iter()
            .find(|binding| binding.name == export)
            .ok_or_else(|| format!("extension/export-missing: {namespace}/{export}"))?;
        binding.invoke(arguments)
    }

    fn namespace_source(&self) -> Rc<dyn Fn(&str) -> Option<String>> {
        let resources = self.resources.clone();
        Rc::new(move |name: &str| resources.get(name).cloned())
    }

    fn load_wasm_extension_namespace(&mut self, name: &str) -> Result<String, String> {
        let bindings = self
            .wasm_extensions
            .get_mut(name)
            .ok_or_else(|| format!("extension/not-found: {name}"))?
            .require()?;
        let namespace = self.namespace_registry.find_or_create(name);
        for binding in bindings {
            let arity = binding.specification.arguments.len();
            let function_name = format!("{name}/{}", binding.name);
            let binding_name = binding.name.clone();
            namespace.intern(
                &binding_name,
                core::native_function(&function_name, arity, move |arguments| {
                    binding.invoke(&arguments)
                }),
            );
        }
        self.refresh_qualified_bindings();
        Ok(":loaded".into())
    }
}

#[cfg(target_arch = "wasm32")]
fn js_error_string(error: JsValue) -> String {
    if let Some(message) = error.as_string() {
        return message;
    }
    js_sys::Reflect::get(&error, &JsValue::from_str("message"))
        .ok()
        .and_then(|message| message.as_string())
        .unwrap_or_else(|| format!("{error:?}"))
}

#[cfg(target_arch = "wasm32")]
fn host_key_to_string(key: &core::Value) -> String {
    match key {
        core::Value::String(text) => text.clone(),
        core::Value::Keyword(keyword) => keyword.as_str().to_owned(),
        core::Value::Symbol(symbol) => symbol.as_str().to_owned(),
        other => other.display(),
    }
}

#[cfg(target_arch = "wasm32")]
fn host_seq_to_js<'a>(values: impl Iterator<Item = &'a core::Value>) -> Result<JsValue, String> {
    let array = js_sys::Array::new();
    for value in values {
        array.push(&value_to_js(value)?);
    }
    Ok(array.into())
}

#[cfg(target_arch = "wasm32")]
fn value_to_js(value: &core::Value) -> Result<JsValue, String> {
    match value {
        core::Value::Nil => Ok(JsValue::NULL),
        core::Value::Bool(flag) => Ok(JsValue::from_bool(*flag)),
        core::Value::Number(number)
            if (*number as i128).abs() <= js_sys::Number::MAX_SAFE_INTEGER as i128 =>
        {
            Ok(JsValue::from_f64(*number as f64))
        }
        core::Value::Number(number) => Ok(js_sys::BigInt::from(*number).into()),
        core::Value::Float(number) => Ok(JsValue::from_f64(*number)),
        core::Value::String(text) => Ok(JsValue::from_str(text)),
        core::Value::Keyword(keyword) => Ok(JsValue::from_str(keyword.as_str())),
        core::Value::Symbol(symbol) => Ok(JsValue::from_str(symbol.as_str())),
        core::Value::Bytes(bytes) => Ok(js_sys::Uint8Array::from(&bytes[..]).into()),
        core::Value::Vector(values) => host_seq_to_js(values.iter()),
        core::Value::List(values) => host_seq_to_js(values.iter()),
        core::Value::Set(values) => host_seq_to_js(values.iter()),
        core::Value::OrderedSet(values) => host_seq_to_js(values.iter()),
        core::Value::Map(values) => {
            let object = js_sys::Object::new();
            for (key, value) in values.iter() {
                js_sys::Reflect::set(
                    &object,
                    &JsValue::from_str(&host_key_to_string(key)),
                    &value_to_js(value)?,
                )
                .map_err(js_error_string)?;
            }
            Ok(object.into())
        }
        core::Value::OrderedMap(values) => {
            let object = js_sys::Object::new();
            for entry in values.iter() {
                js_sys::Reflect::set(
                    &object,
                    &JsValue::from_str(&host_key_to_string(&entry.0)),
                    &value_to_js(&entry.1)?,
                )
                .map_err(js_error_string)?;
            }
            Ok(object.into())
        }
        other => Err(format!(
            "std.native.Host/call type-not-transportable: {}",
            other.display()
        )),
    }
}

#[cfg(target_arch = "wasm32")]
fn js_to_value(value: &JsValue) -> Result<core::Value, String> {
    use crate::lang::data::{OrderedMap as POrderedMap, Vector as PVector};
    use wasm_bindgen::JsCast;

    if value.is_null() || value.is_undefined() {
        return Ok(core::Value::Nil);
    }
    if let Some(flag) = value.as_bool() {
        return Ok(core::Value::Bool(flag));
    }
    if value.is_bigint() {
        let integer: js_sys::BigInt = value.clone().unchecked_into();
        return i64::try_from(integer)
            .map(core::Value::Number)
            .map_err(|_| "std.native.Host/call bigint is outside the signed 64-bit range".into());
    }
    if let Some(number) = value.as_f64() {
        if number.fract() == 0.0
            && number >= js_sys::Number::MIN_SAFE_INTEGER
            && number <= js_sys::Number::MAX_SAFE_INTEGER
        {
            return Ok(core::Value::Number(number as i64));
        }
        return Ok(core::Value::Float(number));
    }
    if let Some(text) = value.as_string() {
        return Ok(core::Value::String(text));
    }
    if value.is_instance_of::<js_sys::Uint8Array>() {
        return Ok(core::Value::Bytes(js_sys::Uint8Array::new(value).to_vec()));
    }
    if js_sys::Array::is_array(value) {
        let array = js_sys::Array::from(value);
        let mut items = Vec::with_capacity(array.length() as usize);
        for index in 0..array.length() {
            items.push(js_to_value(&array.get(index))?);
        }
        return Ok(core::Value::Vector(PVector::from_iter(items)));
    }
    if value.is_object() {
        let entries = js_sys::Object::entries(value.unchecked_ref::<js_sys::Object>());
        let mut items = Vec::with_capacity(entries.length() as usize);
        for index in 0..entries.length() {
            let entry = js_sys::Array::from(&entries.get(index));
            let key = entry.get(0).as_string().unwrap_or_default();
            let item = js_to_value(&entry.get(1))?;
            items.push((core::Value::String(key), item));
        }
        return Ok(core::Value::OrderedMap(Box::new(POrderedMap::from_iter(
            items,
        ))));
    }
    Err("std.native.Host/call type-not-transportable: unsupported JS result".into())
}

#[cfg(target_arch = "wasm32")]
fn host_call_bridge(
    handler: js_sys::Function,
) -> Rc<dyn Fn(String, String, Vec<core::Value>) -> Result<core::Value, String>> {
    Rc::new(move |service, method, args| {
        let js_args = js_sys::Array::new();
        for value in &args {
            js_args.push(&value_to_js(value)?);
        }
        let result = handler
            .call3(
                &JsValue::NULL,
                &JsValue::from(service),
                &JsValue::from(method),
                js_args.as_ref(),
            )
            .map_err(js_error_string)?;
        js_to_value(&result)
    })
}

#[wasm_bindgen]
pub fn target_profile() -> String {
    if cfg!(target_os = "wasi") {
        "wasi".into()
    } else if cfg!(target_arch = "wasm32") {
        "wasm".into()
    } else {
        "native".into()
    }
}

#[wasm_bindgen]
pub fn version() -> String {
    "hara-wasm/0.1 L0 slice".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_text(relative: &str) -> Option<String> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(relative);
        match std::fs::read_to_string(&path) {
            Ok(content) => Some(content),
            Err(_) => {
                eprintln!(
                    "skipping: {} is unavailable (specs/docs submodule not initialized)",
                    path.display()
                );
                None
            }
        }
    }

    fn module_case(id: &str) -> Vec<(Form, Form)> {
        fn entry<'a>(entries: &'a [(Form, Form)], key: &str) -> Option<&'a Form> {
            entries
                .iter()
                .find_map(|(candidate, value)| match candidate {
                    Form::Keyword(name) if name == key => Some(value),
                    _ => None,
                })
        }

        let corpus = repo_text("specs/00-unsorted/platform-language/draft/conformance/modules.edn")
            .expect("specs submodule must be initialized for module conformance tests");
        let manifest = kernel::parse_forms(&corpus)
            .expect("module conformance corpus must parse")
            .remove(0);
        let Form::Map(manifest) = manifest else {
            panic!("module conformance corpus must be a map")
        };
        let Some(Form::Vector(cases)) = entry(&manifest, "cases") else {
            panic!("module conformance :cases must be a vector")
        };
        cases
            .iter()
            .find_map(|case| {
                let Form::Map(case) = case else {
                    return None;
                };
                matches!(entry(case, "id"), Some(Form::Keyword(candidate)) if candidate == id)
                    .then(|| case.clone())
            })
            .unwrap_or_else(|| panic!("missing module conformance case :{id}"))
    }

    fn module_expect(id: &str, key: &str) -> Form {
        let case = module_case(id);
        let expect = case.iter().find_map(|(candidate, value)| match candidate {
            Form::Keyword(name) if name == "expect" => Some(value),
            _ => None,
        });
        let Some(Form::Map(expect)) = expect else {
            panic!("module conformance case :{id} must have an :expect map")
        };
        expect
            .iter()
            .find_map(|(candidate, value)| {
                matches!(candidate, Form::Keyword(name) if name == key).then(|| value.clone())
            })
            .unwrap_or_else(|| panic!("module conformance case :{id} is missing :expect :{key}"))
    }

    fn module_runtime_profile(runtime: &str, key: &str) -> Form {
        fn entry<'a>(entries: &'a [(Form, Form)], key: &str) -> Option<&'a Form> {
            entries
                .iter()
                .find_map(|(candidate, value)| match candidate {
                    Form::Keyword(name) if name == key => Some(value),
                    _ => None,
                })
        }

        let corpus = repo_text("specs/00-unsorted/platform-language/draft/conformance/modules.edn")
            .expect("specs submodule must be initialized for module conformance tests");
        let manifest = kernel::parse_forms(&corpus)
            .expect("module conformance corpus must parse")
            .remove(0);
        let Form::Map(manifest) = manifest else {
            panic!("module conformance corpus must be a map")
        };
        let Some(Form::Map(profiles)) = entry(&manifest, "runtime/profiles") else {
            panic!("module conformance corpus must declare :runtime/profiles")
        };
        let Some(Form::Map(profile)) = entry(profiles, runtime) else {
            panic!("module conformance corpus has no :{runtime} profile")
        };
        entry(profile, key)
            .cloned()
            .unwrap_or_else(|| panic!("module runtime profile :{runtime} has no :{key}"))
    }

    fn host_conformance_case(id: &str) -> Vec<(Form, Form)> {
        fn entry<'a>(entries: &'a [(Form, Form)], key: &str) -> Option<&'a Form> {
            entries
                .iter()
                .find_map(|(candidate, value)| match candidate {
                    Form::Keyword(name) if name == key => Some(value),
                    _ => None,
                })
        }

        let document_source = repo_text("specs/00-unsorted/runtime/draft/host-runtime.edn")
            .expect("specs submodule must be initialized for host runtime conformance tests");
        let document = kernel::parse_forms(&document_source)
            .expect("Host runtime specification must parse")
            .remove(0);
        let Form::Map(document) = document else {
            panic!("Host runtime specification must be a map")
        };
        let Some(Form::Vector(cases)) = entry(&document, "host/conformance") else {
            panic!("Host runtime specification must declare :host/conformance")
        };
        cases
            .iter()
            .find_map(|case| {
                let Form::Map(case) = case else {
                    return None;
                };
                matches!(entry(case, "id"), Some(Form::Keyword(candidate)) if candidate == id)
                    .then(|| case.clone())
            })
            .unwrap_or_else(|| panic!("missing Host conformance case :{id}"))
    }

    #[test]
    fn session_kernel_mounts_preserve_state_and_enforce_lifetime() {
        let mut kernel = SessionKernel::new();
        kernel.create_session("alpha").unwrap();
        kernel.create_session("beta").unwrap();
        assert_eq!(
            kernel.eval("alpha", "(def answer 41) answer").unwrap(),
            "41"
        );
        assert_eq!(kernel.eval("beta", "(def answer 6) answer").unwrap(), "6");
        let mount = kernel.create_memory_filesystem("/workspace");
        kernel.attach_filesystem("alpha", mount).unwrap();
        kernel.attach_filesystem("beta", mount).unwrap();
        assert_eq!(kernel.filesystem("alpha"), Some(mount));
        assert_eq!(kernel.eval("alpha", "answer").unwrap(), "41");
        assert_eq!(
            kernel
                .eval(
                    "alpha",
                    "(do (require [std.foundation.file :as file]) \
                     (deref (file/write \"/workspace/shared.bin\" (bytes 7 8))))",
                )
                .unwrap(),
            "nil"
        );
        assert_eq!(
            kernel
                .eval(
                    "beta",
                    "(do (require [std.foundation.file :as file]) \
                     (deref (file/exists? \"/workspace/shared.bin\")))",
                )
                .unwrap(),
            "true"
        );
        assert_eq!(
            kernel
                .eval(
                    "alpha",
                    "(do (require [std.foundation.file :as file]) \
                     (deref (file/write \"/workspace/source.hal\" \
                       (str/encode-utf8 \"(+ 19 23)\"))))",
                )
                .unwrap(),
            "nil"
        );
        assert_eq!(
            kernel
                .eval(
                    "beta",
                    "(do (require [std.foundation.file :as file]) \
                     (str/decode-utf8 \
                       (deref (file/read \"/workspace/source.hal\"))))",
                )
                .unwrap(),
            "\"(+ 19 23)\""
        );
        assert_eq!(
            kernel.close_filesystem(mount).unwrap_err(),
            format!("FILESYSTEM_ATTACHED {mount}")
        );
        kernel.detach_filesystem("alpha").unwrap();
        kernel.detach_filesystem("beta").unwrap();
        kernel.close_filesystem(mount).unwrap();
        assert_eq!(
            kernel.session_names(),
            vec!["ROOT".to_string(), "alpha".to_string(), "beta".to_string()]
        );
    }

    fn ignore_socket_event(_event: core::SocketEvent) {}

    static SOCKET_EVENTS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    fn count_socket_event(_event: core::SocketEvent) {
        SOCKET_EVENTS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn native_socket_provider_sends_callbacks_and_bytes() {
        use crate::core::SocketProvider;
        use std::io::Read;
        use std::net::TcpListener;
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut bytes = [0u8; 3];
            stream.read_exact(&mut bytes).unwrap();
            bytes
        });
        SOCKET_EVENTS.store(0, std::sync::atomic::Ordering::SeqCst);
        let sockets = core::NativeSocketProvider::default();
        let handle = sockets
            .connect("127.0.0.1", port, Rc::new(count_socket_event))
            .unwrap();
        assert_eq!(sockets.send(handle, &[7, 8, 9]).unwrap(), 3);
        sockets.close(handle).unwrap();
        assert_eq!(server.join().unwrap(), [7, 8, 9]);
        assert_eq!(SOCKET_EVENTS.load(std::sync::atomic::Ordering::SeqCst), 3);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn native_socket_server_streams_real_tcp_events() {
        use crate::core::{PromiseState, SocketProvider};
        use std::io::Write;
        let sockets = core::NativeSocketProvider::default();
        let server = sockets.listen("127.0.0.1", 0, Rc::new(|_| {})).unwrap();
        let (host, port) = sockets.endpoint(server).unwrap();
        let stream = sockets.events(server).unwrap();
        let mut client = std::net::TcpStream::connect((host.as_str(), port)).unwrap();
        let open = sockets.next(stream).unwrap().wait_state();
        assert!(
            matches!(open, PromiseState::Fulfilled(value) if value.display().contains(":open"))
        );
        client.write_all(b"ping").unwrap();
        let data = sockets.next(stream).unwrap().wait_state();
        assert!(
            matches!(data, PromiseState::Fulfilled(value) if value.display().contains(":data") && value.display().contains("112 105 110 103"))
        );
        sockets.close(server).unwrap();
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn native_file_provider_round_trips_bytes() {
        use crate::core::FileProvider;
        let path = std::env::temp_dir().join(format!("hara-wasm-test-{}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        let provider = core::NativeFileProvider::new(&path);
        let resolved = provider
            .resolve(path.to_str().unwrap(), "data.bin")
            .unwrap();
        assert_eq!(
            provider.write(&resolved, vec![4, 5, 6]).unwrap().state(),
            core::PromiseState::Fulfilled(core::Value::Nil)
        );
        assert_eq!(
            provider.read(&resolved).unwrap().state(),
            core::PromiseState::Fulfilled(core::Value::Bytes(vec![4, 5, 6]))
        );
        std::fs::remove_file(resolved).unwrap();
        std::fs::remove_dir(path).unwrap();
    }

    #[test]
    fn extension_provider_values_load_and_iterate_through_protocols() {
        let mut runtime = Runtime::new();
        runtime.extensions.install(RangeExtension);
        assert!(runtime.extension_available("range"));
        assert_eq!(runtime.require_resource("range").unwrap(), ":loaded");
        let value = runtime
            .extensions
            .construct("range", "range", &[core::Value::Number(3)])
            .unwrap();
        assert_eq!(core::receiver_category(&value), "extension");
        runtime.env.insert("r".into(), value);
        assert_eq!(runtime.eval_text("(iter-next (iter r))").unwrap(), "0");
        assert_eq!(runtime.eval_text("(iter-next (iter r))").unwrap(), "0");
        assert_eq!(runtime.require_resource("range").unwrap(), ":loaded");
    }

    #[test]
    fn runtime_routes_file_operations_through_provider_registry() {
        let mut runtime = Runtime::new();
        assert!(!runtime.file_supported());
        runtime.install_memory_file_provider("/sandbox");
        assert!(runtime.file_supported());
        let path = runtime.file_resolve("/sandbox", "data.bin").unwrap();
        assert_eq!(
            runtime
                .file_write(&path, vec![1, 2, 3])
                .unwrap()
                .value()
                .unwrap(),
            "nil"
        );
        assert_eq!(
            runtime.file_read(&path).unwrap().value().unwrap(),
            "#bytes[1 2 3]"
        );
        runtime.install_loopback_socket_provider();
        assert!(runtime.socket_supported());
    }

    #[test]
    fn runtime_core_evaluates_embedded_commands() {
        let mut runtime = Runtime::core();
        assert_eq!(runtime.eval_text("(+ 19 23)").unwrap(), "42");
        assert_eq!(runtime.eval_text("(let (x 7) (* x 6))").unwrap(), "42");
        assert_eq!(runtime.eval_text("(if true 1 0)").unwrap(), "1");
    }

    #[test]
    fn portable_pretty_renderer_groups_and_breaks_documents() {
        let mut runtime = Runtime::new();
        runtime.require_resource("std.foundation.pretty").unwrap();
        assert_eq!(
            runtime
                .eval_text("(std.foundation.pretty/render \"abc\")")
                .unwrap(),
            "\"abc\""
        );
        let document = "[:group \"(\" [:nest 2 [:line] \"alpha\" [:line] \"beta\"] \")\"]";
        assert_eq!(
            runtime
                .eval_text(&format!(
                    "(std.foundation.pretty/render {document} {{:width 80}})"
                ))
                .unwrap(),
            "\"( alpha beta)\""
        );
        assert_eq!(
            runtime
                .eval_text(&format!(
                    "(std.foundation.pretty/render {document} {{:width 8}})"
                ))
                .unwrap(),
            "\"(\\n  alpha\\n  beta)\""
        );
    }

    #[test]
    fn threading_macros_expand_finite_iterator_clauses() {
        let mut runtime = Runtime::new();
        assert_eq!(runtime.eval_text("(cond-> 1 (= 1 1) inc)").unwrap(), "2");
        assert_eq!(runtime.eval_text("(cond-> 1 (= 1 2) inc)").unwrap(), "1");
        assert_eq!(runtime.eval_text("(cond->> 1 (= 1 1) inc)").unwrap(), "2");
        assert_eq!(runtime.eval_text("(cond->> 1 (= 1 2) inc)").unwrap(), "1");
        assert_eq!(
            runtime.eval_text("(vec (drop 2 [1 2 3 4]))").unwrap(),
            "[3 4]"
        );
    }

    #[test]
    fn hara_file_operations_use_capability_providers() {
        let mut runtime = Runtime::new();
        assert!(runtime
            .eval_text("(file/read \"/sandbox/data.bin\")")
            .unwrap_err()
            .contains("unsupported or file access is denied"));

        runtime.install_memory_file_provider("/sandbox");
        assert_eq!(
            runtime
                .eval_text("(file/resolve \"/sandbox\" \"data.bin\")")
                .unwrap(),
            "\"/sandbox/data.bin\""
        );
        assert_eq!(
            runtime
                .eval_text("(deref (file/write \"/sandbox/data.bin\" (bytes 0 127 255)))")
                .unwrap(),
            "nil"
        );
        assert_eq!(
            runtime
                .eval_text("(deref (file/read \"/sandbox/data.bin\"))")
                .unwrap(),
            "#bytes[0 127 -1]"
        );
        assert!(runtime
            .eval_text("(file/resolve \"/sandbox\" \"../escape\")")
            .unwrap_err()
            .contains("file/denied"));
        assert_eq!(
            runtime
                .eval_text("(deref (file/exists? \"/sandbox/data.bin\"))")
                .unwrap(),
            "true"
        );
        assert_eq!(
            runtime
                .eval_text("(deref (file/exists? \"/sandbox/missing.bin\"))")
                .unwrap(),
            "false"
        );
        assert_eq!(
            runtime
                .eval_text("(deref (file/write \"/sandbox/list/a.bin\" (bytes 1)))")
                .unwrap(),
            "nil"
        );
        assert_eq!(
            runtime
                .eval_text("(deref (file/write \"/sandbox/list/b.bin\" (bytes 2)))")
                .unwrap(),
            "nil"
        );
        assert_eq!(
            runtime
                .eval_text("(count (deref (file/list \"/sandbox/list\")))")
                .unwrap(),
            "2"
        );
        assert_eq!(
            runtime
                .eval_text("(deref (file/delete \"/sandbox/data.bin\"))")
                .unwrap(),
            "nil"
        );
        assert_eq!(
            runtime
                .eval_text("(deref (file/exists? \"/sandbox/data.bin\"))")
                .unwrap(),
            "false"
        );
    }

    #[test]
    fn hara_socket_operations_use_callback_providers() {
        let mut runtime = Runtime::new();
        assert!(runtime
            .eval_text("(socket/connect \"localhost\" 8080 {} (fn [error socket] socket))")
            .unwrap_err()
            .contains("unsupported or network access is denied"));

        runtime.install_loopback_socket_provider();
        assert_eq!(
            runtime
                .eval_text("(def socket-handle (socket/connect \"localhost\" 8080 {} (fn [error socket] socket)))")
                .unwrap(),
            "1"
        );
        assert_eq!(
            runtime
                .eval_text("(socket/send socket-handle (bytes 0 127 255))")
                .unwrap(),
            "3"
        );
        assert_eq!(
            runtime.eval_text("(socket/close socket-handle)").unwrap(),
            "nil"
        );
        assert!(runtime
            .eval_text("(socket/send socket-handle (bytes 1))")
            .unwrap_err()
            .contains("socket/invalid"));
    }

    #[test]
    fn provider_registry_reports_installed_capabilities() {
        let mut registry = core::ProviderRegistry::new();
        assert_eq!(
            registry.capabilities(),
            core::ProviderCapabilities {
                file: false,
                socket: false
            }
        );
        registry.install_file(core::MemoryFileProvider::new("/sandbox"));
        registry.install_socket(core::LoopbackSocketProvider::default());
        assert_eq!(
            registry.capabilities(),
            core::ProviderCapabilities {
                file: true,
                socket: true
            }
        );
        assert!(registry.file().is_some());
        assert!(registry.socket().is_some());
    }

    #[test]
    fn runtime_routes_socket_handles_through_callback_provider() {
        let mut runtime = Runtime::new();
        runtime.install_loopback_socket_provider();
        let socket = runtime.socket_connect("localhost", 8080).unwrap();
        assert_eq!(runtime.socket_send(socket, vec![1, 2, 3]).unwrap(), 3);
        runtime.socket_close(socket).unwrap();
    }

    #[test]
    fn loopback_socket_is_callback_based_and_counts_bytes() {
        use crate::core::SocketProvider;
        SOCKET_EVENTS.store(0, std::sync::atomic::Ordering::SeqCst);
        let sockets = core::LoopbackSocketProvider::default();
        let handle = sockets
            .connect("localhost", 8080, Rc::new(count_socket_event))
            .unwrap();
        assert_eq!(sockets.send(handle, &[1, 2, 3]).unwrap(), 3);
        sockets.close(handle).unwrap();
        assert_eq!(SOCKET_EVENTS.load(std::sync::atomic::Ordering::SeqCst), 3);
        assert_eq!(
            sockets.send(handle, &[9]).unwrap_err(),
            core::SocketError::Invalid("unknown socket".into())
        );
    }

    #[test]
    fn memory_file_provider_enforces_root_and_preserves_bytes() {
        use crate::core::FileProvider;
        let files = core::MemoryFileProvider::new("/sandbox");
        assert_eq!(
            files.resolve("/sandbox", "docs/../secret").unwrap_err(),
            core::FileError::Denied
        );
        let path = files.resolve("/sandbox", "data.bin").unwrap();
        let write = files.write(&path, vec![0, 127, 255]).unwrap();
        assert_eq!(
            write.state(),
            core::PromiseState::Fulfilled(core::Value::Nil)
        );
        let read = files.read(&path).unwrap();
        assert_eq!(
            read.state(),
            core::PromiseState::Fulfilled(core::Value::Bytes(vec![0, 127, 255]))
        );
        assert_eq!(
            files.read("/outside/data.bin").unwrap_err(),
            core::FileError::Denied
        );
    }

    #[test]
    fn unsupported_capabilities_fail_stably() {
        use crate::core::{FileProvider, SocketProvider};
        let files = core::UnsupportedFileProvider;
        assert_eq!(
            files.resolve("/root", "data.bin").unwrap_err(),
            core::FileError::Unsupported
        );
        assert_eq!(
            files.read("data.bin").unwrap_err(),
            core::FileError::Unsupported
        );
        let sockets = core::UnsupportedSocketProvider;
        assert_eq!(
            sockets
                .connect("localhost", 80, Rc::new(ignore_socket_event))
                .unwrap_err(),
            core::SocketError::Unsupported
        );
        assert_eq!(
            sockets.send(1, &[1, 2]).unwrap_err(),
            core::SocketError::Unsupported
        );
        assert_eq!(
            sockets.close(1).unwrap_err(),
            core::SocketError::Unsupported
        );
    }

    #[test]
    fn namespace_aliases_route_evaluation_and_resources() {
        let mut runtime = Runtime::new();
        assert!(runtime.create_namespace("hara.math"));
        assert!(runtime.alias_namespace("math", "hara.math"));
        assert_eq!(runtime.resolve_namespace("math"), "hara.math");
        assert_eq!(
            runtime
                .eval_in_namespace("math", "(defn answer [] 42) (answer)")
                .unwrap(),
            "42"
        );
        runtime.register_resource("helpers", "(defn helper [] 7) (helper)");
        assert_eq!(
            runtime
                .require_resource_in_namespace("helpers", "math")
                .unwrap(),
            "7"
        );
        assert_eq!(runtime.eval_text("(helper)").unwrap(), "7");
    }

    #[test]
    fn foundation_host_routes_calls_to_the_native_host_type() {
        let mut runtime = Runtime::new();
        let error = runtime
            .eval_text(
                "(ns user (:require [std.foundation.host :as host])) (deref (host/call \"browser.dom\" \"set-text\" \"#sel\" \"hi\"))",
            )
            .unwrap_err();
        assert!(
            error.contains("host/unavailable"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn host_modules_route_through_the_foundation_wrapper() {
        let mut runtime = Runtime::new();
        runtime.register_resource(
            "host.browser.dom",
            "(ns host.browser.dom (:require [std.foundation.host :as host])) (defn set-text [selector text] (host/call \"browser.dom\" \"set-text\" selector text))",
        );
        let error = runtime
            .eval_text(
                "(ns user (:require [host.browser.dom :as dom])) (deref (dom/set-text \"#sel\" \"hi\"))",
            )
            .unwrap_err();
        assert!(
            error.contains("host/unavailable"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn namespace_registry_owns_qualified_vars_without_changing_identity() {
        let mut runtime = Runtime::new();
        runtime.use_namespace("alpha");
        runtime
            .eval_text("(def ^{:dynamic true} answer 41)")
            .unwrap();
        let local = match runtime.env.get("answer").unwrap() {
            core::Value::Var(var) => var.clone(),
            _ => panic!("definition must be a Var"),
        };
        assert_eq!(local.symbol().as_str(), "alpha/answer");
        let qualified = match runtime.env.get("alpha/answer").unwrap() {
            core::Value::Var(var) => var.clone(),
            _ => panic!("qualified definition must be a Var"),
        };
        assert!(local.same_identity(&qualified));
        assert!(qualified.is_dynamic());
        runtime.use_namespace("user");
        runtime.alias_namespace("a", "alpha");
        let alias = match runtime.env.get("a/answer").unwrap() {
            core::Value::Var(var) => var.clone(),
            _ => panic!("alias must resolve to a Var"),
        };
        assert!(local.same_identity(&alias));
    }

    #[test]
    fn qualified_namespace_symbols_resolve_shared_vars_and_aliases() {
        let mut runtime = Runtime::new();
        assert!(runtime.create_namespace("alpha"));
        assert_eq!(
            runtime
                .eval_in_namespace("alpha", "(def answer 41)")
                .unwrap(),
            "41"
        );
        runtime.use_namespace("user");
        assert_eq!(runtime.eval_text("alpha/answer").unwrap(), "41");
        assert!(runtime.alias_namespace("a", "alpha"));
        assert_eq!(runtime.eval_text("a/answer").unwrap(), "41");
        assert_eq!(
            runtime
                .eval_text("(do (set! alpha/answer 42) alpha/answer)")
                .unwrap(),
            "42"
        );
        runtime.use_namespace("alpha");
        assert_eq!(runtime.eval_text("answer").unwrap(), "42");
    }

    #[test]
    fn namespaces_isolate_bindings_and_can_be_selected() {
        let mut runtime = Runtime::new();
        assert_eq!(runtime.current_namespace(), "user");
        assert!(runtime.create_namespace("math"));
        runtime.eval_text("(defn answer [] 42)").unwrap();
        runtime.use_namespace("math");
        assert_eq!(
            runtime.eval_text("(defn answer [] 7) (answer)").unwrap(),
            "7"
        );
        runtime.use_namespace("user");
        assert_eq!(runtime.eval_text("(answer)").unwrap(), "42");
        runtime.use_namespace("math");
        assert_eq!(runtime.eval_text("(answer)").unwrap(), "7");
    }

    #[test]
    fn generated_namespaces_configure_aliases_refers_and_intrinsics_without_sources() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime.eval_text("(str/trim \"  hara  \")").unwrap(),
            "\"hara\""
        );
        assert_eq!(
            runtime
                .eval_text(
                    "(ns app (:intrinsics {:exclude [bytes] :aliases {string text}})                       (:require [hara.lib.string :as s :refer [trim]]))                       (trim (s/trim (text/upper \" x \")))"
                )
                .unwrap(),
            "\"X\""
        );
        assert!(runtime
            .eval_text("(bytes/count (bytes 1))")
            .unwrap_err()
            .contains("bytes/count"));
        assert_eq!(
            runtime
                .eval_text("(ns core-user (:require [hara.lib.core :as core])) (core/bit-not 0)")
                .unwrap(),
            "-1"
        );
    }

    #[test]
    fn generated_namespace_require_never_falls_back_to_registered_source() {
        let mut runtime = Runtime::new();
        runtime.register_resource("std.foundation.string", "(def poisoned 42)");
        assert_eq!(
            runtime
                .eval_text("(ns app (:require [hara.lib.string :as text])) (text/trim \" x \")")
                .unwrap(),
            "\"x\""
        );
        assert!(runtime
            .eval_text("poisoned")
            .unwrap_err()
            .contains("unbound symbol"));
    }

    #[test]
    fn strict_json_and_pretty_libraries_match_the_portable_contract() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text(
                    "(std.foundation.json/read \"[null,true,-2,\\\"x\\\",[3],{\\\"a\\\":4}]\")"
                )
                .unwrap(),
            "[nil true -2 \"x\" [3] {\"a\" 4}]"
        );
        assert_eq!(
            runtime
                .eval_text("(std.foundation.json/write {\"a\" 1 \"b\" [true nil]})")
                .unwrap(),
            "\"{\\\"a\\\":1,\\\"b\\\":[true,null]}\""
        );
        assert_eq!(
            runtime.eval_text("(Json/write {\"a\" 1})").unwrap(),
            "\"{\\\"a\\\":1}\""
        );
        assert_eq!(
            runtime
                .eval_text("(std.foundation.json/pretty {\"a\" 1} {})")
                .unwrap(),
            "\"{\\n  \\\"a\\\": 1\\n}\""
        );
        assert!(runtime
            .eval_text("(std.foundation.json/pretty {\"a\" 1} nil)")
            .unwrap_err()
            .contains("options map"));
        assert!(runtime
            .eval_text("(std.foundation.json/read \"1.5\")")
            .unwrap_err()
            .contains("signed 64-bit integers"));
        assert_eq!(
            runtime
                .eval_text("(do (require 'std.pretty) (std.pretty/pprint-str {:a [1 2]}))")
                .unwrap(),
            "\"{:a [1 2]}\""
        );
    }

    #[test]
    fn restricted_edn_library_reads_and_writes_without_evaluation() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text(
                    "(do (require 'std.foundation.edn) \
                     (std.foundation.edn/read \"{:a [1 2] :b #{:x}}\"))"
                )
                .unwrap(),
            "{:a [1 2] :b #{:x}}"
        );
        assert_eq!(
            runtime
                .eval_text(
                    "(do (require 'std.foundation.edn) \
                     [(std.foundation.edn/write {:a [1 2]}) \
                      (std.foundation.edn/pretty [:a 1] {})])"
                )
                .unwrap(),
            "[\"{:a [1 2]}\" \"[:a 1]\"]"
        );
        assert!(runtime
            .eval_text(
                "(do (require 'std.foundation.edn) \
                 (std.foundation.edn/pretty [:a 1] nil))"
            )
            .unwrap_err()
            .contains("options map"));
        assert_eq!(
            runtime
                .eval_text(
                    "(do (require 'std.foundation.edn) \
                     (std.foundation.edn/read \"(+ 1 2)\"))"
                )
                .unwrap(),
            "(+ 1 2)"
        );
        assert_eq!(
            runtime
                .eval_text(
                    "(try \
                       (throw (ex-info \"bad input\" {:kind :invalid})) \
                       (catch Throwable error \
                         [(ex-message error) (ex-data error)]))"
                )
                .unwrap(),
            "[\"bad input\" {:kind :invalid}]"
        );
        assert_eq!(
            runtime
                .eval_text(
                    "(IExInfo/data \
                       (ex-info \"bad input\" {:kind :invalid}))"
                )
                .unwrap(),
            "{:kind :invalid}"
        );
        for source in ["1/2", "1N", "1M", "1 2"] {
            let escaped = source.replace('\\', "\\\\").replace('"', "\\\"");
            assert!(runtime
                .eval_text(&format!(
                    "(do (require 'std.foundation.edn) \
                     (std.foundation.edn/read \"{escaped}\"))"
                ))
                .is_err());
        }
    }

    #[test]
    fn resource_sources_accept_namespace_declarations() {
        let mut runtime = Runtime::new();
        runtime.register_resource(
            "module",
            "(ns demo (:require [core])) (defn answer [] 42) (answer)",
        );
        assert_eq!(runtime.load_resource("module").unwrap(), "42");
    }

    #[test]
    fn substrate_protocol_resource_loads_in_the_native_runtime() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text("(require 'std.lib.substrate.protocol) :loaded")
                .unwrap(),
            ":loaded"
        );
    }

    #[test]
    fn guest_struct_protocols_dispatch_like_truffle() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text(
                    "(do (defstruct Box [value]) \
                     (defprotocol BoxOps (read [self]) (add [self amount])) \
                     (extend-type Box BoxOps \
                       (read [self] (field self :value)) \
                       (add [self amount] (+ (field self :value) amount))) \
                     [(read (Box 40)) \
                      (add (map->Box {:value 40}) 2) \
                      (user/read (Box 41)) \
                      (instance? Box (Box 1))])",
                )
                .unwrap(),
            "[40 42 41 true]"
        );
        assert!(runtime
            .eval_text(
                "(do (ns protocol-probe (:config {:blank true}) (:require [std.foundation :refer :all :exclude [get]])) (defstruct Missing []) (defprotocol Needed (get [self])) \
                     (get (Missing)))",
            )
            .unwrap_err()
            .contains("missing protocol implementation: protocol-probe/Needed/get"));
    }

    #[test]
    fn guest_protocol_methods_reload_and_reject_collisions_atomically() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text(
                    "(do (defstruct Box [value]) \
                     (defprotocol BoxOps (read [self])) \
                     (extend-type Box BoxOps (read [self] (field self :value))) \
                     [(read (Box 41)) (user/read (Box 42))])",
                )
                .unwrap(),
            "[41 42]"
        );
        assert_eq!(
            runtime
                .eval_text("(defprotocol BoxOps (read [self]))")
                .unwrap(),
            "#protocol[user/BoxOps]"
        );
        let collision = runtime
            .eval_text(
                "(do (def ordinary 1) \
                 (defprotocol Broken (fresh [self]) (ordinary [self])))",
            )
            .unwrap_err();
        assert!(collision.contains("Protocol method Var already exists"));
        assert_eq!(runtime.eval_text("ordinary").unwrap(), "1");
        assert!(runtime.eval_text("fresh").is_err());
        assert!(runtime.eval_text("Broken").is_err());
        assert!(runtime
            .eval_text("(protocol-call BoxOps read (Box 1))")
            .is_err());
        assert!(runtime.eval_text("(BoxOps/read (Box 1))").is_err());
    }

    #[test]
    fn required_guest_protocol_methods_are_called_through_namespace_aliases() {
        let mut runtime = Runtime::new();
        runtime.register_resource(
            "acme.box",
            "(ns acme.box) \
             (defstruct Box [value]) \
             (defprotocol BoxOps (read [self])) \
             (extend-type Box BoxOps (read [self] (field self :value)))",
        );
        assert_eq!(
            runtime
                .eval_text(
                    "(ns consumer (:require [acme.box :as box])) \
                     (box/read (acme.box/Box 42))"
                )
                .unwrap(),
            "42"
        );
    }

    #[test]
    fn foundation_protocols_are_canonical_and_method_names_reject_bangs() {
        let mut runtime = Runtime::new();
        let Some(contract) = repo_text("specs/00-unsorted/platform-language/draft/conformance/protocols.edn") else {
            return;
        };
        let fixture =
            include_str!("../../lib/test-fixtures/std/foundation/protocol_conformance.hal");
        assert_eq!(core::FOUNDATION_PROTOCOLS.len(), 53);
        assert_eq!(
            core::FOUNDATION_PROTOCOLS
                .iter()
                .map(|(_, methods)| methods.len())
                .sum::<usize>(),
            103
        );
        let foundation = runtime
            .namespace_registry
            .find("std.foundation")
            .expect("std.foundation namespace");
        for (name, methods) in core::FOUNDATION_PROTOCOLS {
            assert!(
                contract.contains(&format!(":name {name}")),
                "shared contract is missing {name}"
            );
            let namespace_name = core::builtin_protocol_namespace(name);
            let namespace = runtime
                .namespace_registry
                .find(&namespace_name)
                .unwrap_or_else(|| panic!("missing {namespace_name} namespace"));
            let protocol = namespace
                .resolve(&lang::data::Symbol::parse(name))
                .unwrap_or_else(|| panic!("missing {namespace_name}/{name}"))
                .deref_value();
            let core::Value::Protocol(descriptor) = &protocol else {
                panic!("{namespace_name}/{name} is not a protocol");
            };
            assert_eq!(descriptor.name, core::builtin_protocol_name(name));
            assert_eq!(descriptor.methods.len(), methods.len());
            assert!(descriptor
                .methods
                .keys()
                .all(|method| !method.ends_with('!')));
            assert_eq!(
                foundation
                    .resolve(&lang::data::Symbol::parse(name))
                    .unwrap_or_else(|| panic!("missing std.foundation/{name} alias"))
                    .deref_value(),
                protocol
            );
            for (method, _) in *methods {
                let canonical_method = namespace
                    .resolve(&lang::data::Symbol::parse(method))
                    .unwrap_or_else(|| panic!("missing {namespace_name}/{method}"))
                    .deref_value();
                let aliased_method = foundation
                    .resolve(&lang::data::Symbol::parse(&format!("{name}/{method}")))
                    .unwrap_or_else(|| panic!("missing global alias {name}/{method}"))
                    .deref_value();
                assert_eq!(aliased_method, canonical_method);
                assert!(
                    fixture.contains(&format!("({namespace_name}/{method} fixture")),
                    "shared fixture does not directly call {namespace_name}/{method}"
                );
            }
        }
        for protocol in [
            "IColl",
            "IMetadata",
            "IHasRuntime",
            "IRanged",
            "IValidate",
            "IComponentOptions",
            "IComponentProps",
            "IComponentQuery",
            "IComponentTrack",
        ] {
            let namespace = core::builtin_protocol_namespace(protocol);
            assert!(
                runtime
                    .eval_text(&format!("{namespace}/{protocol}"))
                    .unwrap_err()
                    .contains("unbound symbol"),
                "{namespace}/{protocol} must not be guest-visible"
            );
            assert!(
                runtime
                    .eval_text(&format!("std.foundation/{protocol}"))
                    .unwrap_err()
                    .contains("unbound symbol"),
                "std.foundation/{protocol} must not be guest-visible"
            );
        }
        assert_eq!(
            runtime
                .eval_text("(std.protocol.icount/count [1 2 3])")
                .unwrap(),
            "3"
        );
        assert_eq!(
            runtime
                .eval_text("(std.protocol.icas/cas (atom 1) 1 2)")
                .unwrap(),
            "true"
        );
        assert_eq!(
            runtime
                .eval_text(
                    "(std.protocol.ireduce/reduce \
                       [1 2 3] (fn [left right] (+ left right)) 0)",
                )
                .unwrap(),
            "6"
        );
        assert_eq!(
            runtime
                .eval_text("(std.protocol.ipromise/state (std.foundation.promise/from 7))")
                .unwrap(),
            ":fulfilled"
        );
        assert_eq!(
            runtime
                .eval_text("(require 'std.protocol.ifind) :loaded")
                .unwrap(),
            ":loaded"
        );
        assert_eq!(
            runtime
                .eval_text("(defprotocol PredicateProtocol (ready? [self]))")
                .unwrap(),
            "#protocol[user/PredicateProtocol]"
        );
        assert!(runtime
            .eval_text("(defprotocol MutatingProtocol (mutate! [self]))")
            .unwrap_err()
            .contains("protocol method names must not end with !"));
    }

    #[test]
    fn shared_foundation_protocol_conformance_fixture_runs_in_the_native_runtime() {
        let mut runtime = Runtime::new();
        let result = runtime
            .eval_text(include_str!(
                "../../lib/test-fixtures/std/foundation/protocol_conformance.hal"
            ))
            .unwrap();
        assert!(!result.contains(":pass false"), "{result}");
        assert_eq!(result.matches(":pass true").count(), 53, "{result}");
    }

    #[test]
    fn shared_foundation_protocol_functionality_fixture_runs_in_the_native_runtime() {
        let source =
            include_str!("../../lib/test-fixtures/std/foundation/protocol_functionality.hal");
        let Some(catalog) = repo_text("specs/00-unsorted/platform-language/draft/conformance/protocol-method-cases.edn")
        else {
            return;
        };
        assert_eq!(catalog.matches("{:protocol ").count(), 88);
        let mut runtime = Runtime::new();
        let result = runtime.eval_text(source).unwrap();
        assert!(!result.contains(":pass false"), "{result}");
        assert_eq!(result.matches(":pass true").count(), 88, "{result}");

        let method_vars = source
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                let start = line.find("(protocol-case ")?;
                line[(start + "(protocol-case ".len())..]
                    .split_whitespace()
                    .nth(2)
            })
            .collect::<Vec<_>>();
        assert_eq!(method_vars.len(), 88);
        for method_var in method_vars {
            let mut segments = method_var.split(['.', '/']);
            let protocol_namespace = segments.nth(2).expect("protocol namespace");
            let method = segments.next().expect("protocol method");
            assert!(
                catalog.contains(&format!(":method {method} ")),
                "case catalog is missing {protocol_namespace}/{method}"
            );
            let error = runtime.eval_text(&format!("({method_var})")).unwrap_err();
            assert!(
                error.contains("protocol/arity"),
                "{method_var} returned an uncategorized arity error: {error}"
            );
        }

        let failure_forms = source
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                let start = line.find("'(std.protocol.")? + 1;
                let form = &line[start..];
                let mut depth = 0_usize;
                for (index, character) in form.char_indices() {
                    match character {
                        '(' => depth += 1,
                        ')' => {
                            depth -= 1;
                            if depth == 0 {
                                return Some(form[..=index].to_owned());
                            }
                        }
                        _ => {}
                    }
                }
                None
            })
            .collect::<Vec<_>>();
        assert_eq!(failure_forms.len(), 88);
        for failure_form in failure_forms {
            let call = failure_form.replacen("unsupported", "(UnsupportedUseCase)", 1);
            let error = runtime.eval_text(&call).unwrap_err();
            assert!(
                error.contains("protocol/unsupported-receiver"),
                "{call} returned an uncategorized dispatch error: {error}"
            );
        }

        assert_eq!(
            runtime
                .eval_text(
                    "(try (std.protocol.icount/count) false \
                       (catch Throwable error true))"
                )
                .unwrap(),
            "true"
        );
    }

    #[test]
    fn foundation_iterator_protocols_traverse_and_close_native_iterators() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text(
                    "(let [it (IIter/iter [1 2])] \
                       [(IIterator/iter-next? it) \
                        (IIterator/iter-next it) \
                        (IIterator/iter-next it) \
                        (IIterator/iter-next? it) \
                        (IClose/close it)])"
                )
                .unwrap(),
            "[true 1 2 false nil]"
        );
    }

    #[test]
    fn foundation_state_protocols_dispatch_and_watch_keys_come_first() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text(
                    "(let [a (atom 1) seen (atom nil)] \
                       (IWatch/watch-add a :log \
                         (fn [key ref old new] \
                           (IReset/reset seen [key old new]))) \
                       (IReset/reset a 2) \
                       [(IDeref/deref a) \
                        (IDeref/deref seen)])"
                )
                .unwrap(),
            "[2 [:log 1 2]]"
        );
    }

    #[test]
    fn shared_protocol_conformance_fixture_runs_in_the_native_runtime() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text(include_str!(
                    "../../lib/test-fixtures/std/lib/substrate/protocol_conformance.hal"
                ))
                .unwrap(),
            "[40 42]"
        );
    }

    #[test]
    fn shared_substrate_frame_conformance_fixture_runs_in_the_native_runtime() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text(include_str!("../../lib/test-fixtures/std/lib/substrate/frame_conformance.hal"))
                .unwrap(),
            "\"{\\\"version\\\":\\\"substrate.v1\\\",\\\"kind\\\":\\\"request\\\",\\\"id\\\":\\\"req-1\\\",\\\"source\\\":\\\"client/a\\\",\\\"target\\\":\\\"server/b\\\",\\\"space\\\":\\\"workspace/main\\\",\\\"meta\\\":{\\\"trace\\\":\\\"trace-1\\\"},\\\"action\\\":\\\"math/add\\\",\\\"args\\\":[19,23],\\\"reply_to\\\":null,\\\"status\\\":null,\\\"data\\\":null,\\\"error\\\":null,\\\"signal\\\":null,\\\"cause\\\":null}\""
        );
        assert!(runtime
            .eval_text(
                "(do (require 'std.lib.substrate.frame) \\
                     (std.lib.substrate.frame/normalize-frame {:kind :unknown :id \"evt-1\"}))",
            )
            .is_err());
    }

    #[test]
    fn shared_substrate_node_lifecycle_fixture_runs_in_the_native_runtime() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text(include_str!(
                    "../../lib/test-fixtures/std/lib/substrate/node_lifecycle_conformance.hal"
                ))
                .unwrap(),
            "[84 42 :rejected]"
        );
    }

    #[test]
    fn atom_backed_substrate_capabilities_work_in_the_native_runtime() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text(
                    "(require 'std.lib.substrate) \
                     (def node (std.lib.substrate/node-create \"node-1\")) \
                     [(std.lib.substrate.protocol/set-service node \"cache\" 42) \
                      (std.lib.substrate.protocol/get-service node \"cache\") \
                      (std.lib.substrate.protocol/set-space-state node \"main\" {:count 1}) \
                      (std.lib.substrate.protocol/get-space-state node \"main\") \
                      (def subscription (std.lib.substrate.protocol/subscribe node \"main\" \"changed\" \"sub-1\" {})) \
                      (std.lib.substrate.protocol/receive-frame node subscription {:transport-id \"peer-a\"}) \
                      (std.lib.substrate.protocol/list-subscriptions node \"main\" \"changed\")]",
                )
                .unwrap(),
            "[42 42 {:count 1} {:count 1} #std.lib.substrate/SubstrateFrame{:id \"sub-1\" :kind :subscribe :space \"main\" :meta {} :action nil :args [] :reply-to nil :status nil :data nil :error nil :signal \"changed\" :cause nil} {\"peer-a\" {:id \"sub-1\" :meta {}}} [\"peer-a\"]]"
        );
    }

    #[test]
    fn substrate_routes_streams_and_settles_transport_requests() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text(
                    "(require 'std.lib.substrate) \
                     (def node (std.lib.substrate/node-create \"node-1\")) \
                     (std.lib.substrate.protocol/attach-transport node \"peer-a\" \
                       (fn [frame] \
                         (std.lib.substrate.protocol/set-service node \"sent\" \
                           (std.lib.substrate.protocol/frame-data frame)))) \
                     (def subscription (std.lib.substrate.protocol/subscribe node \"main\" \"changed\" \"sub-1\" {})) \
                     (std.lib.substrate.protocol/receive-frame node subscription {:transport-id \"peer-a\"}) \
                     (std.lib.substrate.protocol/publish node \"main\" \"changed\" 42 {:id \"evt-1\"}) \
                     (std.lib.substrate.protocol/get-service node \"sent\")",
                )
                .unwrap(),
            "42"
        );

        assert_eq!(
            runtime
                .eval_text(
                    "(def requester (std.lib.substrate/node-create \"node-2\")) \
                     (std.lib.substrate.protocol/attach-transport requester \"peer-b\" \
                       (fn [frame] \
                         (std.lib.substrate.protocol/receive-frame requester \
                           (std.lib.substrate/node-frame :response \"res-1\" \"main\" {} nil [] \
                             (std.lib.substrate.protocol/frame-id frame) :ok 84 nil nil nil) \
                           {:transport-id \"peer-b\"}))) \
                     (def reply (std.lib.substrate.protocol/request requester \"main\" \"sum\" [] \
                                  {:id \"req-1\" :transport-id \"peer-b\"})) \
                     (promise/value reply)",
                )
                .unwrap(),
            "84"
        );
    }

    #[test]
    fn substrate_cancellation_and_rejection_settle_pending_promises() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text(
                    "(require 'std.lib.substrate) \
                     (def node (std.lib.substrate/node-create \"node-1\")) \
                     (std.lib.substrate.protocol/attach-transport node \"peer-a\" (fn [frame] nil)) \
                     (def cancelled (std.lib.substrate.protocol/request node \"main\" \"wait\" [] \
                                      {:id \"req-cancel\" :transport-id \"peer-a\"})) \
                     (std.lib.substrate.protocol/cancel-request node \"req-cancel\" :cancelled) \
                     (promise/state cancelled)",
                )
                .unwrap(),
            ":rejected"
        );
    }

    #[test]
    fn registered_resources_load_into_the_runtime_environment() {
        let mut runtime = Runtime::new();
        runtime.register_resource("demo", "(defn answer [] 42) (answer)");
        assert_eq!(runtime.load_resource("demo").unwrap(), "42");
        assert_eq!(runtime.eval_text("(answer)").unwrap(), "42");
        assert_eq!(runtime.require_resource("demo").unwrap(), "42");
        assert_eq!(runtime.require_resource("demo").unwrap(), ":loaded");
    }

    #[test]
    fn vector_literals_are_values() {
        let mut runtime = Runtime::new();
        assert_eq!(runtime.eval_text("[1 2 3]").unwrap(), "[1 2 3]");
    }

    #[test]
    fn set_literals_reject_duplicate_items() {
        let mut runtime = Runtime::new();
        assert!(runtime
            .eval_text("#{1 (+ 1 1) 1}")
            .unwrap_err()
            .contains("Duplicate item"));
        assert!(runtime
            .eval_text("(count #{1 2 2})")
            .unwrap_err()
            .contains("Duplicate item"));
        assert_eq!(runtime.eval_text("(has? #{1 2} 2)").unwrap(), "true");
        assert_eq!(runtime.eval_text("(conj #{1} 2)").unwrap(), "#{1 2}");
        assert_eq!(runtime.eval_text("(= (set 1 2 1) #{1 2})").unwrap(), "true");
        assert_eq!(runtime.eval_text("(= #{1 2} #{2 1})").unwrap(), "true");
        assert_eq!(runtime.eval_text("(get #{1 2} 2 :missing)").unwrap(), "2");
    }

    #[test]
    fn syntax_quote_matches_java_expansion_semantics() {
        let mut runtime = Runtime::new();
        assert_eq!(runtime.eval_text("`foo").unwrap(), "foo");
        assert_eq!(
            runtime.eval_text("`(a ~(+ 1 2) ~@[4 5])").unwrap(),
            "(a 3 4 5)"
        );
        assert_eq!(runtime.eval_text("`[a ~(+ 1 2)]").unwrap(), "[a 3]");
        assert_eq!(runtime.eval_text("`{:a ~(+ 1 2)}").unwrap(), "{:a 3}");
        assert_eq!(
            runtime.eval_text("`(a (unquote))").unwrap_err(),
            "unquote expects one argument"
        );
        assert_eq!(
            runtime.eval_text("`(a ~@1)").unwrap_err(),
            "iter expects a collection"
        );
    }

    #[test]
    fn fn_star_and_eval_forms_execute_while_hash_dispatch_extensions_are_rejected() {
        let mut runtime = Runtime::new();
        assert_eq!(runtime.eval_text("((fn* [x] (+ x 1)) 4)").unwrap(), "5");
        assert!(runtime
            .eval_text("#=(+ 2 3)")
            .unwrap_err()
            .contains("No dispatch macro for: ="));
        assert!(runtime
            .eval_text("#[(def x 4) (+ x 2)]")
            .unwrap_err()
            .contains("No dispatch macro for: ["));
        assert!(runtime
            .eval_text("(eval)")
            .unwrap_err()
            .contains("one form"));
    }

    #[test]
    fn runtime_readable_strings_escape_and_round_trip() {
        let mut runtime = Runtime::new();
        let sources = [
            r#""quote: \" slash: \\ newline: \n tab: \t""#,
            r#"{:text "line\nvalue" :nested ["a\tb" "c\\d"]}"#,
            r#"["\u0000" "unicode λ"]"#,
            r#"#"a\"b""#,
        ];
        for source in sources {
            let readable = runtime.eval_text(source).unwrap();
            assert_eq!(
                kernel::parse(&readable).unwrap(),
                kernel::parse(source).unwrap()
            );
        }
    }

    #[test]
    fn reader_literals_are_first_class_runtime_values() {
        let mut runtime = Runtime::new();
        let cases = [
            ("1.5", "1.5"),
            ("\\newline", "\\newline"),
            ("#\"a+\"", "#\"a+\""),
            ("#demo {:a 1}", "#demo{:a 1}"),
            ("##Inf", "##Inf"),
            ("##-Inf", "##-Inf"),
            ("##NaN", "##NaN"),
        ];
        for (source, expected) in cases {
            assert_eq!(runtime.eval_text(source).unwrap(), expected, "{source}");
        }
        for unsupported in ["123N", "1.20M", "9223372036854775808"] {
            assert!(runtime.eval_text(unsupported).is_err(), "{unsupported}");
        }
        assert_eq!(runtime.eval_text("(= ##NaN ##NaN)").unwrap(), "true");
        assert_eq!(runtime.eval_text("'#demo [1 2]").unwrap(), "#demo[1 2]");
    }

    #[test]
    fn basic_math_has_the_portable_root_surface_and_explicit_numeric_boundary() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text(
                    "[(= E 2.718281828459045) (= PI 3.141592653589793) \
                     (sin 0) (cos 0) (tan 0) (asin 0) (acos 1) (atan 0) \
                     (atan2 0 1) (sinh 0) (cosh 0) (tanh 0) \
                     (asinh 0) (acosh 1) (atanh 0) \
                     (floor 1.75) (ceil 1.25) (pow 2 3) (abs -3) \
                     (exp 0) (sqrt 9)]"
                )
                .unwrap(),
            "[true true 0 1 0 0 0 0 0 0 1 0 0 0 0 1 2 8 3 1 3]"
        );
        assert_eq!(runtime.eval_text("(= (sqrt -1) ##NaN)").unwrap(), "true");
        assert_eq!(runtime.eval_text("(sqrt (long 9.9))").unwrap(), "3");
        assert_eq!(runtime.eval_text("(sqrt (double 9))").unwrap(), "3");
        assert!(runtime
            .eval_text("(abs -9223372036854775808)")
            .unwrap_err()
            .contains("overflow"));
        assert_eq!(
            runtime
                .eval_text("[(= (asinh 1.0e300) ##Inf) (= (acosh 1.0e300) ##Inf)]")
                .unwrap(),
            "[false false]"
        );
        for source in ["(sin)", "(pow 2)", "(sqrt \"9\")"] {
            assert!(runtime.eval_text(source).is_err(), "{source}");
        }
    }

    #[test]
    fn closed_native_method_inventory_is_classified_and_callable() {
        fn entry<'a>(entries: &'a [(Form, Form)], key: &str) -> &'a Form {
            entries
                .iter()
                .find_map(|(candidate, value)| match candidate {
                    Form::Keyword(name) if name == key => Some(value),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("missing :{key}"))
        }
        fn symbols(value: &Form, label: &str) -> Vec<String> {
            let Form::Vector(values) = value else {
                panic!("{label} must be a vector")
            };
            values
                .iter()
                .map(|value| match value {
                    Form::Symbol(name) => name.clone(),
                    _ => panic!("{label} must contain symbols"),
                })
                .collect()
        }
        fn classified(value: Option<&Form>, all: &[String], label: &str) -> Vec<String> {
            match value {
                None => Vec::new(),
                Some(Form::Keyword(marker)) if marker == "all" => all.to_vec(),
                Some(value) => symbols(value, label),
            }
        }
        fn wrapper_source(path: &str) -> &'static str {
            EMBEDDED_HAL_RESOURCES
                .iter()
                .find_map(|(_, resource_path, source)| (*resource_path == path).then_some(*source))
                .unwrap_or_else(|| panic!("unknown wrapper source: {path}"))
        }

        let Some(contract_source) = repo_text("specs/00-unsorted/platform-language/draft/conformance/native.edn")
        else {
            return;
        };
        let contract = kernel::parse_forms(&contract_source).unwrap().remove(0);
        let Form::Map(contract) = contract else {
            panic!("native contract must be a map")
        };
        let Form::Map(inventory) = entry(&contract, "inventory") else {
            panic!(":inventory must be a map")
        };
        assert!(matches!(entry(inventory, "closed"), Form::Bool(true)));
        let Form::Vector(types) = entry(&contract, "types") else {
            panic!(":types must be a vector")
        };
        assert_eq!(
            entry(inventory, "type-count"),
            &Form::Number(types.len() as i64)
        );

        let mut specified = Vec::new();
        let mut direct_cases = Vec::new();
        for value in types {
            let Form::Map(native_type) = value else {
                panic!("native type entries must be maps")
            };
            let Form::Symbol(name) = entry(native_type, "name") else {
                panic!("native :name must be a symbol")
            };
            let methods = symbols(entry(native_type, "methods"), ":methods");
            let Form::Keyword(availability) = entry(native_type, "availability") else {
                panic!("native :availability must be a keyword")
            };
            assert!(
                ["implemented", "capability-gated"].contains(&availability.as_str()),
                "unsupported availability: {availability}"
            );
            let Form::Map(classification) = entry(native_type, "method-classification") else {
                panic!(":method-classification must be a map")
            };
            let hal_wrappers = classified(
                classification.iter().find_map(|(key, value)| {
                    matches!(key, Form::Keyword(name) if name == "hal-wrapper").then_some(value)
                }),
                &methods,
                ":hal-wrapper",
            );
            let primitives = classified(
                classification.iter().find_map(|(key, value)| {
                    matches!(key, Form::Keyword(name) if name == "foundation-primitive")
                        .then_some(value)
                }),
                &methods,
                ":foundation-primitive",
            );
            let native_only = classified(
                classification.iter().find_map(|(key, value)| {
                    matches!(key, Form::Keyword(name) if name == "native-only").then_some(value)
                }),
                &methods,
                ":native-only",
            );
            let mut exposed = hal_wrappers.clone();
            exposed.extend(primitives);
            exposed.extend(native_only);
            assert_eq!(
                exposed
                    .iter()
                    .collect::<std::collections::HashSet<_>>()
                    .len(),
                methods.len(),
                "{name} methods must have one Foundation exposure"
            );
            assert_eq!(
                methods.iter().collect::<std::collections::HashSet<_>>(),
                exposed.iter().collect::<std::collections::HashSet<_>>(),
                "{name} method classifications are incomplete"
            );
            if !hal_wrappers.is_empty() {
                let Form::String(path) = entry(native_type, "wrapper-source") else {
                    panic!("{name} HAL wrappers require :wrapper-source")
                };
                let source = wrapper_source(path);
                for method in &hal_wrappers {
                    assert!(
                        source.contains(&format!("{name}/{method}")),
                        "missing HAL wrapper for {name}/{method}"
                    );
                }
            }
            let mut type_cases = Vec::new();
            for method in &methods {
                let symbol = format!("{name}/{method}");
                type_cases.push(format!(
                    "(native-method-result '{symbol} \
                     (fn [] ({symbol} nil nil nil nil nil nil nil nil nil)))"
                ));
            }
            direct_cases.push((name.clone(), type_cases));
            specified.push((name.clone(), methods));
        }

        let runtime_inventory = core::NATIVE_TYPES
            .iter()
            .map(|(name, methods)| {
                (
                    (*name).to_owned(),
                    methods.iter().map(|method| (*method).to_owned()).collect(),
                )
            })
            .collect::<Vec<(String, Vec<String>)>>();
        assert_eq!(specified, runtime_inventory);
        assert_eq!(
            entry(inventory, "method-count"),
            &Form::Number(
                specified
                    .iter()
                    .map(|(_, methods)| methods.len())
                    .sum::<usize>() as i64
            )
        );

        for (type_name, type_cases) in &direct_cases {
            let mut runtime = Runtime::new();
            runtime
                .eval_text(include_str!(
                    "../../lib/test-fixtures/std/foundation/native_method_conformance.hal"
                ))
                .unwrap();
            for direct_case in type_cases {
                let result = runtime.eval_text(direct_case).unwrap();
                assert!(
                    result.contains(":pass true"),
                    "{direct_case} returned {result}"
                );
            }
            assert!(
                !type_cases.is_empty(),
                "{type_name} has no conformance cases"
            );
        }
        assert_eq!(
            direct_cases
                .iter()
                .map(|(_, type_cases)| type_cases.len())
                .sum::<usize>(),
            specified
                .iter()
                .map(|(_, methods)| methods.len())
                .sum::<usize>()
        );
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text(
                    "[(Error/message \
                        (Error/new \"native failure\" {})) \
                      (string? (Error/class \
                        (Error/new \"native failure\" {}))) \
                      (Runtime/load-string \"(+ 19 23)\")]"
                )
                .unwrap(),
            "[\"native failure\" true 42]"
        );
    }

    #[test]
    fn native_types_are_descriptors_and_foundation_libraries_are_hal_wrappers() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text(
                    "[(str std.native.Maths) \
                      (INamespaced/name std.native.Maths) \
                      (INamespaced/namespace std.native.Maths) \
                      (= std.native.Maths (with-meta std.native.Maths {:doc \"math\"})) \
                      (Maths/sin 0) \
                      (String/upper \"hara\") \
                      (str/upper \"hara\") \
                      (Bytes/u8 -1) \
                      (bytes/u8 -1)]"
                )
                .unwrap(),
            "[\"#<native-type std.native.Maths>\" \"Maths\" \"std.native\" true 0 \"HARA\" \"HARA\" 255 255]"
        );
        assert!(runtime.eval_text("(std.native.Maths 1)").is_err());
        assert_eq!(
            runtime
                .eval_text("(ns legacy.activation (:config {:builtins [inc]}))")
                .unwrap(),
            "nil"
        );
    }

    #[test]
    fn startup_defaults_expose_edn_native_types_and_protocols() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text(
                    "(ns startup.defaults) \
                     [(edn/write {:a 1}) \
                      (= Maths std.native.Maths std.foundation/Maths) \
                      (= Edn std.native.Edn std.foundation/Edn) \
                      (= Json std.native.Json std.foundation/Json) \
                      (= Host std.native.Host std.foundation/Host) \
                      (= Arr std.native.Arr std.foundation/Arr) \
                      (= Obj std.native.Obj std.foundation/Obj) \
                      (let [arr (Arr/new 1 2)] \
                        (Arr/set-index arr 1 7) \
                        (Arr/get-index arr 1)) \
                      (let [obj (Obj/new \"a\" 1)] \
                        (Obj/set-key obj \"a\" 9) \
                        (Obj/get-key obj \"a\")) \
                      (ICount/count [1 2 3])]"
                )
                .unwrap(),
            "[\"{:a 1}\" true true true true true true 7 9 3]"
        );
        assert_eq!(
            runtime
                .eval_text(
                    "(ns blank.native (:config {:blank true})) \
                     [(= Iter std.native.Iter) \
                      (Iter/iter-next (Iter/iter-map (fn [value] value) [1]))]"
                )
                .unwrap(),
            "[true 1]"
        );
        let symbols = runtime.visible_symbols();
        assert!(symbols.iter().any(|symbol| symbol == "edn/pretty"));
        for native_type in [
            "Maths",
            "Numbers",
            "Bits",
            "String",
            "Bytes",
            "File",
            "Socket",
            "Promise",
            "Coroutine",
            "Arr",
            "Obj",
            "Runtime",
            "Printer",
            "Edn",
            "Json",
            "Host",
            "Regex",
            "UUID",
            "Error",
            "Iter",
            "Kernel",
        ] {
            assert!(
                symbols.iter().any(|symbol| symbol == native_type),
                "{native_type}"
            );
        }
    }

    #[test]
    fn strings_and_maps_are_values() {
        let mut runtime = Runtime::new();
        assert_eq!(runtime.eval_text("\"hello\"").unwrap(), "\"hello\"");
        assert_eq!(runtime.eval_text("{\"a\" 1}").unwrap(), "{\"a\" 1}");
    }

    #[test]
    fn application_and_pair_helpers_support_bootstrap_code() {
        let mut runtime = Runtime::new();
        assert_eq!(runtime.eval_text("(identity 42)").unwrap(), "42");
        assert_eq!(runtime.eval_text("(apply + [19 23])").unwrap(), "42");
        assert_eq!(runtime.eval_text("(apply + 19 [23])").unwrap(), "42");
        assert_eq!(runtime.eval_text("(key [1 2])").unwrap(), "1");
        assert_eq!(runtime.eval_text("(val [1 2])").unwrap(), "2");
        assert_eq!(runtime.eval_text("(reverse [1 2 3])").unwrap(), "(3 2 1)");
    }

    #[test]
    fn structural_hashes_are_stable_and_order_independent_for_maps_and_sets() {
        let mut runtime = Runtime::new();
        let _ = &mut runtime;
        let map_a = core::Value::Map(
            vec![
                (core::Value::Keyword("a".into()), core::Value::Number(1)),
                (core::Value::Keyword("b".into()), core::Value::Number(2)),
            ]
            .into_iter()
            .collect(),
        );
        let map_b = core::Value::Map(
            vec![
                (core::Value::Keyword("b".into()), core::Value::Number(2)),
                (core::Value::Keyword("a".into()), core::Value::Number(1)),
            ]
            .into_iter()
            .collect(),
        );
        let set_a = core::Value::Set(
            vec![
                core::Value::Number(1),
                core::Value::Number(2),
                core::Value::Number(3),
            ]
            .into(),
        );
        let set_b = core::Value::Set(
            vec![
                core::Value::Number(3),
                core::Value::Number(1),
                core::Value::Number(2),
            ]
            .into(),
        );
        assert_eq!(map_a.stable_hash(), map_b.stable_hash());
        assert_eq!(set_a.stable_hash(), set_b.stable_hash());
    }

    #[test]
    fn sequential_representations_share_java_equality_and_hash_semantics() {
        let values = vec![core::Value::Number(1), core::Value::Number(2)];
        let list = core::Value::List(values.clone().into());
        let tuple = core::Value::Tuple(Box::new(
            crate::lang::data::Tuple::from_values(values.clone()).unwrap(),
        ));
        let vector = core::Value::Vector(values.into());

        assert_eq!(list, tuple);
        assert_eq!(tuple, vector);
        assert_eq!(list.stable_hash(), tuple.stable_hash());
        assert_eq!(tuple.stable_hash(), vector.stable_hash());

        let mut runtime = Runtime::new();
        assert_eq!(runtime.eval_text("(= [1 2] '(1 2))").unwrap(), "true");
        assert_eq!(runtime.eval_text("(= [1 2] (list 1 2))").unwrap(), "true");
        assert_eq!(runtime.eval_text("(conj (list 2) 1)").unwrap(), "(1 2)");
        assert_eq!(runtime.eval_text("(pair 1 2)").unwrap(), "[1 2]");
        assert_eq!(runtime.eval_text("(key (pair 1 2))").unwrap(), "1");
        assert_eq!(runtime.eval_text("(val (pair 1 2))").unwrap(), "2");
        assert_eq!(runtime.eval_text("(tup 1 2 3 4 5)").unwrap(), "[1 2 3 4 5]");
        assert!(runtime
            .eval_text("(tup 1 2 3 4 5 6)")
            .unwrap_err()
            .contains("at most 5"));
        assert_eq!(runtime.eval_text("(= [1 2] [1 2 3])").unwrap(), "false");
        assert_eq!(
            runtime.eval_text("(get {[1 2] :found} '(1 2))").unwrap(),
            ":found"
        );
        assert_eq!(
            runtime.eval_text("(get #{[1 2]} '(1 2) :missing)").unwrap(),
            "[1 2]"
        );
    }

    #[test]
    fn java_collection_families_are_first_class_runtime_values() {
        let mut runtime = Runtime::new();
        for source in [
            "(= (hash-map :a 1 :b 2) (ordered-map :b 2 :a 1))",
            "(= (hash-map :a 1 :b 2) (sorted-map :b 2 :a 1))",
            "(= (hash-set 1 2) (ordered-set 2 1))",
            "(= (hash-set 1 2) (sorted-set 2 1))",
            "(= (queue 1 2) [1 2])",
        ] {
            assert_eq!(runtime.eval_text(source).unwrap(), "true", "{source}");
        }
        assert_eq!(runtime.eval_text("(get (hash-map :a 1) :a)").unwrap(), "1");
        assert_eq!(
            runtime.eval_text("(get (ordered-map :a 1) :a)").unwrap(),
            "1"
        );
        assert_eq!(
            runtime.eval_text("(get (sorted-map :a 1) :a)").unwrap(),
            "1"
        );
        assert_eq!(
            runtime
                .eval_text("(get (trie \"alpha\" 7) \"alpha\")")
                .unwrap(),
            "7"
        );
        assert_eq!(
            runtime.eval_text("(keys (sorted-map :b 2 :a 1))").unwrap(),
            "[:a :b]"
        );
        assert_eq!(runtime.eval_text("(nth (queue 4 5 6) 1)").unwrap(), "5");
        assert_eq!(
            runtime.eval_text("(last (conj (queue 4 5) 6))").unwrap(),
            "6"
        );
        assert_eq!(
            runtime
                .eval_text("(count (dissoc (ordered-set 1 2) 1))")
                .unwrap(),
            "1"
        );
        assert_eq!(
            runtime
                .eval_text("(get (assoc (trie) \"x\" 9) \"x\")")
                .unwrap(),
            "9"
        );
        assert!(runtime
            .eval_text("(hash-map :a)")
            .unwrap_err()
            .contains("even number"));
        assert!(runtime
            .eval_text("(trie :a 1)")
            .unwrap_err()
            .contains("string keys"));
    }

    #[test]
    fn map_membership_keys_and_values_are_portable() {
        let mut runtime = Runtime::new();
        assert_eq!(runtime.eval_text(r#"(has? {"a" 1} "a")"#).unwrap(), "true");
        assert_eq!(runtime.eval_text("(has? [1 2] 1)").unwrap(), "true");
        assert_eq!(runtime.eval_text("(has? [1 2] 2)").unwrap(), "false");
        assert_eq!(
            runtime.eval_text(r#"(has? {"a" nil} "a")"#).unwrap(),
            "true"
        );
        assert_eq!(
            runtime.eval_text(r#"(keys {"a" 1 "b" 2})"#).unwrap(),
            "[\"a\" \"b\"]"
        );
        assert_eq!(
            runtime.eval_text(r#"(vals {"a" 1 "b" 2})"#).unwrap(),
            "[1 2]"
        );
    }

    #[test]
    fn core_collection_navigation_and_predicates_are_host_neutral() {
        let mut runtime = Runtime::new();
        assert_eq!(runtime.eval_text("(first [1 2 3])").unwrap(), "1");
        assert_eq!(
            runtime
                .eval_text("[(seq? (rest [1 2 3])) (vec (rest [1 2 3]))]")
                .unwrap(),
            "[true [2 3]]"
        );
        assert_eq!(runtime.eval_text("(last [1 2 3])").unwrap(), "3");
        assert_eq!(runtime.eval_text("(empty? [])").unwrap(), "true");
        assert_eq!(runtime.eval_text("(not false)").unwrap(), "true");
        assert_eq!(runtime.eval_text("(< 1 2 3)").unwrap(), "true");
        assert_eq!(runtime.eval_text("(>= 3 3)").unwrap(), "true");
        assert_eq!(runtime.eval_text("(mod 7 3)").unwrap(), "1");
    }

    #[test]
    fn atoms_match_java_identity_and_mutation_semantics() {
        let mut runtime = Runtime::new();
        assert_eq!(runtime.eval_text("(let [a (atom 1)] @a)").unwrap(), "1");
        assert_eq!(
            runtime
                .eval_text("(let [a (atom 1)] (do (reset! a 2) @a))")
                .unwrap(),
            "2"
        );
        assert_eq!(
            runtime
                .eval_text("(let [a (atom 1)] (do (swap! a (fn [x y] (+ x y)) 4) @a))")
                .unwrap(),
            "5"
        );
        assert_eq!(
            runtime
                .eval_text("(let [a (atom 1)] [(cas! a 1 2) @a])")
                .unwrap(),
            "[true 2]"
        );
        assert_eq!(
            runtime
                .eval_text("(let [a (atom 1)] [(cas! a 0 2) @a])")
                .unwrap(),
            "[false 1]"
        );
        assert_eq!(
            runtime.eval_text("(let [a (atom 1) b a] (= a b))").unwrap(),
            "true"
        );
        assert_eq!(runtime.eval_text("(= (atom 1) (atom 1))").unwrap(), "false");
        assert_eq!(
            runtime.eval_text("(let [a (atom 1) seen (atom nil)] (do (watch-add a :log (fn [key ref old new] (reset! seen [key @ref old new]))) (reset! a 2) @seen))").unwrap(),
            "[:log 2 1 2]"
        );
        assert_eq!(
            runtime.eval_text("(let [a (atom 1)] (do (watch-add a :log (fn [key ref old new] new)) (watch-add a :log (fn [key ref old new] old)) (count (watch-list a))))").unwrap(),
            "1"
        );
        assert_eq!(
            runtime.eval_text("(let [a (atom 1) seen (atom nil)] (do (watch-add a :log (fn [key ref old new] (reset! seen new))) (watch-remove a :log) (reset! a 2) @seen))").unwrap(),
            "nil"
        );
        assert!(runtime
            .eval_text("(watch-add (atom:basic 1) :log (fn [key ref old new] new))")
            .unwrap_err()
            .contains("watch-add"));
        assert!(runtime
            .eval_text("(reset! 1 2)")
            .unwrap_err()
            .contains("IReset/reset"));
        assert!(runtime
            .eval_text("(swap! (atom 1) 2)")
            .unwrap_err()
            .contains("expects a function"));
        for legacy in [
            "compare:set!",
            "compare-and-set!",
            "add-watch",
            "remove-watch",
            "get-watches",
        ] {
            assert!(
                runtime.eval_text(legacy).unwrap_err().contains("unbound"),
                "{legacy} should not remain public"
            );
        }
    }

    #[test]
    fn keywords_maps_and_sets_match_java_callable_semantics() {
        let mut runtime = Runtime::new();
        assert_eq!(runtime.eval_text("(:answer {:answer 42})").unwrap(), "42");
        assert_eq!(runtime.eval_text("(:missing {:answer 42})").unwrap(), "nil");
        assert_eq!(runtime.eval_text("(:missing nil 7)").unwrap(), "7");
        assert_eq!(runtime.eval_text("({:answer 42} :answer)").unwrap(), "42");
        assert_eq!(runtime.eval_text("({:answer 42} :missing 7)").unwrap(), "7");
        assert_eq!(
            runtime.eval_text("(#{:answer} :answer)").unwrap(),
            ":answer"
        );
        assert_eq!(runtime.eval_text("(#{:answer} :missing 7)").unwrap(), "7");
        assert_eq!(
            runtime.eval_text("(:answer)").unwrap_err(),
            "keyword invocation expects one or two arguments"
        );
        assert_eq!(
            runtime.eval_text("({} :a :b :c)").unwrap_err(),
            "map invocation expects one or two arguments"
        );
    }

    #[test]
    fn foundation_fallback_is_eager_canonical_and_shadowable() {
        let mut runtime = Runtime::new();
        let foundation = runtime
            .namespace_registry
            .find("std.foundation")
            .expect("foundation is bootstrapped");
        let canonical = foundation
            .resolve(&crate::lang::data::Symbol::parse("identity"))
            .expect("identity fallback is installed");
        assert_eq!(canonical.origin(), kernel::VarOrigin::HalFallback);
        let referred = runtime
            .namespace_registry
            .find("user")
            .unwrap()
            .resolve(&crate::lang::data::Symbol::parse("identity"))
            .unwrap();
        assert!(canonical.same_identity(&referred));
        assert_eq!(runtime.eval_text("(identity 42)").unwrap(), "42");
        assert_eq!(runtime.eval_text("(first (range 3))").unwrap(), "0");
        assert_eq!(runtime.eval_text("(first (range 2 5))").unwrap(), "2");

        assert_eq!(
            runtime
                .eval_text("(ns project.app (:config {:blank true}) (:require [std.foundation :refer :all :exclude [identity]])) (def identity (fn [value] 7)) (identity 42)")
                .unwrap(),
            "7"
        );
        let local = runtime
            .namespace_registry
            .find("project.app")
            .unwrap()
            .resolve(&crate::lang::data::Symbol::parse("identity"))
            .unwrap();
        assert!(!canonical.same_identity(&local));
        assert_eq!(
            runtime.eval_text("(std.foundation/identity 42)").unwrap(),
            "42"
        );
    }

    #[test]
    fn fallback_definitions_never_replace_rust_library_vars() {
        let mut runtime = Runtime::new();
        let foundation = runtime.namespace_registry.find_or_create("std.foundation");
        let native = foundation.intern_with_origin(
            "optimized",
            core::Value::Number(7),
            kernel::VarOrigin::RustLibrary,
        );
        let identity = native.identity_address();
        core::with_definition_origin(kernel::VarOrigin::HalFallback, || {
            runtime.eval_text(concat!(
                "(ns std.foundation)",
                " (defn ^{:schema [:fn [:int] :int]} optimized",
                " \"Documents the native implementation.\" [value] 9)"
            ))
        })
        .unwrap();
        let refreshed = foundation
            .resolve(&crate::lang::data::Symbol::parse("optimized"))
            .unwrap();
        assert_eq!(refreshed.identity_address(), identity);
        assert_eq!(refreshed.origin(), kernel::VarOrigin::RustLibrary);
        assert_eq!(refreshed.deref_value().display(), "7");
        assert_eq!(
            refreshed
                .hara_metadata()
                .and_then(|metadata| metadata.doc().map(str::to_owned)),
            Some("Documents the native implementation.".into())
        );
        let metadata = refreshed.hara_metadata().expect("fallback metadata");
        assert_eq!(
            metadata.get_keyword("arglists"),
            Some(&crate::lang::data::MetadataValue::Vector(vec![
                crate::lang::data::MetadataValue::Vector(vec![
                    crate::lang::data::MetadataValue::Symbol(crate::lang::data::Symbol::from(
                        "value"
                    ))
                ])
            ]))
        );
        assert_eq!(
            metadata.get_keyword("schema"),
            Some(&crate::lang::data::MetadataValue::Vector(vec![
                crate::lang::data::MetadataValue::Keyword(crate::lang::data::Keyword::from("fn")),
                crate::lang::data::MetadataValue::Vector(vec![
                    crate::lang::data::MetadataValue::Keyword(crate::lang::data::Keyword::from(
                        "int"
                    ))
                ]),
                crate::lang::data::MetadataValue::Keyword(crate::lang::data::Keyword::from("int"))
            ]))
        );
    }

    #[test]
    fn function_metadata_is_visible_through_meta_and_var_literals() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text(concat!(
                    "(defn ^{:schema [:fn [:int] :int]} documented",
                    " \"Returns its argument.\" [value] value)",
                    " (let [m (meta #'documented)]",
                    " [(get m :doc) (get m :arglists) (get m :schema)])"
                ))
                .unwrap(),
            "[\"Returns its argument.\" [[value]] [:fn [:int] :int]]"
        );
        assert_eq!(
            runtime
                .eval_text(concat!(
                    "(let [m (meta #'std.foundation.string/length)]",
                    " [(get m :doc) (get m :arglists) (get m :schema)])"
                ))
                .unwrap(),
            concat!(
                "[\"Returns the portable character count of value.\"",
                " [[value]] [:fn [:str] :int]]"
            )
        );
    }

    #[test]
    fn definitions_accept_source_metadata_around_hir_syntax() {
        let mut runtime = Runtime::new();
        runtime
            .eval_text(concat!(
                "(defn wrapped ^{:line 1} [value] value)",
                " (defn wrapped-many",
                " ^{:line 2} ([value] value)",
                " ^{:line 3} ([left right] (+ left right)))"
            ))
            .unwrap();
        assert_eq!(
            runtime
                .eval_text(concat!(
                    "[(wrapped 42)",
                    " (wrapped-many 42)",
                    " (wrapped-many 19 23)]"
                ))
                .unwrap(),
            "[42 42 42]"
        );
    }

    #[test]
    fn namespace_values_and_operations_match_java_registry_semantics() {
        let mut runtime = Runtime::new();
        let initial_namespace_count = runtime.namespace_registry.all().len();
        assert_eq!(
            runtime
                .eval_text("(ns:name (ns:create (quote example.lib)))")
                .unwrap(),
            "example.lib"
        );
        assert_eq!(
            runtime
                .eval_text("(= (ns:create (quote example.lib)) (ns:create (quote example.lib)))")
                .unwrap(),
            "true"
        );
        assert_eq!(
            runtime
                .eval_text("(ns example.lib) (def answer 42) (ns user) (deref (get (ns:map (ns:find (quote example.lib))) (quote answer)))")
                .unwrap(),
            "42"
        );
        assert_eq!(
            runtime.eval_text("(count (ns:list))").unwrap(),
            (initial_namespace_count + 1).to_string()
        );
        assert_eq!(
            runtime.eval_text("(ns:find (quote missing.lib))").unwrap(),
            "nil"
        );
        runtime.alias_namespace("lib", "example.lib");
        assert_eq!(
            runtime
                .eval_text("(= (get (ns:aliases (ns:find (quote user))) (quote lib)) (ns:find (quote example.lib)))")
                .unwrap(),
            "true"
        );
        assert!(runtime
            .eval_text("(ns:create (quote bad/name))")
            .unwrap_err()
            .contains("unqualified symbol"));
    }

    #[test]
    fn namespace_use_refers_portable_test_vars_and_macros() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text(concat!(
                    "(ns code.test-rust-probe (:use code.test))",
                    " (def lifecycle (atom []))",
                    " (fact \"promise assertion\"",
                    "   {:before (fn []",
                    "              (swap! lifecycle",
                    "                     (fn [events] (conj events :before))))",
                    "    :after (fn []",
                    "             (swap! lifecycle",
                    "                    (fn [events] (conj events :after))))}",
                    "   (promise/from 42) => 42",
                    "   (+ 1 1) => 2)",
                    " (let [summary (run {:namespace \"code.test-rust-probe\"})",
                    "       timer (function-timer",
                    "              (fn [promise milliseconds]",
                    "                {:promise (promise/from {:test/status :timeout})",
                    "                 :timeout milliseconds})",
                    "              (fn [timeout] timeout))",
                    "       timed (check (fn [] (promise/from 42)) 42",
                    "                    {:timer timer :timeout 25})",
                    "       positional (run '[code])",
                    "       cancelled",
                    "       (run {:namespace \"code.test-rust-probe\"",
                    "             :control (function-control (fn [fact] true))})]",
                    " [(:status summary)",
                    "  (:passed (:counts summary))",
                    "  (count (:checks (first (:results summary))))",
                    "  (:status timed)",
                    "  (:timeout timed)",
                    "  (:facts positional)",
                    "  (:cancelled (:counts cancelled))])"
                ))
                .unwrap(),
            "[:passed 1 2 :timeout 25 1 1]"
        );
    }

    #[test]
    fn foundation_code_test_compatibility_namespaces_are_embedded() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text(
                    "(ns code-test-compat-rust-probe \
                       (:require [code.test :as test] \
                                 [code.test.checker.common :as common] \
                                 [code.test.checker.collection :as collection] \
                                 [code.test.checker.logic :as logic] \
                                 [code.test.base.runtime :as runtime] \
                                 [code.test.compile.types :as types] \
                                 [code.test.task :as task])) \
                     (let [fact (types/Fact :core 'id 'probe nil nil \
                                            \"portable\" 1 1 nil nil \
                                            (fn [] 42) {})] \
                       [(common/succeeded? \
                         (common/verify (common/exactly 1) 1)) \
                        (:pass (test/check \
                                (fn [] {:a 1 :b 2}) \
                                (collection/contains-map {:a 1}))) \
                        (:pass (test/check \
                                (fn [] 3) \
                                (logic/all (fn [value] (number? value)) \
                                           (fn [value] (= 1 (mod value 2)))))) \
                        (types/fact? fact) \
                        (fact) \
                        (task/process-test-args \
                         [\":only\" \"std\" \"code\"])])"
                )
                .unwrap(),
            "[true true true true 42 {:ns [std code]}]"
        );
    }

    #[test]
    fn canonical_component_and_context_libraries_load_without_old_aliases() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text(
                    "(ns std-lib-context-rust-probe \
                       (:require [std.lib.component :as component] \
                                 [std.lib.context :as context])) \
                     (let [runtime (context/runtime-null)] \
                       [(component/started? runtime) \
                        (context/call runtime :a :b)])"
                )
                .unwrap(),
            "[true [:a :b]]"
        );
        assert!(runtime
            .eval_text("(require [std.foundation.component :as old])")
            .unwrap_err()
            .contains("missing"));
    }

    #[test]
    fn portable_task_bulk_execution_is_data_first() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text(
                    "(ns std-task-rust-probe \
                       (:require [std.lib.task :as task] \
                                 [std.lib.task.bulk :as bulk])) \
                     (task/deftask double-task \
                       {:template :default \
                        :main {:fn (fn [value] (* 2 value))}}) \
                     (task/deftask selected-task \
                       {:template :default \
                        :main {:fn (fn [value suffix] (str value suffix))} \
                        :item {:list (fn [lookup env] \
                                      ['code.alpha 'code.beta 'std.gamma])}}) \
                     (task/deftask aggregate-task \
                       {:template :default \
                        :main {:fn (fn [value] {:score value})} \
                        :result {:ignore (fn [data] \
                                           (= 0 (get data :score))) \
                                 :output (fn [data] (get data :score))} \
                        :summary {:aggregate \
                                  {:score-total \
                                   [(fn [data] (get data :score)) \
                                    (fn [total score] (+ total score)) 0]}}}) \
                     (task/deftask constructed-task \
                       {:template :default \
                        :main {:argcount 4 \
                               :fn (fn [input params lookup env] \
                                     [input (get params :flag) \
                                      (get lookup :input) \
                                      (get env :environment)])} \
                        :construct \
                        {:input (fn [task] 7) \
                         :env (fn [options] {:environment :ready}) \
                         :lookup (fn [task options] \
                                   {:input (get options :environment)})} \
                        :params {:flag true}}) \
                     (let [reporter (bulk/event-reporter) \
                           output (task/invoke double-task [1 2 3] \
                                               {:reporter reporter :return :all \
                                                :package :records}) \
                           aggregate-output \
                           (task/invoke aggregate-task [0 2 3] \
                                        {:return :all})] \
                       [(get output :summary) \
                        (vec (map (fn [result] (get result :data)) \
                                  (get output :results))) \
                        (count (bulk/reporter-events reporter)) \
                        (task/invoke double-task 4) \
                        (vec (map (fn [result] (get result :data)) \
                                  (task/invoke selected-task 'code \
                                               {:package :records} \
                                               :args \"!\"))) \
                        (get aggregate-output :results) \
                        (get (get aggregate-output :summary) \
                             :score-total) \
                        (task/invoke constructed-task)])"
                )
                .unwrap(),
            "[{:items 3 :results 3 :warnings 0 :errors 0 :cumulative 0 :elapsed 0} [2 4 6] 8 8 [\"code.alpha!\" \"code.beta!\"] {2 2 3 3} 5 [7 true :ready :ready]]"
        );
    }

    #[test]
    fn portable_block_preserves_source_value_and_structure() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text(
                    "(ns std-block-rust-probe \
                       (:require [std.lib.block :as block] \
                                 [std.lib.block.grid :as grid] \
                                 [std.lib.block.reader :as reader])) \
                     (let [parsed (block/parse-string \"[1 2 3]\") \
                           first-block (block/parse-first \"[1 2 3]\") \
                           spaces (block/spaces 3) \
                           wrapped (block/layout '(if ready [1 2] [3 4]) \
                                                 {:width 10}) \
                           gridded (grid/grid \
                                    (block/parse-first \"(if\\nready\\ndone)\") \
                                    0 {:rules {'if {:indent 1}}}) \
                           modified (block/parse-first \"[1 #_2 3]\") \
                           original (block/block [1 2]) \
                           input-reader (reader/create \"ab\\ncd\") \
                           first-two (reader/read-times input-reader \
                                                        reader/read-char 2) \
                           newline (reader/read-char input-reader) \
                           edited (std.lib.zip/result \
                                   (std.lib.zip/replace-right \
                                    (std.lib.zip/step-right \
                                     (std.lib.zip/step-right \
                                     (std.lib.zip/step-inside \
                                      (block/block-zip original)))) \
                                    (block/block 3)))] \
                       [(block/string parsed) \
                        (block/value parsed) \
                        (block/type first-block) \
                        (block/tag first-block) \
                        (vec (map block/value \
                                  (filter block/code? \
                                          (block/children first-block)))) \
                        (block/string spaces) \
                        (block/space? spaces) \
                        (block/string wrapped) \
                        (block/string gridded) \
                        (block/value modified) \
                        (block/child-values modified) \
                        (block/string original) \
                        (block/string edited) \
                        first-two \
                        (reader/reader-position input-reader) \
                        (reader/read-to-boundary input-reader) \
                        (block/value (block/parse-string \"[4 5]\"))])"
                )
                .unwrap(),
            "[\"[1 2 3]\" [1 2 3] :container :vector [1 2 3] \"   \" true \"(if\\n  ready\\n  [1 2]\\n  [3 4]\\n)\" \"(if\\n  ready\\n  done)\" [1 3] [1 3] \"[1 2]\" \"[1 3]\" [\"a\" \"b\"] [2 1] \"cd\" [4 5]]"
        );
    }

    #[test]
    fn portable_zip_is_embedded_and_preserves_original_values() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text(
                    "(ns std-lib-zip-rust-probe \
                       (:require [std.lib.zip :as zip])) \
                     (let [root [1 2 3] \
                           location (zip/step-right \
                                     (zip/step-inside (zip/vector-zip root))) \
                           edited (zip/replace-right \
                                   (zip/insert-left location 9) 8)] \
                       [(zip/result edited) root])"
                )
                .unwrap(),
            "[[1 9 8 3] [1 2 3]]"
        );
    }

    #[test]
    fn portable_collection_execution_preserves_hal_semantics_and_errors() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text(
                    "[(reduce (fn [total value] (+ total value)) 0 [1 2 3 4]) \
                      (let [answer 41] (+ answer 1) (+ answer 2))]"
                )
                .unwrap(),
            "[10 43]"
        );
        assert!(runtime
            .eval_text("(vec (map (fn [value] (/ 1 value)) [1 0]))")
            .unwrap_err()
            .contains("division by zero"));
    }

    #[test]
    fn named_values_expose_java_basic_object_operations() {
        let mut runtime = Runtime::new();
        assert_eq!(runtime.eval_text("(compare :a :b)").unwrap(), "-1");
        assert_eq!(
            runtime
                .eval_text("(compare (symbol \"a\") (symbol \"a\"))")
                .unwrap(),
            "0"
        );
        assert_eq!(
            runtime
                .eval_text("(= (hash [1 2]) (hash (list 1 2)))")
                .unwrap(),
            "true"
        );
        assert_eq!(runtime.eval_text("(meta :answer)").unwrap(), "nil");
        assert_eq!(
            runtime
                .eval_text("(with-meta :answer {:doc \"ignored\"})")
                .unwrap(),
            ":answer"
        );
        assert_eq!(
            runtime
                .eval_text("(get (meta (with-meta (symbol \"answer\") {:doc \"named\"})) :doc)")
                .unwrap(),
            "\"named\""
        );
        assert_eq!(
            runtime
                .eval_text("(get (meta (with-meta [1] {:doc \"vector\"})) :doc)")
                .unwrap(),
            "\"vector\""
        );
        assert_eq!(
            runtime.eval_text("(hash)").unwrap_err(),
            "hash expects one value"
        );
    }

    #[test]
    fn cons_pointer_and_tagged_literals_are_first_class_runtime_values() {
        let mut runtime = Runtime::new();
        assert_eq!(runtime.eval_text("(cons 0 [1 2])").unwrap(), "(0 1 2)");
        assert_eq!(
            runtime.eval_text("(type (cons 0 [1 2]))").unwrap(),
            ":hara.type/cons"
        );
        assert_eq!(runtime.eval_text("(count (cons 0 [1 2]))").unwrap(), "3");
        assert_eq!(runtime.eval_text("(get (cons 0 [1 2]) 2)").unwrap(), "2");
        assert_eq!(
            runtime
                .eval_text("(pointer \"hara.core\" \"value\")")
                .unwrap(),
            "#\x27hara.core/value"
        );
        assert_eq!(
            runtime
                .eval_text("(type (pointer \"hara.core/value\"))")
                .unwrap(),
            ":hara.type/pointer"
        );
        assert_eq!(
            runtime.eval_text("(type #sample [1 2])").unwrap(),
            ":hara.type/tagged-literal"
        );
        assert_eq!(runtime.eval_text("(ILookup/lookup (IObjType/meta (IObjType/with-meta (cons 0 [1]) {:doc \"cons\"})) :doc)").unwrap(), "\"cons\"");
    }

    #[test]
    fn keyword_symbol_constructors_and_namespaced_protocol_match_java() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime.eval_text("(keyword \"answer\")").unwrap(),
            ":answer"
        );
        assert_eq!(
            runtime.eval_text("(keyword \"core\" \"answer\")").unwrap(),
            ":core/answer"
        );
        assert_eq!(runtime.eval_text("(symbol \"answer\")").unwrap(), "answer");
        assert_eq!(
            runtime.eval_text("(symbol \"core\" \"answer\")").unwrap(),
            "core/answer"
        );
        assert_eq!(
            runtime
                .eval_text("(INamespaced/name :core/answer)")
                .unwrap(),
            "\"answer\""
        );
        assert_eq!(
            runtime
                .eval_text("(INamespaced/namespace (symbol \"core\" \"answer\"))")
                .unwrap(),
            "\"core\""
        );
        assert_eq!(
            runtime
                .eval_text("(INamespaced/namespace :answer)")
                .unwrap(),
            "nil"
        );
        assert!(runtime
            .eval_text("(keyword \"a/b/c\")")
            .unwrap_err()
            .contains("one slash"));
        assert!(runtime
            .eval_text("(symbol 1)")
            .unwrap_err()
            .contains("string arguments"));
    }

    #[test]
    fn reader_vectors_use_java_tuple_arity_selection() {
        let mut env = HashMap::new();
        let small = core::eval(&kernel::parse("[1 2 3]").unwrap(), &mut env).unwrap();
        let large = core::eval(&kernel::parse("[1 2 3 4 5 6]").unwrap(), &mut env).unwrap();
        assert!(matches!(small, core::Value::Tuple(_)));
        assert!(matches!(large, core::Value::Vector(_)));

        let mut runtime = Runtime::new();
        assert_eq!(runtime.eval_text("(nth [1 2 3] 1)").unwrap(), "2");
        assert_eq!(runtime.eval_text("(conj [1 2 3] 4)").unwrap(), "[1 2 3 4]");
        assert_eq!(
            runtime
                .eval_text(
                    "(loop [values [] n 0]
                       (if (< n 32)
                         (recur (conj values n) (+ n 1))
                         [(count values) (first values) (nth values 31)]))"
                )
                .unwrap(),
            "[32 0 31]"
        );
        let promoted = core::eval(
            &kernel::parse("(conj (conj (conj (conj [0 1 2 3 4] 5) 6) 7) 8)").unwrap(),
            &mut env,
        )
        .unwrap();
        assert!(matches!(promoted, core::Value::Vector(values) if values.len() == 9));
        assert_eq!(
            runtime
                .eval_text("(ILookup/lookup (IObjType/meta (IObjType/with-meta [1] {:doc \"tuple\"})) :doc)")
                .unwrap(),
            "\"tuple\""
        );
    }

    #[test]
    fn reader_maps_and_sets_preserve_java_insertion_order() {
        let mut runtime = Runtime::new();
        assert_eq!(runtime.eval_text("{:b 2 :a 1}").unwrap(), "{:b 2 :a 1}");
        assert_eq!(runtime.eval_text("(keys {:b 2 :a 1})").unwrap(), "[:b :a]");
        assert_eq!(runtime.eval_text("#{:b :a}").unwrap(), "#{:b :a}");
        assert_eq!(
            runtime
                .eval_text("(conj (dissoc {:a 1 :b 2} :a) [:a 3])")
                .unwrap(),
            "{:b 2 :a 3}"
        );
    }

    #[test]
    fn collection_operations_are_values() {
        let mut runtime = Runtime::new();
        assert_eq!(runtime.eval_text("(count [1 2 3])").unwrap(), "3");
        assert_eq!(runtime.eval_text("(get {\"a\" 9} \"a\")").unwrap(), "9");
        assert_eq!(runtime.eval_text("(nth (conj [1] 2) 1)").unwrap(), "2");
        assert_eq!(
            runtime.eval_text(r#"(conj {"a" 1} ["b" 2])"#).unwrap(),
            r#"{"a" 1 "b" 2}"#
        );
        assert_eq!(
            runtime
                .eval_text(r#"(get (conj {"a" 1} ["a" 9]) "a")"#)
                .unwrap(),
            "9"
        );
        assert_eq!(
            runtime.eval_text(r#"(dissoc {"a" 1 "b" 2} "a")"#).unwrap(),
            r#"{"b" 2}"#
        );
        assert_eq!(
            runtime
                .eval_text(r#"(dissoc {"a" 1 "b" 2} "a" "b")"#)
                .unwrap(),
            "{}"
        );
        assert_eq!(runtime.eval_text("(cons 0 [1 2])").unwrap(), "(0 1 2)");
        assert_eq!(runtime.eval_text("(= :ready :ready)").unwrap(), "true");
    }

    #[test]
    fn persistent_vectors_and_lists_keep_previous_values() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text("(let (source [1 2]) (get (conj source 3) 2))")
                .unwrap(),
            "3"
        );
        assert_eq!(
            runtime
                .eval_text("(let (source [1 2]) (count source))")
                .unwrap(),
            "2"
        );
        assert!(runtime
            .eval_text("(conj (rest [1 2]) 2)")
            .unwrap_err()
            .contains("IConj/conj expects a collection"));
        assert_eq!(
            runtime.eval_text("(vec (cons 0 (rest [1 2])))").unwrap(),
            "[0 2]"
        );
        assert_eq!(
            runtime
                .eval_text("(let (source (rest [1 2])) (count source))")
                .unwrap(),
            "1"
        );
    }

    struct RangeExtension;

    impl core::ExtensionProvider for RangeExtension {
        fn name(&self) -> &str {
            "range"
        }

        fn install(&self, protocols: &mut core::ProtocolRegistry) {
            protocols.register("IIter", "iter", |arguments| match arguments.first() {
                Some(core::Value::Extension(value))
                    if value.provider == "range" && value.type_name == "range" =>
                {
                    Ok(core::iterator_from_values(
                        (0..value.handle)
                            .map(|index| core::Value::Number(index as i64))
                            .collect(),
                    ))
                }
                _ => Err("range/IIter does not accept this value".into()),
            });
        }

        fn construct(
            &self,
            type_name: &str,
            arguments: &[core::Value],
        ) -> Result<core::Value, String> {
            if type_name != "range" {
                return Err("range/type-not-found".into());
            }
            let count = match arguments.first() {
                Some(core::Value::Number(count)) if *count >= 0 => *count as u64,
                _ => return Err("range expects a non-negative count".into()),
            };
            Ok(core::Value::Extension(core::ExtensionValue {
                provider: "range".into(),
                type_name: "range".into(),
                handle: count,
            }))
        }
    }

    fn protocol_identity(arguments: &[core::Value]) -> Result<core::Value, String> {
        arguments
            .first()
            .cloned()
            .ok_or_else(|| "missing receiver".into())
    }

    fn protocol_custom_iterator(_arguments: &[core::Value]) -> Result<core::Value, String> {
        Ok(core::iterator_from_values(vec![
            core::Value::Number(7),
            core::Value::Number(8),
        ]))
    }

    #[test]
    fn promise_constructors_and_composition() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text("(deref (promise/new (fn [resolve reject] (resolve 42))))")
                .unwrap(),
            "42"
        );
        assert_eq!(
            runtime.eval_text("(deref (promise (fn [] 40)))").unwrap(),
            "40"
        );
        assert_eq!(
            runtime.eval_text("(deref (promise/from 42))").unwrap(),
            "42"
        );
        assert_eq!(
            runtime.eval_text("(promise? (promise/from 1))").unwrap(),
            "true"
        );
        assert_eq!(runtime.eval_text("(promise? 1)").unwrap(), "false");
        assert_eq!(
            runtime
                .eval_text("(deref (promise/then (promise (fn [] 40)) (fn [x] (+ x 2))))")
                .unwrap(),
            "42"
        );
        assert_eq!(
            runtime
                .eval_text("(deref (promise/catch (promise (fn [] (throw :bad))) (fn [error] 7)))")
                .unwrap(),
            "7"
        );
        assert_eq!(
            runtime
                .eval_text("(deref (promise/finally (promise (fn [] 4)) (fn [] 99)))")
                .unwrap(),
            "4"
        );
        assert_eq!(
            runtime
                .eval_text(
                    "(. (deref (promise/all [(promise (fn [] 1)) 2 (promise (fn [] 3))])) (get 1))"
                )
                .unwrap(),
            "2"
        );
        assert_eq!(
            runtime
                .eval_text("(deref (promise (fn [] (promise (fn [] 9)))))")
                .unwrap(),
            "9"
        );
        assert_eq!(
            runtime
                .eval_text("(deref (promise/delay 0 (fn [] 5)))")
                .unwrap(),
            "5"
        );
        assert!(runtime
            .eval_text("(promise/delay -1 (fn [] 1))")
            .unwrap_err()
            .contains("non-negative"));
        assert!(runtime
            .eval_text("(promise/new 1)")
            .unwrap_err()
            .contains("expects a function"));
    }
    #[test]
    fn promise_continuations_preserve_registration_order_and_late_delivery() {
        let promise = core::Promise::new();
        let events = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let first = events.clone();
        promise.on_settle(std::rc::Rc::new(move |_| first.borrow_mut().push(1)));
        let second = events.clone();
        promise.on_settle(std::rc::Rc::new(move |_| second.borrow_mut().push(2)));
        assert!(promise.resolve(core::Value::Number(7)));
        assert_eq!(*events.borrow(), vec![1, 2]);
        let late = events.clone();
        promise.on_settle(std::rc::Rc::new(move |_| late.borrow_mut().push(3)));
        assert_eq!(*events.borrow(), vec![1, 2, 3]);
        assert!(!promise.reject("late"));
    }

    #[test]
    fn promises_settle_once_and_adopt() {
        let pending = core::Promise::new();
        let adopted = core::Promise::new();
        assert_eq!(pending.state(), core::PromiseState::Pending);
        assert!(adopted.adopt(&pending));
        assert!(pending.resolve(core::Value::Number(7)));
        assert!(!pending.reject("late"));
        assert_eq!(
            adopted.state(),
            core::PromiseState::Fulfilled(core::Value::Number(7))
        );
    }

    #[test]
    fn marker_mutation_methods_cover_array_and_object_boundaries() {
        let mut runtime = Runtime::new();
        assert_eq!(runtime.eval_text("(let (a (array 2)) (do (. a (push-first 1)) (. a (push-last 3)) (. a (insert 1 9)) (. a (get 1))))").unwrap(), "9");
        assert_eq!(
            runtime
                .eval_text(
                    "(let (a (array 1 2)) (do (. a (pop-first)) (. a (pop-last)) (count a)))"
                )
                .unwrap(),
            "0"
        );
        assert_eq!(
            runtime
                .eval_text(r#"(. (object "a" 1 "b" 2) (keys))"#)
                .unwrap(),
            r#"(array "a" "b")"#
        );
        assert_eq!(
            runtime
                .eval_text(r#"(. (object "a" 1 "b" 2) (vals))"#)
                .unwrap(),
            "(array 1 2)"
        );
        assert_eq!(
            runtime
                .eval_text(
                    r#"(let (o (object "a" 1)) (do (. o (assign (object "b" 2))) (. o (get "b"))))"#
                )
                .unwrap(),
            "2"
        );
    }

    #[test]
    fn marker_dot_contract_covers_results_identity_callbacks_and_rejections() {
        let mut runtime = Runtime::new();
        let cases = [
            ("(. (. (array 1 2 3) (map (fn [x] (* x 2)))) (get 2))", "6"),
            (
                "(. (. (array 1 2 3 4) (filter (fn [x] (> x 2)))) (get 0))",
                "3",
            ),
            ("(. (. (array 1 2 3) (slice 1)) (get 1))", "3"),
            (
                "(. (array 1 2 3) (fold-left (fn [out x] (- out x)) 0))",
                "-6",
            ),
            (
                "(. (array 1 2 3) (fold-right (fn [x out] (- x out)) 0))",
                "2",
            ),
            ("(let [a (array 1)] (= a (. a (push-last 2))))", "true"),
            ("(let [a (array 1)] (= a (. a (set 0 2))))", "true"),
            ("(let [a (array 1)] (= a (. a (insert 1 2))))", "true"),
            ("(let [a (array 1)] (= a (. a (clone))))", "false"),
            (
                r#"(let [o (object "a" 1)] (= o (. o (set "a" 2))))"#,
                "true",
            ),
            (r#"(. (object "a" 1) (delete "a"))"#, "1"),
            (r#"(. (object "a" 1) (delete "missing"))"#, "nil"),
            (r#"(. (. (object "a" 1) (keys)) (get 0))"#, r#""a""#),
            (r#"(. (. (. (object "a" 1) (pairs)) (get 0)) (get 1))"#, "1"),
            ("(iter-next (iter (array 7 8)))", "7"),
            (r#"(second (iter-next (iter (object "a" 7))))"#, "7"),
        ];
        for (source, expected) in cases {
            assert_eq!(runtime.eval_text(source).unwrap(), expected, "{source}");
        }

        let invalid = [
            ("(. [1 2] (get 0))", "array or object marker"),
            ("(. {} (get \"a\"))", "array or object marker"),
            ("(. 1 (get 0))", "array or object marker"),
            ("(. (array 1) (unknown))", "unsupported array method"),
            (
                r#"(. (object "a" 1) (unknown))"#,
                "unsupported object method",
            ),
            ("(. (array 1) (set 0))", "expects an index and value"),
            ("(. (array 1) (clone 1))", "expects no arguments"),
            (r#"(. (object "a" 1) (clone 1))"#, "expects no arguments"),
            (
                "(. (array 1) (map (fn [x y] x)))",
                "function expects 2 arguments",
            ),
            ("(x:array 1)", "unbound symbol: x:array"),
            ("(x:object)", "unbound symbol: x:object"),
            ("(x:get nil 0)", "unbound symbol: x:get"),
            ("(x:set nil 0 1)", "unbound symbol: x:set"),
            (
                r#"(host-symbol "java.lang.String")"#,
                "unbound symbol: host-symbol",
            ),
            (r#"(host-get nil "value")"#, "unbound symbol: host-get"),
            (r#"(host-call nil "run")"#, "unbound symbol: host-call"),
        ];
        for (source, message) in invalid {
            assert!(
                runtime.eval_text(source).unwrap_err().contains(message),
                "{source}"
            );
        }
    }
    #[test]
    fn marker_arrays_and_objects_use_restricted_dot_calls() {
        let mut runtime = Runtime::new();
        assert_eq!(runtime.eval_text("(count (array 1 2 3))").unwrap(), "3");
        assert_eq!(runtime.eval_text("(. (array 1 2) (get 1))").unwrap(), "2");
        assert_eq!(
            runtime
                .eval_text("(let (a (array 1 2)) (do (. a (set 1 7)) (. a (get 1))))")
                .unwrap(),
            "7"
        );
        assert_eq!(
            runtime
                .eval_text("(let (a (array 1)) (do (. a (push-last 2)) (count a)))")
                .unwrap(),
            "2"
        );
        assert_eq!(
            runtime
                .eval_text(r#"(. (object "answer" 41) (get "answer"))"#)
                .unwrap(),
            "41"
        );
        assert_eq!(
            runtime
                .eval_text(
                    r#"(let (o (object)) (do (. o (set "answer" 42)) (. o (get "answer"))))"#
                )
                .unwrap(),
            "42"
        );
        assert_eq!(
            runtime
                .eval_text(r#"(. (object "answer" 41) (has? "answer"))"#)
                .unwrap(),
            "true"
        );
    }

    #[test]
    fn strings_and_bytes_support_utf8_copy_and_slice() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime.eval_text(r#"(str "hello" " " "world")"#).unwrap(),
            "\"hello world\""
        );
        assert_eq!(runtime.eval_text(r#"(str/length "a😀b")"#).unwrap(), "3");
        assert_eq!(
            runtime.eval_text(r#"(str/char-at "a😀b" 1)"#).unwrap(),
            "\"😀\""
        );
        assert_eq!(
            runtime.eval_text(r#"(str/slice "a😀b" 1 2)"#).unwrap(),
            "\"😀\""
        );
        assert_eq!(
            runtime.eval_text(r#"(str/index-of "a😀b" "b")"#).unwrap(),
            "2"
        );
        assert_eq!(
            runtime
                .eval_text(r#"(str/last-index-of "😀a😀" "😀")"#)
                .unwrap(),
            "2"
        );
        assert_eq!(
            runtime.eval_text(r#"(str/pad-left "x" 3 "😀")"#).unwrap(),
            "\"😀😀x\""
        );
        assert_eq!(
            runtime.eval_text(r#"(str/trim "  hara  ")"#).unwrap(),
            "\"hara\""
        );
        assert_eq!(
            runtime
                .eval_text(r#"(str/decode-utf8 (str/encode-utf8 "hé"))"#)
                .unwrap(),
            "\"hé\""
        );
        assert_eq!(
            runtime
                .eval_text("(bytes/slice (bytes 1 2 3) 1 3)")
                .unwrap(),
            "(bytes 2 3)"
        );
        assert_eq!(runtime.eval_text("(let (source (bytes 1 2)) (let (copy (bytes/copy source)) (do (bytes/set copy 0 9) (bytes/get source 0))))").unwrap(), "1");
    }

    #[test]
    fn byte_buffers_preserve_signed_storage_and_unsigned_reads() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime.eval_text("(bytes 1 2 -3)").unwrap(),
            "(bytes 1 2 -3)"
        );
        assert_eq!(
            runtime.eval_text("(bytes/get (bytes 1 2 -3) 2)").unwrap(),
            "253"
        );
        assert_eq!(runtime.eval_text("(bytes/u8 -1)").unwrap(), "255");
        assert_eq!(runtime.eval_text("(bytes/s8 255)").unwrap(), "-1");
        assert_eq!(
            runtime
                .eval_text("(let (b (bytes 1 2)) (do (bytes/set b 0 9) (bytes/get b 0)))")
                .unwrap(),
            "9"
        );
        assert_eq!(runtime.eval_text("(bytes/get (bytes 1) 4 7)").unwrap(), "7");
        assert_eq!(
            runtime.eval_text("(bytes/count (bytes 1 2 -3))").unwrap(),
            "3"
        );
    }

    #[test]
    fn bytes_and_bits_cover_conversion_copy_and_overflow_boundaries() {
        let mut runtime = Runtime::new();
        let cases = [
            ("(bytes/u8 -128)", "128"),
            ("(bytes/u8 255)", "255"),
            ("(bytes/s8 -128)", "-128"),
            ("(bytes/s8 128)", "-128"),
            ("(bytes/s8 255)", "-1"),
            ("(bytes/get (bytes -128 0 127 255) 0)", "128"),
            ("(bytes/get (bytes -128 0 127 255) 3)", "255"),
            ("(bytes/slice (bytes 1 2 3) 1)", "(bytes 2 3)"),
            ("(bytes/slice (bytes 1 2 3) 1 1)", "(bytes)"),
            (
                "(let [b (bytes 0)] (count [(bytes/set b 0 255) (bytes/get b 0)]))",
                "2",
            ),
            ("(bit-not -2147483648)", "2147483647"),
            ("(bit-not 2147483647)", "-2147483648"),
            ("(bit-and -2147483648 2147483647)", "0"),
            ("(bit-or -2147483648 1)", "-2147483647"),
            ("(bit-xor -1 2147483647)", "-2147483648"),
            ("(bit-shift-left 1 0)", "1"),
            ("(bit-shift-left 1 31)", "-2147483648"),
            ("(bit-shift-left 2147483647 1)", "-2"),
            ("(bit-shift-right -2147483648 31)", "-1"),
            ("(bit-shift-right 2147483647 31)", "0"),
            ("(bit-shift-left 2147483648 0)", "-2147483648"),
        ];
        for (source, expected) in cases {
            assert_eq!(runtime.eval_text(source).unwrap(), expected, "{source}");
        }

        let invalid = [
            ("(bytes -129)", "range -128..255"),
            ("(bytes 256)", "range -128..255"),
            ("(bytes/u8 -129)", "range -128..255"),
            ("(bytes/s8 256)", "range -128..255"),
            ("(bytes/get (bytes 1) 1)", "out of bounds"),
            ("(bytes/set (bytes 1) 1 0)", "out of bounds"),
            ("(bytes/slice (bytes 1 2) 2 1)", "out of bounds"),
            ("(bytes/slice (bytes 1 2) 0 3)", "out of bounds"),
            ("(str/decode-utf8 (bytes 255))", "invalid UTF-8"),
            ("(bit-shift-left 1 -1)", "range 0..31"),
            ("(bit-shift-right 1 32)", "range 0..31"),
        ];
        for (source, message) in invalid {
            assert!(
                runtime.eval_text(source).unwrap_err().contains(message),
                "{source}"
            );
        }

        assert_eq!(
            runtime
                .eval_text("(let [source (bytes 1 2 3) copy (bytes/copy source)] (do (bytes/set copy 0 9) (bytes/get source 0)))")
                .unwrap(),
            "1"
        );
        assert_eq!(
            runtime
                .eval_text("(let [source (bytes 1 2 3) part (bytes/slice source 0 2)] (do (bytes/set part 0 9) (bytes/get source 0)))")
                .unwrap(),
            "1"
        );
    }
    #[test]
    fn iterator_aliases_and_combinators_match_core_shapes() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text("(nth (map (fn [x] (* x 2)) [1 2]) 0)")
                .unwrap(),
            "2"
        );
        assert_eq!(
            runtime
                .eval_text("(nth (filter (fn [x] (= x 2)) [1 2 3]) 0)")
                .unwrap(),
            "2"
        );
        assert_eq!(
            runtime
                .eval_text("(nth (take 1 (drop 1 [1 2 3])) 0)")
                .unwrap(),
            "2"
        );
        assert_eq!(
            runtime.eval_text("(nth (nth (zip [1] [2]) 0) 1)").unwrap(),
            "2"
        );
        assert_eq!(
            runtime
                .eval_text(
                    "(let (it (cycle [1 2])) (do (iter-next it) (iter-next it) (iter-next it)))"
                )
                .unwrap(),
            "1"
        );
        assert_eq!(
            runtime.eval_text("(iter-next (concat [1] [2]))").unwrap(),
            "1"
        );
    }

    #[test]
    fn seq_boundaries_and_source_aware_transforms_match_design() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime.eval_text("(vector? (map inc [1 2 3]))").unwrap(),
            "true"
        );
        assert_eq!(runtime.eval_text("(first (map inc [1 2 3]))").unwrap(), "2");
        assert_eq!(
            runtime.eval_text("(first ((map inc) [1 2 3]))").unwrap(),
            "2"
        );
        assert_eq!(
            runtime
                .eval_text("(first ((map inc) (seq [1 2 3])))")
                .unwrap(),
            "2"
        );
        assert_eq!(
            runtime
                .eval_text("(first (seq (map inc) [1 2 3]))")
                .unwrap(),
            "2"
        );
        assert_eq!(
            runtime
                .eval_text("(first ((comp (map inc) (map inc)) [1 2 3]))")
                .unwrap(),
            "3"
        );
        assert_eq!(
            runtime
                .eval_text("(first (seq (comp (map inc) (map inc)) [1 2 3]))")
                .unwrap(),
            "3"
        );
    }

    #[test]
    fn issue_200_nil_terminates_sequences_and_iterator_lookahead_is_exact() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text(
                    "[(nil? (seq nil)) \
                      (nil? (seq [])) \
                      (nil? (rest nil)) \
                      (nil? (rest [])) \
                      (nil? (rest [1])) \
                      (seq? (rest [1 2])) \
                      (vec (rest [1])) \
                      (vec (rest [1 2]))]"
                )
                .unwrap(),
            "[true true true true true true [] [2]]"
        );
        assert_eq!(
            runtime
                .eval_text(
                    "(let [it (Iter/iter-drop 1 [1])] \
                       [(iter-next? it) (iter-next? it) (nil? (seq it))])"
                )
                .unwrap(),
            "[false false true]"
        );
        assert_eq!(
            runtime
                .eval_text(
                    "(let [it (Iter/iter-map inc [1])] \
                       [(iter-next? it) (iter-next? it) (iter-next it) (iter-next? it)])"
                )
                .unwrap(),
            "[true true 2 false]"
        );
    }

    #[test]
    fn issue_200_finite_generated_iterators_materialize_and_failures_propagate() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text(
                    "[(last (Iter/iter-take 2 [1 2 3])) \
                      (vec (reverse (Iter/iter-take 2 [1 2 3]))) \
                      (vec (Iter/iter-zip [1] (repeat 0))) \
                      (vec (Iter/iter-interleave [1] (repeat 0)))]"
                )
                .unwrap(),
            "[2 [2 1] [[1 0]] [1 0]]"
        );
        assert!(runtime
            .eval_text("(count (Iter/iter-map (fn [x] (throw \"boom\")) [1]))")
            .unwrap_err()
            .contains("boom"));
        assert!(runtime
            .eval_text("(count (Iter/iter-map (fn [x] (throw \"weekend\")) [1]))")
            .unwrap_err()
            .contains("weekend"));
        assert_eq!(
            runtime
                .eval_text(
                    "[(seq? (seq [1])) \
                      (iter? (seq [1])) \
                      (vec (cons 0 (rest [1 2]))) \
                      (vec (Iter/iter-take 4 (cons 0 (repeat 1))))]"
                )
                .unwrap(),
            "[true true [0 2] [0 1 1 1]]"
        );
        assert!(runtime
            .eval_text("(cycle [])")
            .unwrap_err()
            .contains("cycle expects a non-empty source"));
    }

    #[test]
    fn iterators_are_closeable_and_support_map_filter() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text("(let (it (iter [1 2])) (iter-next it))")
                .unwrap(),
            "1"
        );
        assert_eq!(
            runtime
                .eval_text("(let (it (iter [1 2])) (do (iter-next it) (iter-next it)))")
                .unwrap(),
            "2"
        );
        assert_eq!(runtime.eval_text("(iter-next? (iter [1]))").unwrap(), "true");
        assert_eq!(
            runtime
                .eval_text("(let (it (iter [1])) (do (Iter/iter-close it) (iter-next? it)))")
                .unwrap(),
            "false"
        );
        assert_eq!(
            runtime
                .eval_text("(let (it (Iter/iter-cycle [1 2])) (do (iter-next it) (Iter/iter-close it) (iter-next? it)))")
                .unwrap(),
            "false"
        );
        assert_eq!(
            runtime
                .eval_text("(let (it (Iter/iter-zip [1 2] [3 4])) (do (Iter/iter-close it) (iter-next? it)))")
                .unwrap(),
            "false"
        );
        assert_eq!(
            runtime
                .eval_text("(iter-next (Iter/iter-map (fn [x] (* x 2)) [1 2]))")
                .unwrap(),
            "2"
        );
        assert_eq!(
            runtime
                .eval_text("(iter-next (Iter/iter-filter (fn [x] (= x 2)) [1 2 3]))")
                .unwrap(),
            "2"
        );
        assert_eq!(
            runtime
                .eval_text("(receiver-category (iter [1]))")
                .unwrap_err(),
            "unbound symbol: receiver-category"
        );
    }

    #[test]
    fn evaluator_protocol_calls_cover_collections_and_bytes() {
        let mut runtime = Runtime::new();
        assert_eq!(runtime.eval_text("(ICount/count [1 2 3])").unwrap(), "3");
        assert_eq!(
            runtime.eval_text("(INth/nth (bytes 1 -3) 1)").unwrap(),
            "-3"
        );
        assert_eq!(
            runtime
                .eval_text(r#"(ILookup/lookup {"a" 9} "a")"#)
                .unwrap(),
            "9"
        );
        assert_eq!(
            runtime.eval_text(r#"(has? {"a" nil} "a")"#).unwrap(),
            "true"
        );
        assert_eq!(runtime.eval_text("(has? [10 20] 1)").unwrap(), "true");
        assert_eq!(runtime.eval_text("(has? [10 20] 10)").unwrap(), "false");
        assert_eq!(
            runtime
                .eval_text(r#"(IAssoc/assoc {"a" 9} "b" 10)"#)
                .unwrap(),
            r#"{"a" 9 "b" 10}"#
        );
        assert_eq!(runtime.eval_text(r#"(IConj/conj [1] 2)"#).unwrap(), "[1 2]");
        assert_eq!(
            runtime
                .eval_text(r#"(IDissoc/dissoc {"a" 9 "b" 10} "a")"#)
                .unwrap(),
            r#"{"b" 10}"#
        );
        runtime
            .protocols
            .register("IIter", "iter", protocol_custom_iterator);
        assert_eq!(runtime.eval_text("(iter-next (iter 99))").unwrap(), "7");
        assert!(runtime.has_protocol_method("IAssoc", "assoc"));
        assert!(runtime
            .eval_text("(ICount/count 1)")
            .unwrap_err()
            .contains("protocol/unsupported-receiver"));
    }

    #[test]
    fn portable_type_descriptors_cover_named_and_collection_values() {
        let mut runtime = Runtime::new();
        for (source, expected) in [
            ("nil", ":hara.type/nil"),
            (":key", ":hara.type/keyword"),
            ("(symbol \"hara/name\")", ":hara.type/symbol"),
            ("[]", ":hara.type/tuple"),
            ("(list)", ":hara.type/list"),
            ("(queue)", ":hara.type/queue"),
            ("(vector)", ":hara.type/vector"),
            ("(hash-map)", ":hara.type/hash-map"),
            ("{}", ":hara.type/ordered-map"),
            ("(sorted-map)", ":hara.type/sorted-map"),
            ("(trie)", ":hara.type/trie"),
            ("(hash-set)", ":hara.type/hash-set"),
            ("#{}", ":hara.type/ordered-set"),
            ("(sorted-set)", ":hara.type/sorted-set"),
            ("(bytes)", ":hara.type/byte-buffer"),
            ("(array)", ":hara.type/array"),
            ("(object)", ":hara.type/object"),
            ("(atom 0)", ":hara.type/atom"),
            ("(ns:create (quote example))", ":hara.type/namespace"),
        ] {
            assert_eq!(
                runtime.eval_text(&format!("(type {source})")).unwrap(),
                expected
            );
        }
        assert_eq!(
            runtime.eval_text("(type (type []))").unwrap(),
            ":hara.type/keyword"
        );
        assert!(runtime
            .eval_text("(type)")
            .unwrap_err()
            .contains("one value"));
    }

    #[test]
    fn protocol_registry_dispatches_by_protocol_and_method() {
        let mut registry = core::ProtocolRegistry::new();
        registry.register("IIdentity", "identity", protocol_identity);
        assert!(core::ProtocolRegistry::core().contains("IAssoc", "assoc"));
        assert!(registry.contains("IIdentity", "identity"));
        assert_eq!(
            registry
                .invoke("IIdentity", "identity", &[core::Value::Number(7)])
                .unwrap(),
            core::Value::Number(7)
        );
        assert!(registry
            .invoke("IIdentity", "missing", &[])
            .unwrap_err()
            .contains("missing protocol method"));
        assert_eq!(
            core::receiver_category(&core::Value::Vector(Default::default())),
            "vector"
        );
    }

    #[test]
    fn functions_support_variadic_rest_parameters() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text("((fn [x & rest] (+ x (count rest))) 40 1 2)")
                .unwrap(),
            "42"
        );
        assert_eq!(
            runtime
                .eval_text("(do (defn collect [x & rest] (count rest)) (collect 1 2 3 4))")
                .unwrap(),
            "3"
        );
        assert!(runtime
            .eval_text("((fn [x & rest] x))")
            .unwrap_err()
            .contains("at least 1"));
    }

    #[test]
    fn issue_133_cases_run_from_the_shared_l0_conformance_corpus() {
        fn entry<'a>(entries: &'a [(Form, Form)], key: &str) -> &'a Form {
            entries
                .iter()
                .find_map(|(candidate, value)| match candidate {
                    Form::Keyword(name) if name == key => Some(value),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("missing :{key}"))
        }

        let Some(corpus) = repo_text("specs/00-unsorted/platform-language/draft/conformance/l0.edn") else {
            return;
        };
        let manifest = kernel::parse_forms(&corpus).unwrap().remove(0);
        let Form::Map(manifest) = manifest else {
            panic!("L0 conformance corpus must be a map")
        };
        let Form::Vector(cases) = entry(&manifest, "cases") else {
            panic!("L0 conformance :cases must be a vector")
        };
        let ids = [
            "function/closure-capture",
            "function/fixed-arity",
            "function/variadic-arity",
            "function/multiple-arities",
            "function/arity-error",
            "binding/let-sequential",
            "binding/sequential-destructuring",
            "binding/map-destructuring",
            "binding/missing-destructuring",
            "binding/nil-map-default",
            "definition/doc-metadata",
            "definition/schema-metadata",
            "definition/arglists-metadata",
            "sequence/empty-is-nil",
            "sequence/non-empty-rest",
            "sequence/is-iterator",
            "sequence/lazy-cons",
            "sequence/reject-conj",
            "iterator/exact-lookahead",
            "iterator/generated-exhaustion",
            "iterator/shortest-source-finite",
            "iterator/nil-requires-conversion",
            "iterator/native-combinator-qualified",
            "iterator/root-combinator-unbound",
            "iterator/empty-cycle-rejected",
            "runtime/recur-outside-target",
            "runtime/recur-arity",
            "error/catch-guest-value",
            "error/catch-order",
            "error/unmatched-catch",
            "error/finally-normal",
            "error/finally-unwind",
        ];

        for id in ids {
            let case = cases
                .iter()
                .find(|case| {
                    matches!(
                        case,
                        Form::Map(entries)
                            if matches!(entry(entries, "id"), Form::Keyword(name) if name == id)
                    )
                })
                .unwrap_or_else(|| panic!("missing conformance case :{id}"));
            let Form::Map(case) = case else {
                unreachable!()
            };
            let Form::String(source) = entry(case, "source") else {
                panic!(":{id} source must be a string")
            };
            let Form::Map(expect) = entry(case, "expect") else {
                panic!(":{id} expect must be a map")
            };
            let mut runtime = Runtime::new();
            if expect
                .iter()
                .any(|(key, _)| matches!(key, Form::Keyword(name) if name == "error"))
            {
                assert!(runtime.eval_text(source).is_err(), ":{id} should fail");
            } else {
                let expected = match entry(expect, "value") {
                    Form::Number(value) => value.to_string(),
                    Form::String(value) => format!("{value:?}"),
                    Form::Bool(value) => value.to_string(),
                    Form::Nil => "nil".to_owned(),
                    value => panic!(":{id} has unsupported expected value {value:?}"),
                };
                let actual = runtime
                    .eval_text(source)
                    .unwrap_or_else(|error| panic!(":{id} unexpectedly failed: {error}"));
                assert_eq!(actual, expected, ":{id}");
            }
        }
    }

    #[test]
    fn issue_134_module_scenarios_have_machine_readable_acceptance_data() {
        fn entry<'a>(entries: &'a [(Form, Form)], key: &str) -> Option<&'a Form> {
            entries
                .iter()
                .find_map(|(candidate, value)| match candidate {
                    Form::Keyword(name) if name == key => Some(value),
                    _ => None,
                })
        }

        let Some(corpus) = repo_text("specs/00-unsorted/platform-language/draft/conformance/modules.edn") else {
            return;
        };
        let manifest = kernel::parse_forms(&corpus).unwrap().remove(0);
        let Form::Map(manifest) = manifest else {
            panic!("module conformance corpus must be a map")
        };
        let Some(Form::Vector(cases)) = entry(&manifest, "cases") else {
            panic!("module conformance :cases must be a vector")
        };
        assert!(cases.len() >= 20);
        let mut ids = HashSet::new();
        for case in cases {
            let Form::Map(case) = case else {
                panic!("module conformance cases must be maps")
            };
            let Some(Form::Keyword(id)) = entry(case, "id") else {
                panic!("module conformance case is missing :id")
            };
            assert!(ids.insert(id.clone()), "duplicate module case :{id}");
            assert!(
                matches!(entry(case, "area"), Some(Form::Keyword(_))),
                ":{id}"
            );
            assert!(
                matches!(entry(case, "scenario"), Some(Form::Keyword(_))),
                ":{id}"
            );
            assert!(matches!(entry(case, "expect"), Some(Form::Map(_))), ":{id}");
        }
    }

    #[test]
    fn module_ns_require_reload_executes_shared_spec_fixture() {
        fn entry<'a>(entries: &'a [(Form, Form)], key: &str) -> Option<&'a Form> {
            entries.iter().find_map(|(candidate, value)| {
                matches!(candidate, Form::Keyword(name) if name == key).then_some(value)
            })
        }

        let case = module_case("module/ns-require-reload");
        let Some(Form::Map(fixture)) = entry(&case, "fixture") else {
            panic!(":module/ns-require-reload must declare :fixture")
        };
        let Some(Form::Map(resource)) = entry(fixture, "resource") else {
            panic!("reload fixture must declare :resource")
        };
        let Some(Form::String(namespace)) = entry(resource, "namespace") else {
            panic!("reload resource must declare string :namespace")
        };
        let Some(Form::Map(revisions)) = entry(resource, "revisions") else {
            panic!("reload resource must declare :revisions")
        };
        let Some(Form::Vector(steps)) = entry(fixture, "steps") else {
            panic!("reload fixture must declare :steps")
        };

        let mut runtime = Runtime::new();
        for step in steps {
            let Form::Map(step) = step else {
                panic!("reload fixture steps must be maps")
            };
            let Some(Form::Keyword(operation)) = entry(step, "op") else {
                panic!("reload fixture step must declare :op")
            };
            match operation.as_str() {
                "resource/use" => {
                    let Some(Form::Keyword(revision)) = entry(step, "revision") else {
                        panic!(":resource/use must declare :revision")
                    };
                    let Some(Form::String(source)) = entry(revisions, revision) else {
                        panic!("missing reload resource revision :{revision}")
                    };
                    runtime.register_resource(namespace, source);
                }
                "eval" => {
                    let Some(Form::String(source)) = entry(step, "source") else {
                        panic!(":eval must declare string :source")
                    };
                    let Some(Form::Map(expect)) = entry(step, "expect") else {
                        panic!(":eval must declare :expect")
                    };
                    if let Some(Form::String(expected)) = entry(expect, "display") {
                        assert_eq!(
                            runtime.eval_text(source).unwrap_or_else(|error| {
                                panic!("shared reload eval failed for {source}: {error}")
                            }),
                            *expected
                        );
                    } else if matches!(entry(expect, "error"), Some(Form::Bool(true))) {
                        runtime
                            .eval_text(source)
                            .expect_err("shared reload eval must fail");
                    } else if let Some(Form::String(marker)) =
                        entry(expect, "error-contains")
                    {
                        let error = runtime
                            .eval_text(source)
                            .expect_err("shared reload eval must fail");
                        assert!(error.contains(marker), "{error}");
                    } else {
                        panic!("unsupported shared reload expectation")
                    }
                }
                "assert/revision" => {
                    let Some(Form::Number(expected)) = entry(step, "expect") else {
                        panic!(":assert/revision must declare numeric :expect")
                    };
                    assert_eq!(
                        runtime
                            .eval_text(&format!("(module-revision '{namespace})"))
                            .unwrap(),
                        expected.to_string()
                    );
                }
                other => panic!("unsupported shared reload operation :{other}"),
            }
        }
    }

    #[test]
    fn callable_var_scenarios_execute_from_shared_spec() {
        fn entry<'a>(entries: &'a [(Form, Form)], key: &str) -> Option<&'a Form> {
            entries.iter().find_map(|(candidate, value)| {
                matches!(candidate, Form::Keyword(name) if name == key).then_some(value)
            })
        }

        for id in [
            "namespace/callable-var-precedence",
            "namespace/callable-var-lexical-shadow",
            "namespace/callable-var-late-binding",
            "namespace/referred-var-protected",
        ] {
            let case = module_case(id);
            let Some(Form::String(setup)) = entry(&case, "setup") else {
                panic!(":{id} must declare string :setup")
            };
            let Some(Form::String(source)) = entry(&case, "source") else {
                panic!(":{id} must declare string :source")
            };
            let Some(Form::Map(expect)) = entry(&case, "expect") else {
                panic!(":{id} must declare :expect")
            };
            let mut runtime = Runtime::new();
            runtime
                .eval_text(setup)
                .unwrap_or_else(|error| panic!(":{id} setup failed: {error}"));
            if let Some(Form::String(expected)) = entry(expect, "display") {
                assert_eq!(
                    runtime
                        .eval_text(source)
                        .unwrap_or_else(|error| panic!(":{id} failed: {error}")),
                    *expected,
                    ":{id}"
                );
            } else if let Some(Form::String(marker)) = entry(expect, "error-contains") {
                let error = runtime
                    .eval_text(source)
                    .expect_err(&format!(":{id} must fail"));
                assert!(error.contains(marker), ":{id}: {error}");
            } else {
                panic!(":{id} has unsupported expectation")
            }
        }
    }

    #[test]
    fn issue_134_lazy_namespace_state_is_non_forcing_and_failure_is_sticky() {
        if repo_text("specs/00-unsorted/platform-language/draft/conformance/modules.edn").is_none() {
            return;
        }
        assert_eq!(
            module_expect("lazy/non-forcing", "state"),
            Form::Keyword("unloaded".into())
        );
        assert_eq!(
            module_expect("lazy/non-forcing", "target-evaluated"),
            Form::Bool(false)
        );
        assert_eq!(
            module_expect("lazy/qualified-force", "target-evaluations"),
            Form::Number(1)
        );
        assert_eq!(
            module_expect("lazy/failure-state", "state"),
            Form::Keyword("failed".into())
        );
        assert_eq!(
            module_expect("lazy/failure-state", "partial-state"),
            Form::Bool(false)
        );
        assert_eq!(
            module_expect("lazy/explicit-retry", "ordinary-force-retries"),
            Form::Bool(false)
        );
        assert_eq!(
            module_expect("lazy/explicit-retry", "reload-retries"),
            Form::Bool(true)
        );
        assert_eq!(
            module_expect("module/reload-revision", "revision-increment"),
            Form::Number(1)
        );
        assert_eq!(
            module_expect("module/reload-rollback", "previous-revision-preserved"),
            Form::Bool(true)
        );
        assert_eq!(
            module_expect("namespace/loading-state", "non-forcing"),
            Form::Bool(true)
        );
        assert_eq!(
            module_expect("namespace/cross-namespace-alias-state", "owner-explicit"),
            Form::Bool(true)
        );

        let mut runtime = Runtime::new();
        runtime.register_resource(
            "example.lazy",
            "(ns example.lazy) (def observed-state (ns-state 'example.lazy)) (def answer 42)",
        );

        assert_eq!(
            runtime
                .eval_text("(require [example.lazy :as lazy :lazy true])")
                .unwrap(),
            "nil"
        );
        assert_eq!(
            runtime.eval_text("(ns-state 'example.lazy)").unwrap(),
            ":unloaded"
        );
        assert_eq!(
            runtime
                .eval_text("(get (ns-alias-state 'lazy) :state)")
                .unwrap(),
            ":unloaded"
        );
        assert_eq!(runtime.eval_text("lazy/answer").unwrap(), "42");
        assert_eq!(
            runtime.eval_text("lazy/observed-state").unwrap(),
            ":loading"
        );
        assert_eq!(
            runtime.eval_text("(ns-state 'example.lazy)").unwrap(),
            ":loaded"
        );
        assert_eq!(
            runtime
                .eval_text("(module-revision 'example.lazy)")
                .unwrap(),
            "1"
        );

        runtime.register_resource("example.lazy", "(ns example.lazy) (def answer 43)");
        runtime
            .eval_text("(require [example.lazy :as lazy :reload true])")
            .unwrap();
        assert_eq!(runtime.eval_text("lazy/answer").unwrap(), "43");
        assert_eq!(
            runtime
                .eval_text("(module-revision 'example.lazy)")
                .unwrap(),
            "2"
        );

        runtime.register_resource(
            "example.lazy",
            "(ns example.lazy) (def answer 99) (def reload-leaked-134 1) (throw :reload-failed)",
        );
        assert!(runtime
            .eval_text("(require [example.lazy :as lazy :reload true])")
            .is_err());
        assert_eq!(runtime.eval_text("lazy/answer").unwrap(), "43");
        assert_eq!(
            runtime
                .eval_text("(module-revision 'example.lazy)")
                .unwrap(),
            "2"
        );
        assert_eq!(
            runtime.eval_text("(ns-state 'example.lazy)").unwrap(),
            ":loaded"
        );
        assert!(runtime
            .namespace_registry
            .find("example.lazy")
            .unwrap()
            .resolve(&crate::lang::data::Symbol::parse("reload-leaked-134"))
            .is_none());

        runtime.register_resource(
            "example.broken",
            "(ns example.broken) (def leaked 1) (throw :broken)",
        );
        runtime
            .eval_text("(require [example.broken :as broken :lazy true])")
            .unwrap();
        assert!(runtime.eval_text("broken/leaked").is_err());
        assert_eq!(
            runtime.eval_text("(ns-state 'example.broken)").unwrap(),
            ":failed"
        );
        assert_eq!(
            runtime
                .eval_text("(get (ns-alias-state 'broken) :state)")
                .unwrap(),
            ":failed"
        );
        assert!(runtime.namespace_registry.find("example.broken").is_none());
        assert_eq!(runtime.current_namespace(), "user");
        assert_eq!(
            runtime
                .namespace_registry
                .current()
                .lazy_target("broken")
                .map(|name| name.as_str().to_owned()),
            Some("example.broken".into())
        );

        runtime.register_resource("example.broken", "(ns example.broken) (def answer 42)");
        let sticky_error = runtime.eval_text("broken/answer").unwrap_err();
        assert!(
            sticky_error.contains("explicit reload"),
            "unexpected sticky lazy-load error: {sticky_error}"
        );
        assert!(
            sticky_error.contains("initial failure"),
            "sticky error should retain the initial failure detail: {sticky_error}"
        );
        runtime
            .eval_text("(require [example.broken :as broken :reload true])")
            .unwrap();
        assert_eq!(runtime.eval_text("broken/answer").unwrap(), "42");
        assert_eq!(
            runtime.eval_text("(ns-state 'example.broken)").unwrap(),
            ":loaded"
        );

        runtime.eval_text("(ns observer)").unwrap();
        assert_eq!(
            runtime
                .eval_text("(get (ns-alias-state 'user 'broken) :state)")
                .unwrap(),
            ":loaded"
        );

        let mut isolated = Runtime::new();
        assert_eq!(
            isolated.eval_text("(ns-state 'example.lazy)").unwrap(),
            ":unknown"
        );
    }

    #[test]
    fn issue_134_dependency_order_cycles_and_canonical_cache_are_transactional() {
        if repo_text("specs/00-unsorted/platform-language/draft/conformance/modules.edn").is_none() {
            return;
        }
        assert_eq!(
            module_expect("module/canonical-cache", "duplicate-evaluation"),
            Form::Bool(false)
        );
        assert_eq!(
            module_expect("module/dependency-order", "order"),
            Form::Keyword("dependency-first-source-order".into())
        );
        assert_eq!(
            module_expect("module/cycle-rollback", "partial-state"),
            Form::Bool(false)
        );
        assert_eq!(
            module_expect("module/failure-rollback", "revision-increment"),
            Form::Bool(false)
        );

        let mut runtime = Runtime::new();
        runtime.register_resource("graph.dependency", "(ns graph.dependency) (def value 41)");
        runtime.register_resource(
            "graph.root",
            concat!(
                "(ns graph.root) ",
                "(require [graph.dependency :as dependency]) ",
                "(def answer (+ dependency/value 1))"
            ),
        );

        runtime
            .eval_text("(require [graph.root :as graph])")
            .unwrap();
        assert_eq!(runtime.eval_text("graph/answer").unwrap(), "42");
        assert_eq!(
            runtime
                .eval_text("(module-revision 'graph.dependency)")
                .unwrap(),
            "1"
        );
        assert_eq!(
            runtime.eval_text("(module-revision 'graph.root)").unwrap(),
            "1"
        );
        assert_eq!(
            runtime
                .namespace_registry
                .module_dependencies("graph.root")
                .iter()
                .map(|name| name.as_str().to_owned())
                .collect::<Vec<_>>(),
            vec!["graph.dependency"]
        );

        runtime
            .eval_text("(require [graph.root :as graph])")
            .unwrap();
        assert_eq!(
            runtime.eval_text("(module-revision 'graph.root)").unwrap(),
            "1"
        );

        runtime.register_resource(
            "cycle.first",
            concat!(
                "(ns cycle.first) ",
                "(def leaked-first 1) ",
                "(require [cycle.second :as second])"
            ),
        );
        runtime.register_resource(
            "cycle.second",
            concat!(
                "(ns cycle.second) ",
                "(def leaked-second 2) ",
                "(require [cycle.first :as first])"
            ),
        );

        let cycle = runtime
            .eval_text("(require [cycle.first :as cycle])")
            .unwrap_err();
        assert!(cycle.contains("Cyclic namespace require"), "{cycle}");
        assert!(runtime.namespace_registry.find("cycle.first").is_none());
        assert!(runtime.namespace_registry.find("cycle.second").is_none());
        assert_eq!(
            runtime.eval_text("(module-revision 'cycle.first)").unwrap(),
            "0"
        );
        assert_eq!(
            runtime
                .eval_text("(module-revision 'cycle.second)")
                .unwrap(),
            "0"
        );
        assert!(runtime
            .namespace_registry
            .module_dependencies("cycle.first")
            .is_empty());
        assert!(runtime
            .namespace_registry
            .module_dependencies("cycle.second")
            .is_empty());

        runtime.register_resource(
            "failure.root",
            concat!(
                "(ns failure.root) ",
                "(require [graph.dependency :as dependency]) ",
                "(def leaked dependency/value) ",
                "(throw :failure)"
            ),
        );
        assert!(runtime
            .eval_text("(require [failure.root :as failure])")
            .is_err());
        assert!(runtime.namespace_registry.find("failure.root").is_none());
        assert!(runtime
            .namespace_registry
            .module_dependencies("failure.root")
            .is_empty());
    }

    #[test]
    fn issue_134_with_ns_uses_target_globals_and_restores_the_caller() {
        if repo_text("specs/00-unsorted/platform-language/draft/conformance/modules.edn").is_none() {
            return;
        }
        assert_eq!(
            module_expect("namespace/with-ns-success", "caller-restored"),
            Form::Bool(true)
        );
        assert_eq!(
            module_expect("namespace/with-ns-failure", "caller-restored"),
            Form::Bool(true)
        );
        assert_eq!(
            module_expect(
                "namespace/with-ns-lexical-isolation",
                "caller-locals-visible"
            ),
            Form::Bool(false)
        );

        let mut runtime = Runtime::new();
        runtime
            .eval_text("(ns target) (def answer 41) (ns user)")
            .unwrap();
        assert_eq!(
            runtime
                .eval_text("(with-ns 'target (def answer 42) answer)")
                .unwrap(),
            "42"
        );
        assert_eq!(runtime.current_namespace(), "user");
        assert_eq!(runtime.eval_text("target/answer").unwrap(), "42");

        assert!(runtime
            .eval_text("(with-ns 'target (throw :with-ns-failed))")
            .is_err());
        assert_eq!(runtime.current_namespace(), "user");

        assert!(runtime
            .eval_text("(let [caller-local 42] (with-ns 'target caller-local))")
            .is_err());
        assert_eq!(runtime.current_namespace(), "user");
    }

    #[test]
    fn issue_134_facade_vars_copy_roots_and_metadata_without_sharing_identity() {
        if repo_text("specs/00-unsorted/platform-language/draft/conformance/modules.edn").is_none() {
            return;
        }
        assert_eq!(
            module_expect("namespace/facade-var-copy", "same-var"),
            Form::Bool(false)
        );
        assert_eq!(
            module_expect("namespace/facade-var-copy", "copied-root"),
            Form::Bool(true)
        );
        assert_eq!(
            module_expect("namespace/facade-var-copy", "copied-metadata"),
            Form::Bool(true)
        );

        let mut runtime = Runtime::new();
        runtime
            .eval_text("(ns source) (def ^{:doc \"copied\"} answer 41)")
            .unwrap();
        runtime.eval_text("(ns target)").unwrap();
        assert_eq!(
            runtime.eval_text("(deref (var source/answer))").unwrap(),
            "41"
        );
        runtime
            .eval_text("(intern-var 'target 'answer (var source/answer))")
            .unwrap();
        let source = runtime
            .namespace_registry
            .find("source")
            .unwrap()
            .resolve(&crate::lang::data::Symbol::parse("answer"))
            .unwrap();
        let target = runtime
            .namespace_registry
            .find("target")
            .unwrap()
            .resolve(&crate::lang::data::Symbol::parse("answer"))
            .unwrap();

        assert!(!source.same_identity(&target));
        assert_eq!(source.deref_value(), target.deref_value());
        assert_eq!(source.metadata(), target.metadata());
    }

    #[test]
    fn issue_134_aliases_and_refers_share_live_var_identity() {
        if repo_text("specs/00-unsorted/platform-language/draft/conformance/modules.edn").is_none() {
            return;
        }
        assert_eq!(
            module_expect("namespace/alias-var-identity", "same-var"),
            Form::Bool(true)
        );
        assert_eq!(
            module_expect("namespace/alias-var-identity", "live-root"),
            Form::Bool(true)
        );
        assert_eq!(
            module_expect("namespace/refer-var-identity", "same-var"),
            Form::Bool(true)
        );
        assert_eq!(
            module_expect("namespace/refer-var-identity", "live-root"),
            Form::Bool(true)
        );

        let mut runtime = Runtime::new();
        runtime.register_resource("identity.source", "(ns identity.source) (def answer 41)");
        runtime
            .eval_text("(require [identity.source :as source :refer [answer]])")
            .unwrap();
        let source = runtime
            .namespace_registry
            .find("identity.source")
            .unwrap()
            .resolve(&crate::lang::data::Symbol::parse("answer"))
            .unwrap();
        let alias = runtime
            .namespace_registry
            .resolve(&crate::lang::data::Symbol::parse("source/answer"))
            .unwrap();
        let referred = runtime
            .namespace_registry
            .find("user")
            .unwrap()
            .resolve(&crate::lang::data::Symbol::parse("answer"))
            .unwrap();
        assert!(source.same_identity(&alias));
        assert!(source.same_identity(&referred));
        source.reset_value(core::Value::Number(42));
        assert_eq!(runtime.eval_text("source/answer").unwrap(), "42");
        assert_eq!(runtime.eval_text("answer").unwrap(), "42");
    }

    #[test]
    fn issue_134_macro_reload_only_changes_new_compilations() {
        if repo_text("specs/00-unsorted/platform-language/draft/conformance/modules.edn").is_none() {
            return;
        }
        assert_eq!(
            module_expect("macro/reload-new-compilation", "existing-call-target"),
            Form::Keyword("unchanged".into())
        );
        assert_eq!(
            module_expect("macro/reload-new-compilation", "new-compilation"),
            Form::Keyword("new-expansion".into())
        );

        let mut runtime = Runtime::new();
        runtime.register_resource(
            "reload.macros",
            "(ns reload.macros) (defmacro answer [] 41)",
        );
        runtime
            .eval_text(
                "(require [reload.macros :refer-macros [answer]]) \
                 (def compiled-before (macroexpand '(answer)))",
            )
            .unwrap();
        assert_eq!(runtime.eval_text("compiled-before").unwrap(), "41");

        runtime.register_resource(
            "reload.macros",
            "(ns reload.macros) (defmacro answer [] 42)",
        );
        runtime
            .eval_text("(require [reload.macros :reload true :refer-macros [answer]])")
            .unwrap();
        assert_eq!(runtime.eval_text("compiled-before").unwrap(), "41");
        assert_eq!(runtime.eval_text("(answer)").unwrap(), "42");
    }

    #[test]
    fn issue_134_session_namespace_module_and_macro_state_is_isolated() {
        if repo_text("specs/00-unsorted/platform-language/draft/conformance/modules.edn").is_none() {
            return;
        }
        assert_eq!(
            module_expect("session/namespace-isolation", "vars-shared"),
            Form::Bool(false)
        );
        assert_eq!(
            module_expect("session/namespace-isolation", "modules-shared"),
            Form::Bool(false)
        );
        assert_eq!(
            module_expect("session/namespace-isolation", "macros-shared"),
            Form::Bool(false)
        );

        let mut kernel = SessionKernel::new();
        kernel.register_resource(
            "session.module",
            "(ns session.module) (defmacro chosen [] 41) (def answer 41)",
        );
        kernel.create_session("alpha").unwrap();
        kernel.create_session("beta").unwrap();
        kernel
            .eval(
                "alpha",
                "(do (require [session.module :as module :refer-macros [chosen]]) \
                     (def local-answer (chosen)) nil)",
            )
            .unwrap();
        assert_eq!(kernel.eval("alpha", "local-answer").unwrap(), "41");
        assert!(kernel.eval("beta", "local-answer").is_err());
        assert_eq!(
            kernel
                .eval("alpha", "(module-revision 'session.module)")
                .unwrap(),
            "1"
        );
        assert_eq!(
            kernel
                .eval("beta", "(module-revision 'session.module)")
                .unwrap(),
            "0"
        );
        assert!(kernel.eval("beta", "(chosen)").is_err());
    }

    #[test]
    fn issue_134_source_and_hir_have_value_metadata_and_error_parity() {
        if repo_text("specs/00-unsorted/platform-language/draft/conformance/modules.edn").is_none() {
            return;
        }
        assert_eq!(
            module_expect("module/source-hir-parity", "same-value"),
            Form::Bool(true)
        );
        assert_eq!(
            module_expect("module/source-hir-parity", "same-var-metadata"),
            Form::Bool(true)
        );
        assert_eq!(
            module_expect("module/source-hir-parity", "same-error-category"),
            Form::Bool(true)
        );

        use crate::kernel::hir::encode_hir_module;

        let source = "(ns parity.demo) (defn value \"answer\" [] 42) (value)";
        let forms = kernel::parse_forms(source).unwrap();
        let artifact = encode_hir_module("parity.demo", "parity/demo.hal", source, forms);

        let mut source_runtime = Runtime::new();
        let mut hir_runtime = Runtime::new();
        assert_eq!(source_runtime.eval_text(source).unwrap(), "42");
        assert_eq!(hir_runtime.eval_hir(&artifact).unwrap(), "42");

        let source_var = source_runtime
            .namespace_registry
            .find("parity.demo")
            .unwrap()
            .resolve(&crate::lang::data::Symbol::parse("value"))
            .unwrap();
        let hir_var = hir_runtime
            .namespace_registry
            .find("parity.demo")
            .unwrap()
            .resolve(&crate::lang::data::Symbol::parse("value"))
            .unwrap();
        assert_eq!(source_var.metadata(), hir_var.metadata());

        let failing_source = "(throw :parity-failed)";
        let failing_artifact = encode_hir_module(
            "parity.failure",
            "parity/failure.hal",
            failing_source,
            kernel::parse_forms(failing_source).unwrap(),
        );
        let source_error = source_runtime.eval_text(failing_source).unwrap_err();
        let hir_error = hir_runtime.eval_hir(&failing_artifact).unwrap_err();
        assert!(source_error.contains("thrown: :parity-failed"));
        assert!(hir_error.contains("thrown: :parity-failed"));
    }

    #[test]
    fn issue_134_runtime_profile_declares_deterministic_resource_precedence() {
        if repo_text("specs/00-unsorted/platform-language/draft/conformance/modules.edn").is_none() {
            return;
        }
        assert_eq!(
            module_expect("module/resource-precedence", "deterministic"),
            Form::Bool(true)
        );
        assert_eq!(
            module_expect("module/resource-precedence", "declared-by-runtime-profile"),
            Form::Bool(true)
        );
        assert_eq!(
            module_runtime_profile("rust", "resource-order"),
            Form::Vector(vec![
                Form::Keyword("loaded-native-namespace".into()),
                Form::Keyword("registered-resource".into()),
                Form::Keyword("registered-extension".into()),
            ])
        );

        let mut runtime = Runtime::new();
        runtime.extensions.install(RangeExtension);
        runtime.register_resource(
            "range",
            "(def resource-precedence-marker 42) resource-precedence-marker",
        );
        assert_eq!(runtime.require_resource("range").unwrap(), "42");
        assert_eq!(runtime.require_resource("range").unwrap(), ":loaded");
        assert_eq!(
            runtime.eval_text("resource-precedence-marker").unwrap(),
            "42"
        );
    }

    #[test]
    fn issue_134_sessions_unwind_bindings_and_transfer_only_immutable_data() {
        if repo_text("specs/00-unsorted/platform-language/draft/conformance/modules.edn").is_none() {
            return;
        }
        assert_eq!(
            module_expect("session/dynamic-unwind", "binding-session-local"),
            Form::Bool(true)
        );
        assert_eq!(
            module_expect("session/dynamic-unwind", "restored-after-error"),
            Form::Bool(true)
        );
        assert_eq!(
            module_expect("session/immutable-transfer", "immutable-data"),
            Form::Bool(true)
        );
        for kind in [
            "functions",
            "vars",
            "mutable-references",
            "streams",
            "sockets",
            "host-handles",
        ] {
            assert_eq!(
                module_expect("session/reject-live-transfer", kind),
                Form::Bool(false)
            );
        }

        let mut kernel = SessionKernel::new();
        kernel.create_session("alpha").unwrap();
        kernel.create_session("beta").unwrap();
        assert_eq!(
            kernel
                .eval("alpha", "(do (def ^:dynamic *answer* 1) nil)")
                .unwrap(),
            "nil"
        );
        assert_eq!(
            kernel
                .eval("beta", "(do (def ^:dynamic *answer* 10) nil)")
                .unwrap(),
            "nil"
        );
        assert!(kernel
            .eval("alpha", "(binding [*answer* 2] (throw :binding-failed))")
            .is_err());
        assert_eq!(kernel.eval("alpha", "*answer*").unwrap(), "1");
        assert_eq!(kernel.eval("beta", "*answer*").unwrap(), "10");

        assert_eq!(
            kernel
                .eval("alpha", "{:answer [1 2 {:nested #{:immutable}}]}")
                .unwrap(),
            "{:answer [1 2 {:nested #{:immutable}}]}"
        );
        for source in [
            "(fn [value] value)",
            "(var *answer*)",
            "(atom 1)",
            "(iter [1 2 3])",
        ] {
            let error = kernel.eval("alpha", source).unwrap_err();
            assert!(
                error.contains("SESSION_TRANSFER_REJECTED"),
                "{source} unexpectedly produced {error}"
            );
        }
        assert!(!core::session_transferable(&core::Value::Extension(
            core::ExtensionValue {
                provider: "socket".into(),
                type_name: "Socket".into(),
                handle: 1,
            }
        )));
    }

    #[test]
    fn issue_134_retained_repl_state_survives_errors_and_multiline_forms() {
        if repo_text("specs/00-unsorted/platform-language/draft/conformance/modules.edn").is_none() {
            return;
        }
        assert_eq!(
            module_expect("repl/retained-state", "namespace-retained"),
            Form::Bool(true)
        );
        assert_eq!(
            module_expect("repl/retained-state", "multiline"),
            Form::Bool(true)
        );
        assert_eq!(
            module_expect("repl/error-recovery", "session-survives"),
            Form::Bool(true)
        );
        assert_eq!(
            module_expect("repl/error-recovery", "namespace-restored"),
            Form::Bool(true)
        );

        let mut kernel = SessionKernel::new();
        kernel.create_session("repl").unwrap();
        assert_eq!(
            kernel
                .eval(
                    "repl",
                    "(ns retained.repl)\n(def answer\n  (+ 40\n     2))\nnil"
                )
                .unwrap(),
            "nil"
        );
        assert!(kernel.eval("repl", "missing-symbol").is_err());
        assert_eq!(kernel.session_namespace("repl").unwrap(), "retained.repl");
        assert_eq!(kernel.eval("repl", "answer").unwrap(), "42");
    }

    #[test]
    fn issue_134_host_facades_are_loaded_session_local_and_non_transferable() {
        if repo_text("specs/00-unsorted/runtime/draft/host-runtime.edn").is_none() {
            return;
        }
        for id in [
            "host/type-identity",
            "host/session-local-facade",
            "host/namespace-loaded",
            "host/no-live-transfer",
            "host/rejected-ex-info",
        ] {
            assert!(!host_conformance_case(id).is_empty());
        }

        let mut first = Runtime::new();
        let second = Runtime::new();
        assert_eq!(
            first
                .eval_text("(= Host std.native.Host std.foundation/Host)")
                .unwrap(),
            "true"
        );
        assert_eq!(
            first.eval_text("(ns-state 'std.native)").unwrap(),
            ":loaded"
        );
        assert_eq!(
            first.eval_text("(ns-state 'std.native.Host)").unwrap(),
            ":loaded"
        );

        let first_host = first
            .namespace_registry
            .find("std.native")
            .unwrap()
            .resolve(&crate::lang::data::Symbol::parse("Host"))
            .unwrap()
            .deref_value();
        let second_host = second
            .namespace_registry
            .find("std.native")
            .unwrap()
            .resolve(&crate::lang::data::Symbol::parse("Host"))
            .unwrap()
            .deref_value();
        let (core::Value::NativeType(first_host), core::Value::NativeType(second_host)) =
            (first_host, second_host)
        else {
            panic!("Host must be a native façade descriptor")
        };
        assert!(!Rc::ptr_eq(&first_host, &second_host));

        let mut kernel = SessionKernel::new();
        kernel.create_session("host-transfer").unwrap();
        let error = kernel.eval("host-transfer", "Host").unwrap_err();
        assert!(error.contains("SESSION_TRANSFER_REJECTED"), "{error}");

        assert_eq!(
            first
                .eval_text(
                    "(try
                       (deref (Host/call \"missing\" \"missing\" []))
                       (catch error
                         [(ex-message error)
                          (get (ex-data error) :error/code)]))"
                )
                .unwrap(),
            "[\"Host capability provider is unavailable\" :host/unavailable]"
        );
        assert_eq!(
            first
                .eval_text(
                    "(deref
                       (promise/catch
                         (Host/call \"missing\" \"missing\" [])
                         (fn [error]
                           (get (ex-data error) :error/code))))"
                )
                .unwrap(),
            ":host/unavailable"
        );
        assert_eq!(
            kernel
                .eval(
                    "host-transfer",
                    "(try
                       (deref (Host/call \"missing\" \"missing\" []))
                       (catch error
                         (get (ex-data error) :error/code)))"
                )
                .unwrap(),
            ":host/unavailable"
        );
    }

    #[test]
    fn throw_and_try_catch_finally_are_host_neutral() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text("(try (throw :failed) (catch error error))")
                .unwrap(),
            ":failed"
        );
        assert_eq!(runtime.eval_text("(try 42 (finally 0))").unwrap(), "42");
        assert_eq!(
            runtime
                .eval_text("(try (throw :failed) (catch error (str error :handled)))")
                .unwrap(),
            "\":failed:handled\""
        );
        assert!(runtime
            .eval_text("(throw :failed)")
            .unwrap_err()
            .contains("thrown: :failed"));
    }

    #[test]
    fn def_binds_values_in_the_current_environment() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text("(do (def answer 41) (+ answer 1))")
                .unwrap(),
            "42"
        );
        assert_eq!(
            runtime
                .eval_text("(do (def answer 42) (deref (var answer)))")
                .unwrap(),
            "42"
        );
        assert!(runtime
            .eval_text("(deref 42)")
            .unwrap_err()
            .contains("deref expects a var"));
        assert_eq!(
            runtime
                .eval_text("(do (def answer 1) (def answer 42) answer)")
                .unwrap(),
            "42"
        );
        assert!(runtime
            .eval_text("(def 1 2)")
            .unwrap_err()
            .contains("def name must be a symbol"));
    }

    #[test]
    fn vars_preserve_identity_and_support_root_mutation() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text("(do (def answer 1) (= (var answer) (var answer)))")
                .unwrap(),
            "true"
        );
        assert_eq!(
            runtime
                .eval_text("(do (def answer 1) (set! answer 42) (deref (var answer)))")
                .unwrap(),
            "42"
        );
        assert_eq!(
            runtime
                .eval_text(
                    "(do (def answer 1) (let (v (var answer)) (do (set! answer 7) (deref v))))"
                )
                .unwrap(),
            "7"
        );
        assert_eq!(runtime.eval_text("(do (def answer 1) (defn add [x y] (+ x y)) (alter-var-root (var answer) add 40) answer)").unwrap(), "41");
        assert_eq!(
            runtime.eval_text("(assoc [1 2 3] 0 :x)").unwrap(),
            "[:x 2 3]"
        );
        assert_eq!(
            runtime.eval_text("(assoc [1 2 3] 3 :x)").unwrap(),
            "[1 2 3 :x]"
        );
        assert_eq!(
            runtime.eval_text("(assoc [1 2 3] 5 :x)").unwrap_err(),
            "assoc index out of bounds"
        );
        assert_eq!(
            runtime.eval_text("(set! missing 1)").unwrap_err(),
            "unbound var: missing"
        );
    }

    #[test]
    fn functions_capture_lexical_values_and_support_defn() {
        let mut runtime = Runtime::new();
        assert_eq!(runtime.eval_text("((fn [x] (+ x 1)) 41)").unwrap(), "42");
        assert_eq!(
            runtime
                .eval_text("(let (inc (fn [x] (+ x 1))) (inc 41))")
                .unwrap(),
            "42"
        );
        assert_eq!(
            runtime
                .eval_text("(do (defn add1 [x] (+ x 1)) (add1 41))")
                .unwrap(),
            "42"
        );
        assert_eq!(runtime.eval_text("(do (defn factorial [n] (if (<= n 1) 1 (* n (factorial (dec n))))) (factorial 5))").unwrap(), "120");
        assert_eq!(
            runtime
                .eval_text("(let (x 40) (let (f (fn [y] (+ x y))) (f 2)))")
                .unwrap(),
            "42"
        );
    }

    #[test]
    fn quote_lists_and_do_match_core_shapes() {
        let mut runtime = Runtime::new();
        assert_eq!(runtime.eval_text("'(1 2)").unwrap(), "(1 2)");
        assert_eq!(runtime.eval_text("(count '(1 2 3))").unwrap(), "3");
        assert_eq!(runtime.eval_text("(nth (cons 0 '(1 2)) 0)").unwrap(), "0");
        assert_eq!(runtime.eval_text("(do 1 2 3)").unwrap(), "3");
    }

    #[test]
    fn signed_32_bit_operations_match_core_contract() {
        let mut runtime = Runtime::new();
        assert_eq!(runtime.eval_text("(bit-and 6 3)").unwrap(), "2");
        assert_eq!(runtime.eval_text("(bit-or 1 2)").unwrap(), "3");
        assert_eq!(runtime.eval_text("(bit-xor 7 3)").unwrap(), "4");
        assert_eq!(runtime.eval_text("(bit-not 0)").unwrap(), "-1");
        assert_eq!(runtime.eval_text("(bit-shift-right -4 1)").unwrap(), "-2");
        assert_eq!(
            runtime.eval_text("(bit-shift-left 1 31)").unwrap(),
            "-2147483648"
        );
        assert!(runtime
            .eval_text("(bit-shift-left 1 -1)")
            .unwrap_err()
            .contains("distance must be in the range 0..31"));
    }

    #[test]
    fn l0_numeric_and_truth_predicates_are_available() {
        let mut runtime = Runtime::new();
        assert_eq!(runtime.eval_text("(inc 41)").unwrap(), "42");
        assert_eq!(runtime.eval_text("(dec 43)").unwrap(), "42");
        assert_eq!(runtime.eval_text("(zero? 0)").unwrap(), "true");
        assert_eq!(runtime.eval_text("(pos? 1)").unwrap(), "true");
        assert_eq!(runtime.eval_text("(neg? -1)").unwrap(), "true");
        assert_eq!(runtime.eval_text("(even? 4)").unwrap(), "true");
        assert_eq!(runtime.eval_text("(odd? 3)").unwrap(), "true");
        assert_eq!(runtime.eval_text("(nil? nil)").unwrap(), "true");
        assert_eq!(runtime.eval_text("(true? true)").unwrap(), "true");
        assert_eq!(runtime.eval_text("(false? false)").unwrap(), "true");
    }

    #[test]
    fn core_sequence_navigation_ranges_and_quantifiers() {
        let mut runtime = Runtime::new();
        assert_eq!(runtime.eval_text("(second [10 20 30])").unwrap(), "20");
        assert_eq!(runtime.eval_text("(not-empty [])").unwrap(), "nil");
        assert_eq!(runtime.eval_text("(not-empty [1])").unwrap(), "[1]");
        assert_eq!(runtime.eval_text("(range 3)").unwrap(), "<seq>");
        assert_eq!(
            runtime
                .eval_text("(vector? (map (fn [x] (+ x 1)) [1 2 3]))")
                .unwrap(),
            "true"
        );
        assert_eq!(
            runtime
                .eval_text("(first (map (fn [x] (+ x 1)) [1 2 3]))")
                .unwrap(),
            "2"
        );
        assert_eq!(runtime.eval_text("(count (range 2 5))").unwrap(), "3");
        assert_eq!(runtime.eval_text("(count (repeat 4 :x))").unwrap(), "4");
        assert_eq!(
            runtime
                .eval_text("(every? (fn [x] (pos? x)) [1 2 3])")
                .unwrap(),
            "true"
        );
        assert_eq!(
            runtime
                .eval_text("(any? (fn [x] (= x 2)) [1 2 3])")
                .unwrap(),
            "true"
        );
    }

    #[test]
    fn map_and_zip_support_multiple_collections() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text("(nth (map (fn [x y] (+ x y)) [1 2] [10 20]) 1)")
                .unwrap(),
            "22"
        );
        assert_eq!(
            runtime
                .eval_text("(count (map (fn [x y z] (+ x (+ y z))) [1 2] [10 20] [100 200]))")
                .unwrap(),
            "2"
        );
        assert_eq!(
            runtime
                .eval_text("(nth (zip [1 2] [:a :b] [true false]) 0)")
                .unwrap(),
            "[1 :a true]"
        );
        assert_eq!(
            runtime.eval_text("(count (zip [1 2 3] [:a :b]))").unwrap(),
            "2"
        );
    }

    #[test]
    fn lazy_iterator_generators_are_bounded_by_consumers() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime.eval_text("(count ((take 4) (repeat :x)))").unwrap(),
            "4"
        );
        assert_eq!(
            runtime.eval_text("(first ((drop 3) (repeat :x)))").unwrap(),
            ":x"
        );
        assert_eq!(
            runtime
                .eval_text("(Iter/iter-finite? (repeat :x))")
                .unwrap(),
            "false"
        );
        assert_eq!(
            runtime
                .eval_text("(count ((take 3) (repeatedly (constantly 7))))")
                .unwrap(),
            "3"
        );
        assert_eq!(
            runtime
                .eval_text("(count ((take 5) (iterate (fn [x] (+ x 2)) 0)))")
                .unwrap(),
            "5"
        );
        assert_eq!(
            runtime
                .eval_text(
                    "(count ((take 3) ((take-while (fn [x] (< x 10))) (iterate (fn [x] (+ x 2)) 0))))"
                )
                .unwrap(),
            "3"
        );
        assert_eq!(
            runtime
                .eval_text(
                    "(first ((take 2) ((drop-while (fn [x] (< x 4))) (iterate (fn [x] (+ x 2)) 0))))"
                )
                .unwrap(),
            "4"
        );
        assert_eq!(
            runtime
                .eval_text(
                    "(nth ((take 4) ((map (fn [x] (* x 2))) (iterate (fn [x] (+ x 1)) 0))) 3)"
                )
                .unwrap(),
            "6"
        );
        assert_eq!(
            runtime
                .eval_text(
                    "(first ((take 2) ((filter (fn [x] (even? x))) (iterate (fn [x] (+ x 1)) 0))))"
                )
                .unwrap(),
            "0"
        );
        assert_eq!(
            runtime
                .eval_text(
                    "(nth ((take 4) ((mapcat (fn [x] [x x])) (iterate (fn [x] (+ x 1)) 0))) 3)"
                )
                .unwrap(),
            "1"
        );
        assert_eq!(runtime.eval_text("(first ((take 2) ((keep (fn [x] (if (even? x) (* x 10) nil))) (iterate (fn [x] (+ x 1)) 0))))").unwrap(), "0");
        assert_eq!(
            runtime
                .eval_text(
                    "(nth ((take 3) (Iter/iter-zip (iterate (fn [x] (+ x 1)) 0) (repeat :x))) 2)"
                )
                .unwrap(),
            "[2 :x]"
        );
        assert_eq!(
            runtime
                .eval_text("(nth ((take 4) (Iter/iter-interleave (iterate (fn [x] (+ x 1)) 0) (repeat :x))) 3)")
                .unwrap(),
            ":x"
        );
        assert_eq!(
            runtime
                .eval_text("(nth ((take 3) ((partition-all 2) (iterate (fn [x] (+ x 1)) 0))) 2)")
                .unwrap(),
            "[4 5]"
        );
        assert_eq!(
            runtime
                .eval_text("(nth ((take 2) ((partition 2) (iterate (fn [x] (+ x 1)) 0))) 1)")
                .unwrap(),
            "[2 3]"
        );
        assert_eq!(
            runtime
                .eval_text("(first ((take 4) (iterate (fn [x] (+ x 2)) 0)))")
                .unwrap(),
            "0"
        );
        assert_eq!(runtime.eval_text("(second (repeat :x))").unwrap(), ":x");
        assert_eq!(
            runtime
                .eval_text("(first (rest (iterate (fn [x] (+ x 1)) 0)))")
                .unwrap(),
            "1"
        );
        assert_eq!(
            runtime
                .eval_text("(nth (take 4 (iterate (fn [x] (+ x 2)) 0)) 3)")
                .unwrap(),
            "6"
        );
    }

    #[test]
    fn function_combinators_capture_values_and_functions() {
        let mut runtime = Runtime::new();
        assert_eq!(runtime.eval_text("((constantly 42) 1 2 3)").unwrap(), "42");
        assert_eq!(
            runtime
                .eval_text("((complement (fn [x] (> x 2))) 1)")
                .unwrap(),
            "true"
        );
        assert_eq!(
            runtime
                .eval_text("((comp (fn [x] (+ x 1)) (fn [x] (* x 2))) 20)")
                .unwrap(),
            "41"
        );
        assert_eq!(
            runtime
                .eval_text("((comp (fn [x] (+ x 1)) (fn [x] (+ x 1)) (fn [x] (+ x 1))) 39)")
                .unwrap(),
            "42"
        );
        assert_eq!(
            runtime.eval_text("((comp inc inc inc inc) 38)").unwrap(),
            "42"
        );
    }

    #[test]
    fn public_map_doto_and_set_helpers_are_portable() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text(
                    "[(map-keys (fn [x] (+ x 1)) {1 :a 2 :b}) \
                     (map-vals (fn [x] (+ x 1)) {:a 1 :b 2}) \
                     (let [calls (atom 0) \
                           value (doto \
                                   (do (swap! calls (fn [x] (+ x 1))) (atom [])) \
                                   (swap! (fn [values item] (conj values item)) 1) \
                                   (swap! (fn [values item] (conj values item)) 2))] \
                       [(deref calls) (deref value)])]"
                )
                .unwrap(),
            "[{2 :a 3 :b} {:a 2 :b 3} [1 [1 2]]]"
        );
        assert_eq!(
            runtime
                .eval_text(
                    "(do \
                       (require [std.foundation.set :as set]) \
                       [(set/union #{1 2} #{2 3}) \
                        (set/intersection #{1 2 3} #{2 3 4} #{3 5}) \
                        (set/difference #{1 2 3} #{2} #{3}) \
                        (set/subset? #{1 2} #{1 2 3}) \
                        (set/superset? #{1 2 3} #{1 2}) \
                        (set/select odd? #{1 2 3 4})])"
                )
                .unwrap(),
            "[#{1 2 3} #{3} #{1} true true #{1 3}]"
        );
    }

    #[test]
    fn nested_associative_helpers_match_l0_shapes() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime.eval_text("(get-in {:a {:b 42}} [:a :b])").unwrap(),
            "42"
        );
        assert_eq!(
            runtime
                .eval_text("(get-in (object :a (object :b 42)) [:a :b])")
                .unwrap(),
            "42"
        );
        assert_eq!(runtime.eval_text("(get (object :a 7) :a)").unwrap(), "7");
        assert_eq!(
            runtime
                .eval_text("(get-in {:a {:b 42}} [:a :missing])")
                .unwrap(),
            "nil"
        );
        assert_eq!(
            runtime
                .eval_text("(get-in (assoc-in {} [:a :b] 42) [:a :b])")
                .unwrap(),
            "42"
        );
        assert_eq!(runtime.eval_text("(get {:a 3} :a)").unwrap(), "3");
        assert_eq!(
            runtime
                .eval_text("(get (update {:a 3} :a (fn [x] (+ x 2))) :a)")
                .unwrap(),
            "5"
        );
        assert_eq!(
            runtime
                .eval_text("(get-in (update-in {:a {:b 3}} [:a :b] (fn [x y] (+ x y)) 4) [:a :b])")
                .unwrap(),
            "7"
        );
        assert_eq!(
            runtime.eval_text("(get (assoc {} :a 1 :b 2) :b)").unwrap(),
            "2"
        );
    }

    #[test]
    fn opaque_extensions_use_compact_tagged_display() {
        let value = core::Value::Extension(core::ExtensionValue {
            provider: "math.tensor".into(),
            type_name: "tensor".into(),
            handle: 42,
        });
        assert_eq!(value.display(), "#ht[:handle 42]");
    }
    #[test]
    fn iterator_combinators_cover_core_shapes() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text("(count (take-while (fn [x] (< x 3)) (range 5)))")
                .unwrap(),
            "3"
        );
        assert_eq!(
            runtime
                .eval_text("(count (drop-while (fn [x] (< x 3)) (range 5)))")
                .unwrap(),
            "2"
        );
        assert_eq!(
            runtime
                .eval_text("(count (mapcat (fn [x] [x x]) [1 2]))")
                .unwrap(),
            "4"
        );
        assert_eq!(
            runtime
                .eval_text("(count (keep (fn [x] (if (even? x) (* x 10) nil)) [1 2 3 4]))")
                .unwrap(),
            "2"
        );
        assert_eq!(
            runtime
                .eval_text("(count (partition-all 2 [1 2 3]))")
                .unwrap(),
            "2"
        );
        assert_eq!(
            runtime.eval_text("(count (partition 2 [1 2 3]))").unwrap(),
            "1"
        );
        assert_eq!(
            runtime.eval_text("(count (interpose :x [1 2 3]))").unwrap(),
            "5"
        );
        assert_eq!(
            runtime
                .eval_text("(count (interleave [1 2] [:a :b]))")
                .unwrap(),
            "4"
        );
        assert_eq!(
            runtime
                .eval_text("(count (partition-pair [1 2 3]))")
                .unwrap(),
            "1"
        );
    }

    #[test]
    fn arithmetic() {
        let mut runtime = Runtime::new();
        assert_eq!(runtime.eval_text("(+ 19 23)").unwrap(), "42");
    }

    #[test]
    fn declare_noop() {
        let mut runtime = Runtime::new();
        assert_eq!(runtime.eval_text("(declare x)").unwrap(), "nil");
    }

    #[test]
    fn recur_cannot_escape_loop_or_function_boundaries() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime.eval_text("(recur 1)").unwrap_err(),
            "recur must be inside loop"
        );
        assert_eq!(
            runtime.eval_text("((fn [] (recur 1)))").unwrap_err(),
            "recur must be inside loop"
        );
    }

    #[test]
    fn loop_supports_binding_vectors_and_multiple_recur_values() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text("(loop [x 0 y 1] (if (< x 4) (recur (+ x 1) (+ y x)) y))")
                .unwrap(),
            "7"
        );
        assert!(runtime
            .eval_text("(loop [x 0 y 1] (recur 2))")
            .unwrap_err()
            .contains("loop recur arity mismatch"));
    }

    #[test]
    fn loop_and_recur_support_tail_recursive_bootstrap_forms() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text("(loop (x 0) (if (< x 5) (recur (+ x 1)) x))")
                .unwrap(),
            "5"
        );
        assert_eq!(
            runtime
                .eval_text("(loop (x 1) (do (if (< x 3) (recur (* x 2)) x)))")
                .unwrap(),
            "4"
        );
    }

    #[test]
    fn let_accepts_binding_vectors_and_multiple_sequential_pairs() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime.eval_text("(let [x 19 y 23] (+ x y))").unwrap(),
            "42"
        );
        assert_eq!(
            runtime.eval_text("(let (x 19 y (+ x 23)) y)").unwrap(),
            "42"
        );
        assert!(runtime
            .eval_text("(let [x 1 y] y)")
            .unwrap_err()
            .contains("name/value pairs"));
    }

    #[test]
    fn conditional_and_let() {
        let mut runtime = Runtime::new();
        // Var display is namespace-qualified, matching the JVM runtime
        // (issue #223).
        assert_eq!(
            runtime.eval_text("(defn rank [score] score)").unwrap(),
            "#'user/rank"
        );
        assert_eq!(
            runtime
                .eval_text("(let (x 19) (if true (+ x 23) 0))")
                .unwrap(),
            "42"
        );
        assert_eq!(
            runtime
                .eval_text("(cond false \"gold\" (>= 70 50) \"silver\" :else \"bronze\")")
                .unwrap(),
            "\"silver\""
        );
        assert_eq!(runtime.eval_text("(cond false 1)").unwrap(), "nil");
        assert!(runtime
            .eval_text("(cond true 1 false)")
            .unwrap_err()
            .contains("test/expression pairs"));
    }

    #[test]
    fn lesson_definition_cases_run_from_the_l0_conformance_corpus() {
        fn entry<'a>(entries: &'a [(Form, Form)], key: &str) -> &'a Form {
            entries
                .iter()
                .find_map(|(candidate, value)| match candidate {
                    Form::Keyword(name) if name == key => Some(value),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("missing :{key}"))
        }

        let Some(corpus) = repo_text("docs/docs/reference/l0-conformance.edn") else {
            return;
        };
        let manifest = kernel::parse_forms(&corpus).unwrap().remove(0);
        let Form::Map(manifest) = manifest else {
            panic!("conformance corpus must be a map")
        };
        let Form::Vector(cases) = entry(&manifest, "cases") else {
            panic!("conformance :cases must be a vector")
        };
        let mut runtime = Runtime::new();

        for id in ["compiler/defn-var", "runtime/cond-defined-function"] {
            let case = cases
                .iter()
                .find(|case| {
                    matches!(
                        case,
                        Form::Map(entries)
                            if matches!(entry(entries, "id"), Form::Keyword(name) if name == id)
                    )
                })
                .unwrap_or_else(|| panic!("missing conformance case :{id}"));
            let Form::Map(case) = case else {
                unreachable!()
            };
            let Form::String(source) = entry(case, "source") else {
                panic!(":{id} source must be a string")
            };
            let Form::Map(expect) = entry(case, "expect") else {
                panic!(":{id} expect must be a map")
            };
            let Form::String(expected) = entry(expect, "value") else {
                panic!(":{id} expected value must be a string")
            };
            let Form::Keyword(expected_type) = entry(expect, "type") else {
                panic!(":{id} expected type must be a keyword")
            };
            let expected = if expected_type == "string" {
                format!("{expected:?}")
            } else {
                expected.clone()
            };
            assert_eq!(runtime.eval_text(source).unwrap(), expected, ":{id}");
        }
    }

    #[test]
    fn errors_are_stable() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime.eval_text("unknown").unwrap_err(),
            "unbound symbol: unknown"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn native_error_traces_are_opt_in_and_nested() {
        let mut runtime = Runtime::new();
        assert_eq!(runtime.eval_native("(+ 19 23)").unwrap(), "42");
        assert_eq!(
            runtime.eval_native("unknown").unwrap_err(),
            "unbound symbol: unknown"
        );
        let error = runtime
            .eval_native_traced("(do (defn inner [] (/ 1 0)) (defn outer [] (inner)) (outer))")
            .unwrap_err();
        assert!(error.contains("[hara stack]"));
        assert!(error.contains("at inner"));
        assert!(error.contains("at outer"));
        assert_eq!(error.matches("[hara stack]").count(), 1);
    }
    #[test]
    fn runtime_metadata_round_trips_through_protocols_and_reader_literals() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime.eval_text("(ILookup/lookup (IObjType/meta (IObjType/with-meta [1] {:doc \"vector\"})) :doc)").unwrap(),
            "\"vector\""
        );
        assert_eq!(
            runtime
                .eval_text("(ILookup/lookup (IObjType/meta (quote ^{:doc \"quoted\"} [1])) :doc)")
                .unwrap(),
            "\"quoted\""
        );
    }
    #[test]
    fn typed_vars_preserve_definition_metadata_and_dynamic_binding_scope() {
        let mut runtime = Runtime::new();
        assert_eq!(runtime.eval_text("(do (def ^:dynamic *answer* 1) (binding [*answer* 42] (binding [*answer* 43] *answer*)))").unwrap(), "43");
        assert_eq!(runtime.eval_text("*answer*").unwrap(), "1");
        assert_eq!(
            runtime
                .eval_text("(ILookup/lookup (IObjType/meta (var *answer*)) :dynamic)")
                .unwrap(),
            "true"
        );
        assert_eq!(runtime.eval_text("(do (def ^{:doc \"answer doc\"} answer 42) (ILookup/lookup (IObjType/meta (var answer)) :doc))").unwrap(), "\"answer doc\"");
        assert!(runtime
            .eval_text("(do (def plain 1) (binding [plain 2] plain))")
            .unwrap_err()
            .contains("dynamic Var"));
        let err = runtime
            .eval_text("(do (def ^:dynamic *left* 1) (binding [*left* 2 plain 3] *left*))")
            .unwrap_err();
        eprintln!("ERROR: {err}");
        assert!(err.contains("dynamic Var") || err.contains("name must be"));
        assert_eq!(runtime.eval_text("*left*").unwrap(), "1");
    }
    #[test]
    fn coroutine_introspection_works_in_cli_path() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime.eval_text("(std.foundation.coroutine/status (std.foundation.coroutine/create (fn [x] x)))").unwrap(),
            ":suspended"
        );
        assert_eq!(
            runtime.eval_text("(std.foundation.coroutine/coroutine? (std.foundation.coroutine/create (fn [] 1)))").unwrap(),
            "true"
        );
        assert_eq!(
            runtime
                .eval_text("(std.foundation.coroutine/coroutine? 42)")
                .unwrap(),
            "false"
        );
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime.eval_text("(def c (std.foundation.coroutine/create (fn [] 1))) (std.foundation.coroutine/status (std.foundation.coroutine/close c))").unwrap(),
            ":dead"
        );
        assert!(runtime
            .eval_text("(std.foundation.coroutine/resume c)")
            .unwrap_err()
            .contains("cannot resume a dead coroutine"));
        assert!(runtime
            .eval_text("(std.foundation.coroutine/yield 1)")
            .unwrap_err()
            .contains("coroutine/yield used outside of a coroutine"));
        assert_eq!(
            runtime
                .eval_text("(std.foundation.coroutine/await (promise/run (fn [] 1)))")
                .unwrap(),
            "1"
        );
    }
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn coroutine_suspending_forms_error_in_traced_path() {
        let mut runtime = Runtime::new();
        assert!(runtime
            .eval_native_traced("(def c (std.foundation.coroutine/create (fn [] 1))) (std.foundation.coroutine/resume c)")
            .unwrap_err()
            .contains("fiber evaluator"));
        assert!(runtime
            .eval_native_traced("(std.foundation.coroutine/yield 1)")
            .unwrap_err()
            .contains("fiber evaluator"));
        assert!(runtime
            .eval_native_traced("(std.foundation.coroutine/await (promise (fn [] 1)))")
            .unwrap_err()
            .contains("fiber evaluator"));
    }
    #[test]
    fn fiber_cli_path_evaluates_coroutine_resume_and_yield() {
        let mut runtime = Runtime::new();
        runtime
            .eval_text("(require [std.foundation.coroutine :as c])")
            .unwrap();
        assert_eq!(
            runtime.eval_text("(do (def co (c/create (fn [x] (let [y (c/yield (* x 2))] (+ y 1))))) (c/resume co 21))").unwrap(),
            "42"
        );
        assert_eq!(runtime.eval_text("(c/resume co 20)").unwrap(), "21");
    }
    #[test]
    fn binding_forms_evaluate_multiple_body_expressions() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text("(let [a (array 1 2 3)] (. a (push-last 4)) (. a (get 3)))")
                .unwrap(),
            "4"
        );
        assert_eq!(
            runtime
                .eval_text("(loop [n 0] (+ n 1) (if (< n 2) (recur (+ n 1)) n))")
                .unwrap(),
            "2"
        );
    }
    #[test]
    fn fiber_cli_path_awaits_promise_inside_coroutine() {
        let mut runtime = Runtime::new();
        runtime
            .eval_text("(require [std.foundation.coroutine :as c])")
            .unwrap();
        assert_eq!(
            runtime
                .eval_text(
                    "(def co (c/create (fn [] (c/await (promise/run (fn [] 42)))))) (c/resume co)"
                )
                .unwrap(),
            "42"
        );
    }
    #[test]
    fn coroutine_namespace_can_be_required_and_aliased() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text("(require 'std.foundation.coroutine) :loaded")
                .unwrap(),
            ":loaded"
        );
        assert_eq!(
            runtime
                .eval_text("(coroutine/status (coroutine/create (fn [x] x)))")
                .unwrap(),
            ":suspended"
        );
        assert_eq!(
            runtime.eval_text("(require [std.foundation.coroutine :as co]) (co/coroutine? (co/create (fn [] 1)))").unwrap(),
            "true"
        );
    }
    #[test]
    fn coroutine_default_alias_is_co() {
        let mut runtime = Runtime::new();
        assert_eq!(
            runtime
                .eval_text("(require 'std.foundation.coroutine) (co/status (co/create (fn [] 1)))")
                .unwrap(),
            ":suspended"
        );
    }
    #[test]
    fn eval_hir_runs_encoded_library() {
        use crate::kernel::hir::encode_hir_module;

        let source = "(ns demo)\n(def answer 42)\nanswer";
        let forms = kernel::parse_forms(source).unwrap();
        let artifact = encode_hir_module("demo", "demo.hal", source, forms);
        let mut runtime = Runtime::new();
        assert_eq!(runtime.eval_hir(&artifact).unwrap(), "42");
    }

    #[test]
    #[ignore = "requires a Truffle-compiled foundation HIR artifact"]
    fn truffle_compiled_foundation_hir_loads_with_foundation_semantics() {
        let artifact = std::env::var("HARA_TRUFFLE_FOUNDATION_HIR")
            .expect("HARA_TRUFFLE_FOUNDATION_HIR must point to the compiled artifact");
        let bytes = std::fs::read(&artifact).expect("read Truffle-compiled foundation HIR");
        let mut runtime = Runtime::new();

        assert_eq!(runtime.eval_hir(&bytes).unwrap(), "<fn>");
        assert_eq!(runtime.eval_native("((comp inc inc) 40)").unwrap(), "42");
    }

    #[cfg(feature = "dev-trace")]
    #[test]
    fn development_trace_uses_the_real_macro_and_invocation_paths() {
        let mut runtime = Runtime::new();
        let trace = runtime
            .eval_native_trace("(defn observed [x] x) (if-not false (observed 5))")
            .unwrap();

        assert_eq!(trace.schema, crate::trace::SCHEMA);
        assert_eq!(trace.result.as_ref().unwrap().display, "5");
        assert!(trace.events.iter().any(|event| {
            event.kind == crate::trace::TraceEventKind::MacroExpand
                && event.function.as_deref() == Some("if-not")
        }));
        assert!(trace.events.iter().any(|event| {
            event.kind == crate::trace::TraceEventKind::OperationEnter
                && event.function.as_deref() == Some("observed")
                && event
                    .values
                    .first()
                    .is_some_and(|value| value.display == "5")
        }));
        assert!(trace.events.iter().any(|event| {
            event.kind == crate::trace::TraceEventKind::OperationReturn
                && event.function.as_deref() == Some("observed")
                && event
                    .values
                    .first()
                    .is_some_and(|value| value.display == "5")
        }));
    }
}
