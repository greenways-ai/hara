//! Federated package tap trust primitives.
//!
//! A tap is an independently operated registry/identity pair.  The local tap
//! store contains only public, out-of-band trust anchors; it never contains a
//! publisher private key.

use crate::kernel::{parse, Form};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tap {
    pub name: String,
    pub registry: Vec<String>,
    pub identity: Vec<String>,
    /// SHA-256 fingerprint of the policy-signing Ed25519 public key.
    pub identity_key: String,
    pub trust: TrustMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustMode {
    SignedRoot,
    GithubGoverned,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityPolicy {
    pub revision: String,
    pub publisher_keys: BTreeMap<String, PublisherKey>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublisherKey {
    pub public_key: String,
    pub coordinates: Vec<String>,
    pub revoked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitializedTap {
    pub tap: Tap,
    pub fingerprint: String,
}

pub fn config_root() -> PathBuf {
    if let Some(root) = env::var_os("HARA_CONFIG_HOME") {
        return PathBuf::from(root);
    }
    if let Some(root) = env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(root).join("hara");
    }
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config/hara")
}

pub fn add(root: &Path, tap: Tap) -> Result<(), String> {
    validate_tap(&tap)?;
    let mut taps = load(root)?;
    taps.insert(tap.name.clone(), tap);
    save(root, &taps)
}

/// Installs the only built-in bootstrap profile. It intentionally names the
/// GitHub-governed official Hara repositories; arbitrary taps remain signed
/// root-key taps added explicitly by the user.
pub fn bootstrap(root: &Path, profile: &str) -> Result<Tap, String> {
    let tap = match profile {
        "official" => Tap {
            name: "official".into(),
            registry: vec!["https://packages.hara-lang.org".into()],
            identity: vec!["https://packages.hara-lang.org/identity.git".into()],
            identity_key: format!("sha256:{}", "00".repeat(32)),
            trust: TrustMode::SignedRoot,
        },
        _ => return Err(format!("unknown built-in tap profile: {profile}")),
    };
    add(root, tap.clone())?;
    Ok(tap)
}

pub fn add_mirror(
    root: &Path,
    name: &str,
    registry: Option<String>,
    identity: Option<String>,
) -> Result<Tap, String> {
    if registry.is_none() && identity.is_none() {
        return Err("mirror add requires --registry and/or --identity".into());
    }
    let mut taps = load(root)?;
    let tap = taps
        .get_mut(name)
        .ok_or_else(|| format!("tap is not trusted: {name}"))?;
    if let Some(url) = registry {
        if !tap.registry.contains(&url) {
            tap.registry.push(url);
        }
    }
    if let Some(url) = identity {
        if !tap.identity.contains(&url) {
            tap.identity.push(url);
        }
    }
    let updated = tap.clone();
    save(root, &taps)?;
    Ok(updated)
}

pub fn remove(root: &Path, name: &str) -> Result<(), String> {
    let mut taps = load(root)?;
    if taps.remove(name).is_none() {
        return Err(format!("tap is not trusted: {name}"));
    }
    save(root, &taps)
}

pub fn load(root: &Path) -> Result<BTreeMap<String, Tap>, String> {
    let path = root.join("taps.edn");
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let document = parse(&source).map_err(|error| format!("{}: {error}", path.display()))?;
    let entries = map(&document, "taps.edn must be an EDN map")?;
    let Some(taps) = lookup(entries, "taps") else {
        return Ok(BTreeMap::new());
    };
    let taps = map(taps, "taps.edn :taps must be an EDN map")?;
    let mut output = BTreeMap::new();
    for (key, value) in taps {
        let name = scalar(key, "tap name")?;
        let values = map(value, "tap declaration must be an EDN map")?;
        let tap = Tap {
            name: name.clone(),
            registry: strings(required(values, "registry")?, "tap :registry")?,
            identity: strings(required(values, "identity")?, "tap :identity")?,
            identity_key: string(required(values, "identity-key")?, "tap :identity-key")?,
            trust: match lookup(values, "trust") {
                Some(Form::Keyword(value)) if value == "github-governed" => {
                    TrustMode::GithubGoverned
                }
                Some(Form::Keyword(value)) if value == "signed-root" => TrustMode::SignedRoot,
                Some(_) => return Err("tap :trust must be :signed-root or :github-governed".into()),
                None => TrustMode::SignedRoot,
            },
        };
        validate_tap(&tap)?;
        output.insert(name, tap);
    }
    Ok(output)
}

pub fn trusted(root: &Path, name: &str) -> Result<Tap, String> {
    load(root)?
        .remove(name)
        .ok_or_else(|| format!("tap is not trusted: {name}; add it with `hara package tap add`"))
}

pub fn trusted_or_builtin(root: &Path, name: &str) -> Result<Tap, String> {
    if name == "official" {
        return Ok(Tap {
            name: "official".into(),
            registry: vec!["https://packages.hara-lang.org".into()],
            identity: vec!["https://packages.hara-lang.org/identity.git".into()],
            identity_key: format!("sha256:{}", "00".repeat(32)),
            trust: TrustMode::SignedRoot,
        });
    }
    trusted(root, name)
}

/// Creates the two local repositories that make up a new tap.
///
/// The caller supplies only public key material.  `HARA_SIGNER` signs the
/// initial identity policy, so no private key can enter either repository.
pub fn initialize(
    name: &str,
    registry: &Path,
    identity: &Path,
    root_key: &str,
) -> Result<InitializedTap, String> {
    if !valid_name(name) {
        return Err("tap name must contain only lowercase letters, numbers, and hyphens".into());
    }
    let root_key = read_hex(root_key, "identity root public key")?;
    if root_key.len() != 32 {
        return Err("identity root public key must be 32-byte Ed25519 hex".into());
    }
    empty_directory(registry, "registry")?;
    empty_directory(identity, "identity")?;
    let policy = format!(
        "{{:identity/format 1\n :identity/root-key \"{}\"\n :publisher-keys {{}}}}\n",
        hex(&root_key)
    );
    let (_, signature) = sign(policy.as_bytes())?;
    verify(&root_key, policy.as_bytes(), &signature)?;
    initialize_signed(name, registry, identity, &root_key, &policy, &signature)
}

/// Writes an already signed initial policy. This is public for embedders that
/// use a signer API rather than the `HARA_SIGNER` command protocol.
pub fn initialize_signed(
    name: &str,
    registry: &Path,
    identity: &Path,
    root_key: &[u8],
    policy: &str,
    signature: &str,
) -> Result<InitializedTap, String> {
    if !valid_name(name) {
        return Err("tap name must contain only lowercase letters, numbers, and hyphens".into());
    }
    if root_key.len() != 32 {
        return Err("identity root public key must be 32 bytes".into());
    }
    verify(root_key, policy.as_bytes(), signature)?;
    fs::write(identity.join("identity.edn"), policy).map_err(io)?;
    fs::write(identity.join("identity.edn.sig"), format!("{signature}\n")).map_err(io)?;
    fs::write(identity.join("README.md"), identity_readme(name)).map_err(io)?;
    fs::create_dir_all(registry.join("requests")).map_err(io)?;
    fs::write(registry.join("requests/.gitkeep"), "").map_err(io)?;
    fs::write(
        registry.join("registry.edn"),
        registry_document(name, identity, &root_key),
    )
    .map_err(io)?;
    fs::write(registry.join("README.md"), registry_readme(name)).map_err(io)?;
    fs::create_dir_all(registry.join(".github/workflows")).map_err(io)?;
    fs::write(
        registry.join(".github/workflows/verify-request.yml"),
        registry_workflow(),
    )
    .map_err(io)?;
    let fingerprint = format!("sha256:{}", sha256_hex(&root_key));
    let tap = Tap {
        name: name.into(),
        registry: vec![registry.to_string_lossy().into_owned()],
        identity: vec![identity.to_string_lossy().into_owned()],
        identity_key: fingerprint.clone(),
        trust: TrustMode::SignedRoot,
    };
    Ok(InitializedTap { tap, fingerprint })
}

/// Fetches identity policy from any configured mirror and verifies the policy
/// signature against the local, out-of-band fingerprint before reading grants.
pub fn fetch_verified_policy(tap: &Tap, scratch: &Path) -> Result<IdentityPolicy, String> {
    let checkout = scratch.join("identity");
    clone_first(&tap.identity, &checkout, "identity")?;
    let bytes = fs::read(checkout.join("identity.edn"))
        .map_err(|error| format!("identity policy is missing identity.edn: {error}"))?;
    let text = std::str::from_utf8(&bytes).map_err(|_| "identity.edn must be UTF-8")?;
    let document = parse(text).map_err(|error| format!("identity.edn: {error}"))?;
    let entries = map(&document, "identity.edn must be an EDN map")?;
    match tap.trust {
        TrustMode::SignedRoot => {
            let signature = fs::read_to_string(checkout.join("identity.edn.sig"))
                .map_err(|error| format!("identity policy is missing identity.edn.sig: {error}"))?;
            let root_public = read_hex(
                &string(
                    required(entries, "identity/root-key")?,
                    "identity :identity/root-key",
                )?,
                "identity root key",
            )?;
            if sha256_hex(&root_public) != tap.identity_key.trim_start_matches("sha256:") {
                return Err(
                    "identity policy root key does not match the locally pinned tap fingerprint"
                        .into(),
                );
            }
            verify(&root_public, &bytes, signature.trim())?;
        }
        TrustMode::GithubGoverned => verify_official_hara_policy(tap, entries)?,
    }
    let revision = git(&checkout, ["rev-parse", "HEAD"])?;
    let keys = lookup(entries, "publisher-keys")
        .or_else(|| lookup(entries, "keys"))
        .ok_or("identity policy is missing :publisher-keys or :keys")?;
    let keys = map(keys, "identity publisher keys must be an EDN map")?;
    let mut publisher_keys = BTreeMap::new();
    for (id, value) in keys {
        let id = scalar(id, "publisher key id")?;
        let entry = map(value, "publisher key must be an EDN map")?;
        publisher_keys.insert(
            id,
            PublisherKey {
                public_key: string(required(entry, "public-key")?, "publisher :public-key")?,
                coordinates: lookup(entry, "coordinates")
                    .map(|value| strings(value, "publisher :coordinates"))
                    .transpose()?
                    .unwrap_or_default(),
                revoked: matches!(lookup(entry, "revoked"), Some(Form::Bool(true))),
            },
        );
    }
    Ok(IdentityPolicy {
        revision,
        publisher_keys,
    })
}

pub fn authorize(
    policy: &IdentityPolicy,
    key_id: &str,
    coordinate: &str,
    intent: &[u8],
    signature: &str,
) -> Result<(), String> {
    let key = policy
        .publisher_keys
        .get(key_id)
        .ok_or_else(|| format!("identity policy does not authorize publisher key: {key_id}"))?;
    if key.revoked {
        return Err(format!("publisher key is revoked: {key_id}"));
    }
    if !key
        .coordinates
        .iter()
        .any(|candidate| candidate == coordinate)
    {
        return Err(format!(
            "publisher key {key_id} is not authorized for {coordinate}"
        ));
    }
    verify(
        &read_hex(&key.public_key, "publisher public key")?,
        intent,
        signature,
    )
}

/// The external signer receives canonical intent bytes on stdin and returns
/// `{:key/id "..." :signature "<hex-ed25519-signature>"}` on stdout.
pub fn sign(intent: &[u8]) -> Result<(String, String), String> {
    let signer = env::var("HARA_SIGNER")
        .map_err(|_| "HARA_SIGNER must name an external signer command".to_owned())?;
    let mut child = Command::new(signer)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|error| format!("cannot start HARA_SIGNER: {error}"))?;
    use std::io::Write;
    child
        .stdin
        .as_mut()
        .ok_or("cannot open signer stdin")?
        .write_all(intent)
        .map_err(|error| format!("cannot write publisher intent to signer: {error}"))?;
    let output = child
        .wait_with_output()
        .map_err(|error| format!("cannot wait for signer: {error}"))?;
    if !output.status.success() {
        return Err(format!("external signer failed with {}", output.status));
    }
    let response =
        parse(std::str::from_utf8(&output.stdout).map_err(|_| "signer response must be UTF-8")?)
            .map_err(|error| format!("signer response: {error}"))?;
    let response = map(&response, "signer response must be an EDN map")?;
    Ok((
        string(required(response, "key/id")?, "signer :key/id")?,
        string(required(response, "signature")?, "signer :signature")?,
    ))
}

pub fn canonical_intent(
    coordinate: &str,
    version: &str,
    repository: &str,
    tag: &str,
    commit: &str,
    archive_sha256: &str,
    tap: &str,
    identity_revision: &str,
) -> String {
    format!("{{:intent/format 1 :tap \"{tap}\" :coordinate \"{coordinate}\" :version \"{version}\" :repository \"{repository}\" :tag \"{tag}\" :commit \"{commit}\" :archive-sha256 \"sha256:{archive_sha256}\" :identity-revision \"{identity_revision}\"}}\n")
}

pub fn canonical_recipe_intent(
    coordinate: &str,
    version: &str,
    repository: &str,
    tag: &str,
    commit: &str,
    recipe_sha256: &str,
    tap: &str,
    identity_revision: &str,
) -> String {
    format!("{{:intent/format 2 :tap \"{tap}\" :coordinate \"{coordinate}\" :version \"{version}\" :repository \"{repository}\" :tag \"{tag}\" :commit \"{commit}\" :recipe-sha256 \"sha256:{recipe_sha256}\" :identity-revision \"{identity_revision}\"}}\n")
}

pub fn git(
    root: &Path,
    arguments: impl IntoIterator<Item = impl AsRef<std::ffi::OsStr>>,
) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(arguments)
        .output()
        .map_err(|error| format!("cannot run git: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

pub fn clone_first(mirrors: &[String], destination: &Path, label: &str) -> Result<(), String> {
    let mut errors = Vec::new();
    for mirror in mirrors {
        if destination.exists() {
            let _ = fs::remove_dir_all(destination);
        }
        let output = Command::new("git")
            .args(["clone", "--depth", "1", mirror])
            .arg(destination)
            .output()
            .map_err(|error| format!("cannot run git: {error}"))?;
        if output.status.success() {
            return Ok(());
        }
        errors.push(format!(
            "{mirror}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Err(format!(
        "cannot fetch {label} from any configured mirror: {}",
        errors.join("; ")
    ))
}

fn save(root: &Path, taps: &BTreeMap<String, Tap>) -> Result<(), String> {
    fs::create_dir_all(root)
        .map_err(|error| format!("cannot create {}: {error}", root.display()))?;
    let mut text = String::from("{:tap-store/format 1\n :taps {\n");
    for (name, tap) in taps {
        let trust = match tap.trust {
            TrustMode::SignedRoot => "signed-root",
            TrustMode::GithubGoverned => "github-governed",
        };
        text.push_str(&format!(
            "  \"{name}\" {{:registry {} :identity {} :identity-key \"{}\" :trust :{trust}}}\n",
            vector(&tap.registry),
            vector(&tap.identity),
            tap.identity_key
        ));
    }
    text.push_str(" }}\n");
    fs::write(root.join("taps.edn"), text)
        .map_err(|error| format!("cannot write tap store: {error}"))
}

fn empty_directory(path: &Path, label: &str) -> Result<(), String> {
    if path.exists() {
        let mut entries = fs::read_dir(path).map_err(io)?;
        if entries.next().is_some() {
            return Err(format!(
                "{label} directory must be empty: {}",
                path.display()
            ));
        }
    } else {
        fs::create_dir_all(path).map_err(io)?;
    }
    Ok(())
}

fn identity_readme(name: &str) -> String {
    format!("# {name} identity policy\n\nThis repository contains public keys and signed policy only. Do not add private keys.\n\n`identity.edn` is signed by the root key declared in the document. Add publisher grants under `:publisher-keys`, then re-sign the exact file through the external identity signer.\n")
}
fn registry_document(name: &str, identity: &Path, root_key: &[u8]) -> String {
    format!("{{:registry/format 1\n :tap \"{name}\"\n :identity {{:repository \"{}\" :root-key-sha256 \"sha256:{}\"}}\n :packages {{}}}}\n", identity.display(), sha256_hex(root_key))
}
fn registry_readme(name: &str) -> String {
    format!("# {name} package registry\n\nPublication requests are submitted below `requests/` as a canonical publisher intent plus detached signature. Protect `main` and require CI review. CI must verify the paired identity policy, validate the signed source tag, rebuild the HARP archive, and create its own registry attestation before merging a release record.\n")
}
fn registry_workflow() -> &'static str {
    "name: Verify package request\non:\n  pull_request:\n    paths: [\"requests/**\"]\npermissions:\n  contents: read\njobs:\n  verify:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n      - name: Verify signed request\n        run: |\n          echo 'Install a pinned hara CLI and invoke your registry verifier here.'\n          echo 'Do not expose publishing or signing credentials to this job.'\n          exit 1\n"
}

fn vector(values: &[String]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|value| format!("\"{value}\""))
            .collect::<Vec<_>>()
            .join(" ")
    )
}
fn validate_tap(tap: &Tap) -> Result<(), String> {
    if !valid_name(&tap.name) || tap.registry.is_empty() || tap.identity.is_empty() {
        return Err(
            "tap requires a lowercase name plus at least one registry and identity mirror".into(),
        );
    }
    match tap.trust {
        TrustMode::SignedRoot
            if read_hex(
                tap.identity_key.trim_start_matches("sha256:"),
                "tap identity key fingerprint",
            )?
            .len()
                != 32 =>
        {
            Err("tap identity key fingerprint must be SHA-256 hex".into())
        }
        TrustMode::GithubGoverned
            if tap.name != "hara"
                || !tap
                    .registry
                    .iter()
                    .any(|url| url.contains("github.com/hara-lang/hara-packages"))
                || !tap
                    .identity
                    .iter()
                    .any(|url| url.contains("github.com/hara-lang/hara-identity")) =>
        {
            Err("github-governed trust is reserved for the built-in hara profile".into())
        }
        _ => Ok(()),
    }
}
fn verify_official_hara_policy(tap: &Tap, entries: &[(Form, Form)]) -> Result<(), String> {
    if tap.name != "hara"
        || !matches!(lookup(entries, "identity/name"), Some(Form::String(value)) if value == "hara")
        || !matches!(lookup(entries, "identity/trust"), Some(Form::Keyword(value)) if value == "github-governed")
    {
        return Err("GitHub-governed trust only accepts the canonical hara identity policy".into());
    }
    Ok(())
}
fn valid_name(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|value| value.is_ascii_lowercase() || value.is_ascii_digit() || value == '-')
}
fn verify(public_key: &[u8], message: &[u8], signature: &str) -> Result<(), String> {
    let key = VerifyingKey::from_bytes(
        &public_key
            .try_into()
            .map_err(|_| "Ed25519 public key must be 32 bytes")?,
    )
    .map_err(|error| format!("invalid Ed25519 public key: {error}"))?;
    let signature = Signature::from_bytes(
        &read_hex(signature, "Ed25519 signature")?
            .try_into()
            .map_err(|_| "Ed25519 signature must be 64 bytes")?,
    );
    key.verify(message, &signature)
        .map_err(|_| "Ed25519 signature verification failed".into())
}
fn read_hex(value: &str, label: &str) -> Result<Vec<u8>, String> {
    let value = value.trim().trim_start_matches("sha256:");
    if value.len() % 2 != 0 {
        return Err(format!("{label} must be hexadecimal"));
    }
    (0..value.len())
        .step_by(2)
        .map(|index| {
            u8::from_str_radix(&value[index..index + 2], 16)
                .map_err(|_| format!("{label} must be hexadecimal"))
        })
        .collect()
}
fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
fn io(error: std::io::Error) -> String {
    error.to_string()
}
fn map<'a>(form: &'a Form, message: &str) -> Result<&'a Vec<(Form, Form)>, String> {
    if let Form::Map(entries) = form {
        Ok(entries)
    } else {
        Err(message.into())
    }
}
fn lookup<'a>(entries: &'a [(Form, Form)], key: &str) -> Option<&'a Form> {
    entries
        .iter()
        .find(|(candidate, _)| matches!(candidate, Form::Keyword(value) if value == key))
        .map(|(_, value)| value)
}
fn required<'a>(entries: &'a [(Form, Form)], key: &str) -> Result<&'a Form, String> {
    lookup(entries, key).ok_or_else(|| format!("missing required key :{key}"))
}
fn scalar(form: &Form, label: &str) -> Result<String, String> {
    match form {
        Form::String(value) | Form::Symbol(value) => Ok(value.clone()),
        _ => Err(format!("{label} must be a string or symbol")),
    }
}
fn string(form: &Form, label: &str) -> Result<String, String> {
    match form {
        Form::String(value) => Ok(value.clone()),
        _ => Err(format!("{label} must be a string")),
    }
}
fn strings(form: &Form, label: &str) -> Result<Vec<String>, String> {
    match form {
        Form::Vector(values) => values.iter().map(|value| string(value, label)).collect(),
        _ => Err(format!("{label} must be a vector of strings")),
    }
}
