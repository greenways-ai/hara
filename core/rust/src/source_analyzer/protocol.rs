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
        let kind = definition_kind(value_number(values[4])?)?;
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
            ("kind", Json::String(kind.to_owned())),
            ("name", Json::String(name)),
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

fn definition_kind(kind: i64) -> Result<&'static str, String> {
    match kind {
        1 => Ok("variable"),
        2 => Ok("function"),
        3 => Ok("macro"),
        4 => Ok("multimethod"),
        5 => Ok("method"),
        6 => Ok("protocol"),
        7 => Ok("record"),
        8 => Ok("type"),
        9 => Ok("test"),
        _ => Err(format!("unknown definition kind {kind}")),
    }
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
        (
            Value::String("request_id".into()),
            Value::String("unknown".into()),
        ),
        (
            Value::String("op".into()),
            Value::String("unknown".into()),
        ),
    ])))
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
        let mut encoded = String::new();
        self.encode(&mut encoded)?;
        output
            .write_all(encoded.as_bytes())
            .map_err(|error| format!("stdout: {error}"))
    }

    fn encode(&self, output: &mut String) -> Result<(), String> {
        match self {
            Self::Null => output.push_str("null"),
            Self::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
            Self::Integer(value) => output.push_str(&value.to_string()),
            Self::Float(value) if value.is_finite() => output.push_str(&value.to_string()),
            Self::Float(_) => return Err("JSON cannot encode non-finite numbers".into()),
            Self::String(value) => json_string_into(output, value),
            Self::Array(values) => {
                output.push('[');
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        output.push(',');
                    }
                    value.encode(output)?;
                }
                output.push(']');
            }
            Self::Object(entries) => {
                output.push('{');
                for (index, (key, value)) in entries.iter().enumerate() {
                    if index > 0 {
                        output.push(',');
                    }
                    json_string_into(output, key);
                    output.push(':');
                    value.encode(output)?;
                }
                output.push('}');
            }
        }
        Ok(())
    }
}

fn response(request_id: &str, op: &str, key: &'static str, body: Json) -> Json {
    Json::object([
        (
            "protocol_version",
            Json::String(PROTOCOL_VERSION.to_owned()),
        ),
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
    let mut output = String::new();
    json_string_into(&mut output, value);
    output
}

fn json_string_into(output: &mut String, value: &str) {
    output.push('"');
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
}
