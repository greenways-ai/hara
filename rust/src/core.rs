#![allow(clippy::too_many_lines)] // Temporary compatibility facade during Java-port split.
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet, VecDeque};

pub use crate::kernel::Form;
use crate::kernel::{NamespaceRegistry, Var as KernelVar, VarOrigin};
use crate::lang::data::List as PList;
use crate::lang::data::{
    Atom as PAtom, Cons as PCons, Keyword, Map as PMap, OrderedMap as POrderedMap,
    OrderedSet as POrderedSet, Pointer as PPointer, Queue as PQueue, Set as PSet,
    SortedMap as PSortedMap, SortedSet as PSortedSet, Symbol, TaggedLiteral as PTaggedLiteral,
    Trie as PTrie, Tuple as PTuple, Vector as PVector,
};
use crate::lang::data::{Metadata, MetadataValue};
use crate::lang::protocol::{IDisplay, IMetadata, INamespaced};
pub use crate::task::{LocalPromiseProvider, Promise, PromiseProvider, PromiseState};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

#[path = "fiber.rs"]
mod fiber;
pub use fiber::{EvalFiber, EvalFiberState, Step};

pub fn completion_symbols() -> &'static [&'static str] {
    fiber::completion_symbols()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionValue {
    pub provider: String,
    pub type_name: String,
    pub handle: u64,
}

#[derive(Debug, Clone)]
pub struct StructType {
    pub name: String,
    pub fields: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct StructValue {
    pub ty: Rc<StructType>,
    pub values: Vec<Value>,
}

#[derive(Debug, Clone)]
pub struct GuestProtocol {
    pub name: String,
    pub methods: HashMap<String, usize>,
}

#[derive(Debug, Clone)]
pub struct NativeType {
    pub name: String,
    pub methods: Vec<String>,
    pub metadata: Option<Rc<Metadata>>,
}

pub(crate) const NATIVE_TYPES: &[(&str, &[&str])] = &[
    ("Maths", &["abs", "acos", "acosh", "asin", "asinh", "atan", "atan2", "atanh", "ceil", "cos", "cosh", "exp", "floor", "pow", "sin", "sinh", "sqrt", "tan", "tanh"]),
    ("Numbers", &["long", "double"]),
    ("Bits", &["and", "or", "xor", "not", "shift-left", "shift-right"]),
    ("String", &["length", "blank?", "includes?", "starts-with?", "ends-with?", "char-at", "slice", "index-of", "last-index-of", "join", "split", "split-lines", "repeat", "replace", "replace-first", "trim", "trim-left", "trim-right", "upper", "lower", "capitalize", "decapitalize", "pad-left", "pad-right", "reverse", "encode-utf8", "decode-utf8", "comp", "lt?", "gt?", "to-fixed"]),
    ("Bytes", &["new", "instance?", "count", "get", "set", "copy", "slice", "u8", "s8"]),
    ("File", &["resolve", "read", "write", "exists?", "list", "mkdir", "delete"]),
    ("Socket", &["connect", "listen", "endpoint", "events", "next", "send", "close"]),
    ("Promise", &["run", "new", "from", "all", "delay", "instance?"]),
    ("Coroutine", &["create", "yield", "await", "instance?"]),
    ("Array", &["new", "instance?"]),
    ("Object", &["new", "instance?"]),
    ("Runtime", &["load-string", "macroexpand-1", "gensym", "var-sym"]),
    ("Printer", &["str", "pr-str"]),
    ("Edn", &["read"]),
    ("Json", &["read", "write", "pretty"]),
    ("Regex", &["instance?"]),
    ("UUID", &["instance?"]),
    ("Error", &["new", "message", "class"]),
];

pub(crate) fn native_type_values() -> Vec<(String, Value)> {
    NATIVE_TYPES
        .iter()
        .map(|(name, methods)| {
            (
                (*name).to_owned(),
                Value::NativeType(Rc::new(NativeType {
                    name: format!("std.native/{name}"),
                    methods: methods.iter().map(|method| (*method).to_owned()).collect(),
                    metadata: None,
                })),
            )
        })
        .collect()
}

pub(crate) const FOUNDATION_PROTOCOLS: &[(&str, &[(&str, usize)])] = &[
    (
        "IApplicable",
        &[
            ("apply-in", 3),
            ("apply-default", 1),
            ("transform-in", 3),
            ("transform-out", 4),
        ],
    ),
    ("IAssoc", &[("assoc", 3)]),
    ("ICas", &[("cas", 3)]),
    ("IClose", &[("close", 1)]),
    (
        "IComponent",
        &[
            ("props", 1),
            ("status", 1),
            ("started?", 1),
            ("stopped?", 1),
            ("start", 1),
            ("stop", 1),
            ("kill", 1),
            ("remote?", 1),
        ],
    ),
    ("IConj", &[("conj", 2)]),
    ("ICons", &[("cons", 2)]),
    ("IContext", &[("call", usize::MAX)]),
    ("ICoroutine", &[("status", 1), ("resume", usize::MAX)]),
    (
        "IContextLifeCycle",
        &[
            ("has-module?", 2),
            ("setup-module", 2),
            ("teardown-module", 2),
            ("has-pointer?", 2),
            ("setup-pointer", 2),
            ("teardown-pointer", 2),
        ],
    ),
    ("ICount", &[("count", 1)]),
    ("IDeref", &[("deref", 1)]),
    ("IDerefTimeout", &[("deref-timeout", 3)]),
    ("IDisplay", &[("display", 1)]),
    ("IDissoc", &[("dissoc", 2)]),
    ("IEmpty", &[("empty", 1)]),
    ("IEquality", &[("equality", 2)]),
    ("IExInfo", &[("data", 1)]),
    ("IFind", &[("find", 2)]),
    ("IFn", &[("invoke", usize::MAX)]),
    ("IHash", &[("hash", 1)]),
    ("IHashCached", &[("hash-current", 1), ("hash-put", 2)]),
    ("IIndexed", &[("index-of", 2)]),
    ("IIndexedKV", &[("index-of-key", 2), ("index-of-val", 2)]),
    ("IInvokeIn", &[("invoke-in", usize::MAX)]),
    ("IIter", &[("iter", 1)]),
    ("IIterator", &[("iter-next?", 1), ("iter-next", 1)]),
    ("ILookup", &[("lookup", usize::MAX)]),
    ("IMutable", &[]),
    ("INamespaced", &[("name", 1), ("namespace", 1)]),
    ("INth", &[("nth", 2)]),
    ("IOFn", &[]),
    ("IObjType", &[("meta", 1), ("with-meta", 2)]),
    ("IPair", &[("key", 1), ("value", 1)]),
    ("IPeekFirst", &[("peek-first", 1)]),
    ("IPeekLast", &[("peek-last", 1)]),
    ("IPersistent", &[]),
    (
        "IPromise",
        &[
            ("state", 1),
            ("value", 1),
            ("then", 2),
            ("catch", 2),
            ("finally", 2),
            ("cancel", 1),
        ],
    ),
    (
        "IPointer",
        &[("ptr-context", 1), ("ptr-keys", 1), ("ptr-val", 2)],
    ),
    ("IPopFirst", &[("pop-first", 1)]),
    ("IPopLast", &[("pop-last", 1)]),
    ("IPushFirst", &[("push-first", 2)]),
    ("IPushLast", &[("push-last", 2)]),
    ("IRealize", &[("realized?", 1), ("realize", 1)]),
    ("IReduce", &[("reduce", usize::MAX)]),
    ("IReset", &[("reset", 2)]),
    (
        "ISpace",
        &[
            ("context-set", 4),
            ("context-unset", 2),
            ("context-list", 1),
            ("context-get", 2),
            ("rt-active", 1),
            ("rt-get", 2),
            ("rt-start", 2),
            ("rt-started?", 2),
            ("rt-stopped?", 2),
            ("rt-stop", 2),
        ],
    ),
    ("IToMutable", &[("to-mutable", 1)]),
    ("IToPersistent", &[("to-persistent", 1)]),
    (
        "IWatch",
        &[("watch-add", 3), ("watch-remove", 2), ("watch-list", 1)],
    ),
];

pub(crate) fn builtin_protocol_namespace(protocol: &str) -> String {
    format!("std.protocol.{}", protocol.to_ascii_lowercase())
}

pub(crate) fn builtin_protocol_name(protocol: &str) -> String {
    format!("{}/{}", builtin_protocol_namespace(protocol), protocol)
}

fn canonical_protocol_name(protocol: &str) -> String {
    let simple = protocol.strip_prefix("std.foundation/").unwrap_or(protocol);
    if FOUNDATION_PROTOCOLS
        .iter()
        .any(|(candidate, _)| *candidate == simple)
    {
        builtin_protocol_name(simple)
    } else {
        protocol.to_owned()
    }
}

pub(crate) fn foundation_protocol_values() -> Vec<(String, Value)> {
    FOUNDATION_PROTOCOLS
        .iter()
        .map(|(name, methods)| {
            (
                (*name).to_owned(),
                Value::Protocol(Rc::new(GuestProtocol {
                    name: builtin_protocol_name(name),
                    methods: methods
                        .iter()
                        .map(|(method, arity)| ((*method).to_owned(), *arity))
                        .collect(),
                })),
            )
        })
        .collect()
}

pub(crate) fn builtin_protocol_method_values() -> Vec<(String, String, Value)> {
    FOUNDATION_PROTOCOLS
        .iter()
        .flat_map(|(protocol, methods)| {
            methods.iter().map(move |(method, arity)| {
                let namespace = builtin_protocol_namespace(protocol);
                let protocol_name = builtin_protocol_name(protocol);
                let method_name = (*method).to_owned();
                let display_name = format!("{namespace}/{method}");
                let arity_display_name = display_name.clone();
                let (minimum_arity, maximum_arity) =
                    builtin_protocol_arity_range(protocol, method, *arity);
                (
                    namespace,
                    (*method).to_owned(),
                    native_variadic_function(&display_name, move |arguments| {
                        if arguments.len() < minimum_arity
                            || maximum_arity.is_some_and(|maximum| arguments.len() > maximum)
                        {
                            let expected = match maximum_arity {
                                Some(maximum) if maximum == minimum_arity => {
                                    minimum_arity.to_string()
                                }
                                Some(maximum) => format!("{minimum_arity} to {maximum}"),
                                None => format!("at least {minimum_arity}"),
                            };
                            return Err(format!(
                                "protocol/arity: {arity_display_name} expects {expected} arguments, received {}",
                                arguments.len()
                            ));
                        }
                        protocol_call(&protocol_name, &method_name, &arguments)
                    }),
                )
            })
        })
        .collect()
}

fn builtin_protocol_arity_range(
    protocol: &str,
    method: &str,
    declared_arity: usize,
) -> (usize, Option<usize>) {
    if declared_arity != usize::MAX {
        return (declared_arity, Some(declared_arity));
    }
    match (protocol, method) {
        ("ILookup", "lookup") | ("IReduce", "reduce") => (2, Some(3)),
        ("IInvokeIn", "invoke-in") => (2, None),
        _ => (1, None),
    }
}

#[derive(Debug, Clone)]
pub struct ExceptionInfo {
    pub message: String,
    pub data: Box<Value>,
    pub cause: Option<Box<Value>>,
}

#[derive(Debug, Clone)]
pub enum Value {
    Number(i64),
    Float(f64),
    BigInteger(String),
    Decimal(String),
    Character(char),
    Regex(String),
    Tagged(Box<PTaggedLiteral<Value>>),
    Bool(bool),
    String(String),
    Keyword(Keyword),
    Bytes(Vec<u8>),
    ByteBuffer(Rc<RefCell<Vec<u8>>>),
    Array(Rc<RefCell<Vec<Value>>>),
    Object(Rc<RefCell<Vec<(String, Value)>>>),
    Promise(Promise),
    Atom(Box<RuntimeAtom>),
    Recur(Vec<Value>),
    Map(PMap<Value, Value>),
    OrderedMap(Box<POrderedMap<Value, Value>>),
    SortedMap(Box<PSortedMap<Value, Value>>),
    Trie(Box<PTrie<Value>>),
    Set(PSet<Value>),
    OrderedSet(Box<POrderedSet<Value>>),
    SortedSet(Box<PSortedSet<Value>>),
    List(PList<Value>),
    Cons(Box<PCons<Value>>),
    Queue(Box<PQueue<Value>>),
    Symbol(Symbol),
    Pointer(PPointer),
    Function(Rc<Function>),
    Tuple(Box<PTuple<Value>>),
    Vector(PVector<Value>),
    Iterator(Rc<RefCell<IteratorState>>),
    Var(KernelVar<Value>),
    Namespace(Rc<crate::kernel::Namespace<Value>>),
    Extension(ExtensionValue),
    StructType(Rc<StructType>),
    Struct(Rc<StructValue>),
    Protocol(Rc<GuestProtocol>),
    NativeType(Rc<NativeType>),
    Coroutine(Rc<Coroutine>),
    ExceptionInfo(Rc<ExceptionInfo>),
    Nil,
}

#[derive(Clone)]
pub struct Function {
    params: Vec<String>,
    variadic: Option<String>,
    body: Vec<Form>,
    captured: Rc<RefCell<HashMap<String, Value>>>,
    pub name: Option<String>,
    native: Option<Rc<dyn Fn(Vec<Value>) -> Result<Value, String>>>,
    /// Arity clauses for multi-arity `defn`/`fn` dispatchers; empty otherwise.
    clauses: Vec<Rc<Function>>,
    /// Whether this function is a macro expander.
    is_macro: bool,
}

impl std::fmt::Debug for Function {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Function")
            .field("params", &self.params)
            .field("variadic", &self.variadic)
            .field("name", &self.name)
            .field("native", &self.native.is_some())
            .finish()
    }
}

/// State of a portable coroutine value.
pub enum CoroutineState {
    /// The body has not started yet; stores the body function.
    New(Value),
    /// Parked at a yield/await; stores the continuation that resumes the body.
    Suspended(Box<dyn FnOnce(Value) -> Step>),
    /// Currently executing on a fiber.
    Running,
    /// Completed, closed, or killed by an error.
    Dead,
}

impl std::fmt::Debug for CoroutineState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::New(_) => formatter.debug_tuple("New").finish(),
            Self::Suspended(_) => formatter.debug_tuple("Suspended").finish(),
            Self::Running => formatter.write_str("Running"),
            Self::Dead => formatter.write_str("Dead"),
        }
    }
}

/// A re-entrant coroutine implemented with the fiber/CPS evaluator.
pub struct Coroutine {
    pub state: RefCell<CoroutineState>,
}

impl std::fmt::Debug for Coroutine {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Coroutine")
            .field("state", &self.state.borrow())
            .finish()
    }
}

impl Coroutine {
    pub fn new(body: Value) -> Self {
        Self {
            state: RefCell::new(CoroutineState::New(body)),
        }
    }
}

#[derive(Clone)]
pub struct RuntimeAtom {
    value: PAtom<Value>,
    watches: Rc<RefCell<Vec<(Value, Rc<Function>)>>>,
    watchable: bool,
}

impl RuntimeAtom {
    pub(crate) fn new(value: Value, watchable: bool) -> Self {
        Self {
            value: PAtom::new(value),
            watches: Rc::new(RefCell::new(Vec::new())),
            watchable,
        }
    }
    fn same_identity(&self, other: &Self) -> bool {
        self.value.same_identity(&other.value)
    }
    fn identity_address(&self) -> usize {
        self.value.identity_address()
    }
    pub(crate) fn deref_value(&self) -> Value {
        self.value.deref_value()
    }
    fn reset(&self, new_value: Value) -> Result<Value, String> {
        let old_value = self.value.deref_value();
        let result = self.value.reset(new_value.clone())?;
        self.notify(old_value, new_value)?;
        Ok(result)
    }
    fn compare_and_set(&self, old: &Value, new_value: Value) -> Result<bool, String> {
        let prior = self.value.deref_value();
        let changed = self.value.compare_and_set(old, new_value.clone())?;
        if changed {
            self.notify(prior, new_value)?;
        }
        Ok(changed)
    }
    fn add_watch(&self, key: Value, function: Rc<Function>) -> Result<(), String> {
        if !self.watchable {
            return Err("watch-add expects a standard atom".into());
        }
        let mut watches = self.watches.borrow_mut();
        watches.retain(|(candidate, _)| candidate != &key);
        watches.push((key, function));
        Ok(())
    }
    fn remove_watch(&self, key: &Value) -> Result<(), String> {
        if !self.watchable {
            return Err("watch-remove expects a standard atom".into());
        }
        self.watches
            .borrow_mut()
            .retain(|(candidate, _)| candidate != key);
        Ok(())
    }
    fn watch_entries(&self) -> Result<Vec<Value>, String> {
        if !self.watchable {
            return Err("watch-list expects a standard atom".into());
        }
        self.watches
            .borrow()
            .iter()
            .map(|(key, function)| {
                vector_literal(vec![key.clone(), Value::Function(function.clone())])
            })
            .collect()
    }
    fn notify(&self, old_value: Value, new_value: Value) -> Result<(), String> {
        let watches = self.watches.borrow().clone();
        for (key, function) in watches {
            call_function(
                &function,
                vec![
                    key,
                    Value::Atom(Box::new(self.clone())),
                    old_value.clone(),
                    new_value.clone(),
                ],
            )?;
        }
        Ok(())
    }
}

impl std::fmt::Debug for RuntimeAtom {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RuntimeAtom")
            .finish_non_exhaustive()
    }
}

pub(crate) fn native_function(
    name: &str,
    arity: usize,
    callback: impl Fn(Vec<Value>) -> Result<Value, String> + 'static,
) -> Value {
    Value::Function(Rc::new(Function {
        params: (0..arity).map(|index| format!("arg{index}")).collect(),
        variadic: None,
        body: Vec::new(),
        captured: Rc::new(RefCell::new(HashMap::new())),
        name: Some(name.into()),
        native: Some(Rc::new(callback)),
        clauses: Vec::new(),
        is_macro: false,
    }))
}

pub(crate) fn native_variadic_function(
    name: &str,
    callback: impl Fn(Vec<Value>) -> Result<Value, String> + 'static,
) -> Value {
    Value::Function(Rc::new(Function {
        params: Vec::new(),
        variadic: Some("arguments".into()),
        body: Vec::new(),
        captured: Rc::new(RefCell::new(HashMap::new())),
        name: Some(name.into()),
        native: Some(Rc::new(callback)),
        clauses: Vec::new(),
        is_macro: false,
    }))
}

pub(crate) fn exception_function_values() -> Vec<(&'static str, Value)> {
    vec![
        (
            "ex-info",
            native_variadic_function("ex-info", |arguments| {
                if !(2..=3).contains(&arguments.len()) {
                    return Err("ex-info expects a message, data map, and optional cause".into());
                }
                let Value::String(message) = &arguments[0] else {
                    return Err("ex-info expects a string message".into());
                };
                if map_entries(&arguments[1]).is_none() {
                    return Err("ex-info expects a data map".into());
                }
                Ok(Value::ExceptionInfo(Rc::new(ExceptionInfo {
                    message: message.clone(),
                    data: Box::new(arguments[1].clone()),
                    cause: arguments.get(2).cloned().map(Box::new),
                })))
            }),
        ),
        (
            "ex-data",
            native_function("ex-data", 1, |arguments| match &arguments[0] {
                Value::ExceptionInfo(value) => Ok((*value.data).clone()),
                _ => Ok(Value::Nil),
            }),
        ),
        (
            "ex-message",
            native_function("ex-message", 1, |arguments| match &arguments[0] {
                Value::ExceptionInfo(value) => Ok(Value::String(value.message.clone())),
                Value::String(value) => Ok(Value::String(value.clone())),
                value => Ok(Value::String(value.display())),
            }),
        ),
    ]
}

pub fn with_macros<R>(
    macros: Rc<RefCell<HashMap<(String, String), Rc<Function>>>>,
    operation: impl FnOnce() -> R,
) -> R {
    ACTIVE_MACROS.with(|active| {
        let previous = active.replace(Some(macros));
        let result = operation();
        active.replace(previous);
        result
    })
}

fn register_macro(namespace: &str, name: &str, function: Rc<Function>) -> Result<(), String> {
    ACTIVE_MACROS.with(|active| {
        active
            .try_borrow_mut()
            .map_err(|_| "macro registry is busy".into())
            .and_then(|mut opt| {
                if let Some(macros) = opt.as_ref() {
                    macros
                        .try_borrow_mut()
                        .map_err(|_| "macro registry is busy".into())
                        .map(|mut macros| {
                            macros.insert((namespace.into(), name.into()), function);
                        })
                } else {
                    Err("macro registry is unavailable".into())
                }
            })
    })
}

fn resolve_macro_in(namespace: &str, name: &str) -> Option<Rc<Function>> {
    ACTIVE_MACROS.with(|active| {
        active.borrow().as_ref().and_then(|macros| {
            macros
                .borrow()
                .get(&(namespace.into(), name.into()))
                .cloned()
        })
    })
}

pub(crate) fn resolve_macro(name: &str) -> Option<Rc<Function>> {
    if let Some((namespace, local)) = name.split_once('/') {
        return resolve_macro_in(namespace, local);
    }
    let current = namespace_registry()
        .map(|registry| registry.current().name().as_str().to_owned())
        .ok()?;
    resolve_macro_in(&current, name).or_else(|| resolve_macro_in("std.foundation", name))
}

fn gensym(prefix: &str) -> String {
    let index = GENSYM_COUNTER.with(|counter| {
        let value = counter.get();
        counter.set(value + 1);
        value
    });
    format!("{prefix}{index}")
}

fn form_to_value(form: &Form) -> Result<Value, String> {
    literal_value(form)
}

fn value_to_form(value: &Value) -> Result<Form, String> {
    match value {
        Value::Nil => Ok(Form::Nil),
        Value::Bool(value) => Ok(Form::Bool(*value)),
        Value::Number(value) => Ok(Form::Number(*value)),
        Value::Float(value) => Ok(Form::Float(*value)),
        Value::BigInteger(value) => Ok(Form::BigInteger(value.clone())),
        Value::Decimal(value) => Ok(Form::Decimal(value.clone())),
        Value::Character(value) => Ok(Form::Character(*value)),
        Value::String(value) => Ok(Form::String(value.clone())),
        Value::Keyword(value) => Ok(Form::Keyword(value.as_str().into())),
        Value::Symbol(value) => Ok(Form::Symbol(value.as_str().into())),
        Value::Tagged(value) => Ok(Form::Tagged(
            value.tag().get_name().into(),
            Box::new(value_to_form(value.form())?),
        )),
        Value::List(values) => Ok(Form::List(
            values
                .iter()
                .map(|v| value_to_form(v))
                .collect::<Result<_, _>>()?,
        )),
        Value::Queue(values) => Ok(Form::List(
            values
                .iter()
                .map(|v| value_to_form(v))
                .collect::<Result<_, _>>()?,
        )),
        Value::Cons(values) => Ok(Form::List(
            values
                .iter()
                .map(|v| value_to_form(&v))
                .collect::<Result<_, _>>()?,
        )),
        Value::Vector(values) => Ok(Form::Vector(
            values
                .iter()
                .map(|v| value_to_form(v))
                .collect::<Result<_, _>>()?,
        )),
        Value::Tuple(values) => Ok(Form::Vector(
            values
                .iter()
                .map(|v| value_to_form(v))
                .collect::<Result<_, _>>()?,
        )),
        Value::Set(_) | Value::OrderedSet(_) | Value::SortedSet(_) => Ok(Form::Set(
            set_items(value)
                .unwrap()
                .iter()
                .copied()
                .map(value_to_form)
                .collect::<Result<_, _>>()?,
        )),
        Value::Map(_) | Value::OrderedMap(_) | Value::SortedMap(_) | Value::Trie(_) => {
            Ok(Form::Map(
                map_entries(value)
                    .unwrap()
                    .into_iter()
                    .map(|(key, value)| -> Result<(Form, Form), String> {
                        Ok((value_to_form(&key)?, value_to_form(&value)?))
                    })
                    .collect::<Result<_, _>>()?,
            ))
        }
        value => Err(format!("cannot use {} as code", portable_type_name(value))),
    }
}

fn macro_environment() -> Result<Value, String> {
    let namespace = namespace_registry()?.current().name().as_str().to_owned();
    let entries = vec![
        (
            Value::Keyword(Keyword::from("ns")),
            Value::Symbol(Symbol::from(namespace)),
        ),
        (
            Value::Keyword(Keyword::from("locals")),
            Value::OrderedMap(Box::new(POrderedMap::new())),
        ),
        (
            Value::Keyword(Keyword::from("aliases")),
            Value::OrderedMap(Box::new(POrderedMap::new())),
        ),
    ];
    Ok(Value::OrderedMap(Box::new(POrderedMap::from_iter(entries))))
}

fn macroexpand_call(
    name: &str,
    invocation: &[Form],
    env: &mut HashMap<String, Value>,
) -> Result<Option<Form>, String> {
    let function = match resolve_macro(name) {
        Some(function) => function,
        None => return Ok(None),
    };
    let mut arguments = Vec::with_capacity(invocation.len() + 1);
    arguments.push(form_to_value(&Form::List(invocation.to_vec()))?);
    arguments.push(macro_environment()?);
    for form in &invocation[1..] {
        arguments.push(form_to_value(form)?);
    }
    let expansion = call_function(&function, arguments)?;
    let expansion = value_to_form(&expansion)?;
    #[cfg(feature = "dev-trace")]
    development_trace_macro(name, &Form::List(invocation.to_vec()), &expansion);
    Ok(Some(expansion))
}

fn macro_clause_with_implicit_params(clause: &Form) -> Result<Form, String> {
    match clause {
        Form::List(parts) if !parts.is_empty() => {
            let params = match &parts[0] {
                Form::Vector(params) => params,
                _ => return Err("macro arity must start with a parameter vector".into()),
            };
            let mut implicit = vec![Form::Symbol("&form".into()), Form::Symbol("&env".into())];
            implicit.extend_from_slice(params);
            let mut new_parts = vec![Form::Vector(implicit)];
            new_parts.extend_from_slice(&parts[1..]);
            Ok(Form::List(new_parts))
        }
        _ => Err("macro arity must be a list".into()),
    }
}

fn macroexpand_once(form: &Form, env: &mut HashMap<String, Value>) -> Result<Form, String> {
    match form {
        Form::List(values) if !values.is_empty() => {
            if let Form::Symbol(name) = &values[0] {
                if let Some(expanded) = macroexpand_call(name, values, env)? {
                    return Ok(expanded);
                }
            }
            Ok(form.clone())
        }
        _ => Ok(form.clone()),
    }
}

thread_local! {
    static TRACE_ENABLED: Cell<bool> = const { Cell::new(false) };
    static TRACE_STACK: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    #[cfg(feature = "dev-trace")]
    static DEVELOPMENT_TRACE: RefCell<Option<crate::trace::TraceCollector>> = const { RefCell::new(None) };
    #[cfg(feature = "dev-trace")]
    static DEVELOPMENT_TRACE_STACK: RefCell<Vec<crate::trace::OperationId>> = const { RefCell::new(Vec::new()) };
    static ACTIVE_MACROS: RefCell<Option<Rc<RefCell<HashMap<(String, String), Rc<Function>>>>>> =
        const { RefCell::new(None) };
    static GENSYM_COUNTER: Cell<u64> = const { Cell::new(0) };
}

#[cfg(feature = "dev-trace")]
fn development_preview(value: &Value) -> crate::trace::ValuePreview {
    DEVELOPMENT_TRACE.with(|active| {
        active
            .borrow()
            .as_ref()
            .expect("development trace must be active")
            .preview_value(portable_type_name(value), value.display())
    })
}

#[cfg(feature = "dev-trace")]
fn development_trace_enter(
    function: &Function,
    arguments: &[Value],
) -> Option<crate::trace::OperationId> {
    if DEVELOPMENT_TRACE.with(|active| active.borrow().is_none()) {
        return None;
    }
    let values = arguments.iter().map(development_preview).collect();
    let parent_operation = DEVELOPMENT_TRACE_STACK.with(|stack| stack.borrow().last().copied());
    let depth = DEVELOPMENT_TRACE_STACK.with(|stack| stack.borrow().len());
    DEVELOPMENT_TRACE.with(|active| {
        let mut active = active.borrow_mut();
        let collector = active.as_mut()?;
        let operation = collector.next_operation_id();
        let mut event = crate::trace::TraceEvent::new(crate::trace::TraceEventKind::OperationEnter);
        event.operation = Some(operation);
        event.parent_operation = parent_operation;
        event.depth = depth;
        event.function = Some(
            function
                .name
                .clone()
                .unwrap_or_else(|| "<anonymous>".into()),
        );
        event.values = values;
        collector.record(event);
        DEVELOPMENT_TRACE_STACK.with(|stack| stack.borrow_mut().push(operation));
        Some(operation)
    })
}

#[cfg(feature = "dev-trace")]
fn development_trace_exit(
    operation: Option<crate::trace::OperationId>,
    function: &Function,
    result: Option<&Value>,
) {
    let Some(operation) = operation else { return };
    let value = result.map(development_preview);
    DEVELOPMENT_TRACE.with(|active| {
        if let Some(collector) = active.borrow_mut().as_mut() {
            let mut event =
                crate::trace::TraceEvent::new(crate::trace::TraceEventKind::OperationReturn);
            event.operation = Some(operation);
            event.function = Some(
                function
                    .name
                    .clone()
                    .unwrap_or_else(|| "<anonymous>".into()),
            );
            event.values = value.into_iter().collect();
            collector.record(event);
        }
    });
    DEVELOPMENT_TRACE_STACK.with(|stack| {
        let popped = stack.borrow_mut().pop();
        debug_assert_eq!(popped, Some(operation));
    });
}

#[cfg(feature = "dev-trace")]
fn development_trace_macro(name: &str, source: &Form, expansion: &Form) {
    DEVELOPMENT_TRACE.with(|active| {
        if let Some(collector) = active.borrow_mut().as_mut() {
            let mut event =
                crate::trace::TraceEvent::new(crate::trace::TraceEventKind::MacroExpand);
            event.function = Some(name.into());
            event.values = vec![
                collector.preview_value("form", source.to_string()),
                collector.preview_value("form", expansion.to_string()),
            ];
            collector.record(event);
        }
    });
}

struct TraceGuard {
    previous: bool,
}

impl TraceGuard {
    fn enable() -> Self {
        let previous = TRACE_ENABLED.with(|enabled| {
            let previous = enabled.get();
            enabled.set(true);
            previous
        });
        TRACE_STACK.with(|stack| stack.borrow_mut().clear());
        Self { previous }
    }
}

impl Drop for TraceGuard {
    fn drop(&mut self) {
        TRACE_STACK.with(|stack| stack.borrow_mut().clear());
        TRACE_ENABLED.with(|enabled| enabled.set(self.previous));
    }
}

fn tracing_enabled() -> bool {
    TRACE_ENABLED.with(Cell::get)
}

fn append_trace(error: String) -> String {
    if !tracing_enabled() {
        return error;
    }
    let frames = TRACE_STACK.with(|stack| stack.borrow().iter().rev().cloned().collect::<Vec<_>>());
    if frames.is_empty() {
        return error;
    }
    if error.contains("\n[hara stack]") {
        return error;
    }
    format!(
        "{error}\n[hara stack]\n{}",
        frames
            .iter()
            .map(|frame| format!("  at {frame}"))
            .collect::<Vec<_>>()
            .join("\n")
    )
}

#[derive(Debug, Clone)]
enum IteratorGenerator {
    Constant(Value),
    Repeated(Rc<Function>),
    Iterate(Rc<Function>, Value),
    Take(Value, usize),
    Drop(Value, usize),
    Cycle(Value, Vec<Value>, usize, bool),
    TakeWhile(Rc<Function>, Value),
    DropWhile(Rc<Function>, Value, bool),
    Map(Rc<Function>, Value),
    Filter(Rc<Function>, Value),
    Mapcat(Rc<Function>, Value, Option<Value>),
    Keep(Rc<Function>, Value),
    Zip(Vec<Value>),
    Interleave(Vec<Value>, usize),
    Partition(Value, usize, bool),
}

#[derive(Debug, Clone)]
pub struct IteratorState {
    values: Vec<Value>,
    index: usize,
    closed: bool,
    cycle: bool,
    seq: bool,
    generator: Option<IteratorGenerator>,
}

impl IteratorState {
    fn new(values: Vec<Value>) -> Self {
        Self {
            values,
            index: 0,
            closed: false,
            cycle: false,
            seq: false,
            generator: None,
        }
    }
    fn generated(generator: IteratorGenerator) -> Self {
        Self {
            values: Vec::new(),
            index: 0,
            closed: false,
            cycle: false,
            seq: false,
            generator: Some(generator),
        }
    }
    fn has_next(&self) -> bool {
        !self.closed
            && (self.generator.is_some()
                || (!self.values.is_empty() && (self.cycle || self.index < self.values.len())))
    }
    fn next(&mut self) -> Result<Value, String> {
        if self.closed {
            return Err("iter-next reached the end of the iterator".into());
        }
        if let Some(generator) = &mut self.generator {
            return match generator {
                IteratorGenerator::Constant(value) => Ok(value.clone()),
                IteratorGenerator::Repeated(function) => call_function(function, Vec::new()),
                IteratorGenerator::Iterate(function, current) => {
                    let output = current.clone();
                    *current = call_function(function, vec![current.clone()])?;
                    Ok(output)
                }
                IteratorGenerator::Take(source, remaining) => {
                    if *remaining == 0 {
                        self.closed = true;
                        Err("iter-next reached the end of the iterator".into())
                    } else {
                        *remaining -= 1;
                        iterator_next(source)
                    }
                }
                IteratorGenerator::Drop(source, remaining) => {
                    while *remaining > 0 {
                        if iterator_next(source).is_err() {
                            self.closed = true;
                            return Err("iter-next reached the end of the iterator".into());
                        }
                        *remaining -= 1;
                    }
                    iterator_next(source)
                }
                IteratorGenerator::Cycle(source, cache, index, exhausted) => {
                    if *index < cache.len() {
                        let value = cache[*index].clone();
                        *index += 1;
                        Ok(value)
                    } else if *exhausted {
                        if cache.is_empty() {
                            self.closed = true;
                            Err("iter-next reached the end of the iterator".into())
                        } else {
                            *index = 1;
                            Ok(cache[0].clone())
                        }
                    } else {
                        match iterator_next(source) {
                            Ok(value) => {
                                cache.push(value.clone());
                                *index += 1;
                                Ok(value)
                            }
                            Err(_) => {
                                *exhausted = true;
                                if cache.is_empty() {
                                    self.closed = true;
                                    Err("iter-next reached the end of the iterator".into())
                                } else {
                                    *index = 1;
                                    Ok(cache[0].clone())
                                }
                            }
                        }
                    }
                }
                IteratorGenerator::TakeWhile(function, source) => {
                    let value = iterator_next(source)?;
                    if call_function(function, vec![value.clone()])?.truthy() {
                        Ok(value)
                    } else {
                        self.closed = true;
                        Err("iter-next reached the end of the iterator".into())
                    }
                }
                IteratorGenerator::DropWhile(function, source, started) => loop {
                    let value = iterator_next(source)?;
                    if *started || !call_function(function, vec![value.clone()])?.truthy() {
                        *started = true;
                        break Ok(value);
                    }
                },
                IteratorGenerator::Map(function, source) => {
                    let value = iterator_next(source)?;
                    match value {
                        Value::Tuple(values) => {
                            call_function(function, values.iter().cloned().collect())
                        }
                        Value::Vector(values) => {
                            call_function(function, values.iter().cloned().collect())
                        }
                        value => call_function(function, vec![value]),
                    }
                }
                IteratorGenerator::Filter(function, source) => loop {
                    let value = iterator_next(source)?;
                    if call_function(function, vec![value.clone()])?.truthy() {
                        break Ok(value);
                    }
                },
                IteratorGenerator::Mapcat(function, source, pending) => loop {
                    if let Some(iterator) = pending {
                        match iterator_next(iterator) {
                            Ok(value) => break Ok(value),
                            Err(_) => *pending = None,
                        }
                    }
                    let value = iterator_next(source)?;
                    *pending = Some(make_iterator(call_function(function, vec![value])?)?);
                },
                IteratorGenerator::Keep(function, source) => loop {
                    let value = iterator_next(source)?;
                    let mapped = call_function(function, vec![value])?;
                    if !matches!(mapped, Value::Nil) {
                        break Ok(mapped);
                    }
                },
                IteratorGenerator::Zip(sources) => {
                    let mut values = Vec::new();
                    for source in sources.iter() {
                        match iterator_next(source) {
                            Ok(value) => values.push(value),
                            Err(error) => {
                                self.closed = true;
                                return Err(error);
                            }
                        }
                    }
                    Ok(Value::Vector(values.into()))
                }
                IteratorGenerator::Interleave(sources, index) => {
                    if sources.is_empty() {
                        self.closed = true;
                        return Err("iter-next reached the end of the iterator".into());
                    }
                    let source = &sources[*index];
                    let value = iterator_next(source).map_err(|error| {
                        self.closed = true;
                        error
                    })?;
                    *index = (*index + 1) % sources.len();
                    Ok(value)
                }
                IteratorGenerator::Partition(source, amount, all) => {
                    let mut values = Vec::new();
                    for _ in 0..*amount {
                        match iterator_next(source) {
                            Ok(value) => values.push(value),
                            Err(error) => {
                                self.closed = true;
                                if values.is_empty() || !*all {
                                    return Err(error);
                                }
                                break;
                            }
                        }
                    }
                    if values.is_empty() {
                        self.closed = true;
                        Err("iter-next reached the end of the iterator".into())
                    } else {
                        Ok(Value::Vector(values.into()))
                    }
                }
            };
        }
        if self.values.is_empty() {
            return Err("iter-next reached the end of the iterator".into());
        }
        if self.cycle && self.index >= self.values.len() {
            self.index = 0;
        }
        if self.index >= self.values.len() {
            return Err("iter-next reached the end of the iterator".into());
        }
        let value = self.values[self.index].clone();
        self.index += 1;
        Ok(value)
    }
    fn close(&mut self) {
        self.closed = true;
    }
}

#[inline(never)]
fn sequential_equality(left: &Value, right: &Value) -> Option<bool> {
    fn items(value: &Value) -> Option<Vec<Value>> {
        match value {
            Value::List(values) => Some(values.iter().cloned().collect()),
            Value::Cons(values) => Some(values.iter().collect()),
            Value::Queue(values) => Some(values.iter().cloned().collect()),
            Value::Tuple(values) => Some(values.iter().cloned().collect()),
            Value::Vector(values) => Some(values.iter().cloned().collect()),
            _ => None,
        }
    }
    Some(items(left)? == items(right)?)
}

pub(crate) fn map_entries(value: &Value) -> Option<Vec<(Value, Value)>> {
    match value {
        Value::Map(values) => Some(values.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
        Value::OrderedMap(values) => {
            Some(values.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        }
        Value::SortedMap(values) => {
            Some(values.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        }
        Value::Trie(values) => Some(
            values
                .entries()
                .into_iter()
                .map(|(k, v)| (Value::String(k), v.clone()))
                .collect(),
        ),
        _ => None,
    }
}

fn map_value<'a>(value: &'a Value, key: &Value) -> Option<&'a Value> {
    match value {
        Value::Map(values) => values.get(key),
        Value::OrderedMap(values) => values.get(key),
        Value::SortedMap(values) => values.get(key),
        Value::Trie(values) => match key {
            Value::String(key) => values.get(key),
            _ => None,
        },
        _ => None,
    }
}

fn map_equality(left: &Value, right: &Value) -> Option<bool> {
    let left_entries = map_entries(left)?;
    let right_entries = map_entries(right)?;
    Some(
        left_entries.len() == right_entries.len()
            && left_entries
                .iter()
                .all(|(key, value)| map_value(right, key) == Some(value)),
    )
}

fn set_items(value: &Value) -> Option<Vec<&Value>> {
    match value {
        Value::Set(values) => Some(values.iter().collect()),
        Value::OrderedSet(values) => Some(values.iter().collect()),
        Value::SortedSet(values) => Some(values.iter().collect()),
        _ => None,
    }
}

fn set_equality(left: &Value, right: &Value) -> Option<bool> {
    let left_items = set_items(left)?;
    let right_items = set_items(right)?;
    Some(
        left_items.len() == right_items.len()
            && left_items.iter().all(|item| right_items.contains(item)),
    )
}

fn map_assoc_value(collection: &Value, key: Value, value: Value) -> Result<Value, String> {
    Ok(match collection {
        Value::Map(values) => Value::Map(values.assoc_value(key, value)),
        Value::OrderedMap(values) => Value::OrderedMap(Box::new(values.assoc_value(key, value))),
        Value::SortedMap(values) => Value::SortedMap(Box::new(values.assoc_value(key, value))),
        Value::Trie(values) => match key {
            Value::String(key) => Value::Trie(Box::new(values.assoc_value(key, value))),
            _ => return Err("trie expects string keys".into()),
        },
        _ => return Err("assoc expects a map".into()),
    })
}

fn map_dissoc_value(collection: &Value, key: &Value) -> Result<Value, String> {
    Ok(match collection {
        Value::Map(values) => Value::Map(values.dissoc_value(key)),
        Value::OrderedMap(values) => Value::OrderedMap(Box::new(values.dissoc_value(key))),
        Value::SortedMap(values) => Value::SortedMap(Box::new(values.dissoc_value(key))),
        Value::Trie(values) => match key {
            Value::String(key) => Value::Trie(Box::new(values.dissoc_value(key))),
            _ => return Err("trie expects string keys".into()),
        },
        _ => return Err("dissoc expects a map".into()),
    })
}

fn set_find(collection: &Value, key: &Value) -> Option<Value> {
    set_items(collection)?
        .into_iter()
        .find(|value| *value == key)
        .cloned()
}

fn set_conj_value(collection: &Value, value: Value) -> Result<Value, String> {
    Ok(match collection {
        Value::Set(values) => Value::Set(values.conj_value(value)),
        Value::OrderedSet(values) => Value::OrderedSet(Box::new(values.conj_value(value))),
        Value::SortedSet(values) => Value::SortedSet(Box::new(values.conj_value(value))),
        _ => return Err("conj expects a set".into()),
    })
}

fn set_dissoc_value(collection: &Value, value: &Value) -> Result<Value, String> {
    Ok(match collection {
        Value::Set(values) => Value::Set(values.dissoc_value(value)),
        Value::OrderedSet(values) => Value::OrderedSet(Box::new(values.dissoc_value(value))),
        Value::SortedSet(values) => Value::SortedSet(Box::new(values.dissoc_value(value))),
        _ => return Err("dissoc expects a set".into()),
    })
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        if let Some(equal) = sequential_equality(self, other) {
            return equal;
        }
        if let Some(equal) = map_equality(self, other) {
            return equal;
        }
        if let Some(equal) = set_equality(self, other) {
            return equal;
        }
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a.to_bits() == b.to_bits(),
            (Value::BigInteger(a), Value::BigInteger(b)) => a == b,
            (Value::Decimal(a), Value::Decimal(b)) => a == b,
            (Value::Character(a), Value::Character(b)) => a == b,
            (Value::Regex(a), Value::Regex(b)) => a == b,
            (Value::Tagged(a), Value::Tagged(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Keyword(a), Value::Keyword(b)) => a == b,
            (Value::Bytes(a), Value::Bytes(b)) => a == b,
            (Value::ByteBuffer(a), Value::ByteBuffer(b)) => *a.borrow() == *b.borrow(),
            (Value::Array(a), Value::Array(b)) => Rc::ptr_eq(a, b),
            (Value::Object(a), Value::Object(b)) => Rc::ptr_eq(a, b),
            (Value::Promise(a), Value::Promise(b)) => a.same_identity(b),
            (Value::Atom(a), Value::Atom(b)) => a.same_identity(b),
            (Value::Recur(a), Value::Recur(b)) => a == b,
            (Value::Map(a), Value::Map(b)) => a == b,
            (Value::Set(a), Value::Set(b)) => a == b,
            (Value::List(a), Value::List(b)) => a == b,
            (Value::Cons(a), Value::Cons(b)) => a == b,
            (Value::Symbol(a), Value::Symbol(b)) => a == b,
            (Value::Pointer(a), Value::Pointer(b)) => a == b,
            (Value::Function(a), Value::Function(b)) => Rc::ptr_eq(a, b),
            (Value::Tuple(a), Value::Tuple(b)) => a == b,
            (Value::Vector(a), Value::Vector(b)) => a == b,
            (Value::Iterator(a), Value::Iterator(b)) => Rc::ptr_eq(a, b),
            (Value::Var(a), Value::Var(b)) => a.same_identity(b),
            (Value::Namespace(a), Value::Namespace(b)) => a.same_identity(b),
            (Value::Extension(a), Value::Extension(b)) => a == b,
            (Value::StructType(a), Value::StructType(b)) => Rc::ptr_eq(a, b),
            (Value::Struct(a), Value::Struct(b)) => Rc::ptr_eq(a, b),
            (Value::Protocol(a), Value::Protocol(b)) => Rc::ptr_eq(a, b),
            (Value::NativeType(a), Value::NativeType(b)) => a.name == b.name,
            (Value::Coroutine(a), Value::Coroutine(b)) => Rc::ptr_eq(a, b),
            (Value::ExceptionInfo(a), Value::ExceptionInfo(b)) => Rc::ptr_eq(a, b),
            (Value::Nil, Value::Nil) => true,
            _ => false,
        }
    }
}

impl Eq for Value {}
impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Value {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self == other {
            return std::cmp::Ordering::Equal;
        }
        match (self, other) {
            (Value::Number(left), Value::Number(right)) => return left.cmp(right),
            (Value::Float(left), Value::Float(right)) => return left.total_cmp(right),
            (Value::Character(left), Value::Character(right)) => return left.cmp(right),
            (Value::Bool(left), Value::Bool(right)) => return left.cmp(right),
            (Value::String(left), Value::String(right)) => return left.cmp(right),
            (Value::Keyword(left), Value::Keyword(right)) => return left.cmp(right),
            (Value::BigInteger(left), Value::BigInteger(right))
            | (Value::Decimal(left), Value::Decimal(right)) => return left.cmp(right),
            _ => {}
        }
        fn rank(value: &Value) -> u8 {
            match value {
                Value::Nil => 0,
                Value::Bool(_) => 1,
                Value::Number(_) => 2,
                Value::Float(_) => 3,
                Value::BigInteger(_) => 4,
                Value::Decimal(_) => 5,
                Value::Character(_) => 6,
                Value::String(_) => 7,
                Value::Keyword(_) => 8,
                Value::Symbol(_) => 9,
                Value::Pointer(_) => 9,
                Value::List(_)
                | Value::Cons(_)
                | Value::Queue(_)
                | Value::Tuple(_)
                | Value::Vector(_) => 10,
                Value::Map(_) | Value::OrderedMap(_) | Value::SortedMap(_) | Value::Trie(_) => 11,
                Value::Set(_) | Value::OrderedSet(_) | Value::SortedSet(_) => 12,
                Value::Bytes(_) => 13,
                Value::ByteBuffer(_) => 14,
                Value::Regex(_) => 15,
                Value::Tagged(_) => 16,
                Value::Array(_) => 17,
                Value::Object(_) => 18,
                Value::Promise(_) => 19,
                Value::Atom(_) => 26,
                Value::Recur(_) => 20,
                Value::Function(_) => 21,
                Value::Iterator(_) => 22,
                Value::Var(_) => 23,
                Value::Namespace(_) => 24,
                Value::Extension(_) => 25,
                Value::StructType(_) => 27,
                Value::Struct(_) => 28,
                Value::Protocol(_) => 29,
                Value::NativeType(_) => 30,
                Value::Coroutine(_) => 31,
                Value::ExceptionInfo(_) => 32,
            }
        }
        rank(self)
            .cmp(&rank(other))
            .then_with(|| self.display().cmp(&other.display()))
            .then_with(|| self.stable_hash().cmp(&other.stable_hash()))
    }
}
impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.stable_hash());
    }
}

impl Value {
    pub fn display(&self) -> String {
        match self {
            Self::Number(v) => v.to_string(),
            Self::Float(v) if v.is_nan() => "##NaN".into(),
            Self::Float(v) if *v == f64::INFINITY => "##Inf".into(),
            Self::Float(v) if *v == f64::NEG_INFINITY => "##-Inf".into(),
            Self::Float(v) => v.to_string(),
            Self::BigInteger(v) => format!("{v}N"),
            Self::Decimal(v) => format!("{v}M"),
            Self::Character('\n') => "\\newline".into(),
            Self::Character(' ') => "\\space".into(),
            Self::Character('\t') => "\\tab".into(),
            Self::Character('\u{0008}') => "\\backspace".into(),
            Self::Character('\u{000c}') => "\\formfeed".into(),
            Self::Character('\r') => "\\return".into(),
            Self::Character(v) if v.is_control() => format!("\\u{:04X}", *v as u32),
            Self::Character(v) => format!("\\{v}"),
            Self::Regex(v) => crate::kernel::form::display_regex(v),
            Self::Tagged(value) => format!("#{}{}", value.tag().as_str(), value.form().display()),
            Self::Bool(v) => v.to_string(),
            Self::String(v) => crate::kernel::form::display_string(v),
            Self::Keyword(v) => format!(":{}", v.as_str()),
            Self::Bytes(values) => format!(
                "#bytes[{}]",
                values
                    .iter()
                    .map(|v| (*v as i8).to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            Self::ByteBuffer(values) => {
                let body = values
                    .borrow()
                    .iter()
                    .map(|v| (*v as i8).to_string())
                    .collect::<Vec<_>>()
                    .join(" ");
                if body.is_empty() {
                    "(bytes)".into()
                } else {
                    format!("(bytes {body})")
                }
            }
            Self::Array(values) => format!(
                "(array {})",
                values
                    .borrow()
                    .iter()
                    .map(Value::display)
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            Self::Object(values) => format!(
                "(object {})",
                values
                    .borrow()
                    .iter()
                    .map(|(key, value)| format!("\"{}\" {}", key, value.display()))
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            Self::Promise(_) => "<promise>".into(),
            Self::Atom(value) => format!("#atom <{}>", value.deref_value().display()),
            Self::Recur(values) => format!(
                "<recur {}>",
                values
                    .iter()
                    .map(Value::display)
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            value @ (Self::Map(_) | Self::OrderedMap(_) | Self::SortedMap(_) | Self::Trie(_)) => {
                format!(
                    "{{{}}}",
                    map_entries(value)
                        .unwrap()
                        .iter()
                        .map(|(k, v)| format!("{} {}", k.display(), v.display()))
                        .collect::<Vec<_>>()
                        .join(" ")
                )
            }
            value @ (Self::Set(_) | Self::OrderedSet(_) | Self::SortedSet(_)) => format!(
                "#{{{}}}",
                set_items(value)
                    .unwrap()
                    .iter()
                    .map(|item| item.display())
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            Self::Queue(values) => format!(
                "#queue[{}]",
                values
                    .iter()
                    .map(Value::display)
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            Self::Cons(values) => format!(
                "({})",
                values
                    .iter()
                    .map(|value| value.display())
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            Self::List(values) => format!(
                "({})",
                values
                    .iter()
                    .map(Value::display)
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            Self::Symbol(v) => v.as_str().to_owned(),
            Self::Pointer(v) => v.display(),
            Self::Function(_) => "<fn>".into(),
            Self::Tuple(values) => format!(
                "[{}]",
                values
                    .iter()
                    .map(Value::display)
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            Self::Vector(values) => format!(
                "[{}]",
                values
                    .iter()
                    .map(Value::display)
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            Self::Iterator(iterator) => {
                if iterator.borrow().seq {
                    "<seq>".into()
                } else {
                    "<iterator>".into()
                }
            }
            Self::Var(value) => value.display(),
            Self::Namespace(value) => format!("#namespace[{}]", value.name().as_str()),
            Self::Extension(value) => format!("#ht[:handle {}]", value.handle),
            Self::StructType(value) => value.name.clone(),
            Self::Struct(value) => format!(
                "#{}{{{}}}",
                value.ty.name,
                value
                    .ty
                    .fields
                    .iter()
                    .zip(&value.values)
                    .map(|(field, value)| format!(":{field} {}", value.display()))
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            Self::Protocol(value) => format!("#protocol[{}]", value.name),
            Self::NativeType(value) => format!("#<native-type {}>", value.name),
            Self::Coroutine(value) => {
                let status = match &*value.state.borrow() {
                    CoroutineState::New(_) | CoroutineState::Suspended(_) => "suspended",
                    CoroutineState::Running => "running",
                    CoroutineState::Dead => "dead",
                };
                format!("#<coroutine {status}>")
            }
            Self::ExceptionInfo(value) => {
                format!("#error[{} {}]", Self::String(value.message.clone()).display(), value.data.display())
            }
            Self::Nil => "nil".into(),
        }
    }
    fn truthy(&self) -> bool {
        !matches!(self, Self::Nil | Self::Bool(false))
    }

    /// Stable structural hash used by protocol and collection conformance tests.
    pub fn stable_hash(&self) -> u64 {
        fn hash_value(value: &Value, state: &mut std::collections::hash_map::DefaultHasher) {
            let type_tag: u64 = match value {
                Value::Number(_) => 0,
                Value::Bool(_) => 1,
                Value::String(_) => 2,
                Value::Keyword(_) => 3,
                Value::Bytes(_) => 4,
                Value::ByteBuffer(_) => 5,
                Value::Array(_) => 6,
                Value::Object(_) => 7,
                Value::Promise(_) => 8,
                Value::Atom(_) => 28,
                Value::Recur(_) => 9,
                Value::Map(_) | Value::OrderedMap(_) | Value::SortedMap(_) | Value::Trie(_) => 10,
                Value::Set(_) | Value::OrderedSet(_) | Value::SortedSet(_) => 11,
                Value::List(_)
                | Value::Cons(_)
                | Value::Queue(_)
                | Value::Tuple(_)
                | Value::Vector(_) => 12,
                Value::Symbol(_) => 13,
                Value::Pointer(_) => 13,
                Value::Function(_) => 14,
                Value::Iterator(_) => 16,
                Value::Var(_) => 17,
                Value::Namespace(_) => 27,
                Value::Extension(_) => 18,
                Value::StructType(_) => 26,
                Value::Struct(_) => 27,
                Value::Protocol(_) => 28,
                Value::NativeType(_) => 29,
                Value::Coroutine(_) => 30,
                Value::ExceptionInfo(_) => 31,
                Value::Nil => 19,
                Value::Float(_) => 20,
                Value::BigInteger(_) => 21,
                Value::Decimal(_) => 22,
                Value::Character(_) => 23,
                Value::Regex(_) => 24,
                Value::Tagged(_) => 25,
            };
            type_tag.hash(state);
            match value {
                Value::Number(v) => v.hash(state),
                Value::Float(v) => v.to_bits().hash(state),
                Value::BigInteger(v) | Value::Decimal(v) | Value::Regex(v) => v.hash(state),
                Value::Character(v) => v.hash(state),
                Value::Tagged(value) => {
                    value.tag().hash(state);
                    hash_value(value.form(), state);
                }
                Value::Bool(v) => v.hash(state),
                Value::String(v) => v.hash(state),
                Value::Keyword(v) => v.hash(state),
                Value::Symbol(v) => v.hash(state),
                Value::Pointer(v) => v.hash(state),
                Value::Bytes(v) => v.hash(state),
                Value::ByteBuffer(v) => v.borrow().hash(state),
                Value::Array(v) => v.borrow().iter().for_each(|item| hash_value(item, state)),
                Value::Object(v) => v.borrow().iter().for_each(|(key, item)| {
                    key.hash(state);
                    hash_value(item, state);
                }),
                Value::Promise(v) => v.identity_address().hash(state),
                Value::Atom(v) => v.identity_address().hash(state),
                Value::Recur(v) => v.iter().for_each(|item| hash_value(item, state)),
                value @ (Value::Map(_)
                | Value::OrderedMap(_)
                | Value::SortedMap(_)
                | Value::Trie(_)) => {
                    let mut entries = map_entries(value)
                        .unwrap()
                        .iter()
                        .map(|(key, item)| {
                            let mut h = std::collections::hash_map::DefaultHasher::new();
                            hash_value(key, &mut h);
                            hash_value(item, &mut h);
                            h.finish()
                        })
                        .collect::<Vec<_>>();
                    entries.sort_unstable();
                    entries.hash(state);
                }
                value @ (Value::Set(_) | Value::OrderedSet(_) | Value::SortedSet(_)) => {
                    let mut entries = set_items(value)
                        .unwrap()
                        .iter()
                        .map(|item| {
                            let mut h = std::collections::hash_map::DefaultHasher::new();
                            hash_value(item, &mut h);
                            h.finish()
                        })
                        .collect::<Vec<_>>();
                    entries.sort_unstable();
                    entries.hash(state);
                }
                Value::List(v) => v.iter().for_each(|item| hash_value(item, state)),
                Value::Cons(v) => v.iter().for_each(|item| hash_value(&item, state)),
                Value::Queue(v) => v.iter().for_each(|item| hash_value(item, state)),
                Value::Tuple(v) => v.iter().for_each(|item| hash_value(item, state)),
                Value::Vector(v) => v.iter().for_each(|item| hash_value(item, state)),
                Value::Function(v) => Rc::as_ptr(v).hash(state),
                Value::Iterator(v) => Rc::as_ptr(v).hash(state),
                Value::Var(v) => v.identity_address().hash(state),
                Value::Namespace(v) => v.identity_address().hash(state),
                Value::Extension(v) => {
                    v.provider.hash(state);
                    v.type_name.hash(state);
                    v.handle.hash(state);
                }
                Value::StructType(v) => v.name.hash(state),
                Value::Struct(v) => Rc::as_ptr(v).hash(state),
                Value::Protocol(v) => v.name.hash(state),
                Value::NativeType(v) => v.name.hash(state),
                Value::Coroutine(v) => {
                    Rc::as_ptr(v).hash(state);
                }
                Value::ExceptionInfo(v) => Rc::as_ptr(v).hash(state),
                Value::Nil => {}
            }
        }
        let mut state = std::collections::hash_map::DefaultHasher::new();
        hash_value(self, &mut state);
        state.finish()
    }
}

pub type ProtocolFn = Rc<dyn Fn(&[Value]) -> Result<Value, String>>;

#[derive(Default, Clone)]
pub struct ProtocolRegistry {
    methods: Rc<RefCell<HashMap<(String, String), Vec<ProtocolFn>>>>,
    guest_methods: Rc<RefCell<HashMap<(String, String, String), Rc<Function>>>>,
    guest_declarations: Rc<RefCell<HashSet<(String, String)>>>,
}

#[allow(dead_code)]
impl ProtocolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<F>(
        &mut self,
        protocol: impl Into<String>,
        method: impl Into<String>,
        function: F,
    ) where
        F: Fn(&[Value]) -> Result<Value, String> + 'static,
    {
        let protocol = canonical_protocol_name(&protocol.into());
        self.methods
            .borrow_mut()
            .entry((protocol, method.into()))
            .or_default()
            .push(Rc::new(function));
    }

    pub fn register_guest(
        &self,
        protocol: impl Into<String>,
        type_name: impl Into<String>,
        method: impl Into<String>,
        function: Rc<Function>,
    ) {
        self.guest_methods
            .borrow_mut()
            .insert((protocol.into(), type_name.into(), method.into()), function);
    }

    pub fn declare_guest(&self, protocol: impl Into<String>, method: impl Into<String>) {
        self.guest_declarations
            .borrow_mut()
            .insert((protocol.into(), method.into()));
    }

    pub fn invoke(
        &self,
        protocol: &str,
        method: &str,
        arguments: &[Value],
    ) -> Result<Value, String> {
        let qualified;
        let protocol = if protocol.contains('/') {
            qualified = canonical_protocol_name(protocol);
            qualified.as_str()
        } else if self
            .methods
            .borrow()
            .contains_key(&(protocol.to_owned(), method.to_owned()))
            || self
                .guest_declarations
                .borrow()
                .contains(&(protocol.to_owned(), method.to_owned()))
        {
            protocol
        } else {
            qualified = canonical_protocol_name(protocol);
            qualified.as_str()
        };
        if let Some(Value::Struct(receiver)) = arguments.first() {
            if let Some(function) = self.guest_methods.borrow().get(&(
                protocol.to_owned(),
                receiver.ty.name.clone(),
                method.to_owned(),
            )) {
                return call_function(function, arguments.to_vec());
            }
            if FOUNDATION_PROTOCOLS
                .iter()
                .any(|(name, _)| builtin_protocol_name(name) == protocol)
            {
                return Err(format!(
                    "protocol/unsupported-receiver: missing protocol implementation: {protocol}/{method}"
                ));
            }
        }
        if self
            .guest_declarations
            .borrow()
            .contains(&(protocol.to_owned(), method.to_owned()))
        {
            return Err(format!(
                "missing protocol implementation: {protocol}/{method}"
            ));
        }
        let methods = self.methods.borrow();
        let implementations = methods
            .get(&(protocol.to_string(), method.to_string()))
            .ok_or_else(|| format!("missing protocol method: {protocol}/{method}"))?;
        let mut last_error = format!("missing protocol implementation: {protocol}/{method}");
        for implementation in implementations {
            match implementation(arguments) {
                Ok(value) => return Ok(value),
                Err(error) => last_error = error,
            }
        }
        Err(last_error)
    }

    pub fn contains(&self, protocol: &str, method: &str) -> bool {
        let protocol = canonical_protocol_name(protocol);
        let methods = self.methods.borrow();
        methods
            .get(&(protocol, method.to_string()))
            .is_some_and(|implementations| !implementations.is_empty())
    }

    /// Returns the built-in collection protocol registry used by evaluator dispatch.
    pub fn core() -> Self {
        let mut registry = Self::new();
        for (protocol, methods) in FOUNDATION_PROTOCOLS {
            for (method, _) in *methods {
                let protocol_name = builtin_protocol_name(protocol);
                let method_name = (*method).to_owned();
                let missing_protocol = protocol_name.clone();
                let missing_method = method_name.clone();
                registry.register(protocol_name, method_name, move |_| {
                    Err(format!(
                        "missing protocol implementation: {missing_protocol}/{missing_method}"
                    ))
                });
            }
        }
        registry.register("std.protocol.icount/ICount", "count", protocol_count);
        registry.register("std.protocol.inth/INth", "nth", protocol_nth);
        registry.register("std.protocol.ilookup/ILookup", "lookup", protocol_lookup);
        registry.register("std.protocol.ifind/IFind", "find", protocol_find);
        registry.register("std.protocol.iassoc/IAssoc", "assoc", protocol_assoc);
        registry.register("std.protocol.iconj/IConj", "conj", protocol_conj);
        registry.register("std.protocol.icons/ICons", "cons", protocol_cons);
        registry.register("std.protocol.idissoc/IDissoc", "dissoc", protocol_dissoc);
        registry.register("std.protocol.iempty/IEmpty", "empty", protocol_empty);
        registry.register(
            "std.protocol.iequality/IEquality",
            "equality",
            protocol_equality,
        );
        registry.register(
            "std.protocol.idisplay/IDisplay",
            "display",
            protocol_display,
        );
        registry.register(
            "std.protocol.iexinfo/IExInfo",
            "data",
            |arguments| match arguments {
                [Value::ExceptionInfo(value)] => Ok((*value.data).clone()),
                [_] => Err("missing protocol implementation: std.protocol.iexinfo/IExInfo/data".into()),
                _ => Err("IExInfo/data expects one argument".into()),
            },
        );
        registry.register("std.protocol.ihash/IHash", "hash", protocol_hash);
        registry.register("std.protocol.ifn/IFn", "invoke", protocol_invoke);
        registry.register("std.protocol.ipair/IPair", "key", protocol_pair_key);
        registry.register("std.protocol.ipair/IPair", "value", protocol_pair_value);
        registry.register(
            "std.protocol.ipeekfirst/IPeekFirst",
            "peek-first",
            protocol_peek_first,
        );
        registry.register(
            "std.protocol.ipeeklast/IPeekLast",
            "peek-last",
            protocol_peek_last,
        );
        registry.register("std.protocol.iiter/IIter", "iter", protocol_iter);
        registry.register(
            "std.protocol.iiterator/IIterator",
            "iter-next?",
            |arguments| {
                arguments
                    .first()
                    .ok_or_else(|| "IIterator/iter-next? expects one argument".to_string())
                    .and_then(iterator_has_next)
            },
        );
        registry.register(
            "std.protocol.iiterator/IIterator",
            "iter-next",
            |arguments| {
                arguments
                    .first()
                    .ok_or_else(|| "IIterator/iter-next expects one argument".to_string())
                    .and_then(iterator_next)
            },
        );
        registry.register(
            "std.protocol.iclose/IClose",
            "close",
            |arguments| match arguments {
                [Value::Coroutine(coroutine)] => {
                    coroutine_close(coroutine)?;
                    Ok(Value::Coroutine(coroutine.clone()))
                }
                [value] => iterator_close(value),
                _ => Err("IClose/close expects one argument".into()),
            },
        );
        registry.register(
            "std.protocol.inamespaced/INamespaced",
            "name",
            protocol_namespaced_name,
        );
        registry.register(
            "std.protocol.inamespaced/INamespaced",
            "namespace",
            protocol_namespaced_namespace,
        );
        registry.register("std.protocol.iobjtype/IObjType", "meta", protocol_meta);
        registry.register(
            "std.protocol.iobjtype/IObjType",
            "with-meta",
            protocol_with_meta,
        );
        registry.register("std.protocol.ideref/IDeref", "deref", protocol_deref);
        registry.register("std.protocol.ireset/IReset", "reset", protocol_reset);
        registry.register("std.protocol.icas/ICas", "cas", protocol_cas);
        registry.register("std.protocol.ireduce/IReduce", "reduce", protocol_reduce);
        registry.register(
            "std.protocol.ipromise/IPromise",
            "state",
            protocol_promise_state,
        );
        registry.register(
            "std.protocol.ipromise/IPromise",
            "value",
            protocol_promise_value,
        );
        registry.register("std.protocol.ipromise/IPromise", "then", |arguments| {
            protocol_promise_chain("promise/then", arguments)
        });
        registry.register("std.protocol.ipromise/IPromise", "catch", |arguments| {
            protocol_promise_chain("promise/catch", arguments)
        });
        registry.register("std.protocol.ipromise/IPromise", "finally", |arguments| {
            protocol_promise_chain("promise/finally", arguments)
        });
        registry.register(
            "std.protocol.ipromise/IPromise",
            "cancel",
            protocol_promise_cancel,
        );
        registry.register(
            "std.protocol.icoroutine/ICoroutine",
            "status",
            protocol_coroutine_status,
        );
        registry.register(
            "std.protocol.iwatch/IWatch",
            "watch-add",
            protocol_watch_add,
        );
        registry.register(
            "std.protocol.iwatch/IWatch",
            "watch-remove",
            protocol_watch_remove,
        );
        registry.register(
            "std.protocol.iwatch/IWatch",
            "watch-list",
            protocol_watch_list,
        );
        registry
    }
}

thread_local! {
    static ACTIVE_PROTOCOLS: RefCell<Option<ProtocolRegistry>> = const { RefCell::new(None) };
    static ACTIVE_NAMESPACES: RefCell<Option<NamespaceRegistry<Value>>> = const { RefCell::new(None) };
    static ACTIVE_DEFINITION_ORIGIN: Cell<VarOrigin> = const { Cell::new(VarOrigin::Source) };
    static ACTIVE_PROMISE_PROVIDER: RefCell<Option<Rc<dyn PromiseProvider>>> = const { RefCell::new(None) };
    static ACTIVE_FILE_PROVIDER: RefCell<Option<Rc<dyn FileProvider>>> = const { RefCell::new(None) };
    static ACTIVE_SOCKET_PROVIDER: RefCell<Option<Rc<dyn SocketProvider>>> = const { RefCell::new(None) };
    static HOST_CALL_HANDLER: RefCell<Option<Rc<dyn Fn(String, String, Vec<Value>) -> Result<Value, String>>>> = const { RefCell::new(None) };
    static NAMESPACE_SOURCE_PROVIDER: RefCell<Option<Rc<dyn Fn(&str) -> Option<String>>>> = const { RefCell::new(None) };
    static ACTIVE_THROWN_VALUE: RefCell<Option<(String, Value)>> = const { RefCell::new(None) };
}

pub(crate) fn thrown_error(value: Value) -> String {
    let error = format!("thrown: {}", value.display());
    ACTIVE_THROWN_VALUE.with(|active| {
        *active.borrow_mut() = Some((error.clone(), value));
    });
    error
}

pub(crate) fn caught_error(error: &str) -> Value {
    ACTIVE_THROWN_VALUE.with(|active| {
        let mut active = active.borrow_mut();
        if active
            .as_ref()
            .is_some_and(|(thrown_error, _)| thrown_error == error)
        {
            return active.take().unwrap().1;
        }
        Value::String(error.to_owned())
    })
}

/// Runs an evaluation with a namespace registry available to namespace builtins.
pub fn with_namespace_registry<R>(
    registry: &NamespaceRegistry<Value>,
    operation: impl FnOnce() -> R,
) -> R {
    ACTIVE_NAMESPACES.with(|active| {
        let previous = active.replace(Some(registry.clone()));
        let result = operation();
        active.replace(previous);
        result
    })
}

pub fn with_definition_origin<R>(origin: VarOrigin, operation: impl FnOnce() -> R) -> R {
    ACTIVE_DEFINITION_ORIGIN.with(|active| {
        let previous = active.replace(origin);
        let result = operation();
        active.set(previous);
        result
    })
}

pub(crate) fn definition_origin() -> VarOrigin {
    ACTIVE_DEFINITION_ORIGIN.with(Cell::get)
}

pub(crate) fn binding_is_local(var: &KernelVar<Value>) -> bool {
    namespace_registry()
        .map(|registry| var.symbol().get_namespace() == Some(registry.current().name().as_str()))
        .unwrap_or(true)
}

pub(crate) fn protected_fallback_binding(
    env: &HashMap<String, Value>,
    name: &str,
) -> Option<Value> {
    if definition_origin() != VarOrigin::HalFallback {
        return None;
    }
    match env.get(name) {
        Some(Value::Var(var))
            if matches!(
                var.origin(),
                VarOrigin::RustLibrary | VarOrigin::RuntimePrimitive
            ) =>
        {
            Some(var.deref_value())
        }
        _ => None,
    }
}

fn namespace_registry() -> Result<NamespaceRegistry<Value>, String> {
    ACTIVE_NAMESPACES
        .with(|active| active.borrow().clone())
        .ok_or_else(|| "namespace runtime is unavailable".into())
}

/// Saves all unqualified evaluator bindings into the registry current namespace.
pub fn save_namespace_environment(
    registry: &NamespaceRegistry<Value>,
    env: &mut HashMap<String, Value>,
) {
    let namespace = registry.current();
    let namespace_name = namespace.name().as_str().to_owned();
    let locals = env
        .iter()
        .filter(|(name, _)| !name.contains('/'))
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect::<Vec<_>>();
    for (name, value) in locals {
        let path = format!("{namespace_name}/{name}");
        if matches!(&value, Value::Var(var) if var.symbol().get_namespace().is_some() && var.symbol().get_namespace() != Some(namespace_name.as_str()))
        {
            continue;
        }
        let var = match value {
            Value::Var(var) if var.symbol().as_str() == path => var,
            Value::Var(var) => var.requalify(&path),
            value => namespace.intern(&name, value),
        };
        namespace.map_var(crate::lang::data::Symbol::parse(&name), var.clone());
        env.insert(name, Value::Var(var));
    }
}

/// Rebuilds qualified and aliased bindings without changing local bindings.
pub fn refresh_namespace_environment(
    registry: &NamespaceRegistry<Value>,
    env: &mut HashMap<String, Value>,
) {
    env.retain(|name, _| !name.contains('/'));
    for namespace in registry.all() {
        for (_, var) in namespace.mappings() {
            env.insert(var.symbol().as_str().to_owned(), Value::Var(var));
        }
    }
    for (alias, namespace) in registry.current().aliases() {
        for (local, var) in namespace.mappings() {
            env.insert(
                format!("{}/{}", alias.as_str(), local.as_str()),
                Value::Var(var),
            );
        }
    }
}

/// Saves the current namespace, selects name, and loads its bindings.
pub fn select_namespace_environment(
    registry: &NamespaceRegistry<Value>,
    env: &mut HashMap<String, Value>,
    name: &str,
) {
    save_namespace_environment(registry, env);
    let namespace = registry.set_current(name);
    *env = namespace
        .mappings()
        .into_iter()
        .map(|(name, var)| (name.as_str().to_owned(), Value::Var(var)))
        .collect();
    refresh_namespace_environment(registry, env);
}

pub(crate) fn refer_startup_defaults(
    registry: &NamespaceRegistry<Value>,
    namespace: &str,
) {
    let target = registry.find_or_create(namespace);
    if namespace != "std.foundation" {
        if let Some(foundation) = registry.find("std.foundation") {
            for (name, var) in foundation.mappings() {
                if target.resolve(&name).is_none() {
                    target.map_var(name, var);
                }
            }
        }
    }
    for (protocol, _) in FOUNDATION_PROTOCOLS {
        let protocol_namespace = builtin_protocol_namespace(protocol);
        if let Some(source) = registry.find(&protocol_namespace) {
            target.alias(protocol, source);
        }
    }
    if let Some(edn) = registry.find("std.foundation.edn") {
        target.alias("edn", edn);
    }
}

/// Runs an evaluation with a registry available to protocol dispatch.
pub fn with_protocols<R>(registry: &ProtocolRegistry, operation: impl FnOnce() -> R) -> R {
    ACTIVE_PROTOCOLS.with(|active| {
        let previous = active.replace(Some(registry.clone()));
        let result = operation();
        active.replace(previous);
        result
    })
}

/// Runs an evaluation through the selected runtime promise provider.
pub fn with_promise_provider<R>(
    provider: Rc<dyn PromiseProvider>,
    operation: impl FnOnce() -> R,
) -> R {
    ACTIVE_PROMISE_PROVIDER.with(|active| {
        let previous = active.replace(Some(provider));
        let result = operation();
        active.replace(previous);
        result
    })
}

fn promise_provider() -> Rc<dyn PromiseProvider> {
    ACTIVE_PROMISE_PROVIDER.with(|active| {
        active
            .borrow()
            .clone()
            .unwrap_or_else(|| Rc::new(LocalPromiseProvider))
    })
}
/// Runs an evaluation through the selected runtime capability providers.
pub fn with_capability_providers<R>(
    file: Option<Rc<dyn FileProvider>>,
    socket: Option<Rc<dyn SocketProvider>>,
    operation: impl FnOnce() -> R,
) -> R {
    ACTIVE_FILE_PROVIDER.with(|active_file| {
        ACTIVE_SOCKET_PROVIDER.with(|active_socket| {
            let previous_file = active_file.replace(file);
            let previous_socket = active_socket.replace(socket);
            let result = operation();
            active_file.replace(previous_file);
            active_socket.replace(previous_socket);
            result
        })
    })
}

fn file_provider(operation: &str) -> Result<Rc<dyn FileProvider>, String> {
    ACTIVE_FILE_PROVIDER.with(|active| {
        active
            .borrow()
            .clone()
            .ok_or_else(|| format!("{operation} is unsupported or file access is denied"))
    })
}

fn socket_provider(operation: &str) -> Result<Rc<dyn SocketProvider>, String> {
    ACTIVE_SOCKET_PROVIDER.with(|active| {
        active
            .borrow()
            .clone()
            .ok_or_else(|| format!("{operation} is unsupported or network access is denied"))
    })
}

fn file_error(operation: &str, error: FileError) -> String {
    format!("{operation} failed: file/{}", error.code())
}

fn socket_error(operation: &str, error: SocketError) -> String {
    format!("{operation} failed: socket/{}", error.code())
}

fn file_operation(
    operation: &str,
    forms: &[Form],
    env: &mut HashMap<String, Value>,
) -> Result<Value, String> {
    match operation {
        "file/resolve" => {
            if forms.len() != 2 {
                return Err("file/resolve expects a root and path".into());
            }
            let root = match eval(&forms[0], env)? {
                Value::String(value) => value,
                _ => return Err("file/resolve expects a root and path".into()),
            };
            let path = match eval(&forms[1], env)? {
                Value::String(value) => value,
                _ => return Err("file/resolve expects a root and path".into()),
            };
            file_provider(operation)?
                .resolve(&root, &path)
                .map(Value::String)
                .map_err(|error| file_error(operation, error))
        }
        "file/read" => {
            if forms.len() != 1 {
                return Err("file/read expects a path".into());
            }
            let path = match eval(&forms[0], env)? {
                Value::String(value) => value,
                _ => return Err("file/read expects a path".into()),
            };
            file_provider(operation)?
                .read(&path)
                .map(Value::Promise)
                .map_err(|error| file_error(operation, error))
        }
        "file/write" => {
            if forms.len() != 2 {
                return Err("file/write expects a path and bytes".into());
            }
            let path = match eval(&forms[0], env)? {
                Value::String(value) => value,
                _ => return Err("file/write expects a path and bytes".into()),
            };
            let bytes = match eval(&forms[1], env)? {
                Value::Bytes(value) => value,
                Value::ByteBuffer(value) => value.borrow().clone(),
                _ => return Err("file/write expects a path and bytes".into()),
            };
            file_provider(operation)?
                .write(&path, bytes)
                .map(Value::Promise)
                .map_err(|error| file_error(operation, error))
        }
        "file/exists?" => {
            if forms.len() != 1 {
                return Err("file/exists? expects a path".into());
            }
            let path = match eval(&forms[0], env)? {
                Value::String(value) => value,
                _ => return Err("file/exists? expects a path".into()),
            };
            file_provider(operation)?
                .exists(&path)
                .map(Value::Promise)
                .map_err(|error| file_error(operation, error))
        }
        "file/list" => {
            if forms.len() != 1 {
                return Err("file/list expects a path".into());
            }
            let path = match eval(&forms[0], env)? {
                Value::String(value) => value,
                _ => return Err("file/list expects a path".into()),
            };
            file_provider(operation)?
                .list(&path)
                .map(Value::Promise)
                .map_err(|error| file_error(operation, error))
        }
        "file/mkdir" => {
            if forms.len() != 1 {
                return Err("file/mkdir expects a path".into());
            }
            let path = match eval(&forms[0], env)? {
                Value::String(value) => value,
                _ => return Err("file/mkdir expects a path".into()),
            };
            file_provider(operation)?
                .mkdir(&path)
                .map(Value::Promise)
                .map_err(|error| file_error(operation, error))
        }
        "file/delete" => {
            if forms.len() != 1 {
                return Err("file/delete expects a path".into());
            }
            let path = match eval(&forms[0], env)? {
                Value::String(value) => value,
                _ => return Err("file/delete expects a path".into()),
            };
            file_provider(operation)?
                .delete(&path)
                .map(Value::Promise)
                .map_err(|error| file_error(operation, error))
        }
        _ => unreachable!(),
    }
}

fn socket_operation(
    operation: &str,
    forms: &[Form],
    env: &mut HashMap<String, Value>,
) -> Result<Value, String> {
    let operation = operation.strip_prefix("std.native.Socket/").unwrap_or(operation);
    match operation {
        "socket/connect" => {
            if forms.len() != 4 {
                return Err("socket/connect expects a host, port, options, and callback".into());
            }
            let host = match eval(&forms[0], env)? {
                Value::String(value) => value,
                _ => {
                    return Err("socket/connect expects a host, port, options, and callback".into())
                }
            };
            let port = match eval(&forms[1], env)? {
                Value::Number(value) if value > 0 && value <= u16::MAX as i64 => value as u16,
                _ => return Err("socket/connect expects a valid port".into()),
            };
            let _options = eval(&forms[2], env)?;
            let callback = match eval(&forms[3], env)? {
                Value::Function(value) => value,
                _ => return Err("socket/connect expects a callback".into()),
            };
            let callback = Rc::new(move |event| {
                let arguments = match event {
                    SocketEvent::Connected(handle) => {
                        vec![Value::Nil, Value::Number(handle as i64)]
                    }
                    SocketEvent::Failed(_, error) => vec![Value::String(error), Value::Nil],
                    SocketEvent::Data(_, _) | SocketEvent::Closed(_) => return,
                };
                let _ = call_function(&callback, arguments);
            });
            socket_provider(operation)?
                .connect(&host, port, callback)
                .map(|handle| Value::Number(handle as i64))
                .map_err(|error| socket_error(operation, error))
        }
        "socket/listen" => {
            if forms.len() != 4 {
                return Err("socket/listen expects a host, port, options, and callback".into());
            }
            let host = match eval(&forms[0], env)? {
                Value::String(value) => value,
                _ => return Err("socket/listen expects a host string".into()),
            };
            let port = match eval(&forms[1], env)? {
                Value::Number(value) if (0..=u16::MAX as i64).contains(&value) => value as u16,
                _ => return Err("socket/listen expects a valid port".into()),
            };
            let _options = eval(&forms[2], env)?;
            let callback = match eval(&forms[3], env)? {
                Value::Function(value) => value,
                _ => return Err("socket/listen expects a callback".into()),
            };
            let callback = Rc::new(move |event| {
                let _ = call_function(&callback, vec![socket_server_event_value(event)]);
            });
            socket_provider(operation)?
                .listen(&host, port, callback)
                .map(|handle| Value::Number(handle as i64))
                .map_err(|error| socket_error(operation, error))
        }
        "socket/endpoint" => {
            if forms.len() != 1 {
                return Err("socket/endpoint expects a server".into());
            }
            let server = socket_handle(&eval(&forms[0], env)?, "socket/endpoint")?;
            socket_provider(operation)?
                .endpoint(server)
                .map(|(host, port)| Value::Map(PMap::from_iter([
                    (Value::Keyword("host".into()), Value::String(host)),
                    (Value::Keyword("port".into()), Value::Number(port as i64)),
                ])))
                .map_err(|error| socket_error(operation, error))
        }
        "socket/events" => {
            if forms.len() != 2 {
                return Err("socket/events expects a socket handle and options".into());
            }
            let handle = socket_handle(&eval(&forms[0], env)?, "socket/events")?;
            let _options = eval(&forms[1], env)?;
            socket_provider(operation)?
                .events(handle)
                .map(|stream| Value::Number(stream as i64))
                .map_err(|error| socket_error(operation, error))
        }
        "socket/next" => {
            if forms.len() != 1 {
                return Err("socket/next expects a socket stream".into());
            }
            let stream = socket_handle(&eval(&forms[0], env)?, "socket/next")?;
            socket_provider(operation)?
                .next(stream)
                .map(Value::Promise)
                .map_err(|error| socket_error(operation, error))
        }
        "socket/send" => {
            if forms.len() != 2 {
                return Err("socket/send expects a socket connection and bytes".into());
            }
            let socket = match eval(&forms[0], env)? {
                Value::Number(value) if value >= 0 => value as SocketHandle,
                _ => return Err("socket/send expects a socket connection and bytes".into()),
            };
            let bytes = match eval(&forms[1], env)? {
                Value::Bytes(value) => value,
                Value::ByteBuffer(value) => value.borrow().clone(),
                _ => return Err("socket/send expects a socket connection and bytes".into()),
            };
            socket_provider(operation)?
                .send(socket, &bytes)
                .map(|count| Value::Number(count as i64))
                .map_err(|error| socket_error(operation, error))
        }
        "socket/close" => {
            if forms.len() != 1 {
                return Err("socket/close expects a socket connection".into());
            }
            let socket = match eval(&forms[0], env)? {
                Value::Number(value) if value >= 0 => value as SocketHandle,
                _ => return Err("socket/close expects a socket connection".into()),
            };
            socket_provider(operation)?
                .close(socket)
                .map(|()| Value::Nil)
                .map_err(|error| socket_error(operation, error))
        }
        _ => unreachable!(),
    }
}

fn socket_handle(value: &Value, operation: &str) -> Result<SocketHandle, String> {
    match value {
        Value::Number(value) if *value >= 0 => Ok(*value as SocketHandle),
        _ => Err(format!("{operation} expects a socket handle")),
    }
}
/// Installs the explicit host-call boundary for one evaluation.
pub fn with_host_calls<R>(
    handler: Rc<dyn Fn(String, String, Vec<Value>) -> Result<Value, String>>,
    operation: impl FnOnce() -> R,
) -> R {
    HOST_CALL_HANDLER.with(|active| {
        let previous = active.replace(Some(handler));
        let result = operation();
        active.replace(previous);
        result
    })
}

/// Runs an evaluation with a source provider used to satisfy `require` loads.
pub fn with_namespace_source<R>(
    provider: Rc<dyn Fn(&str) -> Option<String>>,
    action: impl FnOnce() -> R,
) -> R {
    NAMESPACE_SOURCE_PROVIDER.with(|active| {
        let previous = active.borrow_mut().replace(provider);
        let result = action();
        *active.borrow_mut() = previous;
        result
    })
}

pub trait ExtensionProvider {
    fn name(&self) -> &str;
    fn install(&self, protocols: &mut ProtocolRegistry);
    fn construct(&self, type_name: &str, arguments: &[Value]) -> Result<Value, String>;
}

#[derive(Default, Clone)]
pub struct ExtensionRegistry {
    providers: HashMap<String, Rc<dyn ExtensionProvider>>,
    loaded: HashSet<String>,
}

impl ExtensionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn install<P: ExtensionProvider + 'static>(&mut self, provider: P) {
        self.providers
            .insert(provider.name().to_string(), Rc::new(provider));
    }

    pub fn contains(&self, name: &str) -> bool {
        self.providers.contains_key(name)
    }

    pub fn require(
        &mut self,
        name: &str,
        protocols: &mut ProtocolRegistry,
    ) -> Result<String, String> {
        let provider = self
            .providers
            .get(name)
            .cloned()
            .ok_or_else(|| format!("extension/not-found: {name}"))?;
        if self.loaded.insert(name.to_string()) {
            provider.install(protocols);
        }
        Ok(if self.loaded.len() == 1 {
            ":loaded".into()
        } else {
            ":loaded".into()
        })
    }

    pub fn construct(
        &self,
        provider: &str,
        type_name: &str,
        arguments: &[Value],
    ) -> Result<Value, String> {
        self.providers
            .get(provider)
            .ok_or_else(|| format!("extension/not-found: {provider}"))?
            .construct(type_name, arguments)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum FileError {
    Unsupported,
    Denied,
    Invalid(String),
}

impl FileError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Unsupported => "unsupported",
            Self::Denied => "denied",
            Self::Invalid(_) => "invalid",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SocketError {
    Unsupported,
    Denied,
    Invalid(String),
}

impl SocketError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Unsupported => "unsupported",
            Self::Denied => "denied",
            Self::Invalid(_) => "invalid",
        }
    }
}

pub trait FileProvider {
    fn resolve(&self, root: &str, path: &str) -> Result<String, FileError>;
    fn read(&self, path: &str) -> Result<Promise, FileError>;
    fn write(&self, path: &str, bytes: Vec<u8>) -> Result<Promise, FileError>;
    fn exists(&self, path: &str) -> Result<Promise, FileError>;
    fn list(&self, path: &str) -> Result<Promise, FileError>;
    fn mkdir(&self, path: &str) -> Result<Promise, FileError>;
    fn delete(&self, path: &str) -> Result<Promise, FileError>;
}

pub type SocketHandle = u64;
pub type SocketCallback = Rc<dyn Fn(SocketEvent)>;

#[derive(Debug, Clone, PartialEq)]
pub enum SocketEvent {
    Connected(SocketHandle),
    Data(SocketHandle, Vec<u8>),
    Closed(SocketHandle),
    Failed(SocketHandle, String),
}

pub type SocketServerCallback = Rc<dyn Fn(SocketServerEvent)>;

#[derive(Debug, Clone, PartialEq)]
pub enum SocketServerEvent {
    Open { server: SocketHandle, connection: SocketHandle },
    Data { server: SocketHandle, connection: SocketHandle, bytes: Vec<u8> },
    Closed { server: SocketHandle, connection: SocketHandle },
    Failed { server: SocketHandle, connection: SocketHandle, error: String },
}

fn socket_server_event_value(event: SocketServerEvent) -> Value {
    let mut entries = Vec::new();
    match event {
        SocketServerEvent::Open { server, connection } => {
            entries.push((Value::Keyword("type".into()), Value::Keyword("open".into())));
            entries.push((Value::Keyword("server".into()), Value::Number(server as i64)));
            entries.push((Value::Keyword("connection".into()), Value::Number(connection as i64)));
        }
        SocketServerEvent::Data { server, connection, bytes } => {
            entries.push((Value::Keyword("type".into()), Value::Keyword("data".into())));
            entries.push((Value::Keyword("server".into()), Value::Number(server as i64)));
            entries.push((Value::Keyword("connection".into()), Value::Number(connection as i64)));
            entries.push((Value::Keyword("bytes".into()), Value::Bytes(bytes)));
        }
        SocketServerEvent::Closed { server, connection } => {
            entries.push((Value::Keyword("type".into()), Value::Keyword("close".into())));
            entries.push((Value::Keyword("server".into()), Value::Number(server as i64)));
            entries.push((Value::Keyword("connection".into()), Value::Number(connection as i64)));
        }
        SocketServerEvent::Failed { server, connection, error } => {
            entries.push((Value::Keyword("type".into()), Value::Keyword("error".into())));
            entries.push((Value::Keyword("server".into()), Value::Number(server as i64)));
            entries.push((Value::Keyword("connection".into()), Value::Number(connection as i64)));
            entries.push((Value::Keyword("error".into()), Value::String(error)));
        }
    }
    Value::Map(PMap::from_iter(entries))
}

pub trait SocketProvider {
    fn connect(
        &self,
        host: &str,
        port: u16,
        callback: SocketCallback,
    ) -> Result<SocketHandle, SocketError>;
    fn send(&self, socket: SocketHandle, bytes: &[u8]) -> Result<usize, SocketError>;
    fn close(&self, socket: SocketHandle) -> Result<(), SocketError>;
    fn listen(
        &self,
        _host: &str,
        _port: u16,
        _callback: SocketServerCallback,
    ) -> Result<SocketHandle, SocketError> {
        Err(SocketError::Unsupported)
    }
    fn endpoint(&self, _server: SocketHandle) -> Result<(String, u16), SocketError> {
        Err(SocketError::Unsupported)
    }
    fn events(&self, _handle: SocketHandle) -> Result<SocketHandle, SocketError> {
        Err(SocketError::Unsupported)
    }
    fn next(&self, _stream: SocketHandle) -> Result<Promise, SocketError> {
        Err(SocketError::Unsupported)
    }
}

#[cfg(not(target_arch = "wasm32"))]
use std::io::{Read, Write};
#[cfg(not(target_arch = "wasm32"))]
use std::net::{Shutdown, TcpListener, TcpStream};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::{mpsc, Arc, Mutex};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
#[cfg(any(not(target_arch = "wasm32"), target_os = "wasi"))]
use std::path::{Path, PathBuf};

#[cfg(any(not(target_arch = "wasm32"), target_os = "wasi"))]
#[derive(Debug, Clone)]
pub struct NativeFileProvider {
    root: PathBuf,
}

#[cfg(any(not(target_arch = "wasm32"), target_os = "wasi"))]
impl NativeFileProvider {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    fn scoped(&self, path: &str) -> Result<PathBuf, FileError> {
        let relative = Path::new(path);
        if relative.is_absolute() {
            return if relative == self.root || relative.strip_prefix(&self.root).is_ok() {
                Ok(relative.to_path_buf())
            } else {
                Err(FileError::Denied)
            };
        }
        if relative
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
        {
            return Err(FileError::Denied);
        }
        Ok(self.root.join(relative))
    }
}

#[cfg(any(not(target_arch = "wasm32"), target_os = "wasi"))]
impl FileProvider for NativeFileProvider {
    fn resolve(&self, root: &str, path: &str) -> Result<String, FileError> {
        if Path::new(root) != self.root {
            return Err(FileError::Denied);
        }
        self.scoped(path)
            .map(|path| path.to_string_lossy().into_owned())
    }

    fn read(&self, path: &str) -> Result<Promise, FileError> {
        let path = self.scoped(path)?;
        let promise = Promise::new();
        match std::fs::read(path) {
            Ok(bytes) => {
                promise.resolve(Value::Bytes(bytes));
            }
            Err(error) => {
                promise.reject(error.to_string());
            }
        }
        Ok(promise)
    }

    fn write(&self, path: &str, bytes: Vec<u8>) -> Result<Promise, FileError> {
        let path = self.scoped(path)?;
        let promise = Promise::new();
        match std::fs::write(path, bytes) {
            Ok(()) => {
                promise.resolve(Value::Nil);
            }
            Err(error) => {
                promise.reject(error.to_string());
            }
        }
        Ok(promise)
    }

    fn exists(&self, path: &str) -> Result<Promise, FileError> {
        let path = self.scoped(path)?;
        let promise = Promise::new();
        promise.resolve(Value::Bool(path.exists()));
        Ok(promise)
    }

    fn list(&self, path: &str) -> Result<Promise, FileError> {
        let path = self.scoped(path)?;
        let promise = Promise::new();
        match std::fs::read_dir(path) {
            Ok(entries) => {
                let mut names: Vec<String> = entries
                    .filter_map(|entry| entry.ok().map(|e| e.path().to_string_lossy().into_owned()))
                    .collect();
                names.sort();
                promise.resolve(Value::Array(Rc::new(RefCell::new(
                    names.into_iter().map(Value::String).collect(),
                ))));
            }
            Err(error) => {
                promise.reject(error.to_string());
            }
        }
        Ok(promise)
    }

    fn mkdir(&self, path: &str) -> Result<Promise, FileError> {
        let path = self.scoped(path)?;
        let promise = Promise::new();
        match std::fs::create_dir_all(path) {
            Ok(()) => {
                promise.resolve(Value::Nil);
            }
            Err(error) => {
                promise.reject(error.to_string());
            }
        }
        Ok(promise)
    }

    fn delete(&self, path: &str) -> Result<Promise, FileError> {
        let path = self.scoped(path)?;
        let promise = Promise::new();
        match std::fs::remove_file(path) {
            Ok(()) => {
                promise.resolve(Value::Nil);
            }
            Err(error) => {
                promise.reject(error.to_string());
            }
        }
        Ok(promise)
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
enum RawSocketEvent {
    Open { server: SocketHandle, connection: SocketHandle, stream: Arc<Mutex<TcpStream>> },
    Data { server: SocketHandle, connection: SocketHandle, bytes: Vec<u8> },
    Closed { server: SocketHandle, connection: SocketHandle },
    Failed { server: SocketHandle, connection: SocketHandle, error: String },
}

#[cfg(not(target_arch = "wasm32"))]
struct NativeServer {
    host: String,
    port: u16,
    alive: Arc<AtomicBool>,
}

#[cfg(not(target_arch = "wasm32"))]
struct NativeSocketStream {
    handle: SocketHandle,
    queue: VecDeque<Value>,
    queued_bytes: usize,
    pending: Option<Promise>,
    closed: bool,
}

#[cfg(not(target_arch = "wasm32"))]
struct NativeSocketState {
    next_handle: Arc<AtomicU64>,
    sockets: HashMap<SocketHandle, TcpStream>,
    callbacks: HashMap<SocketHandle, SocketCallback>,
    servers: HashMap<SocketHandle, NativeServer>,
    connections: HashMap<SocketHandle, Arc<Mutex<TcpStream>>>,
    connection_servers: HashMap<SocketHandle, SocketHandle>,
    server_callbacks: HashMap<SocketHandle, SocketServerCallback>,
    streams: HashMap<SocketHandle, NativeSocketStream>,
    sender: mpsc::Sender<RawSocketEvent>,
    receiver: mpsc::Receiver<RawSocketEvent>,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
pub struct NativeSocketProvider {
    state: Rc<RefCell<NativeSocketState>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for NativeSocketProvider {
    fn default() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            state: Rc::new(RefCell::new(NativeSocketState {
                next_handle: Arc::new(AtomicU64::new(1)),
                sockets: HashMap::new(),
                callbacks: HashMap::new(),
                servers: HashMap::new(),
                connections: HashMap::new(),
                connection_servers: HashMap::new(),
                server_callbacks: HashMap::new(),
                streams: HashMap::new(),
                sender,
                receiver,
            })),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl NativeSocketProvider {
    fn next_handle(&self) -> SocketHandle {
        self.state.borrow().next_handle.fetch_add(1, Ordering::Relaxed)
    }

    fn pump(&self) {
        loop {
            let event = { self.state.borrow().receiver.try_recv().ok() };
            let Some(event) = event else { break; };
            self.dispatch(event);
        }
    }

    fn wait_and_pump(&self) {
        let event = { self.state.borrow().receiver.recv().ok() };
        if let Some(event) = event {
            self.dispatch(event);
        }
        self.pump();
    }

    fn dispatch(&self, raw: RawSocketEvent) {
        let event = match raw {
            RawSocketEvent::Open { server, connection, stream } => {
                let mut state = self.state.borrow_mut();
                state.connections.insert(connection, stream);
                state.connection_servers.insert(connection, server);
                SocketServerEvent::Open { server, connection }
            }
            RawSocketEvent::Data { server, connection, bytes } => {
                SocketServerEvent::Data { server, connection, bytes }
            }
            RawSocketEvent::Closed { server, connection } => {
                self.state.borrow_mut().connections.remove(&connection);
                SocketServerEvent::Closed { server, connection }
            }
            RawSocketEvent::Failed { server, connection, error } => {
                SocketServerEvent::Failed { server, connection, error }
            }
        };
        let callback = {
            self.state.borrow().server_callbacks.get(&match &event {
                SocketServerEvent::Open { server, .. }
                | SocketServerEvent::Data { server, .. }
                | SocketServerEvent::Closed { server, .. }
                | SocketServerEvent::Failed { server, .. } => *server,
            }).cloned()
        };
        if let Some(callback) = callback {
            callback(event.clone());
        }
        let (server, connection, bytes) = match &event {
            SocketServerEvent::Open { server, connection }
            | SocketServerEvent::Closed { server, connection }
            | SocketServerEvent::Failed { server, connection, .. } => (*server, *connection, 0),
            SocketServerEvent::Data { server, connection, bytes } => (*server, *connection, bytes.len()),
        };
        let value = socket_server_event_value(event);
        let overflow = {
            let mut state = self.state.borrow_mut();
            let mut overflow = false;
            for stream in state.streams.values_mut().filter(|stream| stream.handle == server || stream.handle == connection) {
                if stream.closed { continue; }
                if stream.queue.len() >= 256 || stream.queued_bytes.saturating_add(bytes) > 1_048_576 {
                    stream.closed = true;
                    if let Some(promise) = stream.pending.take() {
                        promise.resolve(Value::Map(PMap::from_iter([
                            (Value::Keyword("type".into()), Value::Keyword("error".into())),
                            (Value::Keyword("error".into()), Value::String("buffer-overflow".into())),
                        ])));
                    }
                    overflow = true;
                    continue;
                }
                if let Some(promise) = stream.pending.take() {
                    promise.resolve(value.clone());
                } else {
                    stream.queued_bytes += bytes;
                    stream.queue.push_back(value.clone());
                }
            }
            overflow
        };
        if overflow {
            let _ = self.close(connection);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl SocketProvider for NativeSocketProvider {
    fn connect(
        &self,
        host: &str,
        port: u16,
        callback: SocketCallback,
    ) -> Result<SocketHandle, SocketError> {
        if host.is_empty() || port == 0 {
            return Err(SocketError::Invalid("host and port are required".into()));
        }
        let stream = TcpStream::connect((host, port))
            .map_err(|error| SocketError::Invalid(error.to_string()))?;
        let handle = self.next_handle();
        self.state.borrow_mut().sockets.insert(handle, stream);
        self.state.borrow_mut().callbacks.insert(handle, callback.clone());
        callback(SocketEvent::Connected(handle));
        Ok(handle)
    }

    fn send(&self, socket: SocketHandle, bytes: &[u8]) -> Result<usize, SocketError> {
        let mut state = self.state.borrow_mut();
        let stream = state.sockets
            .get_mut(&socket)
            .ok_or_else(|| SocketError::Invalid("unknown socket".into()))?;
        stream
            .write_all(bytes)
            .map_err(|error| SocketError::Invalid(error.to_string()))?;
        drop(state);
        if let Some(callback) = self.state.borrow().callbacks.get(&socket).cloned() {
            callback(SocketEvent::Data(socket, bytes.to_vec()));
        }
        Ok(bytes.len())
    }

    fn close(&self, socket: SocketHandle) -> Result<(), SocketError> {
        if self.state.borrow_mut().sockets.remove(&socket).is_some() {
            if let Some(callback) = self.state.borrow_mut().callbacks.remove(&socket) {
                callback(SocketEvent::Closed(socket));
            }
            return Ok(());
        }
        let server = { self.state.borrow_mut().servers.remove(&socket) };
        if let Some(server) = server {
            server.alive.store(false, Ordering::Relaxed);
            self.state.borrow_mut().server_callbacks.remove(&socket);
            return Ok(());
        }
        if let Some(stream) = self.state.borrow_mut().connections.remove(&socket) {
            let server = self.state.borrow().connection_servers.get(&socket).copied().unwrap_or(0);
            let _ = stream.lock().map(|stream| stream.shutdown(Shutdown::Both));
            let _ = self.state.borrow().sender.send(RawSocketEvent::Closed { server, connection: socket });
            self.pump();
            return Ok(());
        }
        Err(SocketError::Invalid("unknown socket".into()))
    }

    fn listen(&self, host: &str, port: u16, callback: SocketServerCallback) -> Result<SocketHandle, SocketError> {
        if host.is_empty() { return Err(SocketError::Invalid("host is required".into())); }
        let listener = TcpListener::bind((host, port)).map_err(|error| SocketError::Invalid(error.to_string()))?;
        let endpoint = listener.local_addr().map_err(|error| SocketError::Invalid(error.to_string()))?;
        listener.set_nonblocking(true).map_err(|error| SocketError::Invalid(error.to_string()))?;
        let server = self.next_handle();
        let alive = Arc::new(AtomicBool::new(true));
        let sender = self.state.borrow().sender.clone();
        let next_handle = self.state.borrow().next_handle.clone();
        let thread_alive = alive.clone();
        std::thread::Builder::new().name(format!("hara-socket-{server}")).spawn(move || {
            while thread_alive.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let connection = next_handle.fetch_add(1, Ordering::Relaxed);
                        if let Err(error) = stream.set_nonblocking(false) {
                            let _ = sender.send(RawSocketEvent::Failed { server, connection, error: error.to_string() });
                            continue;
                        }
                        let shared = Arc::new(Mutex::new(stream));
                        let _ = sender.send(RawSocketEvent::Open { server, connection, stream: shared.clone() });
                        let reader = shared.clone();
                        let reader_sender = sender.clone();
                        std::thread::spawn(move || {
                            let mut buffer = [0u8; 8192];
                            loop {
                                let read = match reader.lock() { Ok(mut stream) => stream.read(&mut buffer), Err(_) => return };
                                match read {
                                    Ok(0) => { let _ = reader_sender.send(RawSocketEvent::Closed { server, connection }); break; }
                                    Ok(count) => { let _ = reader_sender.send(RawSocketEvent::Data { server, connection, bytes: buffer[..count].to_vec() }); }
                                    Err(error) => { let _ = reader_sender.send(RawSocketEvent::Failed { server, connection, error: error.to_string() }); break; }
                                }
                            }
                        });
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(std::time::Duration::from_millis(5)),
                    Err(error) => { let _ = sender.send(RawSocketEvent::Failed { server, connection: 0, error: error.to_string() }); break; }
                }
            }
        }).map_err(|error| SocketError::Invalid(error.to_string()))?;
        let mut state = self.state.borrow_mut();
        state.servers.insert(server, NativeServer { host: endpoint.ip().to_string(), port: endpoint.port(), alive });
        state.server_callbacks.insert(server, callback);
        Ok(server)
    }

    fn endpoint(&self, server: SocketHandle) -> Result<(String, u16), SocketError> {
        let state = self.state.borrow();
        let server = state.servers.get(&server).ok_or_else(|| SocketError::Invalid("unknown socket server".into()))?;
        Ok((server.host.clone(), server.port))
    }

    fn events(&self, handle: SocketHandle) -> Result<SocketHandle, SocketError> {
        let mut state = self.state.borrow_mut();
        if !state.servers.contains_key(&handle) && !state.connections.contains_key(&handle) {
            return Err(SocketError::Invalid("unknown socket handle".into()));
        }
        let stream = state.next_handle.fetch_add(1, Ordering::Relaxed);
        state.streams.insert(stream, NativeSocketStream { handle, queue: VecDeque::new(), queued_bytes: 0, pending: None, closed: false });
        Ok(stream)
    }

    fn next(&self, stream: SocketHandle) -> Result<Promise, SocketError> {
        self.pump();
        let promise = Promise::new();
        {
            let mut state = self.state.borrow_mut();
            let stream = state.streams.get_mut(&stream).ok_or_else(|| SocketError::Invalid("unknown socket stream".into()))?;
            if let Some(event) = stream.queue.pop_front() {
                stream.queued_bytes = 0;
                promise.resolve(event);
                return Ok(promise);
            }
            if stream.closed {
                promise.resolve(Value::Map(PMap::from_iter([ (Value::Keyword("type".into()), Value::Keyword("close".into())) ])));
                return Ok(promise);
            }
            if stream.pending.is_some() { return Err(SocketError::Invalid("socket stream already has a pending next".into())); }
            stream.pending = Some(promise.clone());
        }
        let provider = self.clone();
        promise.set_poller(Rc::new(move || provider.pump()));
        let provider = self.clone();
        promise.set_waiter(Rc::new(move || provider.wait_and_pump()));
        Ok(promise)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderCapabilities {
    pub file: bool,
    pub socket: bool,
}

pub struct ProviderRegistry {
    file: Option<Rc<dyn FileProvider>>,
    socket: Option<Rc<dyn SocketProvider>>,
    promise: Rc<dyn PromiseProvider>,
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self {
            file: None,
            socket: None,
            promise: Rc::new(LocalPromiseProvider),
        }
    }
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn install_file<P: FileProvider + 'static>(&mut self, provider: P) {
        self.file = Some(Rc::new(provider));
    }
    pub fn install_socket<P: SocketProvider + 'static>(&mut self, provider: P) {
        self.socket = Some(Rc::new(provider));
    }
    pub fn install_promise<P: PromiseProvider + 'static>(&mut self, provider: P) {
        self.promise = Rc::new(provider);
    }
    pub fn promise(&self) -> Rc<dyn PromiseProvider> {
        self.promise.clone()
    }
    pub fn file(&self) -> Option<Rc<dyn FileProvider>> {
        self.file.clone()
    }
    pub fn socket(&self) -> Option<Rc<dyn SocketProvider>> {
        self.socket.clone()
    }
    pub fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            file: self.file.is_some(),
            socket: self.socket.is_some(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MemoryFileProvider {
    root: String,
    files: Rc<RefCell<HashMap<String, Vec<u8>>>>,
}

impl MemoryFileProvider {
    pub fn new(root: impl Into<String>) -> Self {
        Self {
            root: root.into().trim_end_matches('/').to_string(),
            files: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    fn within_root(&self, path: &str) -> bool {
        path == self.root
            || path
                .strip_prefix(&self.root)
                .is_some_and(|rest| rest.starts_with('/'))
    }

    pub fn insert(&self, path: &str, bytes: Vec<u8>) -> Result<(), FileError> {
        if !self.within_root(path) {
            return Err(FileError::Denied);
        }
        self.files.borrow_mut().insert(path.to_string(), bytes);
        Ok(())
    }
}

impl FileProvider for MemoryFileProvider {
    fn resolve(&self, root: &str, path: &str) -> Result<String, FileError> {
        if root != self.root || path.starts_with('/') {
            return Err(FileError::Denied);
        }
        let mut result = self.root.clone();
        for segment in path.split('/') {
            match segment {
                "" | "." => {}
                ".." => return Err(FileError::Denied),
                segment if segment.contains('\0') => {
                    return Err(FileError::Invalid("path contains NUL".into()))
                }
                segment => {
                    result.push('/');
                    result.push_str(segment);
                }
            }
        }
        Ok(result)
    }

    fn read(&self, path: &str) -> Result<Promise, FileError> {
        if !self.within_root(path) {
            return Err(FileError::Denied);
        }
        let promise = Promise::new();
        match self.files.borrow().get(path) {
            Some(bytes) => {
                promise.resolve(Value::Bytes(bytes.clone()));
            }
            None => {
                promise.reject("file not found");
            }
        }
        Ok(promise)
    }

    fn write(&self, path: &str, bytes: Vec<u8>) -> Result<Promise, FileError> {
        if !self.within_root(path) {
            return Err(FileError::Denied);
        }
        self.files.borrow_mut().insert(path.to_string(), bytes);
        let promise = Promise::new();
        promise.resolve(Value::Nil);
        Ok(promise)
    }

    fn exists(&self, path: &str) -> Result<Promise, FileError> {
        if !self.within_root(path) {
            return Err(FileError::Denied);
        }
        let promise = Promise::new();
        let exists = path == self.root || self.files.borrow().contains_key(path);
        promise.resolve(Value::Bool(exists));
        Ok(promise)
    }

    fn list(&self, path: &str) -> Result<Promise, FileError> {
        if !self.within_root(path) {
            return Err(FileError::Denied);
        }
        let prefix = if path == self.root {
            format!("{}/", self.root)
        } else {
            format!("{path}/")
        };
        let mut names: Vec<String> = self
            .files
            .borrow()
            .keys()
            .filter(|key| key.starts_with(&prefix) && !key[prefix.len()..].contains('/'))
            .cloned()
            .collect();
        names.sort();
        let promise = Promise::new();
        promise.resolve(Value::Array(Rc::new(RefCell::new(
            names.into_iter().map(Value::String).collect(),
        ))));
        Ok(promise)
    }

    fn mkdir(&self, path: &str) -> Result<Promise, FileError> {
        if !self.within_root(path) {
            return Err(FileError::Denied);
        }
        let promise = Promise::new();
        promise.resolve(Value::Nil);
        Ok(promise)
    }

    fn delete(&self, path: &str) -> Result<Promise, FileError> {
        if !self.within_root(path) {
            return Err(FileError::Denied);
        }
        let promise = Promise::new();
        if self.files.borrow_mut().remove(path).is_some() {
            promise.resolve(Value::Nil);
        } else {
            promise.reject("file not found");
        }
        Ok(promise)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct UnsupportedFileProvider;

impl FileProvider for UnsupportedFileProvider {
    fn resolve(&self, _root: &str, _path: &str) -> Result<String, FileError> {
        Err(FileError::Unsupported)
    }
    fn read(&self, _path: &str) -> Result<Promise, FileError> {
        Err(FileError::Unsupported)
    }
    fn write(&self, _path: &str, _bytes: Vec<u8>) -> Result<Promise, FileError> {
        Err(FileError::Unsupported)
    }
    fn exists(&self, _path: &str) -> Result<Promise, FileError> {
        Err(FileError::Unsupported)
    }
    fn list(&self, _path: &str) -> Result<Promise, FileError> {
        Err(FileError::Unsupported)
    }
    fn mkdir(&self, _path: &str) -> Result<Promise, FileError> {
        Err(FileError::Unsupported)
    }
    fn delete(&self, _path: &str) -> Result<Promise, FileError> {
        Err(FileError::Unsupported)
    }
}

#[derive(Clone)]
pub struct LoopbackSocketProvider {
    next_handle: Rc<Cell<SocketHandle>>,
    callbacks: Rc<RefCell<HashMap<SocketHandle, SocketCallback>>>,
}

impl Default for LoopbackSocketProvider {
    fn default() -> Self {
        Self {
            next_handle: Rc::new(Cell::new(1)),
            callbacks: Rc::new(RefCell::new(HashMap::new())),
        }
    }
}

impl SocketProvider for LoopbackSocketProvider {
    fn connect(
        &self,
        host: &str,
        port: u16,
        callback: SocketCallback,
    ) -> Result<SocketHandle, SocketError> {
        if host.is_empty() || port == 0 {
            return Err(SocketError::Invalid("host and port are required".into()));
        }
        let handle = self.next_handle.get();
        self.next_handle.set(handle + 1);
        self.callbacks.borrow_mut().insert(handle, callback.clone());
        callback(SocketEvent::Connected(handle));
        Ok(handle)
    }

    fn send(&self, socket: SocketHandle, bytes: &[u8]) -> Result<usize, SocketError> {
        let callback = self
            .callbacks
            .borrow()
            .get(&socket)
            .cloned()
            .ok_or_else(|| SocketError::Invalid("unknown socket".into()))?;
        callback(SocketEvent::Data(socket, bytes.to_vec()));
        Ok(bytes.len())
    }

    fn close(&self, socket: SocketHandle) -> Result<(), SocketError> {
        let callback = self
            .callbacks
            .borrow_mut()
            .remove(&socket)
            .ok_or_else(|| SocketError::Invalid("unknown socket".into()))?;
        callback(SocketEvent::Closed(socket));
        Ok(())
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct UnsupportedSocketProvider;

impl SocketProvider for UnsupportedSocketProvider {
    fn connect(
        &self,
        _host: &str,
        _port: u16,
        _callback: SocketCallback,
    ) -> Result<SocketHandle, SocketError> {
        Err(SocketError::Unsupported)
    }
    fn send(&self, _socket: SocketHandle, _bytes: &[u8]) -> Result<usize, SocketError> {
        Err(SocketError::Unsupported)
    }
    fn close(&self, _socket: SocketHandle) -> Result<(), SocketError> {
        Err(SocketError::Unsupported)
    }
}

fn portable_type_name(value: &Value) -> &str {
    match value {
        Value::Nil => "nil",
        Value::Number(_) => "integer",
        Value::Float(_) => "float",
        Value::BigInteger(_) => "big-integer",
        Value::Decimal(_) => "decimal",
        Value::Character(_) => "character",
        Value::Regex(_) => "pattern",
        Value::Tagged(_) => "tagged-literal",
        Value::Bool(_) => "boolean",
        Value::String(_) => "string",
        Value::Keyword(_) => "keyword",
        Value::Symbol(_) => "symbol",
        Value::Pointer(_) => "pointer",
        Value::Function(_) => "function",
        Value::Bytes(_) => "bytes",
        Value::ByteBuffer(_) => "byte-buffer",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
        Value::Promise(_) => "promise",
        Value::Atom(_) => "atom",
        Value::Recur(_) => "recur",
        Value::List(_) => "list",
        Value::Cons(_) => "cons",
        Value::Queue(_) => "queue",
        Value::Tuple(_) => "tuple",
        Value::Vector(_) => "vector",
        Value::Map(_) => "hash-map",
        Value::OrderedMap(_) => "ordered-map",
        Value::SortedMap(_) => "sorted-map",
        Value::Trie(_) => "trie",
        Value::Set(_) => "hash-set",
        Value::OrderedSet(_) => "ordered-set",
        Value::SortedSet(_) => "sorted-set",
        Value::Iterator(_) => "iterator",
        Value::Var(_) => "var",
        Value::Namespace(_) => "namespace",
        Value::Extension(_) => "extension",
        Value::StructType(_) => "struct-type",
        Value::Struct(_) => "struct",
        Value::Protocol(_) => "protocol",
        Value::NativeType(_) => "native-type",
        Value::Coroutine(_) => "coroutine",
        Value::ExceptionInfo(_) => "error",
    }
}

pub fn receiver_category(value: &Value) -> &'static str {
    match value {
        Value::Nil => "nil",
        Value::Number(_) | Value::Float(_) | Value::BigInteger(_) | Value::Decimal(_) => "number",
        Value::Character(_) => "character",
        Value::Regex(_) => "pattern",
        Value::Tagged(_) => "tagged",
        Value::Bool(_) => "boolean",
        Value::String(_) => "string",
        Value::Keyword(_) => "keyword",
        Value::Symbol(_) => "symbol",
        Value::Pointer(_) => "pointer",
        Value::Function(_) => "function",
        Value::Bytes(_) | Value::ByteBuffer(_) => "bytes",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
        Value::Promise(_) => "promise",
        Value::Atom(_) => "atom",
        Value::Recur(_) => "recur",
        Value::List(_) => "list",
        Value::Cons(_) => "cons",
        Value::Queue(_) => "queue",
        Value::Tuple(_) => "tuple",
        Value::Vector(_) => "vector",
        Value::Map(_) | Value::OrderedMap(_) | Value::SortedMap(_) | Value::Trie(_) => "map",
        Value::Set(_) | Value::OrderedSet(_) | Value::SortedSet(_) => "set",
        Value::Iterator(_) => "iterator",
        Value::Var(_) => "var",
        Value::Namespace(_) => "namespace",
        Value::Extension(_) => "extension",
        Value::StructType(_) => "struct-type",
        Value::Struct(_) => "struct",
        Value::Protocol(_) => "protocol",
        Value::NativeType(_) => "native-type",
        Value::Coroutine(_) => "coroutine",
        Value::ExceptionInfo(_) => "error",
    }
}

fn coroutine_status(coroutine: &Coroutine) -> Value {
    let state = coroutine.state.borrow();
    Value::Keyword(Keyword::from(match &*state {
        CoroutineState::New(_) | CoroutineState::Suspended(_) => "suspended",
        CoroutineState::Running => "running",
        CoroutineState::Dead => "dead",
    }))
}

fn coroutine_close(coroutine: &Coroutine) -> Result<(), String> {
    let mut state = coroutine.state.borrow_mut();
    match &*state {
        CoroutineState::Dead => Ok(()),
        CoroutineState::Running => Err("coroutine/close: cannot close a running coroutine".into()),
        _ => {
            *state = CoroutineState::Dead;
            Ok(())
        }
    }
}

fn parse(source: &str) -> Result<Form, String> {
    crate::kernel::parse(source)
}
fn parse_forms(source: &str) -> Result<Vec<Form>, String> {
    crate::kernel::parse_forms(source)
}

pub fn read_edn(source: &str) -> Result<Value, String> {
    let forms = parse_forms(source).map_err(|error| format!("edn/read: {error}"))?;
    if forms.len() != 1 {
        return Err("edn/read expects exactly one value".into());
    }
    form_to_value(&forms[0]).map_err(|error| format!("edn/read: {error}"))
}

fn arithmetic(op: &str, args: &[Form], env: &mut HashMap<String, Value>) -> Result<Value, String> {
    if args.is_empty() {
        return Err(format!("{op} expects arguments"));
    }
    let values: Result<Vec<i64>, String> = args
        .iter()
        .map(|f| match eval(f, env)? {
            Value::Number(v) => Ok(v),
            _ => Err(format!("{op} expects numbers")),
        })
        .collect();
    let values = values?;
    let result = values.iter().skip(1).try_fold(values[0], |r, v| match op {
        "+" => r.checked_add(*v).ok_or_else(|| "integer overflow".to_string()),
        "-" => r.checked_sub(*v).ok_or_else(|| "integer overflow".to_string()),
        "*" => r.checked_mul(*v).ok_or_else(|| "integer overflow".to_string()),
        "/" => {
            if *v == 0 {
                Err("division by zero".into())
            } else {
                r.checked_div(*v).ok_or_else(|| "integer overflow".to_string())
            }
        }
        "%" => {
            if *v == 0 {
                Err("division by zero".into())
            } else {
                r.checked_rem(*v).ok_or_else(|| "integer overflow".to_string())
            }
        }
        _ => unreachable!(),
    })?;
    Ok(Value::Number(result))
}

fn bit_operation(
    op: &str,
    args: &[Form],
    env: &mut HashMap<String, Value>,
) -> Result<Value, String> {
    let op = match op.strip_prefix("std.native.Bits/").unwrap_or(op) {
        "and" => "bit-and",
        "or" => "bit-or",
        "xor" => "bit-xor",
        "not" => "bit-not",
        "shift-left" => "bit-shift-left",
        "shift-right" => "bit-shift-right",
        operation => operation,
    };
    let values = args
        .iter()
        .map(|form| eval(form, env))
        .collect::<Result<Vec<_>, _>>()?;
    let integer = |value: &Value| match value {
        Value::Number(value) => Ok(*value as i32),
        _ => Err(format!("{op} expects integers")),
    };
    match op {
        "bit-not" => {
            if values.len() != 1 {
                return Err("bit-not expects one integer".into());
            }
            Ok(Value::Number((!integer(&values[0])?) as i64))
        }
        "bit-and" | "bit-or" | "bit-xor" => {
            if values.len() != 2 {
                return Err(format!("{op} expects two integers"));
            }
            let a = integer(&values[0])?;
            let b = integer(&values[1])?;
            let result = match op {
                "bit-and" => a & b,
                "bit-or" => a | b,
                _ => a ^ b,
            };
            Ok(Value::Number(result as i64))
        }
        "bit-shift-left" | "bit-shift-right" => {
            if values.len() != 2 {
                return Err(format!("{op} expects an integer and distance"));
            }
            let value = integer(&values[0])?;
            let distance = match &values[1] {
                Value::Number(distance) if (0..=31).contains(distance) => *distance as u32,
                _ => return Err("distance must be in the range 0..31".into()),
            };
            let result = if op == "bit-shift-left" {
                value.wrapping_shl(distance)
            } else {
                value.wrapping_shr(distance)
            };
            Ok(Value::Number(result as i64))
        }
        _ => Err(format!("unknown bit operation: {op}")),
    }
}

fn number_conversion(
    operation: &str,
    args: &[Form],
    env: &mut HashMap<String, Value>,
) -> Result<Value, String> {
    let operation = operation
        .strip_prefix("std.native.Numbers/")
        .unwrap_or(operation);
    if args.len() != 1 {
        return Err(format!("{operation} expects one numeric value"));
    }
    let value = eval(&args[0], env)?;
    match (operation, value) {
        ("long", Value::Number(value)) => Ok(Value::Number(value)),
        ("long", Value::Float(value))
            if value.is_finite()
                && value.trunc() >= i64::MIN as f64
                && value.trunc() < -(i64::MIN as f64) =>
        {
            Ok(Value::Number(value.trunc() as i64))
        }
        ("double", Value::Number(value)) => Ok(Value::Float(value as f64)),
        ("double", Value::Float(value)) => Ok(Value::Float(value)),
        ("long" | "double", _) => {
            Err(format!("{operation} cannot convert non-numeric value"))
        }
        _ => Err(format!("unknown number conversion: {operation}")),
    }
}

fn numeric_to_f64(value: &Value, operation: &str) -> Result<f64, String> {
    match value {
        Value::Number(value) => Ok(*value as f64),
        Value::Float(value) => Ok(*value),
        Value::BigInteger(value) | Value::Decimal(value) => value
            .parse::<f64>()
            .map_err(|_| format!("{operation} expects a numeric value")),
        _ => Err(format!("{operation} expects a numeric value")),
    }
}

fn numeric_abs(value: Value) -> Result<Value, String> {
    match value {
        Value::Number(value) => match value.checked_abs() {
            Some(value) => Ok(Value::Number(value)),
            None => Err("integer overflow".into()),
        },
        Value::Float(value) => Ok(Value::Float(value.abs())),
        Value::BigInteger(value) => Ok(Value::BigInteger(
            value.strip_prefix('-').unwrap_or(&value).to_string(),
        )),
        Value::Decimal(value) => Ok(Value::Decimal(
            value.strip_prefix('-').unwrap_or(&value).to_string(),
        )),
        _ => Err("abs expects a numeric value".into()),
    }
}

fn math_operation(
    operation: &str,
    args: &[Form],
    env: &mut HashMap<String, Value>,
) -> Result<Value, String> {
    let operation = operation
        .strip_prefix("std.native.Maths/")
        .unwrap_or(operation);
    let expected = if matches!(operation, "atan2" | "pow") {
        2
    } else {
        1
    };
    if args.len() != expected {
        return Err(format!(
            "{operation} expects {} numeric {}",
            if expected == 1 { "one" } else { "two" },
            if expected == 1 { "value" } else { "values" }
        ));
    }
    let values = args
        .iter()
        .map(|form| eval(form, env))
        .collect::<Result<Vec<_>, _>>()?;
    if operation == "abs" {
        return numeric_abs(values.into_iter().next().unwrap());
    }
    let first = numeric_to_f64(&values[0], operation)?;
    let result = match operation {
        "acos" => first.acos(),
        "acosh" => first.acosh(),
        "asin" => first.asin(),
        "asinh" => first.asinh(),
        "atan" => first.atan(),
        "atan2" => first.atan2(numeric_to_f64(&values[1], operation)?),
        "atanh" => first.atanh(),
        "ceil" => first.ceil(),
        "cos" => first.cos(),
        "cosh" => first.cosh(),
        "exp" => first.exp(),
        "floor" => first.floor(),
        "pow" => first.powf(numeric_to_f64(&values[1], operation)?),
        "sin" => first.sin(),
        "sinh" => first.sinh(),
        "sqrt" => first.sqrt(),
        "tan" => first.tan(),
        "tanh" => first.tanh(),
        _ => return Err(format!("unknown math operation: {operation}")),
    };
    Ok(Value::Float(result))
}

fn native_error_operation(
    operation: &str,
    args: &[Form],
    env: &mut HashMap<String, Value>,
) -> Result<Value, String> {
    let operation = operation
        .strip_prefix("std.native.Error/")
        .unwrap_or(operation);
    match operation {
        "new" => {
            if !(2..=3).contains(&args.len()) {
                return Err("std.native.Error/new expects a message, data map, and optional cause"
                    .into());
            }
            let values = args
                .iter()
                .map(|form| eval(form, env))
                .collect::<Result<Vec<_>, _>>()?;
            let Value::String(message) = &values[0] else {
                return Err("std.native.Error/new expects a string message".into());
            };
            if map_entries(&values[1]).is_none() {
                return Err("std.native.Error/new expects a data map".into());
            }
            Ok(Value::ExceptionInfo(Rc::new(ExceptionInfo {
                message: message.clone(),
                data: Box::new(values[1].clone()),
                cause: values.get(2).cloned().map(Box::new),
            })))
        }
        "message" => {
            if args.len() != 1 {
                return Err("std.native.Error/message expects one value".into());
            }
            Ok(match eval(&args[0], env)? {
                Value::ExceptionInfo(value) => Value::String(value.message.clone()),
                Value::String(value) => Value::String(value),
                value => Value::String(value.display()),
            })
        }
        "class" => {
            if args.len() != 1 {
                return Err("std.native.Error/class expects one value".into());
            }
            let value = eval(&args[0], env)?;
            Ok(Value::String(portable_type_name(&value).into()))
        }
        _ => Err(format!("unknown native error operation: {operation}")),
    }
}

fn comparison(op: &str, args: &[Form], env: &mut HashMap<String, Value>) -> Result<Value, String> {
    if args.len() < 2 {
        return Err(format!("{op} expects at least two arguments"));
    }
    let values = args
        .iter()
        .map(|form| eval(form, env))
        .collect::<Result<Vec<_>, _>>()?;
    let numbers = values
        .iter()
        .map(|value| match value {
            Value::Number(number) => Ok(*number),
            _ => Err(format!("{op} expects numbers")),
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(Value::Bool(numbers.windows(2).all(|pair| match op {
        "<" => pair[0] < pair[1],
        ">" => pair[0] > pair[1],
        "<=" => pair[0] <= pair[1],
        ">=" => pair[0] >= pair[1],
        _ => false,
    })))
}

fn value_index(value: &Value) -> Result<usize, String> {
    match value {
        Value::Number(index) if *index >= 0 => Ok(*index as usize),
        _ => Err("index must be a non-negative integer".into()),
    }
}

fn value_to_metadata(value: &Value) -> Result<MetadataValue, String> {
    match value {
        Value::Nil => Ok(MetadataValue::Nil),
        Value::Bool(value) => Ok(MetadataValue::Boolean(*value)),
        Value::Number(value) => Ok(MetadataValue::Number(*value)),
        Value::String(value) => Ok(MetadataValue::String(value.clone())),
        Value::Keyword(value) => Ok(MetadataValue::Keyword(value.clone())),
        Value::Symbol(value) => Ok(MetadataValue::Symbol(value.clone())),
        Value::Tuple(values) => Ok(MetadataValue::Vector(
            values
                .iter()
                .map(value_to_metadata)
                .collect::<Result<_, _>>()?,
        )),
        Value::Vector(values) => Ok(MetadataValue::Vector(
            values
                .iter()
                .map(value_to_metadata)
                .collect::<Result<_, _>>()?,
        )),
        Value::Queue(values) => Ok(MetadataValue::List(
            values
                .iter()
                .map(value_to_metadata)
                .collect::<Result<_, _>>()?,
        )),
        Value::List(values) => Ok(MetadataValue::List(
            values
                .iter()
                .map(value_to_metadata)
                .collect::<Result<_, _>>()?,
        )),
        value @ (Value::Set(_) | Value::OrderedSet(_) | Value::SortedSet(_)) => {
            Ok(MetadataValue::Set(
                set_items(value)
                    .unwrap()
                    .into_iter()
                    .map(value_to_metadata)
                    .collect::<Result<_, _>>()?,
            ))
        }
        value @ (Value::Map(_) | Value::OrderedMap(_) | Value::SortedMap(_) | Value::Trie(_)) => {
            Ok(MetadataValue::Map(
                map_entries(value)
                    .unwrap()
                    .iter()
                    .map(|(key, value)| Ok((value_to_metadata(key)?, value_to_metadata(value)?)))
                    .collect::<Result<_, String>>()?,
            ))
        }
        _ => Err("value cannot be stored in runtime-neutral metadata".into()),
    }
}

fn metadata_to_value(value: &MetadataValue) -> Result<Value, String> {
    match value {
        MetadataValue::Nil => Ok(Value::Nil),
        MetadataValue::Boolean(value) => Ok(Value::Bool(*value)),
        MetadataValue::Number(value) => Ok(Value::Number(*value)),
        MetadataValue::String(value) => Ok(Value::String(value.clone())),
        MetadataValue::Keyword(value) => Ok(Value::Keyword(value.clone())),
        MetadataValue::Symbol(value) => Ok(Value::Symbol(value.clone())),
        MetadataValue::Vector(values) => Ok(Value::Vector(
            values
                .iter()
                .map(metadata_to_value)
                .collect::<Result<_, _>>()?,
        )),
        MetadataValue::List(values) => Ok(Value::List(
            values
                .iter()
                .map(metadata_to_value)
                .collect::<Result<_, _>>()?,
        )),
        MetadataValue::Set(values) => Ok(Value::Set(
            values
                .iter()
                .map(metadata_to_value)
                .collect::<Result<Vec<_>, _>>()?
                .into(),
        )),
        MetadataValue::Map(values) => Ok(Value::Map(
            values
                .iter()
                .map(|(key, value)| Ok((metadata_to_value(key)?, metadata_to_value(value)?)))
                .collect::<Result<Vec<_>, String>>()?
                .into_iter()
                .collect(),
        )),
        _ => Err("metadata value is not supported by the L0 evaluator".into()),
    }
}

fn value_metadata(value: &Value) -> Option<Rc<Metadata>> {
    match value {
        Value::Symbol(value) => value.meta().cloned(),
        Value::Pointer(value) => value.meta().cloned(),
        Value::Tuple(value) => value.meta().cloned(),
        Value::Vector(value) => value.meta().cloned(),
        Value::List(value) => value.meta().cloned(),
        Value::Cons(value) => value.meta().cloned(),
        Value::Queue(value) => value.meta().cloned(),
        Value::Map(value) => value.meta().cloned(),
        Value::OrderedMap(value) => value.meta().cloned(),
        Value::SortedMap(value) => value.meta().cloned(),
        Value::Trie(value) => value.meta().cloned(),
        Value::Set(value) => value.meta().cloned(),
        Value::OrderedSet(value) => value.meta().cloned(),
        Value::SortedSet(value) => value.meta().cloned(),
        Value::Var(value) => value.hara_metadata(),
        Value::NativeType(value) => value.metadata.clone(),
        _ => None,
    }
}

fn protocol_meta(arguments: &[Value]) -> Result<Value, String> {
    if arguments.len() != 1 {
        return Err("IObjType/meta expects one argument".into());
    }
    match value_metadata(&arguments[0]) {
        None => Ok(Value::Nil),
        Some(metadata) => metadata_to_value(&MetadataValue::Map(metadata.entries().to_vec())),
    }
}

fn protocol_with_meta(arguments: &[Value]) -> Result<Value, String> {
    if arguments.len() != 2 {
        return Err("IObjType/with-meta expects a value and metadata map".into());
    }
    let MetadataValue::Map(entries) = value_to_metadata(&arguments[1])? else {
        return Err("IObjType/with-meta expects a metadata map".into());
    };
    attach_metadata(arguments[0].clone(), Metadata::new(entries))
}

fn protocol_count(arguments: &[Value]) -> Result<Value, String> {
    if arguments.len() == 1 {
        collection_count(&arguments[0])
    } else {
        Err("ICount/count expects one argument".into())
    }
}

fn protocol_nth(arguments: &[Value]) -> Result<Value, String> {
    if arguments.len() != 2 {
        return Err("INth/nth expects a collection and index".into());
    }
    if let Value::Bytes(bytes) = &arguments[0] {
        let index = value_index(&arguments[1])?;
        return bytes
            .get(index)
            .map(|byte| Value::Number(*byte as i8 as i64))
            .ok_or_else(|| "nth index out of bounds".into());
    }
    if let Value::ByteBuffer(bytes) = &arguments[0] {
        let index = value_index(&arguments[1])?;
        return bytes
            .borrow()
            .get(index)
            .map(|byte| Value::Number(*byte as i8 as i64))
            .ok_or_else(|| "nth index out of bounds".into());
    }
    collection_nth(&arguments[0], &arguments[1])
}

fn namespaced_parts(value: &Value) -> Option<(String, Option<String>)> {
    match value {
        Value::Keyword(value) => Some((
            value.get_name().to_owned(),
            value.get_namespace().map(str::to_owned),
        )),
        Value::Symbol(value) => Some((
            value.get_name().to_owned(),
            value.get_namespace().map(str::to_owned),
        )),
        Value::Var(value) => Some((
            value.get_name().to_owned(),
            value.get_namespace().map(str::to_owned),
        )),
        Value::NativeType(value) => value
            .name
            .rsplit_once('/')
            .map(|(namespace, name)| (name.to_owned(), Some(namespace.to_owned()))),
        _ => None,
    }
}

fn protocol_namespaced_name(arguments: &[Value]) -> Result<Value, String> {
    if arguments.len() != 1 {
        return Err("INamespaced/name expects one value".into());
    }
    namespaced_parts(&arguments[0])
        .map(|(name, _)| Value::String(name))
        .ok_or_else(|| "INamespaced/name has no implementation for this value".into())
}

fn protocol_namespaced_namespace(arguments: &[Value]) -> Result<Value, String> {
    if arguments.len() != 1 {
        return Err("INamespaced/namespace expects one value".into());
    }
    namespaced_parts(&arguments[0])
        .map(|(_, namespace)| namespace.map(Value::String).unwrap_or(Value::Nil))
        .ok_or_else(|| "INamespaced/namespace has no implementation for this value".into())
}

fn protocol_lookup(arguments: &[Value]) -> Result<Value, String> {
    if arguments.len() == 2 || arguments.len() == 3 {
        collection_get(
            &arguments[0],
            &arguments[1],
            arguments.get(2).cloned().unwrap_or(Value::Nil),
        )
    } else {
        Err("ILookup/lookup expects a collection, key, and optional default".into())
    }
}

fn protocol_assoc(arguments: &[Value]) -> Result<Value, String> {
    if arguments.len() == 3 {
        collection_assoc(&arguments[0], &arguments[1], arguments[2].clone())
    } else {
        Err("IAssoc/assoc expects a collection, key, and value".into())
    }
}

fn protocol_dissoc(arguments: &[Value]) -> Result<Value, String> {
    if arguments.len() == 2 {
        collection_dissoc(&arguments[0], &[arguments[1].clone()])
    } else {
        Err("IDissoc/dissoc expects a collection and key".into())
    }
}

fn pair_parts(value: &Value) -> Option<(Value, Value)> {
    match value {
        Value::Tuple(values) if values.len() >= 2 => Some((
            values.get(0).unwrap().clone(),
            values.get(1).unwrap().clone(),
        )),
        Value::Vector(values) if values.len() >= 2 => Some((values[0].clone(), values[1].clone())),
        Value::List(values) if values.len() >= 2 => Some((values[0].clone(), values[1].clone())),
        _ => None,
    }
}

fn indexed_find(value: Option<&Value>, index: usize) -> Result<Value, String> {
    Ok(value
        .map(|value| {
            Value::Vector(PVector::from_iter([
                Value::Number(index as i64),
                value.clone(),
            ]))
        })
        .unwrap_or(Value::Nil))
}

fn protocol_find(arguments: &[Value]) -> Result<Value, String> {
    if arguments.len() != 2 {
        return Err("IFind/find expects a collection and key".into());
    }
    let collection = &arguments[0];
    let key = &arguments[1];
    match collection {
        value @ (Value::Map(_) | Value::OrderedMap(_) | Value::SortedMap(_) | Value::Trie(_)) => {
            Ok(map_entries(value)
                .unwrap()
                .into_iter()
                .find(|(candidate, _)| candidate == key)
                .map(|(candidate, value)| Value::Vector(PVector::from_iter([candidate, value])))
                .unwrap_or(Value::Nil))
        }
        Value::Object(values) => {
            let key = match key {
                Value::String(value) => value.as_str(),
                Value::Keyword(value) => value.as_str(),
                _ => return Err("IFind/find object expects a string or keyword key".into()),
            };
            Ok(values
                .borrow()
                .iter()
                .find(|(candidate, _)| candidate == key)
                .map(|(candidate, value)| {
                    Value::Vector(PVector::from_iter([
                        Value::String(candidate.clone()),
                        value.clone(),
                    ]))
                })
                .unwrap_or(Value::Nil))
        }
        value @ (Value::Set(_) | Value::OrderedSet(_) | Value::SortedSet(_)) => {
            Ok(set_find(value, key).unwrap_or(Value::Nil))
        }
        Value::Tuple(values) => indexed_find(values.get(value_index(key)?), value_index(key)?),
        Value::Vector(values) => indexed_find(values.get(value_index(key)?), value_index(key)?),
        Value::List(values) => indexed_find(values.get(value_index(key)?), value_index(key)?),
        Value::Cons(values) => {
            let index = value_index(key)?;
            indexed_find(values.iter().nth(index).as_ref(), index)
        }
        Value::Queue(values) => indexed_find(values.get(value_index(key)?), value_index(key)?),
        _ => Err("IFind/find has no implementation for this value".into()),
    }
}

fn protocol_iter(arguments: &[Value]) -> Result<Value, String> {
    match arguments {
        [value]
            if matches!(
                value,
                Value::Iterator(_)
                    | Value::Nil
                    | Value::String(_)
                    | Value::Bytes(_)
                    | Value::ByteBuffer(_)
                    | Value::Array(_)
                    | Value::Object(_)
                    | Value::Map(_)
                    | Value::OrderedMap(_)
                    | Value::SortedMap(_)
                    | Value::Trie(_)
                    | Value::Set(_)
                    | Value::OrderedSet(_)
                    | Value::SortedSet(_)
                    | Value::List(_)
                    | Value::Cons(_)
                    | Value::Queue(_)
                    | Value::Tuple(_)
                    | Value::Vector(_)
            ) =>
        {
            make_iterator(value.clone())
        }
        _ => Err("IIter/iter expects one value".into()),
    }
}

fn protocol_deref(arguments: &[Value]) -> Result<Value, String> {
    match arguments {
        [Value::Atom(atom)] => Ok(atom.deref_value()),
        [Value::Var(var)] => Ok(var.deref_value()),
        [Value::Promise(promise)] => promise_value_result(promise),
        _ => Err("IDeref/deref has no implementation for this value".into()),
    }
}

fn protocol_reset(arguments: &[Value]) -> Result<Value, String> {
    match arguments {
        [Value::Atom(atom), value] => atom.reset(value.clone()),
        _ => Err("IReset/reset expects an atom and value".into()),
    }
}

fn protocol_cas(arguments: &[Value]) -> Result<Value, String> {
    match arguments {
        [Value::Atom(atom), old_value, new_value] => Ok(Value::Bool(
            atom.compare_and_set(old_value, new_value.clone())?,
        )),
        _ => Err("ICas/cas expects an atom, old value, and new value".into()),
    }
}

fn protocol_reduce(arguments: &[Value]) -> Result<Value, String> {
    let (source, function, mut accumulator) = match arguments {
        [source, Value::Function(function), initial] => (source, function, Some(initial.clone())),
        [source, Value::Function(function)] => (source, function, None),
        _ => {
            return Err(
                "IReduce/reduce expects a value, function, and optional initial value".into(),
            )
        }
    };
    let iterator = make_iterator(source.clone())?;
    loop {
        match iterator_next(&iterator) {
            Ok(value) => {
                accumulator = Some(match accumulator {
                    Some(current) => call_function(function, vec![current, value])?,
                    None => value,
                });
            }
            Err(error) if error.contains("end") => break,
            Err(error) => return Err(error),
        }
    }
    accumulator.ok_or_else(|| "IReduce/reduce cannot reduce an empty value without init".into())
}

fn protocol_promise_state(arguments: &[Value]) -> Result<Value, String> {
    match arguments {
        [Value::Promise(promise)] => Ok(promise_state_value(promise)),
        _ => Err("IPromise/state expects a promise".into()),
    }
}

fn protocol_promise_value(arguments: &[Value]) -> Result<Value, String> {
    match arguments {
        [Value::Promise(promise)] => promise_value_result(promise),
        _ => Err("IPromise/value expects a promise".into()),
    }
}

fn protocol_promise_chain(operation: &str, arguments: &[Value]) -> Result<Value, String> {
    match arguments {
        [Value::Promise(promise), Value::Function(function)] => Ok(Value::Promise(promise_chain(
            promise.clone(),
            operation,
            function.clone(),
        ))),
        _ => Err(format!(
            "IPromise/{operation} expects a promise and function"
        )),
    }
}

fn protocol_promise_cancel(arguments: &[Value]) -> Result<Value, String> {
    match arguments {
        [Value::Promise(promise)] => {
            promise.cancel();
            Ok(Value::Promise(promise.clone()))
        }
        _ => Err("IPromise/cancel expects a promise".into()),
    }
}

fn protocol_coroutine_status(arguments: &[Value]) -> Result<Value, String> {
    match arguments {
        [Value::Coroutine(coroutine)] => Ok(coroutine_status(coroutine)),
        _ => Err("ICoroutine/status expects a coroutine".into()),
    }
}

fn protocol_watch_add(arguments: &[Value]) -> Result<Value, String> {
    match arguments {
        [Value::Atom(atom), key, Value::Function(function)] => {
            atom.add_watch(key.clone(), function.clone())?;
            Ok(Value::Atom(atom.clone()))
        }
        _ => Err("IWatch/watch-add expects an atom, key, and function".into()),
    }
}

fn protocol_watch_remove(arguments: &[Value]) -> Result<Value, String> {
    match arguments {
        [Value::Atom(atom), key] => {
            atom.remove_watch(key)?;
            Ok(Value::Atom(atom.clone()))
        }
        _ => Err("IWatch/watch-remove expects an atom and key".into()),
    }
}

fn protocol_watch_list(arguments: &[Value]) -> Result<Value, String> {
    match arguments {
        [Value::Atom(atom)] => Ok(iterator_from_values(atom.watch_entries()?)),
        _ => Err("IWatch/watch-list expects an atom".into()),
    }
}

fn protocol_empty(arguments: &[Value]) -> Result<Value, String> {
    match arguments {
        [value] => collection_empty_value(value.clone()),
        _ => Err("IEmpty/empty expects one collection".into()),
    }
}

fn protocol_equality(arguments: &[Value]) -> Result<Value, String> {
    match arguments {
        [left, right] => Ok(Value::Bool(left == right)),
        _ => Err("IEquality/equality expects two values".into()),
    }
}

fn protocol_display(arguments: &[Value]) -> Result<Value, String> {
    match arguments {
        [value] => Ok(Value::String(value.display())),
        _ => Err("IDisplay/display expects one value".into()),
    }
}

fn protocol_hash(arguments: &[Value]) -> Result<Value, String> {
    match arguments {
        [value] => Ok(Value::Number(value.stable_hash() as i64)),
        _ => Err("IHash/hash expects one value".into()),
    }
}

fn protocol_invoke(arguments: &[Value]) -> Result<Value, String> {
    match arguments {
        [callable, rest @ ..] => call_value(callable.clone(), rest.to_vec()),
        _ => Err("IFn/invoke expects a callable receiver".into()),
    }
}

fn protocol_pair_key(arguments: &[Value]) -> Result<Value, String> {
    match arguments {
        [value] => pair_parts(value)
            .map(|(key, _)| key)
            .ok_or_else(|| "IPair/key has no implementation for this value".into()),
        _ => Err("IPair/key expects one pair".into()),
    }
}

fn protocol_pair_value(arguments: &[Value]) -> Result<Value, String> {
    match arguments {
        [value] => pair_parts(value)
            .map(|(_, value)| value)
            .ok_or_else(|| "IPair/value has no implementation for this value".into()),
        _ => Err("IPair/value expects one pair".into()),
    }
}

fn protocol_peek_first(arguments: &[Value]) -> Result<Value, String> {
    match arguments {
        [value] => collection_first(value.clone()),
        _ => Err("IPeekFirst/peek-first expects one collection".into()),
    }
}

fn protocol_peek_last(arguments: &[Value]) -> Result<Value, String> {
    match arguments {
        [value] => collection_last(value.clone()),
        _ => Err("IPeekLast/peek-last expects one collection".into()),
    }
}

fn protocol_cons(arguments: &[Value]) -> Result<Value, String> {
    let [collection, item] = arguments else {
        return Err("ICons/cons expects a collection and value".into());
    };
    match collection {
        Value::Cons(values) => Ok(Value::Cons(Box::new(
            PCons::new(item.clone(), values.iter().collect()).with_meta(values.meta().cloned()),
        ))),
        Value::Tuple(values) => Ok(Value::Cons(Box::new(PCons::new(
            item.clone(),
            values.iter().cloned().collect(),
        )))),
        Value::Vector(values) => Ok(Value::Cons(Box::new(PCons::new(
            item.clone(),
            values.iter().cloned().collect(),
        )))),
        Value::List(values) => Ok(Value::Cons(Box::new(PCons::new(
            item.clone(),
            values.clone(),
        )))),
        Value::Nil => Ok(Value::Cons(Box::new(PCons::new(
            item.clone(),
            PList::new(),
        )))),
        _ => Err("ICons/cons has no implementation for this value".into()),
    }
}

fn tuple_push_last(values: &PTuple<Value>, item: Value) -> Result<Value, String> {
    if values.len() < 8 {
        return Ok(Value::Tuple(Box::new(values.push_last(item)?)));
    }
    Ok(Value::Vector(
        PVector::from_iter(values.iter().cloned().chain(std::iter::once(item)))
            .with_meta(values.meta().cloned()),
    ))
}

fn tuple_push_first(values: &PTuple<Value>, item: Value) -> Result<Value, String> {
    if values.len() < 8 {
        return Ok(Value::Tuple(Box::new(values.push_first(item)?)));
    }
    Ok(Value::Vector(
        PVector::from_iter(std::iter::once(item).chain(values.iter().cloned()))
            .with_meta(values.meta().cloned()),
    ))
}

fn protocol_conj(arguments: &[Value]) -> Result<Value, String> {
    if arguments.len() != 2 {
        return Err("IConj/conj expects a collection and value".into());
    }
    let collection = &arguments[0];
    let item = &arguments[1];
    match collection {
        Value::Tuple(values) => tuple_push_last(values, item.clone()),
        Value::Vector(values) => {
            let output = values.push_last(item.clone());
            Ok(Value::Vector(output))
        }
        Value::Queue(values) => Ok(Value::Queue(Box::new(values.push_last(item.clone())))),
        Value::Cons(values) => Ok(Value::Cons(Box::new(
            PCons::new(item.clone(), values.iter().collect()).with_meta(values.meta().cloned()),
        ))),
        Value::List(values) => {
            let output = std::iter::once(item.clone())
                .chain(values.iter().cloned())
                .collect();
            Ok(Value::List(output))
        }
        value @ (Value::Set(_) | Value::OrderedSet(_) | Value::SortedSet(_)) => {
            set_conj_value(value, item.clone())
        }
        value @ (Value::Map(_) | Value::OrderedMap(_) | Value::SortedMap(_) | Value::Trie(_)) => {
            let (entry_key, entry_value) = pair_parts(item)
                .ok_or_else(|| "IConj/conj map expects a two-element entry".to_string())?;
            map_assoc_value(value, entry_key, entry_value)
        }
        _ => Err("IConj/conj expects a collection".into()),
    }
}

fn protocol_call(protocol: &str, method: &str, arguments: &[Value]) -> Result<Value, String> {
    ACTIVE_PROTOCOLS.with(|active| {
        active
            .borrow()
            .as_ref()
            .cloned()
            .unwrap_or_else(ProtocolRegistry::core)
            .invoke(protocol, method, arguments)
    })
}

fn promise_value(value: &Value, operation: &str) -> Result<Promise, String> {
    match value {
        Value::Promise(promise) => Ok(promise.clone()),
        _ => Err(format!("{operation} expects a promise")),
    }
}

fn promise_state_value(promise: &Promise) -> Value {
    Value::Keyword(
        match promise.state() {
            PromiseState::Pending => "pending",
            PromiseState::Fulfilled(_) => "fulfilled",
            PromiseState::Rejected(error) if error == "cancelled" => "cancelled",
            PromiseState::Rejected(_) => "rejected",
        }
        .into(),
    )
}

fn promise_value_result(promise: &Promise) -> Result<Value, String> {
    match promise.state() {
        PromiseState::Pending => Err("promise is pending".into()),
        PromiseState::Fulfilled(value) => Ok(value),
        PromiseState::Rejected(error) => Err(error),
    }
}

fn promise_from(value: Value) -> Promise {
    match value {
        Value::Promise(promise) => promise,
        value => {
            let promise = Promise::new();
            promise.resolve(value);
            promise
        }
    }
}

fn promise_all(values: Vec<Value>) -> Promise {
    let output = Promise::new();
    if values.is_empty() {
        output.resolve(Value::Array(Rc::new(RefCell::new(Vec::new()))));
        return output;
    }
    let count = values.len();
    let remaining = Rc::new(Cell::new(count));
    let results = Rc::new(RefCell::new(vec![Value::Nil; count]));
    for (index, value) in values.into_iter().enumerate() {
        let source = match value {
            Value::Promise(promise) => promise,
            value => {
                let promise = Promise::new();
                promise.resolve(value);
                promise
            }
        };
        let destination = output.clone();
        let remaining = remaining.clone();
        let results = results.clone();
        source.on_settle(Rc::new(move |state| match state {
            PromiseState::Fulfilled(value) => {
                results.borrow_mut()[index] = value;
                let left = remaining.get() - 1;
                remaining.set(left);
                if left == 0 {
                    destination.resolve(Value::Array(Rc::new(RefCell::new(
                        results.borrow().clone(),
                    ))));
                }
            }
            PromiseState::Rejected(error) => {
                destination.reject(error);
            }
            PromiseState::Pending => {}
        }));
    }
    output
}
fn settle_promise_result(destination: &Promise, result: Result<Value, String>) {
    match result {
        Ok(Value::Promise(source)) => {
            destination.adopt(&source);
        }
        Ok(value) => {
            destination.resolve(value);
        }
        Err(error) => {
            destination.reject(error);
        }
    }
}

fn finish_promise(destination: Promise, original: PromiseState, cleanup: Result<Value, String>) {
    let preserved_destination = destination.clone();
    let preserve = move || match original.clone() {
        PromiseState::Fulfilled(value) => {
            preserved_destination.resolve(value);
        }
        PromiseState::Rejected(error) => {
            preserved_destination.reject(error);
        }
        PromiseState::Pending => {}
    };
    match cleanup {
        Ok(Value::Promise(cleanup)) => {
            cleanup.on_settle(Rc::new(move |state| match state {
                PromiseState::Fulfilled(_) => preserve(),
                PromiseState::Rejected(error) => {
                    destination.reject(error);
                }
                PromiseState::Pending => {}
            }));
        }
        Ok(_) => preserve(),
        Err(error) => {
            destination.reject(error);
        }
    }
}

fn promise_chain(source: Promise, operation: &str, function: Rc<Function>) -> Promise {
    let output = Promise::new();
    let operation = operation.to_string();
    let destination = output.clone();
    source.on_settle(Rc::new(move |state| match state.clone() {
        PromiseState::Fulfilled(value) if operation == "promise/then" => {
            settle_promise_result(&destination, call_function(&function, vec![value]));
        }
        PromiseState::Rejected(error) if operation == "promise/catch" => {
            settle_promise_result(
                &destination,
                call_function(&function, vec![Value::String(error)]),
            );
        }
        PromiseState::Fulfilled(_) | PromiseState::Rejected(_)
            if operation == "promise/finally" =>
        {
            finish_promise(
                destination.clone(),
                state,
                call_function(&function, Vec::new()),
            );
        }
        PromiseState::Fulfilled(value) => {
            destination.resolve(value);
        }
        PromiseState::Rejected(error) => {
            destination.reject(error);
        }
        PromiseState::Pending => {}
    }));
    output
}

fn string_value<'a>(value: &'a Value, operation: &str) -> Result<&'a str, String> {
    match value {
        Value::String(value) => Ok(value),
        _ => Err(format!("{operation} expects a string")),
    }
}

fn code_point_length(text: &str) -> usize {
    text.chars().count()
}

fn code_point_slice(text: &str, start: usize, end: usize) -> Result<String, String> {
    let length = text.chars().count();
    if start > end || end > length {
        return Err("str/slice range is out of bounds".into());
    }
    Ok(text.chars().skip(start).take(end - start).collect())
}

fn code_point_char_at(text: &str, index: usize) -> Result<String, String> {
    text.chars()
        .nth(index)
        .map(|ch| ch.to_string())
        .ok_or_else(|| "str/char-at index out of bounds".into())
}

fn code_point_byte_index(text: &str, code_point_offset: usize) -> usize {
    text.char_indices()
        .nth(code_point_offset)
        .map(|(byte_index, _)| byte_index)
        .unwrap_or(text.len())
}

fn code_point_index(text: &str, byte_index: usize) -> usize {
    text[..byte_index.min(text.len())].chars().count()
}

fn code_point_index_of(text: &str, part: &str, offset: usize) -> i64 {
    let byte_offset = code_point_byte_index(text, offset);
    text[byte_offset..]
        .find(part)
        .map(|index| (code_point_index(text, byte_offset + index)) as i64)
        .unwrap_or(-1)
}

fn code_point_last_index_of(text: &str, part: &str, offset: usize) -> i64 {
    let len = code_point_length(text);
    if part.is_empty() {
        return (offset.min(len)) as i64;
    }
    let mut code_point_index = 0;
    let mut last: Option<usize> = None;
    for (byte_index, _) in text.char_indices() {
        if code_point_index > offset && offset < len {
            break;
        }
        if text[byte_index..].starts_with(part) {
            last = Some(code_point_index);
        }
        code_point_index += 1;
    }
    last.map(|index| index as i64).unwrap_or(-1)
}

fn string_operation(operation: &str, values: Vec<Value>) -> Result<Value, String> {
    let pair = |values: &[Value]| -> Result<(String, String), String> {
        if values.len() != 2 {
            return Err(format!("{operation} expects two strings"));
        }
        Ok((
            string_value(&values[0], operation)?.to_owned(),
            string_value(&values[1], operation)?.to_owned(),
        ))
    };
    match operation {
        "str/comp" | "str/lt?" | "str/gt?" => {
            let (left, right) = pair(&values)?;
            let ordering = left.cmp(&right);
            Ok(match operation {
                "str/comp" => Value::Number(match ordering {
                    std::cmp::Ordering::Less => -1,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => 1,
                }),
                "str/lt?" => Value::Bool(ordering.is_lt()),
                _ => Value::Bool(ordering.is_gt()),
            })
        }
        "str/starts-with?" | "str/ends-with?" => {
            let (text, part) = pair(&values)?;
            Ok(Value::Bool(if operation == "str/starts-with?" {
                text.starts_with(&part)
            } else {
                text.ends_with(&part)
            }))
        }
        "str/includes?" => {
            let (text, part) = pair(&values)?;
            Ok(Value::Bool(text.contains(&part)))
        }
        "str/pad-left" | "str/pad-right" => {
            if values.len() != 3 {
                return Err(format!(
                    "{operation} expects a string, length, and padding string"
                ));
            }
            let text = string_value(&values[0], operation)?;
            let length = value_index(&values[1])?;
            let padding = string_value(&values[2], operation)?;
            let text_length = code_point_length(text);
            if padding.is_empty() || text_length >= length {
                return Ok(Value::String(text.into()));
            }
            let needed = length - text_length;
            let padding_chars: Vec<char> = padding.chars().collect();
            let fill: String = padding_chars.iter().cycle().take(needed).copied().collect();
            Ok(Value::String(if operation == "str/pad-left" {
                format!("{fill}{text}")
            } else {
                format!("{text}{fill}")
            }))
        }
        "str/char-at" | "str/char" => {
            if values.len() != 2 {
                return Err("str/char-at expects a string and index".into());
            }
            let text = string_value(&values[0], operation)?;
            let index = value_index(&values[1])?;
            code_point_char_at(text, index).map(Value::String)
        }
        "str/split" => {
            let (text, separator) = pair(&values)?;
            let parts = text
                .split(&separator)
                .map(|part| Value::String(part.into()))
                .collect();
            Ok(Value::Array(Rc::new(RefCell::new(parts))))
        }
        "str/split-lines" => {
            if values.len() != 1 {
                return Err("str/split-lines expects one string".into());
            }
            let text = string_value(&values[0], operation)?;
            let parts = text
                .split('\n')
                .map(|part| Value::String(part.into()))
                .collect();
            Ok(Value::Array(Rc::new(RefCell::new(parts))))
        }
        "str/join" => {
            if values.len() != 2 {
                return Err("str/join expects a separator and collection".into());
            }
            let separator = string_value(&values[0], operation)?;
            let parts = iterator_values(values[1].clone())?
                .into_iter()
                .map(|value| match value {
                    Value::String(value) => Ok(value),
                    _ => Err("str/join expects a collection of strings".into()),
                })
                .collect::<Result<Vec<String>, String>>()?;
            Ok(Value::String(parts.join(separator)))
        }
        "str/index-of" => {
            if values.len() != 2 && values.len() != 3 {
                return Err("str/index-of expects a string, substring, and optional offset".into());
            }
            let text = string_value(&values[0], operation)?;
            let part = string_value(&values[1], operation)?;
            let offset = if values.len() == 3 {
                value_index(&values[2])?
            } else {
                0
            };
            Ok(Value::Number(code_point_index_of(text, part, offset)))
        }
        "str/last-index-of" => {
            if values.len() != 2 && values.len() != 3 {
                return Err(
                    "str/last-index-of expects a string, substring, and optional offset".into(),
                );
            }
            let text = string_value(&values[0], operation)?;
            let part = string_value(&values[1], operation)?;
            let offset = if values.len() == 3 {
                value_index(&values[2])?
            } else {
                code_point_length(text)
            };
            Ok(Value::Number(code_point_last_index_of(text, part, offset)))
        }
        "str/slice" | "str/substring" => {
            if values.len() != 2 && values.len() != 3 {
                return Err("str/slice expects a string, start, and optional end".into());
            }
            let text = string_value(&values[0], operation)?;
            let start = value_index(&values[1])?;
            let end = if values.len() == 3 {
                value_index(&values[2])?
            } else {
                code_point_length(text)
            };
            code_point_slice(text, start, end).map(Value::String)
        }
        "str/to-fixed" => {
            if values.len() != 2 {
                return Err("str/to-fixed expects a number and precision".into());
            }
            let number = match values[0] {
                Value::Number(number) => number as f64,
                _ => return Err("str/to-fixed expects a number and precision".into()),
            };
            let precision = value_index(&values[1])?;
            if precision > 100 {
                return Err("str/to-fixed precision must be in the range 0..100".into());
            }
            Ok(Value::String(format!("{number:.precision$}")))
        }
        "str/replace" => {
            if values.len() != 3 {
                return Err("str/replace expects a string, match, and replacement".into());
            }
            Ok(Value::String(string_value(&values[0], operation)?.replace(
                string_value(&values[1], operation)?,
                string_value(&values[2], operation)?,
            )))
        }
        "str/replace-first" => {
            if values.len() != 3 {
                return Err("str/replace-first expects a string, match, and replacement".into());
            }
            let text = string_value(&values[0], operation)?;
            let part = string_value(&values[1], operation)?;
            let replacement = string_value(&values[2], operation)?;
            Ok(Value::String(text.replacen(part, replacement, 1)))
        }
        "str/trim" | "str/trim-left" | "str/trim-right" => {
            if values.len() != 1 {
                return Err(format!("{operation} expects one string"));
            }
            let text = string_value(&values[0], operation)?;
            Ok(Value::String(match operation {
                "str/trim" => text.trim().into(),
                "str/trim-left" => text.trim_start().into(),
                _ => text.trim_end().into(),
            }))
        }
        "str/length" | "str/len" | "str/count" => {
            if values.len() != 1 {
                return Err(format!("{operation} expects one string"));
            }
            let text = string_value(&values[0], operation)?;
            Ok(Value::Number(code_point_length(text) as i64))
        }
        "str/blank?" => {
            if values.len() != 1 {
                return Err("str/blank? expects one string".into());
            }
            let text = string_value(&values[0], operation)?;
            Ok(Value::Bool(text.trim().is_empty()))
        }
        "str/repeat" => {
            if values.len() != 2 {
                return Err("str/repeat expects a string and count".into());
            }
            let text = string_value(&values[0], operation)?;
            let count = value_index(&values[1])?;
            Ok(Value::String(text.repeat(count)))
        }
        "str/capitalize" => {
            if values.len() != 1 {
                return Err("str/capitalize expects one string".into());
            }
            let text = string_value(&values[0], operation)?;
            let mut chars = text.chars();
            match chars.next() {
                Some(first) => Ok(Value::String(
                    first.to_uppercase().collect::<String>() + chars.as_str(),
                )),
                None => Ok(Value::String(text.into())),
            }
        }
        "str/decapitalize" => {
            if values.len() != 1 {
                return Err("str/decapitalize expects one string".into());
            }
            let text = string_value(&values[0], operation)?;
            let mut chars = text.chars();
            match chars.next() {
                Some(first) => Ok(Value::String(
                    first.to_lowercase().collect::<String>() + chars.as_str(),
                )),
                None => Ok(Value::String(text.into())),
            }
        }
        "str/upper" | "str/to-upper" => {
            if values.len() != 1 {
                return Err(format!("{operation} expects one string"));
            }
            let text = string_value(&values[0], operation)?;
            Ok(Value::String(text.to_uppercase()))
        }
        "str/lower" | "str/to-lower" => {
            if values.len() != 1 {
                return Err(format!("{operation} expects one string"));
            }
            let text = string_value(&values[0], operation)?;
            Ok(Value::String(text.to_lowercase()))
        }
        "str/reverse" => {
            if values.len() != 1 {
                return Err("str/reverse expects one string".into());
            }
            let text = string_value(&values[0], operation)?;
            Ok(Value::String(text.chars().rev().collect()))
        }
        "str/encode-utf8" | "str/encode" => {
            if values.len() != 1 {
                return Err(format!("{operation} expects one string"));
            }
            match &values[0] {
                Value::String(text) => Ok(Value::ByteBuffer(Rc::new(RefCell::new(
                    text.as_bytes().to_vec(),
                )))),
                _ => Err(format!("{operation} expects a string")),
            }
        }
        "str/decode-utf8" | "str/decode" => {
            if values.len() != 1 {
                return Err(format!("{operation} expects bytes"));
            }
            let bytes = byte_buffer(&values[0], operation)?;
            let raw = bytes.borrow().clone();
            String::from_utf8(raw)
                .map(Value::String)
                .map_err(|_| format!("{operation} invalid UTF-8"))
        }
        _ => Err(format!("unknown string operation: {operation}")),
    }
}
fn marker_key(value: &Value, operation: &str) -> Result<String, String> {
    match value {
        Value::String(key) => Ok(key.clone()),
        Value::Keyword(key) => Ok(key.as_str().to_owned()),
        _ => Err(format!("{operation} expects a string key")),
    }
}

fn dot_call(
    receiver: Value,
    method: &Form,
    env: &mut HashMap<String, Value>,
) -> Result<Value, String> {
    let parts = match method {
        Form::List(parts) if !parts.is_empty() => parts,
        _ => return Err("dot call expects a method list".into()),
    };
    let name = match &parts[0] {
        Form::Symbol(name) => name.as_str(),
        _ => return Err("dot method must be a symbol".into()),
    };
    let args = parts[1..]
        .iter()
        .map(|form| eval(form, env))
        .collect::<Result<Vec<_>, _>>()?;
    match receiver {
        Value::Array(array) => match name {
            "get" => {
                if args.len() < 1 || args.len() > 2 {
                    return Err("array/get expects an index and optional default".into());
                }
                let index = value_index(&args[0])?;
                Ok(array
                    .borrow()
                    .get(index)
                    .cloned()
                    .or_else(|| args.get(1).cloned())
                    .unwrap_or(Value::Nil))
            }
            "set" => {
                if args.len() != 2 {
                    return Err("array/set expects an index and value".into());
                }
                let index = value_index(&args[0])?;
                let mut values = array.borrow_mut();
                if index >= values.len() {
                    return Err("array/set index out of bounds".into());
                }
                values[index] = args[1].clone();
                drop(values);
                Ok(Value::Array(array))
            }
            "push-first" => {
                if args.len() != 1 {
                    return Err("array/push-first expects one value".into());
                }
                array.borrow_mut().insert(0, args[0].clone());
                Ok(Value::Array(array))
            }
            "push-last" => {
                if args.len() != 1 {
                    return Err("array/push-last expects one value".into());
                }
                array.borrow_mut().push(args[0].clone());
                Ok(Value::Array(array))
            }
            "pop-first" => {
                if !args.is_empty() {
                    return Err("array/pop-first expects no arguments".into());
                }
                let mut values = array.borrow_mut();
                Ok(if values.is_empty() {
                    Value::Nil
                } else {
                    values.remove(0)
                })
            }
            "pop-last" => {
                if !args.is_empty() {
                    return Err("array/pop-last expects no arguments".into());
                }
                Ok(array.borrow_mut().pop().unwrap_or(Value::Nil))
            }
            "insert" => {
                if args.len() != 2 {
                    return Err("array/insert expects an index and value".into());
                }
                let index = value_index(&args[0])?;
                let mut values = array.borrow_mut();
                if index > values.len() {
                    return Err("array/insert index out of bounds".into());
                }
                values.insert(index, args[1].clone());
                drop(values);
                Ok(Value::Array(array))
            }
            "remove" => {
                if args.len() != 1 {
                    return Err("array/remove expects an index".into());
                }
                let index = value_index(&args[0])?;
                let mut values = array.borrow_mut();
                if index >= values.len() {
                    return Err("array/remove index out of bounds".into());
                }
                Ok(values.remove(index))
            }
            "clone" => {
                if !args.is_empty() {
                    return Err("array/clone expects no arguments".into());
                }
                Ok(Value::Array(Rc::new(RefCell::new(array.borrow().clone()))))
            }
            "slice" => {
                if args.is_empty() || args.len() > 2 {
                    return Err("array/slice expects start and optional end".into());
                }
                let start = value_index(&args[0])?;
                let end = if args.len() == 2 {
                    value_index(&args[1])?
                } else {
                    array.borrow().len()
                };
                let values = array.borrow();
                if start > end || end > values.len() {
                    return Err("array/slice range is out of bounds".into());
                }
                Ok(Value::Array(Rc::new(RefCell::new(
                    values[start..end].to_vec(),
                ))))
            }
            "map" | "filter" => {
                if args.len() != 1 {
                    return Err(format!("array/{name} expects one function"));
                }
                let function = match &args[0] {
                    Value::Function(function) => function,
                    _ => return Err(format!("array/{name} expects a function")),
                };
                let mut output = Vec::new();
                for value in array.borrow().iter().cloned() {
                    let mapped = call_function(function, vec![value.clone()])?;
                    if name == "map" {
                        output.push(mapped);
                    } else if mapped.truthy() {
                        output.push(value);
                    }
                }
                Ok(Value::Array(Rc::new(RefCell::new(output))))
            }
            "fold-left" | "fold-right" => {
                if args.len() != 2 {
                    return Err(format!("array/{name} expects a function and initial value"));
                }
                let function = match &args[0] {
                    Value::Function(function) => function,
                    _ => return Err(format!("array/{name} expects a function")),
                };
                let values = array.borrow();
                let mut output = args[1].clone();
                if name == "fold-left" {
                    for value in values.iter().cloned() {
                        output = call_function(function, vec![output, value])?;
                    }
                } else {
                    for value in values.iter().rev().cloned() {
                        output = call_function(function, vec![value, output])?;
                    }
                }
                Ok(output)
            }
            _ => Err(format!("unsupported array method: {name}")),
        },
        Value::Object(object) => match name {
            "has?" => {
                if args.len() != 1 {
                    return Err("object/has? expects a key".into());
                }
                let key = marker_key(&args[0], "object/has?")?;
                Ok(Value::Bool(
                    object
                        .borrow()
                        .iter()
                        .any(|(candidate, _)| candidate == &key),
                ))
            }
            "get" => {
                if args.len() < 1 || args.len() > 2 {
                    return Err("object/get expects a key and optional default".into());
                }
                let key = marker_key(&args[0], "object/get")?;
                Ok(object
                    .borrow()
                    .iter()
                    .find(|(candidate, _)| candidate == &key)
                    .map(|(_, value)| value.clone())
                    .or_else(|| args.get(1).cloned())
                    .unwrap_or(Value::Nil))
            }
            "set" => {
                if args.len() != 2 {
                    return Err("object/set expects a key and value".into());
                }
                let key = marker_key(&args[0], "object/set")?;
                let mut values = object.borrow_mut();
                if let Some((_, value)) = values.iter_mut().find(|(candidate, _)| candidate == &key)
                {
                    *value = args[1].clone();
                } else {
                    values.push((key, args[1].clone()));
                }
                drop(values);
                Ok(Value::Object(object))
            }
            "delete" => {
                if args.len() != 1 {
                    return Err("object/delete expects a key".into());
                }
                let key = marker_key(&args[0], "object/delete")?;
                let mut values = object.borrow_mut();
                if let Some(index) = values.iter().position(|(candidate, _)| candidate == &key) {
                    Ok(values.remove(index).1)
                } else {
                    Ok(Value::Nil)
                }
            }
            "keys" | "vals" | "pairs" => {
                if !args.is_empty() {
                    return Err(format!("object/{name} expects no arguments"));
                }
                let output = object
                    .borrow()
                    .iter()
                    .map(|(key, value)| match name {
                        "keys" => Value::String(key.clone()),
                        "vals" => value.clone(),
                        _ => Value::Array(Rc::new(RefCell::new(vec![
                            Value::String(key.clone()),
                            value.clone(),
                        ]))),
                    })
                    .collect();
                Ok(Value::Array(Rc::new(RefCell::new(output))))
            }
            "assign" => {
                if args.len() != 1 {
                    return Err("object/assign expects an object".into());
                }
                let other = match &args[0] {
                    Value::Object(other) => other.clone(),
                    _ => return Err("object/assign expects an object".into()),
                };
                let mut values = object.borrow_mut();
                for (key, value) in other.borrow().iter() {
                    if let Some((_, existing)) =
                        values.iter_mut().find(|(candidate, _)| candidate == key)
                    {
                        *existing = value.clone();
                    } else {
                        values.push((key.clone(), value.clone()));
                    }
                }
                drop(values);
                Ok(Value::Object(object))
            }
            "clone" => {
                if !args.is_empty() {
                    return Err("object/clone expects no arguments".into());
                }
                Ok(Value::Object(Rc::new(RefCell::new(
                    object.borrow().clone(),
                ))))
            }
            _ => Err(format!("unsupported object method: {name}")),
        },
        _ => Err("dot calls require an array or object marker".into()),
    }
}

fn byte_input(value: &Value, operation: &str) -> Result<u8, String> {
    match value {
        Value::Number(number) if (-128..=255).contains(number) => Ok((*number as i8) as u8),
        _ => Err(format!(
            "{operation} expects a value in the range -128..255"
        )),
    }
}

fn byte_buffer(value: &Value, operation: &str) -> Result<Rc<RefCell<Vec<u8>>>, String> {
    match value {
        Value::ByteBuffer(bytes) => Ok(bytes.clone()),
        _ => Err(format!("{operation} expects bytes")),
    }
}

fn byte_count(value: &Value) -> Result<Value, String> {
    match value {
        Value::Bytes(bytes) => Ok(Value::Number(bytes.len() as i64)),
        Value::ByteBuffer(bytes) => Ok(Value::Number(bytes.borrow().len() as i64)),
        _ => Err("bytes/count expects bytes".into()),
    }
}

fn byte_get(value: &Value, index: &Value, default: Option<Value>) -> Result<Value, String> {
    let index = value_index(index)?;
    let found = match value {
        Value::Bytes(bytes) => bytes.get(index).copied(),
        Value::ByteBuffer(bytes) => bytes.borrow().get(index).copied(),
        _ => return Err("bytes/get expects bytes".into()),
    };
    match found {
        Some(byte) => Ok(Value::Number(byte as i64)),
        None => default.ok_or_else(|| "bytes/get index out of bounds".into()),
    }
}

fn byte_copy(value: &Value) -> Result<Value, String> {
    let bytes = byte_buffer(value, "bytes/copy")?;
    let copied = bytes.borrow().clone();
    Ok(Value::ByteBuffer(Rc::new(RefCell::new(copied))))
}

fn byte_slice(value: &Value, start: &Value, end: &Value) -> Result<Value, String> {
    let start = value_index(start)?;
    let end = value_index(end)?;
    let bytes = byte_buffer(value, "bytes/slice")?;
    let bytes = bytes.borrow();
    if start > end || end > bytes.len() {
        return Err(format!(
            "bytes/slice range is out of bounds: {start}..{end}"
        ));
    }
    Ok(Value::ByteBuffer(Rc::new(RefCell::new(
        bytes[start..end].to_vec(),
    ))))
}

fn byte_set(value: &Value, index: &Value, item: &Value) -> Result<Value, String> {
    let index = value_index(index)?;
    let item = byte_input(item, "bytes/set")?;
    let bytes = byte_buffer(value, "bytes/set")?;
    let mut bytes = bytes.borrow_mut();
    if index >= bytes.len() {
        return Err("bytes/set index out of bounds".into());
    }
    bytes[index] = item;
    Ok(value.clone())
}

fn iterator_values(value: Value) -> Result<Vec<Value>, String> {
    match value {
        Value::Nil => Ok(Vec::new()),
        Value::Tuple(values) => Ok(values.iter().cloned().collect()),
        Value::Vector(values) => Ok(values.iter().cloned().collect()),
        Value::List(values) => Ok(values.iter().cloned().collect()),
        Value::Cons(values) => Ok(values.iter().collect()),
        Value::Queue(values) => Ok(values.iter().cloned().collect()),
        Value::String(text) => Ok(text.chars().map(|c| Value::String(c.to_string())).collect()),
        Value::Bytes(bytes) => Ok(bytes
            .into_iter()
            .map(|byte| Value::Number(byte as i8 as i64))
            .collect()),
        Value::ByteBuffer(bytes) => Ok(bytes
            .borrow()
            .iter()
            .map(|byte| Value::Number(*byte as i8 as i64))
            .collect()),
        Value::Array(values) => Ok(values.borrow().clone()),
        Value::Object(values) => Ok(values
            .borrow()
            .iter()
            .map(|(key, value)| {
                Value::Vector(PVector::from_iter([
                    Value::String(key.clone()),
                    value.clone(),
                ]))
            })
            .collect()),
        value @ (Value::Map(_) | Value::OrderedMap(_) | Value::SortedMap(_) | Value::Trie(_)) => {
            Ok(map_entries(&value)
                .unwrap()
                .into_iter()
                .map(|(key, value)| Value::Vector(PVector::from_iter([key, value])))
                .collect())
        }
        value @ (Value::Set(_) | Value::OrderedSet(_) | Value::SortedSet(_)) => {
            Ok(set_items(&value).unwrap().into_iter().cloned().collect())
        }
        Value::Iterator(iterator) => {
            let mut state = iterator.borrow_mut();
            if state.closed {
                return Ok(Vec::new());
            }
            if state.generator.is_some() {
                return Err("cannot materialize an infinite iterator".into());
            }
            let values = state.values[state.index..].to_vec();
            state.index = state.values.len();
            Ok(values)
        }
        _ => Err("iter expects a collection".into()),
    }
}

fn iterator_to_vec(value: Value) -> Result<Vec<Value>, String> {
    if let Value::Iterator(_) = &value {
        if !iterator_is_finite(&value) {
            return Err("cannot materialize an infinite iterator".into());
        }
        let mut output = Vec::new();
        loop {
            match iterator_next(&value) {
                Ok(value) => output.push(value),
                Err(_) => break,
            }
        }
        return Ok(output);
    }
    iterator_values(value)
}

fn make_iterator(value: Value) -> Result<Value, String> {
    match &value {
        Value::Iterator(_) => Ok(value),
        Value::Nil
        | Value::String(_)
        | Value::Bytes(_)
        | Value::ByteBuffer(_)
        | Value::Array(_)
        | Value::Object(_)
        | Value::Map(_)
        | Value::OrderedMap(_)
        | Value::SortedMap(_)
        | Value::Trie(_)
        | Value::Set(_)
        | Value::OrderedSet(_)
        | Value::SortedSet(_)
        | Value::List(_)
        | Value::Cons(_)
        | Value::Queue(_)
        | Value::Tuple(_)
        | Value::Vector(_) => Ok(Value::Iterator(Rc::new(RefCell::new(IteratorState::new(
            iterator_values(value)?,
        ))))),
        _ => match protocol_call("IIter", "iter", &[value])? {
            Value::Iterator(iterator) => Ok(Value::Iterator(iterator)),
            _ => Err("IIter/iter must return an iterator".into()),
        },
    }
}

pub fn iterator_from_values(values: Vec<Value>) -> Value {
    Value::Iterator(Rc::new(RefCell::new(IteratorState::new(values))))
}

fn iterator_seq(value: Value) -> Result<Value, String> {
    match value {
        Value::Iterator(iterator) => {
            iterator.borrow_mut().seq = true;
            Ok(Value::Iterator(iterator))
        }
        value => {
            let values = iterator_values(value)?;
            let mut state = IteratorState::new(values);
            state.seq = true;
            Ok(Value::Iterator(Rc::new(RefCell::new(state))))
        }
    }
}

fn iterator_constant(value: Value) -> Value {
    Value::Iterator(Rc::new(RefCell::new(IteratorState::generated(
        IteratorGenerator::Constant(value),
    ))))
}
fn iterator_repeated(function: Rc<Function>) -> Value {
    Value::Iterator(Rc::new(RefCell::new(IteratorState::generated(
        IteratorGenerator::Repeated(function),
    ))))
}
fn iterator_iterate(function: Rc<Function>, seed: Value) -> Value {
    Value::Iterator(Rc::new(RefCell::new(IteratorState::generated(
        IteratorGenerator::Iterate(function, seed),
    ))))
}
fn iterator_take_while(function: Rc<Function>, value: Value) -> Result<Value, String> {
    let source = match value {
        Value::Iterator(iterator) => Value::Iterator(iterator),
        value => make_iterator(value)?,
    };
    Ok(Value::Iterator(Rc::new(RefCell::new(
        IteratorState::generated(IteratorGenerator::TakeWhile(function, source)),
    ))))
}
fn iterator_map(function: Rc<Function>, value: Value) -> Result<Value, String> {
    let source = match value {
        Value::Iterator(iterator) => Value::Iterator(iterator),
        value => make_iterator(value)?,
    };
    Ok(Value::Iterator(Rc::new(RefCell::new(
        IteratorState::generated(IteratorGenerator::Map(function, source)),
    ))))
}
fn iterator_partition(value: Value, amount: usize, all: bool) -> Result<Value, String> {
    if amount == 0 {
        return Err("partition amount must be positive".into());
    }
    let source = match value {
        Value::Iterator(iterator) => Value::Iterator(iterator),
        value => make_iterator(value)?,
    };
    Ok(Value::Iterator(Rc::new(RefCell::new(
        IteratorState::generated(IteratorGenerator::Partition(source, amount, all)),
    ))))
}

fn iterator_interleave(values: Vec<Value>) -> Result<Value, String> {
    let sources = values
        .into_iter()
        .map(|value| match value {
            Value::Iterator(iterator) => Ok(Value::Iterator(iterator)),
            value => make_iterator(value),
        })
        .collect::<Result<Vec<_>, _>>()?;
    if sources.iter().any(
        |value| matches!(value,Value::Iterator(iterator) if iterator.borrow().generator.is_some()),
    ) {
        return Ok(Value::Iterator(Rc::new(RefCell::new(
            IteratorState::generated(IteratorGenerator::Interleave(sources, 0)),
        ))));
    }
    let collections = sources
        .iter()
        .map(|value| iterator_values(value.clone()))
        .collect::<Result<Vec<_>, _>>()?;
    let limit = collections.iter().map(Vec::len).min().unwrap_or(0);
    let mut output = Vec::new();
    for index in 0..limit {
        for values in &collections {
            output.push(values[index].clone());
        }
    }
    Ok(iterator_from_values(output))
}

fn iterator_zip(values: Vec<Value>) -> Result<Value, String> {
    let sources = values
        .into_iter()
        .map(|value| match value {
            Value::Iterator(iterator) => Ok(Value::Iterator(iterator)),
            value => make_iterator(value),
        })
        .collect::<Result<Vec<_>, _>>()?;
    if sources.iter().any(
        |value| matches!(value,Value::Iterator(iterator) if iterator.borrow().generator.is_some()),
    ) {
        return Ok(Value::Iterator(Rc::new(RefCell::new(
            IteratorState::generated(IteratorGenerator::Zip(sources)),
        ))));
    }
    let collections = sources
        .iter()
        .map(|value| iterator_values(value.clone()))
        .collect::<Result<Vec<_>, _>>()?;
    let limit = collections.iter().map(Vec::len).min().unwrap_or(0);
    Ok(iterator_from_values(
        (0..limit)
            .map(|index| {
                Value::Vector(
                    collections
                        .iter()
                        .map(|values| values[index].clone())
                        .collect(),
                )
            })
            .collect(),
    ))
}

fn iterator_mapcat(function: Rc<Function>, value: Value) -> Result<Value, String> {
    let source = match value {
        Value::Iterator(iterator) => Value::Iterator(iterator),
        value => make_iterator(value)?,
    };
    if let Value::Iterator(iterator) = &source {
        if iterator.borrow().generator.is_none() {
            let values = iterator_values(source)?;
            let mut output = Vec::new();
            for value in values {
                output.extend(iterator_values(call_function(&function, vec![value])?)?);
            }
            return Ok(iterator_from_values(output));
        }
    }
    Ok(Value::Iterator(Rc::new(RefCell::new(
        IteratorState::generated(IteratorGenerator::Mapcat(function, source, None)),
    ))))
}
fn iterator_keep(function: Rc<Function>, value: Value) -> Result<Value, String> {
    let source = match value {
        Value::Iterator(iterator) => Value::Iterator(iterator),
        value => make_iterator(value)?,
    };
    if let Value::Iterator(iterator) = &source {
        if iterator.borrow().generator.is_none() {
            let values = iterator_values(source)?;
            let mut output = Vec::new();
            for value in values {
                let mapped = call_function(&function, vec![value])?;
                if !matches!(mapped, Value::Nil) {
                    output.push(mapped);
                }
            }
            return Ok(iterator_from_values(output));
        }
    }
    Ok(Value::Iterator(Rc::new(RefCell::new(
        IteratorState::generated(IteratorGenerator::Keep(function, source)),
    ))))
}

fn iterator_filter(function: Rc<Function>, value: Value) -> Result<Value, String> {
    let source = match value {
        Value::Iterator(iterator) => Value::Iterator(iterator),
        value => make_iterator(value)?,
    };
    if let Value::Iterator(iterator) = &source {
        if iterator.borrow().generator.is_none() {
            let values = iterator_values(source)?;
            return Ok(iterator_from_values(
                values
                    .into_iter()
                    .filter_map(
                        |value| match call_function(&function, vec![value.clone()]) {
                            Ok(result) if result.truthy() => Some(Ok(value)),
                            Ok(_) => None,
                            Err(error) => Some(Err(error)),
                        },
                    )
                    .collect::<Result<Vec<_>, _>>()?,
            ));
        }
    }
    Ok(Value::Iterator(Rc::new(RefCell::new(
        IteratorState::generated(IteratorGenerator::Filter(function, source)),
    ))))
}

fn iterator_drop_while(function: Rc<Function>, value: Value) -> Result<Value, String> {
    let source = match value {
        Value::Iterator(iterator) => Value::Iterator(iterator),
        value => make_iterator(value)?,
    };
    if let Value::Iterator(iterator) = &source {
        if iterator.borrow().generator.is_none() {
            let values = iterator_values(source)?;
            let mut output = Vec::new();
            let mut dropping = true;
            for value in values {
                if dropping && call_function(&function, vec![value.clone()])?.truthy() {
                    continue;
                }
                dropping = false;
                output.push(value);
            }
            return Ok(iterator_from_values(output));
        }
    }
    Ok(Value::Iterator(Rc::new(RefCell::new(
        IteratorState::generated(IteratorGenerator::DropWhile(function, source, false)),
    ))))
}
fn iterator_take(value: Value, amount: usize) -> Result<Value, String> {
    let source = match value {
        Value::Iterator(iterator) => Value::Iterator(iterator),
        value => make_iterator(value)?,
    };
    Ok(Value::Iterator(Rc::new(RefCell::new(
        IteratorState::generated(IteratorGenerator::Take(source, amount)),
    ))))
}
fn iterator_drop(value: Value, amount: usize) -> Result<Value, String> {
    let source = match value {
        Value::Iterator(iterator) => Value::Iterator(iterator),
        value => make_iterator(value)?,
    };
    Ok(Value::Iterator(Rc::new(RefCell::new(
        IteratorState::generated(IteratorGenerator::Drop(source, amount)),
    ))))
}

fn iterator_cycle(value: Value) -> Result<Value, String> {
    let source = match value {
        Value::Iterator(iterator) => Value::Iterator(iterator),
        value => make_iterator(value)?,
    };
    Ok(Value::Iterator(Rc::new(RefCell::new(
        IteratorState::generated(IteratorGenerator::Cycle(source, Vec::new(), 0, false)),
    ))))
}

fn iterator_has_next(value: &Value) -> Result<Value, String> {
    match value {
        Value::Iterator(iterator) => Ok(Value::Bool(iterator.borrow().has_next())),
        _ => Err("iter-has? expects an iterator".into()),
    }
}

fn iterator_next(value: &Value) -> Result<Value, String> {
    match value {
        Value::Iterator(iterator) => iterator.borrow_mut().next(),
        _ => Err("iter-next expects an iterator".into()),
    }
}

fn iterator_close(value: &Value) -> Result<Value, String> {
    match value {
        Value::Iterator(iterator) => {
            iterator.borrow_mut().close();
            Ok(Value::Nil)
        }
        _ => Err("iter-close expects an iterator".into()),
    }
}

fn collection_keys(value: &Value) -> Result<Value, String> {
    match value {
        value @ (Value::Map(_) | Value::OrderedMap(_) | Value::SortedMap(_) | Value::Trie(_)) => {
            Ok(Value::Vector(
                map_entries(value)
                    .unwrap()
                    .into_iter()
                    .map(|(key, _)| key)
                    .collect(),
            ))
        }
        Value::Object(values) => Ok(Value::Vector(
            values
                .borrow()
                .iter()
                .map(|(key, _)| Value::String(key.clone()))
                .collect(),
        )),
        _ => Err("keys expects a map or object".into()),
    }
}

fn collection_vals(value: &Value) -> Result<Value, String> {
    match value {
        value @ (Value::Map(_) | Value::OrderedMap(_) | Value::SortedMap(_) | Value::Trie(_)) => {
            Ok(Value::Vector(
                map_entries(value)
                    .unwrap()
                    .into_iter()
                    .map(|(_, value)| value)
                    .collect(),
            ))
        }
        Value::Object(values) => Ok(Value::Vector(
            values
                .borrow()
                .iter()
                .map(|(_, value)| value.clone())
                .collect(),
        )),
        _ => Err("vals expects a map or object".into()),
    }
}

fn collection_first(value: Value) -> Result<Value, String> {
    match value {
        Value::Iterator(iterator) => {
            let mut iterator = iterator.borrow_mut();
            if iterator.has_next() {
                iterator.next()
            } else {
                Ok(Value::Nil)
            }
        }
        value => Ok(iterator_values(value)?
            .into_iter()
            .next()
            .unwrap_or(Value::Nil)),
    }
}

fn collection_rest(value: Value) -> Result<Value, String> {
    if matches!(value, Value::Iterator(_)) {
        return iterator_drop(value, 1);
    }
    let mut values = iterator_values(value)?;
    if !values.is_empty() {
        values.remove(0);
    }
    Ok(Value::List(values.into_iter().collect()))
}

fn collection_last(value: Value) -> Result<Value, String> {
    Ok(iterator_values(value)?
        .into_iter()
        .last()
        .unwrap_or(Value::Nil))
}

fn collection_second(value: Value) -> Result<Value, String> {
    if let Value::Iterator(iterator) = &value {
        let mut state = iterator.borrow_mut();
        let _ = state.next()?;
        return Ok(state.next().unwrap_or(Value::Nil));
    }
    let mut values = iterator_values(value)?.into_iter();
    values.next();
    Ok(values.next().unwrap_or(Value::Nil))
}

fn collection_empty(value: Value) -> Result<Value, String> {
    match value {
        Value::Iterator(iterator) => Ok(Value::Bool(!iterator.borrow().has_next())),
        value => Ok(Value::Bool(iterator_values(value)?.is_empty())),
    }
}

fn collection_empty_value(value: Value) -> Result<Value, String> {
    match value {
        Value::Nil => Ok(Value::Nil),
        Value::List(_) => Ok(Value::List(PList::new())),
        Value::Cons(_) | Value::Queue(_) => Ok(Value::List(PList::new())),
        Value::Vector(_) | Value::Tuple(_) => Ok(Value::Vector(PVector::new())),
        Value::Map(_) | Value::OrderedMap(_) | Value::SortedMap(_) | Value::Trie(_) => {
            Ok(Value::OrderedMap(Box::new(POrderedMap::new())))
        }
        Value::Set(_) | Value::OrderedSet(_) | Value::SortedSet(_) => {
            Ok(Value::OrderedSet(Box::new(POrderedSet::new())))
        }
        value => Err(format!(
            "empty expects a collection, got {}",
            portable_type_name(&value)
        )),
    }
}

fn collection_count(value: &Value) -> Result<Value, String> {
    let count = match value {
        Value::Nil => 0,
        Value::String(v) => v.chars().count(),
        Value::Tuple(v) => v.len(),
        Value::Vector(v) => v.len(),
        Value::List(v) => v.len(),
        Value::Cons(v) => v.iter().count(),
        Value::Queue(v) => v.len(),
        value @ (Value::Map(_) | Value::OrderedMap(_) | Value::SortedMap(_) | Value::Trie(_)) => {
            map_entries(value).unwrap().len()
        }
        value @ (Value::Set(_) | Value::OrderedSet(_) | Value::SortedSet(_)) => {
            set_items(value).unwrap().len()
        }
        Value::Bytes(v) => v.len(),
        Value::ByteBuffer(v) => v.borrow().len(),
        Value::Array(v) => v.borrow().len(),
        Value::Object(v) => v.borrow().len(),
        Value::Iterator(_) => {
            if !iterator_is_finite(value) {
                return Err("count expects a finite collection".into());
            }
            let mut count = 0;
            loop {
                match iterator_next(value) {
                    Ok(_) => count += 1,
                    Err(_) => break,
                }
            }
            count
        }
        _ => return Err("count expects a collection".into()),
    };
    Ok(Value::Number(count as i64))
}

fn iterator_is_finite(value: &Value) -> bool {
    match value {
        Value::Iterator(iterator) => match &iterator.borrow().generator {
            None => true,
            Some(IteratorGenerator::Constant(_))
            | Some(IteratorGenerator::Repeated(_))
            | Some(IteratorGenerator::Iterate(_, _)) => false,
            Some(IteratorGenerator::Take(_, _)) => true,
            Some(IteratorGenerator::Cycle(_, _, _, _)) => false,
            Some(IteratorGenerator::Drop(source, _))
            | Some(IteratorGenerator::TakeWhile(_, source))
            | Some(IteratorGenerator::DropWhile(_, source, _))
            | Some(IteratorGenerator::Map(_, source))
            | Some(IteratorGenerator::Filter(_, source))
            | Some(IteratorGenerator::Mapcat(_, source, _))
            | Some(IteratorGenerator::Keep(_, source))
            | Some(IteratorGenerator::Partition(source, _, _)) => iterator_is_finite(source),
            Some(IteratorGenerator::Zip(sources))
            | Some(IteratorGenerator::Interleave(sources, _)) => {
                sources.iter().all(iterator_is_finite)
            }
        },
        _ => true,
    }
}

fn collection_get(value: &Value, key: &Value, default: Value) -> Result<Value, String> {
    match value {
        Value::Nil => Ok(default),
        Value::Tuple(values) => {
            let index = value_index(key)?;
            Ok(values.get(index).cloned().unwrap_or(default))
        }
        Value::Vector(values) => {
            let index = value_index(key)?;
            Ok(values.get(index).cloned().unwrap_or(default))
        }
        Value::Array(values) => {
            let index = value_index(key)?;
            Ok(values.borrow().get(index).cloned().unwrap_or(default))
        }
        Value::Cons(values) => {
            let index = value_index(key)?;
            Ok(values.iter().nth(index).unwrap_or(default))
        }
        Value::List(values) => {
            let index = value_index(key)?;
            Ok(values.get(index).cloned().unwrap_or(default))
        }
        Value::Queue(values) => {
            let index = value_index(key)?;
            Ok(values.get(index).cloned().unwrap_or(default))
        }
        Value::String(text) => {
            let index = value_index(key)?;
            Ok(text
                .chars()
                .nth(index)
                .map(|c| Value::String(c.to_string()))
                .unwrap_or(default))
        }
        value @ (Value::Map(_) | Value::OrderedMap(_) | Value::SortedMap(_) | Value::Trie(_)) => {
            Ok(map_value(value, key).cloned().unwrap_or(default))
        }
        value @ (Value::Set(_) | Value::OrderedSet(_) | Value::SortedSet(_)) => {
            Ok(set_find(value, key).unwrap_or(default))
        }
        Value::Object(entries) => {
            let name = match key {
                Value::String(name) => name.as_str(),
                Value::Keyword(name) => name.as_str(),
                _ => return Ok(default),
            };
            Ok(entries
                .borrow()
                .iter()
                .find(|(candidate, _)| candidate == name)
                .map(|(_, value)| value.clone())
                .unwrap_or(default))
        }
        _ => Err("get expects a collection".into()),
    }
}

fn collection_nth(value: &Value, key: &Value) -> Result<Value, String> {
    let index = value_index(key)?;
    if let Value::Iterator(iterator) = value {
        let mut state = iterator.borrow_mut();
        for _ in 0..index {
            let _ = state.next()?;
        }
        return state.next().map_err(|_| "nth index out of bounds".into());
    }
    let missing = Value::Nil;
    collection_get(value, key, missing).and_then(|result| {
        if result == Value::Nil {
            Err("nth index out of bounds".into())
        } else {
            Ok(result)
        }
    })
}

fn collection_assoc(value: &Value, key: &Value, replacement: Value) -> Result<Value, String> {
    match value {
        value @ (Value::Map(_) | Value::OrderedMap(_) | Value::SortedMap(_) | Value::Trie(_)) => {
            map_assoc_value(value, key.clone(), replacement)
        }
        Value::Object(entries) => {
            let name = marker_key(key, "object")?;
            let mut output = entries.borrow().clone();
            if let Some((_, item)) = output.iter_mut().find(|(candidate, _)| candidate == &name) {
                *item = replacement;
            } else {
                output.push((name, replacement));
            }
            Ok(Value::Object(Rc::new(RefCell::new(output))))
        }
        Value::Nil => Ok(Value::Map(
            PMap::new().assoc_value(key.clone(), replacement),
        )),
        _ => Err("assoc expects a map or object".into()),
    }
}

fn collection_dissoc(value: &Value, keys: &[Value]) -> Result<Value, String> {
    match value {
        value @ (Value::Map(_) | Value::OrderedMap(_) | Value::SortedMap(_) | Value::Trie(_)) => {
            keys.iter()
                .try_fold(value.clone(), |map, key| map_dissoc_value(&map, key))
        }
        value @ (Value::Set(_) | Value::OrderedSet(_) | Value::SortedSet(_)) => keys
            .iter()
            .try_fold(value.clone(), |set, key| set_dissoc_value(&set, key)),
        Value::Nil => Ok(Value::Map(PMap::new())),
        _ => Err("dissoc expects a map".into()),
    }
}

fn collection_get_in(value: Value, keys: &[Value]) -> Result<Value, String> {
    if keys.is_empty() {
        return Ok(value);
    }
    let next = collection_get(&value, &keys[0], Value::Nil)?;
    if matches!(next, Value::Nil) {
        Ok(Value::Nil)
    } else {
        collection_get_in(next, &keys[1..])
    }
}

fn collection_assoc_in(value: Value, keys: &[Value], replacement: Value) -> Result<Value, String> {
    if keys.is_empty() {
        return Ok(replacement);
    }
    let current = if matches!(value, Value::Nil) {
        Value::Map(PMap::new())
    } else {
        value
    };
    let child = collection_get(&current, &keys[0], Value::Nil)?;
    let updated = collection_assoc_in(child, &keys[1..], replacement)?;
    collection_assoc(&current, &keys[0], updated)
}

fn unique_values(values: Vec<Value>) -> Vec<Value> {
    let mut unique = Vec::new();
    for value in values {
        if !unique.contains(&value) {
            unique.push(value);
        }
    }
    unique
}

fn metadata_value(form: &Form) -> Result<MetadataValue, String> {
    match form {
        Form::Nil => Ok(MetadataValue::Nil),
        Form::Bool(value) => Ok(MetadataValue::Boolean(*value)),
        Form::Number(value) => Ok(MetadataValue::Number(*value)),
        Form::Float(value) => Ok(MetadataValue::Float(*value)),
        Form::BigInteger(value) => Ok(MetadataValue::BigInteger(value.clone())),
        Form::Decimal(value) => Ok(MetadataValue::Decimal(value.clone())),
        Form::Character(value) => Ok(MetadataValue::Character(*value)),
        Form::Regex(value) => Ok(MetadataValue::Regex(value.clone())),
        Form::Tagged(tag, value) => Ok(MetadataValue::Tagged(
            tag.clone(),
            Box::new(metadata_value(value)?),
        )),
        Form::Metadata(_, value) => metadata_value(value),
        Form::Symbol(value) => Ok(MetadataValue::Symbol(Symbol::from(value.clone()))),
        Form::Keyword(value) => Ok(MetadataValue::Keyword(Keyword::from(value.clone()))),
        Form::String(value) => Ok(MetadataValue::String(value.clone())),
        Form::Vector(values) => Ok(MetadataValue::Vector(
            values
                .iter()
                .map(metadata_value)
                .collect::<Result<_, _>>()?,
        )),
        Form::List(values) => Ok(MetadataValue::List(
            values
                .iter()
                .map(metadata_value)
                .collect::<Result<_, _>>()?,
        )),
        Form::Set(values) => Ok(MetadataValue::Set(
            values
                .iter()
                .map(metadata_value)
                .collect::<Result<_, _>>()?,
        )),
        Form::Map(values) => Ok(MetadataValue::Map(
            values
                .iter()
                .map(|(key, value)| Ok((metadata_value(key)?, metadata_value(value)?)))
                .collect::<Result<_, String>>()?,
        )),
    }
}

pub(crate) fn metadata_from_form(form: &Form) -> Result<Rc<Metadata>, String> {
    let MetadataValue::Map(entries) = metadata_value(form)? else {
        return Err("reader metadata must be a map".into());
    };
    Ok(Metadata::new(entries))
}

fn attach_metadata(value: Value, metadata: Rc<Metadata>) -> Result<Value, String> {
    Ok(match value {
        Value::Symbol(value) => Value::Symbol(value.with_meta(Some(metadata.clone()))),
        Value::Pointer(value) => Value::Pointer(value.with_meta(Some(metadata))),
        Value::Tuple(value) => Value::Tuple(Box::new(value.with_meta(Some(metadata)))),
        Value::Vector(value) => Value::Vector(value.with_meta(Some(metadata))),
        Value::List(value) => Value::List(value.with_meta(Some(metadata.clone()))),
        Value::Cons(value) => Value::Cons(Box::new(value.with_meta(Some(metadata)))),
        Value::Queue(value) => Value::Queue(Box::new(value.with_meta(Some(metadata)))),
        Value::Map(value) => Value::Map(value.with_meta(Some(metadata))),
        Value::OrderedMap(value) => Value::OrderedMap(Box::new(value.with_meta(Some(metadata)))),
        Value::SortedMap(value) => Value::SortedMap(Box::new(value.with_meta(Some(metadata)))),
        Value::Trie(value) => Value::Trie(Box::new(value.with_meta(Some(metadata)))),
        Value::Set(value) => Value::Set(value.with_meta(Some(metadata))),
        Value::OrderedSet(value) => Value::OrderedSet(Box::new(value.with_meta(Some(metadata)))),
        Value::SortedSet(value) => Value::SortedSet(Box::new(value.with_meta(Some(metadata)))),
        Value::Var(value) => {
            value.set_hara_metadata(Some(metadata));
            Value::Var(value)
        }
        Value::NativeType(value) => Value::NativeType(Rc::new(NativeType {
            name: value.name.clone(),
            methods: value.methods.clone(),
            metadata: Some(metadata),
        })),
        Value::Keyword(value) => Value::Keyword(value),
        _ => return Err("metadata can only be applied to object values".into()),
    })
}

fn eval_sequential_constructor(
    name: &str,
    forms: &[Form],
    env: &mut HashMap<String, Value>,
) -> Result<Value, String> {
    let maximum = match name {
        "pair" => Some(2),
        "tup" => Some(5),
        _ => None,
    };
    if let Some(maximum) = maximum {
        let valid = if name == "pair" {
            forms.len() == maximum
        } else {
            forms.len() <= maximum
        };
        if !valid {
            return Err(if name == "pair" {
                format!("pair expects two arguments, got {}", forms.len())
            } else {
                format!("tup expects at most 5 arguments, got {}", forms.len())
            });
        }
    }
    let values = forms
        .iter()
        .map(|form| eval(form, env))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(match name {
        "list" => Value::List(values.into()),
        "vector" => Value::Vector(values.into()),
        "pair" | "tup" => Value::Tuple(Box::new(PTuple::from_values(values)?)),
        _ => unreachable!("guarded sequential constructor"),
    })
}

fn eval_collection_constructor(
    name: &str,
    forms: &[Form],
    env: &mut HashMap<String, Value>,
) -> Result<Value, String> {
    let values = forms
        .iter()
        .map(|form| eval(form, env))
        .collect::<Result<Vec<_>, _>>()?;
    match name {
        "hash-map" | "ordered-map" | "sorted-map" | "trie" => {
            if values.len() % 2 != 0 {
                return Err(format!(
                    "{name} expects an even number of key/value arguments"
                ));
            }
            let entries = values
                .chunks_exact(2)
                .map(|pair| (pair[0].clone(), pair[1].clone()));
            Ok(match name {
                "hash-map" => Value::Map(PMap::from_iter(entries)),
                "ordered-map" => Value::OrderedMap(Box::new(POrderedMap::from_iter(entries))),
                "sorted-map" => Value::SortedMap(Box::new(PSortedMap::from_iter(entries))),
                "trie" => {
                    let mut trie = PTrie::new();
                    for (key, value) in entries {
                        let Value::String(key) = key else {
                            return Err("trie expects string keys".into());
                        };
                        trie = trie.assoc_value(key, value);
                    }
                    Value::Trie(Box::new(trie))
                }
                _ => unreachable!("guarded map constructor"),
            })
        }
        "hash-set" => Ok(Value::Set(values.into_iter().collect())),
        "ordered-set" => Ok(Value::OrderedSet(Box::new(values.into_iter().collect()))),
        "sorted-set" => Ok(Value::SortedSet(Box::new(values.into_iter().collect()))),
        "queue" => Ok(Value::Queue(Box::new(values.into_iter().collect()))),
        _ => unreachable!("guarded collection constructor"),
    }
}

fn vector_literal(values: Vec<Value>) -> Result<Value, String> {
    if values.len() <= 5 {
        Ok(Value::Tuple(Box::new(PTuple::from_values(values)?)))
    } else {
        Ok(Value::Vector(values.into()))
    }
}

fn literal_value(form: &Form) -> Result<Value, String> {
    match form {
        Form::Nil => Ok(Value::Nil),
        Form::Bool(value) => Ok(Value::Bool(*value)),
        Form::Character(value) => Ok(Value::Character(*value)),
        Form::Float(value) => Ok(Value::Float(*value)),
        Form::BigInteger(value) => Ok(Value::BigInteger(value.clone())),
        Form::Decimal(value) => Ok(Value::Decimal(value.clone())),
        Form::Regex(value) => Ok(Value::Regex(value.clone())),
        Form::Tagged(tag, value) => Ok(Value::Tagged(Box::new(PTaggedLiteral::new(
            Symbol::parse(tag),
            literal_value(value)?,
        )))),
        Form::Metadata(metadata, value) => {
            attach_metadata(literal_value(value)?, metadata_from_form(metadata)?)
        }
        Form::Number(v) => Ok(Value::Number(*v)),
        Form::String(v) => Ok(Value::String(v.clone())),
        Form::Keyword(v) => Ok(Value::Keyword(v.clone().into())),
        Form::Symbol(v) => Ok(Value::Symbol(v.clone().into())),
        Form::Vector(values) => {
            vector_literal(values.iter().map(literal_value).collect::<Result<_, _>>()?)
        }
        Form::Set(values) => Ok(Value::OrderedSet(Box::new(
            unique_values(values.iter().map(literal_value).collect::<Result<_, _>>()?)
                .into_iter()
                .collect(),
        ))),
        Form::List(values) => Ok(Value::List(
            values.iter().map(literal_value).collect::<Result<_, _>>()?,
        )),
        Form::Map(values) => Ok(Value::OrderedMap(Box::new(
            values
                .iter()
                .map(|(k, v)| Ok((literal_value(k)?, literal_value(v)?)))
                .collect::<Result<_, String>>()?,
        ))),
    }
}

fn generated_function(
    params: Vec<String>,
    body: Vec<Form>,
    mut captured: HashMap<String, Value>,
    bindings: Vec<(&str, Value)>,
) -> Value {
    for (name, value) in bindings {
        captured.insert(name.to_string(), value);
    }
    Value::Function(Rc::new(Function {
        params,
        variadic: None,
        body,
        captured: Rc::new(RefCell::new(captured)),
        name: None,
        native: None,
        clauses: Vec::new(),
        is_macro: false,
    }))
}

fn function_parts(form: &Form) -> Result<(Vec<String>, Option<String>), String> {
    let list = match form {
        Form::Vector(values) => values,
        _ => return Err("function parameters must be a vector".into()),
    };
    let mut params = Vec::new();
    let mut variadic = None;
    let mut index = 0;
    while index < list.len() {
        match &list[index] {
            Form::Symbol(name) if name == "&" => {
                if variadic.is_some() || index + 1 >= list.len() || index + 2 != list.len() {
                    return Err("variadic marker must precede the final parameter".into());
                }
                match &list[index + 1] {
                    Form::Symbol(name) => variadic = Some(name.clone()),
                    _ => return Err("variadic parameter must be a symbol".into()),
                }
                index += 2;
            }
            Form::Symbol(name) => {
                params.push(name.clone());
                index += 1;
            }
            _ => return Err("function parameters must be symbols".into()),
        }
    }
    Ok((params, variadic))
}

fn select_clause(functions: &[Rc<Function>], argument_count: usize) -> Option<Rc<Function>> {
    functions
        .iter()
        .find(|function| function.variadic.is_none() && function.params.len() == argument_count)
        .or_else(|| {
            functions
                .iter()
                .filter(|function| {
                    function.variadic.is_some() && argument_count >= function.params.len()
                })
                .max_by_key(|function| function.params.len())
        })
        .cloned()
}

fn multi_arity_function(
    name: &str,
    clauses: &[Form],
    captured: &HashMap<String, Value>,
    is_macro: bool,
) -> Result<Value, String> {
    let mut functions = Vec::with_capacity(clauses.len());
    for clause in clauses {
        let parts = match clause {
            Form::List(parts) if parts.len() >= 2 => parts,
            _ => return Err("defn arity must contain parameters and a body".into()),
        };
        let (params, variadic) = function_parts(&parts[0])?;
        functions.push(Rc::new(Function {
            params,
            variadic,
            body: parts[1..].to_vec(),
            captured: Rc::new(RefCell::new(captured.clone())),
            name: Some(name.into()),
            native: None,
            clauses: Vec::new(),
            is_macro,
        }));
    }
    if functions.is_empty() {
        return Err("defn expects at least one arity".into());
    }
    let dispatch_name = name.to_owned();
    let clauses = functions.clone();
    Ok(Value::Function(Rc::new(Function {
        params: Vec::new(),
        variadic: Some("arguments".into()),
        body: Vec::new(),
        captured: Rc::new(RefCell::new(HashMap::new())),
        name: Some(dispatch_name.clone()),
        clauses,
        native: Some(Rc::new(move |arguments| {
            let function = select_clause(&functions, arguments.len()).ok_or_else(|| {
                format!(
                    "{dispatch_name} has no arity accepting {} arguments",
                    arguments.len()
                )
            })?;
            call_function(&function, arguments)
        })),
        is_macro,
    })))
}

fn deref_value(value: Value) -> Value {
    match value {
        Value::Var(var) => var.deref_value(),
        value => value,
    }
}

fn binding_value(env: &HashMap<String, Value>, name: &str) -> Option<Value> {
    env.get(name)
        .cloned()
        .map(deref_value)
        .or_else(|| {
            namespace_registry()
                .ok()?
                .resolve(&crate::lang::data::Symbol::parse(name))
                .map(|var| var.deref_value())
        })
        .or_else(|| {
            name.rsplit_once('/')
                .and_then(|(_, local)| env.get(local).cloned().map(deref_value))
        })
}

fn binding_var(env: &mut HashMap<String, Value>, name: &str) -> Option<KernelVar<Value>> {
    match env.get(name) {
        Some(Value::Var(var)) => Some(var.clone()),
        Some(value) => {
            let var = KernelVar::new(name, value.clone());
            env.insert(name.to_string(), Value::Var(var.clone()));
            Some(var)
        }
        None => None,
    }
}

pub(crate) fn call_value(callable: Value, arguments: Vec<Value>) -> Result<Value, String> {
    let lookup =
        |target: &Value, key: &Value, fallback: Value| collection_get(target, key, fallback);
    match callable {
        Value::Function(function) => call_function(&function, arguments),
        Value::StructType(ty) => {
            if arguments.len() != ty.fields.len() {
                return Err(format!("{} expects {} arguments", ty.name, ty.fields.len()));
            }
            Ok(Value::Struct(Rc::new(StructValue {
                ty,
                values: arguments,
            })))
        }
        Value::Keyword(keyword) => match arguments.as_slice() {
            [target] => lookup(target, &Value::Keyword(keyword), Value::Nil),
            [target, fallback] => lookup(target, &Value::Keyword(keyword), fallback.clone()),
            _ => Err("keyword invocation expects one or two arguments".into()),
        },
        value @ (Value::Map(_) | Value::OrderedMap(_) | Value::SortedMap(_) | Value::Trie(_)) => {
            match arguments.as_slice() {
                [key] => Ok(map_value(&value, key).cloned().unwrap_or(Value::Nil)),
                [key, fallback] => Ok(map_value(&value, key)
                    .cloned()
                    .unwrap_or_else(|| fallback.clone())),
                _ => Err("map invocation expects one or two arguments".into()),
            }
        }
        value @ (Value::Set(_) | Value::OrderedSet(_) | Value::SortedSet(_)) => {
            match arguments.as_slice() {
                [key] => Ok(set_find(&value, key).unwrap_or(Value::Nil)),
                [key, fallback] => Ok(set_find(&value, key).unwrap_or_else(|| fallback.clone())),
                _ => Err("set invocation expects one or two arguments".into()),
            }
        }
        _ => Err("value is not callable".into()),
    }
}

pub(crate) fn call_function(function: &Function, arguments: Vec<Value>) -> Result<Value, String> {
    #[cfg(feature = "dev-trace")]
    let operation = development_trace_enter(function, &arguments);
    if let Some(native) = &function.native {
        if function.variadic.is_none() && function.params.len() != arguments.len() {
            #[cfg(feature = "dev-trace")]
            development_trace_exit(operation, function, None);
            return Err(format!(
                "function expects {} arguments",
                function.params.len()
            ));
        }
        let result = native(arguments);
        #[cfg(feature = "dev-trace")]
        development_trace_exit(operation, function, result.as_ref().ok());
        return result;
    }
    let tracing = tracing_enabled();
    if tracing {
        TRACE_STACK.with(|stack| {
            stack.borrow_mut().push(
                function
                    .name
                    .clone()
                    .unwrap_or_else(|| "<anonymous>".into()),
            )
        });
    }
    let result = (|| {
        if function.variadic.is_none() && function.params.len() != arguments.len() {
            return Err(format!(
                "function expects {} arguments",
                function.params.len()
            ));
        }
        if arguments.len() < function.params.len() {
            return Err(format!(
                "function expects at least {} arguments",
                function.params.len()
            ));
        }
        let mut env = function.captured.borrow().clone();
        for (name, value) in function
            .params
            .iter()
            .zip(arguments.iter().take(function.params.len()))
        {
            env.insert(name.clone(), value.clone());
        }
        if let Some(name) = &function.variadic {
            env.insert(
                name.clone(),
                Value::List(arguments.into_iter().skip(function.params.len()).collect()),
            );
        }
        let mut result = Value::Nil;
        for form in &function.body {
            result = eval(form, &mut env)?;
            if matches!(result, Value::Recur(_)) {
                return Err("recur must be inside loop".into());
            }
        }
        Ok(result)
    })();
    let result = result.map_err(append_trace);
    #[cfg(feature = "dev-trace")]
    development_trace_exit(operation, function, result.as_ref().ok());
    if tracing {
        TRACE_STACK.with(|stack| {
            stack.borrow_mut().pop();
        });
    }
    result
}

/// Runs one evaluator operation with a bounded development trace. This is
/// intentionally separate from the legacy stack-trace flag above.
#[cfg(feature = "dev-trace")]
pub(crate) fn with_development_trace<T>(
    trace_id: crate::trace::TraceId,
    limits: crate::trace::TraceLimits,
    evaluate: impl FnOnce() -> Result<T, String>,
    preview: impl FnOnce(&T, &crate::trace::TraceCollector) -> crate::trace::ValuePreview,
) -> (Result<T, String>, crate::trace::Trace) {
    DEVELOPMENT_TRACE_STACK.with(|stack| stack.borrow_mut().clear());
    let previous = DEVELOPMENT_TRACE
        .with(|active| active.replace(Some(crate::trace::TraceCollector::new(trace_id, limits))));
    assert!(
        previous.is_none(),
        "nested development traces are not supported yet"
    );
    DEVELOPMENT_TRACE.with(|active| {
        active
            .borrow_mut()
            .as_mut()
            .expect("development trace must be active")
            .record(crate::trace::TraceEvent::new(
                crate::trace::TraceEventKind::EvaluationStart,
            ));
    });
    let result = evaluate();
    let collector = DEVELOPMENT_TRACE.with(|active| {
        active
            .replace(previous)
            .expect("development trace must be active")
    });
    DEVELOPMENT_TRACE_STACK.with(|stack| stack.borrow_mut().clear());
    let trace = match &result {
        Ok(value) => {
            let result = preview(value, &collector);
            collector.finish(result)
        }
        Err(error) => collector.fail(error.clone()),
    };
    (result, trace)
}

fn binding_symbol(form: &Form, context: &str) -> Result<(String, Option<Rc<Metadata>>), String> {
    match form {
        Form::Symbol(name) => Ok((name.clone(), None)),
        Form::Metadata(metadata, value) => match value.as_ref() {
            Form::Symbol(name) => Ok((name.clone(), Some(metadata_from_form(metadata)?))),
            _ => Err(format!("{context} must be a symbol")),
        },
        _ => Err(format!("{context} must be a symbol")),
    }
}

fn syntax_quote_collection(
    values: &[Form],
    vector: bool,
    env: &mut HashMap<String, Value>,
) -> Result<Value, String> {
    let mut output = Vec::new();
    for value in values {
        match value {
            Form::List(parts)
                if !parts.is_empty()
                    && matches!(&parts[0], Form::Symbol(name) if name == "unquote") =>
            {
                if parts.len() != 2 {
                    return Err("unquote expects one argument".into());
                }
                output.push(eval(&parts[1], env)?);
            }
            Form::List(parts)
                if !parts.is_empty()
                    && matches!(&parts[0], Form::Symbol(name) if name == "unquote-splicing") =>
            {
                if parts.len() != 2 {
                    return Err("unquote-splicing expects one argument".into());
                }
                output.extend(iterator_values(eval(&parts[1], env)?)?);
            }
            value => output.push(syntax_quote_value(value, env)?),
        }
    }
    if vector {
        vector_literal(output)
    } else {
        Ok(Value::List(output.into()))
    }
}

fn syntax_quote_value(form: &Form, env: &mut HashMap<String, Value>) -> Result<Value, String> {
    match form {
        Form::Symbol(_) => literal_value(form),
        Form::List(values)
            if values.len() == 2
                && matches!(&values[0], Form::Symbol(name) if name == "unquote") =>
        {
            eval(&values[1], env)
        }
        Form::List(values) => syntax_quote_collection(values, false, env),
        Form::Vector(values) => syntax_quote_collection(values, true, env),
        Form::Map(values) => Ok(Value::OrderedMap(Box::new(
            values
                .iter()
                .map(|(key, value)| {
                    Ok((
                        syntax_quote_value(key, env)?,
                        syntax_quote_value(value, env)?,
                    ))
                })
                .collect::<Result<Vec<_>, String>>()?
                .into_iter()
                .collect(),
        ))),
        _ => literal_value(form),
    }
}

fn ensure_namespace(
    registry: &NamespaceRegistry<Value>,
    env: &mut HashMap<String, Value>,
    name: &str,
) -> Result<(), String> {
    if registry.find(name).is_some() {
        return Ok(());
    }
    let source = NAMESPACE_SOURCE_PROVIDER
        .with(|active| active.borrow().as_ref().and_then(|provider| provider(name)))
        .ok_or_else(|| format!("Cannot require missing namespace: {name}"))?;
    let requiring = registry.current().name().as_str().to_owned();
    for form in crate::kernel::parse_forms(&source)? {
        eval(&form, env)?;
    }
    select_namespace_environment(registry, env, &requiring);
    Ok(())
}

fn eval_require_spec(
    registry: &NamespaceRegistry<Value>,
    env: &mut HashMap<String, Value>,
    form: &Form,
) -> Result<(), String> {
    let (target, options) = match form {
        Form::Vector(items) => {
            let target = match items.first() {
                Some(Form::Symbol(target)) => target.clone(),
                _ => return Err("require namespace must be a symbol".into()),
            };
            (target, &items[1..])
        }
        Form::List(items)
            if items.len() == 2
                && matches!(&items[0], Form::Symbol(q) if q == "quote")
                && matches!(&items[1], Form::Symbol(_)) =>
        {
            let target = match &items[1] {
                Form::Symbol(target) => target.clone(),
                _ => unreachable!(),
            };
            (target, &[][..])
        }
        _ => return Err("require expects vectors such as [chrome.api :as api]".into()),
    };
    if options.len() % 2 != 0 {
        return Err(format!("Malformed require options for {target}"));
    }
    let lazy = options.chunks(2).any(|option| {
        matches!(&option[0], Form::Keyword(keyword) if keyword.as_str() == "lazy")
            && matches!(&option[1], Form::Bool(true))
    });
    if lazy {
        let has_alias = options.chunks(2).any(|option| {
            matches!(&option[0], Form::Keyword(keyword) if keyword.as_str() == "as")
        });
        if !has_alias {
            return Err("require :lazy requires :as".into());
        }
        for option in options.chunks(2) {
            match &option[0] {
                Form::Keyword(keyword)
                    if keyword.as_str() == "refer" || keyword.as_str() == "refer-macros" =>
                {
                    return Err(format!("require :lazy cannot be combined with :{}", keyword));
                }
                Form::Keyword(keyword)
                    if keyword.as_str() == "lazy" && !matches!(&option[1], Form::Bool(true)) =>
                {
                    return Err("require :lazy expects true".into());
                }
                _ => {}
            }
        }
    } else {
        ensure_namespace(registry, env, &target)?;
    }
    for option in options.chunks(2) {
        let name = match &option[0] {
            Form::Keyword(keyword) => keyword.as_str(),
            _ => return Err("Malformed require options".into()),
        };
        match name {
            "as" => {
                let alias = match &option[1] {
                    Form::Symbol(alias) if !alias.contains('/') => alias.clone(),
                    _ => return Err("require :as expects an unqualified symbol".into()),
                };
                if lazy {
                    registry.current().lazy_alias(alias, &target);
                } else {
                    let namespace = registry
                        .find(&target)
                        .ok_or_else(|| format!("Cannot require missing namespace: {target}"))?;
                    registry.current().alias(alias, namespace);
                }
            }
            "lazy" => {}
            other => return Err(format!("Unsupported require option: :{other}")),
        }
    }
    Ok(())
}

fn eval_require_specs(
    registry: &NamespaceRegistry<Value>,
    env: &mut HashMap<String, Value>,
    specs: &[Form],
) -> Result<(), String> {
    for spec in specs {
        eval_require_spec(registry, env, spec)?;
    }
    refresh_namespace_environment(registry, env);
    Ok(())
}

fn force_lazy_alias(
    registry: &NamespaceRegistry<Value>,
    env: &mut HashMap<String, Value>,
    symbol: &str,
) -> Result<(), String> {
    let Some((alias, _)) = symbol.split_once('/') else {
        return Ok(());
    };
    let Some(target) = registry.current().lazy_target(alias) else {
        return Ok(());
    };
    ensure_namespace(registry, env, target.as_str())?;
    let namespace = registry
        .find(target.as_str())
        .ok_or_else(|| format!("Cannot require missing namespace: {target}"))?;
    registry.current().alias(alias, namespace);
    refresh_namespace_environment(registry, env);
    Ok(())
}

/// Handles the `ns` and `require` special forms.
///
/// Kept out of line so the giant `eval` dispatch does not reserve stack for
/// these locals on every recursive call (the native runtime recurses through
/// `eval` and test threads run on small stacks).
#[inline(never)]
fn eval_namespace_form(fs: &[Form], env: &mut HashMap<String, Value>) -> Result<Value, String> {
    let head = match &fs[0] {
        Form::Symbol(head) => head.as_str(),
        _ => unreachable!("ns/require dispatch guarantees a symbol head"),
    };
    if head == "require" {
        let registry = namespace_registry()?;
        eval_require_specs(&registry, env, &fs[1..])?;
        return Ok(Value::Nil);
    }
    if fs.len() < 2 {
        return Err("ns expects a namespace symbol".into());
    }
    let name = match &fs[1] {
        Form::Symbol(name) if !name.contains('/') => name.clone(),
        _ => return Err("ns expects a namespace symbol".into()),
    };
    let registry = namespace_registry()?;
    refer_startup_defaults(&registry, &name);
    select_namespace_environment(&registry, env, &name);
    for clause in &fs[2..] {
        match clause {
            Form::List(clause_forms) if matches!(clause_forms.first(), Some(Form::Keyword(k)) if k == "require") =>
            {
                eval_require_specs(&registry, env, &clause_forms[1..])?;
            }
            Form::List(clause_forms)
                if matches!(
                    clause_forms.first(),
                    Some(Form::Keyword(k)) if k == "config" || k == "intrinsics"
                ) =>
            {
                // :config and :intrinsics are processed by the generated-namespace
                // machinery for top-level ns forms. For ns forms loaded from source
                // files (e.g. runtime-library activation declarations), they are
                // metadata-only and can be ignored here.
            }
            _ => return Err("unsupported ns clause (only :require is supported)".into()),
        }
    }
    Ok(Value::Nil)
}

#[inline(never)]
fn eval_namespace_operation(
    operation: &str,
    forms: &[Form],
    env: &mut HashMap<String, Value>,
) -> Result<Value, String> {
    let registry = namespace_registry()?;
    match operation {
        "ns:name" => match forms {
            [_, value] => match eval(value, env)? {
                Value::Namespace(namespace) => Ok(Value::Symbol(namespace.name().clone())),
                _ => Err("ns:name expects a namespace".into()),
            },
            _ => Err("ns:name expects one namespace".into()),
        },
        "ns:map" | "ns:aliases" | "ns:imports" => {
            if forms.len() != 2 {
                return Err(format!("{operation} expects one namespace"));
            }
            let namespace = match eval(&forms[1], env)? {
                Value::Namespace(namespace) => namespace,
                _ => return Err(format!("{operation} expects a namespace")),
            };
            let entries: Vec<(Value, Value)> = match operation {
                "ns:map" => namespace
                    .mappings()
                    .into_iter()
                    .map(|(name, value)| (Value::Symbol(name), Value::Var(value)))
                    .collect(),
                "ns:aliases" => namespace
                    .aliases()
                    .into_iter()
                    .map(|(name, value)| (Value::Symbol(name), Value::Namespace(Rc::new(value))))
                    .collect(),
                "ns:imports" => namespace
                    .imports()
                    .into_iter()
                    .map(|(name, value)| (Value::Symbol(name), Value::String(value)))
                    .collect(),
                _ => unreachable!(),
            };
            Ok(Value::Map(PMap::from_iter(entries)))
        }
        "ns:find" | "ns:create" => {
            if forms.len() != 2 {
                return Err(format!("{operation} expects one symbol"));
            }
            let name = match eval(&forms[1], env)? {
                Value::Symbol(name) if name.get_namespace().is_none() => name,
                _ => return Err(format!("{operation} expects an unqualified symbol")),
            };
            let namespace = if operation == "ns:create" {
                Some(registry.find_or_create(name.as_str()))
            } else {
                registry.find(name.as_str())
            };
            Ok(namespace
                .map(|value| Value::Namespace(Rc::new(value)))
                .unwrap_or(Value::Nil))
        }
        "ns:list" => {
            if forms.len() != 1 {
                return Err("ns:list expects no arguments".into());
            }
            Ok(iterator_from_values(
                registry
                    .all()
                    .into_iter()
                    .map(|value| Value::Namespace(Rc::new(value)))
                    .collect(),
            ))
        }
        _ => Err(format!("unsupported namespace operation: {operation}")),
    }
}

fn eval_basic_object_form(
    operation: &str,
    forms: &[Form],
    env: &mut HashMap<String, Value>,
) -> Result<Value, String> {
    match operation {
        "type" => {
            if forms.len() != 2 {
                return Err("type expects one value".into());
            }
            let value = eval(&forms[1], env)?;
            Ok(Value::Keyword(Keyword::create(
                Some("hara.type"),
                portable_type_name(&value),
            )?))
        }
        "compare" => {
            if forms.len() != 3 {
                return Err("compare expects two values".into());
            }
            let left = eval(&forms[1], env)?;
            let right = eval(&forms[2], env)?;
            Ok(Value::Number(match left.cmp(&right) {
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Greater => 1,
            }))
        }
        "hash" => {
            if forms.len() != 2 {
                return Err("hash expects one value".into());
            }
            Ok(Value::Number(eval(&forms[1], env)?.stable_hash() as i64))
        }
        "meta" => {
            if forms.len() != 2 {
                return Err("meta expects one value".into());
            }
            protocol_meta(&[eval(&forms[1], env)?])
        }
        "with-meta" => {
            if forms.len() != 3 {
                return Err("with-meta expects a value and metadata map".into());
            }
            let value = eval(&forms[1], env)?;
            let metadata = eval(&forms[2], env)?;
            protocol_with_meta(&[value, metadata])
        }
        "macroexpand-1" => {
            if forms.len() != 2 {
                return Err("macroexpand-1 expects one form".into());
            }
            let value = eval(&forms[1], env)?;
            let form = value_to_form(&value)?;
            let expanded = macroexpand_once(&form, env)?;
            form_to_value(&expanded)
        }
        "gensym" => {
            let prefix = if forms.len() == 1 {
                "G__".into()
            } else if forms.len() == 2 {
                match eval(&forms[1], env)? {
                    Value::String(prefix) => prefix,
                    value => {
                        return Err(format!(
                            "gensym expects a string prefix, got {}",
                            portable_type_name(&value)
                        ))
                    }
                }
            } else {
                return Err("gensym expects zero or one arguments".into());
            };
            Ok(Value::Symbol(Symbol::from(gensym(&prefix))))
        }
        _ => unreachable!("eval_basic_object_form called for an unknown operation"),
    }
}

fn eval_atom_form(
    operation: &str,
    forms: &[Form],
    env: &mut HashMap<String, Value>,
) -> Result<Value, String> {
    match operation {
        "atom" | "atom:basic" => {
            if forms.len() != 2 {
                return Err(format!("{operation} expects one value"));
            }
            Ok(Value::Atom(Box::new(RuntimeAtom::new(
                eval(&forms[1], env)?,
                operation == "atom",
            ))))
        }
        _ => unreachable!("eval_atom_form called for an unknown operation"),
    }
}

pub fn eval(form: &Form, env: &mut HashMap<String, Value>) -> Result<Value, String> {
    match form {
        Form::Number(v) => Ok(Value::Number(*v)),
        Form::String(v) => Ok(Value::String(v.clone())),
        Form::Keyword(v) => Ok(Value::Keyword(v.clone().into())),
        Form::Nil => Ok(Value::Nil),
        Form::Bool(value) => Ok(Value::Bool(*value)),
        Form::Character(value) => Ok(Value::Character(*value)),
        Form::Float(value) => Ok(Value::Float(*value)),
        Form::BigInteger(value) => Ok(Value::BigInteger(value.clone())),
        Form::Decimal(value) => Ok(Value::Decimal(value.clone())),
        Form::Regex(value) => Ok(Value::Regex(value.clone())),
        Form::Tagged(tag, value) => Ok(Value::Tagged(Box::new(PTaggedLiteral::new(
            Symbol::parse(tag),
            literal_value(value)?,
        )))),
        Form::Metadata(_, value) => eval(value, env),
        Form::List(fs)
            if fs.len() == 2 && matches!(&fs[0], Form::Symbol(name) if name == "syntax-quote") =>
        {
            syntax_quote_value(&fs[1], env)
        }
        Form::List(fs)
            if fs.len() == 2 && matches!(&fs[0], Form::Symbol(name) if name == "quote") =>
        {
            literal_value(&fs[1])
        }
        Form::Map(values) => Ok(Value::OrderedMap(Box::new(
            values
                .iter()
                .map(|(key, value)| Ok((eval(key, env)?, eval(value, env)?)))
                .collect::<Result<_, String>>()?,
        ))),
        Form::Set(values) => Ok(Value::OrderedSet(Box::new(
            unique_values(
                values
                    .iter()
                    .map(|value| eval(value, env))
                    .collect::<Result<_, _>>()?,
            )
            .into_iter()
            .collect(),
        ))),
        Form::Vector(values) => vector_literal(
            values
                .iter()
                .map(|value| eval(value, env))
                .collect::<Result<_, _>>()?,
        ),
        Form::Symbol(n) if n == "nil" => Ok(Value::Nil),
        Form::Symbol(n) if n == "true" => Ok(Value::Bool(true)),
        Form::Symbol(n) if n == "false" => Ok(Value::Bool(false)),
        Form::Symbol(n) if n == "inc" || n == "dec" => {
            let op = if n == "inc" { "+" } else { "-" };
            let body = Form::List(vec![
                Form::Symbol(op.into()),
                Form::Symbol("value".into()),
                Form::Number(1),
            ]);
            Ok(generated_function(
                vec!["value".into()],
                vec![body],
                env.clone(),
                vec![],
            ))
        }
        Form::Symbol(n) => {
            if !env.contains_key(n) {
                if let Ok(registry) = namespace_registry() {
                    force_lazy_alias(&registry, env, n)?;
                }
            }
            binding_value(env, n).ok_or_else(|| format!("unbound symbol: {n}"))
        }
        Form::List(fs) if fs.is_empty() => Ok(Value::Nil),
        Form::List(fs) => match &fs[0] {
            Form::Symbol(n) if n == "fn" || n == "fn*" => {
                if fs.len() < 3 {
                    return Err("fn expects parameters and a body".into());
                }
                let (params, variadic) = function_parts(&fs[1])?;
                Ok(Value::Function(Rc::new(Function {
                    params,
                    variadic,
                    body: fs[2..].to_vec(),
                    captured: Rc::new(RefCell::new(env.clone())),
                    name: None,
                    native: None,
                    clauses: Vec::new(),
                    is_macro: false,
                })))
            }
            Form::Symbol(n) if n == "eval" => {
                if fs.len() != 2 {
                    return Err("eval expects one form".into());
                }
                eval(&fs[1], env)
            }
            Form::Symbol(n) if n == "load-string" => {
                if fs.len() != 2 {
                    return Err("load-string expects one string".into());
                }
                match eval(&fs[1], env)? {
                    Value::String(source) => eval_value_text(&source, env),
                    _ => Err("load-string expects a string".into()),
                }
            }
            Form::Symbol(n) if n == "var-sym" => {
                if fs.len() != 2 {
                    return Err("var-sym expects one var".into());
                }
                match eval(&fs[1], env)? {
                    Value::Var(var) => Ok(Value::Symbol(var.symbol().clone())),
                    _ => Err("var-sym expects a var".into()),
                }
            }
            Form::Symbol(n) if n == "ns-state" || n == "ns-loaded?" => {
                if fs.len() != 2 {
                    return Err(format!("{n} expects one namespace"));
                }
                let name = match eval(&fs[1], env)? {
                    Value::Symbol(value) => value.as_str().to_owned(),
                    Value::String(value) => value,
                    _ => return Err(format!("{n} expects a namespace symbol or string")),
                };
                let registry = namespace_registry()?;
                let loaded = registry.find(&name).is_some();
                if n == "ns-loaded?" {
                    Ok(Value::Bool(loaded))
                } else {
                    let deferred = registry
                        .all()
                        .into_iter()
                        .flat_map(|namespace| namespace.lazy_aliases())
                        .any(|(_, target)| target.as_str() == name);
                    Ok(Value::Keyword(
                        if loaded { "loaded" } else if deferred { "unloaded" } else { "unknown" }.into(),
                    ))
                }
            }
            Form::Symbol(n) if n == "ns-alias-state" => {
                if fs.len() != 2 && fs.len() != 3 {
                    return Err("ns-alias-state expects alias or namespace and alias".into());
                }
                let registry = namespace_registry()?;
                let (owner, alias_form) = if fs.len() == 3 {
                    let owner = match eval(&fs[1], env)? {
                        Value::Symbol(value) => value.as_str().to_owned(),
                        Value::String(value) => value,
                        _ => return Err("ns-alias-state expects a namespace symbol or string".into()),
                    };
                    (owner, &fs[2])
                } else {
                    (registry.current().name().as_str().to_owned(), &fs[1])
                };
                let alias = match eval(alias_form, env)? {
                    Value::Symbol(value) if value.get_namespace().is_none() => value,
                    _ => return Err("ns-alias-state expects an unqualified alias symbol".into()),
                };
                let Some(namespace) = registry.find(&owner) else {
                    return Ok(Value::Nil);
                };
                let target = namespace.lazy_target(alias.as_str()).or_else(|| {
                    namespace
                        .aliases()
                        .into_iter()
                        .find(|(name, _)| name == &alias)
                        .map(|(_, target)| target.name().clone())
                });
                let Some(target) = target else { return Ok(Value::Nil); };
                let state = if registry.find(target.as_str()).is_some() { "loaded" } else { "unloaded" };
                Ok(Value::Map(PMap::from_iter([
                    (Value::Keyword("alias".into()), Value::Symbol(alias)),
                    (Value::Keyword("target".into()), Value::Symbol(target)),
                    (Value::Keyword("state".into()), Value::Keyword(state.into())),
                ])))
            }
            Form::Symbol(n) if n == "eval-in-ns" => {
                if fs.len() != 3 {
                    return Err("eval-in-ns expects namespace and forms".into());
                }
                let target = match eval(&fs[1], env)? {
                    Value::Symbol(name) => name.as_str().to_owned(),
                    Value::String(name) => name,
                    _ => return Err("eval-in-ns expects a namespace symbol or string".into()),
                };
                let forms = iterator_values(eval(&fs[2], env)?)?
                    .into_iter()
                    .map(|value| value_to_form(&value))
                    .collect::<Result<Vec<_>, _>>()?;
                let registry = namespace_registry()?;
                if registry.find(&target).is_none() {
                    return Err(format!("eval-in-ns requires an existing namespace: {target}"));
                }
                let previous = registry.current().name().as_str().to_owned();
                select_namespace_environment(&registry, env, &target);
                let result = (|| {
                    let mut result = Value::Nil;
                    for form in &forms {
                        result = eval(form, env)?;
                    }
                    Ok(result)
                })();
                select_namespace_environment(&registry, env, &previous);
                result
            }
            Form::Symbol(n) if n == "intern-var" => {
                if fs.len() != 4 && fs.len() != 5 {
                    return Err("intern-var expects namespace, symbol, var, and optional metadata".into());
                }
                let target = match eval(&fs[1], env)? {
                    Value::Symbol(name) => name.as_str().to_owned(),
                    Value::String(name) => name,
                    _ => return Err("intern-var expects a namespace symbol or string".into()),
                };
                let name = match eval(&fs[2], env)? {
                    Value::Symbol(name) if name.get_namespace().is_none() => name,
                    _ => return Err("intern-var expects an unqualified target symbol".into()),
                };
                let source = match eval(&fs[3], env)? {
                    Value::Var(var) => var,
                    _ => return Err("intern-var expects a source Var".into()),
                };
                let mut metadata = source.metadata();
                if fs.len() == 5 {
                    match eval(&fs[4], env)? {
                        Value::OrderedMap(entries) => {
                            for (key, value) in entries.iter() {
                                metadata.extra.insert(key.display(), value.display());
                            }
                        }
                        _ => return Err("intern-var metadata extension must be a map".into()),
                    }
                }
                let registry = namespace_registry()?;
                let output = registry.find_or_create(&target).intern_with_metadata(
                    name.as_str(),
                    source.deref_value(),
                    metadata,
                );
                Ok(Value::Var(output))
            }
            Form::Symbol(n) if n == "var" => {
                if fs.len() != 2 {
                    return Err("var expects a symbol".into());
                }
                let name = match &fs[1] {
                    Form::Symbol(name) => name,
                    _ => return Err("var expects a symbol".into()),
                };
                if !env.contains_key(name) {
                    if let Ok(registry) = namespace_registry() {
                        force_lazy_alias(&registry, env, name)?;
                    }
                }
                let cell =
                    binding_var(env, name).ok_or_else(|| format!("unbound symbol: {name}"))?;
                Ok(Value::Var(cell))
            }
            Form::Symbol(n)
                if [
                    "type",
                    "compare",
                    "hash",
                    "meta",
                    "with-meta",
                    "macroexpand-1",
                    "gensym",
                ]
                .contains(&n.as_str()) =>
            {
                eval_basic_object_form(n, fs, env)
            }
            Form::Symbol(n) if ["atom", "atom:basic"].contains(&n.as_str()) => {
                eval_atom_form(n, fs, env)
            }
            Form::Symbol(n) if n == "deref" => {
                if fs.len() != 2 {
                    return Err("deref expects a var".into());
                }
                let target = match &fs[1] {
                    Form::Symbol(name) => match env.get(name) {
                        Some(Value::Var(cell)) => Value::Var(cell.clone()),
                        _ => eval(&fs[1], env)?,
                    },
                    _ => eval(&fs[1], env)?,
                };
                match target {
                    Value::Var(value) => Ok(value.deref_value()),
                    Value::Atom(value) => Ok(value.deref_value()),
                    Value::Promise(promise) => match promise.wait_state() {
                        PromiseState::Fulfilled(value) => Ok(value),
                        PromiseState::Rejected(error) => Err(error),
                        PromiseState::Pending => Err(
                            "deref cannot block on a pending promise outside an HTA fiber".into(),
                        ),
                    },
                    _ => Err("deref expects a var, atom, or promise".into()),
                }
            }
            Form::Symbol(n) if n == "set!" || n == "var/set" => {
                if fs.len() != 3 {
                    return Err(format!("{n} expects a symbol and value"));
                }
                let name = match &fs[1] {
                    Form::Symbol(name) => name,
                    _ => return Err(format!("{n} expects a symbol")),
                };
                let value = eval(&fs[2], env)?;
                let cell = binding_var(env, name).ok_or_else(|| format!("unbound var: {name}"))?;
                cell.reset_value(value.clone());
                Ok(value)
            }
            Form::Symbol(n) if n == "alter-var-root" => {
                if fs.len() < 3 {
                    return Err("alter-var-root expects a var and function".into());
                }
                let target = match eval(&fs[1], env)? {
                    Value::Var(cell) => cell,
                    _ => return Err("alter-var-root expects a var".into()),
                };
                let function = match eval(&fs[2], env)? {
                    Value::Function(function) => function,
                    _ => return Err("alter-var-root expects a function".into()),
                };
                let mut arguments = vec![target.deref_value()];
                arguments.extend(
                    fs[3..]
                        .iter()
                        .map(|form| eval(form, env))
                        .collect::<Result<Vec<_>, _>>()?,
                );
                let value = call_function(&function, arguments)?;
                target.reset_value(value.clone());
                Ok(value)
            }
            Form::Symbol(n) if n == "throw" => {
                if fs.len() != 2 {
                    return Err("throw expects one value".into());
                }
                let value = eval(&fs[1], env)?;
                Err(thrown_error(value))
            }
            Form::Symbol(n) if n == "try" => {
                if fs.len() < 2 {
                    return Err("try expects a body".into());
                }
                let mut body = Vec::new();
                let mut catch_form = None;
                let mut finally_forms = Vec::new();
                for form in &fs[1..] {
                    match form {
                        Form::List(parts)
                            if !parts.is_empty()
                                && matches!(&parts[0],Form::Symbol(name) if name=="catch") =>
                        {
                            catch_form = Some(parts)
                        }
                        Form::List(parts)
                            if !parts.is_empty()
                                && matches!(&parts[0],Form::Symbol(name) if name=="finally") =>
                        {
                            finally_forms.extend_from_slice(&parts[1..])
                        }
                        _ if catch_form.is_none() => body.push(form),
                        _ => return Err("try clauses must follow the body".into()),
                    }
                }
                let mut result = Ok(Value::Nil);
                for form in body {
                    result = eval(form, env);
                    if result.is_err() {
                        break;
                    }
                }
                if let Err(ref error) = result {
                    if let Some(parts) = catch_form {
                        let (binding_index, body_index) = match parts.len() {
                            3 => (1, 2),
                            4 => {
                                if !matches!(&parts[1], Form::Symbol(_)) {
                                    return Err("catch class must be a symbol".into());
                                }
                                (2, 3)
                            }
                            _ => return Err("catch expects a class, name, and body".into()),
                        };
                        let name = match &parts[binding_index] {
                            Form::Symbol(name) => name.clone(),
                            _ => return Err("catch name must be a symbol".into()),
                        };
                        let old = env.insert(name.clone(), caught_error(error));
                        result = eval(&parts[body_index], env);
                        if let Some(old) = old {
                            env.insert(name, old);
                        } else {
                            env.remove(&name);
                        }
                    }
                }
                for form in finally_forms {
                    let final_result = eval(&form, env);
                    if final_result.is_err() {
                        result = final_result;
                    }
                }
                result
            }
            Form::Symbol(n) if n == "def" => {
                if fs.len() != 3 {
                    return Err("def expects a name and value".into());
                }
                let (name, metadata) = binding_symbol(&fs[1], "def name")?;
                if let Some(value) = protected_fallback_binding(env, &name) {
                    return Ok(value);
                }
                let value = eval(&fs[2], env)?;
                if let Some(Value::Var(var)) = env.get(&name) {
                    if !binding_is_local(var) {
                        let var = KernelVar::new(name.clone(), value.clone());
                        var.set_origin(definition_origin());
                        var.set_hara_metadata(metadata);
                        env.insert(name, Value::Var(var));
                        return Ok(value);
                    }
                    var.reset_value(value.clone());
                    var.set_origin(definition_origin());
                    if metadata.is_some() {
                        var.set_hara_metadata(metadata);
                    }
                } else {
                    let var = KernelVar::new(name.clone(), value.clone());
                    var.set_origin(definition_origin());
                    var.set_hara_metadata(metadata);
                    env.insert(name, Value::Var(var));
                }
                Ok(value)
            }
            Form::Symbol(n) if n == "declare" => {
                if fs.len() < 2 {
                    return Err("declare expects at least one symbol".into());
                }
                for form in &fs[1..] {
                    let name = match form {
                        Form::Symbol(name) => name.clone(),
                        _ => return Err("declare expects symbols".into()),
                    };
                    let cell = match env.get(&name) {
                        Some(Value::Var(cell)) if binding_is_local(cell) => cell.clone(),
                        _ => KernelVar::new(name.clone(), Value::Nil),
                    };
                    cell.set_origin(definition_origin());
                    env.insert(name, Value::Var(cell));
                }
                Ok(Value::Nil)
            }
            Form::Symbol(n) if n == "defstruct" => {
                if fs.len() != 3 {
                    return Err("defstruct expects a name and field vector".into());
                }
                let name = match &fs[1] {
                    Form::Symbol(name) if !name.contains('/') => name.clone(),
                    _ => return Err("defstruct name must be an unqualified symbol".into()),
                };
                let fields = match &fs[2] {
                    Form::Vector(fields) => fields
                        .iter()
                        .map(|field| match field {
                            Form::Symbol(field) if !field.contains('/') => Ok(field.clone()),
                            _ => Err("defstruct field names must be unqualified symbols".into()),
                        })
                        .collect::<Result<Vec<_>, String>>()?,
                    _ => return Err("defstruct expects a field vector".into()),
                };
                if fields.iter().collect::<HashSet<_>>().len() != fields.len() {
                    return Err("Duplicate defstruct field".into());
                }
                let namespace = namespace_registry()?.current().name().as_str().to_owned();
                let ty = Rc::new(StructType {
                    name: format!("{namespace}/{name}"),
                    fields,
                });
                let map_type = ty.clone();
                let map_constructor = native_function(&format!("map->{name}"), 1, move |values| {
                    let source = values.first().expect("native arity is checked");
                    let fields = map_type
                        .fields
                        .iter()
                        .map(|field| {
                            Ok(
                                map_value(source, &Value::Keyword(Keyword::from(field.as_str())))
                                    .cloned()
                                    .unwrap_or(Value::Nil),
                            )
                        })
                        .collect::<Result<Vec<_>, String>>()?;
                    Ok(Value::Struct(Rc::new(StructValue {
                        ty: map_type.clone(),
                        values: fields,
                    })))
                });
                for (binding, value) in [
                    (name.clone(), Value::StructType(ty.clone())),
                    (format!("->{name}"), Value::StructType(ty.clone())),
                    (format!("map->{name}"), map_constructor),
                ] {
                    let var = KernelVar::new(format!("{namespace}/{binding}"), value);
                    var.set_origin(definition_origin());
                    env.insert(binding, Value::Var(var));
                }
                Ok(Value::Nil)
            }
            Form::Symbol(n) if n == "field" => {
                if fs.len() != 3 {
                    return Err("field expects a struct and field name".into());
                }
                let field = match &fs[2] {
                    Form::Keyword(field) | Form::Symbol(field) if !field.contains('/') => field,
                    _ => return Err("field name must be an unqualified keyword or symbol".into()),
                };
                let value = eval(&fs[1], env)?;
                let Value::Struct(value) = value else {
                    return Err("field expects a struct".into());
                };
                value
                    .ty
                    .fields
                    .iter()
                    .position(|candidate| candidate == field)
                    .map(|index| value.values[index].clone())
                    .ok_or_else(|| format!("unknown struct field: {field}"))
            }
            Form::Symbol(n) if n == "instance?" => {
                if fs.len() != 3 {
                    return Err("instance? expects a struct type and value".into());
                }
                let ty = match eval(&fs[1], env)? {
                    Value::StructType(ty) => ty,
                    _ => return Err("instance? expects a struct type".into()),
                };
                Ok(Value::Bool(
                    matches!(eval(&fs[2], env)?, Value::Struct(value) if Rc::ptr_eq(&ty, &value.ty)),
                ))
            }
            Form::Symbol(n) if n == "defprotocol" => {
                if fs.len() < 3 {
                    return Err("defprotocol expects a name and method declarations".into());
                }
                let name = match &fs[1] {
                    Form::Symbol(name) if !name.contains('/') => name.clone(),
                    _ => return Err("defprotocol name must be an unqualified symbol".into()),
                };
                let mut methods = HashMap::new();
                for declaration in &fs[2..] {
                    let Form::List(parts) = declaration else {
                        return Err("defprotocol method declaration must be a list".into());
                    };
                    if parts.len() != 2
                        || !matches!(&parts[0], Form::Symbol(_))
                        || !matches!(&parts[1], Form::Vector(_))
                    {
                        return Err(
                            "defprotocol method declaration expects a name and parameter vector"
                                .into(),
                        );
                    }
                    let Form::Symbol(method) = &parts[0] else {
                        unreachable!()
                    };
                    if method.ends_with('!') {
                        return Err("protocol method names must not end with !".into());
                    }
                    let Form::Vector(arguments) = &parts[1] else {
                        unreachable!()
                    };
                    if arguments.is_empty()
                        || methods.insert(method.clone(), arguments.len()).is_some()
                    {
                        return Err("protocol methods must be unique and take a receiver".into());
                    }
                }
                let namespace = namespace_registry()?.current().name().as_str().to_owned();
                let protocol = Value::Protocol(Rc::new(GuestProtocol {
                    name: format!("{namespace}/{name}"),
                    methods,
                }));
                if let Value::Protocol(protocol_value) = &protocol {
                    ACTIVE_PROTOCOLS.with(|active| -> Result<(), String> {
                        let registry = active.borrow();
                        let registry = registry
                            .as_ref()
                            .ok_or_else(|| "protocol registry is unavailable".to_string())?;
                        for method in protocol_value.methods.keys() {
                            registry.declare_guest(protocol_value.name.clone(), method.clone());
                        }
                        Ok(())
                    })?;
                    let current = namespace_registry()?.current();
                    for method in protocol_value.methods.keys() {
                        let local_name = format!("{name}/{method}");
                        let qualified_name = format!("{namespace}/{local_name}");
                        let protocol_name = protocol_value.name.clone();
                        let method_name = method.clone();
                        let function_name = qualified_name.clone();
                        let method_value =
                            native_variadic_function(&function_name, move |arguments| {
                                protocol_call(&protocol_name, &method_name, &arguments)
                            });
                        let method_var = current.intern(&local_name, method_value);
                        method_var.set_origin(definition_origin());
                        env.insert(local_name, Value::Var(method_var.clone()));
                        env.insert(qualified_name, Value::Var(method_var));
                    }
                }
                let var = KernelVar::new(format!("{namespace}/{name}"), protocol.clone());
                var.set_origin(definition_origin());
                env.insert(name, Value::Var(var));
                Ok(protocol)
            }
            Form::Symbol(n) if n == "extend-type" => {
                if fs.len() < 4 {
                    return Err(
                        "extend-type expects a type, protocol, and method implementations".into(),
                    );
                }
                let ty = match eval(&fs[1], env)? {
                    Value::StructType(ty) => ty,
                    _ => return Err("extend-type expects a struct type".into()),
                };
                let protocol = match eval(&fs[2], env)? {
                    Value::Protocol(protocol) => protocol,
                    _ => return Err("extend-type expects a protocol".into()),
                };
                let mut seen = HashSet::new();
                for implementation in &fs[3..] {
                    let Form::List(parts) = implementation else {
                        return Err("extend-type implementations must be method forms".into());
                    };
                    if parts.len() < 3 {
                        return Err("extend-type implementations require a body".into());
                    }
                    let Form::Symbol(method) = &parts[0] else {
                        return Err("extended method name must be a symbol".into());
                    };
                    let Form::Vector(arguments) = &parts[1] else {
                        return Err("extended method arguments must be a vector".into());
                    };
                    if !seen.insert(method.clone()) {
                        return Err("Duplicate extended method".into());
                    }
                    let valid_arity = protocol.methods.get(method).is_some_and(|expected| {
                        *expected == arguments.len()
                            || (*expected == usize::MAX && !arguments.is_empty())
                    });
                    if !valid_arity {
                        return Err(format!("invalid protocol method implementation: {method}"));
                    }
                    let function = eval(
                        &Form::List(
                            std::iter::once(Form::Symbol("fn".into()))
                                .chain(parts[1..].iter().cloned())
                                .collect(),
                        ),
                        env,
                    )?;
                    let Value::Function(function) = function else {
                        unreachable!()
                    };
                    ACTIVE_PROTOCOLS.with(|active| -> Result<(), String> {
                        let registry = active.borrow();
                        let registry = registry
                            .as_ref()
                            .ok_or_else(|| "protocol registry is unavailable".to_string())?;
                        registry.register_guest(
                            protocol.name.clone(),
                            ty.name.clone(),
                            method.clone(),
                            function,
                        );
                        Ok(())
                    })?;
                }
                Ok(Value::Protocol(protocol))
            }
            Form::Symbol(n) if n == "defmacro" => {
                if fs.len() < 4 {
                    return Err("defmacro expects a name, parameters, and a body".into());
                }
                let (name, metadata) = binding_symbol(&fs[1], "defmacro name")?;
                if let Some(value) = protected_fallback_binding(env, &name) {
                    return Ok(value);
                }
                let cell = match env.get(&name) {
                    Some(Value::Var(cell)) if binding_is_local(cell) => cell.clone(),
                    _ => KernelVar::new(name.clone(), Value::Nil),
                };
                if metadata.is_some() {
                    cell.set_hara_metadata(metadata);
                }
                env.insert(name.clone(), Value::Var(cell.clone()));
                let mut rest = &fs[2..];
                if matches!(rest.first(), Some(Form::String(_))) {
                    rest = &rest[1..];
                }
                if matches!(rest.first(), Some(Form::Map(_))) {
                    rest = &rest[1..];
                }
                if rest.is_empty() {
                    return Err("defmacro expects a name, parameters, and a body".into());
                }
                let function = if matches!(rest.first(), Some(Form::Vector(_))) {
                    let params = match &rest[0] {
                        Form::Vector(params) => params,
                        _ => unreachable!(),
                    };
                    let mut macro_params =
                        vec![Form::Symbol("&form".into()), Form::Symbol("&env".into())];
                    macro_params.extend_from_slice(params);
                    let (params, variadic) = function_parts(&Form::Vector(macro_params))?;
                    Value::Function(Rc::new(Function {
                        params,
                        variadic,
                        body: rest[1..].to_vec(),
                        captured: Rc::new(RefCell::new(env.clone())),
                        name: Some(name.clone()),
                        native: None,
                        clauses: Vec::new(),
                        is_macro: true,
                    }))
                } else {
                    let clauses = rest
                        .iter()
                        .map(macro_clause_with_implicit_params)
                        .collect::<Result<Vec<_>, _>>()?;
                    multi_arity_function(&name, &clauses, env, true)?
                };
                if let Value::Function(ref function) = function {
                    let namespace = namespace_registry()?.current().name().as_str().to_owned();
                    register_macro(&namespace, &name, function.clone())?;
                }
                cell.reset_value(function.clone());
                cell.set_origin(definition_origin());
                Ok(function)
            }
            Form::Symbol(n) if n == "defn" || n == "defn-" => {
                if fs.len() < 4 {
                    return Err("defn expects a name, parameters, and a body".into());
                }
                let (name, metadata) = binding_symbol(&fs[1], "defn name")?;
                if let Some(value) = protected_fallback_binding(env, &name) {
                    return Ok(value);
                }
                let cell = match env.get(&name) {
                    Some(Value::Var(cell)) if binding_is_local(cell) => cell.clone(),
                    _ => KernelVar::new(name.clone(), Value::Nil),
                };
                if metadata.is_some() {
                    cell.set_hara_metadata(metadata);
                }
                env.insert(name.clone(), Value::Var(cell.clone()));
                // Optional docstring and attr-map sit between the name and
                // the parameter vector (or arity clauses).
                let mut rest = &fs[2..];
                if matches!(rest.first(), Some(Form::String(_))) {
                    rest = &rest[1..];
                }
                if matches!(rest.first(), Some(Form::Map(_))) {
                    rest = &rest[1..];
                }
                if rest.is_empty() {
                    return Err("defn expects a name, parameters, and a body".into());
                }
                let function = if matches!(rest.first(), Some(Form::Vector(_))) {
                    let (params, variadic) = function_parts(&rest[0])?;
                    Value::Function(Rc::new(Function {
                        params,
                        variadic,
                        body: rest[1..].to_vec(),
                        captured: Rc::new(RefCell::new(env.clone())),
                        name: Some(name.clone()),
                        native: None,
                        clauses: Vec::new(),
                        is_macro: false,
                    }))
                } else {
                    multi_arity_function(&name, rest, env, false)?
                };
                cell.reset_value(function.clone());
                cell.set_origin(definition_origin());
                Ok(Value::Var(cell))
            }
            Form::Symbol(n) if n == "do" => {
                let mut result = Value::Nil;
                for form in &fs[1..] {
                    result = eval(form, env)?;
                    if matches!(result, Value::Recur(_)) {
                        return Ok(result);
                    }
                }
                Ok(result)
            }
            Form::Symbol(n) if n == "declare" => {
                for form in &fs[1..] {
                    if !matches!(form, Form::Symbol(_)) {
                        return Err("declare expects symbols".into());
                    }
                }
                Ok(Value::Nil)
            }
            Form::Symbol(n) if n == "=" => {
                if fs.len() < 3 {
                    return Err("= expects at least 2 arguments".into());
                }
                let first = eval(&fs[1], env)?;
                Ok(Value::Bool(
                    fs[2..]
                        .iter()
                        .map(|form| eval(form, env))
                        .collect::<Result<Vec<_>, _>>()?
                        .iter()
                        .all(|value| *value == first),
                ))
            }
            Form::Symbol(n) if n == "ns" || n == "require" => eval_namespace_form(fs, env),
            Form::Symbol(n) if n == "std.foundation.coroutine/create" => {
                if fs.len() != 2 {
                    return Err("coroutine/create expects one function".into());
                }
                let body = eval(&fs[1], env)?;
                match body {
                    Value::Function(_) => Ok(Value::Coroutine(Rc::new(Coroutine::new(body)))),
                    _ => Err("coroutine/create expects a function".into()),
                }
            }
            Form::Symbol(n) if n == "std.foundation.coroutine/coroutine?" => {
                if fs.len() != 2 {
                    return Err("coroutine/coroutine? expects one value".into());
                }
                Ok(Value::Bool(matches!(
                    eval(&fs[1], env)?,
                    Value::Coroutine(_)
                )))
            }
            Form::Symbol(n) if n == "std.foundation.coroutine/status" => {
                if fs.len() != 2 {
                    return Err("coroutine/status expects one coroutine".into());
                }
                match eval(&fs[1], env)? {
                    Value::Coroutine(coroutine) => Ok(coroutine_status(&coroutine)),
                    _ => Err("coroutine/status expects a coroutine".into()),
                }
            }
            Form::Symbol(n) if n == "std.foundation.coroutine/close" => {
                if fs.len() != 2 {
                    return Err("coroutine/close expects one coroutine".into());
                }
                match eval(&fs[1], env)? {
                    Value::Coroutine(coroutine) => {
                        coroutine_close(&coroutine)?;
                        Ok(Value::Coroutine(coroutine))
                    }
                    _ => Err("coroutine/close expects a coroutine".into()),
                }
            }
            Form::Symbol(n)
                if n == "std.foundation.coroutine/resume"
                    || n == "std.protocol.icoroutine/resume" =>
            {
                Err("coroutine/resume requires the fiber evaluator".into())
            }
            Form::Symbol(n) if n == "std.foundation.coroutine/yield" => {
                Err("coroutine/yield requires the fiber evaluator".into())
            }
            Form::Symbol(n) if n == "std.foundation.coroutine/await" => {
                Err("coroutine/await requires the fiber evaluator".into())
            }
            Form::Symbol(n) if matches!(env.get(n), Some(Value::Var(var)) if (binding_is_local(var) || var.origin() == VarOrigin::RustLibrary) && matches!(var.deref_value(), Value::Function(_))) =>
            {
                let function = binding_value(env, n).expect("function binding was checked");
                let arguments = fs[1..]
                    .iter()
                    .map(|form| eval(form, env))
                    .collect::<Result<Vec<_>, _>>()?;
                call_value(function, arguments)
            }
            Form::Symbol(n) if n == "protocol-call" => {
                if fs.len() < 4 {
                    return Err(
                        "protocol-call expects protocol, method, value, and optional arguments"
                            .into(),
                    );
                }
                let protocol = match eval(&fs[1], env) {
                    Ok(Value::Protocol(protocol)) => protocol.name.clone(),
                    Ok(_) => return Err("protocol-call expects a protocol".into()),
                    Err(_) => match &fs[1] {
                        Form::Symbol(name) => name.clone(),
                        _ => return Err("protocol name must be a symbol".into()),
                    },
                };
                let method = match &fs[2] {
                    Form::Symbol(name) => name.as_str(),
                    _ => return Err("protocol method must be a symbol".into()),
                };
                let mut arguments = vec![eval(&fs[3], env)?];
                arguments.extend(
                    fs[4..]
                        .iter()
                        .map(|form| eval(form, env))
                        .collect::<Result<Vec<_>, _>>()?,
                );
                protocol_call(&protocol, method, &arguments)
            }
            Form::Symbol(n) if n == "promise" || n == "promise/run" => {
                if fs.len() != 2 {
                    return Err("promise expects one function".into());
                }
                let function = match eval(&fs[1], env)? {
                    Value::Function(function) => function,
                    _ => return Err("promise expects a function".into()),
                };
                let provider = promise_provider();
                let task = Rc::new(move || call_function(&function, Vec::new()));
                Ok(Value::Promise(provider.run(task)))
            }
            Form::Symbol(n) if n == "promise?" => {
                if fs.len() != 2 {
                    return Err("promise? expects one value".into());
                }
                Ok(Value::Bool(matches!(eval(&fs[1], env)?, Value::Promise(_))))
            }
            Form::Symbol(n)
                if ["bytes?", "array?", "object?", "regexp?", "uuid?"]
                    .contains(&n.as_str()) =>
            {
                if fs.len() != 2 {
                    return Err(format!("{n} expects one value"));
                }
                let value = eval(&fs[1], env)?;
                Ok(Value::Bool(match n.as_str() {
                    "bytes?" => matches!(value, Value::Bytes(_) | Value::ByteBuffer(_)),
                    "array?" => matches!(value, Value::Array(_)),
                    "object?" => matches!(value, Value::Object(_)),
                    "regexp?" => matches!(value, Value::Regex(_)),
                    // UUID values are not yet represented by the Rust value model.
                    "uuid?" => false,
                    _ => unreachable!(),
                }))
            }
            Form::Symbol(n) if n == "host/call" => {
                if fs.len() < 3 {
                    return Err("host/call expects service, method, and optional arguments".into());
                }
                let service = match eval(&fs[1], env)? {
                    Value::String(value) => value,
                    _ => return Err("host/call service must be a string".into()),
                };
                let method = match eval(&fs[2], env)? {
                    Value::String(value) => value,
                    _ => return Err("host/call method must be a string".into()),
                };
                let arguments = fs[3..]
                    .iter()
                    .map(|form| eval(form, env))
                    .collect::<Result<Vec<_>, _>>()?;
                HOST_CALL_HANDLER.with(|active| {
                    let handler = active
                        .borrow()
                        .as_ref()
                        .cloned()
                        .ok_or_else(|| "host/call is unavailable".to_string())?;
                    handler(service, method, arguments)
                })
            }
            Form::Symbol(n) if n == "promise/new" => {
                if fs.len() != 2 {
                    return Err("promise/new expects one function".into());
                }
                let function = match eval(&fs[1], env)? {
                    Value::Function(function) => function,
                    _ => return Err("promise/new expects a function".into()),
                };
                let promise = Promise::new();
                let resolving = promise.clone();
                let resolve = native_function("promise-resolve", 1, move |mut values| {
                    let value = values.remove(0);
                    settle_promise_result(&resolving, Ok(value.clone()));
                    Ok(value)
                });
                let rejecting = promise.clone();
                let reject = native_function("promise-reject", 1, move |mut values| {
                    let value = values.remove(0);
                    let error = match &value {
                        Value::String(error) => error.clone(),
                        value => value.display(),
                    };
                    rejecting.reject(error);
                    Ok(value)
                });
                if let Err(error) = call_function(&function, vec![resolve, reject]) {
                    promise.reject(error);
                }
                Ok(Value::Promise(promise))
            }
            Form::Symbol(n) if n == "promise/from" => {
                if fs.len() != 2 {
                    return Err("promise/from expects one value".into());
                }
                let value = eval(&fs[1], env)?;
                Ok(Value::Promise(promise_from(value)))
            }
            Form::Symbol(n) if n == "promise/all" => {
                if fs.len() != 2 {
                    return Err("promise/all expects one collection".into());
                }
                Ok(Value::Promise(promise_all(iterator_values(eval(
                    &fs[1], env,
                )?)?)))
            }
            Form::Symbol(n) if n == "promise/delay" => {
                if fs.len() != 3 {
                    return Err("promise/delay expects milliseconds and a function".into());
                }
                let millis = match eval(&fs[1], env)? {
                    Value::Number(value) if value >= 0 => value as u64,
                    _ => return Err("promise/delay expects non-negative milliseconds".into()),
                };
                let function = match eval(&fs[2], env)? {
                    Value::Function(function) => function,
                    _ => return Err("promise/delay expects milliseconds and a function".into()),
                };
                let task = Rc::new(move || call_function(&function, Vec::new()));
                Ok(Value::Promise(
                    promise_provider().delay(std::time::Duration::from_millis(millis), task),
                ))
            }
            Form::Symbol(n) if n == "promise/state" => {
                if fs.len() != 2 {
                    return Err("promise/state expects one promise".into());
                }
                let promise = promise_value(&eval(&fs[1], env)?, n)?;
                Ok(promise_state_value(&promise))
            }
            Form::Symbol(n) if n == "promise/value" => {
                if fs.len() != 2 {
                    return Err("promise/value expects one promise".into());
                }
                let promise = promise_value(&eval(&fs[1], env)?, n)?;
                promise_value_result(&promise)
            }
            Form::Symbol(n) if n == "promise/cancel" => {
                if fs.len() != 2 {
                    return Err("promise/cancel expects a promise".into());
                }
                let promise = promise_value(&eval(&fs[1], env)?, n)?;
                if !promise.cancel() {
                    return Err("promise is already settled".into());
                }
                Ok(Value::Promise(promise))
            }
            Form::Symbol(n)
                if ["promise/then", "promise/catch", "promise/finally"].contains(&n.as_str()) =>
            {
                if fs.len() != 3 {
                    return Err(format!("{n} expects a promise and function"));
                }
                let source = promise_value(&eval(&fs[1], env)?, n)?;
                let function = match eval(&fs[2], env)? {
                    Value::Function(function) => function,
                    _ => return Err(format!("{n} expects a function")),
                };
                Ok(Value::Promise(promise_chain(source, n, function)))
            }
            Form::Symbol(n) if n.starts_with("ns:") => eval_namespace_operation(n, fs, env),
            Form::Symbol(n) if n == "keyword" || n == "symbol" || n == "pointer" => {
                if fs.len() != 2 && fs.len() != 3 {
                    return Err(format!("{n} expects a name or namespace and name"));
                }
                let parts = fs[1..]
                    .iter()
                    .map(|form| match eval(form, env)? {
                        Value::String(value) => Ok(value),
                        _ => Err(format!("{n} expects string arguments")),
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                match (n.as_str(), parts.as_slice()) {
                    ("keyword", [name]) => Keyword::parse(name)
                        .map(Value::Keyword)
                        .map_err(|error| format!("keyword failed: {error}")),
                    ("keyword", [namespace, name]) => Keyword::create(Some(namespace), name)
                        .map(Value::Keyword)
                        .map_err(|error| format!("keyword failed: {error}")),
                    ("symbol", [name]) => Ok(Value::Symbol(Symbol::parse(name))),
                    ("symbol", [namespace, name]) => {
                        Ok(Value::Symbol(Symbol::create(Some(namespace), name)))
                    }
                    ("pointer", [name]) => Ok(Value::Pointer(PPointer::parse(name))),
                    ("pointer", [namespace, name]) => {
                        Ok(Value::Pointer(PPointer::create(Some(namespace), name)))
                    }
                    _ => unreachable!(),
                }
            }
            Form::Symbol(n) if n == "name" => {
                if fs.len() != 2 {
                    return Err("name expects one value".into());
                }
                match eval(&fs[1], env)? {
                    Value::Keyword(value) => Ok(Value::String(value.as_str().into())),
                    Value::Symbol(value) => Ok(Value::String(value.as_str().into())),
                    _ => Err("name expects a keyword or symbol".into()),
                }
            }
            Form::Symbol(n) if ["list", "vector", "pair", "tup"].contains(&n.as_str()) => {
                eval_sequential_constructor(n, &fs[1..], env)
            }
            Form::Symbol(n) if n == "vec" => {
                if fs.len() != 2 {
                    return Err("vec expects one argument".into());
                }
                let value = eval(&fs[1], env)?;
                Ok(match iterator_to_vec(value) {
                    Ok(values) => Value::Vector(PVector::from_iter(values)),
                    Err(error) => return Err(error),
                })
            }
            Form::Symbol(n)
                if [
                    "hash-map",
                    "hash-set",
                    "ordered-map",
                    "ordered-set",
                    "queue",
                    "sorted-map",
                    "sorted-set",
                    "trie",
                ]
                .contains(&n.as_str()) =>
            {
                eval_collection_constructor(n, &fs[1..], env)
            }
            Form::Symbol(n) if n == "set" => Ok(Value::Set(
                unique_values(
                    fs[1..]
                        .iter()
                        .map(|form| eval(form, env))
                        .collect::<Result<_, _>>()?,
                )
                .into(),
            )),
            Form::Symbol(n) if n == "array" => {
                let values = fs[1..]
                    .iter()
                    .map(|form| eval(form, env))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Value::Array(Rc::new(RefCell::new(values))))
            }
            Form::Symbol(n) if n == "object" => {
                if fs.len() % 2 != 1 {
                    return Err("object expects key/value pairs".into());
                }
                let mut values = Vec::new();
                for pair in fs[1..].chunks(2) {
                    let key = marker_key(&eval(&pair[0], env)?, "object")?;
                    values.push((key, eval(&pair[1], env)?));
                }
                Ok(Value::Object(Rc::new(RefCell::new(values))))
            }
            Form::Symbol(n) if n == "." => {
                if fs.len() != 3 {
                    return Err("dot expects a receiver and method".into());
                }
                let receiver = eval(&fs[1], env)?;
                dot_call(receiver, &fs[2], env)
            }
            Form::Symbol(n) if n == "bytes" => {
                let values = fs[1..]
                    .iter()
                    .map(|form| eval(form, env))
                    .collect::<Result<Vec<_>, _>>()?;
                let values = values
                    .iter()
                    .map(|value| byte_input(value, "bytes"))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Value::ByteBuffer(Rc::new(RefCell::new(values))))
            }
            Form::Symbol(n)
                if ["socket/connect", "socket/listen", "socket/endpoint", "socket/events", "socket/next", "socket/send", "socket/close"].contains(&n.as_str())
                    || n.starts_with("std.native.Socket/") =>
            {
                socket_operation(n, &fs[1..], env)
            }
            Form::Symbol(n)
                if [
                    "file/resolve",
                    "file/read",
                    "file/write",
                    "file/exists?",
                    "file/list",
                    "file/mkdir",
                    "file/delete",
                ]
                .contains(&n.as_str()) =>
            {
                file_operation(n, &fs[1..], env)
            }
            Form::Symbol(n) if n == "str" => {
                if fs.len() == 1 {
                    return Ok(Value::String(String::new()));
                }
                let values = fs[1..]
                    .iter()
                    .map(|form| eval(form, env))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Value::String(
                    values
                        .iter()
                        .map(|value| match value {
                            Value::String(text) => text.clone(),
                            _ => value.display(),
                        })
                        .collect::<Vec<_>>()
                        .join(""),
                ))
            }
            Form::Symbol(n) if n == "pr-str" => {
                if fs.len() != 2 {
                    return Err("pr-str expects one value".into());
                }
                Ok(Value::String(eval(&fs[1], env)?.display()))
            }
            Form::Symbol(n)
                if [
                    "str/comp",
                    "str/lt?",
                    "str/gt?",
                    "str/pad-left",
                    "str/pad-right",
                    "str/starts-with?",
                    "str/ends-with?",
                    "str/char",
                    "str/split",
                    "str/join",
                    "str/index-of",
                    "str/substring",
                    "str/to-fixed",
                    "str/replace",
                    "str/trim-left",
                    "str/trim-right",
                    "str/length",
                    "str/blank?",
                    "str/includes?",
                    "str/char-at",
                    "str/slice",
                    "str/last-index-of",
                    "str/split-lines",
                    "str/repeat",
                    "str/replace-first",
                    "str/capitalize",
                    "str/decapitalize",
                    "str/reverse",
                    "str/encode-utf8",
                    "str/decode-utf8",
                ]
                .contains(&n.as_str()) =>
            {
                let values = fs[1..]
                    .iter()
                    .map(|form| eval(form, env))
                    .collect::<Result<Vec<_>, _>>()?;
                string_operation(n, values)
            }
            Form::Symbol(n)
                if n == "str/count" || n == "str/trim" || n == "str/upper" || n == "str/lower" =>
            {
                if fs.len() != 2 {
                    return Err(format!("{n} expects one string"));
                }
                let text = match eval(&fs[1], env)? {
                    Value::String(text) => text,
                    _ => return Err(format!("{n} expects a string")),
                };
                match n.as_str() {
                    "str/count" => Ok(Value::Number(text.chars().count() as i64)),
                    "str/trim" => Ok(Value::String(text.trim().into())),
                    "str/upper" => Ok(Value::String(text.to_uppercase())),
                    "str/lower" => Ok(Value::String(text.to_lowercase())),
                    _ => unreachable!(),
                }
            }
            Form::Symbol(n) if n == "str/encode" => {
                if fs.len() != 2 {
                    return Err("str/encode expects one string".into());
                }
                match eval(&fs[1], env)? {
                    Value::String(text) => {
                        Ok(Value::ByteBuffer(Rc::new(RefCell::new(text.into_bytes()))))
                    }
                    _ => Err("str/encode expects a string".into()),
                }
            }
            Form::Symbol(n) if n == "str/decode" => {
                if fs.len() != 2 {
                    return Err("str/decode expects bytes".into());
                }
                let bytes = byte_buffer(&eval(&fs[1], env)?, "str/decode")?;
                let raw = bytes.borrow().clone();
                String::from_utf8(raw)
                    .map(Value::String)
                    .map_err(|_| "str/decode invalid UTF-8".into())
            }
            Form::Symbol(n) if n == "bytes/copy" => {
                if fs.len() != 2 {
                    return Err("bytes/copy expects bytes".into());
                }
                byte_copy(&eval(&fs[1], env)?)
            }
            Form::Symbol(n) if n == "bytes/slice" => {
                if fs.len() != 3 && fs.len() != 4 {
                    return Err("bytes/slice expects bytes, start, and optional end".into());
                }
                let value = eval(&fs[1], env)?;
                let start = eval(&fs[2], env)?;
                let end = if fs.len() == 4 {
                    eval(&fs[3], env)?
                } else {
                    byte_count(&value)?
                };
                byte_slice(&value, &start, &end)
            }
            Form::Symbol(n) if n == "bytes/count" => {
                if fs.len() != 2 {
                    return Err("bytes/count expects one argument".into());
                }
                byte_count(&eval(&fs[1], env)?)
            }
            Form::Symbol(n) if n == "bytes/get" => {
                if fs.len() != 3 && fs.len() != 4 {
                    return Err("bytes/get expects an index and optional default".into());
                }
                let value = eval(&fs[1], env)?;
                let index = eval(&fs[2], env)?;
                let default = if fs.len() == 4 {
                    Some(eval(&fs[3], env)?)
                } else {
                    None
                };
                let index_num = value_index(&index)?;
                match byte_get(&value, &index, default) {
                    Ok(value) => Ok(value),
                    Err(error) if error.is_empty() => {
                        Err(format!("bytes/get index out of bounds: {index_num}"))
                    }
                    Err(error) => Err(error),
                }
            }
            Form::Symbol(n) if n == "bytes/set" => {
                if fs.len() != 4 {
                    return Err("bytes/set expects bytes, index, and value".into());
                }
                let value = eval(&fs[1], env)?;
                let index = eval(&fs[2], env)?;
                let item = eval(&fs[3], env)?;
                byte_set(&value, &index, &item)
            }
            Form::Symbol(n) if n == "bytes/u8" || n == "bytes/s8" => {
                if fs.len() != 2 {
                    return Err(format!("{n} expects one argument"));
                }
                let number = match eval(&fs[1], env)? {
                    Value::Number(number) => number,
                    _ => return Err(format!("{n} expects a number")),
                };
                if !(-128..=255).contains(&number) {
                    return Err(format!("{n} expects a value in the range -128..255"));
                }
                let raw = (number as i8) as u8;
                Ok(Value::Number(if n == "bytes/u8" {
                    raw as i64
                } else {
                    raw as i8 as i64
                }))
            }
            Form::Symbol(n) if n == "iter" => {
                if fs.len() != 2 {
                    return Err("iter expects one argument".into());
                }
                make_iterator(eval(&fs[1], env)?)
            }
            Form::Symbol(n) if n == "seq" => {
                if fs.len() != 2 && fs.len() != 3 {
                    return Err("seq expects a source, or a transform and source".into());
                }
                let source = eval(&fs[fs.len() - 1], env)?;
                let lazy = iterator_seq(source)?;
                if fs.len() == 2 {
                    Ok(lazy)
                } else {
                    match eval(&fs[1], env)? {
                        Value::Function(function) => {
                            let result = call_function(&function, vec![lazy])?;
                            iterator_seq(result)
                        }
                        _ => Err("seq expects a function and source".into()),
                    }
                }
            }
            Form::Symbol(n) if n == "seq?" || n == "iter?" => {
                if fs.len() != 2 {
                    return Err(format!("{n} expects one value"));
                }
                let value = eval(&fs[1], env)?;
                let result = matches!(value, Value::Iterator(iterator) if n == "seq?" && iterator.borrow().seq || n == "iter?" && !iterator.borrow().seq);
                Ok(Value::Bool(result))
            }
            Form::Symbol(n) if n == "iter-has?" => {
                if fs.len() != 2 {
                    return Err("iter-has? expects one argument".into());
                }
                iterator_has_next(&eval(&fs[1], env)?)
            }
            Form::Symbol(n) if n == "iter-next" => {
                if fs.len() != 2 {
                    return Err("iter-next expects one argument".into());
                }
                iterator_next(&eval(&fs[1], env)?)
            }
            Form::Symbol(n) if n == "iter-close" => {
                if fs.len() != 2 {
                    return Err("iter-close expects one argument".into());
                }
                iterator_close(&eval(&fs[1], env)?)
            }
            Form::Symbol(n)
                if ["iter-map", "map", "iter-filter", "filter"].contains(&n.as_str()) =>
            {
                let is_map = n == "iter-map" || n == "map";
                if n == "map" && fs.len() == 2 {
                    let function = match eval(&fs[1], env)? {
                        Value::Function(function) => function,
                        _ => return Err("map expects a function".into()),
                    };
                    let body = Form::List(vec![
                        Form::Symbol("__map-transform".into()),
                        Form::Symbol("__function".into()),
                        Form::Symbol("value".into()),
                    ]);
                    return Ok(generated_function(
                        vec!["value".into()],
                        vec![body],
                        env.clone(),
                        vec![("__function", Value::Function(function))],
                    ));
                }
                if fs.len() < 3 {
                    return Err(format!("{n} expects a function and collection"));
                }
                let function = eval(&fs[1], env)?;
                if is_map && fs.len() > 3 {
                    let sources = fs[2..]
                        .iter()
                        .map(|form| eval(form, env).and_then(make_iterator))
                        .collect::<Result<Vec<_>, _>>()?;
                    let zipped = iterator_zip(sources)?;
                    let result = match function {
                        Value::Function(function) => iterator_map(function, zipped)?,
                        _ => return Err(format!("{n} expects a function")),
                    };
                    return if n == "map" {
                        iterator_seq(result)
                    } else {
                        Ok(result)
                    };
                }
                let raw_collection = if fs.len() == 3 {
                    Some(eval(&fs[2], env)?)
                } else {
                    None
                };
                if fs.len() == 3 {
                    if let Value::Function(function_ref) = &function {
                        if is_map {
                            if let Some(value) = raw_collection.clone() {
                                return if n == "map" {
                                    iterator_seq(iterator_map(function_ref.clone(), value)?)
                                } else {
                                    iterator_map(function_ref.clone(), value)
                                };
                            }
                        } else if let Some(value) = raw_collection.clone() {
                            return if n == "filter" {
                                iterator_seq(iterator_filter(function_ref.clone(), value)?)
                            } else {
                                iterator_filter(function_ref.clone(), value)
                            };
                        }
                    }
                }
                let collections = if let Some(value) = raw_collection {
                    vec![iterator_values(value)?]
                } else {
                    fs[2..]
                        .iter()
                        .map(|form| eval(form, env).and_then(iterator_values))
                        .collect::<Result<Vec<_>, _>>()?
                };
                let mut output = Vec::new();
                if is_map {
                    let limit = collections.iter().map(Vec::len).min().unwrap_or(0);
                    for index in 0..limit {
                        let args = collections
                            .iter()
                            .map(|values| values[index].clone())
                            .collect();
                        let mapped = match &function {
                            Value::Function(f) => call_function(f, args)?,
                            _ => return Err(format!("{n} expects a function")),
                        };
                        output.push(mapped);
                    }
                } else {
                    if collections.len() != 1 {
                        return Err(format!("{n} expects one collection"));
                    }
                    for value in collections.into_iter().next().unwrap() {
                        let mapped = match &function {
                            Value::Function(f) => call_function(f, vec![value.clone()])?,
                            _ => return Err(format!("{n} expects a function")),
                        };
                        if mapped.truthy() {
                            output.push(value);
                        }
                    }
                }
                if n == "map" {
                    iterator_seq(iterator_from_values(output))
                } else {
                    Ok(iterator_from_values(output))
                }
            }
            Form::Symbol(n) if ["iter-take", "take"].contains(&n.as_str()) => {
                if fs.len() != 3 {
                    return Err(format!("{n} expects an amount and collection"));
                }
                let amount = value_index(&eval(&fs[1], env)?)?;
                let result = iterator_take(eval(&fs[2], env)?, amount)?;
                if n == "take" {
                    iterator_seq(result)
                } else {
                    Ok(result)
                }
            }
            Form::Symbol(n) if ["iter-drop", "drop"].contains(&n.as_str()) => {
                if fs.len() != 3 {
                    return Err(format!("{n} expects an amount and collection"));
                }
                let amount = value_index(&eval(&fs[1], env)?)?;
                let result = iterator_drop(eval(&fs[2], env)?, amount)?;
                if n == "drop" {
                    iterator_seq(result)
                } else {
                    Ok(result)
                }
            }
            Form::Symbol(n)
                if [
                    "iter-take-while",
                    "take-while",
                    "iter-drop-while",
                    "drop-while",
                ]
                .contains(&n.as_str()) =>
            {
                if fs.len() != 3 {
                    return Err(format!("{n} expects a predicate and collection"));
                }
                let predicate = match eval(&fs[1], env)? {
                    Value::Function(function) => function,
                    _ => return Err(format!("{n} expects a function")),
                };
                let value = eval(&fs[2], env)?;
                let result = if n.contains("take-while") {
                    iterator_take_while(predicate, value)?
                } else {
                    iterator_drop_while(predicate, value)?
                };
                if n.starts_with("iter-") {
                    Ok(result)
                } else {
                    iterator_seq(result)
                }
            }
            Form::Symbol(n) if ["iter-mapcat", "mapcat"].contains(&n.as_str()) => {
                if fs.len() != 3 {
                    return Err(format!("{n} expects a function and collection"));
                }
                let function = match eval(&fs[1], env)? {
                    Value::Function(function) => function,
                    _ => return Err(format!("{n} expects a function")),
                };
                let result = iterator_mapcat(function, eval(&fs[2], env)?)?;
                if n == "mapcat" {
                    iterator_seq(result)
                } else {
                    Ok(result)
                }
            }
            Form::Symbol(n) if ["iter-keep", "keep"].contains(&n.as_str()) => {
                if fs.len() != 3 {
                    return Err(format!("{n} expects a function and collection"));
                }
                let function = match eval(&fs[1], env)? {
                    Value::Function(function) => function,
                    _ => return Err(format!("{n} expects a function")),
                };
                let result = iterator_keep(function, eval(&fs[2], env)?)?;
                if n == "keep" {
                    iterator_seq(result)
                } else {
                    Ok(result)
                }
            }
            Form::Symbol(n)
                if [
                    "iter-partition-all",
                    "partition-all",
                    "iter-partition",
                    "partition",
                ]
                .contains(&n.as_str()) =>
            {
                if fs.len() != 3 {
                    return Err(format!("{n} expects an amount and collection"));
                }
                let amount = value_index(&eval(&fs[1], env)?)?;
                let result = iterator_partition(eval(&fs[2], env)?, amount, n.contains("all"))?;
                if n.starts_with("iter-") {
                    Ok(result)
                } else {
                    iterator_seq(result)
                }
            }
            Form::Symbol(n) if ["iter-interpose", "interpose"].contains(&n.as_str()) => {
                if fs.len() != 3 {
                    return Err(format!("{n} expects a separator and collection"));
                }
                let separator = eval(&fs[1], env)?;
                let values = iterator_values(eval(&fs[2], env)?)?;
                let mut output = Vec::new();
                for (index, value) in values.into_iter().enumerate() {
                    if index > 0 {
                        output.push(separator.clone());
                    }
                    output.push(value);
                }
                let result = iterator_from_values(output);
                if n == "interpose" {
                    iterator_seq(result)
                } else {
                    Ok(result)
                }
            }
            Form::Symbol(n) if ["iter-interleave", "interleave"].contains(&n.as_str()) => {
                if fs.len() < 2 {
                    return Err(format!("{n} expects collections"));
                }
                let collections = fs[1..]
                    .iter()
                    .map(|form| eval(form, env))
                    .collect::<Result<Vec<_>, _>>()?;
                let result = iterator_interleave(collections)?;
                if n == "interleave" {
                    iterator_seq(result)
                } else {
                    Ok(result)
                }
            }
            Form::Symbol(n) if ["iter-partition-pair", "partition-pair"].contains(&n.as_str()) => {
                if fs.len() != 2 {
                    return Err(format!("{n} expects one collection"));
                }
                let values = iterator_values(eval(&fs[1], env)?)?;
                let result = iterator_from_values(
                    values
                        .chunks(2)
                        .filter(|chunk| chunk.len() == 2)
                        .map(|chunk| Value::Vector(chunk.to_vec().into()))
                        .collect(),
                );
                if n == "partition-pair" {
                    iterator_seq(result)
                } else {
                    Ok(result)
                }
            }
            Form::Symbol(n) if ["iter-zip", "zip"].contains(&n.as_str()) => {
                if fs.len() < 3 {
                    return Err(format!("{n} expects collections"));
                }
                let collections = fs[1..]
                    .iter()
                    .map(|form| eval(form, env))
                    .collect::<Result<Vec<_>, _>>()?;
                let result = iterator_zip(collections)?;
                if n == "zip" {
                    iterator_seq(result)
                } else {
                    Ok(result)
                }
            }
            Form::Symbol(n) if n == "iter-cycle" || n == "cycle" => {
                if fs.len() != 2 {
                    return Err(format!("{n} expects one collection"));
                }
                let result = iterator_cycle(eval(&fs[1], env)?)?;
                if n == "cycle" {
                    iterator_seq(result)
                } else {
                    Ok(result)
                }
            }
            Form::Symbol(n) if n == "concat" => {
                if fs.len() < 2 {
                    return Err("concat expects collections".into());
                }
                let mut values = Vec::new();
                for form in &fs[1..] {
                    values.extend(iterator_values(eval(form, env)?)?);
                }
                iterator_seq(iterator_from_values(values))
            }
            Form::Symbol(n) if n == "range" => {
                if fs.len() < 1 || fs.len() > 3 {
                    return Err("range expects zero, one, or two bounds".into());
                }
                let nums = fs[1..]
                    .iter()
                    .map(|form| match eval(form, env)? {
                        Value::Number(v) => Ok(v),
                        _ => Err("range bounds must be numbers".into()),
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                let (start, end) = match nums.as_slice() {
                    [] => (0, 0),
                    [end] => (0, *end),
                    [start, end] => (*start, *end),
                    _ => unreachable!(),
                };
                iterator_seq(iterator_from_values(
                    (start..end).map(Value::Number).collect(),
                ))
            }
            Form::Symbol(n) if n == "repeat" => {
                if fs.len() != 2 && fs.len() != 3 {
                    return Err("repeat expects a value or amount and value".into());
                }
                let (amount, form) = if fs.len() == 2 {
                    (None, &fs[1])
                } else {
                    (Some(value_index(&eval(&fs[1], env)?)?), &fs[2])
                };
                let value = eval(form, env)?;
                if amount.is_none() {
                    return iterator_seq(iterator_constant(value));
                }
                let count = amount.unwrap();
                iterator_seq(iterator_from_values(
                    (0..count).map(|_| value.clone()).collect(),
                ))
            }
            Form::Symbol(n) if n == "repeatedly" => {
                if fs.len() != 2 && fs.len() != 3 {
                    return Err("repeatedly expects a function or amount and function".into());
                }
                let (amount, form) = if fs.len() == 2 {
                    (None, &fs[1])
                } else {
                    (Some(value_index(&eval(&fs[1], env)?)?), &fs[2])
                };
                let function = match eval(form, env)? {
                    Value::Function(function) => function,
                    _ => return Err("repeatedly expects a function".into()),
                };
                let generated = iterator_repeated(function);
                let result = if let Some(amount) = amount {
                    iterator_take(generated, amount)?
                } else {
                    generated
                };
                iterator_seq(result)
            }
            Form::Symbol(n) if n == "iterate" => {
                if fs.len() != 3 {
                    return Err("iterate expects a function and seed".into());
                }
                let function = match eval(&fs[1], env)? {
                    Value::Function(function) => function,
                    _ => return Err("iterate expects a function".into()),
                };
                iterator_seq(iterator_iterate(function, eval(&fs[2], env)?))
            }
            Form::Symbol(n) if n == "iter-constantly" => {
                if fs.len() != 2 {
                    return Err("iter-constantly expects a value".into());
                }
                Ok(iterator_constant(eval(&fs[1], env)?))
            }
            Form::Symbol(n) if n == "iter-repeatedly" => {
                if fs.len() != 2 {
                    return Err("iter-repeatedly expects a function".into());
                }
                match eval(&fs[1], env)? {
                    Value::Function(function) => Ok(iterator_repeated(function)),
                    _ => Err("iter-repeatedly expects a function".into()),
                }
            }
            Form::Symbol(n) if n == "iter-iterate" => {
                if fs.len() != 3 {
                    return Err("iter-iterate expects a function and seed".into());
                }
                let function = match eval(&fs[1], env)? {
                    Value::Function(function) => function,
                    _ => return Err("iter-iterate expects a function".into()),
                };
                Ok(iterator_iterate(function, eval(&fs[2], env)?))
            }
            Form::Symbol(n)
                if [
                    "bit-and",
                    "bit-or",
                    "bit-xor",
                    "bit-not",
                    "bit-shift-left",
                    "bit-shift-right",
                ]
                .contains(&n.as_str()) =>
            {
                bit_operation(n, &fs[1..], env)
            }
            Form::Symbol(n)
                if n.starts_with("std.native.Bits/")
                    && ["and", "or", "xor", "not", "shift-left", "shift-right"]
                        .contains(&n.trim_start_matches("std.native.Bits/")) =>
            {
                bit_operation(n, &fs[1..], env)
            }
            Form::Symbol(n)
                if n.starts_with("std.native.Numbers/")
                    && ["long", "double"]
                        .contains(&n.trim_start_matches("std.native.Numbers/")) =>
            {
                number_conversion(n, &fs[1..], env)
            }
            Form::Symbol(n)
                if n.starts_with("std.native.Maths/")
                    && [
                        "abs", "acos", "acosh", "asin", "asinh", "atan", "atan2", "atanh",
                        "ceil", "cos", "cosh", "exp", "floor", "pow", "sin", "sinh", "sqrt",
                        "tan", "tanh",
                    ]
                    .contains(&n.trim_start_matches("std.native.Maths/")) =>
            {
                math_operation(n, &fs[1..], env)
            }
            Form::Symbol(n) if n == "std.native.Edn/read" => {
                if fs.len() != 2 {
                    return Err("edn/read expects one string".into());
                }
                match eval(&fs[1], env)? {
                    Value::String(source) => read_edn(&source),
                    _ => Err("edn/read expects a string".into()),
                }
            }
            Form::Symbol(n)
                if n.starts_with("std.native.Error/")
                    && ["new", "message", "class"]
                        .contains(&n.trim_start_matches("std.native.Error/")) =>
            {
                native_error_operation(n, &fs[1..], env)
            }
            Form::Symbol(n) if ["inc", "dec"].contains(&n.as_str()) => {
                if fs.len() != 2 {
                    return Err(format!("{n} expects one number"));
                }
                match eval(&fs[1], env)? {
                    Value::Number(value) => Ok(Value::Number(if n == "inc" {
                        value + 1
                    } else {
                        value - 1
                    })),
                    _ => Err(format!("{n} expects a number")),
                }
            }
            Form::Symbol(n) if n == "__map-transform" => {
                if fs.len() != 3 {
                    return Err("map transform expects a function and source".into());
                }
                let function = match eval(&fs[1], env)? {
                    Value::Function(function) => function,
                    _ => return Err("map transform expects a function".into()),
                };
                let source = eval(&fs[2], env)?;
                if matches!(source, Value::Iterator(_)) {
                    iterator_seq(iterator_map(function, source)?)
                } else {
                    let values = iterator_values(source)?;
                    let mapped = values
                        .into_iter()
                        .map(|value| call_function(&function, vec![value]))
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(Value::Vector(mapped.into_iter().collect()))
                }
            }
            Form::Symbol(n) if ["zero?", "pos?", "neg?", "even?", "odd?"].contains(&n.as_str()) => {
                if fs.len() != 2 {
                    return Err(format!("{n} expects one number"));
                }
                let value = match eval(&fs[1], env)? {
                    Value::Number(value) => value,
                    _ => return Err(format!("{n} expects a number")),
                };
                let result = match n.as_str() {
                    "zero?" => value == 0,
                    "pos?" => value > 0,
                    "neg?" => value < 0,
                    "even?" => value % 2 == 0,
                    "odd?" => value % 2 != 0,
                    _ => false,
                };
                Ok(Value::Bool(result))
            }
            Form::Symbol(n) if ["nil?", "true?", "false?"].contains(&n.as_str()) => {
                if fs.len() != 2 {
                    return Err(format!("{n} expects one value"));
                }
                let value = eval(&fs[1], env)?;
                let result = match n.as_str() {
                    "nil?" => matches!(value, Value::Nil),
                    "true?" => matches!(value, Value::Bool(true)),
                    "false?" => matches!(value, Value::Bool(false)),
                    _ => false,
                };
                Ok(Value::Bool(result))
            }
            Form::Symbol(n) if ["every?", "any?"].contains(&n.as_str()) => {
                if fs.len() != 3 {
                    return Err(format!("{n} expects a predicate and collection"));
                }
                let predicate = eval(&fs[1], env)?;
                let values = iterator_values(eval(&fs[2], env)?)?;
                for value in values {
                    let result = match &predicate {
                        Value::Function(function) => call_function(function, vec![value])?,
                        _ => return Err(format!("{n} expects a function")),
                    };
                    if n == "every?" && !result.truthy() {
                        return Ok(Value::Bool(false));
                    }
                    if n == "any?" && result.truthy() {
                        return Ok(Value::Bool(true));
                    }
                }
                Ok(Value::Bool(n == "every?"))
            }
            Form::Symbol(n) if n == "constantly" => {
                if fs.len() != 2 {
                    return Err("constantly expects one value".into());
                }
                let value = eval(&fs[1], env)?;
                let mut captured = env.clone();
                captured.insert("__constant".into(), value);
                Ok(Value::Function(Rc::new(Function {
                    params: Vec::new(),
                    variadic: Some("_rest".into()),
                    body: vec![Form::Symbol("__constant".into())],
                    captured: Rc::new(RefCell::new(captured)),
                    name: None,
                    native: None,
                    clauses: Vec::new(),
                    is_macro: false,
                })))
            }
            Form::Symbol(n) if n == "complement" => {
                if fs.len() != 2 {
                    return Err("complement expects one function".into());
                }
                let predicate = eval(&fs[1], env)?;
                if !matches!(predicate, Value::Function(_)) {
                    return Err("complement expects a function".into());
                }
                Ok(generated_function(
                    vec!["value".into()],
                    vec![Form::List(vec![
                        Form::Symbol("not".into()),
                        Form::List(vec![
                            Form::Symbol("__predicate".into()),
                            Form::Symbol("value".into()),
                        ]),
                    ])],
                    env.clone(),
                    vec![("__predicate", predicate)],
                ))
            }
            Form::Symbol(n) if n == "comp" || n == "comp2" || n == "comp3" => {
                let arity = fs.len() - 1;
                let expected = match n.as_str() {
                    "comp2" => arity == 2,
                    "comp3" => arity == 3,
                    _ => arity == 2 || arity == 3,
                };
                if !expected {
                    let arities = if n == "comp" {
                        "2 or 3"
                    } else if n == "comp2" {
                        "2"
                    } else {
                        "3"
                    };
                    return Err(format!("{n} expects {arities} functions"));
                }
                let functions = fs[1..]
                    .iter()
                    .map(|form| eval(form, env))
                    .collect::<Result<Vec<_>, _>>()?;
                if functions
                    .iter()
                    .any(|value| !matches!(value, Value::Function(_)))
                {
                    return Err(format!("{n} expects functions"));
                }
                let body = if arity == 2 {
                    Form::List(vec![
                        Form::Symbol("__f".into()),
                        Form::List(vec![
                            Form::Symbol("__g".into()),
                            Form::Symbol("value".into()),
                        ]),
                    ])
                } else {
                    Form::List(vec![
                        Form::Symbol("__f".into()),
                        Form::List(vec![
                            Form::Symbol("__g".into()),
                            Form::List(vec![
                                Form::Symbol("__h".into()),
                                Form::Symbol("value".into()),
                            ]),
                        ]),
                    ])
                };
                let mut bindings =
                    vec![("__f", functions[0].clone()), ("__g", functions[1].clone())];
                if arity == 3 {
                    bindings.push(("__h", functions[2].clone()));
                }
                Ok(generated_function(
                    vec!["value".into()],
                    vec![body],
                    env.clone(),
                    bindings,
                ))
            }
            Form::Symbol(n) if n == "identity" => {
                if fs.len() != 2 {
                    return Err("identity expects one value".into());
                }
                eval(&fs[1], env)
            }
            Form::Symbol(n) if n == "apply" => {
                if fs.len() < 3 {
                    return Err("apply expects a function and arguments".into());
                }
                let builtin_name = match &fs[1] {
                    Form::Symbol(name) if ["+", "-", "*", "/"].contains(&name.as_str()) => {
                        Some(name.as_str())
                    }
                    _ => None,
                };
                let function = if builtin_name.is_none() {
                    Some(eval(&fs[1], env)?)
                } else {
                    None
                };
                let mut arguments = Vec::new();
                for form in &fs[2..fs.len() - 1] {
                    arguments.push(eval(form, env)?);
                }
                arguments.extend(iterator_values(eval(&fs[fs.len() - 1], env)?)?);
                match function {
                    Some(Value::Function(function)) => call_function(&function, arguments),
                    Some(_) => Err("apply expects a function".into()),
                    None => {
                        let name = builtin_name.unwrap();
                        let numbers = arguments
                            .iter()
                            .map(|value| match value {
                                Value::Number(value) => Ok(*value),
                                _ => Err("apply arithmetic expects numbers".into()),
                            })
                            .collect::<Result<Vec<_>, String>>()?;
                        if numbers.is_empty() {
                            return Err("apply expects a function".into());
                        }
                        let result = numbers[1..].iter().try_fold(numbers[0], |a, b| {
                            match name {
                                "+" => a.checked_add(*b),
                                "-" => a.checked_sub(*b),
                                "*" => a.checked_mul(*b),
                                "/" if *b == 0 => return Err("division by zero".into()),
                                "/" => a.checked_div(*b),
                                _ => return Err("apply expects a function".into()),
                            }
                            .ok_or_else(|| "integer overflow".to_string())
                        })?;
                        Ok(Value::Number(result))
                    }
                }
            }
            Form::Symbol(n) if n == "key" || n == "val" => {
                if fs.len() != 2 {
                    return Err(format!("{n} expects an entry"));
                }
                let entry = eval(&fs[1], env)?;
                match pair_parts(&entry) {
                    Some((key, value)) => Ok(if n == "key" { key } else { value }),
                    None => Err(format!("{n} expects a pair")),
                }
            }
            Form::Symbol(n) if n == "reverse" => {
                if fs.len() != 2 {
                    return Err("reverse expects one collection".into());
                }
                let mut values = iterator_values(eval(&fs[1], env)?)?;
                values.reverse();
                Ok(Value::List(values.into_iter().collect()))
            }
            Form::Symbol(n) if n == "keys" => {
                if fs.len() != 2 {
                    return Err("keys expects one collection".into());
                }
                collection_keys(&eval(&fs[1], env)?)
            }
            Form::Symbol(n) if n == "vals" => {
                if fs.len() != 2 {
                    return Err("vals expects one collection".into());
                }
                collection_vals(&eval(&fs[1], env)?)
            }
            Form::Symbol(n) if n == "not" => {
                if fs.len() != 2 {
                    return Err("not expects one argument".into());
                }
                Ok(Value::Bool(!eval(&fs[1], env)?.truthy()))
            }
            Form::Symbol(n) if ["<", ">", "<=", ">="].contains(&n.as_str()) => {
                comparison(n, &fs[1..], env)
            }
            Form::Symbol(n) if n == "first" => {
                if fs.len() != 2 {
                    return Err("first expects one argument".into());
                }
                collection_first(eval(&fs[1], env)?)
            }
            Form::Symbol(n) if n == "second" => {
                if fs.len() != 2 {
                    return Err("second expects one argument".into());
                }
                collection_second(eval(&fs[1], env)?)
            }
            Form::Symbol(n) if n == "rest" => {
                if fs.len() != 2 {
                    return Err("rest expects one argument".into());
                }
                collection_rest(eval(&fs[1], env)?)
            }
            Form::Symbol(n) if n == "last" => {
                if fs.len() != 2 {
                    return Err("last expects one argument".into());
                }
                collection_last(eval(&fs[1], env)?)
            }
            Form::Symbol(n) if n == "empty?" => {
                if fs.len() != 2 {
                    return Err("empty? expects one argument".into());
                }
                collection_empty(eval(&fs[1], env)?)
            }
            Form::Symbol(n) if n == "empty" => {
                if fs.len() != 2 {
                    return Err("empty expects one argument".into());
                }
                collection_empty_value(eval(&fs[1], env)?)
            }
            Form::Symbol(n)
                if [
                    "list?", "vector?", "map?", "set?", "keyword?", "symbol?", "string?",
                ]
                .contains(&n.as_str()) =>
            {
                if fs.len() != 2 {
                    return Err(format!("{n} expects one argument"));
                }
                let value = eval(&fs[1], env)?;
                Ok(Value::Bool(match n.as_str() {
                    "list?" => matches!(value, Value::List(_) | Value::Cons(_) | Value::Queue(_)),
                    "vector?" => matches!(value, Value::Vector(_) | Value::Tuple(_)),
                    "map?" => matches!(
                        value,
                        Value::Map(_) | Value::OrderedMap(_) | Value::SortedMap(_) | Value::Trie(_)
                    ),
                    "set?" => matches!(
                        value,
                        Value::Set(_) | Value::OrderedSet(_) | Value::SortedSet(_)
                    ),
                    "keyword?" => matches!(value, Value::Keyword(_)),
                    "symbol?" => matches!(value, Value::Symbol(_)),
                    "string?" => matches!(value, Value::String(_)),
                    _ => unreachable!(),
                }))
            }
            Form::Symbol(n) if n == "not-empty" => {
                if fs.len() != 2 {
                    return Err("not-empty expects one argument".into());
                }
                let value = eval(&fs[1], env)?;
                Ok(if collection_empty(value.clone())?.truthy() {
                    Value::Nil
                } else {
                    value
                })
            }
            Form::Symbol(n) if n == "count" => {
                if fs.len() != 2 {
                    return Err("count expects one argument".into());
                }
                collection_count(&eval(&fs[1], env)?)
            }
            Form::Symbol(n) if n == "get" => {
                if fs.len() != 3 && fs.len() != 4 {
                    return Err("get expects 2 or 3 arguments".into());
                }
                let value = eval(&fs[1], env)?;
                let key = eval(&fs[2], env)?;
                let default = if fs.len() == 4 {
                    eval(&fs[3], env)?
                } else {
                    Value::Nil
                };
                collection_get(&value, &key, default)
            }
            Form::Symbol(n) if n == "nth" => {
                if fs.len() != 3 {
                    return Err("nth expects two arguments".into());
                }
                collection_nth(&eval(&fs[1], env)?, &eval(&fs[2], env)?)
            }
            Form::Symbol(n) if n == "assoc" => {
                if fs.len() < 4 || fs.len() % 2 != 0 {
                    return Err("assoc expects a collection and key/value pairs".into());
                }
                let mut value = eval(&fs[1], env)?;
                for pair in fs[2..].chunks(2) {
                    let key = eval(&pair[0], env)?;
                    let replacement = eval(&pair[1], env)?;
                    value = collection_assoc(&value, &key, replacement)?;
                }
                Ok(value)
            }
            Form::Symbol(n) if n == "dissoc" => {
                if fs.len() < 3 {
                    return Err("dissoc expects a map and at least one key".into());
                }
                let value = eval(&fs[1], env)?;
                let keys = fs[2..]
                    .iter()
                    .map(|form| eval(form, env))
                    .collect::<Result<Vec<_>, _>>()?;
                collection_dissoc(&value, &keys)
            }
            Form::Symbol(n) if n == "get-in" => {
                if fs.len() != 3 {
                    return Err("get-in expects a collection and keys".into());
                }
                let value = eval(&fs[1], env)?;
                let keys = iterator_values(eval(&fs[2], env)?)?;
                collection_get_in(value, &keys)
            }
            Form::Symbol(n) if n == "assoc-in" => {
                if fs.len() != 4 {
                    return Err("assoc-in expects a collection, keys, and value".into());
                }
                let value = eval(&fs[1], env)?;
                let keys = iterator_values(eval(&fs[2], env)?)?;
                let replacement = eval(&fs[3], env)?;
                collection_assoc_in(value, &keys, replacement)
            }
            Form::Symbol(n) if n == "update" || n == "update-in" => {
                if (n == "update" && fs.len() < 4) || (n == "update-in" && fs.len() < 4) {
                    return Err(format!("{n} expects a collection, key path, and function"));
                }
                let value = eval(&fs[1], env)?;
                let (keys, function_form, extra_forms) = if n == "update" {
                    (vec![eval(&fs[2], env)?], &fs[3], &fs[4..])
                } else {
                    (iterator_values(eval(&fs[2], env)?)?, &fs[3], &fs[4..])
                };
                let current = collection_get_in(value.clone(), &keys)?;
                let function = eval(function_form, env)?;
                let mut args = vec![current];
                args.extend(
                    extra_forms
                        .iter()
                        .map(|form| eval(form, env))
                        .collect::<Result<Vec<_>, _>>()?,
                );
                let replacement = match function {
                    Value::Function(function) => call_function(&function, args)?,
                    _ => return Err(format!("{n} expects a function")),
                };
                if n == "update" {
                    collection_assoc(&value, &keys[0], replacement)
                } else {
                    collection_assoc_in(value, &keys, replacement)
                }
            }
            Form::Symbol(n) if n == "conj" => {
                if fs.len() != 3 {
                    return Err("conj expects two arguments".into());
                }
                let collection = eval(&fs[1], env)?;
                let item = eval(&fs[2], env)?;
                protocol_conj(&[collection, item])
            }
            Form::Symbol(n) if n == "cons" => {
                if fs.len() != 3 {
                    return Err("cons expects two arguments".into());
                }
                let item = eval(&fs[1], env)?;
                let collection = eval(&fs[2], env)?;
                match collection {
                    Value::Cons(values) => Ok(Value::Cons(Box::new(
                        PCons::new(item, values.iter().collect()).with_meta(values.meta().cloned()),
                    ))),
                    Value::Tuple(values) => Ok(Value::Cons(Box::new(PCons::new(
                        item,
                        values.iter().cloned().collect(),
                    )))),
                    Value::Vector(values) => Ok(Value::Cons(Box::new(PCons::new(
                        item,
                        values.iter().cloned().collect(),
                    )))),
                    Value::List(values) => Ok(Value::Cons(Box::new(PCons::new(item, values)))),
                    Value::Nil => Ok(Value::Cons(Box::new(PCons::new(item, PList::new())))),
                    _ => Err("cons expects a sequential collection".into()),
                }
            }
            Form::Symbol(n) if n == "recur" => {
                if fs.len() < 2 {
                    return Err("recur expects values".into());
                }
                Ok(Value::Recur(
                    fs[1..]
                        .iter()
                        .map(|form| eval(form, env))
                        .collect::<Result<Vec<_>, _>>()?,
                ))
            }
            Form::Symbol(n) if n == "binding" => {
                if fs.len() < 3 {
                    return Err("binding expects bindings and a body".into());
                }
                let pairs = match &fs[1] {
                    Form::List(values) | Form::Vector(values) => values,
                    _ => return Err("binding expects a binding list or vector".into()),
                };
                if pairs.len() % 2 != 0 {
                    return Err("binding bindings require name/value pairs".into());
                }
                let mut pending = Vec::new();
                for pair in pairs.chunks(2) {
                    let name = match &pair[0] {
                        Form::Symbol(name) => name,
                        _ => return Err("binding name must be a symbol".into()),
                    };
                    let var = match env.get(name) {
                        Some(Value::Var(var)) => var.clone(),
                        _ => return Err(format!("binding expects a Var: {name}")),
                    };
                    if !var.is_dynamic() {
                        return Err(format!("binding expects a dynamic Var: {name}"));
                    }
                    let value = eval(&pair[1], env)?;
                    pending.push((var, value));
                }
                for (var, value) in &pending {
                    var.bind(value.clone());
                }
                let bound = pending.into_iter().map(|(var, _)| var).collect::<Vec<_>>();
                let mut result = Ok(Value::Nil);
                for form in &fs[2..] {
                    result = eval(form, env);
                    if result.is_err() {
                        break;
                    }
                }
                for var in bound.into_iter().rev() {
                    if let Err(error) = var.unbind() {
                        if result.is_ok() {
                            result = Err(error);
                        }
                    }
                }
                result
            }
            Form::Symbol(n) if n == "loop" => {
                if fs.len() != 3 {
                    return Err("loop expects bindings and a body".into());
                }
                let bindings = match &fs[1] {
                    Form::List(values) | Form::Vector(values) => values,
                    _ => return Err("loop expects a binding list or vector".into()),
                };
                if bindings.len() % 2 != 0 {
                    return Err("loop bindings require name/value pairs".into());
                }
                let mut names = Vec::new();
                let mut previous = Vec::new();
                for pair in bindings.chunks(2) {
                    let name = match &pair[0] {
                        Form::Symbol(name) => name.clone(),
                        _ => return Err("loop binding name must be a symbol".into()),
                    };
                    let value = eval(&pair[1], env)?;
                    names.push(name.clone());
                    previous.push((name.clone(), env.insert(name, value)));
                }
                let result = loop {
                    match eval(&fs[2], env)? {
                        Value::Recur(values) => {
                            if values.len() != names.len() {
                                break Err("loop recur arity mismatch".into());
                            }
                            for (name, value) in names.iter().cloned().zip(values) {
                                env.insert(name, value);
                            }
                        }
                        result => break Ok(result),
                    }
                };
                for (name, old) in previous.into_iter().rev() {
                    if let Some(old) = old {
                        env.insert(name, old);
                    } else {
                        env.remove(&name);
                    }
                }
                result
            }
            Form::Symbol(n) if n == "if" => {
                if fs.len() != 3 && fs.len() != 4 {
                    return Err("if expects 2 or 3 arguments".into());
                }
                if eval(&fs[1], env)?.truthy() {
                    eval(&fs[2], env)
                } else if fs.len() == 4 {
                    eval(&fs[3], env)
                } else {
                    Ok(Value::Nil)
                }
            }
            Form::Symbol(n) if n == "cond" => {
                if fs.len() % 2 == 0 {
                    return Err("cond expects test/expression pairs".into());
                }
                let mut clauses = fs[1..].chunks_exact(2);
                for clause in &mut clauses {
                    if eval(&clause[0], env)?.truthy() {
                        return eval(&clause[1], env);
                    }
                }
                Ok(Value::Nil)
            }
            Form::Symbol(n) if n == "let" => {
                if fs.len() != 3 {
                    return Err("let expects bindings and a body".into());
                }
                let bindings = match &fs[1] {
                    Form::List(values) | Form::Vector(values) => values,
                    _ => return Err("let expects a binding list or vector".into()),
                };
                if bindings.len() % 2 != 0 {
                    return Err("let bindings require name/value pairs".into());
                }
                let mut previous = Vec::new();
                for pair in bindings.chunks(2) {
                    let name = match &pair[0] {
                        Form::Symbol(name) => name.clone(),
                        _ => return Err("let binding name must be a symbol".into()),
                    };
                    let value = eval(&pair[1], env)?;
                    previous.push((name.clone(), env.insert(name, value)));
                }
                let result = eval(&fs[2], env);
                for (name, old) in previous.into_iter().rev() {
                    if let Some(old) = old {
                        env.insert(name, old);
                    } else {
                        env.remove(&name);
                    }
                }
                result
            }
            Form::Symbol(n) if ["+", "-", "*", "/", "%", "mod"].contains(&n.as_str()) => {
                arithmetic(if n == "mod" { "%" } else { n }, &fs[1..], env)
            }
            _ => {
                if let Form::Symbol(name) = &fs[0] {
                    if let Some(expanded) = macroexpand_call(name, fs, env)? {
                        return eval(&expanded, env);
                    }
                }
                let function = eval(&fs[0], env)?;
                let arguments = fs[1..]
                    .iter()
                    .map(|form| eval(form, env))
                    .collect::<Result<Vec<_>, _>>()?;
                call_value(function, arguments)
            }
        },
    }
}

pub fn eval_traced(form: &Form, env: &mut HashMap<String, Value>) -> Result<Value, String> {
    let _guard = TraceGuard::enable();
    eval(form, env).map_err(append_trace)
}

pub fn eval_text(source: &str, env: &mut HashMap<String, Value>) -> Result<String, String> {
    Ok(eval_value_text(source, env)?.display())
}

pub fn eval_text_traced(source: &str, env: &mut HashMap<String, Value>) -> Result<String, String> {
    let _guard = TraceGuard::enable();
    eval_text(source, env).map_err(append_trace)
}

pub fn eval_value_text_traced(
    source: &str,
    env: &mut HashMap<String, Value>,
) -> Result<Value, String> {
    let _guard = TraceGuard::enable();
    eval_value_text(source, env).map_err(append_trace)
}

pub fn eval_value_text(source: &str, env: &mut HashMap<String, Value>) -> Result<Value, String> {
    let forms = parse_forms(source)?;
    let mut result = Value::Nil;
    for form in forms {
        result = eval(&form, env)?;
        if matches!(result, Value::Recur(_)) {
            return Err("recur must be inside loop".into());
        }
    }
    Ok(result)
}
