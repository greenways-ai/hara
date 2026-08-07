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
        (
            "shape_hash",
            Json::String(sha256(summary.rendered.as_bytes())),
        ),
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
        other => Err(format!(
            "invalid encoded structural shape: {}",
            other.display()
        )),
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
            let mut rendered_parts = Vec::with_capacity(values.len());
            if let Some(tag) = values.first() {
                rendered_parts.push(render_atom(tag));
            }
            rendered_parts.extend(
                child_summaries
                    .iter()
                    .map(|child| child.rendered.clone()),
            );
            let rendered = format!("[{}]", rendered_parts.join(" "));
            let mut features = BTreeSet::from([rendered.clone()]);
            for child in &child_summaries {
                features.extend(child.features.iter().cloned());
            }
            let call_arity = if matches!(
                values.first(),
                Some(Shape::Keyword(value)) if value == "call"
            ) {
                values.len().saturating_sub(1)
            } else {
                0
            };
            ShapeSummary {
                rendered,
                node_count: 1
                    + child_summaries
                        .iter()
                        .map(|child| child.node_count)
                        .sum::<usize>(),
                depth: 1
                    + child_summaries
                        .iter()
                        .map(|child| child.depth)
                        .max()
                        .unwrap_or(0),
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
            let rendered = render_atom(shape);
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

fn render_atom(value: &Shape) -> String {
    match value {
        Shape::Vector(_) => unreachable!("shape tags are scalar"),
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
