#![cfg(not(target_arch = "wasm32"))]

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::extension::ExtensionManifest;

const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;
const MAX_MODULE_BYTES: u64 = 64 * 1024 * 1024;

pub struct ExtensionPackage {
    pub descriptor: PathBuf,
    pub source: String,
    pub manifest: ExtensionManifest,
}

impl ExtensionPackage {
    pub fn discover(namespace: &str, roots: &[PathBuf]) -> Result<Option<Self>, String> {
        let relative = PathBuf::from(namespace.replace('.', "/")).join("hara.extension.edn");
        let mut candidates = Vec::new();
        for root in roots {
            let root = absolute(root)?;
            let descriptor = root.join(&relative);
            if descriptor.is_file() {
                let descriptor = descriptor
                    .canonicalize()
                    .map_err(|error| format!("extension/asset-unavailable: {error}"))?;
                if !descriptor.starts_with(&root) {
                    return Err(format!("extension/path-denied: {namespace}"));
                }
                candidates.push(descriptor);
            }
        }
        candidates.sort();
        candidates.dedup();
        if candidates.is_empty() {
            return Ok(None);
        }
        if candidates.len() != 1 {
            return Err(format!(
                "extension/ambiguous: multiple packages export {namespace}: {candidates:?}"
            ));
        }
        let descriptor = candidates.remove(0);
        let metadata = descriptor
            .metadata()
            .map_err(|error| format!("extension/asset-unavailable: {error}"))?;
        if metadata.len() > MAX_MANIFEST_BYTES {
            return Err(format!(
                "extension/malformed {}: manifest is too large",
                descriptor.display()
            ));
        }
        let source = fs::read_to_string(&descriptor)
            .map_err(|error| format!("extension/malformed {}: {error}", descriptor.display()))?;
        let manifest = ExtensionManifest::parse(&source, &descriptor.display().to_string())?;
        if manifest.namespace != namespace {
            return Err(format!(
                "extension/malformed {}: expected namespace {namespace}",
                descriptor.display()
            ));
        }
        let package = Self {
            descriptor,
            source,
            manifest,
        };
        package.validate_declared_files()?;
        Ok(Some(package))
    }

    pub fn module_bytes(&self) -> Result<Vec<u8>, String> {
        let module =
            self.manifest.module.as_deref().ok_or_else(|| {
                format!("extension/module-unavailable: {}", self.manifest.namespace)
            })?;
        let path = self.resolve(module)?;
        let metadata = path
            .metadata()
            .map_err(|error| format!("extension/module-unavailable: {error}"))?;
        if metadata.len() > MAX_MODULE_BYTES {
            return Err(format!("extension/module-too-large: {}", path.display()));
        }
        fs::read(&path).map_err(|error| format!("extension/module-unavailable: {error}"))
    }

    fn validate_declared_files(&self) -> Result<(), String> {
        let mut paths = self.manifest.assets.clone();
        if let Some(module) = &self.manifest.module {
            paths.push(module.clone());
        }
        paths.extend(
            self.manifest
                .targets
                .values()
                .map(|target| target.module.clone()),
        );
        for relative in paths {
            self.resolve(&relative)?;
        }
        Ok(())
    }

    pub fn resolve(&self, relative: &str) -> Result<PathBuf, String> {
        let root = self
            .descriptor
            .parent()
            .ok_or_else(|| {
                "extension/asset-unavailable: descriptor has no package directory".to_owned()
            })?
            .canonicalize()
            .map_err(|error| format!("extension/asset-unavailable: {error}"))?;
        let path = root.join(relative).canonicalize().map_err(|error| {
            format!(
                "extension/asset-unavailable: {}/{} ({error})",
                self.manifest.namespace, relative
            )
        })?;
        if !path.starts_with(&root) || !path.is_file() {
            return Err(format!("extension/path-denied: {relative}"));
        }
        Ok(path)
    }
}

pub fn configured_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(current) = env::current_dir() {
        for directory in current.ancestors() {
            if directory.join("project.edn").is_file() || directory.join("project.hal").is_file() {
                roots.push(directory.join("extensions"));
                break;
            }
        }
    }
    if let Some(configured) = env::var_os("HARA_EXTENSION_PATH") {
        roots.extend(env::split_paths(&configured));
    }
    roots
}

pub fn package_exists(namespace: &str, roots: &[PathBuf]) -> bool {
    let relative = PathBuf::from(namespace.replace('.', "/")).join("hara.extension.edn");
    roots.iter().any(|root| root.join(&relative).is_file())
}

fn absolute(path: &Path) -> Result<PathBuf, String> {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir()
            .map_err(|error| format!("extension/root-invalid: {error}"))?
            .join(path)
    };
    if path.exists() {
        path.canonicalize()
            .map_err(|error| format!("extension/root-invalid: {error}"))
    } else {
        Ok(path)
    }
}
