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
            "require", ":require",
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
    let mut string_escape = false;
    let mut literal_escape = false;
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
            if string_escape {
                string_escape = false;
            } else if character == '\\' {
                string_escape = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }
        if literal_escape {
            output.push(character);
            literal_escape = false;
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
            '\\' => {
                if whitespace && !output.is_empty() {
                    output.push(' ');
                }
                whitespace = false;
                literal_escape = true;
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

fn sha256(value: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(value);
    format!("{:x}", digest.finalize())
}
