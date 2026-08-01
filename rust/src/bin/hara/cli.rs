use crate::repl;
use hara_wasm::asset;
use hara_wasm::cli_app;
use hara_wasm::extension_tool;
use hara_wasm::identity_tool;
use hara_wasm::package;
use std::env;
use std::io::{self, Read};
use std::path::PathBuf;

#[path = "cli/build.rs"]
mod build;
#[path = "cli/build_check.rs"]
mod build_check;
#[path = "cli/form.rs"]
mod form;
#[path = "cli/metaspec.rs"]
mod metaspec;
#[path = "cli/project.rs"]
mod project;
#[path = "cli/spec.rs"]
mod spec;

#[cfg(feature = "halc-encoder")]
use self::project::compile_halc;
use self::project::{
    check_project, direct_eval, edit_dependency, new_project, run_file, run_headless, run_project,
    run_remote, sync_project, test_project,
};
use self::spec::spec_command;

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
        Some("id") => identity_tool::run(&command[1..]),
        Some("asset") => asset::run(&command[1..]),
        Some("tap") => package::tap_command(&command[1..]),
        Some("package") => package::run(&command[1..]),
        #[cfg(feature = "halc-encoder")]
        Some("compile-halc") => compile_halc(&command[1..]),
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
    if command
        .first()
        .is_some_and(|value| matches!(value.as_str(), "help" | "compile-halc"))
        || command == ["standalone"]
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
        "hara.cli.handler/identity" => "id",
        "hara.cli.handler/asset" => "asset",
        "hara.cli.handler/tap" => "tap",
        _ => return command.to_vec(),
    };
    let mut routed = vec![legacy.to_owned()];
    if resolved.route.id == "hara.cli.route/package-extension" {
        // `package extension` is a grouped spelling of the legacy top-level
        // extension command, not an `extension extension` subcommand.
    } else if matches!(
        resolved.route.handler.as_str(),
        "hara.cli.handler/package"
            | "hara.cli.handler/spec"
            | "hara.cli.handler/extension"
            | "hara.cli.handler/identity"
            | "hara.cli.handler/asset"
            | "hara.cli.handler/tap"
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

fn usage() {
    println!("Hara CLI · Rust runtime");
    println!();
    println!("Usage:");
    println!("  hara [OPTIONS] repl");
    println!("  hara eval EXPRESSION | run FILE | stdin");
    println!("  hara server | remote HOST:PORT");
    println!("  hara project <new|check|run|test|add|remove|sync|update> ...");
    println!("  hara package <COMMAND> ...");
    println!("  hara id <login|enroll|status|key|namespace> ...");
    println!("  hara asset <check|build|inspect|publish|status|search|info|pull|sync|yank> ...");
    println!("  hara tap <bootstrap|init|add|remove|list|verify|mirror> ...");
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
    use super::build::{
        canonical_build_form, canonical_build_from_edn, read_build_source, write_build_surface,
    };
    use super::build_check::{
        build_obligation_report, build_report_status, check_build, check_build_graph,
    };
    use super::form::{keyword, map_form, map_get};
    use super::metaspec::{
        METASPEC_REQUIRED_KEYS, lint_metaspec, metaspec_report, metaspec_template,
        read_spec_document, validate_against_metaspec, verify_metaspec,
    };
    use super::spec::check_contribution;
    use super::{error_exit_code, routed_command};
    use hara_wasm::cli_app;
    use hara_wasm::kernel::{Form, parse};
    use std::fs;
    use std::path::Path;

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
    fn grouped_package_extension_does_not_duplicate_the_command() {
        assert_eq!(
            routed_command(&[
                "package".into(),
                "extension".into(),
                "check".into(),
                "demo".into()
            ]),
            ["extension", "check", "demo"]
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
        assert!(
            findings
                .iter()
                .any(|finding| finding.rule == "hara.metaspec.rule/schema-reference")
        );
        let report = metaspec_report(&document, &findings);
        assert_eq!(map_get(&report, "report/status"), Some(&keyword("fail")));
    }

    #[test]
    fn greenways_buildspec_validates_against_artifact_metaspec() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let metaspec_path =
            repository.join("specs/00-unsorted/artifact/metaspec/artifact-metaspec.edn");
        if !metaspec_path.is_file() {
            eprintln!("skipping: specs submodule not initialized");
            return;
        }
        let document_path =
            repository.join("contrib/greenways/build/spec/draft/greenways-buildspec.edn");
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
        assert!(
            findings
                .iter()
                .any(|finding| finding.kind == "greenways/checker-commit")
        );
        let report = build_obligation_report(&checker_build, &findings);
        assert_eq!(build_report_status(&report), "blocked");
    }

    #[test]
    fn greenways_contribution_envelopes_verify_offline() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        if !repository
            .join("specs/00-unsorted/artifact/metaspec")
            .is_dir()
        {
            eprintln!("skipping: specs submodule not initialized");
            return;
        }
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
