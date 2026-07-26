use super::*;

pub fn pack_values(values: Vec<Value>) -> Result<Value, String> {
    match values.len() {
        0 => Ok(Value::Nil),
        1 => Ok(values.into_iter().next().unwrap()),
        _ => vector_literal(values),
    }
}

pub fn run_coroutine(step: Step, coroutine: Rc<Coroutine>, k: Cont) -> Step {
    match step {
        Step::Done(Ok(v)) => {
            *coroutine.state.borrow_mut() = CoroutineState::Dead;
            k(Ok(v))
        }
        Step::Done(Err(e)) => {
            *coroutine.state.borrow_mut() = CoroutineState::Dead;
            k(Err(e))
        }
        Step::Yield(value, resume) => {
            *coroutine.state.borrow_mut() = CoroutineState::Suspended(resume);
            k(Ok(value))
        }
        Step::Wait(promise, resume) => Step::Wait(
            promise,
            Box::new(move |state| run_coroutine(resume(state), coroutine, k)),
        ),
    }
}

pub fn coroutine_resume(coroutine: Rc<Coroutine>, args: Vec<Value>, k: Cont) -> Step {
    let mut state = coroutine.state.borrow_mut();
    match std::mem::replace(&mut *state, CoroutineState::Running) {
        CoroutineState::New(body) => {
            drop(state);
            match body {
                Value::Function(f) => {
                    let step = call(f, args, Box::new(move |r| Step::Done(r)));
                    run_coroutine(step, coroutine, k)
                }
                _ => k(Err("coroutine/create expects a function".into())),
            }
        }
        CoroutineState::Suspended(resume) => {
            drop(state);
            match pack_values(args) {
                Ok(packed) => run_coroutine(resume(packed), coroutine, k),
                Err(e) => k(Err(e)),
            }
        }
        CoroutineState::Running => {
            k(Err("coroutine/resume: cannot resume a running coroutine".into()))
        }
        CoroutineState::Dead => k(Err("coroutine/resume: cannot resume a dead coroutine".into())),
    }
}

pub fn create_form(v: Vec<Form>, env: Rc<RefCell<HashMap<String, Value>>>, k: Cont) -> Step {
    if v.len() != 2 {
        return k(Err("coroutine/create expects one function".into()));
    }
    one(
        v[1].clone(),
        env,
        Box::new(move |r| match r {
            Ok(body @ Value::Function(_)) => {
                let coroutine = Coroutine::new(body);
                k(Ok(Value::Coroutine(Rc::new(coroutine))))
            }
            Ok(_) => k(Err("coroutine/create expects a function".into())),
            Err(e) => k(Err(e)),
        }),
    )
}

pub fn predicate_form(v: Vec<Form>, env: Rc<RefCell<HashMap<String, Value>>>, k: Cont) -> Step {
    if v.len() != 2 {
        return k(Err("coroutine/coroutine? expects one value".into()));
    }
    one(
        v[1].clone(),
        env,
        Box::new(move |r| match r {
            Ok(value) => k(Ok(Value::Bool(matches!(value, Value::Coroutine(_))))),
            Err(e) => k(Err(e)),
        }),
    )
}

pub fn status_form(v: Vec<Form>, env: Rc<RefCell<HashMap<String, Value>>>, k: Cont) -> Step {
    if v.len() != 2 {
        return k(Err("coroutine/status expects one coroutine".into()));
    }
    one(
        v[1].clone(),
        env,
        Box::new(move |r| match r {
            Ok(Value::Coroutine(coroutine)) => k(Ok(coroutine_status(&coroutine))),
            Ok(_) => k(Err("coroutine/status expects a coroutine".into())),
            Err(e) => k(Err(e)),
        }),
    )
}

pub fn close_form(v: Vec<Form>, env: Rc<RefCell<HashMap<String, Value>>>, k: Cont) -> Step {
    if v.len() != 2 {
        return k(Err("coroutine/close expects one coroutine".into()));
    }
    one(
        v[1].clone(),
        env,
        Box::new(move |r| match r {
            Ok(Value::Coroutine(coroutine)) => match coroutine_close(&coroutine) {
                Ok(()) => k(Ok(Value::Coroutine(coroutine))),
                Err(e) => k(Err(e)),
            },
            Ok(_) => k(Err("coroutine/close expects a coroutine".into())),
            Err(e) => k(Err(e)),
        }),
    )
}

pub fn resume_form(v: Vec<Form>, env: Rc<RefCell<HashMap<String, Value>>>, k: Cont) -> Step {
    if v.len() < 2 {
        return k(Err("coroutine/resume expects a coroutine".into()));
    }
    let arg_forms = v[2..].to_vec();
    let coroutine_form = v[1].clone();
    one(
        coroutine_form,
        env.clone(),
        Box::new(move |r| match r {
            Ok(Value::Coroutine(coroutine)) => values_cps(
                Rc::new(arg_forms),
                0,
                Vec::new(),
                env,
                Box::new(move |r| match r {
                    Ok(args) => coroutine_resume(coroutine, args, k),
                    Err(e) => k(Err(e)),
                }),
            ),
            Ok(_) => k(Err("coroutine/resume expects a coroutine".into())),
            Err(e) => k(Err(e)),
        }),
    )
}

pub fn yield_form(v: Vec<Form>, env: Rc<RefCell<HashMap<String, Value>>>, k: Cont) -> Step {
    values_cps(
        Rc::new(v[1..].to_vec()),
        0,
        Vec::new(),
        env,
        Box::new(move |r| match r {
            Ok(values) => match pack_values(values) {
                Ok(packed) => Step::Yield(packed, Box::new(move |value| k(Ok(value)))),
                Err(e) => k(Err(e)),
            },
            Err(e) => k(Err(e)),
        }),
    )
}

pub fn await_form(v: Vec<Form>, env: Rc<RefCell<HashMap<String, Value>>>, k: Cont) -> Step {
    if v.len() != 2 {
        return k(Err("coroutine/await expects one derefable".into()));
    }
    one(
        v[1].clone(),
        env,
        Box::new(move |r| match r {
            Ok(Value::Var(x)) => k(Ok(x.deref_value())),
            Ok(Value::Promise(p)) => match p.state() {
                PromiseState::Fulfilled(x) => k(Ok(x)),
                PromiseState::Rejected(e) => k(Err(e)),
                PromiseState::Pending => Step::Wait(
                    p,
                    Box::new(move |s| match s {
                        PromiseState::Fulfilled(x) => k(Ok(x)),
                        PromiseState::Rejected(e) => k(Err(e)),
                        PromiseState::Pending => k(Err("coroutine/await resumed pending".into())),
                    }),
                ),
            },
            Ok(_) => k(Err("coroutine/await expects a derefable (e.g. a promise)".into())),
            Err(e) => k(Err(e)),
        }),
    )
}
