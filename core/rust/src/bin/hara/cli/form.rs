use hara_wasm::kernel::Form;

pub(crate) fn keyword(name: &str) -> Form {
    Form::Keyword(name.into())
}

pub(crate) fn string(value: impl Into<String>) -> Form {
    Form::String(value.into())
}

pub(crate) fn map_form(entries: Vec<(&str, Form)>) -> Form {
    Form::Map(
        entries
            .into_iter()
            .map(|(key, value)| (keyword(key), value))
            .collect(),
    )
}

pub(crate) fn map_entries(value: &Form) -> Option<&[(Form, Form)]> {
    match value {
        Form::Map(entries) => Some(entries),
        _ => None,
    }
}

pub(crate) fn map_get<'a>(value: &'a Form, key: &str) -> Option<&'a Form> {
    map_entries(value)?.iter().find_map(|(candidate, value)| {
        matches!(candidate, Form::Keyword(name) if name == key).then_some(value)
    })
}

pub(crate) fn qualified_keyword(value: &Form) -> bool {
    matches!(value, Form::Keyword(name) if name.split_once('/').is_some_and(|(namespace, name)| !namespace.is_empty() && !name.is_empty()))
}

pub(crate) fn walk_form(
    value: &Form,
    path: &mut Vec<Form>,
    function: &mut impl FnMut(&Form, &[Form]),
) {
    function(value, path);
    match value {
        Form::Map(entries) => {
            for (key, value) in entries {
                path.push(key.clone());
                walk_form(value, path, function);
                path.pop();
            }
        }
        Form::Vector(values) | Form::List(values) | Form::Set(values) => {
            for (index, value) in values.iter().enumerate() {
                path.push(Form::Number(index as i64));
                walk_form(value, path, function);
                path.pop();
            }
        }
        _ => {}
    }
}

pub(crate) fn keyword_name(value: &Form) -> Option<String> {
    match value {
        Form::Keyword(value) => Some(value.clone()),
        _ => None,
    }
}

pub(crate) fn string_value(value: &Form) -> Option<String> {
    match value {
        Form::String(value) => Some(value.clone()),
        _ => None,
    }
}
