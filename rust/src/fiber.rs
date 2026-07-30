use super::*;
use crate::lang::data::{OrderedMap, OrderedSet};

#[path = "fiber/coroutine.rs"]
mod coroutine;
#[cfg(test)]
#[path = "fiber/coroutine_tests.rs"]
mod coroutine_tests;

/// Core special forms that must be routed through the synchronous `eval` path
/// because they need unevaluated arguments, structural handling, or namespace
/// side effects. Forms with dedicated CPS arms in `list` are listed here too
/// so that they do not accidentally reach `application`, but the dedicated arms
/// take precedence.
const SYNC_SPECIAL_FORMS: &[&str] = &[
    ".",
    "alter-var-root",
    "apply",
    "binding",
    "declare",
    "def",
    "defstruct",
    "defprotocol",
    "defmulti",
    "defmethod",
    "defmacro",
    "defn",
    "defn-",
    "do",
    "deref",
    "extend-type",
    "field",
    "eval",
    "fn",
    "fn*",
    "if",
    "intern-var",
    "instance?",
    "let",
    "loop",
    "macroexpand-1",
    "meta",
    "ns",
    "recur",
    "require",
    "set!",
    "syntax-quote",
    "throw",
    "type",
    "try",
    "var",
    "var/set",
    "with-meta",
];

/// All names that `core::eval` handles as special forms. Used in `application`
/// to ensure runtime builtins take precedence over foundation redefinitions
/// without forcing the whole call through the synchronous path (which would
/// break nested `deref`/`yield`/`await`).
const CORE_SPECIAL_FORMS: &[&str] = &[
    "=",
    "+",
    "-",
    "*",
    "/",
    "%",
    "mod",
    "<",
    ">",
    "<=",
    ">=",
    ".",
    "abs",
    "acos",
    "acosh",
    "alter-var-root",
    "any?",
    "apply",
    "array",
    "asin",
    "asinh",
    "assoc",
    "assoc-in",
    "atan",
    "atan2",
    "atanh",
    "binding",
    "bit-and",
    "bit-or",
    "bit-xor",
    "bit-not",
    "bit-shift-left",
    "bit-shift-right",
    "bytes",
    "bytes/copy",
    "bytes/count",
    "bytes/get",
    "bytes/set",
    "bytes/s8",
    "bytes/slice",
    "bytes/u8",
    "ceil",
    "comp",
    "comp2",
    "comp3",
    "complement",
    "concat",
    "conj",
    "cons",
    "constantly",
    "cos",
    "cosh",
    "count",
    "cycle",
    "declare",
    "def",
    "defmacro",
    "defmethod",
    "defmulti",
    "defn",
    "defn-",
    "dissoc",
    "deref",
    "do",
    "drop",
    "drop-while",
    "empty",
    "empty?",
    "eval",
    "even?",
    "every?",
    "exp",
    "false?",
    "file/read",
    "file/resolve",
    "file/write",
    "file/exists?",
    "file/list",
    "file/mkdir",
    "file/delete",
    "filter",
    "first",
    "floor",
    "fn",
    "fn*",
    "get",
    "get-in",
    "identity",
    "if",
    "interleave",
    "interpose",
    "interpose",
    "iter",
    "iter-close",
    "iter-cycle",
    "iter-drop",
    "iter-drop-while",
    "iter-every?",
    "iter-any?",
    "iter-finite?",
    "iter-has?",
    "iter-interleave",
    "iter-interpose",
    "iter-iterate",
    "iter-keep",
    "iter-map",
    "iter-mapcat",
    "iter-materialize",
    "iter-next",
    "iter-partition-pair",
    "iter-partition",
    "iter-partition-all",
    "iter-range",
    "iter-repeatedly",
    "iter-constantly",
    "iter-filter",
    "iter-take",
    "iter-take-while",
    "iter-zip",
    "iter?",
    "keep",
    "key",
    "keys",
    "keyword",
    "last",
    "let",
    "list",
    "list?",
    "loop",
    "map",
    "map?",
    "mapcat",
    "keep",
    "mod",
    "neg?",
    "nil?",
    "ns",
    "nth",
    "not",
    "not-empty",
    "object",
    "odd?",
    "pair",
    "partition-pair",
    "partition",
    "partition-all",
    "pointer",
    "pos?",
    "pow",
    "promise",
    "promise/run",
    "promise?",
    "promise/all",
    "promise/cancel",
    "promise/delay",
    "promise/from",
    "promise/new",
    "range",
    "recur",
    "repeat",
    "repeatedly",
    "require",
    "rest",
    "reverse",
    "second",
    "seq",
    "seq?",
    "set",
    "set!",
    "set?",
    "sin",
    "sinh",
    "socket/close",
    "socket/connect",
    "socket/send",
    "str",
    "str/decode",
    "str/decode-utf8",
    "str/encode",
    "str/encode-utf8",
    "str/length",
    "str/blank?",
    "str/includes?",
    "str/starts-with?",
    "str/ends-with?",
    "str/char-at",
    "str/slice",
    "str/index-of",
    "str/last-index-of",
    "str/split",
    "str/split-lines",
    "str/join",
    "str/repeat",
    "str/replace",
    "str/replace-first",
    "str/trim",
    "str/trim-left",
    "str/trim-right",
    "str/upper",
    "str/lower",
    "str/capitalize",
    "str/decapitalize",
    "str/pad-left",
    "str/pad-right",
    "str/reverse",
    "sqrt",
    "symbol",
    "take",
    "take-while",
    "tan",
    "tanh",
    "throw",
    "true?",
    "try",
    "tup",
    "update",
    "update-in",
    "val",
    "vals",
    "var",
    "var/set",
    "vec",
    "vector",
    "vector?",
    "zero?",
    "zip",
    "__map-transform",
    "__iterator-transform",
];

pub(crate) fn completion_symbols() -> &'static [&'static str] {
    CORE_SPECIAL_FORMS
}

type Cont = Box<dyn FnOnce(Result<Value, String>) -> Step>;
pub type Resume = Box<dyn FnOnce(PromiseState) -> Step>;
pub enum Step {
    Done(Result<Value, String>),
    Wait(Promise, Resume),
    Yield(Value, Box<dyn FnOnce(Value) -> Step>),
    /// Defers the next synchronous continuation to the fiber driver.  Without
    /// this trampoline, a document with many top-level forms keeps one Rust
    /// stack frame per completed form (and can exhaust the smaller WASM stack).
    Continue(Box<dyn FnOnce() -> Step>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum EvalFiberState {
    Running,
    Suspended,
    Completed(Value),
    Failed(String),
    Cancelled,
}

pub struct EvalFiber {
    env: Rc<RefCell<HashMap<String, Value>>>,
    pending: Option<Promise>,
    resume: Option<Resume>,
    state: EvalFiberState,
}
impl EvalFiber {
    pub fn start(source: &str, env: HashMap<String, Value>) -> Result<Self, String> {
        let forms = parse_forms(source)?;
        Self::start_forms(forms, env)
    }
    pub fn start_forms(forms: Vec<Form>, env: HashMap<String, Value>) -> Result<Self, String> {
        let env = Rc::new(RefCell::new(env));
        let step = forms_cps(
            Rc::new(forms),
            0,
            Value::Nil,
            env.clone(),
            Box::new(Step::Done),
        );
        let mut fiber = Self {
            env,
            pending: None,
            resume: None,
            state: EvalFiberState::Running,
        };
        fiber.accept(step);
        Ok(fiber)
    }
    pub fn state(&self) -> EvalFiberState {
        self.state.clone()
    }
    pub fn pending(&self) -> Option<Promise> {
        self.pending.clone()
    }
    pub fn environment(&self) -> HashMap<String, Value> {
        self.env.borrow().clone()
    }
    pub fn resume(&mut self, state: PromiseState) -> EvalFiberState {
        if !matches!(self.state, EvalFiberState::Suspended) {
            return self.state();
        }
        let Some(resume) = self.resume.take() else {
            self.state = EvalFiberState::Failed("fiber continuation missing".into());
            return self.state();
        };
        self.pending = None;
        self.state = EvalFiberState::Running;
        let step = resume(state);
        self.accept(step);
        self.state()
    }
    pub fn cancel(&mut self) -> bool {
        if matches!(
            self.state,
            EvalFiberState::Completed(_) | EvalFiberState::Failed(_) | EvalFiberState::Cancelled
        ) {
            return false;
        }
        self.pending = None;
        self.resume = None;
        self.state = EvalFiberState::Cancelled;
        true
    }
    pub fn drive_sync(&mut self) -> Result<Value, String> {
        loop {
            match self.state() {
                EvalFiberState::Completed(v) => return Ok(v),
                EvalFiberState::Failed(e) => return Err(e),
                EvalFiberState::Cancelled => return Err("eval cancelled".into()),
                EvalFiberState::Running => return Err("fiber is running".into()),
                EvalFiberState::Suspended => {
                    let Some(pending) = self.pending() else {
                        return Err("fiber suspended without promise".into());
                    };
                    match pending.state() {
                        PromiseState::Fulfilled(v) => {
                            self.resume(PromiseState::Fulfilled(v));
                        }
                        PromiseState::Rejected(e) => {
                            self.resume(PromiseState::Rejected(e));
                        }
                        PromiseState::Pending => {
                            return Err(
                                "deref cannot block on a pending promise outside an HTA fiber"
                                    .into(),
                            );
                        }
                    }
                }
            }
        }
    }
    fn accept(&mut self, mut step: Step) {
        loop {
            match step {
                Step::Continue(next) => step = next(),
                Step::Done(Ok(v)) => {
                    self.state = EvalFiberState::Completed(v);
                    return;
                }
                Step::Done(Err(e)) => {
                    self.state = EvalFiberState::Failed(e);
                    return;
                }
                Step::Wait(p, r) => {
                    self.pending = Some(p);
                    self.resume = Some(r);
                    self.state = EvalFiberState::Suspended;
                    return;
                }
                Step::Yield(_, _) => {
                    self.state = EvalFiberState::Failed(
                        "coroutine/yield used outside of a coroutine".into(),
                    );
                    return;
                }
            }
        }
    }
}

fn forms_cps(
    forms: Rc<Vec<Form>>,
    i: usize,
    last: Value,
    env: Rc<RefCell<HashMap<String, Value>>>,
    k: Cont,
) -> Step {
    if i == forms.len() || matches!(last, Value::Recur(_)) {
        return k(Ok(last));
    }
    let next = forms.clone();
    let e = env.clone();
    one(
        forms[i].clone(),
        env,
        Box::new(move |r| match r {
            Ok(v) => Step::Continue(Box::new(move || forms_cps(next, i + 1, v, e, k))),
            Err(x) => k(Err(x)),
        }),
    )
}
fn values_cps(
    forms: Rc<Vec<Form>>,
    i: usize,
    values: Vec<Value>,
    env: Rc<RefCell<HashMap<String, Value>>>,
    k: Box<dyn FnOnce(Result<Vec<Value>, String>) -> Step>,
) -> Step {
    if i == forms.len() {
        return k(Ok(values));
    }
    let next = forms.clone();
    let e = env.clone();
    one(
        forms[i].clone(),
        env,
        Box::new(move |r| match r {
            Ok(v) => {
                let mut values = values;
                values.push(v);
                Step::Continue(Box::new(move || values_cps(next, i + 1, values, e, k)))
            }
            Err(x) => k(Err(x)),
        }),
    )
}
fn one(form: Form, env: Rc<RefCell<HashMap<String, Value>>>, k: Cont) -> Step {
    match form {
        Form::Map(entries) => {
            let flat = Rc::new(entries.into_iter().flat_map(|(a, b)| [a, b]).collect());
            values_cps(
                flat,
                0,
                Vec::new(),
                env,
                Box::new(move |r| {
                    k(r.map(|v| {
                        Value::OrderedMap(Box::new(
                            v.chunks_exact(2)
                                .map(|p| (p[0].clone(), p[1].clone()))
                                .collect::<OrderedMap<Value, Value>>(),
                        ))
                    }))
                }),
            )
        }
        Form::Set(v) => values_cps(
            Rc::new(v),
            0,
            Vec::new(),
            env,
            Box::new(move |r| {
                k(r.map(|v| {
                    Value::OrderedSet(Box::new(
                        unique_values(v).into_iter().collect::<OrderedSet<Value>>(),
                    ))
                }))
            }),
        ),
        Form::Vector(v) => values_cps(
            Rc::new(v),
            0,
            Vec::new(),
            env,
            Box::new(move |r| k(r.and_then(vector_literal))),
        ),
        Form::List(v) if v.is_empty() => k(Ok(Value::Nil)),
        Form::List(v) if v.len() == 2 && matches!(&v[0],Form::Symbol(n)if n=="quote") => {
            k(literal_value(&v[1]))
        }
        Form::List(v) => list(v, env, k),
        simple => sync(simple, env, k),
    }
}
fn sync(form: Form, env: Rc<RefCell<HashMap<String, Value>>>, k: Cont) -> Step {
    let result = {
        let mut borrowed = env.borrow_mut();
        eval(&form, &mut borrowed)
    };
    k(result)
}
fn list(v: Vec<Form>, env: Rc<RefCell<HashMap<String, Value>>>, k: Cont) -> Step {
    let head = match &v[0] {
        Form::Symbol(n) => Some(n.as_str()),
        _ => None,
    };
    match head {
        Some("deref") => {
            if v.len() != 2 {
                return k(Err("deref expects a var".into()));
            }
            if let Form::Symbol(name) = &v[1] {
                let target = {
                    let env = env.borrow();
                    match env.get(name) {
                        Some(Value::Var(cell)) => Some(Value::Var(cell.clone())),
                        Some(Value::Atom(cell)) => Some(Value::Atom(cell.clone())),
                        Some(Value::Promise(p)) => Some(Value::Promise(p.clone())),
                        _ => None,
                    }
                };
                if let Some(target) = target {
                    return match target {
                        Value::Var(x) => k(Ok(x.deref_value())),
                        Value::Atom(x) => k(Ok(x.deref_value())),
                        Value::Promise(p) => match p.state() {
                            PromiseState::Fulfilled(x) => k(Ok(x)),
                            PromiseState::Rejected(e) => {
                                k(Err(crate::core::promise_rejection_error(e)))
                            }
                            PromiseState::Pending => Step::Wait(
                                p,
                                Box::new(move |s| match s {
                                    PromiseState::Fulfilled(x) => k(Ok(x)),
                                    PromiseState::Rejected(e) => {
                                        k(Err(crate::core::promise_rejection_error(e)))
                                    }
                                    PromiseState::Pending => k(Err("fiber resumed pending".into())),
                                }),
                            ),
                        },
                        _ => unreachable!(),
                    };
                }
            }
            one(
                v[1].clone(),
                env,
                Box::new(move |r| match r {
                    Ok(Value::Var(x)) => k(Ok(x.deref_value())),
                    Ok(Value::Atom(x)) => k(Ok(x.deref_value())),
                    Ok(Value::Promise(p)) => match p.state() {
                        PromiseState::Fulfilled(x) => k(Ok(x)),
                        PromiseState::Rejected(e) => {
                            k(Err(crate::core::promise_rejection_error(e)))
                        }
                        PromiseState::Pending => Step::Wait(
                            p,
                            Box::new(move |s| match s {
                                PromiseState::Fulfilled(x) => k(Ok(x)),
                                PromiseState::Rejected(e) => {
                                    k(Err(crate::core::promise_rejection_error(e)))
                                }
                                PromiseState::Pending => k(Err("fiber resumed pending".into())),
                            }),
                        ),
                    },
                    Ok(_) => k(Err("deref expects a var or promise".into())),
                    Err(e) => k(Err(e)),
                }),
            )
        }
        Some("do") => forms_cps(Rc::new(v[1..].to_vec()), 0, Value::Nil, env, k),
        Some("if") => {
            if v.len() != 3 && v.len() != 4 {
                return k(Err("if expects 2 or 3 arguments".into()));
            }
            let vv = v.clone();
            let e = env.clone();
            one(
                v[1].clone(),
                env,
                Box::new(move |r| match r {
                    Ok(x) if x.truthy() => one(vv[2].clone(), e, k),
                    Ok(_) if vv.len() == 4 => one(vv[3].clone(), e, k),
                    Ok(_) => k(Ok(Value::Nil)),
                    Err(x) => k(Err(x)),
                }),
            )
        }
        Some("let") => scoped(v, env, k, false),
        Some("loop") => scoped(v, env, k, true),
        Some("recur") => values_cps(
            Rc::new(v[1..].to_vec()),
            0,
            Vec::new(),
            env,
            Box::new(move |r| k(r.map(Value::Recur))),
        ),
        Some("try") => try_cps(v, env, k),
        Some("throw") => {
            if v.len() != 2 {
                return k(Err("throw expects one value".into()));
            }
            one(
                v[1].clone(),
                env,
                Box::new(move |r| match r {
                    Ok(x) => k(Err(thrown_error(x))),
                    Err(x) => k(Err(x)),
                }),
            )
        }
        Some("std.foundation.coroutine/create") => coroutine::create_form(v, env, k),
        Some("std.foundation.coroutine/coroutine?") => coroutine::predicate_form(v, env, k),
        Some("std.foundation.coroutine/status") => coroutine::status_form(v, env, k),
        Some("std.foundation.coroutine/close") => coroutine::close_form(v, env, k),
        Some("std.foundation.coroutine/resume") => coroutine::resume_form(v, env, k),
        Some("std.protocol.icoroutine/resume") => coroutine::resume_protocol_form(v, env, k),
        Some("std.foundation.coroutine/yield") => coroutine::yield_form(v, env, k),
        Some("std.foundation.coroutine/await") => coroutine::await_form(v, env, k),
        Some("def") | Some("set!") | Some("var/set") => bind_form(v, env, k),
        Some(name) if SYNC_SPECIAL_FORMS.contains(&name) => sync(Form::List(v), env, k),
        _ => application(v, env, k),
    }
}

type Previous = Vec<(String, Option<Value>)>;
fn bindings(forms: &[Form], op: &str) -> Result<Vec<Form>, String> {
    let v = match forms.get(1) {
        Some(Form::List(v)) | Some(Form::Vector(v)) => v.clone(),
        _ => return Err(format!("{op} expects bindings")),
    };
    if v.len() % 2 != 0 {
        return Err(format!("{op} bindings require name/value pairs"));
    }
    Ok(v)
}
fn bind_values(
    v: Rc<Vec<Form>>,
    i: usize,
    old: Previous,
    env: Rc<RefCell<HashMap<String, Value>>>,
    k: Box<dyn FnOnce(Result<Previous, String>, Rc<RefCell<HashMap<String, Value>>>) -> Step>,
) -> Step {
    if i == v.len() {
        return k(Ok(old), env);
    }
    let pattern = v[i].clone();
    let vv = v.clone();
    let e = env.clone();
    one(
        v[i + 1].clone(),
        env,
        Box::new(move |r| match r {
            Ok(x) => {
                let mut old = old;
                let before = e.borrow().clone();
                let mut names = Vec::new();
                let binding = {
                    let mut environment = e.borrow_mut();
                    crate::core::bind_pattern(&pattern, x, &mut environment, &mut names, None)
                };
                if let Err(error) = binding {
                    return k(Err(format!("destructuring failed: {error}")), e);
                }
                for name in names {
                    old.push((name.clone(), before.get(&name).cloned()));
                }
                bind_values(vv, i + 2, old, e, k)
            }
            Err(x) => k(Err(x), e),
        }),
    )
}
fn restore(env: &mut HashMap<String, Value>, old: Previous) {
    for (n, v) in old.into_iter().rev() {
        if let Some(v) = v {
            env.insert(n, v);
        } else {
            env.remove(&n);
        }
    }
}
fn scoped(v: Vec<Form>, env: Rc<RefCell<HashMap<String, Value>>>, k: Cont, is_loop: bool) -> Step {
    if v.len() < 3 {
        return k(Err("binding form expects bindings and body".into()));
    }
    let b = match bindings(&v, if is_loop { "loop" } else { "let" }) {
        Ok(x) => x,
        Err(x) => return k(Err(x)),
    };
    let patterns = Rc::new(b.chunks(2).map(|pair| pair[0].clone()).collect());
    let body = if v.len() == 3 {
        v[2].clone()
    } else {
        Form::List(
            std::iter::once(Form::Symbol("do".into()))
                .chain(v[2..].iter().cloned())
                .collect(),
        )
    };
    bind_values(
        Rc::new(b),
        0,
        Vec::new(),
        env,
        Box::new(move |r, e| match r {
            Ok(old) if is_loop => loop_body(patterns, body, old, e, k),
            Ok(old) => {
                let re = e.clone();
                one(
                    body,
                    e,
                    Box::new(move |r| {
                        restore(&mut re.borrow_mut(), old);
                        k(r)
                    }),
                )
            }
            Err(x) => k(Err(x)),
        }),
    )
}
fn loop_body(
    patterns: Rc<Vec<Form>>,
    body: Form,
    old: Previous,
    env: Rc<RefCell<HashMap<String, Value>>>,
    k: Cont,
) -> Step {
    let pp = patterns.clone();
    let bb = body.clone();
    let oo = old.clone();
    let ee = env.clone();
    one(
        body,
        env,
        Box::new(move |r| match r {
            Ok(Value::Recur(v)) => {
                if v.len() != pp.len() {
                    restore(&mut ee.borrow_mut(), oo);
                    return k(Err("loop recur arity mismatch".into()));
                }
                for (pattern, value) in pp.iter().zip(v) {
                    let mut names = Vec::new();
                    if let Err(error) = crate::core::bind_pattern(
                        pattern,
                        value,
                        &mut ee.borrow_mut(),
                        &mut names,
                        None,
                    ) {
                        restore(&mut ee.borrow_mut(), oo);
                        return k(Err(format!("loop destructuring failed: {error}")));
                    }
                }
                loop_body(pp, bb, oo, ee, k)
            }
            r => {
                restore(&mut ee.borrow_mut(), oo);
                k(r)
            }
        }),
    )
}
fn bind_form(v: Vec<Form>, env: Rc<RefCell<HashMap<String, Value>>>, k: Cont) -> Step {
    if v.len() != 3 {
        return k(Err("binding form expects symbol and value".into()));
    }
    let op = match &v[0] {
        Form::Symbol(n) => n.clone(),
        _ => unreachable!(),
    };
    let (name, metadata) = match &v[1] {
        Form::Symbol(n) => (n.clone(), None),
        Form::Metadata(meta, value) => match value.as_ref() {
            Form::Symbol(n) => match crate::core::metadata_from_form(meta) {
                Ok(metadata) => (n.clone(), Some(metadata)),
                Err(error) => return k(Err(error)),
            },
            _ => return k(Err(format!("{op} name must be a symbol"))),
        },
        _ => return k(Err(format!("{op} name must be a symbol"))),
    };
    let e = env.clone();
    one(
        v[2].clone(),
        env,
        Box::new(move |r| match r {
            Ok(x) => {
                let mut env = e.borrow_mut();
                if op == "def" {
                    if let Some(protected) =
                        crate::core::protected_fallback_binding(&env, &name, metadata.clone())
                    {
                        drop(env);
                        return k(Ok(protected));
                    }
                    let origin = crate::core::definition_origin();
                    if let Some(Value::Var(var)) = env.get(&name) {
                        if crate::core::binding_is_local(var) {
                            var.reset_value(x.clone());
                            var.set_origin(origin);
                            if let Some(meta) = metadata {
                                var.set_hara_metadata(Some(meta));
                            }
                        } else {
                            let var = crate::kernel::Var::new(name.clone(), x.clone());
                            var.set_origin(origin);
                            if let Some(meta) = &metadata {
                                var.set_hara_metadata(Some(meta.clone()));
                            }
                            env.insert(name.clone(), Value::Var(var));
                        }
                    } else {
                        let var = crate::kernel::Var::new(name.clone(), x.clone());
                        var.set_origin(origin);
                        if let Some(meta) = &metadata {
                            var.set_hara_metadata(Some(meta.clone()));
                        }
                        env.insert(name.clone(), Value::Var(var));
                    }
                } else {
                    let Some(c) = binding_var(&mut env, &name) else {
                        return k(Err(format!("unbound var: {name}")));
                    };
                    c.reset_value(x.clone());
                    if let Some(meta) = metadata {
                        c.set_hara_metadata(Some(meta));
                    }
                }
                drop(env);
                k(Ok(x))
            }
            Err(x) => k(Err(x)),
        }),
    )
}

fn try_cps(v: Vec<Form>, env: Rc<RefCell<HashMap<String, Value>>>, k: Cont) -> Step {
    let mut body = Vec::new();
    let mut catches = Vec::new();
    let mut finals = Vec::new();
    let mut clauses_started = false;
    for f in v.into_iter().skip(1) {
        match &f {
            Form::List(p) if !p.is_empty() && matches!(&p[0],Form::Symbol(n)if n=="catch") => {
                clauses_started = true;
                catches.push(p.clone())
            }
            Form::List(p) if !p.is_empty() && matches!(&p[0],Form::Symbol(n)if n=="finally") => {
                clauses_started = true;
                finals.extend_from_slice(&p[1..])
            }
            _ if !clauses_started => body.push(f),
            _ => return k(Err("try clauses must follow body".into())),
        }
    }
    let e = env.clone();
    forms_cps(
        Rc::new(body),
        0,
        Value::Nil,
        env,
        Box::new(move |r| finish_try(r, catches, finals, e, k)),
    )
}
fn finish_try(
    r: Result<Value, String>,
    catches: Vec<Vec<Form>>,
    finals: Vec<Form>,
    env: Rc<RefCell<HashMap<String, Value>>>,
    k: Cont,
) -> Step {
    match r {
        Err(x) => {
            let Some(p) = catches.into_iter().find(|parts| match parts.as_slice() {
                [_, Form::Symbol(_), _] => crate::core::catch_matches(&x, "Exception"),
                [_, Form::Symbol(class), Form::Symbol(_), ..] => {
                    crate::core::catch_matches(&x, class)
                }
                _ => false,
            }) else {
                return finally(Err(x), finals, env, k);
            };
            let (binding_index, body_index) = match p.len() {
                3 => (1, 2),
                length if length >= 4 => {
                    if !matches!(&p[1], Form::Symbol(_)) {
                        return k(Err("catch class must be symbol".into()));
                    }
                    (2, 3)
                }
                _ => return k(Err("catch expects class, name, and body".into())),
            };
            let n = match &p[binding_index] {
                Form::Symbol(n) => n.clone(),
                _ => return k(Err("catch name must be symbol".into())),
            };
            let old = env.borrow_mut().insert(n.clone(), caught_error(&x));
            let e = env.clone();
            forms_cps(
                Rc::new(p[body_index..].to_vec()),
                0,
                Value::Nil,
                env,
                Box::new(move |r| {
                    restore(&mut e.borrow_mut(), vec![(n, old)]);
                    finally(r, finals, e, k)
                }),
            )
        }
        result => finally(result, finals, env, k),
    }
}
fn finally(
    result: Result<Value, String>,
    v: Vec<Form>,
    env: Rc<RefCell<HashMap<String, Value>>>,
    k: Cont,
) -> Step {
    forms_cps(
        Rc::new(v),
        0,
        Value::Nil,
        env,
        Box::new(move |r| match r {
            Err(x) => k(Err(x)),
            Ok(_) => k(result),
        }),
    )
}

thread_local! {static TEMP:Cell<u64>=const{Cell::new(0)};}
fn temp() -> String {
    TEMP.with(|x| {
        let n = x.get();
        x.set(n + 1);
        format!("__fiber_{n}")
    })
}
fn application(v: Vec<Form>, env: Rc<RefCell<HashMap<String, Value>>>, k: Cont) -> Step {
    if let Some(Form::Symbol(n)) = v.first() {
        if crate::core::resolve_macro(n).is_some() {
            let r = {
                let mut env = env.borrow_mut();
                eval(&Form::List(v), &mut env)
            };
            return k(r);
        }
    }
    let head_symbol = match &v[0] {
        Form::Symbol(n) => Some(n.as_str()),
        _ => None,
    };
    if let Some(name) = head_symbol {
        if CORE_SPECIAL_FORMS.contains(&name) || name.starts_with("std.native.") {
            return eval_special_form(v, env, k);
        }
    }
    let f = head_symbol.and_then(|n| binding_value(&env.borrow(), n));
    if let Some(Value::Function(f)) = f {
        return values_cps(
            Rc::new(v[1..].to_vec()),
            0,
            Vec::new(),
            env,
            Box::new(move |r| match r {
                Ok(a) => call(f, a, k),
                Err(x) => k(Err(x)),
            }),
        );
    }
    if head_symbol.is_none() {
        let forms = Rc::new(v[1..].to_vec());
        let arguments_env = env.clone();
        return one(
            v[0].clone(),
            env,
            Box::new(move |result| match result {
                Ok(Value::Function(function)) => values_cps(
                    forms,
                    0,
                    Vec::new(),
                    arguments_env,
                    Box::new(move |arguments| match arguments {
                        Ok(arguments) => call(function, arguments, k),
                        Err(error) => k(Err(error)),
                    }),
                ),
                Ok(value) => values_cps(
                    forms,
                    0,
                    Vec::new(),
                    arguments_env,
                    Box::new(move |arguments| match arguments {
                        Ok(arguments) => k(crate::core::call_value(value, arguments)),
                        Err(error) => k(Err(error)),
                    }),
                ),
                Err(error) => k(Err(error)),
            }),
        );
    }
    eval_special_form(v, env, k)
}
fn eval_special_form(v: Vec<Form>, env: Rc<RefCell<HashMap<String, Value>>>, k: Cont) -> Step {
    let op = v[0].clone();
    let e = env.clone();
    values_cps(
        Rc::new(v[1..].to_vec()),
        0,
        Vec::new(),
        env,
        Box::new(move |r| match r {
            Ok(values) => {
                let mut env = e.borrow_mut();
                let mut old = Vec::new();
                let mut list = vec![op];
                for x in values {
                    let n = temp();
                    let prior = env.insert(n.clone(), x);
                    old.push((n.clone(), prior));
                    list.push(Form::Symbol(n));
                }
                let r = eval(&Form::List(list), &mut env);
                restore(&mut env, old);
                drop(env);
                k(r)
            }
            Err(x) => k(Err(x)),
        }),
    )
}
fn call(f: Rc<Function>, args: Vec<Value>, k: Cont) -> Step {
    if !f.clauses.is_empty() {
        let Some(clause) = select_clause(&f.clauses, args.len()) else {
            let name = f.name.clone().unwrap_or_else(|| "<anonymous>".into());
            return k(Err(format!(
                "{name} has no arity accepting {} arguments",
                args.len()
            )));
        };
        return call(clause, args, k);
    }
    if f.native.is_some() {
        return k(crate::core::call_function(&f, args));
    }
    if f.variadic.is_none() && f.params.len() != args.len() {
        return k(Err(format!(
            "function expects {} arguments",
            f.params.len()
        )));
    }
    if args.len() < f.params.len() {
        return k(Err(format!(
            "function expects at least {} arguments",
            f.params.len()
        )));
    }
    let mut env = f.captured.borrow().clone();
    for (n, x) in f.params.iter().zip(args.iter()) {
        env.insert(n.clone(), x.clone());
    }
    if let Some(n) = &f.variadic {
        let skip = f.params.len();
        env.insert(
            n.clone(),
            Value::List(args.into_iter().skip(skip).collect()),
        );
    }
    forms_cps(
        Rc::new(f.body.clone()),
        0,
        Value::Nil,
        Rc::new(RefCell::new(env)),
        Box::new(move |r| match r {
            Ok(Value::Recur(_)) => k(Err("recur must be inside loop".into())),
            r => k(r),
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resumes_nested() {
        let p = Promise::new();
        let mut e = HashMap::new();
        e.insert("p".into(), Value::Promise(p.clone()));
        let mut f = EvalFiber::start("(let [x 1] (+ x (deref p)))", e).unwrap();
        assert_eq!(f.state(), EvalFiberState::Suspended);
        p.resolve(Value::Number(41));
        assert_eq!(
            f.resume(p.state()),
            EvalFiberState::Completed(Value::Number(42))
        );
    }
    #[test]
    fn resumes_function_finally() {
        let p = Promise::new();
        let mut e = HashMap::new();
        e.insert("p".into(), Value::Promise(p.clone()));
        let mut f = EvalFiber::start(
            "(do (def f (fn [x] (try (+ x (deref p)) (finally nil)))) (f 2))",
            e,
        )
        .unwrap();
        assert_eq!(f.state(), EvalFiberState::Suspended);
        p.resolve(Value::Number(40));
        assert_eq!(
            f.resume(p.state()),
            EvalFiberState::Completed(Value::Number(42))
        );
    }
    #[test]
    fn resumes_multi_arity_dispatch() {
        let p = Promise::new();
        let mut e = HashMap::new();
        e.insert("p".into(), Value::Promise(p.clone()));
        let mut f = EvalFiber::start(
            "(do (defn g ([x] (+ x 1)) ([x y] (+ x y (deref p)))) (g 1 2))",
            e,
        )
        .unwrap();
        assert_eq!(f.state(), EvalFiberState::Suspended);
        p.resolve(Value::Number(39));
        assert_eq!(
            f.resume(p.state()),
            EvalFiberState::Completed(Value::Number(42))
        );
        let mut f = EvalFiber::start(
            "(do (defn h ([x] (+ x 1)) ([x y] (+ x y))) (h 41))",
            HashMap::new(),
        )
        .unwrap();
        assert_eq!(f.state(), EvalFiberState::Completed(Value::Number(42)));
    }

    #[test]
    fn computed_function_head_can_suspend() {
        let promise = Promise::new();
        let mut environment = HashMap::new();
        environment.insert("p".into(), Value::Promise(promise.clone()));
        let mut fiber = EvalFiber::start(
            "(do (def entry [:task (fn [] (std.foundation.coroutine/await p))]) \
             ((nth entry 1)))",
            environment,
        )
        .unwrap();
        assert_eq!(fiber.state(), EvalFiberState::Suspended);
        promise.resolve(Value::Number(42));
        assert_eq!(
            fiber.resume(promise.state()),
            EvalFiberState::Completed(Value::Number(42))
        );
    }
}
