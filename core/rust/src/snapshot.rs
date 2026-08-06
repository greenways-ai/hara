//! Portable, deterministic Hara session startup snapshots.
//!
//! HSS1 stores immutable language state and declarations only. Live runtime
//! values (promises, fibers, host handles, mutable references, and authority)
//! are intentionally outside the format. An artifact may inherit unchanged
//! namespace payloads from a content-addressed base snapshot.

use crate::core::{self, Value};
use sha2::{Digest as ShaDigest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const MAGIC: &[u8; 4] = b"HSS1";
const FORMAT_VERSION: u16 = 1;
const HASH_BYTES: usize = 32;
const MAX_PAYLOAD_BYTES: usize = 512 * 1024 * 1024;
const MAX_ITEMS: usize = 1_000_000;

pub type Digest = [u8; HASH_BYTES];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryRef {
    pub id: String,
    pub version: String,
    pub digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NamespaceImage {
    pub name: String,
    pub digest: Digest,
    /// `None` means that the payload is inherited from `base`.
    pub halc: Option<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SecretRequirement {
    pub id: String,
    pub purpose: String,
    pub required: bool,
    /// Optional provider version/key identifier. This is not secret material.
    pub version: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeAccelerator {
    pub runtime: String,
    pub format: String,
    pub version: String,
    pub digest: Digest,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SnapshotManifest {
    pub language_version: String,
    pub dependency_lock_digest: Digest,
    pub libraries: Vec<LibraryRef>,
    pub namespaces: Vec<NamespaceImage>,
    pub entrypoints: BTreeMap<String, String>,
    pub initial_state: BTreeMap<String, Value>,
    pub capabilities: BTreeSet<String>,
    pub secrets: Vec<SecretRequirement>,
    pub accelerators: Vec<RuntimeAccelerator>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SnapshotArtifact {
    /// Logical digest of the fully resolved base snapshot.
    pub base: Option<Digest>,
    pub manifest: SnapshotManifest,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedSnapshot {
    pub digest: Digest,
    pub manifest: SnapshotManifest,
}

impl SnapshotArtifact {
    pub fn is_incremental(&self) -> bool {
        self.base.is_some()
    }

    pub fn validate(&self) -> Result<(), String> {
        validate_manifest(&self.manifest, self.base.is_some())
    }

    pub fn resolve(&self, base: Option<&ResolvedSnapshot>) -> Result<ResolvedSnapshot, String> {
        self.validate()?;
        match (self.base, base) {
            (None, None) => {}
            (None, Some(_)) => return Err("snapshot/full-does-not-accept-base".into()),
            (Some(_), None) => return Err("snapshot/base-required".into()),
            (Some(expected), Some(actual)) if expected != actual.digest => {
                return Err(format!(
                    "snapshot/base-digest-mismatch: expected {}, received {}",
                    hex(&expected),
                    hex(&actual.digest)
                ));
            }
            (Some(_), Some(_)) => {}
        }

        let mut manifest = self.manifest.clone();
        if let Some(base) = base {
            let inherited = base
                .manifest
                .namespaces
                .iter()
                .map(|namespace| (namespace.name.as_str(), namespace))
                .collect::<BTreeMap<_, _>>();
            for namespace in &mut manifest.namespaces {
                if namespace.halc.is_some() {
                    continue;
                }
                let source = inherited.get(namespace.name.as_str()).ok_or_else(|| {
                    format!("snapshot/inherited-namespace-missing: {}", namespace.name)
                })?;
                if source.digest != namespace.digest {
                    return Err(format!(
                        "snapshot/inherited-namespace-digest-mismatch: {}",
                        namespace.name
                    ));
                }
                namespace.halc = source.halc.clone();
            }
        }
        if manifest.namespaces.iter().any(|value| value.halc.is_none()) {
            return Err("snapshot/full-artifact-has-inherited-namespace".into());
        }
        let digest = logical_digest(&manifest)?;
        Ok(ResolvedSnapshot { digest, manifest })
    }
}

pub fn encode(artifact: &SnapshotArtifact) -> Result<Vec<u8>, String> {
    artifact.validate()?;
    let mut payload = Writer::default();
    payload.optional_digest(artifact.base.as_ref());
    encode_manifest(&artifact.manifest, &mut payload, true)?;
    let payload = payload.finish();
    if payload.len() > MAX_PAYLOAD_BYTES {
        return Err("snapshot/artifact-too-large".into());
    }
    let checksum: Digest = Sha256::digest(&payload).into();
    let mut output = Vec::with_capacity(4 + 2 + 4 + HASH_BYTES + payload.len());
    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&FORMAT_VERSION.to_be_bytes());
    output.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    output.extend_from_slice(&checksum);
    output.extend_from_slice(&payload);
    Ok(output)
}

pub fn decode(bytes: &[u8]) -> Result<SnapshotArtifact, String> {
    let mut reader = Reader::new(bytes);
    if reader.bytes(MAGIC.len())? != MAGIC {
        return Err("snapshot/invalid-magic".into());
    }
    let version = reader.u16()?;
    if version != FORMAT_VERSION {
        return Err(format!("snapshot/unsupported-version: {version}"));
    }
    let length = reader.u32()? as usize;
    if length > MAX_PAYLOAD_BYTES {
        return Err("snapshot/artifact-too-large".into());
    }
    let expected = reader.digest()?;
    let payload = reader.bytes(length)?;
    if !reader.done() {
        return Err("snapshot/trailing-artifact-bytes".into());
    }
    let actual: Digest = Sha256::digest(payload).into();
    if expected != actual {
        return Err("snapshot/checksum-mismatch".into());
    }
    let mut payload = Reader::new(payload);
    let base = payload.optional_digest()?;
    let manifest = decode_manifest(&mut payload)?;
    if !payload.done() {
        return Err("snapshot/trailing-payload-bytes".into());
    }
    let artifact = SnapshotArtifact { base, manifest };
    artifact.validate()?;
    Ok(artifact)
}

pub fn artifact_digest(bytes: &[u8]) -> Digest {
    Sha256::digest(bytes).into()
}

pub fn logical_digest(manifest: &SnapshotManifest) -> Result<Digest, String> {
    validate_manifest(manifest, false)?;
    let mut writer = Writer::default();
    encode_manifest(manifest, &mut writer, false)?;
    Ok(Sha256::digest(writer.finish()).into())
}

pub fn hex(digest: &Digest) -> String {
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn validate_manifest(manifest: &SnapshotManifest, incremental: bool) -> Result<(), String> {
    if manifest.language_version.is_empty() {
        return Err("snapshot/language-version-required".into());
    }
    unique(
        manifest.libraries.iter().map(|value| value.id.as_str()),
        "library",
    )?;
    unique(
        manifest.namespaces.iter().map(|value| value.name.as_str()),
        "namespace",
    )?;
    unique(
        manifest.secrets.iter().map(|value| value.id.as_str()),
        "secret",
    )?;
    unique(
        manifest
            .accelerators
            .iter()
            .map(|value| format!("{}:{}", value.runtime, value.format)),
        "accelerator",
    )?;
    for namespace in &manifest.namespaces {
        if namespace.name.is_empty() {
            return Err("snapshot/namespace-name-required".into());
        }
        match &namespace.halc {
            Some(bytes) => {
                let actual: Digest = Sha256::digest(bytes).into();
                if actual != namespace.digest {
                    return Err(format!("snapshot/namespace-checksum: {}", namespace.name));
                }
            }
            None if !incremental => {
                return Err(format!(
                    "snapshot/inherited-namespace-in-full: {}",
                    namespace.name
                ));
            }
            None => {}
        }
    }
    for (name, value) in &manifest.initial_state {
        if name.is_empty() || !core::session_transferable(value) {
            return Err(format!("snapshot/non-transferable-state: {name}"));
        }
        crate::hta::encode(value)
            .map_err(|error| format!("snapshot/state-encoding {name}: {error}"))?;
    }
    for secret in &manifest.secrets {
        if secret.id.is_empty() || secret.purpose.is_empty() {
            return Err("snapshot/secret-id-and-purpose-required".into());
        }
    }
    for accelerator in &manifest.accelerators {
        let actual: Digest = Sha256::digest(&accelerator.bytes).into();
        if actual != accelerator.digest {
            return Err(format!(
                "snapshot/accelerator-checksum: {}:{}",
                accelerator.runtime, accelerator.format
            ));
        }
    }
    Ok(())
}

fn unique<I, S>(values: I, kind: &str) -> Result<(), String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut seen = BTreeSet::new();
    for value in values {
        let value = value.as_ref();
        if !seen.insert(value.to_owned()) {
            return Err(format!("snapshot/duplicate-{kind}: {value}"));
        }
    }
    Ok(())
}

fn encode_manifest(
    manifest: &SnapshotManifest,
    writer: &mut Writer,
    include_accelerators: bool,
) -> Result<(), String> {
    writer.string(&manifest.language_version)?;
    writer.digest(&manifest.dependency_lock_digest);

    let mut libraries = manifest.libraries.clone();
    libraries.sort_by(|left, right| left.id.cmp(&right.id));
    writer.count(libraries.len())?;
    for library in libraries {
        writer.string(&library.id)?;
        writer.string(&library.version)?;
        writer.digest(&library.digest);
    }

    let mut namespaces = manifest.namespaces.clone();
    namespaces.sort_by(|left, right| left.name.cmp(&right.name));
    writer.count(namespaces.len())?;
    for namespace in namespaces {
        writer.string(&namespace.name)?;
        writer.digest(&namespace.digest);
        writer.optional_bytes(namespace.halc.as_deref())?;
    }

    writer.count(manifest.entrypoints.len())?;
    for (name, target) in &manifest.entrypoints {
        writer.string(name)?;
        writer.string(target)?;
    }

    writer.count(manifest.initial_state.len())?;
    for (name, value) in &manifest.initial_state {
        writer.string(name)?;
        writer.bytes(&crate::hta::encode(value)?)?;
    }

    writer.count(manifest.capabilities.len())?;
    for capability in &manifest.capabilities {
        writer.string(capability)?;
    }

    let mut secrets = manifest.secrets.clone();
    secrets.sort_by(|left, right| left.id.cmp(&right.id));
    writer.count(secrets.len())?;
    for secret in secrets {
        writer.string(&secret.id)?;
        writer.string(&secret.purpose)?;
        writer.boolean(secret.required);
        writer.optional_string(secret.version.as_deref())?;
    }

    if include_accelerators {
        let mut accelerators = manifest.accelerators.clone();
        accelerators.sort_by(|left, right| {
            (&left.runtime, &left.format).cmp(&(&right.runtime, &right.format))
        });
        writer.count(accelerators.len())?;
        for accelerator in accelerators {
            writer.string(&accelerator.runtime)?;
            writer.string(&accelerator.format)?;
            writer.string(&accelerator.version)?;
            writer.digest(&accelerator.digest);
            writer.bytes(&accelerator.bytes)?;
        }
    } else {
        writer.count(0)?;
    }
    Ok(())
}

fn decode_manifest(reader: &mut Reader<'_>) -> Result<SnapshotManifest, String> {
    let language_version = reader.string()?;
    let dependency_lock_digest = reader.digest()?;
    let libraries = reader.items(|reader| {
        Ok(LibraryRef {
            id: reader.string()?,
            version: reader.string()?,
            digest: reader.digest()?,
        })
    })?;
    let namespaces = reader.items(|reader| {
        Ok(NamespaceImage {
            name: reader.string()?,
            digest: reader.digest()?,
            halc: reader.optional_bytes()?,
        })
    })?;
    let entrypoints = reader.map(|reader| Ok((reader.string()?, reader.string()?)))?;
    let initial_state = reader.map(|reader| {
        let name = reader.string()?;
        let value = crate::hta::decode(&reader.owned_bytes()?)?;
        Ok((name, value))
    })?;
    let capabilities = reader
        .items(Reader::string)?
        .into_iter()
        .collect::<BTreeSet<_>>();
    let secrets = reader.items(|reader| {
        Ok(SecretRequirement {
            id: reader.string()?,
            purpose: reader.string()?,
            required: reader.boolean()?,
            version: reader.optional_string()?,
        })
    })?;
    let accelerators = reader.items(|reader| {
        Ok(RuntimeAccelerator {
            runtime: reader.string()?,
            format: reader.string()?,
            version: reader.string()?,
            digest: reader.digest()?,
            bytes: reader.owned_bytes()?,
        })
    })?;
    Ok(SnapshotManifest {
        language_version,
        dependency_lock_digest,
        libraries,
        namespaces,
        entrypoints,
        initial_state,
        capabilities,
        secrets,
        accelerators,
    })
}

#[derive(Default)]
struct Writer(Vec<u8>);

impl Writer {
    fn finish(self) -> Vec<u8> {
        self.0
    }
    fn boolean(&mut self, value: bool) {
        self.0.push(u8::from(value));
    }
    fn count(&mut self, count: usize) -> Result<(), String> {
        let count = u32::try_from(count).map_err(|_| "snapshot/too-many-items")?;
        self.0.extend_from_slice(&count.to_be_bytes());
        Ok(())
    }
    fn bytes(&mut self, bytes: &[u8]) -> Result<(), String> {
        self.count(bytes.len())?;
        self.0.extend_from_slice(bytes);
        Ok(())
    }
    fn optional_bytes(&mut self, bytes: Option<&[u8]>) -> Result<(), String> {
        self.boolean(bytes.is_some());
        if let Some(bytes) = bytes {
            self.bytes(bytes)?;
        }
        Ok(())
    }
    fn string(&mut self, value: &str) -> Result<(), String> {
        self.bytes(value.as_bytes())
    }
    fn optional_string(&mut self, value: Option<&str>) -> Result<(), String> {
        self.boolean(value.is_some());
        if let Some(value) = value {
            self.string(value)?;
        }
        Ok(())
    }
    fn digest(&mut self, digest: &Digest) {
        self.0.extend_from_slice(digest);
    }
    fn optional_digest(&mut self, digest: Option<&Digest>) {
        self.boolean(digest.is_some());
        if let Some(digest) = digest {
            self.digest(digest);
        }
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }
    fn done(&self) -> bool {
        self.cursor == self.bytes.len()
    }
    fn bytes(&mut self, count: usize) -> Result<&'a [u8], String> {
        let end = self
            .cursor
            .checked_add(count)
            .ok_or("snapshot/length-overflow")?;
        if end > self.bytes.len() {
            return Err("snapshot/truncated".into());
        }
        let bytes = &self.bytes[self.cursor..end];
        self.cursor = end;
        Ok(bytes)
    }
    fn u16(&mut self) -> Result<u16, String> {
        Ok(u16::from_be_bytes(self.bytes(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_be_bytes(self.bytes(4)?.try_into().unwrap()))
    }
    fn boolean(&mut self) -> Result<bool, String> {
        match self.bytes(1)?[0] {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err("snapshot/invalid-boolean".into()),
        }
    }
    fn owned_bytes(&mut self) -> Result<Vec<u8>, String> {
        let count = self.u32()? as usize;
        if count > MAX_PAYLOAD_BYTES {
            return Err("snapshot/section-too-large".into());
        }
        Ok(self.bytes(count)?.to_vec())
    }
    fn optional_bytes(&mut self) -> Result<Option<Vec<u8>>, String> {
        self.boolean()?.then(|| self.owned_bytes()).transpose()
    }
    fn string(&mut self) -> Result<String, String> {
        String::from_utf8(self.owned_bytes()?).map_err(|_| "snapshot/invalid-utf8".into())
    }
    fn optional_string(&mut self) -> Result<Option<String>, String> {
        self.boolean()?.then(|| self.string()).transpose()
    }
    fn digest(&mut self) -> Result<Digest, String> {
        Ok(self.bytes(HASH_BYTES)?.try_into().unwrap())
    }
    fn optional_digest(&mut self) -> Result<Option<Digest>, String> {
        self.boolean()?.then(|| self.digest()).transpose()
    }
    fn items<T>(
        &mut self,
        mut item: impl FnMut(&mut Reader<'a>) -> Result<T, String>,
    ) -> Result<Vec<T>, String> {
        let count = self.u32()? as usize;
        if count > MAX_ITEMS {
            return Err("snapshot/too-many-items".into());
        }
        (0..count).map(|_| item(self)).collect()
    }
    fn map<K: Ord, V>(
        &mut self,
        mut item: impl FnMut(&mut Reader<'a>) -> Result<(K, V), String>,
    ) -> Result<BTreeMap<K, V>, String> {
        let entries = self.items(|reader| item(reader))?;
        let count = entries.len();
        let values = entries.into_iter().collect::<BTreeMap<_, _>>();
        if values.len() != count {
            return Err("snapshot/duplicate-map-key".into());
        }
        Ok(values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(bytes: &[u8]) -> Digest {
        Sha256::digest(bytes).into()
    }

    fn manifest(namespace_bytes: Option<Vec<u8>>) -> SnapshotManifest {
        let bytes = namespace_bytes.as_deref().unwrap_or(b"namespace");
        SnapshotManifest {
            language_version: "0.1.0".into(),
            dependency_lock_digest: digest(b"lock"),
            libraries: vec![LibraryRef {
                id: "app/common".into(),
                version: "1.0.0".into(),
                digest: digest(b"library"),
            }],
            namespaces: vec![NamespaceImage {
                name: "app.common".into(),
                digest: digest(bytes),
                halc: namespace_bytes,
            }],
            entrypoints: BTreeMap::from([("api".into(), "app.common/handle".into())]),
            initial_state: BTreeMap::from([("flags".into(), Value::Bool(true))]),
            capabilities: BTreeSet::from(["nginx/timer".into()]),
            secrets: vec![SecretRequirement {
                id: "stripe-key".into(),
                purpose: "sign Stripe requests".into(),
                required: true,
                version: Some("kms/version/7".into()),
            }],
            accelerators: vec![],
        }
    }

    #[test]
    fn full_snapshot_round_trips_deterministically() {
        let artifact = SnapshotArtifact {
            base: None,
            manifest: manifest(Some(b"namespace".to_vec())),
        };
        let first = encode(&artifact).unwrap();
        let decoded = decode(&first).unwrap();
        let second = encode(&decoded).unwrap();
        assert_eq!(first, second);
        assert_eq!(decoded.resolve(None).unwrap().manifest, artifact.manifest);
    }

    #[test]
    fn incremental_snapshot_inherits_namespace_payloads() {
        let base_artifact = SnapshotArtifact {
            base: None,
            manifest: manifest(Some(b"namespace".to_vec())),
        };
        let base = base_artifact.resolve(None).unwrap();
        let mut delta_manifest = manifest(None);
        delta_manifest
            .initial_state
            .insert("revision".into(), Value::Number(2));
        let delta = SnapshotArtifact {
            base: Some(base.digest),
            manifest: delta_manifest,
        };
        let resolved = decode(&encode(&delta).unwrap())
            .unwrap()
            .resolve(Some(&base))
            .unwrap();
        assert_eq!(
            resolved.manifest.namespaces[0].halc.as_deref(),
            Some(b"namespace".as_slice())
        );
        assert_eq!(
            resolved.manifest.initial_state["revision"],
            Value::Number(2)
        );
        assert_ne!(resolved.digest, base.digest);
    }

    #[test]
    fn incremental_snapshot_requires_the_exact_base() {
        let base = SnapshotArtifact {
            base: None,
            manifest: manifest(Some(b"namespace".to_vec())),
        }
        .resolve(None)
        .unwrap();
        let delta = SnapshotArtifact {
            base: Some(digest(b"wrong")),
            manifest: manifest(None),
        };
        assert!(delta
            .resolve(Some(&base))
            .unwrap_err()
            .contains("base-digest-mismatch"));
    }

    #[test]
    fn rejects_live_values_and_secret_material_is_not_part_of_requirements() {
        let mut invalid_manifest = manifest(Some(b"namespace".to_vec()));
        invalid_manifest
            .initial_state
            .insert("pending".into(), Value::Promise(core::Promise::new()));
        assert!(SnapshotArtifact {
            base: None,
            manifest: invalid_manifest
        }
        .validate()
        .unwrap_err()
        .contains("non-transferable-state"));

        let artifact = SnapshotArtifact {
            base: None,
            manifest: manifest(Some(b"namespace".to_vec())),
        };
        let encoded = encode(&artifact).unwrap();
        assert!(!encoded
            .windows(b"sk_live_secret".len())
            .any(|window| window == b"sk_live_secret"));
    }

    #[test]
    fn accelerator_bytes_do_not_change_the_portable_digest() {
        let mut first = manifest(Some(b"namespace".to_vec()));
        let digest_before = logical_digest(&first).unwrap();
        first.accelerators.push(RuntimeAccelerator {
            runtime: "rust".into(),
            format: "HBC2".into(),
            version: "1".into(),
            digest: digest(b"compiled"),
            bytes: b"compiled".to_vec(),
        });
        assert_eq!(digest_before, logical_digest(&first).unwrap());
    }
}
