#![cfg(not(target_arch = "wasm32"))]

//! Stateful, RESP-facing coordination fabric.
//!
//! Spaces are authority/routing boundaries, sessions are isolated Hara
//! runtimes, namespaces are installed extension APIs, and reports are the
//! bounded message path between sessions. The implementation deliberately
//! keeps routing and storage behind this host boundary so guest WASM receives
//! no ambient network or filesystem authority.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};

use crate::extension::ExtensionManifest;
use crate::native_cli::RuntimeBroker;

pub const MAX_REPORT_BYTES: usize = 1024 * 1024;
pub const MAX_MODULE_BYTES: usize = 32 * 1024 * 1024;
pub const DEFAULT_MAILBOX_CAPACITY: usize = 256;
pub const DEFAULT_EVENT_CAPACITY: usize = 10_000;

#[derive(Clone, Debug)]
pub struct ServiceConfig {
    pub shards: usize,
    pub data_dir: Option<PathBuf>,
    pub node_id: String,
    pub peers: Vec<String>,
    pub cluster_epoch: String,
    pub mailbox_capacity: usize,
    pub event_capacity: usize,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            shards: std::thread::available_parallelism()
                .map(usize::from)
                .unwrap_or(1)
                .max(1),
            data_dir: None,
            node_id: "local".into(),
            peers: Vec::new(),
            cluster_epoch: "local.v1".into(),
            mailbox_capacity: DEFAULT_MAILBOX_CAPACITY,
            event_capacity: DEFAULT_EVENT_CAPACITY,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    pub id: String,
    pub space: String,
    pub source: String,
    pub target: String,
    pub signal: String,
    pub data: Vec<u8>,
    pub timestamp_ms: u64,
    pub sequence: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReportReceipt {
    pub id: String,
    pub delivered: usize,
    pub retained: bool,
    pub sequence: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnalyticsEvent {
    pub cursor: u64,
    pub timestamp_ms: u64,
    pub kind: String,
    pub space: Option<String>,
    pub session: Option<String>,
    pub detail: BTreeMap<String, String>,
}

impl AnalyticsEvent {
    pub fn json(&self) -> String {
        let mut fields = vec![
            format!("\"cursor\":{}", self.cursor),
            format!("\"timestamp_ms\":{}", self.timestamp_ms),
            format!("\"kind\":\"{}\"", json_escape(&self.kind)),
        ];
        if let Some(space) = &self.space {
            fields.push(format!("\"space\":\"{}\"", json_escape(space)));
        }
        if let Some(session) = &self.session {
            fields.push(format!("\"session\":\"{}\"", json_escape(session)));
        }
        let detail = self
            .detail
            .iter()
            .map(|(key, value)| format!("\"{}\":\"{}\"", json_escape(key), json_escape(value)))
            .collect::<Vec<_>>()
            .join(",");
        fields.push(format!("\"detail\":{{{detail}}}"));
        format!("{{{}}}", fields.join(","))
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Metrics {
    pub spaces: u64,
    pub sessions: u64,
    pub artifacts: u64,
    pub loaded_modules: u64,
    pub subscriptions: u64,
    pub reports_accepted: u64,
    pub reports_delivered: u64,
    pub reports_retained: u64,
    pub report_bytes: u64,
    pub calls: u64,
    pub errors: u64,
}

impl Metrics {
    pub fn json(&self) -> String {
        format!(
            concat!(
                "{{\"spaces\":{},\"sessions\":{},\"artifacts\":{},",
                "\"loaded_modules\":{},\"subscriptions\":{},",
                "\"reports_accepted\":{},\"reports_delivered\":{},",
                "\"reports_retained\":{},\"report_bytes\":{},",
                "\"calls\":{},\"errors\":{}}}"
            ),
            self.spaces,
            self.sessions,
            self.artifacts,
            self.loaded_modules,
            self.subscriptions,
            self.reports_accepted,
            self.reports_delivered,
            self.reports_retained,
            self.report_bytes,
            self.calls,
            self.errors
        )
    }
}

#[derive(Clone)]
pub struct FabricService {
    inner: Arc<Inner>,
}

struct Inner {
    shards: Vec<RuntimeBroker>,
    state: Mutex<State>,
    retention: Mutex<RetentionStore>,
    router: HomeRouter,
    mailbox_capacity: usize,
    event_capacity: usize,
}

struct State {
    spaces: HashMap<String, Space>,
    artifacts: HashMap<String, Artifact>,
    subscriptions: HashMap<String, Subscription>,
    retention: HashMap<(String, String, String), RetentionPolicy>,
    metrics: Metrics,
    events: VecDeque<AnalyticsEvent>,
    next_event: u64,
    next_report: u64,
    next_subscription: u64,
}

struct Space {
    sessions: HashMap<String, Session>,
}

struct Session {
    runtime_name: String,
    shard: usize,
    modules: HashSet<String>,
}

struct Artifact {
    manifest_source: String,
    manifest: ExtensionManifest,
    compiled: crate::wasmtime_provider::CompiledWasmModule,
}

struct Subscription {
    space: String,
    session: String,
    signal: String,
    queue: VecDeque<Report>,
}

#[derive(Clone, Copy)]
struct RetentionPolicy {
    max_events: usize,
    max_age_ms: u64,
}

#[derive(Clone, Debug)]
pub struct HomeRouter {
    node_id: String,
    members: Vec<String>,
    epoch: String,
}

impl HomeRouter {
    pub fn new(node_id: String, peers: Vec<String>, epoch: String) -> Result<Self, String> {
        validate_name(&node_id, "route/node")?;
        let mut members = peers;
        members.push(node_id.clone());
        members.sort();
        members.dedup();
        if members
            .iter()
            .any(|member| validate_name(member, "route/node").is_err())
        {
            return Err("route/node: invalid peer id".into());
        }
        Ok(Self {
            node_id,
            members,
            epoch,
        })
    }

    pub fn home(&self, space: &str, session: &str) -> &str {
        self.members
            .iter()
            .max_by_key(|member| rendezvous_score(&self.epoch, member, space, session))
            .map(String::as_str)
            .unwrap_or(&self.node_id)
    }

    pub fn local(&self, space: &str, session: &str) -> bool {
        self.home(space, session) == self.node_id
    }

    pub fn members(&self) -> &[String] {
        &self.members
    }
}

impl FabricService {
    pub fn start(config: ServiceConfig) -> Result<Self, String> {
        let shard_count = config.shards.max(1);
        let mut shards = Vec::with_capacity(shard_count);
        for _ in 0..shard_count {
            shards.push(RuntimeBroker::start()?);
        }
        let retention = match &config.data_dir {
            Some(path) => RetentionStore::open(path)?,
            None => RetentionStore::memory()?,
        };
        let router = HomeRouter::new(config.node_id, config.peers, config.cluster_epoch)?;
        let mut spaces = HashMap::new();
        spaces.insert(
            "default".into(),
            Space {
                sessions: HashMap::from([(
                    "ROOT".into(),
                    Session {
                        runtime_name: "ROOT".into(),
                        shard: 0,
                        modules: HashSet::new(),
                    },
                )]),
            },
        );
        let service = Self {
            inner: Arc::new(Inner {
                shards,
                state: Mutex::new(State {
                    spaces,
                    artifacts: HashMap::new(),
                    subscriptions: HashMap::new(),
                    retention: HashMap::new(),
                    metrics: Metrics {
                        spaces: 1,
                        sessions: 1,
                        ..Metrics::default()
                    },
                    events: VecDeque::new(),
                    next_event: 0,
                    next_report: 0,
                    next_subscription: 0,
                }),
                retention: Mutex::new(retention),
                router,
                mailbox_capacity: config.mailbox_capacity.max(1),
                event_capacity: config.event_capacity.max(1),
            }),
        };
        service.event(
            "service/started",
            None,
            None,
            [
                ("shards", shard_count.to_string()),
                ("members", service.inner.router.members().len().to_string()),
            ],
        );
        Ok(service)
    }

    pub fn create_space(&self, name: &str) -> Result<(), String> {
        validate_name(name, "space/invalid")?;
        let mut state = self.state()?;
        if state.spaces.contains_key(name) {
            return fail(&mut state, format!("space/already-exists: {name}"));
        }
        state.spaces.insert(
            name.into(),
            Space {
                sessions: HashMap::new(),
            },
        );
        state.metrics.spaces += 1;
        self.push_event(&mut state, "space/created", Some(name), None, []);
        drop(state);
        if let Err(error) = self.create_session(name, "ROOT") {
            let mut state = self.state()?;
            state.spaces.remove(name);
            refresh_gauges(&mut state);
            return Err(error);
        }
        Ok(())
    }

    pub fn drop_space(&self, name: &str) -> Result<(), String> {
        if name == "default" {
            return Err("space/root-protected: default".into());
        }
        let sessions = {
            let state = self.state()?;
            state
                .spaces
                .get(name)
                .ok_or_else(|| format!("space/not-found: {name}"))?
                .sessions
                .keys()
                .cloned()
                .collect::<Vec<_>>()
        };
        for session in sessions {
            self.close_session(name, &session)?;
        }
        let mut state = self.state()?;
        state.spaces.remove(name);
        state.metrics.spaces = state.metrics.spaces.saturating_sub(1);
        self.push_event(&mut state, "space/dropped", Some(name), None, []);
        Ok(())
    }

    pub fn list_spaces(&self) -> Result<Vec<String>, String> {
        let state = self.state()?;
        let mut names = state.spaces.keys().cloned().collect::<Vec<_>>();
        names.sort();
        Ok(names)
    }

    pub fn create_session(&self, space: &str, name: &str) -> Result<(), String> {
        validate_name(name, "session/invalid")?;
        if !self.inner.router.local(space, name) {
            return Err(format!(
                "route/home-unavailable: {}",
                self.inner.router.home(space, name)
            ));
        }
        let (runtime_name, shard) = {
            let mut state = self.state()?;
            let target_space = state
                .spaces
                .get(space)
                .ok_or_else(|| format!("space/not-found: {space}"))?;
            if target_space.sessions.contains_key(name) {
                return fail(&mut state, format!("session/already-exists: {name}"));
            }
            let shard = shard_for(space, name, self.inner.shards.len());
            (runtime_name(space, name), shard)
        };
        self.inner.shards[shard].create(&runtime_name)?;
        let mut state = self.state()?;
        let target_space = state
            .spaces
            .get_mut(space)
            .ok_or_else(|| format!("space/not-found: {space}"))?;
        target_space.sessions.insert(
            name.into(),
            Session {
                runtime_name,
                shard,
                modules: HashSet::new(),
            },
        );
        state.metrics.sessions += 1;
        self.push_event(
            &mut state,
            "session/created",
            Some(space),
            Some(name),
            [
                ("shard", shard.to_string()),
                ("home", self.inner.router.home(space, name).to_owned()),
            ],
        );
        Ok(())
    }

    pub fn close_session(&self, space: &str, name: &str) -> Result<(), String> {
        if space == "default" && name == "ROOT" {
            return Err("session/root-protected: default/ROOT".into());
        }
        let session = {
            let mut state = self.state()?;
            let target_space = state
                .spaces
                .get_mut(space)
                .ok_or_else(|| format!("space/not-found: {space}"))?;
            target_space
                .sessions
                .remove(name)
                .ok_or_else(|| format!("session/not-found: {name}"))?
        };
        self.inner.shards[session.shard].close(&session.runtime_name)?;
        let mut state = self.state()?;
        state.subscriptions.retain(|_, subscription| {
            !(subscription.space == space && subscription.session == name)
        });
        refresh_gauges(&mut state);
        self.push_event(&mut state, "session/closed", Some(space), Some(name), []);
        Ok(())
    }

    pub fn list_sessions(&self, space: &str) -> Result<Vec<String>, String> {
        let state = self.state()?;
        let mut names = state
            .spaces
            .get(space)
            .ok_or_else(|| format!("space/not-found: {space}"))?
            .sessions
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        names.sort();
        Ok(names)
    }

    pub fn eval(&self, space: &str, session: &str, source: &str) -> Result<String, String> {
        let (shard, runtime_name) = self.session_location(space, session)?;
        let started = now_ms();
        let result = self.inner.shards[shard].eval(&runtime_name, source);
        let elapsed = now_ms().saturating_sub(started);
        self.record_call(
            space,
            session,
            "eval",
            source.len(),
            elapsed,
            result.is_err(),
        );
        result
    }

    pub fn put_module(&self, manifest_source: &str, module: &[u8]) -> Result<String, String> {
        if module.len() > MAX_MODULE_BYTES {
            return Err(format!("module/too-large: {}", module.len()));
        }
        let manifest = ExtensionManifest::parse(manifest_source, "MODULE PUT")?;
        if manifest.provider != "wasm" {
            return Err("module/provider: service uploads require :provider :wasm".into());
        }
        let compiled = crate::wasmtime_provider::CompiledWasmModule::compile(module)?;
        let mut hasher = Sha256::new();
        hasher.update(manifest_source.as_bytes());
        hasher.update([0]);
        hasher.update(module);
        let digest = format!("sha256:{}", hex(&hasher.finalize()));
        let mut state = self.state()?;
        state.artifacts.entry(digest.clone()).or_insert(Artifact {
            manifest_source: manifest_source.into(),
            manifest,
            compiled,
        });
        refresh_gauges(&mut state);
        self.push_event(
            &mut state,
            "module/stored",
            None,
            None,
            [
                ("digest", digest.clone()),
                ("bytes", module.len().to_string()),
            ],
        );
        Ok(digest)
    }

    pub fn load_module(&self, space: &str, session: &str, digest: &str) -> Result<String, String> {
        let (shard, runtime_name) = self.session_location(space, session)?;
        let (source, compiled, namespace) = {
            let state = self.state()?;
            let artifact = state
                .artifacts
                .get(digest)
                .ok_or_else(|| format!("module/not-found: {digest}"))?;
            (
                artifact.manifest_source.clone(),
                artifact.compiled.clone(),
                artifact.manifest.namespace.clone(),
            )
        };
        self.inner.shards[shard].install_module(&runtime_name, &source, &compiled)?;
        let mut state = self.state()?;
        state
            .spaces
            .get_mut(space)
            .and_then(|space| space.sessions.get_mut(session))
            .ok_or_else(|| format!("session/not-found: {session}"))?
            .modules
            .insert(namespace.clone());
        refresh_gauges(&mut state);
        self.push_event(
            &mut state,
            "module/loaded",
            Some(space),
            Some(session),
            [
                ("digest", digest.to_owned()),
                ("namespace", namespace.clone()),
            ],
        );
        Ok(namespace)
    }

    pub fn invoke_module(
        &self,
        space: &str,
        session: &str,
        qualified: &str,
        arguments: &[u8],
    ) -> Result<Vec<u8>, String> {
        let (namespace, export) = qualified
            .rsplit_once('/')
            .ok_or_else(|| "module/call: expected namespace/export".to_owned())?;
        let (shard, runtime_name) = self.session_location(space, session)?;
        let started = now_ms();
        let result =
            self.inner.shards[shard].invoke_module(&runtime_name, namespace, export, arguments);
        let elapsed = now_ms().saturating_sub(started);
        self.record_call(
            space,
            session,
            qualified,
            arguments.len(),
            elapsed,
            result.is_err(),
        );
        result
    }

    pub fn retain(
        &self,
        space: &str,
        session: &str,
        signal: &str,
        max_events: usize,
        max_age_ms: u64,
    ) -> Result<(), String> {
        validate_signal(signal)?;
        self.session_location(space, session)?;
        let mut state = self.state()?;
        state.retention.insert(
            (space.into(), session.into(), signal.into()),
            RetentionPolicy {
                max_events: max_events.max(1),
                max_age_ms: max_age_ms.max(1),
            },
        );
        self.push_event(
            &mut state,
            "retention/configured",
            Some(space),
            Some(session),
            [
                ("signal", signal.to_owned()),
                ("max_events", max_events.max(1).to_string()),
                ("max_age_ms", max_age_ms.max(1).to_string()),
            ],
        );
        Ok(())
    }

    pub fn subscribe(&self, space: &str, session: &str, signal: &str) -> Result<String, String> {
        validate_signal(signal)?;
        self.session_location(space, session)?;
        let mut state = self.state()?;
        state.next_subscription += 1;
        let id = format!("sub-{}", state.next_subscription);
        state.subscriptions.insert(
            id.clone(),
            Subscription {
                space: space.into(),
                session: session.into(),
                signal: signal.into(),
                queue: VecDeque::new(),
            },
        );
        refresh_gauges(&mut state);
        self.push_event(
            &mut state,
            "report/subscribed",
            Some(space),
            Some(session),
            [("subscription", id.clone()), ("signal", signal.to_owned())],
        );
        Ok(id)
    }

    pub fn unsubscribe(&self, id: &str) -> Result<bool, String> {
        let mut state = self.state()?;
        let removed = state.subscriptions.remove(id).is_some();
        refresh_gauges(&mut state);
        if removed {
            self.push_event(
                &mut state,
                "report/unsubscribed",
                None,
                None,
                [("subscription", id.to_owned())],
            );
        }
        Ok(removed)
    }

    pub fn next_report(&self, subscription: &str) -> Result<Option<Report>, String> {
        let mut state = self.state()?;
        state
            .subscriptions
            .get_mut(subscription)
            .ok_or_else(|| format!("report/subscription-not-found: {subscription}"))
            .map(|subscription| subscription.queue.pop_front())
    }

    pub fn send_report(
        &self,
        space: &str,
        source: &str,
        target: &str,
        signal: &str,
        data: &[u8],
    ) -> Result<ReportReceipt, String> {
        validate_signal(signal)?;
        if data.len() > MAX_REPORT_BYTES {
            return Err(format!("report/too-large: {}", data.len()));
        }
        self.session_location(space, source)?;
        self.session_location(space, target)?;
        let mut state = self.state()?;
        let matching = state
            .subscriptions
            .iter()
            .filter(|(_, subscription)| {
                subscription.space == space
                    && subscription.session == target
                    && subscription.signal == signal
            })
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        if matching.iter().any(|id| {
            state
                .subscriptions
                .get(id)
                .is_some_and(|subscription| subscription.queue.len() >= self.inner.mailbox_capacity)
        }) {
            return fail(
                &mut state,
                format!("report/backpressure: {space}/{target}/{signal}"),
            );
        }
        state.next_report += 1;
        let timestamp_ms = now_ms();
        let id = format!("rpt-{timestamp_ms}-{}", state.next_report);
        let policy = state
            .retention
            .get(&(space.into(), target.into(), signal.into()))
            .copied();
        let mut report = Report {
            id: id.clone(),
            space: space.into(),
            source: source.into(),
            target: target.into(),
            signal: signal.into(),
            data: data.into(),
            timestamp_ms,
            sequence: None,
        };
        if let Some(policy) = policy {
            let sequence = self
                .inner
                .retention
                .lock()
                .map_err(|_| "service/lock-poisoned".to_owned())?
                .append(&report, policy)?;
            report.sequence = Some(sequence);
        }
        for id in &matching {
            state
                .subscriptions
                .get_mut(id)
                .expect("matching subscription")
                .queue
                .push_back(report.clone());
        }
        state.metrics.reports_accepted += 1;
        state.metrics.reports_delivered += matching.len() as u64;
        state.metrics.report_bytes += data.len() as u64;
        if policy.is_some() {
            state.metrics.reports_retained += 1;
        }
        self.push_event(
            &mut state,
            "report/accepted",
            Some(space),
            Some(target),
            [
                ("report", id.clone()),
                ("source", source.to_owned()),
                ("signal", signal.to_owned()),
                ("bytes", data.len().to_string()),
                ("delivered", matching.len().to_string()),
                ("retained", policy.is_some().to_string()),
            ],
        );
        Ok(ReportReceipt {
            id,
            delivered: matching.len(),
            retained: policy.is_some(),
            sequence: report.sequence,
        })
    }

    pub fn replay(
        &self,
        space: &str,
        session: &str,
        signal: &str,
        after: u64,
        limit: usize,
    ) -> Result<Vec<Report>, String> {
        self.inner
            .retention
            .lock()
            .map_err(|_| "service/lock-poisoned".to_owned())?
            .replay(space, session, signal, after, limit.min(10_000))
    }

    pub fn metrics(&self) -> Result<Metrics, String> {
        Ok(self.state()?.metrics.clone())
    }

    pub fn events_since(&self, cursor: u64, limit: usize) -> Result<Vec<AnalyticsEvent>, String> {
        Ok(self
            .state()?
            .events
            .iter()
            .filter(|event| event.cursor > cursor)
            .take(limit.min(10_000))
            .cloned()
            .collect())
    }

    pub fn topology_json(&self) -> Result<String, String> {
        let state = self.state()?;
        let spaces = state
            .spaces
            .iter()
            .map(|(space_name, space)| {
                let sessions = space
                    .sessions
                    .iter()
                    .map(|(session_name, session)| {
                        let modules = session
                            .modules
                            .iter()
                            .map(|module| format!("\"{}\"", json_escape(module)))
                            .collect::<Vec<_>>()
                            .join(",");
                        format!(
                            "{{\"id\":\"{}\",\"shard\":{},\"home\":\"{}\",\"namespaces\":[{}]}}",
                            json_escape(session_name),
                            session.shard,
                            json_escape(self.inner.router.home(space_name, session_name)),
                            modules
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                format!(
                    "{{\"id\":\"{}\",\"sessions\":[{}]}}",
                    json_escape(space_name),
                    sessions
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        Ok(format!(
            "{{\"node\":\"{}\",\"spaces\":[{}]}}",
            json_escape(&self.inner.router.node_id),
            spaces
        ))
    }

    fn session_location(&self, space: &str, session: &str) -> Result<(usize, String), String> {
        let state = self.state()?;
        let session = state
            .spaces
            .get(space)
            .ok_or_else(|| format!("space/not-found: {space}"))?
            .sessions
            .get(session)
            .ok_or_else(|| format!("session/not-found: {session}"))?;
        Ok((session.shard, session.runtime_name.clone()))
    }

    fn record_call(
        &self,
        space: &str,
        session: &str,
        operation: &str,
        bytes: usize,
        elapsed_ms: u64,
        error: bool,
    ) {
        let Ok(mut state) = self.state() else { return };
        state.metrics.calls += 1;
        if error {
            state.metrics.errors += 1;
        }
        self.push_event(
            &mut state,
            "call/completed",
            Some(space),
            Some(session),
            [
                ("operation", operation.to_owned()),
                ("bytes", bytes.to_string()),
                ("elapsed_ms", elapsed_ms.to_string()),
                ("status", if error { "error" } else { "ok" }.into()),
            ],
        );
    }

    fn event<const N: usize>(
        &self,
        kind: &str,
        space: Option<&str>,
        session: Option<&str>,
        detail: [(&str, String); N],
    ) {
        let Ok(mut state) = self.state() else { return };
        self.push_event(&mut state, kind, space, session, detail);
    }

    fn push_event<const N: usize>(
        &self,
        state: &mut State,
        kind: &str,
        space: Option<&str>,
        session: Option<&str>,
        detail: [(&str, String); N],
    ) {
        state.next_event += 1;
        state.events.push_back(AnalyticsEvent {
            cursor: state.next_event,
            timestamp_ms: now_ms(),
            kind: kind.into(),
            space: space.map(str::to_owned),
            session: session.map(str::to_owned),
            detail: detail
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value))
                .collect(),
        });
        while state.events.len() > self.inner.event_capacity {
            state.events.pop_front();
        }
    }

    fn state(&self) -> Result<std::sync::MutexGuard<'_, State>, String> {
        self.inner
            .state
            .lock()
            .map_err(|_| "service/lock-poisoned".into())
    }
}

struct RetentionStore {
    connection: Connection,
}

impl RetentionStore {
    fn memory() -> Result<Self, String> {
        let connection = Connection::open_in_memory().map_err(sql_error)?;
        Self::initialize(connection)
    }

    fn open(data_dir: &Path) -> Result<Self, String> {
        fs::create_dir_all(data_dir)
            .map_err(|error| format!("retention/open: {}: {error}", data_dir.display()))?;
        let connection = Connection::open(data_dir.join("reports.sqlite3")).map_err(sql_error)?;
        connection
            .pragma_update(None, "journal_mode", "WAL")
            .map_err(sql_error)?;
        connection
            .pragma_update(None, "synchronous", "FULL")
            .map_err(sql_error)?;
        Self::initialize(connection)
    }

    fn initialize(connection: Connection) -> Result<Self, String> {
        connection
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS reports (
                    space TEXT NOT NULL,
                    session TEXT NOT NULL,
                    signal TEXT NOT NULL,
                    sequence INTEGER NOT NULL,
                    id TEXT NOT NULL,
                    source TEXT NOT NULL,
                    timestamp_ms INTEGER NOT NULL,
                    data BLOB NOT NULL,
                    PRIMARY KEY (space, session, signal, sequence),
                    UNIQUE (space, session, id)
                 );
                 CREATE INDEX IF NOT EXISTS reports_replay
                   ON reports(space, session, signal, sequence);",
            )
            .map_err(sql_error)?;
        Ok(Self { connection })
    }

    fn append(&mut self, report: &Report, policy: RetentionPolicy) -> Result<u64, String> {
        let transaction = self.connection.transaction().map_err(sql_error)?;
        let existing = transaction
            .query_row(
                "SELECT sequence FROM reports WHERE space=?1 AND session=?2 AND id=?3",
                params![report.space, report.target, report.id],
                |row| row.get::<_, u64>(0),
            )
            .ok();
        if let Some(sequence) = existing {
            transaction.commit().map_err(sql_error)?;
            return Ok(sequence);
        }
        let sequence = transaction
            .query_row(
                "SELECT COALESCE(MAX(sequence), 0) + 1 FROM reports
                 WHERE space=?1 AND session=?2 AND signal=?3",
                params![report.space, report.target, report.signal],
                |row| row.get::<_, u64>(0),
            )
            .map_err(sql_error)?;
        transaction
            .execute(
                "INSERT INTO reports
                 (space, session, signal, sequence, id, source, timestamp_ms, data)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    report.space,
                    report.target,
                    report.signal,
                    sequence,
                    report.id,
                    report.source,
                    report.timestamp_ms,
                    report.data
                ],
            )
            .map_err(sql_error)?;
        let oldest = now_ms().saturating_sub(policy.max_age_ms);
        transaction
            .execute(
                "DELETE FROM reports WHERE space=?1 AND session=?2 AND signal=?3
                 AND (timestamp_ms < ?4 OR sequence <= (
                   SELECT MAX(sequence) - ?5 FROM reports
                   WHERE space=?1 AND session=?2 AND signal=?3
                 ))",
                params![
                    report.space,
                    report.target,
                    report.signal,
                    oldest,
                    policy.max_events as u64
                ],
            )
            .map_err(sql_error)?;
        transaction.commit().map_err(sql_error)?;
        Ok(sequence)
    }

    fn replay(
        &self,
        space: &str,
        session: &str,
        signal: &str,
        after: u64,
        limit: usize,
    ) -> Result<Vec<Report>, String> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT id, source, timestamp_ms, sequence, data FROM reports
                 WHERE space=?1 AND session=?2 AND signal=?3 AND sequence>?4
                 ORDER BY sequence LIMIT ?5",
            )
            .map_err(sql_error)?;
        let rows = statement
            .query_map(
                params![space, session, signal, after, limit as u64],
                |row| {
                    Ok(Report {
                        id: row.get(0)?,
                        space: space.into(),
                        source: row.get(1)?,
                        target: session.into(),
                        signal: signal.into(),
                        timestamp_ms: row.get(2)?,
                        sequence: Some(row.get(3)?),
                        data: row.get(4)?,
                    })
                },
            )
            .map_err(sql_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(sql_error)
    }
}

fn validate_name(value: &str, code: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_.-".contains(&byte))
    {
        Err(format!("{code}: {value}"))
    } else {
        Ok(())
    }
}

fn validate_signal(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 256
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_.:/-".contains(&byte))
    {
        Err(format!("report/signal-invalid: {value}"))
    } else {
        Ok(())
    }
}

fn shard_for(space: &str, session: &str, shards: usize) -> usize {
    let digest = Sha256::digest(format!("{space}\0{session}").as_bytes());
    u64::from_be_bytes(digest[..8].try_into().expect("sha256 prefix")) as usize % shards
}

fn runtime_name(space: &str, session: &str) -> String {
    let digest = Sha256::digest(format!("{space}\0{session}").as_bytes());
    format!("S.{}", hex(&digest[..12]))
}

fn rendezvous_score(epoch: &str, member: &str, space: &str, session: &str) -> u64 {
    let digest = Sha256::digest(format!("{epoch}\0{member}\0{space}\0{session}").as_bytes());
    u64::from_be_bytes(digest[..8].try_into().expect("sha256 prefix"))
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn refresh_gauges(state: &mut State) {
    state.metrics.spaces = state.spaces.len() as u64;
    state.metrics.sessions = state
        .spaces
        .values()
        .map(|space| space.sessions.len() as u64)
        .sum();
    state.metrics.artifacts = state.artifacts.len() as u64;
    state.metrics.loaded_modules = state
        .spaces
        .values()
        .flat_map(|space| space.sessions.values())
        .map(|session| session.modules.len() as u64)
        .sum();
    state.metrics.subscriptions = state.subscriptions.len() as u64;
}

fn fail<T>(state: &mut State, error: String) -> Result<T, String> {
    state.metrics.errors += 1;
    Err(error)
}

fn sql_error(error: rusqlite::Error) -> String {
    format!("retention/sqlite: {error}")
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

#[cfg(test)]
mod tests {
    use super::{FabricService, HomeRouter, ServiceConfig};
    use crate::extension::Value;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn service() -> FabricService {
        FabricService::start(ServiceConfig {
            shards: 2,
            ..ServiceConfig::default()
        })
        .unwrap()
    }

    #[test]
    fn spaces_and_sessions_are_isolated() {
        let service = service();
        service.create_space("room").unwrap();
        service.create_session("room", "agent").unwrap();
        service.eval("room", "agent", "(def answer 42)").unwrap();
        assert_eq!(service.eval("room", "agent", "answer").unwrap(), "42");
        assert!(service.eval("room", "ROOT", "answer").is_err());
        assert_eq!(
            service.list_sessions("room").unwrap(),
            vec!["ROOT", "agent"]
        );
    }

    #[test]
    fn reports_are_bounded_delivered_and_replayed() {
        let service = service();
        service.create_space("room").unwrap();
        service.create_session("room", "source").unwrap();
        service.create_session("room", "target").unwrap();
        service
            .retain("room", "target", "finding", 10, 60_000)
            .unwrap();
        let subscription = service.subscribe("room", "target", "finding").unwrap();
        let receipt = service
            .send_report("room", "source", "target", "finding", b"safe metadata")
            .unwrap();
        assert_eq!(receipt.delivered, 1);
        assert!(receipt.retained);
        let delivered = service.next_report(&subscription).unwrap().unwrap();
        assert_eq!(delivered.source, "source");
        assert_eq!(delivered.data, b"safe metadata");
        let replay = service.replay("room", "target", "finding", 0, 10).unwrap();
        assert_eq!(replay, vec![delivered]);
    }

    #[test]
    fn analytics_redact_report_payloads() {
        let service = service();
        service.create_space("room").unwrap();
        service.create_session("room", "a").unwrap();
        service.create_session("room", "b").unwrap();
        service
            .send_report("room", "a", "b", "result", b"top-secret")
            .unwrap();
        let json = service
            .events_since(0, 100)
            .unwrap()
            .into_iter()
            .map(|event| event.json())
            .collect::<String>();
        assert!(json.contains("\"bytes\":\"10\""));
        assert!(!json.contains("top-secret"));
    }

    #[test]
    fn rendezvous_home_is_stable_and_epoch_scoped() {
        let left =
            HomeRouter::new("a".into(), vec!["b".into(), "c".into()], "epoch-1".into()).unwrap();
        let right =
            HomeRouter::new("c".into(), vec!["a".into(), "b".into()], "epoch-1".into()).unwrap();
        assert_eq!(left.home("room", "agent"), right.home("room", "agent"));
    }

    #[test]
    fn content_addressed_module_is_compiled_once_and_instanced_per_session() {
        // (module (func (export "add") (param i32 i32) (result i32)
        //   local.get 0 local.get 1 i32.add))
        let wasm = [
            0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x07, 0x01, 0x60, 0x02, 0x7f,
            0x7f, 0x01, 0x7f, 0x03, 0x02, 0x01, 0x00, 0x07, 0x07, 0x01, 0x03, 0x61, 0x64, 0x64,
            0x00, 0x00, 0x0a, 0x09, 0x01, 0x07, 0x00, 0x20, 0x00, 0x20, 0x01, 0x6a, 0x0b,
        ];
        let manifest = r#"
          {:namespace "agent.math"
           :version "0.1.0"
           :provider :wasm
           :module "math.wasm"
           :abi :core.v1
           :exports {"add" {:args [:i32 :i32] :returns :i32 :async false}}
           :capabilities []}"#;
        let service = service();
        service.create_space("room").unwrap();
        service.create_session("room", "a").unwrap();
        service.create_session("room", "b").unwrap();
        let digest = service.put_module(manifest, &wasm).unwrap();
        assert_eq!(service.put_module(manifest, &wasm).unwrap(), digest);
        service.load_module("room", "a", &digest).unwrap();
        service.load_module("room", "b", &digest).unwrap();
        let arguments = crate::hta::encode(&Value::Vector(
            vec![Value::Number(19), Value::Number(23)].into(),
        ))
        .unwrap();
        let result = service
            .invoke_module("room", "a", "agent.math/add", &arguments)
            .unwrap();
        assert_eq!(crate::hta::decode(&result).unwrap(), Value::Number(42));
        let metrics = service.metrics().unwrap();
        assert_eq!(metrics.artifacts, 1);
        assert_eq!(metrics.loaded_modules, 2);
    }

    #[test]
    fn retained_reports_survive_service_restart() {
        let data_dir = std::env::temp_dir().join(format!(
            "hara-fabric-retention-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let config = ServiceConfig {
            shards: 1,
            data_dir: Some(data_dir.clone()),
            ..ServiceConfig::default()
        };
        {
            let service = FabricService::start(config.clone()).unwrap();
            service.create_space("room").unwrap();
            service.create_session("room", "source").unwrap();
            service.create_session("room", "target").unwrap();
            service
                .retain("room", "target", "final", 10, 60_000)
                .unwrap();
            service
                .send_report("room", "source", "target", "final", b"checkpoint")
                .unwrap();
        }
        let restarted = FabricService::start(config).unwrap();
        let reports = restarted.replay("room", "target", "final", 0, 10).unwrap();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].data, b"checkpoint");
        fs::remove_dir_all(data_dir).unwrap();
    }
}
