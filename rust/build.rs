use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn collect_hal(directory: &Path, output: &mut Vec<PathBuf>) {
    let mut entries = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", directory.display()))
        .map(|entry| entry.expect("cannot read HAL resource entry").path())
        .collect::<Vec<_>>();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect_hal(&path, output);
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("hal") {
            output.push(path);
        }
    }
}

fn declared_namespace(source: &str, path: &Path) -> String {
    for line in source.lines() {
        let line = line.trim_start();
        let remainder = line
            .strip_prefix("(ns ")
            .or_else(|| line.strip_prefix("(ns+ "));
        if let Some(remainder) = remainder {
            let namespace = remainder
                .split(|character: char| character.is_whitespace() || character == ')')
                .next()
                .unwrap_or_default();
            if !namespace.is_empty() {
                return namespace.to_owned();
            }
        }
    }
    panic!(
        "{} does not declare an ns or ns+ namespace on its own line",
        path.display()
    );
}

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let source_root = manifest.join("../lib/src");
    println!("cargo:rerun-if-changed={}", source_root.display());

    let mut paths = Vec::new();
    collect_hal(&source_root, &mut paths);
    let mut resources = BTreeMap::new();
    for path in paths {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let namespace = declared_namespace(&source, &path);
        if let Some(previous) = resources.insert(namespace.clone(), path.clone()) {
            panic!(
                "duplicate HAL namespace {namespace}: {} and {}",
                previous.display(),
                path.display()
            );
        }
    }

    let repository_root = manifest
        .parent()
        .expect("rust crate must have a repository parent");
    let mut generated =
        String::from("pub(crate) static EMBEDDED_HAL_RESOURCES: &[(&str, &str, &str)] = &[\n");
    for (namespace, path) in resources {
        let relative = path
            .strip_prefix(repository_root)
            .expect("HAL resource must be inside the repository")
            .to_string_lossy()
            .replace('\\', "/");
        let path = path
            .canonicalize()
            .unwrap_or_else(|error| panic!("cannot resolve {}: {error}", path.display()));
        generated.push_str(&format!(
            "    ({namespace:?}, {relative:?}, include_str!({path:?})),\n",
            namespace = namespace,
            relative = relative,
            path = path.to_string_lossy()
        ));
    }
    generated.push_str("];\n");

    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("embedded_hal.rs");
    fs::write(&output, generated)
        .unwrap_or_else(|error| panic!("cannot write {}: {error}", output.display()));
}
