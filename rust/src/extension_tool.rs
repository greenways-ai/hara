//! Local extension package commands for the native CLI.

use crate::kernel::{parse, Form};
use crate::native_extension::ExtensionPackage;
use crate::project;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn run(args: &[String], allow_process: bool) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("check") => check(package_argument(args, "check")?),
        Some("build") => build(package_argument(args, "build")?, allow_process),
        Some("install") => install(package_argument(args, "install")?),
        Some("test") => test(package_argument(args, "test")?),
        Some("--help") | Some("-h") | None => {
            usage();
            Ok(())
        }
        Some(command) => Err(format!("unknown extension command: {command}")),
    }
}

fn check(root: PathBuf) -> Result<(), String> {
    let package = ExtensionPackage::load(&root)?;
    println!(
        "Extension {}{} {} is valid ({} files)",
        package.manifest.namespace,
        package
            .manifest
            .identity
            .as_ref()
            .map(|identity| format!(" [{identity}]"))
            .unwrap_or_default(),
        package.manifest.version,
        package.declared_files().len()
    );
    Ok(())
}

fn build(root: PathBuf, allow_process: bool) -> Result<(), String> {
    let descriptor = root.join("hara.build.edn");
    let source = fs::read_to_string(&descriptor)
        .map_err(|error| format!("cannot read {}: {error}", descriptor.display()))?;
    let form = parse(&source)?;
    let entries = map(&form, "hara.build.edn")?;
    let adapter = keyword(required(entries, "adapter")?, "adapter")?;
    let output = string(required(entries, "output")?, "output")?;
    let result = match adapter {
        "prebuilt" => root.join(output),
        "command" => {
            if !allow_process {
                return Err("extension/capability-denied: build requires --allow-process".into());
            }
            let command = vector(required(entries, "command")?, "command")?
                .iter()
                .map(|value| string(value, "command").map(str::to_owned))
                .collect::<Result<Vec<_>, _>>()?;
            let (program, arguments) = command
                .split_first()
                .ok_or("hara.build.edn :command cannot be empty")?;
            let working = optional(entries, "working-directory")
                .map(|value| string(value, "working-directory"))
                .transpose()?
                .unwrap_or(".");
            let status = Command::new(program)
                .args(arguments)
                .current_dir(root.join(working))
                .status()
                .map_err(|error| format!("cannot start extension build: {error}"))?;
            if !status.success() {
                return Err(format!("extension build failed with status {status}"));
            }
            root.join(output)
        }
        value => return Err(format!("unsupported extension build adapter: :{value}")),
    };
    let package = ExtensionPackage::load(&result)?;
    println!(
        "Built {} at {}",
        package.manifest.namespace,
        result.display()
    );
    Ok(())
}

fn install(source: PathBuf) -> Result<(), String> {
    let package = ExtensionPackage::load(&source)?;
    let project = project::read(Path::new("."))?;
    let root = project
        .extension_paths
        .first()
        .cloned()
        .unwrap_or_else(|| PathBuf::from("extensions"));
    let destination = project
        .root
        .join(root)
        .join(package.manifest.namespace.replace('.', "/"));
    if destination.exists() {
        return Err(format!(
            "Extension is already installed: {}",
            destination.display()
        ));
    }
    for relative in package.declared_files() {
        let source = if relative == "hara.extension.edn" {
            package.descriptor.clone()
        } else {
            package.resolve(&relative)?
        };
        let target = destination.join(&relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(io_error)?;
        }
        fs::copy(&source, &target).map_err(io_error)?;
    }
    println!(
        "Installed {} at {}",
        package.manifest.namespace,
        destination.display()
    );
    Ok(())
}

fn test(root: PathBuf) -> Result<(), String> {
    let package = ExtensionPackage::load(&root)?;
    println!(
        "Extension {} passed package validation",
        package.manifest.namespace
    );
    Ok(())
}

fn package_argument(args: &[String], operation: &str) -> Result<PathBuf, String> {
    if args.len() != 2 {
        return Err(format!(
            "extension {operation} requires one package directory"
        ));
    }
    let path = PathBuf::from(&args[1]);
    if !path.is_dir() {
        return Err(format!("not an extension directory: {}", path.display()));
    }
    Ok(path)
}

fn usage() {
    println!("hara extension check PACKAGE");
    println!("hara --allow-process extension build SOURCE");
    println!("hara extension install PACKAGE");
    println!("hara --allow-process extension test PACKAGE");
}

fn map<'a>(form: &'a Form, name: &str) -> Result<&'a [(Form, Form)], String> {
    match form {
        Form::Map(entries) => Ok(entries),
        _ => Err(format!("{name} must be a map")),
    }
}

fn vector<'a>(form: &'a Form, name: &str) -> Result<&'a [Form], String> {
    match form {
        Form::Vector(values) => Ok(values),
        _ => Err(format!(":{name} must be a vector")),
    }
}

fn required<'a>(entries: &'a [(Form, Form)], name: &str) -> Result<&'a Form, String> {
    optional(entries, name).ok_or_else(|| format!("hara.build.edn requires :{name}"))
}

fn optional<'a>(entries: &'a [(Form, Form)], name: &str) -> Option<&'a Form> {
    entries.iter().find_map(|(key, value)| {
        matches!(key, Form::Keyword(candidate) if candidate == name).then_some(value)
    })
}

fn keyword<'a>(form: &'a Form, name: &str) -> Result<&'a str, String> {
    match form {
        Form::Keyword(value) => Ok(value),
        _ => Err(format!(":{name} must be a keyword")),
    }
}

fn string<'a>(form: &'a Form, name: &str) -> Result<&'a str, String> {
    match form {
        Form::String(value) if !value.is_empty() => Ok(value),
        _ => Err(format!(":{name} must be a non-empty string")),
    }
}

fn io_error(error: std::io::Error) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::{check, test};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "hara-extension-tool-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("hara.extension.edn"),
            r#"{:namespace "demo.fixture"
                 :version "0.1.0"
                 :provider :wasm
                 :module "fixture.wasm"
                 :abi :core.v1
                 :exports {"value" {:args [] :returns :i32}}
                 :capabilities []}"#,
        )
        .unwrap();
        fs::write(root.join("fixture.wasm"), b"\0asm").unwrap();
        root
    }

    #[test]
    fn check_and_test_accept_a_complete_local_package() {
        let root = fixture();
        assert!(check(root.clone()).is_ok());
        assert!(test(root.clone()).is_ok());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn check_rejects_a_missing_declared_module() {
        let root = fixture();
        fs::remove_file(root.join("fixture.wasm")).unwrap();
        let error = check(root.clone()).unwrap_err();
        assert!(error.contains("extension/asset-unavailable"));
        fs::remove_dir_all(root).unwrap();
    }
}
