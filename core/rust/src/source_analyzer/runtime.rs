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
        analyzer.handle(&request).write(&mut stdout)?;
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

struct SourceAnalyzer {
    module: NativeModule,
    analyze_function: FunctionId,
    descriptor: Json,
    profile: bool,
}

impl SourceAnalyzer {
    fn compile(source: &str) -> Result<Self, String> {
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
            analyze_function,
            descriptor,
            profile: env::var_os("HARA_ANALYZER_PROFILE").is_some(),
        })
    }

    fn handle(&mut self, request: &Value) -> Json {
        let request_id = request_text(request, "request_id").unwrap_or("unknown");
        let op = request_text(request, "op").unwrap_or("unknown");
        if request_text(request, "protocol_version") != Some(PROTOCOL_VERSION) {
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
