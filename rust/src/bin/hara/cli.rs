use crate::repl;
use hara_wasm::cli_app;
use hara_wasm::extension_tool;
#[cfg(feature = "hir-encoder")]
use hara_wasm::kernel::{hir::encode_hir_module, parse_forms};
use hara_wasm::kernel::{parse, read_forms, Form, SpannedForm};
use hara_wasm::native_cli::RuntimeBroker;
use hara_wasm::package;
use hara_wasm::project;
use hara_wasm::resp::{RespConnection, RespServer, RespValue};
use hara_wasm::Runtime;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::io::{self, BufRead, Read};
use std::net::TcpStream;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub(crate) struct Options {
    pub(crate) root: Option<PathBuf>,
    pub(crate) project: Option<PathBuf>,
    pub(crate) native_sockets: bool,
    pub(crate) allow_file: bool,
    pub(crate) allow_process: bool,
    pub(crate) log_requests: bool,
    pub(crate) offline: bool,
    pub(crate) host: String,
    pub(crate) port: u16,
    command: Vec<String>,
    pub(crate) history_file: Option<PathBuf>,
    pub(crate) no_history: bool,
    pub(crate) no_splash: bool,
    pub(crate) no_color: bool,
}

pub(crate) fn parse_options() -> Result<Options, String> {
    let mut options = Options {
        host: "127.0.0.1".into(),
        port: 1311,
        ..Options::default()
    };
    let mut args = env::args().skip(1);
    while let Some(argument) = args.next() {
        if argument == "--" {
            options.command.extend(args);
            break;
        }
        match argument.as_str() {
            "--help" | "-h" => {
                usage();
                std::process::exit(0);
            }
            "--version" | "-V" => {
                println!("hara native {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "--root" => options.root = Some(PathBuf::from(required(&mut args, "--root")?)),
            "--project" => options.project = Some(PathBuf::from(required(&mut args, "--project")?)),
            "--native-sockets" | "--allow-net" => options.native_sockets = true,
            "--allow-file" => options.allow_file = true,
            "--allow-process" => options.allow_process = true,
            "--log-requests" => options.log_requests = true,
            "--offline" => options.offline = true,
            "--no-history" => options.no_history = true,
            "--no-splash" => options.no_splash = true,
            "--no-color" => options.no_color = true,
            "--history" => {
                options.history_file = Some(PathBuf::from(required(&mut args, "--history")?))
            }
            "--host" => options.host = required(&mut args, "--host")?,
            "--port" => {
                options.port = required(&mut args, "--port")?
                    .parse()
                    .map_err(|_| "--port must be between 0 and 65535".to_owned())?
            }
            value if value.starts_with("--history=") => {
                options.history_file = Some(PathBuf::from(&value[10..]))
            }
            value if value.starts_with("--root=") => {
                options.root = Some(PathBuf::from(option_value(value, "--root")?))
            }
            value if value.starts_with("--project=") => {
                options.project = Some(PathBuf::from(option_value(value, "--project")?))
            }
            value if value.starts_with("--host=") => {
                options.host = option_value(value, "--host")?.to_owned()
            }
            value if value.starts_with("--port=") => {
                options.port = option_value(value, "--port")?
                    .parse()
                    .map_err(|_| "--port must be between 0 and 65535".to_owned())?
            }
            value if value.starts_with('-') => return Err(format!("unknown option: {value}")),
            value => {
                options.command.push(value.into());
                options.command.extend(args);
                break;
            }
        }
    }
    Ok(options)
}

fn required(args: &mut impl Iterator<Item = String>, option: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("{option} requires a value"))
}

fn option_value<'a>(argument: &'a str, option: &str) -> Result<&'a str, String> {
    let value = argument
        .strip_prefix(option)
        .and_then(|value| value.strip_prefix('='))
        .unwrap_or_default();
    if value.is_empty() {
        Err(format!("{option} requires a value"))
    } else {
        Ok(value)
    }
}

pub(crate) fn run(options: Options) -> Result<(), String> {
    let command = routed_command(&options.command);
    if command.first().is_some_and(|value| value == "help")
        || command
            .iter()
            .skip(1)
            .any(|value| value == "--help" || value == "-h")
    {
        usage();
        return Ok(());
    }
    match command.first().map(String::as_str) {
        Some("package") => package::run(&command[1..]),
        #[cfg(feature = "hir-encoder")]
        Some("compile-hir") => compile_hir(&command[1..]),
        Some("new") => new_project(&command[1..]),
        Some("check") => check_project(&options, &command[1..]),
        Some("add") => edit_dependency(&options, &command[1..], true),
        Some("remove") => edit_dependency(&options, &command[1..], false),
        Some("sync") => sync_project(&options, &command),
        Some("update") => Err("project update requires the reviewed registry client".into()),
        Some("test") => test_project(&options, &command[1..]),
        Some("spec") => spec_command(&command[1..]),
        Some("extension") => extension_tool::run(&command[1..], options.allow_process),
        Some("eval") => direct_eval(&options, &command[1..].join(" ")),
        Some("run") if command.len() == 1 => run_project(&options),
        Some("run") | Some("--file") => run_file(
            &options,
            command
                .get(1)
                .ok_or_else(|| "run requires a file path".to_owned())?,
        ),
        Some("stdin") => {
            let mut source = String::new();
            io::stdin()
                .read_to_string(&mut source)
                .map_err(|error| format!("stdin: {error}"))?;
            direct_eval(&options, &source)
        }
        Some("headless" | "server") => run_headless(&options),
        Some("remote") => run_remote(
            command
                .get(1)
                .ok_or_else(|| "remote requires HOST:PORT".to_owned())?,
        ),
        Some("standalone") => repl::run_repl(&options, true),
        Some("repl") | None => repl::run_repl(&options, options.offline),
        Some(command) => Err(format!("unknown command: {command}")),
    }
}

fn routed_command(command: &[String]) -> Vec<String> {
    if command.first().is_some_and(|value| {
        matches!(
            value.as_str(),
            "help" | "compile-hir"
        )
    }) || command == ["standalone"]
    {
        return command.to_vec();
    }
    let Some(resolved) = cli_app::router().resolve(command) else {
        return command.to_vec();
    };
    let legacy = match resolved.route.handler.as_str() {
        "hara.cli.handler/eval" => "eval",
        "hara.cli.handler/run-file" => "run",
        "hara.cli.handler/stdin" => "stdin",
        "hara.cli.handler/repl" => "repl",
        "hara.cli.handler/server" => "server",
        "hara.cli.handler/remote" => "remote",
        "hara.cli.handler/project-new" => "new",
        "hara.cli.handler/project-check" => "check",
        "hara.cli.handler/project-run" => "run",
        "hara.cli.handler/project-test" => "test",
        "hara.cli.handler/project-add" => "add",
        "hara.cli.handler/project-remove" => "remove",
        "hara.cli.handler/project-sync" => "sync",
        "hara.cli.handler/project-update" => "update",
        "hara.cli.handler/package" => "package",
        "hara.cli.handler/spec" => "spec",
        "hara.cli.handler/extension" => "extension",
        _ => return command.to_vec(),
    };
    let mut routed = vec![legacy.to_owned()];
    if matches!(
        resolved.route.handler.as_str(),
        "hara.cli.handler/package"
            | "hara.cli.handler/spec"
            | "hara.cli.handler/extension"
    ) {
        routed.extend(resolved.route.path.iter().skip(1).cloned());
    }
    routed.extend(resolved.arguments);
    routed
}

pub(crate) fn error_exit_code(error: &str) -> i32 {
    if error.starts_with("unknown ")
        || error.starts_with("usage:")
        || error.starts_with("unavailable:")
        || error.starts_with("--offline cannot")
        || error.contains(" requires ")
        || error.contains("cannot read")
        || error.contains("Cannot read")
        || error.contains("not found")
    {
        cli_app::CliOutcome::UsageError.exit_code()
    } else {
        cli_app::CliOutcome::Failed.exit_code()
    }
}

#[derive(Clone, Debug, PartialEq)]
struct SpecFinding {
    rule: &'static str,
    requirement: &'static str,
    path: Vec<Form>,
    message: String,
    repair: Form,
}

const METASPEC_REQUIRED_KEYS: &[&str] = &[
    "document/id",
    "document/type",
    "document/version",
    "document/status",
    "document/title",
    "document/summary",
    "spec/conforms-to",
    "spec/artifact-kind",
    "meta/document-schema",
    "meta/schemas",
    "meta/cross-references",
    "meta/requirements",
    "metaspec/generation",
];

const METASPEC_IDENTIFIER_KEYS: &[&str] = &[
    "document/id",
    "schema/id",
    "reference/id",
    "requirement/id",
    "section/id",
    "rule/id",
    "linter/id",
    "form/id",
    "entity/id",
    "relation/id",
    "codec/id",
    "checker/id",
    "law/id",
    "conformance/id",
];

fn spec_command(args: &[String]) -> Result<(), String> {
    let operation = args
        .first()
        .ok_or_else(|| {
            "spec requires lint, verify, validate, template, check-contribution, check, to-edn, from-edn, normalize, graph, or obligations"
                .to_owned()
        })?;
    if operation == "template" {
        if args.len() != 1 {
            exit_error("spec template accepts no file", 2);
        }
        println!("{}", metaspec_template());
        return Ok(());
    }
    if operation == "validate" {
        return spec_validate_command(&args[1..]);
    }
    if operation == "check-contribution" {
        return check_contribution_command(&args[1..]);
    }
    if matches!(
        operation.as_str(),
        "check" | "to-edn" | "from-edn" | "normalize" | "graph" | "obligations"
    ) {
        return build_spec_command(operation, &args[1..]);
    }
    if !matches!(operation.as_str(), "lint" | "verify") {
        exit_error(
            &format!(
                "spec {operation} is not implemented yet; use lint, verify, validate, template, or check-contribution"
            ),
            2,
        );
    }
    let path = args
        .get(1)
        .ok_or_else(|| format!("spec {operation} requires FILE"))?;
    let format = spec_format(&args[2..]).unwrap_or_else(|error| exit_error(&error, 2));
    let source = fs::read_to_string(path)
        .unwrap_or_else(|error| exit_error(&format!("cannot read {path}: {error}"), 2));
    let document = read_spec_document(&source)
        .unwrap_or_else(|error| exit_error(&format!("{path}: {error}"), 2));
    let mut findings = lint_metaspec(&document);
    if operation == "verify" {
        findings.extend(verify_metaspec(&document, Path::new(path)));
    }
    let report = metaspec_report(&document, &findings);
    match format {
        SpecFormat::Edn => println!("{report}"),
        SpecFormat::Text => print_metaspec_text(&document, &findings),
    }
    if findings.is_empty() {
        Ok(())
    } else {
        std::process::exit(1)
    }
}

fn check_contribution_command(args: &[String]) -> Result<(), String> {
    let contribution_root = args
        .first()
        .unwrap_or_else(|| exit_error("spec check-contribution requires DIRECTORY", 2));
    let format = spec_format(&args[1..]).unwrap_or_else(|error| exit_error(&error, 2));
    let contribution_root = Path::new(contribution_root);
    if !contribution_root.is_dir() {
        exit_error(
            &format!(
                "contribution path is not a directory: {}",
                contribution_root.display()
            ),
            2,
        );
    }
    let envelope_path = contribution_root.join("CONTRIBUTION.edn");
    let source = fs::read_to_string(&envelope_path).unwrap_or_else(|error| {
        exit_error(
            &format!("cannot read {}: {error}", envelope_path.display()),
            2,
        )
    });
    let envelope = read_spec_document(&source)
        .unwrap_or_else(|error| exit_error(&format!("{}: {error}", envelope_path.display()), 2));
    let repository_root = find_repository_root(contribution_root).unwrap_or_else(|| {
        exit_error(
            "cannot locate repository root containing specs/ and contrib/",
            2,
        )
    });
    let findings = check_contribution(&envelope, contribution_root, &repository_root);
    let report = contribution_report(&envelope, &findings);
    match format {
        SpecFormat::Edn => println!("{report}"),
        SpecFormat::Text => print_contribution_text(&envelope, &findings),
    }
    if findings.is_empty() {
        Ok(())
    } else {
        std::process::exit(1)
    }
}

fn find_repository_root(path: &Path) -> Option<PathBuf> {
    let absolute = path.canonicalize().ok()?;
    absolute.ancestors().find_map(|candidate| {
        (candidate.join("specs").is_dir() && candidate.join("contrib").is_dir())
            .then(|| candidate.to_path_buf())
    })
}

fn check_contribution(
    envelope: &Form,
    contribution_root: &Path,
    repository_root: &Path,
) -> Vec<SpecFinding> {
    let mut findings = Vec::new();
    for key in [
        "contribution/id",
        "contribution/owner",
        "contribution/version",
        "contribution/status",
        "contribution/title",
        "contribution/summary",
        "contribution/source",
        "contribution/specs",
    ] {
        if map_get(envelope, key).is_none() {
            findings.push(finding(
                "hara.contribution.rule/required-key",
                "hara.contribution/required-fields",
                vec![],
                format!("Missing required contribution key :{key}"),
                map_form(vec![
                    ("action/type", keyword("add-key")),
                    ("action/key", keyword(key)),
                ]),
            ));
        }
    }
    let contribution_id = map_get(envelope, "contribution/id").and_then(keyword_name);
    let owner = map_get(envelope, "contribution/owner").and_then(keyword_name);
    if let (Some(id), Some(owner)) = (&contribution_id, &owner) {
        if !id.starts_with(&format!("{owner}/")) {
            findings.push(finding(
                "hara.contribution.rule/owner-qualified-id",
                "hara.contribution/owner-qualified-identifiers",
                vec![keyword("contribution/id")],
                format!("Contribution ID :{id} is not owned by :{owner}"),
                map_form(vec![("action/type", keyword("use-owner-qualified-id"))]),
            ));
        }
    }
    let status = map_get(envelope, "contribution/status").and_then(keyword_name);
    if !matches!(
        status.as_deref(),
        Some("draft" | "candidate" | "stable" | "deprecated" | "scaffold")
    ) {
        findings.push(finding(
            "hara.contribution.rule/status",
            "hara.contribution/known-status",
            vec![keyword("contribution/status")],
            "Contribution status must be :draft, :candidate, :stable, :deprecated, or :scaffold",
            map_form(vec![("action/type", keyword("select-status"))]),
        ));
    }
    check_contribution_source(envelope, &mut findings);
    let specs = match map_get(envelope, "contribution/specs") {
        Some(Form::Vector(specs)) => specs,
        Some(_) => {
            findings.push(finding(
                "hara.contribution.rule/spec-list",
                "hara.contribution/spec-list",
                vec![keyword("contribution/specs")],
                ":contribution/specs must be a vector",
                map_form(vec![("action/type", keyword("replace-with-vector"))]),
            ));
            return findings;
        }
        None => return findings,
    };
    if status.as_deref() != Some("scaffold") && specs.is_empty() {
        findings.push(finding(
            "hara.contribution.rule/spec-required",
            "hara.contribution/normative-spec",
            vec![keyword("contribution/specs")],
            "A non-scaffold contribution must contain at least one specification",
            map_form(vec![("action/type", keyword("add-specification"))]),
        ));
    }
    for (index, spec) in specs.iter().enumerate() {
        check_contribution_spec(
            spec,
            index,
            contribution_root,
            repository_root,
            &mut findings,
        );
    }
    findings
}

fn check_contribution_source(envelope: &Form, findings: &mut Vec<SpecFinding>) {
    let Some(source) = map_get(envelope, "contribution/source") else {
        return;
    };
    let source_path = vec![keyword("contribution/source")];
    if map_get(source, "source/provider") != Some(&keyword("github")) {
        findings.push(finding(
            "hara.contribution.rule/source-provider",
            "hara.contribution/github-source",
            source_path.clone(),
            "Contribution source provider must be :github",
            map_form(vec![
                ("action/type", keyword("set-value")),
                ("action/value", keyword("github")),
            ]),
        ));
    }
    let repository = map_get(source, "source/repository").and_then(string_value);
    if !repository.as_deref().is_some_and(valid_github_repository) {
        findings.push(finding(
            "hara.contribution.rule/source-repository",
            "hara.contribution/github-source",
            source_path.clone(),
            "Contribution source repository must be owner/name",
            map_form(vec![("action/type", keyword("set-repository"))]),
        ));
    }
    let commit = map_get(source, "source/commit").and_then(string_value);
    if !commit.as_deref().is_some_and(valid_full_git_sha) {
        findings.push(finding(
            "hara.contribution.rule/source-commit",
            "hara.contribution/immutable-source",
            source_path.clone(),
            "Contribution source commit must be a full 40-character Git SHA",
            map_form(vec![("action/type", keyword("resolve-commit-sha"))]),
        ));
    }
    let path = map_get(source, "source/path").and_then(string_value);
    if !path.as_deref().is_some_and(safe_relative_path) {
        findings.push(finding(
            "hara.contribution.rule/source-path",
            "hara.contribution/repository-relative-paths",
            source_path,
            "Contribution source path must be repository-relative",
            map_form(vec![("action/type", keyword("set-relative-path"))]),
        ));
    }
}

fn check_contribution_spec(
    spec: &Form,
    index: usize,
    contribution_root: &Path,
    repository_root: &Path,
    findings: &mut Vec<SpecFinding>,
) {
    let path_prefix = vec![keyword("contribution/specs"), Form::Number(index as i64)];
    for key in [
        "spec/id",
        "spec/version",
        "spec/path",
        "spec/metaspec",
        "spec/sha256",
    ] {
        if map_get(spec, key).is_none() {
            findings.push(finding(
                "hara.contribution.rule/spec-field",
                "hara.contribution/spec-reference",
                path_prefix.clone(),
                format!("Specification reference is missing :{key}"),
                map_form(vec![
                    ("action/type", keyword("add-key")),
                    ("action/key", keyword(key)),
                ]),
            ));
        }
    }
    let Some(relative_path) = map_get(spec, "spec/path").and_then(string_value) else {
        return;
    };
    if !safe_relative_path(&relative_path) {
        findings.push(finding(
            "hara.contribution.rule/spec-path",
            "hara.contribution/repository-relative-paths",
            path_prefix,
            "Specification path must remain inside its contribution",
            map_form(vec![("action/type", keyword("set-relative-path"))]),
        ));
        return;
    }
    let document_path = contribution_root.join(&relative_path);
    let bytes = match fs::read(&document_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            findings.push(finding(
                "hara.contribution.rule/spec-readable",
                "hara.contribution/spec-readable",
                path_prefix,
                format!("Cannot read {}: {error}", document_path.display()),
                map_form(vec![("action/type", keyword("create-or-repair-file"))]),
            ));
            return;
        }
    };
    let expected_digest = map_get(spec, "spec/sha256").and_then(string_value);
    let actual_digest = format!("sha256:{:x}", Sha256::digest(&bytes));
    if expected_digest.as_deref() != Some(actual_digest.as_str()) {
        findings.push(finding(
            "hara.contribution.rule/spec-digest",
            "hara.contribution/content-addressed-spec",
            path_prefix.clone(),
            format!("Specification digest mismatch; actual digest is {actual_digest}"),
            map_form(vec![
                ("action/type", keyword("set-value")),
                ("action/key", keyword("spec/sha256")),
                ("action/value", string(actual_digest)),
            ]),
        ));
    }
    let document_source = match String::from_utf8(bytes) {
        Ok(source) => source,
        Err(_) => {
            findings.push(finding(
                "hara.contribution.rule/spec-edn",
                "hara.contribution/spec-readable",
                path_prefix,
                "Specification is not UTF-8 EDN",
                map_form(vec![("action/type", keyword("rewrite-as-edn"))]),
            ));
            return;
        }
    };
    let document = match read_spec_document(&document_source) {
        Ok(document) => document,
        Err(error) => {
            findings.push(finding(
                "hara.contribution.rule/spec-edn",
                "hara.contribution/spec-readable",
                path_prefix,
                format!("Specification cannot be read: {error}"),
                map_form(vec![("action/type", keyword("repair-edn"))]),
            ));
            return;
        }
    };
    if map_get(spec, "spec/id") != map_get(&document, "document/id") {
        findings.push(finding(
            "hara.contribution.rule/spec-id",
            "hara.contribution/spec-reference",
            path_prefix.clone(),
            "Envelope :spec/id does not match specification :document/id",
            map_form(vec![("action/type", keyword("align-document-id"))]),
        ));
    }
    if map_get(spec, "spec/version") != map_get(&document, "document/version") {
        findings.push(finding(
            "hara.contribution.rule/spec-version",
            "hara.contribution/spec-reference",
            path_prefix.clone(),
            "Envelope :spec/version does not match specification :document/version",
            map_form(vec![("action/type", keyword("align-document-version"))]),
        ));
    }
    let Some(metaspec_path) = map_get(spec, "spec/metaspec").and_then(string_value) else {
        return;
    };
    if !safe_relative_path(&metaspec_path) {
        findings.push(finding(
            "hara.contribution.rule/metaspec-path",
            "hara.contribution/repository-relative-paths",
            path_prefix,
            "Meta-specification path must be repository-relative",
            map_form(vec![("action/type", keyword("set-relative-path"))]),
        ));
        return;
    }
    let metaspec_path = repository_root.join(metaspec_path);
    let metaspec_source = match fs::read_to_string(&metaspec_path) {
        Ok(source) => source,
        Err(error) => {
            findings.push(finding(
                "hara.contribution.rule/metaspec-readable",
                "hara.contribution/metaspec-conformance",
                path_prefix,
                format!("Cannot read {}: {error}", metaspec_path.display()),
                map_form(vec![("action/type", keyword("repair-metaspec-reference"))]),
            ));
            return;
        }
    };
    let metaspec = match read_spec_document(&metaspec_source) {
        Ok(metaspec) => metaspec,
        Err(error) => {
            findings.push(finding(
                "hara.contribution.rule/metaspec-readable",
                "hara.contribution/metaspec-conformance",
                path_prefix,
                format!("Meta-specification cannot be read: {error}"),
                map_form(vec![("action/type", keyword("repair-metaspec"))]),
            ));
            return;
        }
    };
    for meta_finding in validate_against_metaspec(&document, &metaspec, &document_path) {
        findings.push(meta_finding);
    }
}

fn safe_relative_path(value: &str) -> bool {
    let path = Path::new(value);
    !value.is_empty()
        && !path.is_absolute()
        && path.components().all(|component| {
            matches!(
                component,
                std::path::Component::Normal(_) | std::path::Component::CurDir
            )
        })
}

fn contribution_report(envelope: &Form, findings: &[SpecFinding]) -> Form {
    let status = if findings.is_empty() { "pass" } else { "fail" };
    let finding_forms = findings
        .iter()
        .map(|finding| {
            map_form(vec![
                ("finding/id", keyword(finding.rule)),
                ("requirement/id", keyword(finding.requirement)),
                ("finding/level", keyword("error")),
                ("finding/path", Form::Vector(finding.path.clone())),
                ("finding/message", string(&finding.message)),
                ("finding/repair", finding.repair.clone()),
            ])
        })
        .collect();
    map_form(vec![
        ("report/type", keyword("hara/contribution-check")),
        ("report/version", string("0.1.0")),
        (
            "contribution/id",
            map_get(envelope, "contribution/id")
                .cloned()
                .unwrap_or(Form::Nil),
        ),
        ("report/status", keyword(status)),
        (
            "summary",
            map_form(vec![
                (
                    "summary/pass",
                    Form::Number(if findings.is_empty() { 1 } else { 0 }),
                ),
                ("summary/fail", Form::Number(findings.len() as i64)),
                ("summary/unknown", Form::Number(0)),
                ("summary/blocked", Form::Number(0)),
            ]),
        ),
        ("findings", Form::Vector(finding_forms)),
        (
            "next-actions",
            Form::Vector(
                findings
                    .iter()
                    .map(|finding| finding.repair.clone())
                    .collect(),
            ),
        ),
    ])
}

fn print_contribution_text(envelope: &Form, findings: &[SpecFinding]) {
    let id = map_get(envelope, "contribution/id")
        .map(ToString::to_string)
        .unwrap_or_else(|| "<unknown>".into());
    if findings.is_empty() {
        println!("PASS {id}");
    } else {
        println!("FAIL {id} ({} findings)", findings.len());
        for finding in findings {
            println!("  {} — {}", finding.rule, finding.message);
        }
    }
}

fn spec_validate_command(args: &[String]) -> Result<(), String> {
    let path = args
        .first()
        .unwrap_or_else(|| exit_error("spec validate requires FILE --against METASPEC", 2));
    if args.get(1).map(String::as_str) != Some("--against") {
        exit_error("spec validate requires FILE --against METASPEC", 2);
    }
    let metaspec_path = args
        .get(2)
        .unwrap_or_else(|| exit_error("spec validate requires FILE --against METASPEC", 2));
    let format = spec_format(&args[3..]).unwrap_or_else(|error| exit_error(&error, 2));
    let source = fs::read_to_string(path)
        .unwrap_or_else(|error| exit_error(&format!("cannot read {path}: {error}"), 2));
    let metaspec_source = fs::read_to_string(metaspec_path)
        .unwrap_or_else(|error| exit_error(&format!("cannot read {metaspec_path}: {error}"), 2));
    let document = read_spec_document(&source)
        .unwrap_or_else(|error| exit_error(&format!("{path}: {error}"), 2));
    let metaspec = read_spec_document(&metaspec_source)
        .unwrap_or_else(|error| exit_error(&format!("{metaspec_path}: {error}"), 2));
    let meta_findings = lint_metaspec(&metaspec);
    if !meta_findings.is_empty() {
        exit_error("the --against meta-spec does not pass structural lint", 2);
    }
    let findings = validate_against_metaspec(&document, &metaspec, Path::new(path));
    let report = metaspec_report(&document, &findings);
    match format {
        SpecFormat::Edn => println!("{report}"),
        SpecFormat::Text => print_metaspec_text(&document, &findings),
    }
    if findings.is_empty() {
        Ok(())
    } else {
        std::process::exit(1)
    }
}

#[derive(Clone, Debug)]
struct CanonicalBuild {
    file: String,
    id: String,
    artifact_kind: String,
    artifact_output: String,
    specs: Vec<(String, String)>,
    stages: Vec<BuildStage>,
}

#[derive(Clone, Debug)]
struct BuildStage {
    id: String,
    requires: Vec<String>,
    produces: String,
    checkers: Vec<Form>,
    row: usize,
    col: usize,
}

#[derive(Clone, Debug)]
struct BuildFinding {
    kind: &'static str,
    level: &'static str,
    message: String,
    stage: Option<String>,
    row: Option<usize>,
    col: Option<usize>,
    details: Vec<(&'static str, Form)>,
}

fn build_spec_command(operation: &str, args: &[String]) -> Result<(), String> {
    let path = args
        .first()
        .unwrap_or_else(|| exit_error(&format!("spec {operation} requires FILE"), 2));
    let format = spec_format(&args[1..]).unwrap_or_else(|error| exit_error(&error, 2));
    let source = fs::read_to_string(path)
        .unwrap_or_else(|error| exit_error(&format!("cannot read {path}: {error}"), 2));
    let (build, mut findings) = if operation == "from-edn" {
        let data = read_spec_document(&source)
            .unwrap_or_else(|error| exit_error(&format!("{path}: {error}"), 2));
        canonical_build_from_edn(&data)
            .unwrap_or_else(|error| exit_error(&format!("{path}: {error}"), 2))
    } else {
        read_build_source(&source, path)
            .unwrap_or_else(|error| exit_error(&format!("{path}: {error}"), 2))
    };

    match operation {
        "to-edn" | "normalize" => println!("{}", canonical_build_form(&build)),
        "from-edn" => println!("{}", write_build_surface(&build)),
        "graph" => {
            findings.extend(check_build_graph(&build));
            let graph = build_graph_form(&build, &findings);
            match format {
                SpecFormat::Edn => println!("{graph}"),
                SpecFormat::Text => print_build_graph_text(&build, &findings),
            }
            if has_required_failures(&findings) {
                std::process::exit(1);
            }
        }
        "check" | "obligations" => {
            findings.extend(check_build(&build));
            let report = build_obligation_report(&build, &findings);
            match format {
                SpecFormat::Edn => println!("{report}"),
                SpecFormat::Text => print_build_check_text(&build, &findings),
            }
            if build_report_status(&report) != "pass" {
                std::process::exit(1);
            }
        }
        _ => unreachable!(),
    }
    Ok(())
}

fn read_build_source(
    source: &str,
    file: &str,
) -> Result<(CanonicalBuild, Vec<BuildFinding>), String> {
    let roots = read_forms(source).map_err(|error| error.to_string())?;
    if roots.len() != 1 {
        return Err("Build source must contain exactly one root form".into());
    }
    parse_build_root(&roots[0], file)
}

fn parse_build_root(
    root: &SpannedForm,
    file: &str,
) -> Result<(CanonicalBuild, Vec<BuildFinding>), String> {
    let Form::List(values) = &root.form else {
        return Err("Build source root must be a build list".into());
    };
    if !matches!(values.first(), Some(Form::Symbol(head)) if head == "build") {
        return Err("Build source root must begin with build".into());
    }
    let id = match values.get(1) {
        Some(Form::Symbol(id)) | Some(Form::Keyword(id)) => id.clone(),
        _ => return Err("build requires a symbol or keyword identifier".into()),
    };
    let mut findings = Vec::new();
    let mut artifacts = Vec::new();
    let mut specs = Vec::new();
    let mut stages = Vec::new();
    for (index, body) in values.iter().enumerate().skip(2) {
        let span = root.children.get(index).unwrap_or(root);
        let Form::List(form) = body else {
            findings.push(build_finding(
                "greenways/unknown-form",
                "error",
                "Unknown non-list build form".into(),
                None,
                span,
                vec![],
            ));
            continue;
        };
        match form.first() {
            Some(Form::Symbol(head)) if head == "artifact" => {
                let properties = surface_properties(&form[1..], span, &mut findings);
                let kind = properties.get("kind").and_then(|value| keyword_name(value));
                let output = properties
                    .get("output")
                    .and_then(|value| string_value(value));
                match (kind, output) {
                    (Some(kind), Some(output)) => artifacts.push((kind, output)),
                    _ => findings.push(build_finding(
                        "greenways/missing-artifact-property",
                        "error",
                        "Artifact requires :kind and :output".into(),
                        None,
                        span,
                        vec![],
                    )),
                }
            }
            Some(Form::Symbol(head)) if head == "use-spec" => {
                let spec_id = form.get(1).and_then(keyword_name);
                let version = form
                    .get(2)
                    .and_then(|options| map_get(options, "version"))
                    .and_then(string_value);
                match (spec_id, version) {
                    (Some(id), Some(version)) => specs.push((id, version)),
                    _ => findings.push(build_finding(
                        "greenways/invalid-spec-reference",
                        "error",
                        "use-spec requires a keyword ID and string :version".into(),
                        None,
                        span,
                        vec![],
                    )),
                }
            }
            Some(Form::Symbol(head)) if head == "stage" => {
                let stage_id = match form.get(1) {
                    Some(Form::Symbol(id)) | Some(Form::Keyword(id)) => id.clone(),
                    _ => {
                        findings.push(build_finding(
                            "greenways/invalid-stage-id",
                            "error",
                            "stage requires a symbol or keyword identifier".into(),
                            None,
                            span,
                            vec![],
                        ));
                        format!("invalid-stage-{index}")
                    }
                };
                let properties = surface_properties(&form[2..], span, &mut findings);
                let requires = match properties.get("requires") {
                    Some(Form::Vector(values)) => values
                        .iter()
                        .filter_map(|value| match value {
                            Form::Symbol(id) | Form::Keyword(id) => Some(id.clone()),
                            _ => None,
                        })
                        .collect(),
                    None => Vec::new(),
                    _ => {
                        findings.push(build_finding(
                            "greenways/invalid-dependencies",
                            "error",
                            ":requires must be a vector of stage IDs".into(),
                            Some(stage_id.clone()),
                            span,
                            vec![],
                        ));
                        Vec::new()
                    }
                };
                let produces = properties
                    .get("produces")
                    .and_then(|value| keyword_name(value))
                    .unwrap_or_else(|| {
                        findings.push(build_finding(
                            "greenways/missing-stage-producer",
                            "error",
                            "stage requires a keyword :produces value".into(),
                            Some(stage_id.clone()),
                            span,
                            vec![],
                        ));
                        "greenways/invalid".into()
                    });
                let checkers = match properties.get("checkers") {
                    Some(Form::Vector(values)) => values
                        .iter()
                        .filter_map(|checker| {
                            parse_surface_checker(checker, span, &stage_id, &mut findings)
                        })
                        .collect(),
                    None => Vec::new(),
                    _ => {
                        findings.push(build_finding(
                            "greenways/invalid-checkers",
                            "error",
                            ":checkers must be a vector".into(),
                            Some(stage_id.clone()),
                            span,
                            vec![],
                        ));
                        Vec::new()
                    }
                };
                stages.push(BuildStage {
                    id: stage_id,
                    requires,
                    produces,
                    checkers,
                    row: span.span.start.line,
                    col: span.span.start.column,
                });
            }
            Some(Form::Symbol(head)) => findings.push(build_finding(
                "greenways/unknown-form",
                "error",
                format!("Unknown build form: {head}"),
                None,
                span,
                vec![("form/head", Form::Symbol(head.clone()))],
            )),
            _ => findings.push(build_finding(
                "greenways/unknown-form",
                "error",
                "Unknown build form".into(),
                None,
                span,
                vec![],
            )),
        }
    }
    if artifacts.len() != 1 {
        findings.push(build_finding(
            "greenways/artifact-count",
            "error",
            "Build must contain exactly one artifact form".into(),
            None,
            root,
            vec![("artifact/count", Form::Number(artifacts.len() as i64))],
        ));
    }
    if stages.is_empty() {
        findings.push(build_finding(
            "greenways/no-stages",
            "error",
            "Build must contain one or more stages".into(),
            None,
            root,
            vec![],
        ));
    }
    let (artifact_kind, artifact_output) = artifacts
        .into_iter()
        .next()
        .unwrap_or_else(|| ("greenways/invalid".into(), "".into()));
    Ok((
        CanonicalBuild {
            file: file.into(),
            id,
            artifact_kind,
            artifact_output,
            specs,
            stages,
        },
        findings,
    ))
}

fn surface_properties<'a>(
    values: &'a [Form],
    span: &SpannedForm,
    findings: &mut Vec<BuildFinding>,
) -> HashMap<String, &'a Form> {
    let mut properties = HashMap::new();
    let mut index = 0;
    while index < values.len() {
        let key = match &values[index] {
            Form::Keyword(key) => key.clone(),
            value => {
                findings.push(build_finding(
                    "greenways/invalid-property",
                    "error",
                    format!("Expected property keyword, got: {value}"),
                    None,
                    span,
                    vec![],
                ));
                index += 1;
                continue;
            }
        };
        let Some(value) = values.get(index + 1) else {
            findings.push(build_finding(
                "greenways/missing-property-value",
                "error",
                format!("Missing value for property: :{key}"),
                None,
                span,
                vec![("property/key", keyword(&key))],
            ));
            break;
        };
        if properties.insert(key.clone(), value).is_some() {
            findings.push(build_finding(
                "greenways/duplicate-key",
                "error",
                format!("Duplicate property: :{key}"),
                None,
                span,
                vec![("property/key", keyword(&key))],
            ));
        }
        index += 2;
    }
    properties
}

fn parse_surface_checker(
    value: &Form,
    span: &SpannedForm,
    stage_id: &str,
    findings: &mut Vec<BuildFinding>,
) -> Option<Form> {
    let Form::List(values) = value else {
        findings.push(build_finding(
            "greenways/invalid-check",
            "error",
            format!("Invalid check form: {value}"),
            Some(stage_id.into()),
            span,
            vec![],
        ));
        return None;
    };
    if !matches!(values.first(), Some(Form::Symbol(head)) if head == "check") {
        findings.push(build_finding(
            "greenways/invalid-check",
            "error",
            format!("Invalid check form: {value}"),
            Some(stage_id.into()),
            span,
            vec![],
        ));
        return None;
    }
    let Some(Form::Keyword(id)) = values.get(1) else {
        findings.push(build_finding(
            "greenways/invalid-check",
            "error",
            "check requires a keyword ID".into(),
            Some(stage_id.into()),
            span,
            vec![],
        ));
        return None;
    };
    let mut entries = vec![(keyword("checker/id"), keyword(id))];
    if let Some(options @ Form::Map(_)) = values.get(2) {
        if let Some(variation) = map_get(options, "variation/id") {
            entries.push((keyword("checker/variation"), variation.clone()));
        }
        if let Some(source) = map_get(options, "checker/source") {
            entries.push((keyword("checker/source"), source.clone()));
        }
        if let Some(entrypoint) = map_get(options, "checker/entrypoint") {
            entries.push((keyword("checker/entrypoint"), entrypoint.clone()));
        }
    }
    Some(Form::Map(entries))
}

fn keyword_name(value: &Form) -> Option<String> {
    match value {
        Form::Keyword(value) => Some(value.clone()),
        _ => None,
    }
}

fn string_value(value: &Form) -> Option<String> {
    match value {
        Form::String(value) => Some(value.clone()),
        _ => None,
    }
}

fn build_finding(
    kind: &'static str,
    level: &'static str,
    message: String,
    stage: Option<String>,
    span: &SpannedForm,
    details: Vec<(&'static str, Form)>,
) -> BuildFinding {
    BuildFinding {
        kind,
        level,
        message,
        stage,
        row: Some(span.span.start.line),
        col: Some(span.span.start.column),
        details,
    }
}

fn canonical_build_form(build: &CanonicalBuild) -> Form {
    map_form(vec![
        ("greenways/type", keyword("build")),
        ("greenways/version", string("0.1.0")),
        ("build/id", keyword(&build.id)),
        (
            "build/artifact",
            map_form(vec![
                ("artifact/kind", keyword(&build.artifact_kind)),
                ("artifact/output", string(&build.artifact_output)),
            ]),
        ),
        (
            "build/specs",
            Form::Vector(
                build
                    .specs
                    .iter()
                    .map(|(id, version)| {
                        map_form(vec![
                            ("spec/id", keyword(id)),
                            ("spec/version", string(version)),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "build/stages",
            Form::Vector(build.stages.iter().map(canonical_stage_form).collect()),
        ),
    ])
}

fn canonical_stage_form(stage: &BuildStage) -> Form {
    map_form(vec![
        ("stage/id", keyword(&stage.id)),
        (
            "stage/requires",
            Form::Vector(
                stage
                    .requires
                    .iter()
                    .map(|dependency| keyword(dependency))
                    .collect(),
            ),
        ),
        ("stage/produces", keyword(&stage.produces)),
        ("stage/checkers", Form::Vector(stage.checkers.clone())),
    ])
}

fn canonical_build_from_edn(data: &Form) -> Result<(CanonicalBuild, Vec<BuildFinding>), String> {
    let id = map_get(data, "build/id")
        .and_then(keyword_name)
        .ok_or("canonical build requires keyword :build/id")?;
    let artifact =
        map_get(data, "build/artifact").ok_or("canonical build requires :build/artifact")?;
    let artifact_kind = map_get(artifact, "artifact/kind")
        .and_then(keyword_name)
        .ok_or("canonical artifact requires keyword :artifact/kind")?;
    let artifact_output = map_get(artifact, "artifact/output")
        .and_then(string_value)
        .ok_or("canonical artifact requires string :artifact/output")?;
    let specs = match map_get(data, "build/specs") {
        Some(Form::Vector(values)) => values
            .iter()
            .map(|spec| {
                let id = map_get(spec, "spec/id")
                    .and_then(keyword_name)
                    .ok_or("spec reference requires keyword :spec/id")?;
                let version = map_get(spec, "spec/version")
                    .and_then(string_value)
                    .ok_or("spec reference requires string :spec/version")?;
                Ok((id, version))
            })
            .collect::<Result<Vec<_>, String>>()?,
        None => Vec::new(),
        _ => return Err(":build/specs must be a vector".into()),
    };
    let stages = match map_get(data, "build/stages") {
        Some(Form::Vector(values)) => values
            .iter()
            .map(|stage| {
                let id = map_get(stage, "stage/id")
                    .and_then(keyword_name)
                    .ok_or("stage requires keyword :stage/id")?;
                let requires = match map_get(stage, "stage/requires") {
                    Some(Form::Vector(values)) => values
                        .iter()
                        .map(|value| {
                            keyword_name(value)
                                .ok_or_else(|| ":stage/requires values must be keywords".to_owned())
                        })
                        .collect::<Result<Vec<_>, String>>()?,
                    None => Vec::new(),
                    _ => return Err(":stage/requires must be a vector".into()),
                };
                let produces = map_get(stage, "stage/produces")
                    .and_then(keyword_name)
                    .ok_or("stage requires keyword :stage/produces")?;
                let checkers = match map_get(stage, "stage/checkers") {
                    Some(Form::Vector(values)) => values.clone(),
                    None => Vec::new(),
                    _ => return Err(":stage/checkers must be a vector".into()),
                };
                Ok(BuildStage {
                    id,
                    requires,
                    produces,
                    checkers,
                    row: 0,
                    col: 0,
                })
            })
            .collect::<Result<Vec<_>, String>>()?,
        Some(_) => return Err(":build/stages must be a vector".into()),
        None => Vec::new(),
    };
    Ok((
        CanonicalBuild {
            file: "<edn>".into(),
            id,
            artifact_kind,
            artifact_output,
            specs,
            stages,
        },
        Vec::new(),
    ))
}

fn write_build_surface(build: &CanonicalBuild) -> String {
    let mut output = format!(
        "(build {}\n  (artifact\n    :kind :{}\n    :output {})",
        build.id,
        build.artifact_kind,
        Form::String(build.artifact_output.clone())
    );
    for (id, version) in &build.specs {
        output.push_str(&format!(
            "\n\n  (use-spec\n    :{id}\n    {{:version {}}})",
            Form::String(version.clone())
        ));
    }
    for stage in &build.stages {
        output.push_str(&format!("\n\n  (stage {}", stage.id));
        if !stage.requires.is_empty() {
            output.push_str(&format!(
                "\n    :requires [{}]",
                stage
                    .requires
                    .iter()
                    .map(|id| id.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            ));
        }
        output.push_str(&format!("\n    :produces :{}", stage.produces));
        if !stage.checkers.is_empty() {
            output.push_str("\n    :checkers\n    [");
            output.push_str(
                &stage
                    .checkers
                    .iter()
                    .map(write_checker_surface)
                    .collect::<Vec<_>>()
                    .join("\n     "),
            );
            output.push(']');
        }
        output.push(')');
    }
    output.push_str(")\n");
    output
}

fn write_checker_surface(checker: &Form) -> String {
    let id = map_get(checker, "checker/id")
        .map(ToString::to_string)
        .unwrap_or_else(|| ":greenways/invalid".into());
    let mut options = Vec::new();
    if let Some(value) = map_get(checker, "checker/variation") {
        options.push((keyword("variation/id"), value.clone()));
    }
    if let Some(value) = map_get(checker, "checker/source") {
        options.push((keyword("checker/source"), value.clone()));
    }
    if let Some(value) = map_get(checker, "checker/entrypoint") {
        options.push((keyword("checker/entrypoint"), value.clone()));
    }
    if options.is_empty() {
        format!("(check {id})")
    } else {
        format!("(check {id} {})", Form::Map(options))
    }
}

fn check_build(build: &CanonicalBuild) -> Vec<BuildFinding> {
    let mut findings = check_build_graph(build);
    findings.extend(check_checker_identities(build));
    findings
}

fn check_build_graph(build: &CanonicalBuild) -> Vec<BuildFinding> {
    let mut findings = Vec::new();
    let mut counts = HashMap::new();
    for stage in &build.stages {
        *counts.entry(stage.id.clone()).or_insert(0usize) += 1;
    }
    for stage in &build.stages {
        if counts.get(&stage.id).copied().unwrap_or_default() > 1
            && !findings.iter().any(|finding: &BuildFinding| {
                finding.kind == "greenways/duplicate-stage"
                    && finding.stage.as_deref() == Some(&stage.id)
            })
        {
            findings.push(stage_finding(
                stage,
                "greenways/duplicate-stage",
                "error",
                format!("Duplicate stage ID: {}", stage.id),
                vec![],
            ));
        }
    }
    let ids = build
        .stages
        .iter()
        .map(|stage| stage.id.as_str())
        .collect::<HashSet<_>>();
    for stage in &build.stages {
        for dependency in &stage.requires {
            if dependency == &stage.id {
                findings.push(stage_finding(
                    stage,
                    "greenways/self-dependency",
                    "error",
                    format!("Stage depends on itself: {}", stage.id),
                    vec![],
                ));
            } else if !ids.contains(dependency.as_str()) {
                findings.push(stage_finding(
                    stage,
                    "greenways/missing-dependency",
                    "error",
                    format!("Unknown required stage: {dependency}"),
                    vec![("stage/dependency", keyword(dependency))],
                ));
            }
        }
    }
    for cycle in build_cycles(build) {
        let stage = build
            .stages
            .iter()
            .find(|stage| cycle.first() == Some(&stage.id));
        let mut finding = BuildFinding {
            kind: "greenways/dependency-cycle",
            level: "error",
            message: format!("Stage dependency cycle: {}", cycle.join(" → ")),
            stage: None,
            row: stage.map(|stage| stage.row),
            col: stage.map(|stage| stage.col),
            details: vec![(
                "stages",
                Form::Vector(cycle.iter().map(|stage| keyword(stage)).collect()),
            )],
        };
        if finding.row == Some(0) {
            finding.row = None;
            finding.col = None;
        }
        findings.push(finding);
    }
    let producers = build
        .stages
        .iter()
        .filter(|stage| stage.produces == build.artifact_kind)
        .collect::<Vec<_>>();
    if producers.is_empty() {
        findings.push(BuildFinding {
            kind: "greenways/no-final-producer",
            level: "error",
            message: "No stage produces the declared artifact kind".into(),
            stage: None,
            row: None,
            col: None,
            details: vec![("artifact/kind", keyword(&build.artifact_kind))],
        });
    } else if producers.len() > 1 {
        findings.push(BuildFinding {
            kind: "greenways/conflicting-final-producers",
            level: "error",
            message: "Multiple stages produce the declared artifact kind".into(),
            stage: None,
            row: None,
            col: None,
            details: vec![(
                "stages",
                Form::Vector(producers.iter().map(|stage| keyword(&stage.id)).collect()),
            )],
        });
    }
    let reachable = reachable_build_stages(build);
    for stage in &build.stages {
        if !reachable.contains(&stage.id) {
            findings.push(stage_finding(
                stage,
                "greenways/unreachable-stage",
                "warning",
                format!(
                    "Stage does not contribute to the declared artifact: {}",
                    stage.id
                ),
                vec![],
            ));
        }
    }
    findings
}

fn stage_finding(
    stage: &BuildStage,
    kind: &'static str,
    level: &'static str,
    message: String,
    details: Vec<(&'static str, Form)>,
) -> BuildFinding {
    BuildFinding {
        kind,
        level,
        message,
        stage: Some(stage.id.clone()),
        row: (stage.row > 0).then_some(stage.row),
        col: (stage.col > 0).then_some(stage.col),
        details,
    }
}

fn build_cycles(build: &CanonicalBuild) -> Vec<Vec<String>> {
    let dependencies = build
        .stages
        .iter()
        .map(|stage| (stage.id.clone(), stage.requires.clone()))
        .collect::<HashMap<_, _>>();
    let mut cycles = Vec::new();
    let mut covered = HashSet::new();
    for stage in &build.stages {
        if covered.contains(&stage.id) {
            continue;
        }
        if let Some(cycle) = find_build_cycle(&stage.id, &dependencies, &mut Vec::new()) {
            covered.extend(cycle.iter().cloned());
            cycles.push(cycle);
        }
    }
    cycles
}

fn find_build_cycle(
    current: &str,
    dependencies: &HashMap<String, Vec<String>>,
    path: &mut Vec<String>,
) -> Option<Vec<String>> {
    if let Some(index) = path.iter().position(|stage| stage == current) {
        let mut cycle = path[index..].to_vec();
        cycle.push(current.into());
        return Some(cycle);
    }
    path.push(current.into());
    for dependency in dependencies.get(current).into_iter().flatten() {
        if dependencies.contains_key(dependency) {
            if let Some(cycle) = find_build_cycle(dependency, dependencies, path) {
                path.pop();
                return Some(cycle);
            }
        }
    }
    path.pop();
    None
}

fn stable_topological_order(build: &CanonicalBuild) -> Vec<String> {
    let mut ordered = Vec::new();
    let mut remaining = build
        .stages
        .iter()
        .map(|stage| stage.id.clone())
        .collect::<Vec<_>>();
    let dependencies = build
        .stages
        .iter()
        .map(|stage| (stage.id.clone(), stage.requires.clone()))
        .collect::<HashMap<_, _>>();
    while !remaining.is_empty() {
        let ready = remaining.iter().position(|id| {
            dependencies
                .get(id)
                .into_iter()
                .flatten()
                .all(|dependency| ordered.contains(dependency))
        });
        let Some(index) = ready else { break };
        ordered.push(remaining.remove(index));
    }
    ordered
}

fn reachable_build_stages(build: &CanonicalBuild) -> HashSet<String> {
    let dependencies = build
        .stages
        .iter()
        .map(|stage| (stage.id.clone(), stage.requires.clone()))
        .collect::<HashMap<_, _>>();
    let mut pending = build
        .stages
        .iter()
        .filter(|stage| stage.produces == build.artifact_kind)
        .map(|stage| stage.id.clone())
        .collect::<Vec<_>>();
    let mut reachable = HashSet::new();
    while let Some(stage) = pending.pop() {
        if reachable.insert(stage.clone()) {
            pending.extend(dependencies.get(&stage).cloned().unwrap_or_default());
        }
    }
    reachable
}

fn check_checker_identities(build: &CanonicalBuild) -> Vec<BuildFinding> {
    let mut findings = Vec::new();
    for stage in &build.stages {
        for checker in &stage.checkers {
            let Some(source) = map_get(checker, "checker/source") else {
                continue;
            };
            let checker_id = map_get(checker, "checker/id")
                .map(ToString::to_string)
                .unwrap_or_else(|| ":unknown".into());
            if map_get(source, "source/provider") != Some(&keyword("github")) {
                findings.push(stage_finding(
                    stage,
                    "greenways/checker-provider",
                    "error",
                    "Checker source provider must be :github".into(),
                    vec![("checker/id", parse(&checker_id).unwrap_or(Form::Nil))],
                ));
            }
            let repository = map_get(source, "source/repository").and_then(string_value);
            if !repository.as_deref().is_some_and(valid_github_repository) {
                findings.push(stage_finding(
                    stage,
                    "greenways/checker-repository",
                    "error",
                    "Checker repository must be owner/name".into(),
                    vec![("checker/id", parse(&checker_id).unwrap_or(Form::Nil))],
                ));
            }
            let commit = map_get(source, "source/commit").and_then(string_value);
            if !commit.as_deref().is_some_and(valid_full_git_sha) {
                findings.push(stage_finding(
                    stage,
                    "greenways/checker-commit",
                    "error",
                    "Checker commit must be a full immutable 40-character Git SHA".into(),
                    vec![("checker/id", parse(&checker_id).unwrap_or(Form::Nil))],
                ));
            }
            let source_path = map_get(source, "source/path").and_then(string_value);
            if !source_path.as_deref().is_some_and(valid_repository_path) {
                findings.push(stage_finding(
                    stage,
                    "greenways/checker-path",
                    "error",
                    "Checker path must be repository-relative".into(),
                    vec![("checker/id", parse(&checker_id).unwrap_or(Form::Nil))],
                ));
            }
            if !matches!(map_get(checker, "checker/entrypoint"), Some(Form::Symbol(value)) if value.split_once('/').is_some_and(|(namespace, name)| !namespace.is_empty() && !name.is_empty()))
            {
                findings.push(stage_finding(
                    stage,
                    "greenways/checker-entrypoint",
                    "error",
                    "Checker entrypoint must be a qualified symbol".into(),
                    vec![("checker/id", parse(&checker_id).unwrap_or(Form::Nil))],
                ));
            }
        }
    }
    findings
}

fn valid_github_repository(value: &str) -> bool {
    value
        .split_once('/')
        .is_some_and(|(owner, name)| !owner.is_empty() && !name.is_empty() && !name.contains('/'))
}

fn valid_full_git_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_repository_path(value: &str) -> bool {
    !value.is_empty() && !value.starts_with('/') && !value.split('/').any(|segment| segment == "..")
}

fn has_required_failures(findings: &[BuildFinding]) -> bool {
    findings
        .iter()
        .any(|finding| finding.level == "error" || finding.level == "unknown")
}

fn build_finding_form(build: &CanonicalBuild, finding: &BuildFinding) -> Form {
    let mut entries = vec![
        (keyword("type"), keyword(finding.kind)),
        (keyword("level"), keyword(finding.level)),
        (keyword("message"), string(&finding.message)),
    ];
    if let Some(stage) = &finding.stage {
        entries.push((keyword("stage/id"), keyword(stage)));
    }
    if let Some(row) = finding.row {
        entries.push((keyword("file"), string(&build.file)));
        entries.push((keyword("row"), Form::Number(row as i64)));
    }
    if let Some(col) = finding.col {
        entries.push((keyword("col"), Form::Number(col as i64)));
    }
    entries.extend(
        finding
            .details
            .iter()
            .map(|(key, value)| (keyword(key), value.clone())),
    );
    Form::Map(entries)
}

fn build_graph_form(build: &CanonicalBuild, findings: &[BuildFinding]) -> Form {
    let dependencies = Form::Map(
        build
            .stages
            .iter()
            .map(|stage| {
                (
                    keyword(&stage.id),
                    Form::Vector(
                        stage
                            .requires
                            .iter()
                            .map(|dependency| keyword(dependency))
                            .collect(),
                    ),
                )
            })
            .collect(),
    );
    map_form(vec![
        ("graph/type", keyword("greenways/build-dependencies")),
        ("graph/version", string("0.1.0")),
        ("build/id", keyword(&build.id)),
        ("stage/dependencies", dependencies),
        (
            "graph/topological-order",
            Form::Vector(
                stable_topological_order(build)
                    .iter()
                    .map(|stage| keyword(stage))
                    .collect(),
            ),
        ),
        (
            "graph/reachable-stages",
            Form::Vector(
                build
                    .stages
                    .iter()
                    .filter(|stage| reachable_build_stages(build).contains(&stage.id))
                    .map(|stage| keyword(&stage.id))
                    .collect(),
            ),
        ),
        (
            "findings",
            Form::Vector(
                findings
                    .iter()
                    .map(|finding| build_finding_form(build, finding))
                    .collect(),
            ),
        ),
    ])
}

fn build_obligation_report(build: &CanonicalBuild, findings: &[BuildFinding]) -> Form {
    let mut statuses = build
        .stages
        .iter()
        .map(|stage| {
            let failed = findings.iter().any(|finding| {
                finding.stage.as_deref() == Some(&stage.id) && finding.level == "error"
            });
            (stage.id.clone(), if failed { "fail" } else { "pass" })
        })
        .collect::<HashMap<_, _>>();
    let order = stable_topological_order(build);
    let mut blocked_by: HashMap<String, Vec<String>> = HashMap::new();
    for stage_id in order {
        let Some(stage) = build.stages.iter().find(|stage| stage.id == stage_id) else {
            continue;
        };
        if statuses.get(&stage.id) == Some(&"fail") {
            continue;
        }
        let blockers = stage
            .requires
            .iter()
            .filter(|dependency| {
                matches!(
                    statuses.get(*dependency),
                    Some(&"fail") | Some(&"unknown") | Some(&"blocked")
                ) || blocked_by.contains_key(*dependency)
            })
            .cloned()
            .collect::<Vec<_>>();
        if !blockers.is_empty() {
            statuses.insert(stage.id.clone(), "blocked");
            blocked_by.insert(stage.id.clone(), blockers);
        }
    }
    let stage_forms = build
        .stages
        .iter()
        .map(|stage| {
            let status = statuses.get(&stage.id).copied().unwrap_or("unknown");
            let own_findings = findings
                .iter()
                .filter(|finding| finding.stage.as_deref() == Some(&stage.id))
                .map(|finding| build_finding_form(build, finding))
                .collect::<Vec<_>>();
            let mut entries = vec![
                (keyword("stage/id"), keyword(&stage.id)),
                (keyword("stage/status"), keyword(status)),
            ];
            if !own_findings.is_empty() {
                entries.push((keyword("stage/findings"), Form::Vector(own_findings)));
            }
            if let Some(blockers) = blocked_by.get(&stage.id) {
                entries.push((
                    keyword("stage/blocked-by"),
                    Form::Vector(
                        blockers
                            .iter()
                            .map(|dependency| keyword(dependency))
                            .collect(),
                    ),
                ));
            }
            Form::Map(entries)
        })
        .collect::<Vec<_>>();
    let global_findings = findings
        .iter()
        .filter(|finding| finding.stage.is_none())
        .collect::<Vec<_>>();
    let mut counts = HashMap::new();
    for status in statuses.values() {
        *counts.entry(*status).or_insert(0i64) += 1;
    }
    for finding in &global_findings {
        if finding.level == "error" {
            *counts.entry("fail").or_insert(0) += 1;
        } else if finding.level == "unknown" {
            *counts.entry("unknown").or_insert(0) += 1;
        }
    }
    let status = if counts.get("blocked").copied().unwrap_or_default() > 0 {
        "blocked"
    } else if counts.get("fail").copied().unwrap_or_default() > 0 {
        "fail"
    } else if counts.get("unknown").copied().unwrap_or_default() > 0 {
        "unknown"
    } else {
        "pass"
    };
    let next_actions = findings
        .iter()
        .filter(|finding| matches!(finding.level, "error" | "unknown"))
        .map(|finding| {
            let mut entries = vec![
                (keyword("action/type"), keyword("repair")),
                (keyword("action/requirement"), keyword(finding.kind)),
            ];
            if let Some(stage) = &finding.stage {
                entries.push((keyword("action/stage"), keyword(stage)));
            }
            if let Some(row) = finding.row {
                entries.push((keyword("file"), string(&build.file)));
                entries.push((keyword("row"), Form::Number(row as i64)));
            }
            if let Some(col) = finding.col {
                entries.push((keyword("col"), Form::Number(col as i64)));
            }
            Form::Map(entries)
        })
        .collect::<Vec<_>>();
    let identities = build
        .stages
        .iter()
        .flat_map(|stage| {
            stage
                .checkers
                .iter()
                .filter(|checker| map_get(checker, "checker/source").is_some())
                .cloned()
        })
        .collect::<Vec<_>>();
    map_form(vec![
        ("report/type", keyword("greenways/build-check")),
        ("report/version", string("0.1.0")),
        ("build/id", keyword(&build.id)),
        ("build/status", keyword(status)),
        (
            "summary",
            map_form(vec![
                ("pass", Form::Number(*counts.get("pass").unwrap_or(&0))),
                ("fail", Form::Number(*counts.get("fail").unwrap_or(&0))),
                (
                    "unknown",
                    Form::Number(*counts.get("unknown").unwrap_or(&0)),
                ),
                (
                    "blocked",
                    Form::Number(*counts.get("blocked").unwrap_or(&0)),
                ),
            ]),
        ),
        ("stages", Form::Vector(stage_forms)),
        (
            "findings",
            Form::Vector(
                global_findings
                    .iter()
                    .map(|finding| build_finding_form(build, finding))
                    .collect(),
            ),
        ),
        ("next-actions", Form::Vector(next_actions)),
        ("checker/identities", Form::Vector(identities)),
    ])
}

fn build_report_status(report: &Form) -> &str {
    match map_get(report, "build/status") {
        Some(Form::Keyword(status)) => status,
        _ => "unknown",
    }
}

fn print_build_graph_text(build: &CanonicalBuild, findings: &[BuildFinding]) {
    println!("build :{} dependency graph", build.id);
    for stage in &build.stages {
        println!(
            "  :{} <- [{}]",
            stage.id,
            stage
                .requires
                .iter()
                .map(|dependency| format!(":{dependency}"))
                .collect::<Vec<_>>()
                .join(" ")
        );
    }
    for finding in findings {
        println!("{} :{} — {}", finding.level, finding.kind, finding.message);
    }
}

fn print_build_check_text(build: &CanonicalBuild, findings: &[BuildFinding]) {
    if has_required_failures(findings) {
        println!("build :{}: failed", build.id);
        for finding in findings {
            println!("{} :{} — {}", finding.level, finding.kind, finding.message);
        }
    } else {
        println!("build :{}: pass", build.id);
        for finding in findings.iter().filter(|finding| finding.level == "warning") {
            println!("warning :{} — {}", finding.kind, finding.message);
        }
    }
}

#[derive(Clone, Copy)]
enum SpecFormat {
    Text,
    Edn,
}

fn spec_format(args: &[String]) -> Result<SpecFormat, String> {
    match args {
        [] => Ok(SpecFormat::Text),
        [flag, value] if flag == "--format" && value == "text" => Ok(SpecFormat::Text),
        [flag, value] if flag == "--format" && value == "edn" => Ok(SpecFormat::Edn),
        _ => Err("spec format must be --format text or --format edn".into()),
    }
}

fn read_spec_document(source: &str) -> Result<Form, String> {
    let mut forms = read_forms(source).map_err(|error| error.to_string())?;
    if forms.len() != 1 {
        return Err("meta-spec must contain exactly one EDN form".into());
    }
    let form = forms.remove(0).form;
    if !matches!(form, Form::Map(_)) {
        return Err("meta-spec root must be an EDN map".into());
    }
    Ok(form)
}

fn keyword(name: &str) -> Form {
    Form::Keyword(name.into())
}

fn string(value: impl Into<String>) -> Form {
    Form::String(value.into())
}

fn map_form(entries: Vec<(&str, Form)>) -> Form {
    Form::Map(
        entries
            .into_iter()
            .map(|(key, value)| (keyword(key), value))
            .collect(),
    )
}

fn map_entries(value: &Form) -> Option<&[(Form, Form)]> {
    match value {
        Form::Map(entries) => Some(entries),
        _ => None,
    }
}

fn map_get<'a>(value: &'a Form, key: &str) -> Option<&'a Form> {
    map_entries(value)?.iter().find_map(|(candidate, value)| {
        matches!(candidate, Form::Keyword(name) if name == key).then_some(value)
    })
}

fn qualified_keyword(value: &Form) -> bool {
    matches!(value, Form::Keyword(name) if name.split_once('/').is_some_and(|(namespace, name)| !namespace.is_empty() && !name.is_empty()))
}

fn finding(
    rule: &'static str,
    requirement: &'static str,
    path: Vec<Form>,
    message: impl Into<String>,
    repair: Form,
) -> SpecFinding {
    SpecFinding {
        rule,
        requirement,
        path,
        message: message.into(),
        repair,
    }
}

fn walk_form(value: &Form, path: &mut Vec<Form>, function: &mut impl FnMut(&Form, &[Form])) {
    function(value, path);
    match value {
        Form::Map(entries) => {
            for (key, value) in entries {
                path.push(key.clone());
                walk_form(value, path, function);
                path.pop();
            }
        }
        Form::Vector(values) | Form::List(values) | Form::Set(values) => {
            for (index, value) in values.iter().enumerate() {
                path.push(Form::Number(index as i64));
                walk_form(value, path, function);
                path.pop();
            }
        }
        _ => {}
    }
}

fn lint_metaspec(document: &Form) -> Vec<SpecFinding> {
    let mut findings = Vec::new();
    for key in METASPEC_REQUIRED_KEYS {
        if map_get(document, key).is_none() {
            findings.push(finding(
                "hara.metaspec.rule/required-key",
                "hara.metaspec/required-sections",
                vec![],
                format!("Missing required meta-spec key: :{key}"),
                map_form(vec![
                    ("action/type", keyword("add-key")),
                    ("action/path", Form::Vector(vec![])),
                    ("action/key", keyword(key)),
                ]),
            ));
        }
    }

    let mut identifiers: HashMap<(String, String), Vec<Vec<Form>>> = HashMap::new();
    walk_form(document, &mut vec![], &mut |value, path| {
        let Some(entries) = map_entries(value) else {
            return;
        };
        let mut map_keys = HashSet::new();
        for (key, value) in entries {
            let key_path = path
                .iter()
                .cloned()
                .chain([key.clone()])
                .collect::<Vec<_>>();
            if !qualified_keyword(key) {
                findings.push(finding(
                    "hara.metaspec.rule/qualified-key",
                    "hara.metaspec/qualified-keys",
                    key_path.clone(),
                    format!("Map key must be a qualified keyword: {key}"),
                    map_form(vec![
                        ("action/type", keyword("qualify-key")),
                        ("action/path", Form::Vector(path.to_vec())),
                        ("action/key", key.clone()),
                    ]),
                ));
            }
            let key_text = match key {
                Form::Keyword(name) => name.as_str(),
                _ => "",
            };
            let duplicate_key = key.to_string();
            if !map_keys.insert(duplicate_key) {
                findings.push(finding(
                    "hara.metaspec.rule/duplicate-key",
                    "hara.metaspec/unique-identifiers",
                    key_path.clone(),
                    format!("Duplicate map key: {key}"),
                    map_form(vec![
                        ("action/type", keyword("remove-duplicate-key")),
                        ("action/path", Form::Vector(key_path.clone())),
                    ]),
                ));
            }
            if METASPEC_IDENTIFIER_KEYS.contains(&key_text) && !matches!(value, Form::Map(_)) {
                if !qualified_keyword(value) {
                    findings.push(finding(
                        "hara.metaspec.rule/stable-id",
                        "hara.metaspec/stable-identifiers",
                        key_path.clone(),
                        format!("Declaration ID must be a qualified keyword: {value}"),
                        map_form(vec![
                            ("action/type", keyword("replace-value")),
                            ("action/path", Form::Vector(key_path.clone())),
                            ("action/expected", keyword("qualified-keyword")),
                        ]),
                    ));
                }
                identifiers
                    .entry((key_text.into(), value.to_string()))
                    .or_default()
                    .push(key_path);
            }
        }
    });
    let mut duplicate_ids = identifiers
        .into_iter()
        .filter(|(_, paths)| paths.len() > 1)
        .collect::<Vec<_>>();
    duplicate_ids.sort_by(|left, right| left.0.cmp(&right.0));
    for ((_, value), paths) in duplicate_ids {
        findings.push(finding(
            "hara.metaspec.rule/duplicate-id",
            "hara.metaspec/unique-identifiers",
            paths[1].clone(),
            format!("Duplicate declaration identifier: {value}"),
            map_form(vec![
                ("action/type", keyword("rename-id")),
                ("action/path", Form::Vector(paths[1].clone())),
                (
                    "action/value",
                    parse(&value).unwrap_or_else(|_| string(value)),
                ),
            ]),
        ));
    }
    findings
}

fn verify_metaspec(document: &Form, path: &Path) -> Vec<SpecFinding> {
    let mut findings = Vec::new();
    let mut schema_ids = HashSet::new();
    if let Some(id) =
        map_get(document, "meta/document-schema").and_then(|schema| map_get(schema, "schema/id"))
    {
        schema_ids.insert(id.to_string());
    }
    if let Some(Form::Vector(schemas)) = map_get(document, "meta/schemas") {
        for schema in schemas {
            if let Some(id) = map_get(schema, "schema/id") {
                schema_ids.insert(id.to_string());
            }
        }
    }
    walk_form(document, &mut vec![], &mut |value, value_path| {
        let Some(entries) = map_entries(value) else {
            return;
        };
        for (key, reference) in entries {
            let is_schema_reference = matches!(key, Form::Keyword(name) if name == "schema/ref" || name == "schema/items");
            if is_schema_reference
                && matches!(reference, Form::Keyword(_))
                && !schema_ids.contains(&reference.to_string())
            {
                let reference_path = value_path
                    .iter()
                    .cloned()
                    .chain([key.clone()])
                    .collect::<Vec<_>>();
                findings.push(finding(
                    "hara.metaspec.rule/schema-reference",
                    "hara.metaspec/resolved-schema-references",
                    reference_path,
                    format!("Unresolved schema reference: {reference}"),
                    map_form(vec![
                        ("action/type", keyword("declare-schema")),
                        ("action/schema-id", reference.clone()),
                        ("action/path", Form::Vector(vec![keyword("meta/schemas")])),
                    ]),
                ));
            }
        }
    });

    if let Some(Form::Vector(references)) = map_get(document, "meta/cross-references") {
        for (index, reference) in references.iter().enumerate() {
            let base = vec![keyword("meta/cross-references"), Form::Number(index as i64)];
            if map_get(reference, "reference/id").is_none()
                || map_get(reference, "reference/from").is_none()
                || map_get(reference, "reference/to").is_none()
            {
                findings.push(finding(
                    "hara.metaspec.rule/cross-reference",
                    "hara.metaspec/resolved-cross-references",
                    base.clone(),
                    "Cross-reference declaration requires :reference/id, :reference/from and :reference/to",
                    map_form(vec![
                        ("action/type", keyword("complete-cross-reference")),
                        ("action/path", Form::Vector(base)),
                    ]),
                ));
            }
        }
    }

    if let Some(spec_id) =
        map_get(document, "spec/conforms-to").and_then(|reference| map_get(reference, "spec/id"))
    {
        let own_id = map_get(document, "document/id");
        if own_id != Some(spec_id) && !sibling_document_ids(path).contains(&spec_id.to_string()) {
            findings.push(finding(
                "hara.metaspec.rule/conforms-to",
                "hara.metaspec/resolved-cross-references",
                vec![keyword("spec/conforms-to"), keyword("spec/id")],
                format!("Unresolved conforming meta-spec: {spec_id}"),
                map_form(vec![
                    ("action/type", keyword("register-spec")),
                    ("action/spec-id", spec_id.clone()),
                ]),
            ));
        }
    }
    findings
}

fn sibling_document_ids(path: &Path) -> HashSet<String> {
    let Some(parent) = path.parent() else {
        return HashSet::new();
    };
    let Ok(entries) = fs::read_dir(parent) else {
        return HashSet::new();
    };
    let mut paths = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|candidate| {
            candidate
                .extension()
                .is_some_and(|extension| extension == "edn")
        })
        .collect::<Vec<_>>();
    paths.sort();
    paths
        .into_iter()
        .filter(|candidate| candidate != path)
        .filter_map(|candidate| fs::read_to_string(candidate).ok())
        .filter_map(|source| read_spec_document(&source).ok())
        .filter_map(|document| map_get(&document, "document/id").map(ToString::to_string))
        .collect()
}

fn validate_against_metaspec(
    document: &Form,
    metaspec: &Form,
    document_path: &Path,
) -> Vec<SpecFinding> {
    let mut schemas = HashMap::new();
    if let Some(schema) = map_get(metaspec, "meta/document-schema") {
        if let Some(id) = map_get(schema, "schema/id") {
            schemas.insert(id.to_string(), schema);
        }
    }
    if let Some(Form::Vector(declarations)) = map_get(metaspec, "meta/schemas") {
        for schema in declarations {
            if let Some(id) = map_get(schema, "schema/id") {
                schemas.insert(id.to_string(), schema);
            }
        }
    }
    let mut findings = Vec::new();
    if let Some(schema) = map_get(metaspec, "meta/document-schema") {
        validate_schema_value(document, schema, &schemas, &mut vec![], &mut findings);
    } else {
        findings.push(schema_validation_finding(
            vec![],
            "Meta-spec is missing :meta/document-schema",
            map_form(vec![
                ("action/type", keyword("add-key")),
                ("action/key", keyword("meta/document-schema")),
            ]),
        ));
    }

    if let Some(expected) = map_get(metaspec, "document/id") {
        let actual = map_get(document, "spec/conforms-to")
            .and_then(|reference| map_get(reference, "spec/id"));
        if actual != Some(expected) {
            findings.push(finding(
                "hara.metaspec.rule/document-reference",
                "hara.metaspec/generated-document-conformance",
                vec![keyword("spec/conforms-to"), keyword("spec/id")],
                format!("Document must conform to {expected}"),
                map_form(vec![
                    ("action/type", keyword("replace-value")),
                    (
                        "action/path",
                        Form::Vector(vec![keyword("spec/conforms-to"), keyword("spec/id")]),
                    ),
                    ("action/value", expected.clone()),
                ]),
            ));
        }
    }
    findings.extend(validate_declared_references(
        document,
        metaspec,
        document_path,
    ));
    findings
}

fn validate_schema_value(
    value: &Form,
    schema: &Form,
    schemas: &HashMap<String, &Form>,
    path: &mut Vec<Form>,
    findings: &mut Vec<SpecFinding>,
) {
    if let Some(reference) = map_get(schema, "schema/ref") {
        if let Some(resolved) = schemas.get(&reference.to_string()) {
            validate_schema_value(value, resolved, schemas, path, findings);
        } else {
            findings.push(schema_validation_finding(
                path.clone(),
                format!("Cannot validate unresolved schema: {reference}"),
                map_form(vec![
                    ("action/type", keyword("declare-schema")),
                    ("action/schema-id", reference.clone()),
                ]),
            ));
        }
        return;
    }
    if let Some(expected) = map_get(schema, "schema/value") {
        if value != expected {
            findings.push(schema_validation_finding(
                path.clone(),
                format!("Expected exact value {expected}, got {value}"),
                map_form(vec![
                    ("action/type", keyword("replace-value")),
                    ("action/path", Form::Vector(path.clone())),
                    ("action/value", expected.clone()),
                ]),
            ));
        }
    }
    if let Some(Form::Keyword(schema_type)) = map_get(schema, "schema/type") {
        let valid = match schema_type.as_str() {
            "map" => matches!(value, Form::Map(_)),
            "vector" => matches!(value, Form::Vector(_)),
            "keyword" => matches!(value, Form::Keyword(_)),
            "symbol" => matches!(value, Form::Symbol(_)),
            "string" => matches!(value, Form::String(_)),
            "enum" => map_get(schema, "schema/values")
                .and_then(|values| match values {
                    Form::Vector(values) => Some(values.contains(value)),
                    _ => None,
                })
                .unwrap_or(false),
            _ => true,
        };
        if !valid {
            findings.push(schema_validation_finding(
                path.clone(),
                format!("Expected :{schema_type}, got {value}"),
                map_form(vec![
                    ("action/type", keyword("replace-value")),
                    ("action/path", Form::Vector(path.clone())),
                    ("action/expected", keyword(schema_type)),
                ]),
            ));
            return;
        }
    }
    if map_get(schema, "schema/constraint") == Some(&keyword("qualified"))
        && !qualified_keyword(value)
    {
        findings.push(schema_validation_finding(
            path.clone(),
            format!("Expected a qualified keyword, got {value}"),
            map_form(vec![
                ("action/type", keyword("qualify-value")),
                ("action/path", Form::Vector(path.clone())),
            ]),
        ));
    }
    if let (Form::String(value), Some(Form::Number(minimum))) =
        (value, map_get(schema, "schema/min-length"))
    {
        if value.chars().count() < *minimum as usize {
            findings.push(schema_validation_finding(
                path.clone(),
                format!("String must contain at least {minimum} character(s)"),
                map_form(vec![
                    ("action/type", keyword("replace-value")),
                    ("action/path", Form::Vector(path.clone())),
                    ("action/min-length", Form::Number(*minimum)),
                ]),
            ));
        }
    }
    if let Form::Map(_) = value {
        if let Some(Form::Vector(required)) = map_get(schema, "schema/required") {
            for key in required {
                let Some(Form::Keyword(name)) = Some(key) else {
                    continue;
                };
                if map_get(value, name).is_none() {
                    findings.push(schema_validation_finding(
                        path.clone(),
                        format!("Missing required key: {key}"),
                        map_form(vec![
                            ("action/type", keyword("add-key")),
                            ("action/path", Form::Vector(path.clone())),
                            ("action/key", key.clone()),
                        ]),
                    ));
                }
            }
        }
        if let Some(Form::Map(properties)) = map_get(schema, "schema/properties") {
            for (key, property_schema) in properties {
                let Form::Keyword(name) = key else { continue };
                if let Some(property_value) = map_get(value, name) {
                    path.push(key.clone());
                    validate_schema_value(property_value, property_schema, schemas, path, findings);
                    path.pop();
                }
            }
        }
    }
    if let (Form::Vector(values), Some(item_schema)) = (value, map_get(schema, "schema/items")) {
        let resolved = schemas.get(&item_schema.to_string()).copied();
        if let Some(resolved) = resolved {
            for (index, item) in values.iter().enumerate() {
                path.push(Form::Number(index as i64));
                validate_schema_value(item, resolved, schemas, path, findings);
                path.pop();
            }
        }
    }
}

fn schema_validation_finding(
    path: Vec<Form>,
    message: impl Into<String>,
    repair: Form,
) -> SpecFinding {
    finding(
        "hara.metaspec.rule/schema-validation",
        "hara.metaspec/generated-document-conformance",
        path,
        message,
        repair,
    )
}

fn collect_field_values(document: &Form, field: &Form) -> Vec<Form> {
    let mut values = Vec::new();
    walk_form(document, &mut vec![], &mut |value, _| {
        if let Some(entries) = map_entries(value) {
            for (key, value) in entries {
                if key == field {
                    match value {
                        Form::Vector(items) => values.extend(items.iter().cloned()),
                        value => values.push(value.clone()),
                    }
                }
            }
        }
    });
    values
}

fn validate_declared_references(
    document: &Form,
    metaspec: &Form,
    document_path: &Path,
) -> Vec<SpecFinding> {
    let mut findings = Vec::new();
    let Some(Form::Vector(references)) = map_get(metaspec, "meta/cross-references") else {
        return findings;
    };
    for reference in references {
        let Some(from) = map_get(reference, "reference/from") else {
            continue;
        };
        let Some(to) = map_get(reference, "reference/to") else {
            continue;
        };
        let source_fields = match from {
            Form::Vector(fields) => fields.clone(),
            field => vec![field.clone()],
        };
        let source_values = source_fields
            .iter()
            .flat_map(|field| collect_field_values(document, field))
            .collect::<Vec<_>>();
        if to == &keyword("document-relative-path") {
            for source in source_values {
                let Form::String(relative) = source else {
                    continue;
                };
                let resolved = document_path
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join(&relative);
                if !resolved.is_file() {
                    findings.push(finding(
                        "hara.metaspec.rule/document-reference",
                        "hara.metaspec/generated-document-conformance",
                        vec![],
                        format!("Document-relative reference does not exist: {relative}"),
                        map_form(vec![
                            ("action/type", keyword("create-referenced-file")),
                            ("action/path", string(relative)),
                        ]),
                    ));
                }
            }
            continue;
        }
        let targets = collect_field_values(document, to)
            .into_iter()
            .map(|value| value.to_string())
            .collect::<HashSet<_>>();
        for source in source_values {
            if !targets.contains(&source.to_string()) {
                findings.push(finding(
                    "hara.metaspec.rule/document-reference",
                    "hara.metaspec/generated-document-conformance",
                    vec![],
                    format!("Unresolved document reference: {source} -> {to}"),
                    map_form(vec![
                        ("action/type", keyword("declare-reference-target")),
                        ("action/target-field", to.clone()),
                        ("action/value", source),
                    ]),
                ));
            }
        }
    }
    findings
}

fn spec_finding_form(finding: &SpecFinding) -> Form {
    map_form(vec![
        ("finding/id", keyword(finding.rule)),
        ("rule/id", keyword(finding.rule)),
        ("requirement/id", keyword(finding.requirement)),
        ("finding/level", keyword("error")),
        ("finding/path", Form::Vector(finding.path.clone())),
        ("finding/message", string(&finding.message)),
        ("finding/repair", finding.repair.clone()),
    ])
}

fn metaspec_report(document: &Form, findings: &[SpecFinding]) -> Form {
    let failed = findings.len() as i64;
    let status = if findings.is_empty() { "pass" } else { "fail" };
    map_form(vec![
        ("report/type", keyword("hara/metaspec-verification")),
        ("report/version", string("0.1.0")),
        (
            "document/id",
            map_get(document, "document/id")
                .cloned()
                .unwrap_or(Form::Nil),
        ),
        ("report/status", keyword(status)),
        (
            "summary",
            map_form(vec![
                (
                    "pass",
                    Form::Number(if findings.is_empty() { 1 } else { 0 }),
                ),
                ("fail", Form::Number(failed)),
                ("unknown", Form::Number(0)),
                ("blocked", Form::Number(0)),
            ]),
        ),
        (
            "findings",
            Form::Vector(findings.iter().map(spec_finding_form).collect()),
        ),
        (
            "next-actions",
            Form::Vector(
                findings
                    .iter()
                    .map(|finding| {
                        map_form(vec![
                            ("action/type", keyword("repair-finding")),
                            ("action/rule", keyword(finding.rule)),
                            ("action/requirement", keyword(finding.requirement)),
                            ("action/path", Form::Vector(finding.path.clone())),
                            ("action/repair", finding.repair.clone()),
                        ])
                    })
                    .collect(),
            ),
        ),
    ])
}

fn print_metaspec_text(document: &Form, findings: &[SpecFinding]) {
    let id = map_get(document, "document/id")
        .map(ToString::to_string)
        .unwrap_or_else(|| "<missing :document/id>".into());
    if findings.is_empty() {
        println!("meta-spec {id}: pass");
    } else {
        println!("meta-spec {id}: {} finding(s)", findings.len());
        for finding in findings {
            println!(
                "error {} at {} — {}",
                finding.rule,
                Form::Vector(finding.path.clone()),
                finding.message
            );
        }
    }
}

fn metaspec_template() -> Form {
    map_form(vec![
        ("document/id", keyword("example/metaspec")),
        ("document/type", keyword("hara/metaspec")),
        ("document/version", string("0.1.0")),
        ("document/status", keyword("draft")),
        ("document/title", string("Example Meta-Specification")),
        (
            "document/summary",
            string("Describe the generated artifact contract."),
        ),
        (
            "spec/conforms-to",
            map_form(vec![
                ("spec/id", keyword("hara/metaspec-metaspec")),
                ("spec/version", string("0.1.0")),
            ]),
        ),
        ("spec/artifact-kind", keyword("example/artifact")),
        (
            "meta/document-schema",
            map_form(vec![
                ("schema/id", keyword("example/document")),
                ("schema/type", keyword("map")),
            ]),
        ),
        ("meta/schemas", Form::Vector(vec![])),
        ("meta/cross-references", Form::Vector(vec![])),
        ("meta/requirements", Form::Vector(vec![])),
        (
            "metaspec/generation",
            map_form(vec![
                ("generation/input", map_form(vec![])),
                ("generation/output", map_form(vec![])),
                ("generation/process", Form::Vector(vec![])),
                ("generation/acceptance", map_form(vec![])),
            ]),
        ),
    ])
}

#[cfg(feature = "hir-encoder")]
fn compile_hir(args: &[String]) -> Result<(), String> {
    let source_path = args
        .first()
        .ok_or_else(|| "compile-hir requires SOURCE.hal --output OUTPUT.hir".to_owned())?;
    let output_index = args
        .iter()
        .position(|argument| argument == "--output")
        .ok_or_else(|| "compile-hir requires --output OUTPUT.hir".to_owned())?;
    let output_path = args
        .get(output_index + 1)
        .ok_or_else(|| "compile-hir requires --output OUTPUT.hir".to_owned())?;
    let source = fs::read_to_string(source_path)
        .map_err(|error| format!("cannot read {source_path}: {error}"))?;
    let forms = parse_forms(&source)?;
    let namespace = forms
        .iter()
        .find_map(|form| match form {
            Form::List(values)
                if matches!(values.first(), Some(Form::Symbol(head)) if head == "ns" || head == "ns+") =>
            {
                match values.get(1) {
                    Some(Form::Symbol(namespace)) => Some(namespace.clone()),
                    _ => None,
                }
            }
            _ => None,
        })
        .ok_or_else(|| format!("{source_path} does not declare an ns or ns+ namespace"))?;
    let artifact = encode_hir_module(&namespace, source_path, &source, forms);
    if let Some(parent) = std::path::Path::new(output_path).parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    fs::write(output_path, artifact).map_err(|error| format!("cannot write {output_path}: {error}"))
}

fn project_for(options: &Options, args: &[String]) -> Result<project::Project, String> {
    let path = args
        .first()
        .map(PathBuf::from)
        .or_else(|| options.project.clone())
        .unwrap_or_else(|| PathBuf::from("."));
    project::discover(&path)
}

fn new_project(args: &[String]) -> Result<(), String> {
    let name = args
        .first()
        .ok_or_else(|| "new requires a project name".to_owned())?;
    if args.len() > 1 {
        return Err("new accepts exactly one project name".into());
    }
    let project = project::new_app(&PathBuf::from(name), name)?;
    println!("created {}", project.root.display());
    Ok(())
}

fn check_project(options: &Options, args: &[String]) -> Result<(), String> {
    let project = project_for(options, args)?;
    println!("project check: {} {}", project.id, project.version);
    Ok(())
}

fn edit_dependency(options: &Options, args: &[String], add: bool) -> Result<(), String> {
    let coordinate = args.first().ok_or_else(|| {
        if add {
            "add requires COORDINATE@RANGE".to_owned()
        } else {
            "remove requires COORDINATE".to_owned()
        }
    })?;
    if args.len() > 1 {
        return Err("dependency commands accept one coordinate".into());
    }
    let (coordinate, version) = if add {
        coordinate
            .rsplit_once('@')
            .ok_or_else(|| "add requires COORDINATE@RANGE".to_owned())?
    } else {
        (coordinate.as_str(), "")
    };
    let project = project_for(options, &[])?;
    project::set_dependency(&project, coordinate, if add { Some(version) } else { None })?;
    println!("{} {}", if add { "added" } else { "removed" }, coordinate);
    Ok(())
}

fn sync_project(options: &Options, args: &[String]) -> Result<(), String> {
    let project = project_for(options, &[])?;
    let flags: Vec<_> = args.iter().skip(1).collect();
    let mode = match flags.as_slice() {
        [] if options.offline => project::LockMode::Offline,
        [] => project::LockMode::Default,
        [flag] if (*flag).as_str() == "--offline" => project::LockMode::Offline,
        [flag] if (*flag).as_str() == "--locked" => project::LockMode::Locked,
        [flag] if (*flag).as_str() == "--frozen" => project::LockMode::Frozen,
        _ => return Err("sync accepts at most one of --offline, --locked, or --frozen".into()),
    };
    let lock = project::sync_lock(&project, mode)?;
    println!("project sync: {}", lock.display());
    Ok(())
}

fn run_project(options: &Options) -> Result<(), String> {
    let project = project_for(options, &[])?;
    let path = project::main_file(&project)?;
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let mut runtime = Runtime::new();
    runtime.install_native_file_provider(project.root.to_string_lossy().as_ref());
    project::register_sources(&project, &mut runtime)?;
    if options.native_sockets {
        runtime.install_native_socket_provider();
    }
    println!("{}", runtime.eval_native(&source)?);
    Ok(())
}

fn test_project(options: &Options, args: &[String]) -> Result<(), String> {
    if args.len() > 1 {
        return Err("test accepts at most one path".into());
    }
    let project = project_for(options, args)?;
    let files = match args.first().map(PathBuf::from) {
        Some(path)
            if path.is_file()
                && path.extension().and_then(|extension| extension.to_str()) == Some("hal") =>
        {
            vec![path]
        }
        Some(path) if path.is_file() => return Err("test file must use the .hal extension".into()),
        Some(path) if path.is_dir() => project::files_in(&path, &[PathBuf::new()])?,
        Some(path) => return Err(format!("test path does not exist: {}", path.display())),
        None => project::files_in(&project.root, &project.test_paths)?,
    };
    if files.is_empty() {
        return Err("project has no .hal files under :project/test-paths".into());
    }
    let mut passed = 0usize;
    let mut failed = 0usize;
    for path in files {
        let source = fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let mut runtime = Runtime::new();
        runtime.install_native_file_provider(project.root.to_string_lossy().as_ref());
        project::register_sources(&project, &mut runtime)?;
        runtime.eval_native(include_str!("../../../../lib/src/std/lib/test.hal"))?;
        let evaluated = runtime.eval_native(&source)?;
        match test_results(&evaluated) {
            Ok((file_passed, file_failed)) => {
                passed += file_passed;
                failed += file_failed;
                println!(
                    "test {}: {} passed, {} failed",
                    path.display(),
                    file_passed,
                    file_failed
                );
                if file_failed > 0 {
                    eprintln!("{evaluated}");
                }
            }
            Err(error) => {
                failed += 1;
                eprintln!("test {}: {error}", path.display());
            }
        }
    }
    println!("test result: {passed} passed, {failed} failed");
    if failed == 0 {
        Ok(())
    } else {
        Err("test failures".into())
    }
}

fn test_results(value: &str) -> Result<(usize, usize), String> {
    let Form::String(source) = parse(value)? else {
        return Err("test file must finish with test/print-results".into());
    };
    let Form::Vector(results) = parse(&source)? else {
        return Err("test/print-results must return a vector".into());
    };
    let mut passed = 0;
    let mut failed = 0;
    for result in results {
        let Form::Map(entries) = result else {
            return Err("test result must be a map".into());
        };
        let pass = entries
            .iter()
            .find(|(key, _)| matches!(key, Form::Keyword(name) if name == "pass"))
            .map(|(_, value)| value);
        match pass {
            Some(Form::Bool(true)) => passed += 1,
            Some(Form::Bool(false)) => failed += 1,
            _ => return Err("test result is missing boolean :pass".into()),
        }
    }
    Ok((passed, failed))
}

fn direct_eval(options: &Options, source: &str) -> Result<(), String> {
    if source.is_empty() {
        return Err("eval requires a Hara expression".into());
    }
    let mut runtime = Runtime::new();
    if let Some(root) = &options.root {
        runtime.install_native_file_provider(root.to_string_lossy().as_ref());
    }
    if options.native_sockets {
        runtime.install_native_socket_provider();
    }
    println!("{}", runtime.eval_native(source)?);
    Ok(())
}

fn run_file(options: &Options, path: &str) -> Result<(), String> {
    let bytes = fs::read(path).map_err(|error| format!("cannot read {path}: {error}"))?;
    let is_hir = path.ends_with(".hir") || bytes.starts_with(b"HIR\0");
    let mut runtime = Runtime::new();
    if let Some(root) = &options.root {
        runtime.install_native_file_provider(root.to_string_lossy().as_ref());
    }
    if options.native_sockets {
        runtime.install_native_socket_provider();
    }
    if is_hir {
        println!("{}", runtime.eval_hir(&bytes)?);
    } else {
        println!(
            "{}",
            runtime.eval_native(
                &String::from_utf8(bytes)
                    .map_err(|error| format!("{path} is not valid UTF-8: {error}"))?
            )?
        );
    }
    Ok(())
}

fn run_headless(options: &Options) -> Result<(), String> {
    if options.offline {
        return Err("--offline cannot be used with headless".into());
    }
    let broker = RuntimeBroker::start_with(options.root.clone(), options.native_sockets)?;
    let server = RespServer::start(&options.host, options.port, broker)?;
    println!("HARA RESP {} · session ROOT", server.endpoint());
    loop {
        std::thread::park();
    }
}

fn run_remote(endpoint: &str) -> Result<(), String> {
    let (host, port) = repl::parse_endpoint(endpoint, "127.0.0.1")?;
    let stream = TcpStream::connect((host.as_str(), port))
        .map_err(|error| format!("remote connect failed: {error}"))?;
    let mut connection = RespConnection::new(stream)?;
    connection.write(&RespValue::array(["HELLO", "4", "CLIENT", "HARA-REMOTE"]))?;
    println!(
        "{}",
        response_text(connection.read()?.ok_or("remote closed")?)
    );
    let mut request = 0_u64;
    for line in io::stdin().lock().lines() {
        let source = line.map_err(|error| format!("stdin: {error}"))?;
        if matches!(source.trim(), "/quit" | ":quit") {
            connection.write(&RespValue::array(["QUIT"]))?;
            break;
        }
        request += 1;
        let id = format!("REMOTE-{request}");
        connection.write(&RespValue::array(["EVAL", &id, source.trim()]))?;
        if let Some(value) = connection.read()? {
            println!("{}", response_text(value));
        }
        let _ = connection.read()?;
    }
    Ok(())
}

fn response_text(value: RespValue) -> String {
    match value {
        RespValue::Array(Some(values)) => values
            .into_iter()
            .map(response_text)
            .collect::<Vec<_>>()
            .join(" "),
        RespValue::Bulk(Some(bytes)) => String::from_utf8_lossy(&bytes).into_owned(),
        RespValue::Simple(value) | RespValue::Error(value) => value,
        RespValue::Integer(value) => value.to_string(),
        RespValue::Bulk(None) | RespValue::Array(None) => "nil".into(),
    }
}

fn usage() {
    println!("Hara CLI · Rust runtime");
    println!();
    println!("Usage:");
    println!("  hara [OPTIONS] repl");
    println!("  hara eval EXPRESSION | run FILE | stdin");
    println!("  hara server | remote HOST:PORT");
    println!("  hara project <new|check|run|test|add|remove|sync|update> ...");
    println!("  hara package <COMMAND> ...");
    println!("  hara spec <COMMAND> ...");
    println!("  hara extension <check|build|install|test> ...");
    println!();
    println!("Compatibility aliases:");
    println!("  new check test add remove sync update headless standalone");
    println!();
    println!("Global options:");
    println!("  --project PATH, --root PATH, --offline");
    println!("  --allow-file, --allow-net, --allow-process");
    println!("  --host HOST, --port PORT, --history PATH");
    println!("  --no-history, --no-splash, --no-color, --log-requests");
}

pub(crate) fn exit_error(message: &str, status: i32) -> ! {
    eprintln!("hara: {message}");
    std::process::exit(status)
}

#[cfg(test)]
mod spec_tests {
    use super::*;

    #[test]
    fn nested_route_operation_is_preserved_for_the_legacy_adapter() {
        assert_eq!(
            routed_command(&[
                "spec".into(),
                "check-contribution".into(),
                "candidate".into()
            ]),
            ["spec", "check-contribution", "candidate"]
        );
    }

    #[test]
    fn offline_daemon_rejection_is_a_usage_error() {
        assert_eq!(
            error_exit_code("--offline cannot be used with headless"),
            cli_app::CliOutcome::UsageError.exit_code()
        );
    }

    #[test]
    fn generated_metaspec_template_lints_cleanly() {
        assert!(lint_metaspec(&metaspec_template()).is_empty());
    }

    #[test]
    fn missing_keys_have_agent_repair_actions() {
        let document = parse("{}").unwrap();
        let findings = lint_metaspec(&document);
        assert_eq!(findings.len(), METASPEC_REQUIRED_KEYS.len());
        assert_eq!(findings[0].rule, "hara.metaspec.rule/required-key");
        assert_eq!(
            findings[0].repair,
            map_form(vec![
                ("action/type", keyword("add-key")),
                ("action/path", Form::Vector(vec![])),
                ("action/key", keyword("document/id")),
            ])
        );
    }

    #[test]
    fn duplicate_ids_and_map_keys_are_not_silently_overwritten() {
        assert!(
            read_spec_document("{:document/id :demo/spec :document/id :demo/other}")
                .unwrap_err()
                .contains("Duplicate key")
        );
        let document = read_spec_document(
            "{:document/id :demo/spec
              :meta/schemas [{:schema/id :demo/value}
                             {:schema/id :demo/value}]}",
        )
        .unwrap();
        let rules = lint_metaspec(&document)
            .into_iter()
            .map(|finding| finding.rule)
            .collect::<Vec<_>>();
        assert!(rules.contains(&"hara.metaspec.rule/duplicate-id"));
    }

    #[test]
    fn unresolved_schema_references_fail_verification() {
        let mut document = metaspec_template();
        let Form::Map(entries) = &mut document else {
            unreachable!()
        };
        entries.push((
            keyword("example/schema-use"),
            map_form(vec![("schema/ref", keyword("missing/schema"))]),
        ));
        let findings = verify_metaspec(&document, Path::new("metaspec.edn"));
        assert!(findings
            .iter()
            .any(|finding| finding.rule == "hara.metaspec.rule/schema-reference"));
        let report = metaspec_report(&document, &findings);
        assert_eq!(map_get(&report, "report/status"), Some(&keyword("fail")));
    }

    #[test]
    fn greenways_buildspec_validates_against_artifact_metaspec() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let document_path =
            repository.join("contrib/greenways/build/spec/draft/greenways-buildspec.edn");
        let metaspec_path = repository.join("specs/metaspec/draft/hal-artifact-metaspec.edn");
        let document = read_spec_document(&fs::read_to_string(&document_path).unwrap()).unwrap();
        let metaspec = read_spec_document(&fs::read_to_string(metaspec_path).unwrap()).unwrap();
        assert!(validate_against_metaspec(&document, &metaspec, &document_path).is_empty());
    }

    #[test]
    fn build_surface_normalizes_to_exact_canonical_edn() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let source_path = repository.join("contrib/greenways/build/examples/minimal-build.hal");
        let edn_path = repository.join("contrib/greenways/build/examples/minimal-build.edn");
        let source = fs::read_to_string(&source_path).unwrap();
        let canonical = read_spec_document(&fs::read_to_string(edn_path).unwrap()).unwrap();
        let (build, findings) = read_build_source(&source, source_path.to_str().unwrap()).unwrap();
        assert!(findings.is_empty());
        assert_eq!(canonical_build_form(&build), canonical);
    }

    #[test]
    fn build_edn_surface_round_trip_is_semantically_exact() {
        let canonical = read_spec_document(
            "{:greenways/type :build :greenways/version \"0.1.0\"
              :build/id :demo
              :build/artifact {:artifact/kind :demo/output
                               :artifact/output \"dist/demo.hal\"}
              :build/specs []
              :build/stages
              [{:stage/id :source :stage/requires []
                :stage/produces :demo/source :stage/checkers []}
               {:stage/id :output :stage/requires [:source]
                :stage/produces :demo/output :stage/checkers []}]}",
        )
        .unwrap();
        let (build, _) = canonical_build_from_edn(&canonical).unwrap();
        let surface = write_build_surface(&build);
        let (round_trip, findings) = read_build_source(&surface, "round-trip.hal").unwrap();
        assert!(findings.is_empty());
        assert_eq!(canonical_build_form(&round_trip), canonical);
    }

    #[test]
    fn build_cycle_and_blocked_checker_reports_are_structured() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let cycle_path = repository.join("contrib/greenways/build/examples/invalid-cycle.hal");
        let (cycle, parse_findings) = read_build_source(
            &fs::read_to_string(&cycle_path).unwrap(),
            cycle_path.to_str().unwrap(),
        )
        .unwrap();
        assert!(parse_findings.is_empty());
        let graph_findings = check_build_graph(&cycle);
        assert!(graph_findings.iter().any(|finding| {
            finding.kind == "greenways/dependency-cycle"
                && finding.message.contains("parse → emit → analyze → parse")
        }));

        let checker_path = repository.join("contrib/greenways/build/examples/invalid-checker.hal");
        let (checker_build, _) = read_build_source(
            &fs::read_to_string(&checker_path).unwrap(),
            checker_path.to_str().unwrap(),
        )
        .unwrap();
        let findings = check_build(&checker_build);
        assert!(findings
            .iter()
            .any(|finding| finding.kind == "greenways/checker-commit"));
        let report = build_obligation_report(&checker_build, &findings);
        assert_eq!(build_report_status(&report), "blocked");
    }

    #[test]
    fn greenways_contribution_envelopes_verify_offline() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        for path in [
            "contrib/greenways/build",
            "contrib/greenways/supersonic",
            "contrib/greenways/usdskel",
        ] {
            let root = repository.join(path);
            let envelope =
                read_spec_document(&fs::read_to_string(root.join("CONTRIBUTION.edn")).unwrap())
                    .unwrap();
            assert!(
                check_contribution(&envelope, &root, repository).is_empty(),
                "{path} did not verify"
            );
        }
    }
}
