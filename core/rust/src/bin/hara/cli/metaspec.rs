use super::form::{keyword, map_entries, map_form, map_get, qualified_keyword, string, walk_form};
use super::spec::{finding, spec_finding_form, SpecFinding};
use hara_wasm::kernel::{parse, read_forms, Form};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

pub(crate) const METASPEC_REQUIRED_KEYS: &[&str] = &[
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

#[derive(Clone, Copy)]
pub(crate) enum SpecFormat {
    Text,
    Edn,
}

pub(crate) fn spec_format(args: &[String]) -> Result<SpecFormat, String> {
    match args {
        [] => Ok(SpecFormat::Text),
        [flag, value] if flag == "--format" && value == "text" => Ok(SpecFormat::Text),
        [flag, value] if flag == "--format" && value == "edn" => Ok(SpecFormat::Edn),
        _ => Err("spec format must be --format text or --format edn".into()),
    }
}

pub(crate) fn read_spec_document(source: &str) -> Result<Form, String> {
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

pub(crate) fn lint_metaspec(document: &Form) -> Vec<SpecFinding> {
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

pub(crate) fn verify_metaspec(document: &Form, path: &Path) -> Vec<SpecFinding> {
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

pub(crate) fn validate_against_metaspec(
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

pub(crate) fn metaspec_report(document: &Form, findings: &[SpecFinding]) -> Form {
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

pub(crate) fn print_metaspec_text(document: &Form, findings: &[SpecFinding]) {
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

pub(crate) fn metaspec_template() -> Form {
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
