use hara_wasm::kernel::{parse, Form};
use hara_wasm::SessionKernel;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestSummary {
    pub path: PathBuf,
    pub passed: bool,
    pub facts: usize,
    pub checks: usize,
    pub passed_checks: usize,
    pub failed_checks: usize,
    pub errors: usize,
    pub timeouts: usize,
    pub raw: String,
}

impl TestSummary {
    pub fn failure_message(&self) -> String {
        format!("{} failed: {}", self.path.display(), self.raw)
    }
}

pub fn run_file(root: &Path, file: &Path) -> Result<TestSummary, String> {
    let root = root
        .canonicalize()
        .map_err(|error| format!("cannot resolve {}: {error}", root.display()))?;
    let file = file
        .canonicalize()
        .map_err(|error| format!("cannot resolve {}: {error}", file.display()))?;
    if file.extension().and_then(|value| value.to_str()) != Some("hal") {
        return Err(format!("test file must use the .hal extension: {}", file.display()));
    }
    let source = fs::read_to_string(&file)
        .map_err(|error| format!("cannot read {}: {error}", file.display()))?;

    let mut kernel = SessionKernel::new();
    let root_text = root
        .to_str()
        .ok_or_else(|| format!("project root is not UTF-8: {}", root.display()))?;
    let mount = kernel.create_native_filesystem(root_text);
    kernel.attach_filesystem("ROOT", mount)?;
    let output = kernel.eval("ROOT", &source)?;
    parse_summary(file, &output)
}

pub fn run_paths(root: &Path, paths: &[PathBuf]) -> Result<Vec<TestSummary>, String> {
    let mut files = Vec::new();
    for path in paths {
        collect(path, &mut files)?;
    }
    files.sort();
    files.dedup();
    if files.is_empty() {
        return Err("no .hal test files found".into());
    }
    files
        .iter()
        .map(|file| run_file(root, file))
        .collect::<Result<Vec<_>, _>>()
}

fn collect(path: &Path, output: &mut Vec<PathBuf>) -> Result<(), String> {
    if path.is_file() {
        if path.extension().and_then(|value| value.to_str()) == Some("hal") {
            output.push(path.to_path_buf());
        }
        return Ok(());
    }
    if !path.is_dir() {
        return Err(format!("test path not found: {}", path.display()));
    }
    let mut entries = fs::read_dir(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?
        .map(|entry| entry.map(|value| value.path()).map_err(|error| error.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort();
    for entry in entries {
        collect(&entry, output)?;
    }
    Ok(())
}

fn parse_summary(path: PathBuf, output: &str) -> Result<TestSummary, String> {
    let mut form = parse(output).map_err(|error| format!("invalid test result: {error}"))?;
    if let Form::String(encoded) = form {
        form = parse(&encoded).map_err(|error| format!("invalid encoded test result: {error}"))?;
    }

    match form {
        Form::Map(entries) => parse_code_test_summary(path, entries, output),
        Form::Vector(items) | Form::List(items) => parse_legacy_results(path, items, output),
        _ => Err("test file must return code.test/run summary or test/print-results vector".into()),
    }
}

fn parse_code_test_summary(
    path: PathBuf,
    entries: Vec<(Form, Form)>,
    raw: &str,
) -> Result<TestSummary, String> {
    let status = map_get(&entries, "status")
        .and_then(keyword)
        .ok_or("code.test/run result is missing :status")?;
    let counts = match map_get(&entries, "counts") {
        Some(Form::Map(values)) => values,
        _ => return Err("code.test/run result is missing :counts".into()),
    };
    let passed_facts = map_number(counts, "passed", 0);
    let failed_facts = map_number(counts, "failed", 0);
    let errors = map_number(counts, "error", 0);
    let timeouts = map_number(counts, "timeout", 0);
    let skipped = map_number(counts, "skipped", 0);
    let cancelled = map_number(counts, "cancelled", 0);
    let facts = map_number(
        &entries,
        "facts",
        passed_facts + failed_facts + errors + timeouts + skipped + cancelled,
    );
    let passed_checks = map_number(&entries, "passed", passed_facts);
    let failed_checks = map_number(&entries, "failed", failed_facts);
    let checks = map_number(&entries, "checks", passed_checks + failed_checks);

    Ok(TestSummary {
        path,
        passed: status == "passed",
        facts,
        checks,
        passed_checks,
        failed_checks,
        errors,
        timeouts,
        raw: raw.to_owned(),
    })
}

fn parse_legacy_results(
    path: PathBuf,
    items: Vec<Form>,
    raw: &str,
) -> Result<TestSummary, String> {
    let mut passed = 0usize;
    let mut failed = 0usize;
    for item in items {
        let Form::Map(entries) = item else {
            return Err("legacy test result must be a map".into());
        };
        match map_get(&entries, "pass") {
            Some(Form::Bool(true)) => passed += 1,
            Some(Form::Bool(false)) => failed += 1,
            _ => return Err("legacy test result is missing boolean :pass".into()),
        }
    }
    Ok(TestSummary {
        path,
        passed: failed == 0,
        facts: passed + failed,
        checks: passed + failed,
        passed_checks: passed,
        failed_checks: failed,
        errors: 0,
        timeouts: 0,
        raw: raw.to_owned(),
    })
}

fn map_get<'a>(entries: &'a [(Form, Form)], key: &str) -> Option<&'a Form> {
    entries.iter().find_map(|(candidate, value)| match candidate {
        Form::Keyword(name) if name == key => Some(value),
        _ => None,
    })
}

fn map_number(entries: &[(Form, Form)], key: &str, fallback: usize) -> usize {
    match map_get(entries, key) {
        Some(Form::Number(value)) if *value >= 0 => *value as usize,
        _ => fallback,
    }
}

fn keyword(form: &Form) -> Option<&str> {
    match form {
        Form::Keyword(value) => Some(value),
        _ => None,
    }
}

fn parse_arguments() -> Result<(PathBuf, Vec<PathBuf>), String> {
    let mut root = env::current_dir().map_err(|error| error.to_string())?;
    let mut paths = Vec::new();
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == "--root" {
            let value = arguments.next().ok_or("--root requires a path")?;
            root = PathBuf::from(value);
        } else if argument == "--help" || argument == "-h" {
            return Err("usage: hara-test [--root ROOT] FILE_OR_DIRECTORY...".into());
        } else {
            paths.push(PathBuf::from(argument));
        }
    }
    if paths.is_empty() {
        let default = root.join("test");
        if default.exists() {
            paths.push(default);
        } else {
            return Err("usage: hara-test [--root ROOT] FILE_OR_DIRECTORY...".into());
        }
    }
    let paths = paths
        .into_iter()
        .map(|path| if path.is_absolute() { path } else { root.join(path) })
        .collect();
    Ok((root, paths))
}

fn main() {
    let (root, paths) = match parse_arguments() {
        Ok(value) => value,
        Err(error) => {
            eprintln!("hara-test: {error}");
            std::process::exit(2);
        }
    };
    let summaries = match run_paths(&root, &paths) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("hara-test: {error}");
            std::process::exit(2);
        }
    };

    let mut failed = 0usize;
    for summary in &summaries {
        if summary.passed {
            println!(
                "PASS  {} ({} facts, {} checks)",
                summary.path.display(),
                summary.facts,
                summary.checks
            );
        } else {
            failed += 1;
            println!("FAIL  {}", summary.path.display());
            println!("      {}", summary.raw);
        }
    }
    println!(
        "\nHara test summary: {} passed, {} failed, {} total",
        summaries.len() - failed,
        failed,
        summaries.len()
    );
    if failed != 0 {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::run_file;
    use std::path::Path;

    fn repository_root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("rust crate must have repository parent")
    }

    #[test]
    fn runs_passing_code_test_summary() {
        let root = repository_root();
        let summary = run_file(
            root,
            &root.join("lib/test-fixtures/std/native/test_runner_pass.hal"),
        )
        .unwrap();
        assert!(summary.passed, "{}", summary.failure_message());
        assert_eq!(summary.facts, 1);
        assert_eq!(summary.checks, 1);
        assert_eq!(summary.passed_checks, 1);
        assert_eq!(summary.failed_checks, 0);
    }

    #[test]
    fn preserves_failing_summary_for_cargo() {
        let root = repository_root();
        let summary = run_file(
            root,
            &root.join("lib/test-fixtures/std/native/test_runner_fail.hal"),
        )
        .unwrap();
        assert!(!summary.passed);
        assert_eq!(summary.facts, 1);
        assert_eq!(summary.checks, 1);
        assert_eq!(summary.passed_checks, 0);
        assert_eq!(summary.failed_checks, 1);
        assert!(summary.failure_message().contains(":failed"));
    }
}
