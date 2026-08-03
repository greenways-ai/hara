//! Publication-time native crate composition for verified HARP package roots.

use hara_abi::NativeIdentity;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeArtifact {
    pub identity: NativeIdentity,
    pub version: String,
    pub archive_sha256: String,
    pub package_root: PathBuf,
    pub crate_path: PathBuf,
}

impl NativeArtifact {
    pub fn verified(
        identity: NativeIdentity,
        version: impl Into<String>,
        archive_sha256: impl Into<String>,
        package_root: impl Into<PathBuf>,
        crate_relative: impl AsRef<Path>,
    ) -> Result<Self, String> {
        let version = version.into();
        let archive_sha256 = archive_sha256.into();
        if version.is_empty() {
            return Err("native artifact version must not be empty".into());
        }
        let digest = archive_sha256
            .strip_prefix("sha256:")
            .ok_or("native artifact digest must start with sha256:")?;
        if digest.len() != 64 || !digest.chars().all(|value| value.is_ascii_hexdigit()) {
            return Err("native artifact digest must contain 64 hexadecimal characters".into());
        }
        let package_root = package_root.into();
        let relative = crate_relative.as_ref();
        if relative.is_absolute()
            || relative
                .components()
                .any(|component| matches!(component, std::path::Component::ParentDir))
        {
            return Err("native crate path must remain beneath the package root".into());
        }
        let crate_path = package_root.join(relative);
        if !crate_path.join("Cargo.toml").is_file() {
            return Err(format!(
                "native crate has no Cargo.toml: {}",
                crate_path.display()
            ));
        }
        Ok(Self {
            identity,
            version,
            archive_sha256,
            package_root,
            crate_path,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkPlan {
    pub artifacts: Vec<NativeArtifact>,
}

impl LinkPlan {
    pub fn new(mut artifacts: Vec<NativeArtifact>) -> Result<Self, String> {
        artifacts.sort_by(|left, right| left.identity.cmp(&right.identity));
        for pair in artifacts.windows(2) {
            if pair[0].identity == pair[1].identity {
                return Err(format!(
                    "duplicate native adapter {} {}",
                    pair[0].identity.package, pair[0].identity.export
                ));
            }
        }
        Ok(Self { artifacts })
    }

    pub fn cargo_dependencies(&self) -> String {
        let mut dependencies = BTreeMap::new();
        for artifact in &self.artifacts {
            dependencies.insert(
                artifact.identity.crate_name.as_str(),
                artifact.crate_path.to_string_lossy(),
            );
        }
        let mut output = String::from("[dependencies]\n");
        for (crate_name, path) in dependencies {
            output.push_str(&format!(
                "{crate_name} = {{ path = {:?} }}\n",
                toml_string(&path)
            ));
        }
        output
    }

    /// Generates deterministic installation statements for a composed host.
    pub fn registration_source(&self, runtime_expression: &str) -> String {
        let mut output = String::new();
        for artifact in &self.artifacts {
            let crate_name = artifact.identity.crate_name.replace('-', "_");
            output.push_str(&format!(
                "{runtime_expression}.install_native_module({crate_name}::module());\n"
            ));
        }
        output
    }
}

fn toml_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn verified_artifacts_generate_deterministic_cargo_dependencies() {
        let root = std::env::temp_dir().join(format!(
            "hara-native-link-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("crate")).unwrap();
        fs::write(
            root.join("crate/Cargo.toml"),
            "[package]\nname='adapter'\nversion='0.1.0'\n",
        )
        .unwrap();
        let identity = NativeIdentity::new(
            "gh:example:adapter",
            "service/store",
            "adapter",
            "service/1",
        )
        .unwrap();
        let artifact = NativeArtifact::verified(
            identity,
            "0.1.0",
            format!("sha256:{}", "a".repeat(64)),
            &root,
            "crate",
        )
        .unwrap();
        let plan = LinkPlan::new(vec![artifact]).unwrap();
        assert!(plan.cargo_dependencies().contains("adapter = { path ="));
        assert_eq!(
            plan.registration_source("runtime"),
            "runtime.install_native_module(adapter::module());\n"
        );
        fs::remove_dir_all(root).unwrap();
    }
}
