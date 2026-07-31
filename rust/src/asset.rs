//! Local versioned asset collection tooling.

use crate::kernel::{parse, Form};
use crate::project;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetCollection {
    pub root: PathBuf,
    pub coordinate: String,
    pub version: String,
    pub entries: Vec<AssetEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetEntry {
    pub path: String,
    pub media_type: String,
}

pub fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        None | Some("--help" | "-h") => {
            usage();
            Ok(())
        }
        Some("check") => {
            let collection = read_collection(path_arg(args, 1))?;
            verify_files(&collection)?;
            println!(
                "asset check: {} {} files={}",
                collection.coordinate,
                collection.version,
                collection.entries.len()
            );
            Ok(())
        }
        Some("build") => {
            let collection = read_collection(path_arg(args, 1))?;
            let output = option(args, "--output")
                .map(PathBuf::from)
                .unwrap_or_else(|| collection.root.join("target/asset-manifest.edn"));
            let manifest = build_manifest(&collection)?;
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent).map_err(io)?;
            }
            fs::write(&output, manifest).map_err(io)?;
            println!("asset build: {}", output.display());
            Ok(())
        }
        Some("inspect") => {
            let input = args.get(1).ok_or("asset inspect requires MANIFEST")?;
            print!("{}", fs::read_to_string(input).map_err(io)?);
            Ok(())
        }
        Some("publish" | "status" | "search" | "info" | "pull" | "sync" | "yank") => Err(
            format!(
                "unavailable: hara asset {} requires the packages.hara-lang.org registry client",
                args[0]
            ),
        ),
        Some(command) => Err(format!("unknown asset command: {command}")),
    }
}

pub fn read_collection(input: &Path) -> Result<AssetCollection, String> {
    let descriptor = if input.is_dir() {
        input.join("asset.edn")
    } else {
        input.to_owned()
    };
    let root = descriptor
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let source = fs::read_to_string(&descriptor)
        .map_err(|error| format!("cannot read {}: {error}", descriptor.display()))?;
    let form = parse(&source).map_err(|error| format!("{}: {error}", descriptor.display()))?;
    let map = as_map(&form, "asset.edn must be an EDN map")?;
    match required(map, "asset/format")? {
        Form::Number(1) => {}
        _ => return Err("asset.edn :asset/format must be 1".into()),
    }
    let coordinate = project::normalize_coordinate(&string(
        required(map, "asset/coordinate")?,
        ":asset/coordinate",
    )?)?;
    let version = string(required(map, "asset/version")?, ":asset/version")?;
    semver::Version::parse(&version)
        .map_err(|error| format!("asset.edn :asset/version: {error}"))?;
    let entries = vector(required(map, "asset/entries")?, ":asset/entries")?
        .iter()
        .map(|entry| {
            let entry = as_map(entry, "asset entry must be a map")?;
            Ok(AssetEntry {
                path: safe_path(&string(required(entry, "entry/path")?, ":entry/path")?)?,
                media_type: string(
                    required(entry, "entry/media-type")?,
                    ":entry/media-type",
                )?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    if entries.is_empty() {
        return Err("asset.edn :asset/entries must not be empty".into());
    }
    let mut names = std::collections::BTreeSet::new();
    if entries.iter().any(|entry| !names.insert(&entry.path)) {
        return Err("asset.edn contains duplicate :entry/path values".into());
    }
    Ok(AssetCollection {
        root,
        coordinate,
        version,
        entries,
    })
}

pub fn build_manifest(collection: &AssetCollection) -> Result<String, String> {
    verify_files(collection)?;
    let mut entries = collection.entries.clone();
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    let mut output = format!(
        "{{:asset/format 1\n :asset/coordinate {}\n :asset/version {}\n :asset/entries [\n",
        edn_string(&collection.coordinate),
        edn_string(&collection.version)
    );
    for entry in entries {
        let bytes = fs::read(collection.root.join(&entry.path)).map_err(io)?;
        output.push_str(&format!(
            "  {{:entry/path {} :entry/media-type {} :entry/size {} :entry/sha256 \"sha256:{}\"}}\n",
            edn_string(&entry.path),
            edn_string(&entry.media_type),
            bytes.len(),
            sha256(&bytes)
        ));
    }
    output.push_str(" ]}\n");
    Ok(output)
}

fn verify_files(collection: &AssetCollection) -> Result<(), String> {
    for entry in &collection.entries {
        let path = collection.root.join(&entry.path);
        if !path.is_file() {
            return Err(format!("asset entry does not exist: {}", path.display()));
        }
    }
    Ok(())
}

fn safe_path(value: &str) -> Result<String, String> {
    let path = Path::new(value);
    if value.is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(format!("unsafe asset path: {value}"));
    }
    Ok(value.replace('\\', "/"))
}

fn as_map<'a>(form: &'a Form, message: &str) -> Result<&'a [(Form, Form)], String> {
    match form {
        Form::Map(values) => Ok(values),
        _ => Err(message.into()),
    }
}

fn required<'a>(values: &'a [(Form, Form)], key: &str) -> Result<&'a Form, String> {
    values
        .iter()
        .find_map(|(candidate, value)| {
            matches!(candidate, Form::Keyword(name) if name == key).then_some(value)
        })
        .ok_or_else(|| format!("asset.edn is missing :{key}"))
}

fn string(form: &Form, label: &str) -> Result<String, String> {
    match form {
        Form::String(value) => Ok(value.clone()),
        Form::Symbol(value) => Ok(value.clone()),
        _ => Err(format!("{label} must be a string or symbol")),
    }
}

fn vector<'a>(form: &'a Form, label: &str) -> Result<&'a [Form], String> {
    match form {
        Form::Vector(values) => Ok(values),
        _ => Err(format!("{label} must be a vector")),
    }
}

fn sha256(bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(bytes);
    digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn path_arg(args: &[String], index: usize) -> &Path {
    args.get(index)
        .filter(|value| !value.starts_with('-'))
        .map(Path::new)
        .unwrap_or_else(|| Path::new("."))
}

fn option<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter()
        .position(|value| value == name)
        .and_then(|index| args.get(index + 1))
        .map(String::as_str)
}

fn edn_string(value: &str) -> String {
    format!(
        "\"{}\"",
        value.replace('\\', "\\\\").replace('"', "\\\"")
    )
}

fn io(error: std::io::Error) -> String {
    error.to_string()
}

fn usage() {
    println!("hara asset check [PATH]");
    println!("hara asset build [PATH] [--output PATH]");
    println!("hara asset inspect MANIFEST");
    println!("hara asset <publish|status|search|info|pull|sync|yank>");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn builds_a_stable_digest_manifest_and_rejects_unsafe_paths() {
        let root = std::env::temp_dir().join(format!(
            "hara-assets-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("images")).unwrap();
        fs::write(root.join("images/hero.png"), b"png").unwrap();
        fs::write(
            root.join("asset.edn"),
            "{:asset/format 1 :asset/coordinate \"alice/gallery\" :asset/version \"1.0.0\" :asset/entries [{:entry/path \"images/hero.png\" :entry/media-type \"image/png\"}]}\n",
        )
        .unwrap();
        let collection = read_collection(&root).unwrap();
        let first = build_manifest(&collection).unwrap();
        let second = build_manifest(&collection).unwrap();
        assert_eq!(first, second);
        assert!(first.contains(":asset/coordinate \"hara:alice/gallery\""));
        assert!(first.contains("sha256:"));
        fs::write(
            root.join("asset.edn"),
            "{:asset/format 1 :asset/coordinate \"alice/gallery\" :asset/version \"1.0.0\" :asset/entries [{:entry/path \"../escape\" :entry/media-type \"application/octet-stream\"}]}\n",
        )
        .unwrap();
        assert!(read_collection(&root).unwrap_err().contains("unsafe asset path"));
        fs::remove_dir_all(root).unwrap();
    }
}
