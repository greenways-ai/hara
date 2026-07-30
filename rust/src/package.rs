//! Deterministic local package operations for the `hara package` command.
//!
//! Network reconciliation deliberately does not live here yet: package roots
//! are only activated after a registry and identity client has verified them.

use crate::project::{self, Project};
use crate::tap::{self, Tap};
use crate::kernel::{parse, parse_forms, Form};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

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
        Some("publish") => publish(&args[1..]),
        Some("tap") => tap_command(&args[1..]),
        Some("registry") => registry_command(&args[1..]),
        Some("sync") | Some("add") | Some("remove") | Some("update") | Some("search") | Some("info") => Err(format!(
            "hara package {} requires a configured GitHub registry and identity client; local package commands available now: check, build, inspect",
            args[0]
        )),
        Some("--help") | Some("-h") | None => {
            println!(
                "hara package <check|build|inspect|sync|add|remove|update|publish|tap|search|info>\n\n\
                 check [PATH]                 validate project.edn\n\
                 build [PATH] [--output PATH] build deterministic .harp\n\
                 inspect ARCHIVE.harp         print package.edn\n\
                 tap bootstrap hara           install the official bootstrap profile\n\
                 tap init NAME --registry PATH --identity PATH --identity-root-key ED25519_HEX\n\
                 tap add NAME --registry URL --identity URL --identity-key SHA256\n\
                 tap mirror add NAME [--registry URL] [--identity URL]\n\
                 tap list|remove NAME|verify NAME\n\
                 publish --tap NAME [--dry-run] [PATH]"
            );
            Ok(())
        }
        Some(command) => Err(format!("unknown package command: {command}")),
    }
}

fn read_project(path: &Path) -> Result<Project, String> { project::read(path) }

fn registry_command(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("verify-request") => {
            let request = PathBuf::from(required_option(args, "--request")?);
            let identity = PathBuf::from(required_option(args, "--identity")?);
            verify_registry_request(&request, &identity)?;
            println!("registry request verified: {}", request.display());
            Ok(())
        }
        _ => Err("usage: hara package registry verify-request --request PATH --identity PATH".into()),
    }
}

fn verify_registry_request(request: &Path, identity: &Path) -> Result<(), String> {
    let policy = fs::read_to_string(identity).map_err(io_error)?;
    let Form::Map(policy) = parse(&policy)? else { return Err("identity policy must be an EDN map".into()); };
    let trust = policy.iter().find(|(key, _)| matches!(key, Form::Keyword(name) if name == "identity/trust")).map(|(_, value)| value);
    if !matches!(trust, Some(Form::Keyword(mode)) if mode == "github-governed") { return Err("registry bootstrap verifier requires :identity/trust :github-governed".into()); }
    let intent_path = fs::read_dir(request).map_err(io_error)?.filter_map(Result::ok).map(|entry| entry.path()).find(|path| path.file_name().and_then(|name| name.to_str()).is_some_and(|name| name.ends_with(".publisher-intent.edn"))).ok_or("request is missing publisher intent")?;
    let intent = fs::read_to_string(&intent_path).map_err(io_error)?;
    let Form::Map(entries) = parse(&intent)? else { return Err("publisher intent must be an EDN map".into()); };
    for key in ["intent/format", "tap", "coordinate", "version", "repository", "tag", "commit", "archive-sha256", "identity-revision"] {
        if !entries.iter().any(|(candidate, _)| matches!(candidate, Form::Keyword(name) if name == key)) { return Err(format!("publisher intent is missing :{key}")); }
    }
    Ok(())
}

fn tap_command(args: &[String]) -> Result<(), String> {
    let root = tap::config_root();
    match args.first().map(String::as_str) {
        Some("add") => {
            let name = args.get(1).ok_or_else(|| "tap add requires NAME".to_owned())?;
            let registry = option_values(args, "--registry");
            let identity = option_values(args, "--identity");
            let identity_key = option_value(args, "--identity-key")?;
            tap::add(&root, Tap { name: name.clone(), registry, identity, identity_key, trust: tap::TrustMode::SignedRoot })?;
            println!("trusted tap {name}");
            Ok(())
        }
        Some("bootstrap") => {
            let profile = args.get(1).ok_or_else(|| "tap bootstrap requires PROFILE".to_owned())?;
            let tap = tap::bootstrap(&root, profile)?;
            println!("bootstrapped tap {} (GitHub-governed)", tap.name);
            Ok(())
        }
        Some("mirror") if args.get(1).map(String::as_str) == Some("add") => {
            let name = args.get(2).ok_or_else(|| "tap mirror add requires NAME".to_owned())?;
            let tap = tap::add_mirror(&root, name, optional_option(args, "--registry"), optional_option(args, "--identity"))?;
            println!("updated tap {} registry={} identity={}", tap.name, tap.registry.join(","), tap.identity.join(","));
            Ok(())
        }
        Some("init") => {
            let name = args.get(1).ok_or_else(|| "tap init requires NAME".to_owned())?;
            let registry = PathBuf::from(required_option(args, "--registry")?);
            let identity = PathBuf::from(required_option(args, "--identity")?);
            let root_key = required_option(args, "--identity-root-key")?;
            let initialized = tap::initialize(name, &registry, &identity, &root_key)?;
            tap::add(&root, initialized.tap)?;
            println!("initialized tap {name}");
            println!("identity-root fingerprint: {}", initialized.fingerprint);
            println!("scaffolded registry: {}", registry.display());
            println!("scaffolded identity: {}", identity.display());
            Ok(())
        }
        Some("remove") => {
            let name = args.get(1).ok_or_else(|| "tap remove requires NAME".to_owned())?;
            tap::remove(&root, name)?;
            println!("removed tap {name}");
            Ok(())
        }
        Some("list") => {
            for tap in tap::load(&root)?.values() { println!("{} registry={} identity={}", tap.name, tap.registry.join(","), tap.identity.join(",")); }
            Ok(())
        }
        Some("verify") => {
            let name = args.get(1).ok_or_else(|| "tap verify requires NAME".to_owned())?;
            let tap = tap::trusted(&root, name)?;
            let scratch = scratch("verify")?;
            let result = tap::fetch_verified_policy(&tap, &scratch);
            let _ = fs::remove_dir_all(&scratch);
            let policy = result?;
            println!("tap verify: {} identity={}", tap.name, policy.revision);
            Ok(())
        }
        _ => Err("usage: hara package tap <bootstrap|init|add|mirror add|remove|list|verify>".into()),
    }
}

fn publish(args: &[String]) -> Result<(), String> {
    let tap_name = option_value(args, "--tap")?;
    let dry_run = args.iter().any(|arg| arg == "--dry-run");
    let path = args.iter().skip(1).find(|arg| !arg.starts_with('-') && *arg != &tap_name).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    let project = read_project(&path)?;
    let (coordinate_tap, _) = split_coordinate(&project.id)?;
    if coordinate_tap != tap_name { return Err(format!("project id {} belongs to tap {coordinate_tap}, not {tap_name}", project.id)); }
    let trusted_tap = tap::trusted(&tap::config_root(), &tap_name)?;
    let scratch = scratch("publish")?;
    let result = publish_inner(&project, &trusted_tap, dry_run, &scratch);
    let _ = fs::remove_dir_all(&scratch);
    result
}

fn publish_inner(project: &Project, trusted_tap: &Tap, dry_run: bool, scratch_root: &Path) -> Result<(), String> {
    let policy = tap::fetch_verified_policy(trusted_tap, scratch_root)?;
    let tag = format!("v{}", project.version);
    tap::git(&project.root, ["tag", "-v", &tag]).map_err(|error| format!("publish requires a valid signed tag {tag}: {error}"))?;
    let commit = tap::git(&project.root, ["rev-list", "-n", "1", &tag])?;
    let repository = tap::git(&project.root, ["config", "--get", "remote.origin.url"])?;
    let archive = project.root.join("target").join(format!("{}-{}.harp", archive_name(&project.id), project.version));
    build_archive(project, &archive)?;
    let archive_sha256 = file_sha256(&archive)?;
    let intent = tap::canonical_intent(&project.id, &project.version.to_string(), &repository, &tag, &commit, &archive_sha256, &trusted_tap.name, &policy.revision);
    let (key_id, signature) = tap::sign(intent.as_bytes())?;
    tap::authorize(&policy, &key_id, &project.id, intent.as_bytes(), &signature)?;
    if dry_run {
        println!("publish verified: {} {} tap={} archive=sha256:{}", project.id, project.version, trusted_tap.name, archive_sha256);
        return Ok(());
    }
    let registry = scratch_root.join("registry");
    tap::clone_first(&trusted_tap.registry, &registry, "registry")?;
    let branch = format!("requests/{}/{}", project.id.replace(':', "-").replace('/', "-"), project.version);
    tap::git(&registry, ["checkout", "-b", &branch])?;
    let request = registry.join("requests").join(&trusted_tap.name).join(project.id.split(':').nth(1).unwrap_or(&project.id));
    fs::create_dir_all(&request).map_err(io_error)?;
    let base = format!("{}-{}", archive_name(&project.id), project.version);
    fs::write(request.join(format!("{base}.publisher-intent.edn")), &intent).map_err(io_error)?;
    fs::write(request.join(format!("{base}.publisher-intent.sig")), format!("{{:key/id \"{key_id}\" :signature \"{signature}\"}}\n")).map_err(io_error)?;
    tap::git(&registry, ["add", "requests"])?;
    tap::git(&registry, ["commit", "-m", &format!("request {} {}", project.id, project.version)])?;
    tap::git(&registry, ["push", "origin", &format!("HEAD:{branch}")])?;
    let output = std::process::Command::new("gh").current_dir(&registry).args(["pr", "create", "--head", &branch, "--title", &format!("Publish {} {}", project.id, project.version), "--body", "Signed Hara package publication request."]).output().map_err(|error| format!("request branch pushed but cannot start gh pr create: {error}"))?;
    if !output.status.success() { return Err(format!("request branch pushed but GitHub PR creation failed: {}", String::from_utf8_lossy(&output.stderr).trim())); }
    println!("publish requested: {}", String::from_utf8_lossy(&output.stdout).trim());
    Ok(())
}

fn option_value(args: &[String], flag: &str) -> Result<String, String> { let index = args.iter().position(|arg| arg == flag).ok_or_else(|| format!("publish requires {flag}"))?; args.get(index + 1).cloned().ok_or_else(|| format!("{flag} requires a value")) }
fn required_option(args: &[String], flag: &str) -> Result<String, String> { let index = args.iter().position(|arg| arg == flag).ok_or_else(|| format!("tap init requires {flag}"))?; args.get(index + 1).cloned().ok_or_else(|| format!("{flag} requires a value")) }
fn option_values(args: &[String], flag: &str) -> Vec<String> { args.iter().enumerate().filter(|(_, value)| value.as_str() == flag).filter_map(|(index, _)| args.get(index + 1).cloned()).collect() }
fn optional_option(args: &[String], flag: &str) -> Option<String> { args.iter().position(|arg| arg == flag).and_then(|index| args.get(index + 1).cloned()) }
fn split_coordinate(value: &str) -> Result<(&str, &str), String> { let (tap, package) = value.split_once(':').ok_or_else(|| format!("package coordinate must use TAP:owner/name: {value}"))?; if tap.is_empty() || package.is_empty() || package.contains(':') { return Err(format!("invalid tap-qualified package coordinate: {value}")); } Ok((tap, package)) }
fn scratch(label: &str) -> Result<PathBuf, String> { let root = std::env::temp_dir().join(format!("hara-{label}-{}", std::process::id())); if root.exists() { fs::remove_dir_all(&root).map_err(io_error)?; } fs::create_dir_all(&root).map_err(io_error)?; Ok(root) }
fn file_sha256(path: &Path) -> Result<String, String> { Ok(hex(&Sha256::digest(fs::read(path).map_err(io_error)?))) }

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
    // A release archive must be self-describing.  These entries intentionally
    // stay at its root even when :project/archive-root relocates artifacts.
    entries.push(PathBuf::from("project.edn"));
    let lock = project.root.join("project.lock.edn");
    if lock.is_file() {
        entries.push(PathBuf::from("project.lock.edn"));
    } else if !project.dependencies.is_empty() {
        return Err("package build requires project.lock.edn when :project/dependencies is non-empty".into());
    }
    if project.package_workspace {
        let workspace = project.root.join("workspace.edn");
        if !workspace.is_file() {
            return Err("project.edn declares :project/package {:workspace true}, but workspace.edn is missing".into());
        }
        entries.push(PathBuf::from("workspace.edn"));
    }
    let mut archive_entries = Vec::new();
    for source in entries {
        let archive = if matches!(source.as_path(), path if path == Path::new("project.edn") || path == Path::new("project.lock.edn") || path == Path::new("workspace.edn")) {
            source.clone()
        } else { match &project.archive_root {
            Some(root) => source
                .strip_prefix(root)
                .map(PathBuf::from)
                .unwrap_or_else(|_| source.clone()),
            None => source.clone(),
        }};
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
    let package_edn = package_manifest(project, &contents)?;
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

fn package_manifest(project: &Project, contents: &[(PathBuf, Vec<u8>)]) -> Result<String, String> {
    let mut hasher = Sha256::new();
    let mut files = String::new();
    let mut resources = Vec::new();
    let mut extensions = Vec::new();
    for (path, bytes) in contents {
        let path = path_to_slash(path).expect("validated project-relative path");
        hasher.update(path.as_bytes());
        hasher.update([0]);
        hasher.update(bytes);
        files.push_str(&format!(
            "  {} {{:sha256 \"sha256:{}\" :size {}}}\n",
            edn_string(&path),
            hex(&Sha256::digest(bytes)),
            bytes.len()
        ));
        if path.ends_with(".hal") {
            let source = std::str::from_utf8(bytes)
                .map_err(|_| format!("HAL package resource is not UTF-8: {path}"))?;
            if let Some(namespace) = hal_namespace(source)
                .map_err(|error| format!("cannot parse package resource {path}: {error}"))?
            {
                resources.push((namespace, path.clone()));
            }
        } else if path.ends_with(".hir") {
            let module = crate::kernel::hir::decode_hir(bytes)
                .map_err(|error| format!("cannot decode package resource {path}: {error}"))?;
            resources.push((module.namespace, path.clone()));
        }
        if path.ends_with("hara.extension.edn") {
            extensions.push(path.clone());
        }
    }
    resources.sort();
    for pair in resources.windows(2) {
        if pair[0].0 == pair[1].0 {
            return Err(format!("duplicate package namespace: {}", pair[0].0));
        }
    }
    extensions.sort();
    let resources = resources
        .iter()
        .map(|(namespace, path)| {
            format!("  {} {}\n", edn_string(namespace), edn_string(path))
        })
        .collect::<String>();
    let extensions = extensions
        .iter()
        .map(|path| edn_string(path))
        .collect::<Vec<_>>()
        .join(" ");
    Ok(format!(
        "{{:harp/format 1\n :package {{:identity {} :version {}}}\n :files {{\n{}}} :resources {{\n{}}} :extensions [{}]\n :integrity {{:tree-sha256 \"sha256:{}\"}}}}\n",
        edn_string(&project.id),
        edn_string(&project.version.to_string()),
        files,
        resources,
        extensions,
        hex(&hasher.finalize())
    ))
}

fn hal_namespace(source: &str) -> Result<Option<String>, String> {
    for form in parse_forms(source)? {
        let Form::List(forms) = form else { continue };
        let [Form::Symbol(head), Form::Symbol(namespace), ..] = forms.as_slice() else {
            continue;
        };
        if head == "ns" || head == "ns+" {
            return Ok(Some(namespace.clone()));
        }
    }
    Ok(None)
}

fn edn_string(value: &str) -> String {
    Form::String(value.to_owned()).to_string()
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
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

    fn fixture() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "hara-package-{}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("src/example")).unwrap();
        fs::write(root.join("src/example/main.hal"), "(ns example.main) 42\n").unwrap();
        fs::write(root.join("project.edn"), "{:hara/type :project :hara/version \"1.0.0\" :project/id example/app :project/version \"1.2.3\" :project/source-paths [\"src\"] :project/test-paths [\"test\"] :project/extension-paths [\"extensions\"] :project/capabilities #{} :project/dependencies {\"hara:hara/graph\" {:version \"^1.2.0\"}}}").unwrap();
        fs::write(root.join("project.lock.edn"), "{:lock/format 1 :packages {}}\n").unwrap();
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
        let manifest = inspect_archive(&first).unwrap();
        assert!(manifest.contains(":harp/format 1"));
        assert!(manifest.contains("\"example.main\" \"src/example/main.hal\""));
        let file = File::open(&first).unwrap();
        let mut zip = ZipArchive::new(file).unwrap();
        assert!(zip.by_name("project.edn").is_ok());
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

    #[test]
    fn packages_lock_and_explicit_portable_workspace_only() {
        let root = fixture();
        fs::write(root.join("project.lock.edn"), "{:lock/format 1 :packages {}}\n").unwrap();
        fs::write(root.join("workspace.edn"), "{:hara/type :workspace :hara/version \"1.0.0\"}\n").unwrap();
        let undeclared = root.join("undeclared-workspace.harp");
        build_archive(&read_project(&root).unwrap(), &undeclared).unwrap();
        let file = File::open(&undeclared).unwrap();
        let mut zip = ZipArchive::new(file).unwrap();
        assert!(zip.by_name("workspace.edn").is_err());
        fs::write(
            root.join("project.edn"),
            "{:hara/type :project :hara/version \"1.0.0\" :project/id example/app :project/version \"1.2.3\" :project/source-paths [\"src\"] :project/test-paths [\"test\"] :project/extension-paths [\"extensions\"] :project/capabilities #{} :project/dependencies {\"hara:hara/graph\" {:version \"^1.2.0\"}} :project/package {:workspace true}}",
        )
        .unwrap();
        let archive = root.join("workspace.harp");
        build_archive(&read_project(&root).unwrap(), &archive).unwrap();
        let file = File::open(&archive).unwrap();
        let mut zip = ZipArchive::new(file).unwrap();
        assert!(zip.by_name("project.lock.edn").is_ok());
        assert!(zip.by_name("workspace.edn").is_ok());
        fs::remove_dir_all(root).unwrap();
    }
}
