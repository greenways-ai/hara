//! Deterministic local package operations for the `hara package` command.
//!
//! Network reconciliation deliberately does not live here yet: package roots
//! are only activated after a registry and identity client has verified them.

use semver::{Version, VersionReq};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const REQUIRED_PROJECT_KEYS: &[&str] = &[
    ":hara/type",
    ":hara/version",
    ":project/id",
    ":project/version",
    ":project/source-paths",
    ":project/test-paths",
    ":project/extension-paths",
    ":project/capabilities",
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct Project {
    root: PathBuf,
    id: String,
    version: Version,
    source_paths: Vec<PathBuf>,
    artifact_paths: Vec<PathBuf>,
    archive_root: Option<PathBuf>,
}

/// Handles the public `hara package` command group.
pub fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("check") => {
            let root = args.get(1).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
            let project = read_project(&root)?;
            println!("package check: {} {}", project.id, project.version);
            Ok(())
        }
        Some("build") => {
            let root = args.get(1).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
            let project = read_project(&root)?;
            let output = args
                .iter()
                .position(|arg| arg == "--output")
                .and_then(|index| args.get(index + 1))
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    project
                        .root
                        .join("target")
                        .join(format!("{}-{}.harp", archive_name(&project.id), project.version))
                });
            build_archive(&project, &output)?;
            println!("package build: {}", output.display());
            Ok(())
        }
        Some("inspect") => {
            let archive = args
                .get(1)
                .ok_or_else(|| "hara package inspect requires ARCHIVE.harp".to_owned())?;
            println!("{}", inspect_archive(Path::new(archive))?);
            Ok(())
        }
        Some("sync") | Some("add") | Some("remove") | Some("update") | Some("publish")
        | Some("tap") | Some("search") | Some("info") => Err(format!(
            "hara package {} requires a configured GitHub registry and identity client; local package commands available now: check, build, inspect",
            args[0]
        )),
        Some("--help") | Some("-h") | None => {
            println!(
                "hara package <check|build|inspect|sync|add|remove|update|publish|tap|search|info>\n\n\
                 check [PATH]                 validate project.edn\n\
                 build [PATH] [--output PATH] build deterministic .harp\n\
                 inspect ARCHIVE.harp         print package.edn"
            );
            Ok(())
        }
        Some(command) => Err(format!("unknown package command: {command}")),
    }
}

fn read_project(path: &Path) -> Result<Project, String> {
    let (root, manifest_path) = if path.is_dir() {
        (path.to_path_buf(), path.join("project.edn"))
    } else {
        (
            path.parent()
                .ok_or_else(|| format!("cannot determine project root for {}", path.display()))?
                .to_path_buf(),
            path.to_path_buf(),
        )
    };
    let source = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("cannot read {}: {error}", manifest_path.display()))?;
    for key in REQUIRED_PROJECT_KEYS {
        if !source.contains(key) {
            return Err(format!("project.edn missing required key {key}"));
        }
    }
    let id = scalar_after(&source, ":project/id")
        .ok_or_else(|| "project.edn :project/id must be a symbol or string".to_owned())?;
    let version_text = scalar_after(&source, ":project/version")
        .ok_or_else(|| "project.edn :project/version must be a SemVer string".to_owned())?;
    let version = Version::parse(&version_text)
        .map_err(|error| format!("project.edn :project/version is not SemVer: {error}"))?;
    if let Some(dependencies) = map_after(&source, ":project/dependencies") {
        validate_dependencies(dependencies)?;
    }
    let source_paths = vector_after(&source, ":project/source-paths")
        .ok_or_else(|| "project.edn :project/source-paths must be a vector of strings".to_owned())?
        .into_iter()
        .map(PathBuf::from)
        .collect();
    let artifact_paths: Vec<PathBuf> = vector_after(&source, ":project/artifact-paths")
        .unwrap_or_default()
        .into_iter()
        .map(PathBuf::from)
        .collect();
    let archive_root = scalar_after(&source, ":project/archive-root").map(PathBuf::from);
    for path in artifact_paths.iter().chain(archive_root.iter()) {
        validate_relative_path(path)?;
    }
    Ok(Project {
        root,
        id,
        version,
        source_paths,
        artifact_paths,
        archive_root,
    })
}

fn validate_dependencies(source: &str) -> Result<(), String> {
    let mut cursor = source;
    while let Some(index) = cursor.find('"') {
        cursor = &cursor[index + 1..];
        let Some(end) = cursor.find('"') else {
            return Err("unterminated dependency coordinate".into());
        };
        let coordinate = &cursor[..end];
        cursor = &cursor[end + 1..];
        if !coordinate.contains('/') {
            continue;
        }
        if coordinate.starts_with('/') || coordinate.ends_with('/') {
            return Err(format!("invalid package coordinate: {coordinate}"));
        }
        if let Some(version_index) = cursor.find(":version") {
            let after = &cursor[version_index + ":version".len()..];
            if let Some(version) = scalar(after) {
                VersionReq::parse(&version)
                    .map_err(|error| format!("invalid dependency range {version}: {error}"))?;
                cursor = after;
            }
        }
    }
    Ok(())
}

fn build_archive(project: &Project, output: &Path) -> Result<(), String> {
    let mut entries = Vec::new();
    for source_path in &project.source_paths {
        let base = project.root.join(source_path);
        collect_files(&base, &project.root, false, false, &mut entries)?;
    }
    for artifact_path in &project.artifact_paths {
        let base = project.root.join(artifact_path);
        collect_files(&base, &project.root, true, true, &mut entries)?;
    }
    let mut archive_entries = Vec::new();
    for source in entries {
        let archive = match &project.archive_root {
            Some(root) => source
                .strip_prefix(root)
                .map(PathBuf::from)
                .unwrap_or_else(|_| source.clone()),
            None => source.clone(),
        };
        validate_relative_path(&archive)?;
        if archive.as_os_str().is_empty() {
            return Err("package archive path must name a file".into());
        }
        archive_entries.push((archive, source));
    }
    archive_entries.sort_by(|left, right| left.0.cmp(&right.0));
    for pair in archive_entries.windows(2) {
        if pair[0].0 == pair[1].0 {
            return Err(format!(
                "duplicate package archive path: {}",
                pair[0].0.display()
            ));
        }
    }
    if archive_entries.is_empty() {
        return Err(
            "package build found no files in :project/source-paths or :project/artifact-paths"
                .into(),
        );
    }
    let mut contents = Vec::new();
    for (archive, source) in &archive_entries {
        let bytes = fs::read(project.root.join(source))
            .map_err(|error| format!("cannot read {}: {error}", source.display()))?;
        contents.push((archive.clone(), bytes));
    }
    let package_edn = package_manifest(project, &contents);
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    let file = File::create(output)
        .map_err(|error| format!("cannot create {}: {error}", output.display()))?;
    let mut writer = ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .last_modified_time(zip::DateTime::default())
        .unix_permissions(0o644);
    writer
        .start_file("package.edn", options)
        .map_err(zip_error)?;
    writer.write_all(package_edn.as_bytes()).map_err(io_error)?;
    for (path, bytes) in contents {
        let archive_path = path_to_slash(&path)?;
        writer
            .start_file(archive_path, options)
            .map_err(zip_error)?;
        writer.write_all(&bytes).map_err(io_error)?;
    }
    writer.finish().map_err(zip_error)?;
    Ok(())
}

fn inspect_archive(path: &Path) -> Result<String, String> {
    let file =
        File::open(path).map_err(|error| format!("cannot open {}: {error}", path.display()))?;
    let mut archive = ZipArchive::new(file).map_err(zip_error)?;
    let mut manifest = archive
        .by_name("package.edn")
        .map_err(|_| "archive is missing package.edn".to_owned())?;
    let mut text = String::new();
    manifest.read_to_string(&mut text).map_err(io_error)?;
    Ok(text)
}

fn package_manifest(project: &Project, contents: &[(PathBuf, Vec<u8>)]) -> String {
    let mut hasher = Sha256::new();
    let mut files = String::new();
    for (path, bytes) in contents {
        let path = path_to_slash(path).expect("validated project-relative path");
        hasher.update(path.as_bytes());
        hasher.update([0]);
        hasher.update(bytes);
        files.push_str(&format!(
            "  \"{}\" {{:sha256 \"sha256:{}\" :size {}}}\n",
            path,
            hex(&Sha256::digest(bytes)),
            bytes.len()
        ));
    }
    format!(
        "{{:harp/format 1\n :package {{:identity \"{}\" :version \"{}\"}}\n :files {{\n{}}} :integrity {{:tree-sha256 \"sha256:{}\"}}}}\n",
        project.id,
        project.version,
        files,
        hex(&hasher.finalize())
    )
}

fn collect_files(
    directory: &Path,
    root: &Path,
    include_all: bool,
    required: bool,
    entries: &mut Vec<PathBuf>,
) -> Result<(), String> {
    if !directory.exists() {
        return if required {
            Err(format!(
                "declared package path does not exist: {}",
                directory.display()
            ))
        } else {
            Ok(())
        };
    }
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("cannot read {}: {error}", directory.display()))?
    {
        let entry = entry.map_err(io_error)?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(io_error)?;
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "package entries must not be symbolic links: {}",
                path.display()
            ));
        }
        if metadata.is_dir() {
            collect_files(&path, root, include_all, true, entries)?;
        } else if metadata.is_file()
            && (include_all
                || path.extension().and_then(|extension| extension.to_str()) == Some("hal"))
        {
            let relative = path
                .strip_prefix(root)
                .map_err(|_| "package path escapes project root".to_owned())?;
            validate_relative_path(relative)?;
            entries.push(relative.to_path_buf());
        }
    }
    Ok(())
}

fn validate_relative_path(path: &Path) -> Result<(), String> {
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(format!("unsafe package path: {}", path.display()));
    }
    Ok(())
}

fn path_to_slash(path: &Path) -> Result<String, String> {
    validate_relative_path(path)?;
    path.to_str()
        .map(|value| value.replace('\\', "/"))
        .ok_or_else(|| format!("package path is not UTF-8: {}", path.display()))
}

fn archive_name(id: &str) -> String {
    id.replace('/', "-")
}

fn scalar_after(source: &str, key: &str) -> Option<String> {
    source
        .find(key)
        .and_then(|index| scalar(&source[index + key.len()..]))
}

fn scalar(source: &str) -> Option<String> {
    let source = source.trim_start();
    if let Some(rest) = source.strip_prefix('"') {
        rest.find('"').map(|end| rest[..end].to_owned())
    } else {
        source.split_whitespace().next().map(|value| {
            value
                .trim_matches(|character| matches!(character, '}' | ']'))
                .to_owned()
        })
    }
}

fn vector_after(source: &str, key: &str) -> Option<Vec<String>> {
    let source = &source[source.find(key)? + key.len()..];
    let body = &source[source.find('[')? + 1..];
    let body = &body[..body.find(']')?];
    Some(
        body.split('"')
            .skip(1)
            .step_by(2)
            .map(str::to_owned)
            .collect(),
    )
}

fn map_after<'a>(source: &'a str, key: &str) -> Option<&'a str> {
    let source = &source[source.find(key)? + key.len()..];
    let start = source.find('{')?;
    let mut depth = 0usize;
    for (index, character) in source[start..].char_indices() {
        match character {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&source[start + 1..start + index]);
                }
            }
            _ => {}
        }
    }
    None
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
fn io_error(error: std::io::Error) -> String {
    error.to_string()
}
fn zip_error(error: zip::result::ZipError) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "hara-package-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("src/example")).unwrap();
        fs::write(root.join("src/example/main.hal"), "(ns example.main) 42\n").unwrap();
        fs::write(root.join("project.edn"), "{:hara/type :project :hara/version \"1.0.0\" :project/id example/app :project/version \"1.2.3\" :project/source-paths [\"src\"] :project/test-paths [\"test\"] :project/extension-paths [\"extensions\"] :project/capabilities #{} :project/dependencies {\"hara/graph\" {:version \"^1.2.0\"}}}").unwrap();
        root
    }

    #[test]
    fn validates_and_builds_deterministic_archive() {
        let root = fixture();
        let project = read_project(&root).unwrap();
        let first = root.join("one.harp");
        let second = root.join("two.harp");
        build_archive(&project, &first).unwrap();
        build_archive(&project, &second).unwrap();
        assert_eq!(fs::read(&first).unwrap(), fs::read(&second).unwrap());
        assert!(inspect_archive(&first).unwrap().contains(":harp/format 1"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_missing_project_keys_and_bad_ranges() {
        let root = fixture();
        fs::write(root.join("project.edn"), "{:hara/type :project}").unwrap();
        assert!(read_project(&root).unwrap_err().contains(":hara/version"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn packages_declared_artifacts_under_the_archive_root() {
        let root = fixture();
        fs::create_dir_all(root.join("target/package/ledger/noir/assets")).unwrap();
        fs::write(
            root.join("target/package/ledger/noir/hara.extension.edn"),
            "{:namespace \"ledger.noir\"}\n",
        )
        .unwrap();
        fs::write(
            root.join("target/package/ledger/noir/assets/worker.mjs"),
            "export {};\n",
        )
        .unwrap();
        fs::write(
            root.join("project.edn"),
            "{:hara/type :project :hara/version \"1.0.0\" :project/id hara/ledger-noir :project/version \"0.1.0\" :project/source-paths [] :project/test-paths [\"test\"] :project/extension-paths [\"target/package\"] :project/capabilities #{} :project/artifact-paths [\"target/package\"] :project/archive-root \"target/package\"}",
        )
        .unwrap();
        let project = read_project(&root).unwrap();
        let archive = root.join("ledger-noir.harp");
        build_archive(&project, &archive).unwrap();
        let file = File::open(&archive).unwrap();
        let mut zip = ZipArchive::new(file).unwrap();
        assert!(zip.by_name("ledger/noir/hara.extension.edn").is_ok());
        assert!(zip.by_name("ledger/noir/assets/worker.mjs").is_ok());
        assert!(zip
            .by_name("target/package/ledger/noir/hara.extension.edn")
            .is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_missing_declared_artifacts() {
        let root = fixture();
        fs::write(
            root.join("project.edn"),
            "{:hara/type :project :hara/version \"1.0.0\" :project/id example/app :project/version \"1.2.3\" :project/source-paths [] :project/test-paths [\"test\"] :project/extension-paths [\"extensions\"] :project/capabilities #{} :project/artifact-paths [\"target/package\"]}",
        )
        .unwrap();
        let project = read_project(&root).unwrap();
        assert!(build_archive(&project, &root.join("missing.harp"))
            .unwrap_err()
            .contains("does not exist"));
        fs::remove_dir_all(root).unwrap();
    }
}
