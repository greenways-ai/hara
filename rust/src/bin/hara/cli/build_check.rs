use super::build::{BuildFinding, BuildStage, CanonicalBuild};
use super::form::{keyword, map_form, map_get, string, string_value};
use hara_wasm::kernel::{parse, Form};
use std::collections::{HashMap, HashSet};

pub(crate) fn check_build(build: &CanonicalBuild) -> Vec<BuildFinding> {
    let mut findings = check_build_graph(build);
    findings.extend(check_checker_identities(build));
    findings
}

pub(crate) fn check_build_graph(build: &CanonicalBuild) -> Vec<BuildFinding> {
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

pub(crate) fn valid_github_repository(value: &str) -> bool {
    value
        .split_once('/')
        .is_some_and(|(owner, name)| !owner.is_empty() && !name.is_empty() && !name.contains('/'))
}

pub(crate) fn valid_full_git_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_repository_path(value: &str) -> bool {
    !value.is_empty() && !value.starts_with('/') && !value.split('/').any(|segment| segment == "..")
}

pub(crate) fn has_required_failures(findings: &[BuildFinding]) -> bool {
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

pub(crate) fn build_graph_form(build: &CanonicalBuild, findings: &[BuildFinding]) -> Form {
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

pub(crate) fn build_obligation_report(build: &CanonicalBuild, findings: &[BuildFinding]) -> Form {
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

pub(crate) fn build_report_status(report: &Form) -> &str {
    match map_get(report, "build/status") {
        Some(Form::Keyword(status)) => status,
        _ => "unknown",
    }
}

pub(crate) fn print_build_graph_text(build: &CanonicalBuild, findings: &[BuildFinding]) {
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

pub(crate) fn print_build_check_text(build: &CanonicalBuild, findings: &[BuildFinding]) {
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
