use std::collections::{HashMap, HashSet};

use super::Form;

const LIBRARIES: &[(&str, &str, &str)] = &[
    ("string", "std.foundation.string", "str"),
    ("promise", "std.foundation.promise", "promise"),
    ("bytes", "std.foundation.bytes", "bytes"),
    ("socket", "std.foundation.socket", "socket"),
    ("file", "std.foundation.file", "file"),
    ("coroutine", "std.foundation.coroutine", "co"),
    ("edn", "std.foundation.edn", "edn"),
];

#[derive(Debug, Clone, Default)]
pub struct GeneratedNamespaceConfig {
    aliases: HashMap<String, String>,
    refers: HashMap<String, String>,
    required_namespaces: Vec<String>,
    used_namespaces: Vec<String>,
    builtins: Vec<String>,
    blank: bool,
}

impl GeneratedNamespaceConfig {
    pub fn defaults() -> Self {
        let mut aliases: HashMap<String, String> = LIBRARIES
            .iter()
            .map(|(_, namespace, alias)| ((*alias).into(), (*namespace).into()))
            .collect();
        // Allow both the spec alias (coroutine) and the user-facing short alias (co).
        aliases.insert("coroutine".into(), "std.foundation.coroutine".into());
        Self {
            aliases,
            refers: HashMap::new(),
            required_namespaces: Vec::new(),
            used_namespaces: Vec::new(),
            builtins: Vec::new(),
            blank: false,
        }
    }

    pub fn configure(clauses: &[Form]) -> Result<Self, String> {
        Self::configure_with(clauses, known_namespace)
    }

    pub fn configure_with(
        clauses: &[Form],
        available: impl Fn(&str) -> bool,
    ) -> Result<Self, String> {
        let mut excluded = HashSet::new();
        let mut overrides = HashMap::new();
        let mut requires = Vec::new();
        let mut uses = Vec::new();
        let mut builtins = Vec::new();
        let mut blank = false;
        let mut intrinsics_seen = false;
        let mut config_seen = false;

        for clause in clauses {
            let values = list(clause, "ns clauses must be non-empty lists")?;
            let head = values.first().ok_or("ns clauses must be non-empty lists")?;
            let name = keyword(head, "ns clause must start with a keyword")?;
            match name {
                "config" => {
                    if config_seen {
                        return Err("ns accepts only one :config clause".into());
                    }
                    config_seen = true;
                    if values.len() != 2 {
                        return Err(":config expects one map".into());
                    }
                    parse_config(
                        &values[1],
                        &mut builtins,
                        &mut blank,
                        &mut excluded,
                        &mut overrides,
                    )?;
                }
                "intrinsics" => {
                    // Legacy top-level :intrinsics is accepted for backward compatibility,
                    // but the spec places it inside :config.
                    if intrinsics_seen {
                        return Err("ns accepts only one :intrinsics clause".into());
                    }
                    intrinsics_seen = true;
                    if values.len() != 2 {
                        return Err(":intrinsics expects :all or an options map".into());
                    }
                    if !matches!(&values[1], Form::Keyword(name) if name == "all") {
                        parse_intrinsics(&values[1], &mut excluded, &mut overrides)?;
                    }
                }
                "require" => requires.extend(values[1..].iter().cloned()),
                "use" => uses.extend(values[1..].iter().cloned()),
                "flavor" | "import" => {}
                other => return Err(format!("Unsupported ns clause: :{other}")),
            }
        }

        for library in overrides.keys() {
            if excluded.contains(library) {
                return Err(format!(
                    "Intrinsic library cannot be both excluded and aliased: {library}"
                ));
            }
        }

        let mut config = Self::default();
        config.builtins = builtins;
        config.blank = blank;
        for (library, namespace, default_alias) in LIBRARIES {
            if excluded.contains(*library) {
                continue;
            }
            let alias = overrides
                .get(*library)
                .map_or(*default_alias, String::as_str);
            config.put_alias(alias, namespace)?;
        }
        for require in requires {
            config.apply_require(&require, &available)?;
        }
        for use_form in uses {
            config.apply_use(&use_form, &available)?;
        }
        Ok(config)
    }

    pub fn required_namespaces(&self) -> &[String] {
        &self.required_namespaces
    }

    pub fn used_namespaces(&self) -> &[String] {
        &self.used_namespaces
    }

    pub fn builtins(&self) -> &[String] {
        &self.builtins
    }

    pub fn blank(&self) -> bool {
        self.blank
    }

    pub fn aliases(&self) -> Vec<(String, String)> {
        self.aliases
            .iter()
            .map(|(alias, namespace)| (alias.clone(), namespace.clone()))
            .collect()
    }

    pub fn rewrite(&self, form: Form) -> Form {
        match form {
            Form::Symbol(name) => Form::Symbol(self.resolve_symbol(&name)),
            Form::List(values) => {
                if matches!(
                    values.first(),
                    Some(Form::Symbol(name)) if name == "quote" || name == "require"
                ) {
                    Form::List(values)
                } else {
                    Form::List(
                        values
                            .into_iter()
                            .map(|value| self.rewrite(value))
                            .collect(),
                    )
                }
            }
            Form::Vector(values) => Form::Vector(
                values
                    .into_iter()
                    .map(|value| self.rewrite(value))
                    .collect(),
            ),
            Form::Set(values) => Form::Set(
                values
                    .into_iter()
                    .map(|value| self.rewrite(value))
                    .collect(),
            ),
            Form::Map(values) => Form::Map(
                values
                    .into_iter()
                    .map(|(key, value)| (self.rewrite(key), self.rewrite(value)))
                    .collect(),
            ),
            Form::Tagged(tag, value) => Form::Tagged(tag, Box::new(self.rewrite(*value))),
            Form::Metadata(meta, value) => Form::Metadata(meta, Box::new(self.rewrite(*value))),
            value => value,
        }
    }

    fn resolve_symbol(&self, symbol: &str) -> String {
        if let Some((namespace, method)) = symbol.rsplit_once('/') {
            if known_namespace(namespace) {
                return canonical(namespace, method);
            }
        }
        if let Some(canonical) = self.refers.get(symbol) {
            return canonical.clone();
        }
        let Some((alias, method)) = symbol.split_once('/') else {
            return symbol.into();
        };
        if LIBRARIES
            .iter()
            .any(|(_, _, default_alias)| *default_alias == alias)
            && !self.aliases.contains_key(alias)
        {
            return format!("unavailable/{symbol}");
        }
        self.aliases
            .get(alias)
            .map_or_else(|| symbol.into(), |namespace| canonical(namespace, method))
    }

    fn put_alias(&mut self, alias: &str, namespace: &str) -> Result<(), String> {
        if alias.is_empty() {
            return Err("Namespace alias cannot be empty".into());
        }
        if let Some(previous) = self.aliases.get(alias) {
            if previous != namespace {
                return Err(format!(
                    "Namespace alias already refers to {previous}: {alias}"
                ));
            }
            return Ok(());
        }
        self.aliases.insert(alias.into(), namespace.into());
        Ok(())
    }

    pub fn apply_require(
        &mut self,
        form: &Form,
        available: &impl Fn(&str) -> bool,
    ) -> Result<(), String> {
        let (target, options) = match form {
            Form::Vector(items) => {
                let target = match items.first() {
                    Some(Form::Symbol(target)) => target.as_str(),
                    _ => return Err(":require namespace must be a symbol".into()),
                };
                (normalize_namespace(target), &items[1..])
            }
            Form::List(items)
                if items.len() == 2
                    && matches!(&items[0], Form::Symbol(q) if q == "quote")
                    && matches!(&items[1], Form::Symbol(_)) =>
            {
                let target = match &items[1] {
                    Form::Symbol(target) => target.as_str(),
                    _ => unreachable!(),
                };
                (normalize_namespace(target), &[][..])
            }
            _ => return Err(":require expects vectors such as [hara.lib.string :as str]".into()),
        };
        if !known_namespace(target) && !available(target) {
            return Err(format!(
                "Cannot require missing generated namespace: {target}"
            ));
        }
        if !self.required_namespaces.iter().any(|value| value == target) {
            self.required_namespaces.push(target.into());
        }
        if options.len() % 2 != 0 {
            return Err(format!("Malformed :require options for {target}"));
        }
        let lazy = options.chunks(2).any(|option| {
            matches!(&option[0], Form::Keyword(name) if name == "lazy")
                && matches!(&option[1], Form::Bool(true))
        });
        let has_alias = options
            .chunks(2)
            .any(|option| matches!(&option[0], Form::Keyword(name) if name == "as"));
        if lazy && !has_alias {
            return Err(":require :lazy requires :as".into());
        }
        for option in options.chunks(2) {
            let name = keyword(&option[0], "Malformed :require options")?;
            match name {
                "as" => {
                    let alias = symbol(&option[1], ":require :as expects an unqualified symbol")?;
                    if alias.contains('/') {
                        return Err(":require :as expects an unqualified symbol".into());
                    }
                    self.put_alias(alias, target)?;
                }
                "refer" => {
                    if lazy {
                        return Err(":require :lazy cannot be combined with :refer".into());
                    }
                    let names = vector(&option[1], ":require :refer expects a vector of symbols")?;
                    for value in names {
                        let name = symbol(value, ":require :refer expects unqualified symbols")?;
                        if name.contains('/') {
                            return Err(":require :refer expects unqualified symbols".into());
                        }
                        let canonical = canonical(target, name);
                        if let Some(previous) = self.refers.insert(name.into(), canonical) {
                            return Err(format!(
                                "Referred symbol already exists: {name} ({previous})"
                            ));
                        }
                    }
                }
                "refer-macros" => {
                    if lazy {
                        return Err(":require :lazy cannot be combined with :refer-macros".into());
                    }
                    let names = vector(
                        &option[1],
                        ":require :refer-macros expects a vector of symbols",
                    )?;
                    for value in names {
                        let name = symbol(
                            value,
                            ":require :refer-macros expects unqualified symbols",
                        )?;
                        if name.contains('/') {
                            return Err(
                                ":require :refer-macros expects unqualified symbols".into(),
                            );
                        }
                    }
                }
                "lazy" => {
                    if !matches!(&option[1], Form::Bool(true)) {
                        return Err(":require :lazy expects true".into());
                    }
                }
                "reload" => {
                    if !matches!(&option[1], Form::Bool(true)) {
                        return Err(":require :reload expects true".into());
                    }
                }
                other => return Err(format!("Unsupported :require option: :{other}")),
            }
        }
        Ok(())
    }

    pub fn apply_use(
        &mut self,
        form: &Form,
        available: &impl Fn(&str) -> bool,
    ) -> Result<(), String> {
        let target = match form {
            Form::Symbol(target) if !target.contains('/') => normalize_namespace(target),
            _ => return Err(":use expects unqualified namespace symbols".into()),
        };
        if !known_namespace(target) && !available(target) {
            return Err(format!("Cannot use missing generated namespace: {target}"));
        }
        if !self.required_namespaces.iter().any(|value| value == target) {
            self.required_namespaces.push(target.into());
        }
        if !self.used_namespaces.iter().any(|value| value == target) {
            self.used_namespaces.push(target.into());
        }
        Ok(())
    }
}

fn parse_config(
    form: &Form,
    builtins: &mut Vec<String>,
    blank: &mut bool,
    excluded: &mut HashSet<String>,
    overrides: &mut HashMap<String, String>,
) -> Result<(), String> {
    let options = match form {
        Form::Map(options) => options,
        _ => return Err(":config expects one map".into()),
    };
    for (key, value) in options {
        match keyword(key, ":config keys must be unqualified keywords")? {
            "blank" => {
                *blank = match value {
                    Form::Bool(value) => *value,
                    _ => return Err(":config :blank expects a boolean".into()),
                };
            }
            "builtins" => {
                let mut seen = HashSet::new();
                for item in vector(value, ":config :builtins expects a vector of symbols")? {
                    let name = symbol(item, ":config :builtins expects unqualified symbols")?;
                    if name.contains('/') {
                        return Err(":config :builtins expects unqualified symbols".into());
                    }
                    if !seen.insert(name) {
                        return Err(format!("Duplicate builtin declaration: {name}"));
                    }
                    builtins.push(name.into());
                }
            }
            "intrinsics" => {
                parse_intrinsics(value, excluded, overrides)?;
            }
            other => return Err(format!("Unsupported :config option: :{other}")),
        }
    }
    Ok(())
}

fn parse_intrinsics(
    form: &Form,
    excluded: &mut HashSet<String>,
    overrides: &mut HashMap<String, String>,
) -> Result<(), String> {
    let options = match form {
        Form::Map(options) => options,
        _ => return Err(":intrinsics expects :all or an options map".into()),
    };
    for (key, value) in options {
        match keyword(key, ":intrinsics option keys must be keywords")? {
            "exclude" => {
                for item in vector(
                    value,
                    ":intrinsics :exclude expects a vector of library symbols",
                )? {
                    let library = library(symbol(
                        item,
                        ":intrinsics :exclude expects unqualified library symbols",
                    )?)?;
                    if !excluded.insert(library.into()) {
                        return Err(format!("Duplicate intrinsic exclusion: {library}"));
                    }
                }
            }
            "aliases" => {
                let aliases = match value {
                    Form::Map(aliases) => aliases,
                    _ => return Err(":intrinsics :aliases expects a map".into()),
                };
                for (library_form, alias_form) in aliases {
                    let library = library(symbol(
                        library_form,
                        ":intrinsics :aliases expects library symbols",
                    )?)?;
                    let alias =
                        symbol(alias_form, "Intrinsic aliases must be unqualified symbols")?;
                    if alias.contains('/') {
                        return Err("Intrinsic aliases must be unqualified symbols".into());
                    }
                    if overrides.insert(library.into(), alias.into()).is_some() {
                        return Err(format!("Duplicate intrinsic alias: {library}"));
                    }
                }
            }
            other => return Err(format!("Unsupported :intrinsics option: :{other}")),
        }
    }
    Ok(())
}

fn list<'a>(form: &'a Form, error: &str) -> Result<&'a [Form], String> {
    match form {
        Form::List(values) => Ok(values),
        _ => Err(error.into()),
    }
}
fn vector<'a>(form: &'a Form, error: &str) -> Result<&'a [Form], String> {
    match form {
        Form::Vector(values) => Ok(values),
        _ => Err(error.into()),
    }
}
fn keyword<'a>(form: &'a Form, error: &str) -> Result<&'a str, String> {
    match form {
        Form::Keyword(value) => Ok(value),
        _ => Err(error.into()),
    }
}
fn symbol<'a>(form: &'a Form, error: &str) -> Result<&'a str, String> {
    match form {
        Form::Symbol(value) => Ok(value),
        _ => Err(error.into()),
    }
}
fn library(value: &str) -> Result<&str, String> {
    if value.contains('/') {
        return Err("Intrinsic library names must be unqualified symbols".into());
    }
    LIBRARIES
        .iter()
        .find(|(library, _, _)| *library == value)
        .map(|(library, _, _)| *library)
        .ok_or_else(|| format!("Unknown intrinsic library: {value}"))
}
fn normalize_namespace(value: &str) -> &str {
    match value {
        "core" | "hara.lib.core" => "std.foundation",
        "hara.lib.string" => "std.foundation.string",
        "hara.lib.promise" => "std.foundation.promise",
        "hara.lib.bytes" => "std.foundation.bytes",
        "hara.lib.socket" => "std.foundation.socket",
        "hara.lib.file" => "std.foundation.file",
        value => value,
    }
}
fn known_namespace(value: &str) -> bool {
    let value = normalize_namespace(value);
    value == "std.foundation"
        || value == "std.foundation.coroutine"
        || value == "std.native"
        || value.starts_with("std.native.")
        || LIBRARIES
            .iter()
            .any(|(_, namespace, _)| *namespace == value)
}
fn canonical(namespace: &str, method: &str) -> String {
    if namespace == "std.foundation" {
        return format!("std.foundation/{method}");
    }
    match (normalize_namespace(namespace), method) {
        ("std.foundation", method) => method.into(),
        ("std.native.Maths", method) => format!("std.native.Maths/{method}"),
        ("std.native.Numbers", method) => format!("std.native.Numbers/{method}"),
        ("std.native.Bits", method) => format!("std.native.Bits/{method}"),
        ("std.native.String", method) => format!("str/{method}"),
        ("std.native.Bytes", "new") => "bytes".into(),
        ("std.native.Bytes", "instance?") => "bytes?".into(),
        ("std.native.Bytes", method) => format!("bytes/{method}"),
        ("std.native.File", method) => format!("file/{method}"),
        ("std.native.Socket", method) => format!("socket/{method}"),
        ("std.native.Promise", "run") => "promise/run".into(),
        ("std.native.Promise", "instance?") => "promise?".into(),
        ("std.native.Promise", method) => format!("promise/{method}"),
        ("std.native.Coroutine", "instance?") => "std.foundation.coroutine/coroutine?".into(),
        ("std.native.Coroutine", method) => format!("std.foundation.coroutine/{method}"),
        ("std.native.Arr", "new") => "array".into(),
        ("std.native.Arr", "instance?") => "array?".into(),
        ("std.native.Obj", "new") => "object".into(),
        ("std.native.Obj", "instance?") => "object?".into(),
        ("std.native.Runtime", method) => method.into(),
        ("std.native.Printer", method) => method.into(),
        ("std.native.Edn", method) => format!("std.native.Edn/{method}"),
        ("std.native.Json", method) => format!("std.native.Json/{method}"),
        ("std.native.Regex", "instance?") => "regexp?".into(),
        ("std.native.UUID", "instance?") => "uuid?".into(),
        ("std.native.Error", method) => format!("std.native.Error/{method}"),
        ("std.foundation.coroutine", method) => format!("std.foundation.coroutine/{method}"),
        ("std.foundation.string", method) => format!("std.foundation.string/{method}"),
        ("std.lib.string", "len") => "str/count".into(),
        ("std.lib.string", "to-upper") => "str/upper".into(),
        ("std.lib.string", "to-lower") => "str/lower".into(),
        ("std.lib.string", method) => format!("str/{method}"),
        ("std.foundation.promise", method) => format!("std.foundation.promise/{method}"),
        ("std.foundation.bytes", method) => format!("std.foundation.bytes/{method}"),
        ("std.foundation.file", method) => format!("std.foundation.file/{method}"),
        ("std.lib.promise", "then") => "promise/then".into(),
        ("std.lib.promise", "catch") => "promise/catch".into(),
        ("std.lib.promise", method) => format!("promise/{method}"),
        ("std.lib.bytes", method) => format!("bytes/{method}"),
        ("std.foundation.socket", method) => format!("std.foundation.socket/{method}"),
        ("std.lib.socket", method) => format!("socket/{method}"),
        ("std.lib.file", method) => format!("file/{method}"),
        (namespace, method) => format!("{namespace}/{method}"),
    }
}

#[cfg(test)]
mod tests {
    use super::GeneratedNamespaceConfig;
    use crate::kernel::parse_forms;

    #[test]
    fn configures_defaults_exclusions_aliases_and_requires_without_sources() {
        let forms = parse_forms(
            "(:intrinsics {:exclude [bytes] :aliases {string text}}) \
             (:require [hara.lib.string :as s :refer [trim]])",
        )
        .unwrap();
        let config = GeneratedNamespaceConfig::configure(&forms).unwrap();
        let rewritten = config.rewrite(
            parse_forms("(trim (s/trim (text/to-upper \" x \")))")
                .unwrap()
                .remove(0),
        );
        let display = format!("{rewritten:?}");
        assert!(display.contains("std.foundation.string/trim"));
        assert!(display.contains("std.foundation.string/to-upper"));
        assert!(display.contains("bytes/count") == false);
        assert!(GeneratedNamespaceConfig::configure(
            &parse_forms("(:require [missing.lib :as x])").unwrap()
        )
        .unwrap_err()
        .contains("missing generated namespace"));
    }

    #[test]
    fn parses_config_clause_with_builtins_blank_and_intrinsics() {
        let forms = parse_forms(
            "(:config {:blank true \
                       :builtins [+ - = count get] \
                       :intrinsics {:exclude [bytes]}})",
        )
        .unwrap();
        let config = GeneratedNamespaceConfig::configure(&forms).unwrap();
        assert!(config.blank());
        assert_eq!(config.builtins(), &["+", "-", "=", "count", "get"]);
        assert_eq!(
            config
                .rewrite(parse_forms("bytes").unwrap().remove(0))
                .to_string(),
            "bytes"
        );
    }

    #[test]
    fn records_used_namespaces_for_runtime_referral() {
        let config = GeneratedNamespaceConfig::configure_with(
            &parse_forms("(:use code.test)").unwrap(),
            |target| target == "code.test",
        )
        .unwrap();
        assert_eq!(config.required_namespaces(), &["code.test"]);
        assert_eq!(config.used_namespaces(), &["code.test"]);
        assert!(GeneratedNamespaceConfig::configure(
            &parse_forms("(:use [code.test])").unwrap()
        )
        .unwrap_err()
        .contains(":use expects unqualified namespace symbols"));
    }
}
