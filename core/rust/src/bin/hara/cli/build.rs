use super::build_check::{
    build_graph_form, build_obligation_report, build_report_status, check_build, check_build_graph,
    has_required_failures, print_build_check_text, print_build_graph_text,
};
use super::exit_error;
use super::form::{keyword, keyword_name, map_form, map_get, string, string_value};
use super::metaspec::{read_spec_document, spec_format, SpecFormat};
use hara_wasm::kernel::{read_forms, Form, SpannedForm};
use std::collections::HashMap;
use std::fs;

#[derive(Clone, Debug)]
pub(crate) struct CanonicalBuild {
    pub(crate) file: String,
    pub(crate) id: String,
    pub(crate) artifact_kind: String,
    pub(crate) artifact_output: String,
    pub(crate) specs: Vec<(String, String)>,
    pub(crate) stages: Vec<BuildStage>,
}

#[derive(Clone, Debug)]
pub(crate) struct BuildStage {
    pub(crate) id: String,
    pub(crate) requires: Vec<String>,
    pub(crate) produces: String,
    pub(crate) checkers: Vec<Form>,
    pub(crate) row: usize,
    pub(crate) col: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct BuildFinding {
    pub(crate) kind: &'static str,
    pub(crate) level: &'static str,
    pub(crate) message: String,
    pub(crate) stage: Option<String>,
    pub(crate) row: Option<usize>,
    pub(crate) col: Option<usize>,
    pub(crate) details: Vec<(&'static str, Form)>,
}

pub(crate) fn build_spec_command(operation: &str, args: &[String]) -> Result<(), String> {
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

pub(crate) fn read_build_source(
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

pub(crate) fn canonical_build_form(build: &CanonicalBuild) -> Form {
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

pub(crate) fn canonical_build_from_edn(
    data: &Form,
) -> Result<(CanonicalBuild, Vec<BuildFinding>), String> {
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

pub(crate) fn write_build_surface(build: &CanonicalBuild) -> String {
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
