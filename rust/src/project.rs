//! Project manifest discovery and editing for the native CLI.
//!
//! `project.edn` is data, never evaluator input.  Keeping this model separate
//! from `Runtime` makes command behaviour portable to other Hara hosts.

use crate::kernel::{parse, parse_forms, Form};
use crate::Runtime;
use semver::{Version, VersionReq};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

const REQUIRED: &[&str] = &[
    "hara/type",
    "hara/version",
    "project/id",
    "project/version",
    "project/source-paths",
    "project/test-paths",
    "project/extension-paths",
    "project/capabilities",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub root: PathBuf,
    pub manifest_path: PathBuf,
    pub id: String,
    pub version: Version,
    pub source_paths: Vec<PathBuf>,
    pub test_paths: Vec<PathBuf>,
    pub extension_paths: Vec<PathBuf>,
    pub artifact_paths: Vec<PathBuf>,
    pub archive_root: Option<PathBuf>,
    /// Whether the intentionally portable workspace declaration is a package
    /// resource.  This never includes a live Studio workspace or cache.
    pub package_workspace: bool,
    pub main: Option<String>,
    pub dependencies: BTreeMap<String, String>,
    pub recipe: Option<PathBuf>,
}

pub fn discover(start: &Path) -> Result<Project, String> {
    let initial = if start.is_file() {
        start
            .parent()
            .ok_or_else(|| format!("cannot determine project root for {}", start.display()))?
    } else {
        start
    };
    let mut current = initial
        .canonicalize()
        .unwrap_or_else(|_| initial.to_path_buf());
    loop {
        let manifest = current.join("project.edn");
        if manifest.is_file() {
            return read(&manifest);
        }
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => return Err(format!("no project.edn found above {}", initial.display())),
        }
    }
}

pub fn read(input: &Path) -> Result<Project, String> {
    let manifest_path = if input.is_dir() {
        input.join("project.edn")
    } else {
        input.to_path_buf()
    };
    let root = manifest_path
        .parent()
        .ok_or_else(|| {
            format!(
                "cannot determine project root for {}",
                manifest_path.display()
            )
        })?
        .to_path_buf();
    let source = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("cannot read {}: {error}", manifest_path.display()))?;
    let form = parse(&source).map_err(|error| format!("{}: {error}", manifest_path.display()))?;
    let entries = map(&form, "project.edn must be an EDN map")?;
    for key in REQUIRED {
        if lookup(entries, key).is_none() {
            return Err(format!("project.edn missing required key :{key}"));
        }
    }
    if !matches!(lookup(entries, "hara/type"), Some(Form::Keyword(value)) if value == "project") {
        return Err("project.edn :hara/type must be :project".into());
    }
    let id = scalar(
        lookup(entries, "project/id").unwrap(),
        "project.edn :project/id",
    )?;
    let version_text = string(
        lookup(entries, "project/version").unwrap(),
        "project.edn :project/version",
    )?;
    let version = Version::parse(&version_text)
        .map_err(|error| format!("project.edn :project/version is not SemVer: {error}"))?;
    let source_paths = paths(
        lookup(entries, "project/source-paths").unwrap(),
        "project/source-paths",
    )?;
    let test_paths = paths(
        lookup(entries, "project/test-paths").unwrap(),
        "project/test-paths",
    )?;
    let extension_paths = paths(
        lookup(entries, "project/extension-paths").unwrap(),
        "project/extension-paths",
    )?;
    let artifact_paths = lookup(entries, "project/artifact-paths")
        .map(|value| paths(value, "project/artifact-paths"))
        .transpose()?
        .unwrap_or_default();
    let archive_root = lookup(entries, "project/archive-root")
        .map(|value| {
            relative_path(
                &string(value, "project/archive-root")?,
                "project/archive-root",
            )
        })
        .transpose()?;
    let package_workspace = lookup(entries, "project/package")
        .map(package_workspace)
        .transpose()?
        .unwrap_or(false);
    let main = lookup(entries, "project/main")
        .map(|value| scalar(value, "project.edn :project/main"))
        .transpose()?;
    let dependencies = lookup(entries, "project/dependencies")
        .map(dependencies)
        .transpose()?
        .unwrap_or_default();
    let recipe = lookup(entries, "project/recipe")
        .map(|value| relative_path(&string(value, "project/recipe")?, "project/recipe"))
        .transpose()?;
    if let Some(path) = &recipe {
        if !root.join(path).is_file() {
            return Err(format!(
                "project.edn :project/recipe does not exist: {}",
                path.display()
            ));
        }
    }
    Ok(Project {
        root,
        manifest_path,
        id,
        version,
        source_paths,
        test_paths,
        extension_paths,
        artifact_paths,
        archive_root,
        package_workspace,
        main,
        dependencies,
        recipe,
    })
}

pub fn new_app(destination: &Path, name: &str) -> Result<Project, String> {
    if !valid_name(name) {
        return Err(
            "project name must contain only lowercase letters, numbers, and hyphens".into(),
        );
    }
    if destination.exists() {
        return Err(format!(
            "destination already exists: {}",
            destination.display()
        ));
    }
    let namespace = name.replace('-', "_");
    fs::create_dir_all(destination.join("src").join(&namespace)).map_err(io)?;
    fs::create_dir_all(destination.join("test").join(&namespace)).map_err(io)?;
    fs::create_dir_all(destination.join("extensions")).map_err(io)?;
    fs::write(destination.join("project.edn"), format!(
        "{{:hara/type :project\n :hara/version \"1.0.0\"\n :project/id {name}\n :project/version \"0.1.0\"\n :project/source-paths [\"src\"]\n :project/test-paths [\"test\"]\n :project/extension-paths [\"extensions\"]\n :project/main {namespace}.main\n :project/capabilities #{{}}\n :project/dependencies {{}}}}\n"
    )).map_err(io)?;
    fs::write(
        destination.join("workspace.edn"),
        "{:hara/type :workspace :hara/version \"1.0.0\"}\n",
    )
    .map_err(io)?;
    fs::write(
        destination.join("src").join(&namespace).join("main.hal"),
        format!("(ns {namespace}.main)\n\n(defn main []\n  \"Hello from {name}\")\n\n(main)\n"),
    )
    .map_err(io)?;
    fs::write(destination.join("test").join(&namespace).join("main_test.hal"), format!("(ns {namespace}.main-test\n  (:require [std.lib.test :as test]))\n\n(test/print-results\n [(test/check \"starter project runs\" true true)])\n")).map_err(io)?;
    read(&destination.join("project.edn"))
}

pub fn set_dependency(
    project: &Project,
    coordinate: &str,
    version: Option<&str>,
) -> Result<(), String> {
    validate_coordinate(coordinate)?;
    if let Some(version) = version {
        VersionReq::parse(version)
            .map_err(|error| format!("invalid dependency range {version}: {error}"))?;
    }
    let source = fs::read_to_string(&project.manifest_path).map_err(io)?;
    let mut form =
        parse(&source).map_err(|error| format!("{}: {error}", project.manifest_path.display()))?;
    let entries = map_mut(&mut form, "project.edn must be an EDN map")?;
    let dependency_index = entries
        .iter()
        .position(|(key, _)| key_name(key).as_deref() == Some("project/dependencies"));
    let dependency_form = dependency_index.map(|index| &mut entries[index].1);
    let deps = match dependency_form {
        Some(Form::Map(entries)) => entries,
        Some(_) => return Err("project.edn :project/dependencies must be an EDN map".into()),
        None => {
            entries.push((
                Form::Keyword("project/dependencies".into()),
                Form::Map(Vec::new()),
            ));
            match &mut entries.last_mut().unwrap().1 {
                Form::Map(entries) => entries,
                _ => unreachable!(),
            }
        }
    };
    if let Some(index) = deps.iter().position(|(key, _)| {
        scalar(key, "dependency coordinate").ok().as_deref() == Some(coordinate)
    }) {
        if let Some(version) = version {
            deps[index].1 = Form::Map(vec![(
                Form::Keyword("version".into()),
                Form::String(version.into()),
            )]);
        } else {
            deps.remove(index);
        }
    } else if let Some(version) = version {
        deps.push((
            Form::String(coordinate.into()),
            Form::Map(vec![(
                Form::Keyword("version".into()),
                Form::String(version.into()),
            )]),
        ));
    }
    deps.sort_by(|left, right| left.0.to_string().cmp(&right.0.to_string()));
    fs::write(&project.manifest_path, format!("{form}\n")).map_err(io)
}

pub fn files_in(root: &Path, paths: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let mut output = Vec::new();
    for relative in paths {
        collect_hal(&root.join(relative), &mut output)?;
    }
    output.sort();
    Ok(output)
}

/// Registers namespaces from `:project/source-paths` for runtime `require`.
pub fn register_sources(project: &Project, runtime: &mut Runtime) -> Result<(), String> {
    for path in files_in(&project.root, &project.source_paths)? {
        let source = fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let namespace = declared_namespace(&source)
            .map_err(|error| format!("{}: {error}", path.display()))?
            .ok_or_else(|| format!("{} does not declare an ns or ns+ namespace", path.display()))?;
        runtime.register_resource(&namespace, &source);
    }
    Ok(())
}

pub fn main_file(project: &Project) -> Result<PathBuf, String> {
    let namespace = project
        .main
        .as_ref()
        .ok_or_else(|| "project.edn is missing :project/main".to_owned())?;
    let relative = format!("{}.hal", namespace.replace('.', "/").replace('-', "_"));
    for source in &project.source_paths {
        let candidate = project.root.join(source).join(&relative);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err(format!(
        "cannot find :project/main {namespace} in :project/source-paths"
    ))
}

fn declared_namespace(source: &str) -> Result<Option<String>, String> {
    Ok(parse_forms(source)?.into_iter().find_map(|form| match form {
        Form::List(values) if matches!(values.first(), Some(Form::Symbol(head)) if head == "ns" || head == "ns+") => {
            match values.get(1) { Some(Form::Symbol(namespace)) => Some(namespace.clone()), _ => None }
        }
        _ => None,
    }))
}

/// Creates or validates the lockfile for graphs that need no remote packages.
/// Remote graphs deliberately stop here until the reviewed registry and
/// identity clients can provide the required signed release metadata.
pub fn sync_lock(project: &Project, mode: LockMode) -> Result<PathBuf, String> {
    let lock = project.root.join("project.lock.edn");
    if !project.dependencies.is_empty() {
        return Err(format!(
            "project sync requires the reviewed registry client to resolve {} declared dependencies",
            project.dependencies.len()
        ));
    }
    match mode {
        LockMode::Locked | LockMode::Frozen if !lock.is_file() => {
            return Err(format!(
                "{} requires an existing project.lock.edn",
                mode.flag()
            ));
        }
        LockMode::Locked | LockMode::Frozen => validate_empty_lock(&lock)?,
        LockMode::Default | LockMode::Offline => {
            fs::write(&lock, "{:lock/format 1 :packages {}}\n")
                .map_err(|error| format!("cannot write {}: {error}", lock.display()))?;
        }
    }
    Ok(lock)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockMode {
    Default,
    Offline,
    Locked,
    Frozen,
}

impl LockMode {
    pub fn flag(self) -> &'static str {
        match self {
            Self::Default => "sync",
            Self::Offline => "--offline",
            Self::Locked => "--locked",
            Self::Frozen => "--frozen",
        }
    }
}

fn collect_hal(directory: &Path, output: &mut Vec<PathBuf>) -> Result<(), String> {
    if !directory.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(directory).map_err(io)? {
        let path = entry.map_err(io)?.path();
        if path.is_dir() {
            collect_hal(&path, output)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some("hal") {
            output.push(path);
        }
    }
    Ok(())
}

fn validate_empty_lock(path: &Path) -> Result<(), String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let form = parse(&source).map_err(|error| format!("{}: {error}", path.display()))?;
    let entries = map(&form, "project.lock.edn must be an EDN map")?;
    if matches!(lookup(entries, "lock/format"), Some(Form::Number(1)))
        && matches!(lookup(entries, "packages"), Some(Form::Map(entries)) if entries.is_empty())
    {
        Ok(())
    } else {
        Err(format!(
            "{} is not a lockfile written by this CLI",
            path.display()
        ))
    }
}

fn map<'a>(form: &'a Form, message: &str) -> Result<&'a Vec<(Form, Form)>, String> {
    if let Form::Map(entries) = form {
        Ok(entries)
    } else {
        Err(message.into())
    }
}
fn map_mut<'a>(form: &'a mut Form, message: &str) -> Result<&'a mut Vec<(Form, Form)>, String> {
    if let Form::Map(entries) = form {
        Ok(entries)
    } else {
        Err(message.into())
    }
}
fn key_name(key: &Form) -> Option<String> {
    match key {
        Form::Keyword(value) => Some(value.clone()),
        _ => None,
    }
}
fn lookup<'a>(entries: &'a [(Form, Form)], key: &str) -> Option<&'a Form> {
    entries
        .iter()
        .find(|(candidate, _)| key_name(candidate).as_deref() == Some(key))
        .map(|(_, value)| value)
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
fn relative_path(value: &str, label: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(value);
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        Err(format!(
            "project.edn :{label} cannot escape the project root"
        ))
    } else {
        Ok(path)
    }
}
fn paths(form: &Form, label: &str) -> Result<Vec<PathBuf>, String> {
    match form {
        Form::Vector(values) => values
            .iter()
            .map(|value| relative_path(&string(value, &format!("project.edn :{label}"))?, label))
            .collect(),
        _ => Err(format!("project.edn :{label} must be a vector of strings")),
    }
}
fn package_workspace(form: &Form) -> Result<bool, String> {
    let entries = map(form, "project.edn :project/package must be an EDN map")?;
    match lookup(entries, "workspace") {
        None | Some(Form::Bool(false)) => Ok(false),
        Some(Form::Bool(true)) => Ok(true),
        Some(_) => Err("project.edn :project/package :workspace must be a boolean".into()),
    }
}
fn dependencies(form: &Form) -> Result<BTreeMap<String, String>, String> {
    let mut output = BTreeMap::new();
    for (key, value) in map(form, "project.edn :project/dependencies must be an EDN map")? {
        let coordinate = normalize_coordinate(&scalar(key, "dependency coordinate")?)?;
        let version = lookup(
            map(value, "dependency declaration must be an EDN map")?,
            "version",
        )
        .ok_or_else(|| format!("dependency {coordinate} is missing :version"))?;
        let version = string(version, "dependency :version")?;
        VersionReq::parse(&version)
            .map_err(|error| format!("invalid dependency range {version}: {error}"))?;
        output.insert(coordinate, version);
    }
    Ok(output)
}
pub fn normalize_coordinate(value: &str) -> Result<String, String> {
    let qualified = if let Some(package) = value.strip_prefix("official:") {
        format!("hara:{package}")
    } else if value.contains(':') {
        value.to_owned()
    } else {
        format!("hara:{value}")
    };
    let (tap, package) = qualified
        .split_once(':')
        .ok_or_else(|| format!("invalid package coordinate: {value}"))?;
    let mut parts = package.split('/');
    let valid = !tap.is_empty()
        && tap.chars().all(valid_coordinate_char)
        && matches!((parts.next(), parts.next(), parts.next()), (Some(owner), Some(name), None) if !owner.is_empty() && !name.is_empty() && owner.chars().all(valid_coordinate_char) && name.chars().all(valid_coordinate_char));
    if valid {
        Ok(qualified)
    } else {
        Err(format!("invalid package coordinate: {value}"))
    }
}
fn validate_coordinate(value: &str) -> Result<(), String> {
    normalize_coordinate(value).map(|_| ())
}
fn valid_coordinate_char(value: char) -> bool {
    value.is_ascii_lowercase() || value.is_ascii_digit() || matches!(value, '-' | '_' | '.')
}
fn valid_name(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|value| value.is_ascii_lowercase() || value.is_ascii_digit() || value == '-')
}
fn io(error: std::io::Error) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    fn temp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "hara-project-{name}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }
    #[test]
    fn scaffolds_discovers_and_edits_dependencies() {
        let root = temp("app");
        let project = new_app(&root, "hello-app").unwrap();
        assert_eq!(
            discover(&root.join("src/hello_app")).unwrap().id,
            "hello-app"
        );
        set_dependency(&project, "hara:hara/graph", Some("^1.2.0")).unwrap();
        assert_eq!(
            read(&root).unwrap().dependencies["hara:hara/graph"],
            "^1.2.0"
        );
        set_dependency(&project, "hara:hara/graph", None).unwrap();
        assert!(read(&root).unwrap().dependencies.is_empty());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn rejects_escaping_source_paths() {
        let root = temp("unsafe");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("project.edn"), "{:hara/type :project :hara/version \"1\" :project/id x :project/version \"1.0.0\" :project/source-paths [\"../src\"] :project/test-paths [] :project/extension-paths [] :project/capabilities #{}}" ).unwrap();
        assert!(read(&root).unwrap_err().contains("cannot escape"));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn creates_and_validates_an_empty_lock() {
        let root = temp("lock");
        let project = new_app(&root, "lock-app").unwrap();
        let lock = sync_lock(&project, LockMode::Default).unwrap();
        assert_eq!(
            fs::read_to_string(&lock).unwrap(),
            "{:lock/format 1 :packages {}}\n"
        );
        sync_lock(&project, LockMode::Frozen).unwrap();
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn registers_project_sources_for_cross_file_requires() {
        let root = temp("resources");
        fs::create_dir_all(root.join("src/demo")).unwrap();
        fs::write(root.join("project.edn"), "{:hara/type :project :hara/version \"1.0.0\" :project/id demo/app :project/version \"1.0.0\" :project/source-paths [\"src\"] :project/test-paths [] :project/extension-paths [] :project/capabilities #{}}").unwrap();
        fs::write(
            root.join("src/demo/helper.hal"),
            "(ns demo.helper) (defn answer [] 42)",
        )
        .unwrap();
        let project = read(&root).unwrap();
        let mut runtime = Runtime::new();
        register_sources(&project, &mut runtime).unwrap();
        assert_eq!(
            runtime
                .eval_native("(ns demo.main (:require [demo.helper :as helper])) (helper/answer)")
                .unwrap(),
            "42"
        );
        fs::remove_dir_all(root).unwrap();
    }
}
