//! Immutable snapshot-backed sessions for production embeddings.

use crate::core::{self, Value};
use crate::kernel::{ResolvedSecrets, SecretCatalog};
use crate::snapshot::{Digest, ResolvedSnapshot};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionMode {
    Sealed,
    Overlay,
}

#[derive(Clone, Debug)]
pub struct SnapshotSessionDefinition {
    pub id: String,
    pub snapshot: Digest,
    pub mode: SessionMode,
    pub grants: BTreeSet<String>,
}

#[derive(Clone, Debug)]
pub struct FrozenSession {
    id: String,
    snapshot: Rc<ResolvedSnapshot>,
    mode: SessionMode,
    grants: BTreeSet<String>,
    secrets: ResolvedSecrets,
}

impl FrozenSession {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn snapshot(&self) -> &Rc<ResolvedSnapshot> {
        &self.snapshot
    }

    pub fn mode(&self) -> SessionMode {
        self.mode
    }

    pub fn grants(&self) -> &BTreeSet<String> {
        &self.grants
    }

    pub fn secrets(&self) -> &ResolvedSecrets {
        &self.secrets
    }

    pub fn entrypoint(&self, name: &str) -> Option<&str> {
        self.snapshot
            .manifest
            .entrypoints
            .get(name)
            .map(String::as_str)
    }

    pub fn require_mutable_overlay(&self) -> Result<(), String> {
        match self.mode {
            SessionMode::Overlay => Ok(()),
            SessionMode::Sealed => Err("session/sealed-mutation-denied".into()),
        }
    }
}

#[derive(Default)]
pub struct SnapshotRegistry {
    values: BTreeMap<Digest, Rc<ResolvedSnapshot>>,
}

impl SnapshotRegistry {
    pub fn insert(&mut self, snapshot: ResolvedSnapshot) -> Rc<ResolvedSnapshot> {
        self.values
            .entry(snapshot.digest)
            .or_insert_with(|| Rc::new(snapshot))
            .clone()
    }

    pub fn get(&self, digest: &Digest) -> Option<Rc<ResolvedSnapshot>> {
        self.values.get(digest).cloned()
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SharedStateCell {
    revision: u64,
    value: Value,
}

impl SharedStateCell {
    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn value(&self) -> &Value {
        &self.value
    }
}

#[derive(Default)]
pub struct SnapshotKernel {
    snapshots: SnapshotRegistry,
    sessions: BTreeMap<String, Rc<FrozenSession>>,
    shared_state: BTreeMap<String, SharedStateCell>,
}

impl SnapshotKernel {
    pub fn snapshots(&self) -> &SnapshotRegistry {
        &self.snapshots
    }

    pub fn register_snapshot(&mut self, snapshot: ResolvedSnapshot) -> Rc<ResolvedSnapshot> {
        self.snapshots.insert(snapshot)
    }

    pub fn initialize_state_from(&mut self, digest: &Digest) -> Result<(), String> {
        let snapshot = self
            .snapshots
            .get(digest)
            .ok_or_else(|| format!("snapshot/not-registered: {}", crate::snapshot::hex(digest)))?;
        let mut state = BTreeMap::new();
        for (name, value) in &snapshot.manifest.initial_state {
            if !core::session_transferable(value) {
                return Err(format!("snapshot/non-transferable-state: {name}"));
            }
            state.insert(
                name.clone(),
                SharedStateCell {
                    revision: 0,
                    value: value.clone(),
                },
            );
        }
        self.shared_state = state;
        Ok(())
    }

    /// Builds all sessions privately and publishes them only after every
    /// snapshot, capability, and secret requirement validates.
    pub fn load_sessions(
        &mut self,
        definitions: &[SnapshotSessionDefinition],
        secrets: &dyn SecretCatalog,
    ) -> Result<(), String> {
        let mut candidate = BTreeMap::new();
        for definition in definitions {
            if definition.id.is_empty() || candidate.contains_key(&definition.id) {
                return Err(format!("session/duplicate-or-empty: {}", definition.id));
            }
            let snapshot = self.snapshots.get(&definition.snapshot).ok_or_else(|| {
                format!(
                    "session/snapshot-not-registered: {}",
                    crate::snapshot::hex(&definition.snapshot)
                )
            })?;
            for capability in &snapshot.manifest.capabilities {
                if !definition.grants.contains(capability) {
                    return Err(format!(
                        "session/capability-not-granted: {} requires {capability}",
                        definition.id
                    ));
                }
            }
            let resolved_secrets = ResolvedSecrets::resolve(&snapshot.manifest.secrets, secrets)?;
            candidate.insert(
                definition.id.clone(),
                Rc::new(FrozenSession {
                    id: definition.id.clone(),
                    snapshot,
                    mode: definition.mode,
                    grants: definition.grants.clone(),
                    secrets: resolved_secrets,
                }),
            );
        }
        self.sessions = candidate;
        Ok(())
    }

    pub fn session(&self, id: &str) -> Option<Rc<FrozenSession>> {
        self.sessions.get(id).cloned()
    }

    pub fn session_ids(&self) -> impl Iterator<Item = &str> {
        self.sessions.keys().map(String::as_str)
    }

    pub fn state(&self, name: &str) -> Option<&SharedStateCell> {
        self.shared_state.get(name)
    }

    pub fn state_compare_and_set(
        &mut self,
        name: &str,
        expected_revision: u64,
        value: Value,
    ) -> Result<bool, String> {
        if !core::session_transferable(&value) {
            return Err(format!("kernel/non-transferable-state: {name}"));
        }
        let cell = self
            .shared_state
            .get_mut(name)
            .ok_or_else(|| format!("kernel/unknown-state: {name}"))?;
        if cell.revision != expected_revision {
            return Ok(false);
        }
        cell.revision = cell
            .revision
            .checked_add(1)
            .ok_or_else(|| format!("kernel/state-revision-exhausted: {name}"))?;
        cell.value = value;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::SecretDescriptor;
    use crate::snapshot::{SecretRequirement, SnapshotArtifact, SnapshotManifest};

    struct Catalog(BTreeMap<String, SecretDescriptor>);

    impl SecretCatalog for Catalog {
        fn describe(&self, id: &str) -> Result<Option<SecretDescriptor>, String> {
            Ok(self.0.get(id).cloned())
        }
    }

    fn resolved() -> ResolvedSnapshot {
        SnapshotArtifact {
            base: None,
            manifest: SnapshotManifest {
                language_version: "0.1".into(),
                dependency_lock_digest: [0; 32],
                libraries: vec![],
                namespaces: vec![],
                entrypoints: BTreeMap::from([("api".into(), "app/handle".into())]),
                initial_state: BTreeMap::from([("counter".into(), Value::Number(0))]),
                capabilities: BTreeSet::from(["nginx/timer".into()]),
                secrets: vec![SecretRequirement {
                    id: "key".into(),
                    purpose: "sign".into(),
                    required: true,
                    version: Some("1".into()),
                }],
                accelerators: vec![],
            },
        }
        .resolve(None)
        .unwrap()
    }

    fn catalog() -> Catalog {
        Catalog(BTreeMap::from([(
            "key".into(),
            SecretDescriptor {
                id: "key".into(),
                provider: "test".into(),
                version: Some("1".into()),
            },
        )]))
    }

    #[test]
    fn publishes_all_sessions_transactionally_and_shares_snapshot_memory() {
        let mut kernel = SnapshotKernel::default();
        let snapshot = kernel.register_snapshot(resolved());
        kernel.initialize_state_from(&snapshot.digest).unwrap();
        let definitions = ["primary", "sample"].map(|id| SnapshotSessionDefinition {
            id: id.into(),
            snapshot: snapshot.digest,
            mode: SessionMode::Sealed,
            grants: BTreeSet::from(["nginx/timer".into()]),
        });
        kernel.load_sessions(&definitions, &catalog()).unwrap();
        let primary = kernel.session("primary").unwrap();
        let sample = kernel.session("sample").unwrap();
        assert!(Rc::ptr_eq(primary.snapshot(), sample.snapshot()));
        assert_eq!(primary.entrypoint("api"), Some("app/handle"));
        assert!(primary.require_mutable_overlay().is_err());
        assert_eq!(kernel.state("counter").unwrap().revision(), 0);
        assert!(kernel
            .state_compare_and_set("counter", 0, Value::Number(1))
            .unwrap());
        assert_eq!(kernel.state("counter").unwrap().revision(), 1);
    }

    #[test]
    fn failed_candidate_does_not_replace_published_sessions() {
        let mut kernel = SnapshotKernel::default();
        let snapshot = kernel.register_snapshot(resolved());
        let valid = SnapshotSessionDefinition {
            id: "primary".into(),
            snapshot: snapshot.digest,
            mode: SessionMode::Sealed,
            grants: BTreeSet::from(["nginx/timer".into()]),
        };
        kernel.load_sessions(&[valid], &catalog()).unwrap();
        let invalid = SnapshotSessionDefinition {
            id: "candidate".into(),
            snapshot: snapshot.digest,
            mode: SessionMode::Sealed,
            grants: BTreeSet::new(),
        };
        assert!(kernel.load_sessions(&[invalid], &catalog()).is_err());
        assert!(kernel.session("primary").is_some());
        assert!(kernel.session("candidate").is_none());
    }
}
