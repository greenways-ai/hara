use ed25519_dalek::{Signer, SigningKey};
use hara_wasm::tap::{self, IdentityPolicy, PublisherKey, Tap, TrustMode};
use sha2::Digest;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "hara-tap-{name}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn local_tap_trust_store_round_trips_without_private_keys() {
    let root = temp("store");
    tap::add(
        &root,
        Tap {
            name: "acme".into(),
            registry: vec!["https://example.test/acme/packages.git".into()],
            identity: vec!["https://example.test/acme/identity.git".into()],
            identity_key: format!("sha256:{}", "11".repeat(32)),
            trust: TrustMode::SignedRoot,
        },
    )
    .unwrap();
    let loaded = tap::trusted(&root, "acme").unwrap();
    assert_eq!(loaded.registry.len(), 1);
    assert!(!fs::read_to_string(root.join("taps.edn"))
        .unwrap()
        .contains("private"));
    tap::remove(&root, "acme").unwrap();
    assert!(tap::trusted(&root, "acme").is_err());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn official_bootstrap_is_narrowly_scoped_and_accepts_read_only_mirrors() {
    let root = temp("bootstrap");
    let tap = tap::bootstrap(&root, "hara").unwrap();
    assert_eq!(tap.trust, TrustMode::GithubGoverned);
    assert!(tap.registry[0].contains("github.com/hara-lang/hara-packages"));
    let updated = tap::add_mirror(
        &root,
        "hara",
        Some("https://mirror.example.test/hara-packages.git".into()),
        None,
    )
    .unwrap();
    assert_eq!(updated.registry.len(), 2);
    assert!(tap::bootstrap(&root, "other").is_err());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn publisher_signature_requires_an_authorized_non_revoked_key() {
    let signing = SigningKey::from_bytes(&[7; 32]);
    let intent = b"{:intent/format 1 :coordinate \"acme:widgets/core\"}";
    let signature = hex(&signing.sign(intent).to_bytes());
    let mut keys = BTreeMap::new();
    keys.insert(
        "publisher-1".into(),
        PublisherKey {
            public_key: hex(&signing.verifying_key().to_bytes()),
            coordinates: vec!["acme:widgets/core".into()],
            revoked: false,
        },
    );
    let policy = IdentityPolicy {
        revision: "identity-commit".into(),
        publisher_keys: keys,
    };
    tap::authorize(
        &policy,
        "publisher-1",
        "acme:widgets/core",
        intent,
        &signature,
    )
    .unwrap();
    assert!(tap::authorize(
        &policy,
        "publisher-1",
        "other:widgets/core",
        intent,
        &signature
    )
    .is_err());
    assert!(tap::authorize(
        &policy,
        "publisher-1",
        "acme:widgets/core",
        b"changed",
        &signature
    )
    .is_err());
}

#[test]
fn signed_identity_policy_is_verified_against_the_pinned_tap_root() {
    let root = temp("identity");
    fs::create_dir_all(&root).unwrap();
    let signing = SigningKey::from_bytes(&[9; 32]);
    let public = hex(&signing.verifying_key().to_bytes());
    let policy = format!(
        "{{:identity/format 1 :identity/root-key \"{public}\" :publisher-keys {{\"publisher-1\" {{:public-key \"{public}\" :coordinates [\"acme:widgets/core\"] :revoked false}}}}}}\n"
    );
    fs::write(root.join("identity.edn"), &policy).unwrap();
    fs::write(
        root.join("identity.edn.sig"),
        hex(&signing.sign(policy.as_bytes()).to_bytes()),
    )
    .unwrap();
    for arguments in [
        ["init"].as_slice(),
        ["add", "."].as_slice(),
        [
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@example.test",
            "commit",
            "-m",
            "policy",
        ]
        .as_slice(),
    ] {
        assert!(Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(arguments)
            .status()
            .unwrap()
            .success());
    }
    let fingerprint = hex(&sha2::Sha256::digest(signing.verifying_key().to_bytes()));
    let scratch = temp("checkout");
    fs::create_dir_all(&scratch).unwrap();
    let policy = tap::fetch_verified_policy(
        &Tap {
            name: "acme".into(),
            registry: vec!["unused".into()],
            identity: vec![root.to_string_lossy().into_owned()],
            identity_key: format!("sha256:{fingerprint}"),
            trust: TrustMode::SignedRoot,
        },
        &scratch,
    )
    .unwrap();
    assert!(policy.publisher_keys.contains_key("publisher-1"));
    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(scratch).unwrap();
}

#[test]
fn signed_initializer_creates_a_pair_without_private_key_material() {
    let parent = temp("init");
    let registry = parent.join("packages");
    let identity = parent.join("identity");
    fs::create_dir_all(&registry).unwrap();
    fs::create_dir_all(&identity).unwrap();
    let signing = SigningKey::from_bytes(&[4; 32]);
    let root = signing.verifying_key().to_bytes();
    let policy = format!(
        "{{:identity/format 1 :identity/root-key \"{}\" :publisher-keys {{}}}}\n",
        hex(&root)
    );
    let initialized = tap::initialize_signed(
        "acme",
        &registry,
        &identity,
        &root,
        &policy,
        &hex(&signing.sign(policy.as_bytes()).to_bytes()),
    )
    .unwrap();
    assert_eq!(initialized.tap.name, "acme");
    assert!(identity.join("identity.edn.sig").is_file());
    assert!(registry.join("registry.edn").is_file());
    assert!(registry
        .join(".github/workflows/verify-request.yml")
        .is_file());
    assert!(!fs::read_to_string(identity.join("identity.edn"))
        .unwrap()
        .contains("private"));
    fs::remove_dir_all(parent).unwrap();
}
