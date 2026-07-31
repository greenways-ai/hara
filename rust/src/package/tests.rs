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
    fs::write(
        root.join("project.lock.edn"),
        "{:lock/format 1 :packages {}}\n",
    )
    .unwrap();
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
    fs::write(
        root.join("project.lock.edn"),
        "{:lock/format 1 :packages {}}\n",
    )
    .unwrap();
    fs::write(
        root.join("workspace.edn"),
        "{:hara/type :workspace :hara/version \"1.0.0\"}\n",
    )
    .unwrap();
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

#[test]
fn validates_typed_recipes_and_installs_content_addressed_roots() {
    let root = fixture();
    fs::write(root.join("hara.recipe.edn"), "{:recipe/format 1 :recipe/adapter :hal :recipe/toolchain {} :recipe/inputs {} :recipe/outputs []}\n").unwrap();
    let source = fs::read_to_string(root.join("project.edn")).unwrap();
    fs::write(
        root.join("project.edn"),
        source.trim().strip_suffix('}').unwrap().to_owned()
            + " :project/recipe \"hara.recipe.edn\"}\n",
    )
    .unwrap();
    let project = read_project(&root).unwrap();
    assert_eq!(
        validate_recipe(&project).unwrap(),
        root.join("hara.recipe.edn")
    );
    let archive = root.join("package.harp");
    build_archive(&project, &archive).unwrap();
    let dist = root.join("dist");
    let installed = install_archive_at(&archive, &dist).unwrap();
    assert!(installed.join("hara.recipe.edn").is_file());
    assert!(dist
        .join("packages/hara/example/app/1.2.3.edn")
        .is_file());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_shell_recipe_escape_hatches() {
    let root = fixture();
    fs::write(root.join("hara.recipe.edn"), "{:recipe/format 1 :recipe/adapter :hal :recipe/toolchain {} :recipe/inputs {:command [\"sh\"]} :recipe/outputs []}\n").unwrap();
    let source = fs::read_to_string(root.join("project.edn")).unwrap();
    fs::write(
        root.join("project.edn"),
        source.trim().strip_suffix('}').unwrap().to_owned()
            + " :project/recipe \"hara.recipe.edn\"}\n",
    )
    .unwrap();
    assert!(validate_recipe(&read_project(&root).unwrap())
        .unwrap_err()
        .contains("cannot declare commands"));
    fs::remove_dir_all(root).unwrap();
}
