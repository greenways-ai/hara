use super::{
    file_sha256, io_error, read_project, split_coordinate, validate_relative_path, zip_error,
};
use crate::kernel::{parse, Form};
use crate::project::{self, Project};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use zip::ZipArchive;

pub(super) fn validate_recipe(project: &Project) -> Result<PathBuf, String> {
    let relative = project
        .recipe
        .as_ref()
        .ok_or("publication requires :project/recipe")?;
    let path = project.root.join(relative);
    let source = fs::read_to_string(&path).map_err(io_error)?;
    let Form::Map(entries) = parse(&source)? else {
        return Err("hara.recipe.edn must be an EDN map".into());
    };
    for key in [
        "recipe/format",
        "recipe/adapter",
        "recipe/toolchain",
        "recipe/inputs",
        "recipe/outputs",
    ] {
        if !entries
            .iter()
            .any(|(candidate, _)| matches!(candidate, Form::Keyword(name) if name == key))
        {
            return Err(format!("hara.recipe.edn is missing :{key}"));
        }
    }
    let adapter = entries
        .iter()
        .find(|(candidate, _)| matches!(candidate, Form::Keyword(name) if name == "recipe/adapter"))
        .map(|(_, value)| value);
    if !matches!(adapter, Some(Form::Keyword(name)) if matches!(name.as_str(), "rust-wasm" | "node-hta" | "hal"))
    {
        return Err(
            "hara.recipe.edn :recipe/adapter must be :rust-wasm, :node-hta, or :hal".into(),
        );
    }
    if source.contains(":command") || source.contains(":script") || source.contains(":shell") {
        return Err("official recipes cannot declare commands, scripts, or shell fragments".into());
    }
    Ok(path)
}

fn dist_root() -> PathBuf {
    if let Some(root) = std::env::var_os("HARA_DIST_HOME") {
        return PathBuf::from(root);
    }
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".hara/dist")
}

pub(super) fn install_archive(archive: &Path) -> Result<PathBuf, String> {
    install_archive_at(archive, &dist_root())
}

pub(super) fn install_archive_at(archive: &Path, root: &Path) -> Result<PathBuf, String> {
    let digest = file_sha256(archive)?;
    let archive_target = root.join("archives/sha256").join(format!("{digest}.harp"));
    let package_root = root.join("roots/sha256").join(&digest);
    fs::create_dir_all(archive_target.parent().unwrap()).map_err(io_error)?;
    fs::create_dir_all(package_root.parent().unwrap()).map_err(io_error)?;
    if !archive_target.exists() {
        fs::copy(archive, &archive_target).map_err(io_error)?;
    }
    if !package_root.exists() {
        let scratch = root
            .join("roots/sha256")
            .join(format!(".{digest}.tmp-{}", std::process::id()));
        if scratch.exists() {
            fs::remove_dir_all(&scratch).map_err(io_error)?;
        }
        fs::create_dir_all(&scratch).map_err(io_error)?;
        let mut zip =
            ZipArchive::new(File::open(&archive_target).map_err(io_error)?).map_err(zip_error)?;
        for index in 0..zip.len() {
            let mut entry = zip.by_index(index).map_err(zip_error)?;
            let relative = entry
                .enclosed_name()
                .ok_or("archive contains an unsafe path")?
                .to_path_buf();
            validate_relative_path(&relative)?;
            if entry.is_dir() {
                fs::create_dir_all(scratch.join(relative)).map_err(io_error)?;
                continue;
            }
            let output = scratch.join(relative);
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent).map_err(io_error)?;
            }
            let mut file = File::create(output).map_err(io_error)?;
            std::io::copy(&mut entry, &mut file).map_err(io_error)?;
        }
        fs::rename(&scratch, &package_root).map_err(io_error)?;
    }
    let project = read_project(&package_root)?;
    let coordinate = project::normalize_coordinate(&project.id)?;
    let (tap, package) = split_coordinate(&coordinate)?;
    let mut parts = package.split('/');
    let registration = root
        .join("packages")
        .join(tap)
        .join(parts.next().unwrap())
        .join(parts.next().unwrap())
        .join(format!("{}.edn", project.version));
    fs::create_dir_all(registration.parent().unwrap()).map_err(io_error)?;
    fs::write(
        &registration,
        format!(
            "{{:coordinate \"{}\" :version \"{}\" :archive-sha256 \"sha256:{}\" :root \"{}\"}}\n",
            coordinate,
            project.version,
            digest,
            package_root.display()
        ),
    )
    .map_err(io_error)?;
    Ok(package_root)
}

pub(super) fn json_string(value: &str) -> String {
    format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
    )
}
