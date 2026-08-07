//! Generic persistent source-analyzer host for `.hal` modules.
//!
//! The host owns runtime concerns that should not be reimplemented by each
//! application: JSONL framing, the spanned Hara reader, whole-Wasm module
//! preparation, source ranges, hashes, structural summaries, and protocol
//! materialization. Analyzer policy remains in the supplied `.hal` module.
//!
//! A module exposes two typed functions:
//!
//! ```clojure
//! (defn ^{:schema [:fn [] :any]} describe [] ...)
//! (defn ^{:schema [:fn [:any] :any]} analyze [reader-tree] ...)
//! ```
//!
//! `describe` returns:
//! `[name version languages extensions capabilities max-message-bytes]`.
//!
//! `analyze` receives the compact reader tree `[nodes roots]` and returns:
//! `[namespace-token imports definitions references]`. The row formats are
//! documented by the Historia analyzer and intentionally contain only generic
//! source-analysis facts; application-specific decisions stay in HAL.

use crate::core::Value;
use crate::kernel::{normalize_schema, read_forms, Form, SchemaType, Span, SpannedForm};
use crate::lang::data::{OrderedMap, Vector};
use crate::vm::FunctionId;
use crate::whole_wasm::NativeModule;
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap};
use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::time::Instant;

const PROTOCOL_VERSION: &str = "1.0";
const ANALYZER_NAMESPACE: &str = "hara.code-analyzer";
const DEFAULT_MAX_MESSAGE_BYTES: usize = 10 * 1024 * 1024;

pub fn run_jsonl(module_path: &Path) -> Result<(), String> {
    let source = fs::read_to_string(module_path)
        .map_err(|error| format!("cannot read {}: {error}", module_path.display()))?;
    let mut analyzer = SourceAnalyzer::compile(&source)?;
    let stdin = io::stdin();
    let mut stdout = io::BufWriter::new(io::stdout().lock());

    for line in stdin.lock().lines() {
        let line = line.map_err(|error| format!("stdin: {error}"))?;
        let request = crate::json::read(&line).unwrap_or_else(|_| unknown_request());
        let shutdown = request_text(&request, "op").is_some_and(|op| op == "shutdown");
        let response = analyzer.handle(&request);
        response.write(&mut stdout)?;
        stdout
            .write_all(b"\n")
            .map_err(|error| format!("stdout: {error}"))?;
        stdout.flush().map_err(|error| format!("stdout: {error}"))?;
        if shutdown {
            break;
        }
    }
    Ok(())
}

pub struct SourceAnalyzer {
    module: NativeModule,
    describe_function: FunctionId,
    analyze_function: FunctionId,
    descriptor: Json,
    fingerprint: String,
    profile: bool,
}

impl SourceAnalyzer {
    pub fn compile(source: &str) -> Result<Self, String> {
        let mut program = crate::vm::compile_source(source).map_err(|error| error.to_string())?;
        program.namespace = Some(ANALYZER_NAMESPACE.to_owned());
        program.function_types = declared_function_types(source)?;

        for function in &program.functions {
            let Some(name) = function.name.as_deref() else {
                continue;
            };
            let local = name.rsplit('/').next().unwrap_or(name);
            let qualified = format!("{ANALYZER_NAMESPACE}/{local}");
            if !program.function_types.contains_key(&qualified) {
                return Err(format!("analyzer function {local} has no ^:schema declaration"));
            }
        }

        let artifact = crate::whole_wasm::compile_artifact(&program)?;
        let mut module = NativeModule::load(&artifact)?;
        let describe_function = find_function(&module, "describe")?;
        let analyze_function = find_function(&module, "analyze")?;
        let descriptor_value = module.call_value(describe_function, &[])?;
        let fingerprint = sha256(
            [
                source.as_bytes(),
                env!("CARGO_PKG_VERSION").as_bytes(),
                b"hara-rust-full:source-analyzer:value-abi-v1",
            ]
            .concat()
            .as_slice(),
        );
        let descriptor = materialize_descriptor(&descriptor_value, &fingerprint)?;
        Ok(Self {
            module,
            describe_function,
            analyze_function,
            descriptor,
            fingerprint,
            profile: env::var_os("HARA_ANALYZER_PROFILE").is_some(),
        })
    }

    pub fn handle(&mut self, request: &Value) -> Json {
        let request_id = request_text(request, "request_id").unwrap_or("unknown");
        let op = request_text(request, "op").unwrap_or("unknown");
        let version = request_text(request, "protocol_version");
        if version != Some(PROTOCOL_VERSION) {
            return error_response(
                request_id,
                op,
                "invalid_request",
                "unsupported protocol version",
            );
        }

        match op {
            "describe" => response(request_id, op, "result", self.descriptor.clone()),
            "ping" | "shutdown" => response(
                request_id,
                op,
                "result",
                Json::object([("ok", Json::Bool(true))]),
            ),
            "analyze" => match self.analyze(request) {
                Ok(result) => response(request_id, op, "result", result),
                Err(failure) => error_response(request_id, op, failure.code, &failure.message),
            },
            _ => error_response(
                request_id,
                op,
                "unsupported_operation",
                "unsupported operation",
            ),
        }
    }

    fn analyze(&mut self, request: &Value) -> Result<Json, AnalyzerFailure> {
        let started = Instant::now();
        let language = required_string(request, "language", false)?;
        if language != "clojure" && language != "babashka" {
            return Err(AnalyzerFailure::new(
                "unsupported_language",
                format!("unsupported language: {language}"),
            ));
        }
        let path = required_string(request, "path", false)?;
        let blob_oid = required_string(request, "blob_oid", false)?;
        let source = required_string(request, "source", true)?;
        let max_message_bytes = descriptor_max_message_bytes(&self.descriptor)
            .unwrap_or(DEFAULT_MAX_MESSAGE_BYTES);
        if source.len() > max_message_bytes {
            return Err(AnalyzerFailure::new(
                "too_large",
                "source exceeds analyzer limit",
            ));
        }

        let forms = read_forms(source)
            .map_err(|error| AnalyzerFailure::new("parse_error", error.to_string()))?;
        let parsed = started.elapsed();
        let tree = EncodedTree::new(source, &forms);
        let indexed = started.elapsed();
        let input = tree.hara_value();
        let output = self
            .module
            .call_value(self.analyze_function, &[input])
            .map_err(|error| AnalyzerFailure::new("internal_error", error))?;
        let executed = started.elapsed();
        let result = materialize(source, language, path, blob_oid, &tree, &output)
            .map_err(|error| AnalyzerFailure::new("internal_error", error))?;
        let completed = started.elapsed();

        if self.profile {
            eprintln!(
                "{{\"path\":{},\"parse_us\":{},\"tree_us\":{},\"wasm_us\":{},\"materialize_us\":{},\"total_us\":{}}}",
                json_string(path),
                parsed.as_micros(),
                indexed.saturating_sub(parsed).as_micros(),
                executed.saturating_sub(indexed).as_micros(),
                completed.saturating_sub(executed).as_micros(),
                completed.as_micros(),
            );
        }
        Ok(result)
    }

    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    pub fn describe_function(&self) -> FunctionId {
        self.describe_function
    }
}

fn find_function(module: &NativeModule, local_name: &str) -> Result<FunctionId, String> {
    module
        .artifact()
        .program
        .functions
        .iter()
        .position(|function| {
            function
                .name
                .as_deref()
                .is_some_and(|name| name.rsplit('/').next() == Some(local_name))
        })
        .map(|index| index as FunctionId)
        .ok_or_else(|| format!("compiled analyzer has no {local_name} function"))
}

fn declared_function_types(source: &str) -> Result<HashMap<String, SchemaType>, String> {
    let forms = read_forms(source).map_err(|error| error.to_string())?;
    let mut declared = HashMap::new();
    for spanned in forms {
        let Form::List(items) = spanned.form else {
            continue;
        };
        if !matches!(items.first(), Some(Form::Symbol(operator)) if operator == "defn" || operator == "defn-") {
            continue;
        }
        let Some((name, metadata)) = items.get(1).and_then(definition_metadata) else {
            continue;
        };
        let schema = metadata
            .iter()
            .find_map(|(key, value)| match key {
                Form::Keyword(name) if name == "schema" => Some(value),
                _ => None,
            })
            .ok_or_else(|| format!("analyzer function {name} has no :schema metadata"))?;
        let normalized = normalize_schema(schema)
            .map_err(|error| format!("invalid schema for analyzer function {name}: {error}"))?;
        declared.insert(format!("{ANALYZER_NAMESPACE}/{name}"), normalized);
    }
    Ok(declared)
}

fn definition_metadata(form: &Form) -> Option<(String, &[(Form, Form)])> {
    let Form::Metadata(metadata, value) = form else {
        return None;
    };
    let Form::Symbol(name) = value.as_ref() else {
        return None;
    };
    let Form::Map(entries) = metadata.as_ref() else {
        return None;
    };
    Some((name.clone(), entries.as_slice()))
}

struct Tokens {
    values: Vec<String>,
    indexes: HashMap<String, i64>,
}

impl Default for Tokens {
    fn default() -> Self {
        let mut tokens = Self {
            values: Vec::new(),
            indexes: HashMap::new(),
        };
        for value in [
            "def", "defonce", "defn", "defn-", "defmacro", "defmulti",
            "defmethod", "defprotocol", "defrecord", "deftype", "deftest", "ns",
            "fn", "fn*", "let", "letfn", "loop", "recur", "if", "if-not",
            "when", "when-not", "cond", "condp", "case", "do", "quote", "var",
            "set!", "try", "catch", "finally", "throw", "new", ".", "..", "doto",
            "locking", "with-open", "binding", "for", "doseq", "dotimes", "comment",
            "require", ":require", "variable", "function", "macro", "multimethod",
            "method", "protocol", "record", "type", "test",
        ] {
            tokens.intern(value.to_owned());
        }
        tokens
    }
}

impl Tokens {
    fn intern(&mut self, value: String) -> i64 {
        if let Some(index) = self.indexes.get(&value) {
            return *index;
        }
        let index = self.values.len() as i64;
        self.values.push(value.clone());
        self.indexes.insert(value, index);
        index
    }

    fn get(&self, index: i64) -> Result<&str, String> {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.values.get(index))
            .map(String::as_str)
            .ok_or_else(|| format!("unknown token index {index}"))
    }
}

struct HostNode {
    span: Span,
    shape_code: i64,
    token: i64,
    children: Vec<usize>,
}

struct EncodedTree {
    nodes: Vec<HostNode>,
    roots: Vec<usize>,
    tokens: Tokens,
    positions: SourceIndex,
}

impl EncodedTree {
    fn new(source: &str, forms: &[SpannedForm]) -> Self {
        let mut tree = Self {
            nodes: Vec::new(),
            roots: Vec::new(),
            tokens: Tokens::default(),
            positions: SourceIndex::new(source),
        };
        for form in forms {
            let root = tree.push(source, form);
            tree.roots.push(root);
        }
        tree
    }

    fn push(&mut self, source: &str, value: &SpannedForm) -> usize {
        let children = value
            .children
            .iter()
            .map(|child| self.push(source, child))
            .collect::<Vec<_>>();
        let token = token_text(&value.form)
            .map(|token| self.tokens.intern(token))
            .unwrap_or(-1);
        let index = self.nodes.len();
        self.nodes.push(HostNode {
            span: value.span.clone(),
            shape_code: shape_code(source, value),
            token,
            children,
        });
        index
    }

    fn hara_value(&self) -> Value {
        let nodes = value_vector(self.nodes.iter().map(HostNode::hara_value));
        let roots = value_vector(
            self.roots
                .iter()
                .map(|index| Value::Number(*index as i64)),
        );
        value_vector([nodes, roots])
    }
}

impl HostNode {
    fn hara_value(&self) -> Value {
        value_vector([
            Value::Number(self.shape_code),
            Value::Number(self.token),
            value_vector(
                self.children
                    .iter()
                    .map(|index| Value::Number(*index as i64)),
            ),
        ])
    }
}

fn value_vector(values: impl IntoIterator<Item = Value>) -> Value {
    Value::Vector(Vector::from_iter(values))
}

fn shape_code(source: &str, value: &SpannedForm) -> i64 {
    match &value.form {
        Form::Keyword(_) => 1,
        Form::String(_) => 2,
        Form::Number(_) | Form::Float(_) | Form::BigInteger(_) | Form::Decimal(_) => 3,
        Form::Nil | Form::Bool(_) => 4,
        Form::Vector(_) => 11,
        Form::Map(_) => 12,
        Form::Set(_) => 13,
        Form::List(_) => synthetic_prefix(source, value).unwrap_or(10),
        _ => 0,
    }
}

fn synthetic_prefix(source: &str, value: &SpannedForm) -> Option<i64> {
    if value.children.len() != 1 {
        return None;
    }
    let slice = source.get(value.span.start.offset..value.span.end.offset)?;
    if slice.starts_with("~@") {
        Some(19)
    } else if slice.starts_with('~') {
        Some(18)
    } else if slice.starts_with('`') {
        Some(17)
    } else if slice.starts_with('\'') {
        Some(16)
    } else if slice.starts_with('@') {
        Some(15)
    } else {
        None
    }
}

fn token_text(form: &Form) -> Option<String> {
    match form {
        Form::Metadata(_, value) => token_text(value),
        Form::Symbol(value) => Some(value.clone()),
        Form::Keyword(value) => Some(format!(":{value}")),
        Form::String(value) => Some(value.clone()),
        Form::Character(value) => Some(value.to_string()),
        _ => None,
    }
}

fn materialize_descriptor(value: &Value, fingerprint: &str) -> Result<Json, String> {
    let values = vector_values(value)?;
    if values.len() != 6 {
        return Err(format!(
            "analyzer describe returned {} fields, expected 6",
            values.len()
        ));
    }
    Ok(Json::object([
        ("name", Json::String(value_string(values[0])?.to_owned())),
        ("version", Json::String(value_string(values[1])?.to_owned())),
        (
            "protocol_versions",
            Json::Array(vec![Json::String(PROTOCOL_VERSION.to_owned())]),
        ),
        ("languages", string_array(values[2])?),
        ("extensions", string_array(values[3])?),
        ("capabilities", string_array(values[4])?),
        (
            "max_message_bytes",
            Json::Integer(value_number(values[5])?),
        ),
        ("fingerprint", Json::String(fingerprint.to_owned())),
    ]))
}

fn descriptor_max_message_bytes(descriptor: &Json) -> Option<usize> {
    let Json::Object(entries) = descriptor else {
        return None;
    };
    entries.iter().find_map(|(key, value)| {
        if key == "max_message_bytes" {
            match value {
                Json::Integer(value) => usize::try_from(*value).ok(),
                _ => None,
            }
        } else {
            None
        }
    })
}

fn string_array(value: &Value) -> Result<Json, String> {
    vector_values(value)?
        .into_iter()
        .map(|value| value_string(value).map(|value| Json::String(value.to_owned())))
        .collect::<Result<Vec<_>, _>>()
        .map(Json::Array)
}

fn materialize(
    source: &str,
    language: &str,
    path: &str,
    blob_oid: &str,
    tree: &EncodedTree,
    output: &Value,
) -> Result<Json, String> {
    let output = vector_values(output)?;
    if output.len() != 4 {
        return Err(format!(
            "analyzer returned {} fields, expected 4",
            output.len()
        ));
    }
    let namespace_index = value_number(output[0])?;
    let namespace = if namespace_index < 0 {
        None
    } else {
        Some(tree.tokens.get(namespace_index)?.to_owned())
    };

    let imports = vector_values(output[1])?
        .into_iter()
        .map(|value| {
            let node = node(&tree.nodes, value_number(value)?)?;
            Ok(Json::String(source_slice(source, &node.span)?.to_owned()))
        })
        .collect::<Result<Vec<_>, String>>()?;

    let definitions = vector_values(output[2])?;
    let mut symbols = Vec::with_capacity(definitions.len());
    for (index, definition) in definitions.into_iter().enumerate() {
        let values = vector_values(definition)?;
        if values.len() != 7 {
            return Err(format!("definition {index} has {} fields", values.len()));
        }
        let node_id = value_number(values[0])?;
        let head = tree.tokens.get(value_number(values[1])?)?.to_owned();
        let name = tree.tokens.get(value_number(values[2])?)?.to_owned();
        let signature_id = value_number(values[3])?;
        let kind = tree.tokens.get(value_number(values[4])?)?.to_owned();
        let private = value_number(values[5])? == 1;
        let definition_node = node(&tree.nodes, node_id)?;
        let snippet = source_slice(source, &definition_node.span)?;
        let selection_span = definition_node
            .children
            .get(1)
            .and_then(|node_id| tree.nodes.get(*node_id))
            .map(|node| &node.span)
            .unwrap_or(&definition_node.span);

        let features = structural_features(values[6], &tree.tokens)?;
        let structural_hash = features
            .object_field("shape_hash")
            .and_then(Json::as_str)
            .ok_or("structural features have no shape_hash")?
            .to_owned();
        let signature = if signature_id < 0 {
            Json::Null
        } else {
            let signature_node = node(&tree.nodes, signature_id)?;
            Json::String(source_slice(source, &signature_node.span)?.to_owned())
        };
        let qualified_name = namespace
            .as_ref()
            .map(|namespace| format!("{namespace}/{name}"))
            .unwrap_or_else(|| name.clone());

        symbols.push(Json::object([
            ("local_id", Json::String(format!("symbol-{index}"))),
            ("kind", Json::String(kind)),
            ("name", Json::String(name.clone())),
            ("qualified_name", Json::String(qualified_name)),
            ("range", tree.positions.range(source, &definition_node.span)),
            ("selection_range", tree.positions.range(source, selection_span)),
            ("signature", signature),
            (
                "modifiers",
                Json::Array(if private {
                    vec![Json::String("private".to_owned())]
                } else {
                    Vec::new()
                }),
            ),
            ("source_hash", Json::String(sha256(snippet.as_bytes()))),
            ("structural_hash", Json::String(structural_hash)),
            ("structural_features", features),
            (
                "structure",
                Json::object([
                    ("head", Json::String(head)),
                    ("normalized", Json::String(normalize_form(snippet))),
                ]),
            ),
        ]));
    }

    let mut references = vector_values(output[3])?
        .into_iter()
        .map(|reference| {
            let values = vector_values(reference)?;
            if values.len() != 2 {
                return Err(format!("reference has {} fields", values.len()));
            }
            let definition_index = value_number(values[0])?;
            let target = tree.tokens.get(value_number(values[1])?)?.to_owned();
            let candidate = target.contains('/');
            Ok(Json::object([
                ("kind", Json::String("call".to_owned())),
                ("range", tree.positions.zero_range(source)),
                (
                    "source_symbol_local_id",
                    Json::String(format!("symbol-{definition_index}")),
                ),
                ("target_text", Json::String(target)),
                (
                    "resolution",
                    Json::String(if candidate { "candidate" } else { "unresolved" }.to_owned()),
                ),
                ("confidence", Json::Float(if candidate { 0.7 } else { 0.3 })),
            ]))
        })
        .collect::<Result<Vec<_>, String>>()?;
    references.sort_by(|left, right| {
        let left_key = (
            left.object_field("source_symbol_local_id")
                .and_then(Json::as_str)
                .unwrap_or(""),
            left.object_field("target_text")
                .and_then(Json::as_str)
                .unwrap_or(""),
        );
        let right_key = (
            right
                .object_field("source_symbol_local_id")
                .and_then(Json::as_str)
                .unwrap_or(""),
            right
                .object_field("target_text")
                .and_then(Json::as_str)
                .unwrap_or(""),
        );
        left_key.cmp(&right_key)
    });

    Ok(Json::object([
        (
            "file",
            Json::object([
                ("language", Json::String(language.to_owned())),
                ("path", Json::String(path.to_owned())),
                ("blob_oid", Json::String(blob_oid.to_owned())),
                (
                    "namespace",
                    namespace.map(Json::String).unwrap_or(Json::Null),
                ),
                ("imports", Json::Array(imports)),
                ("source_bytes", Json::Integer(source.len() as i64)),
            ]),
        ),
        ("symbols", Json::Array(symbols)),
        ("references", Json::Array(references)),
        ("diagnostics", Json::Array(Vec::new())),
    ]))
}

fn node(nodes: &[HostNode], index: i64) -> Result<&HostNode, String> {
    usize::try_from(index)
        .ok()
        .and_then(|index| nodes.get(index))
        .ok_or_else(|| format!("unknown node index {index}"))
}

fn source_slice<'a>(source: &'a str, span: &Span) -> Result<&'a str, String> {
    source
        .get(span.start.offset..span.end.offset)
        .ok_or_else(|| "source span is outside source".to_owned())
}

fn vector_values(value: &Value) -> Result<Vec<&Value>, String> {
    match value {
        Value::Vector(values) => Ok(values.iter().collect()),
        other => Err(format!("expected encoded vector, got {}", other.display())),
    }
}

fn value_number(value: &Value) -> Result<i64, String> {
    match value {
        Value::Number(value) => Ok(*value),
        other => Err(format!("expected encoded integer, got {}", other.display())),
    }
}

fn value_string(value: &Value) -> Result<&str, String> {
    match value {
        Value::String(value) => Ok(value),
        other => Err(format!("expected string, got {}", other.display())),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Shape {
    Vector(Vec<Shape>),
    Keyword(String),
    String(String),
    Number(i64),
    Nil,
}

fn structural_features(encoded: &Value, tokens: &Tokens) -> Result<Json, String> {
    let shape = decode_shape(encoded, tokens)?;
    let summary = summarize_shape(&shape);
    Ok(Json::object([
        ("shape", Json::String(summary.rendered.clone())),
        ("shape_hash", Json::String(sha256(summary.rendered.as_bytes()))),
        ("node_count", Json::Integer(summary.node_count as i64)),
        ("depth", Json::Integer(summary.depth as i64)),
        ("arity", Json::Integer(summary.arity as i64)),
        (
            "features",
            Json::Array(summary.features.into_iter().map(Json::String).collect()),
        ),
    ]))
}

fn decode_shape(value: &Value, tokens: &Tokens) -> Result<Shape, String> {
    match value {
        Value::Nil => Ok(Shape::Nil),
        Value::Number(value) => Ok(Shape::Number(*value)),
        Value::Vector(values) => {
            let values = values.iter().collect::<Vec<_>>();
            if let Some(Value::Number(tag)) = values.first().copied() {
                decode_tagged(*tag, &values[1..], tokens)
            } else {
                values
                    .into_iter()
                    .map(|value| decode_shape(value, tokens))
                    .collect::<Result<Vec<_>, _>>()
                    .map(Shape::Vector)
            }
        }
        other => Err(format!("invalid encoded structural shape: {}", other.display())),
    }
}

fn decode_tagged(tag: i64, values: &[&Value], tokens: &Tokens) -> Result<Shape, String> {
    let keyword = |name: &str| Shape::Keyword(name.to_owned());
    let vector = |values: Vec<Shape>| Shape::Vector(values);
    let one_child = |name: &str| -> Result<Shape, String> {
        let child = values
            .first()
            .ok_or_else(|| format!("shape tag {tag} has no child"))?;
        Ok(vector(vec![keyword(name), decode_shape(child, tokens)?]))
    };
    let token = || -> Result<String, String> {
        let index = value_number(
            values
                .first()
                .ok_or_else(|| format!("shape tag {tag} has no token"))?,
        )?;
        Ok(tokens.get(index)?.to_owned())
    };

    match tag {
        100 => Ok(vector(vec![keyword("special"), Shape::String(token()?)])),
        101 => Ok(vector(vec![keyword("call")])),
        102 => decode_collection("vector", values, tokens),
        103 => decode_collection("map", values, tokens),
        104 => decode_collection("set", values, tokens),
        105 => decode_collection("namespaced-map", values, tokens),
        106 => one_child("deref"),
        107 => one_child("quote"),
        108 => one_child("syntax-quote"),
        109 => one_child("unquote"),
        110 => one_child("unquote-splicing"),
        111 => Ok(vector(vec![keyword("keyword"), Shape::String(token()?)])),
        112 => Ok(vector(vec![keyword("string")])),
        113 => Ok(vector(vec![keyword("number")])),
        114 => Ok(vector(vec![keyword("literal")])),
        115 => Ok(vector(vec![keyword("symbol")])),
        _ => Err(format!("unknown structural shape tag {tag}")),
    }
}

fn decode_collection(name: &str, values: &[&Value], tokens: &Tokens) -> Result<Shape, String> {
    let mut decoded = vec![Shape::Keyword(name.to_owned())];
    for value in values {
        decoded.push(decode_shape(value, tokens)?);
    }
    Ok(Shape::Vector(decoded))
}

struct ShapeSummary {
    rendered: String,
    node_count: usize,
    depth: usize,
    arity: usize,
    features: BTreeSet<String>,
}

fn summarize_shape(shape: &Shape) -> ShapeSummary {
    match shape {
        Shape::Vector(values) => {
            let child_summaries = values
                .iter()
                .skip(1)
                .map(summarize_shape)
                .collect::<Vec<_>>();
            let rendered = format!(
                "[{}]",
                values.iter().map(render_shape).collect::<Vec<_>>().join(" ")
            );
            let mut features = BTreeSet::from([rendered.clone()]);
            for child in &child_summaries {
                features.extend(child.features.iter().cloned());
            }
            let call_arity = if matches!(values.first(), Some(Shape::Keyword(value)) if value == "call") {
                values.len().saturating_sub(1)
            } else {
                0
            };
            ShapeSummary {
                rendered,
                node_count: 1 + child_summaries.iter().map(|child| child.node_count).sum::<usize>(),
                depth: 1 + child_summaries.iter().map(|child| child.depth).max().unwrap_or(0),
                arity: child_summaries
                    .iter()
                    .map(|child| child.arity)
                    .max()
                    .unwrap_or(0)
                    .max(call_arity),
                features,
            }
        }
        _ => {
            let rendered = render_shape(shape);
            ShapeSummary {
                features: BTreeSet::from([rendered.clone()]),
                rendered,
                node_count: 1,
                depth: 1,
                arity: 0,
            }
        }
    }
}

fn render_shape(value: &Shape) -> String {
    match value {
        Shape::Vector(values) => format!(
            "[{}]",
            values.iter().map(render_shape).collect::<Vec<_>>().join(" ")
        ),
        Shape::Keyword(value) => format!(":{value}"),
        Shape::String(value) => clojure_string(value),
        Shape::Number(value) => value.to_string(),
        Shape::Nil => "nil".to_owned(),
    }
}

fn clojure_string(value: &str) -> String {
    let mut output = String::from("\"");
    for character in value.chars() {
        match character {
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            '\u{0008}' => output.push_str("\\b"),
            '\u{000c}' => output.push_str("\\f"),
            '\\' => output.push_str("\\\\"),
            '"' => output.push_str("\\\""),
            control if control.is_control() => {
                output.push_str(&format!("\\u{:04X}", control as u32));
            }
            character => output.push(character),
        }
    }
    output.push('"');
    output
}

struct SourceIndex {
    line_starts: Vec<usize>,
}

impl SourceIndex {
    fn new(source: &str) -> Self {
        let mut line_starts = vec![0];
        for (offset, byte) in source.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push(offset + 1);
            }
        }
        Self { line_starts }
    }

    fn position(&self, source: &str, offset: usize) -> Json {
        let offset = offset.min(source.len());
        let line_index = self.line_starts.partition_point(|start| *start <= offset) - 1;
        let line_start = self.line_starts[line_index];
        let column = source
            .get(line_start..offset)
            .unwrap_or_default()
            .encode_utf16()
            .count()
            + 1;
        Json::object([
            ("line", Json::Integer((line_index + 1) as i64)),
            ("column", Json::Integer(column as i64)),
        ])
    }

    fn range(&self, source: &str, span: &Span) -> Json {
        self.offset_range(source, span.start.offset, span.end.offset)
    }

    fn zero_range(&self, source: &str) -> Json {
        self.offset_range(source, 0, 0)
    }

    fn offset_range(&self, source: &str, start: usize, end: usize) -> Json {
        Json::object([
            ("start_byte", Json::Integer(start as i64)),
            ("end_byte", Json::Integer(end as i64)),
            ("start", self.position(source, start)),
            ("end", self.position(source, end)),
        ])
    }
}

fn normalize_form(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let mut in_string = false;
    let mut escaped = false;
    let mut in_comment = false;
    let mut whitespace = false;

    for character in source.chars() {
        if in_comment {
            if character == '\n' {
                in_comment = false;
                whitespace = true;
            }
            continue;
        }
        if in_string {
            output.push(character);
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }
        match character {
            ';' => {
                in_comment = true;
                whitespace = true;
            }
            '"' => {
                if whitespace && !output.is_empty() {
                    output.push(' ');
                }
                whitespace = false;
                in_string = true;
                output.push(character);
            }
            character if character.is_whitespace() => whitespace = true,
            character => {
                if whitespace && !output.is_empty() {
                    output.push(' ');
                }
                whitespace = false;
                output.push(character);
            }
        }
    }
    output.trim().to_owned()
}

fn required_string<'a>(
    request: &'a Value,
    key: &str,
    allow_empty: bool,
) -> Result<&'a str, AnalyzerFailure> {
    let value = request_text(request, key).ok_or_else(|| {
        AnalyzerFailure::new(
            "invalid_request",
            format!("missing or invalid field: {key}"),
        )
    })?;
    if !allow_empty && value.trim().is_empty() {
        return Err(AnalyzerFailure::new(
            "invalid_request",
            format!("missing or invalid field: {key}"),
        ));
    }
    Ok(value)
}

fn request_text<'a>(request: &'a Value, key: &str) -> Option<&'a str> {
    let Value::OrderedMap(entries) = request else {
        return None;
    };
    entries.iter().find_map(|(candidate, value)| match (candidate, value) {
        (Value::String(candidate), Value::String(value)) if candidate == key => Some(value.as_str()),
        _ => None,
    })
}

fn unknown_request() -> Value {
    Value::OrderedMap(Box::new(OrderedMap::from_iter([
        (Value::String("request_id".into()), Value::String("unknown".into())),
        (Value::String("op".into()), Value::String("unknown".into())),
    ])))
}

#[derive(Debug)]
struct AnalyzerFailure {
    code: &'static str,
    message: String,
}

impl AnalyzerFailure {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug)]
enum Json {
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    fn object(entries: impl IntoIterator<Item = (&'static str, Json)>) -> Self {
        Self::Object(
            entries
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value))
                .collect(),
        )
    }

    fn object_field(&self, key: &str) -> Option<&Json> {
        let Self::Object(entries) = self else {
            return None;
        };
        entries
            .iter()
            .find_map(|(candidate, value)| (candidate == key).then_some(value))
    }

    fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    fn write(&self, output: &mut impl Write) -> Result<(), String> {
        match self {
            Self::Null => output.write_all(b"null"),
            Self::Bool(value) => output.write_all(if *value { b"true" } else { b"false" }),
            Self::Integer(value) => write!(output, "{value}"),
            Self::Float(value) if value.is_finite() => write!(output, "{value}"),
            Self::Float(_) => return Err("JSON cannot encode non-finite numbers".into()),
            Self::String(value) => output.write_all(json_string(value).as_bytes()),
            Self::Array(values) => {
                output.write_all(b"[")?;
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        output.write_all(b",")?;
                    }
                    value.write(output)?;
                }
                output.write_all(b"]")
            }
            Self::Object(entries) => {
                output.write_all(b"{")?;
                for (index, (key, value)) in entries.iter().enumerate() {
                    if index > 0 {
                        output.write_all(b",")?;
                    }
                    output.write_all(json_string(key).as_bytes())?;
                    output.write_all(b":")?;
                    value.write(output)?;
                }
                output.write_all(b"}")
            }
        }
        .map_err(|error| format!("stdout: {error}"))
    }
}

fn response(request_id: &str, op: &str, key: &'static str, body: Json) -> Json {
    Json::object([
        ("protocol_version", Json::String(PROTOCOL_VERSION.to_owned())),
        ("request_id", Json::String(request_id.to_owned())),
        ("op", Json::String(op.to_owned())),
        (key, body),
    ])
}

fn error_response(request_id: &str, op: &str, code: &str, message: &str) -> Json {
    response(
        request_id,
        op,
        "error",
        Json::object([
            ("code", Json::String(code.to_owned())),
            ("message", Json::String(message.to_owned())),
        ]),
    )
}

fn json_string(value: &str) -> String {
    let mut output = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{08}' => output.push_str("\\b"),
            '\u{0c}' => output.push_str("\\f"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character <= '\u{1f}' => {
                output.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => output.push(character),
        }
    }
    output.push('"');
    output
}

fn sha256(value: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(value);
    format!("{:x}", digest.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_ANALYZER: &str = r#"
      (defn ^{:schema [:fn [] :any]}
        describe
        []
        ["test-hara-analyzer" "0.1.0" ["clojure"] [".clj"] ["symbols"] 1048576])

      (defn ^{:schema [:fn [:any] :any]}
        analyze
        [tree]
        [-1 [] [] []])

      0
    "#;

    #[test]
    fn direct_value_worker_compiles_and_materializes() {
        let mut analyzer = SourceAnalyzer::compile(TEST_ANALYZER).expect("compile analyzer");
        let request = crate::json::read(
            r#"{"protocol_version":"1.0","request_id":"x","op":"analyze","language":"clojure","path":"x.clj","blob_oid":"abc","source":"(def x 1)"}"#,
        )
        .unwrap();
        let response = analyzer.handle(&request);
        assert_eq!(
            response
                .object_field("result")
                .and_then(|result| result.object_field("file"))
                .and_then(|file| file.object_field("path"))
                .and_then(Json::as_str),
            Some("x.clj")
        );
    }

    #[test]
    fn source_index_uses_utf16_display_columns() {
        let index = SourceIndex::new("😀x");
        assert!(matches!(
            index.position("😀x", 4).object_field("column"),
            Some(Json::Integer(3))
        ));
    }

    #[test]
    fn normalization_preserves_semicolons_inside_strings() {
        assert_eq!(
            normalize_form("(defn x ; comment\n [] \"semi;colon\")"),
            "(defn x [] \"semi;colon\")"
        );
    }

    #[test]
    fn collection_nodes_do_not_render_subtrees_during_indexing() {
        assert_eq!(token_text(&Form::List(vec![])), None);
        assert_eq!(token_text(&Form::Symbol("x".into())), Some("x".into()));
    }
}
