//! Canonical semantic representation of the portable HAL schema grammar.
//!
//! HALC keeps surface schemas as ordinary forms on the wire. This module is
//! the first compiler-facing lowering step: it turns those forms into a typed
//! graph without evaluating schema Vars or copying nested definitions.

use super::Form;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct SchemaField {
    pub name: Form,
    pub value_type: SchemaType,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionSchema {
    pub fixed: Vec<SchemaType>,
    pub rest: Option<Box<SchemaType>>,
    pub output: Box<SchemaType>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SchemaType {
    Primitive(String),
    Reference(String),
    Union(Vec<SchemaType>),
    Vector(Box<SchemaType>),
    Tuple(Vec<SchemaType>),
    Map(Vec<SchemaField>),
    Function(Vec<FunctionSchema>),
    Enum(Vec<Form>),
    Extension { head: String, arguments: Vec<Form> },
    Unknown(Form),
}

pub fn normalize_schema(schema: &Form) -> Result<SchemaType, String> {
    match schema {
        Form::Keyword(name) => Ok(SchemaType::Primitive(name.clone())),
        Form::List(reference)
            if reference.len() == 2
                && matches!(&reference[0], Form::Symbol(operator) if operator == "var") =>
        {
            match &reference[1] {
                Form::Symbol(name) if name.contains('/') => Ok(SchemaType::Reference(name.clone())),
                Form::Symbol(name) => Err(format!(
                    "named schema reference is not fully qualified: {name}"
                )),
                _ => Err("named schema reference must target a symbol".into()),
            }
        }
        Form::Vector(items) if !items.is_empty() => normalize_composite(items),
        other => Ok(SchemaType::Unknown(other.clone())),
    }
}

/// Infers conservative function signatures from executable module forms.
/// Declared schemas seed parameter types, but inferred results remain a
/// separate table: annotations are contracts, while these are optimizer facts.
pub fn infer_function_types(
    namespace: &str,
    forms: &[Form],
    declarations: &HashMap<String, SchemaType>,
    definitions: &HashMap<String, SchemaType>,
) -> HashMap<String, SchemaType> {
    let mut inferred = HashMap::new();
    for form in forms {
        let Form::List(items) = super::super::core::form_without_metadata(form) else {
            continue;
        };
        if !matches!(items.first(), Some(Form::Symbol(operator)) if operator == "defn" || operator == "defn-")
        {
            continue;
        }
        let Some(name) = items.get(1).and_then(binding_name) else {
            continue;
        };
        let qualified = format!("{namespace}/{name}");
        let parameters_at = items.iter().enumerate().skip(2).find_map(|(index, value)| {
            matches!(
                super::super::core::form_without_metadata(value),
                Form::Vector(_)
            )
            .then_some(index)
        });
        let declared = declarations
            .get(&qualified)
            .and_then(|schema| resolve_type(schema, definitions));
        let mut arities = Vec::new();
        if let Some(parameters_at) = parameters_at {
            let Form::Vector(parameters) =
                super::super::core::form_without_metadata(&items[parameters_at])
            else {
                continue;
            };
            arities.push(infer_function_arity(
                parameters,
                &items[parameters_at + 1..],
                declared,
            ));
        } else {
            for clause in items.iter().skip(2) {
                let Form::List(clause) = super::super::core::form_without_metadata(clause) else {
                    continue;
                };
                let Some(Form::Vector(parameters)) = clause.first() else {
                    continue;
                };
                arities.push(infer_function_arity(parameters, &clause[1..], declared));
            }
        }
        if !arities.is_empty() {
            inferred.insert(qualified, SchemaType::Function(arities));
        }
    }
    inferred
}

fn infer_function_arity(
    parameters: &[Form],
    body: &[Form],
    declared: Option<&SchemaType>,
) -> FunctionSchema {
    let declared_arity = match declared {
        Some(SchemaType::Function(arities)) => arities.iter().find(|arity| {
            arity.fixed.len()
                == parameters
                    .iter()
                    .take_while(|form| !matches!(form, Form::Symbol(marker) if marker == "&"))
                    .count()
                && arity.rest.is_some()
                    == parameters
                        .iter()
                        .any(|form| matches!(form, Form::Symbol(marker) if marker == "&"))
        }),
        _ => None,
    };
    let mut environment = HashMap::new();
    let mut fixed = Vec::new();
    let mut rest = None;
    let mut parameter_index = 0;
    let mut variadic = false;
    for parameter in parameters {
        if matches!(parameter, Form::Symbol(marker) if marker == "&") {
            variadic = true;
            continue;
        }
        let Some(parameter_name) = binding_name(parameter) else {
            continue;
        };
        let parameter_type = if variadic {
            declared_arity
                .and_then(|arity| arity.rest.as_deref())
                .cloned()
                .unwrap_or_else(unknown_type)
        } else {
            declared_arity
                .and_then(|arity| arity.fixed.get(parameter_index))
                .cloned()
                .unwrap_or_else(unknown_type)
        };
        environment.insert(parameter_name.to_owned(), parameter_type.clone());
        if variadic {
            rest = Some(Box::new(parameter_type));
        } else {
            fixed.push(parameter_type);
            parameter_index += 1;
        }
    }
    let output = body
        .iter()
        .map(|body| infer_expression(body, &mut environment))
        .last()
        .unwrap_or_else(|| SchemaType::Primitive("nil".into()));
    FunctionSchema {
        fixed,
        rest,
        output: Box::new(output),
    }
}

fn binding_name(form: &Form) -> Option<&str> {
    match form {
        Form::Symbol(name) => Some(name),
        Form::Metadata(_, value) => binding_name(value),
        _ => None,
    }
}

fn resolve_type<'a>(
    schema: &'a SchemaType,
    definitions: &'a HashMap<String, SchemaType>,
) -> Option<&'a SchemaType> {
    let mut current = schema;
    let mut visited = std::collections::HashSet::new();
    while let SchemaType::Reference(name) = current {
        if !visited.insert(name) {
            return Some(current);
        }
        current = definitions.get(name)?;
    }
    Some(current)
}

fn unknown_type() -> SchemaType {
    SchemaType::Unknown(Form::Symbol("?".into()))
}

fn infer_expression(form: &Form, environment: &mut HashMap<String, SchemaType>) -> SchemaType {
    match super::super::core::form_without_metadata(form) {
        Form::Nil => SchemaType::Primitive("nil".into()),
        Form::Bool(_) => SchemaType::Primitive("bool".into()),
        Form::Number(_) => SchemaType::Primitive("int".into()),
        Form::Float(_) => SchemaType::Primitive("float".into()),
        Form::BigInteger(_) => SchemaType::Primitive("bigint".into()),
        Form::Decimal(_) => SchemaType::Primitive("decimal".into()),
        Form::Character(_) => SchemaType::Primitive("char".into()),
        Form::Regex(_) => SchemaType::Primitive("regex".into()),
        Form::String(_) => SchemaType::Primitive("str".into()),
        Form::Keyword(_) => SchemaType::Primitive("keyword".into()),
        Form::Symbol(name) => environment.get(name).cloned().unwrap_or_else(unknown_type),
        Form::Vector(values) => SchemaType::Vector(Box::new(join_types(
            values
                .iter()
                .map(|value| infer_expression(value, environment)),
        ))),
        Form::Map(entries) => SchemaType::Map(
            entries
                .iter()
                .map(|(name, value)| SchemaField {
                    name: name.clone(),
                    value_type: infer_expression(value, environment),
                })
                .collect(),
        ),
        Form::Set(_) => SchemaType::Extension {
            head: "set".into(),
            arguments: Vec::new(),
        },
        Form::List(items) if items.is_empty() => SchemaType::Extension {
            head: "list".into(),
            arguments: Vec::new(),
        },
        Form::List(items) => infer_list(items, environment),
        Form::Tagged(_, value) => infer_expression(value, environment),
        Form::Metadata(_, value) => infer_expression(value, environment),
    }
}

fn infer_list(items: &[Form], environment: &mut HashMap<String, SchemaType>) -> SchemaType {
    let Some(Form::Symbol(operator)) = items.first() else {
        return unknown_type();
    };
    match operator.as_str() {
        "do" => items[1..]
            .iter()
            .map(|value| infer_expression(value, environment))
            .last()
            .unwrap_or_else(|| SchemaType::Primitive("nil".into())),
        "if" => join_types(
            items[2..]
                .iter()
                .map(|value| infer_expression(value, environment)),
        ),
        "let" if items.len() >= 3 => {
            let mut nested = environment.clone();
            if let Form::Vector(bindings) = super::super::core::form_without_metadata(&items[1]) {
                for pair in bindings.chunks(2) {
                    if let [name, value] = pair {
                        if let Some(name) = binding_name(name) {
                            let value_type = infer_expression(value, &mut nested);
                            nested.insert(name.to_owned(), value_type);
                        }
                    }
                }
            }
            items[2..]
                .iter()
                .map(|value| infer_expression(value, &mut nested))
                .last()
                .unwrap_or_else(|| SchemaType::Primitive("nil".into()))
        }
        "+" | "-" | "*" | "%" | "mod" => {
            let operands = join_types(
                items[1..]
                    .iter()
                    .map(|value| infer_expression(value, environment)),
            );
            match operands {
                SchemaType::Primitive(name)
                    if matches!(name.as_str(), "int" | "float" | "bigint" | "decimal") =>
                {
                    SchemaType::Primitive(name)
                }
                _ => SchemaType::Primitive("number".into()),
            }
        }
        "/" => SchemaType::Primitive("number".into()),
        "=" | "<" | "<=" | ">" | ">=" | "instance?" => SchemaType::Primitive("bool".into()),
        "count" => SchemaType::Primitive("int".into()),
        "vector" => SchemaType::Vector(Box::new(join_types(
            items[1..]
                .iter()
                .map(|value| infer_expression(value, environment)),
        ))),
        _ => unknown_type(),
    }
}

fn join_types(types: impl IntoIterator<Item = SchemaType>) -> SchemaType {
    let mut members = Vec::new();
    for value in types {
        match value {
            SchemaType::Union(nested) => {
                for member in nested {
                    push_unique(&mut members, member);
                }
            }
            member => push_unique(&mut members, member),
        }
    }
    match members.len() {
        0 => unknown_type(),
        1 => members.pop().unwrap(),
        _ => SchemaType::Union(members),
    }
}

fn normalize_composite(items: &[Form]) -> Result<SchemaType, String> {
    let Form::Keyword(head) = &items[0] else {
        return Ok(SchemaType::Unknown(Form::Vector(items.to_vec())));
    };
    let arguments = &items[1..];
    match head.as_str() {
        "or" => {
            if arguments.is_empty() {
                return Err(":or schema requires at least one member".into());
            }
            let mut members = Vec::new();
            for argument in arguments {
                let normalized = normalize_schema(argument)?;
                match normalized {
                    SchemaType::Union(nested) => {
                        for member in nested {
                            push_unique(&mut members, member);
                        }
                    }
                    member => push_unique(&mut members, member),
                }
            }
            Ok(if members.len() == 1 {
                members.pop().unwrap()
            } else {
                SchemaType::Union(members)
            })
        }
        "maybe" => {
            require_count(head, arguments, 1)?;
            let mut members = Vec::new();
            push_unique(&mut members, normalize_schema(&arguments[0])?);
            push_unique(&mut members, SchemaType::Primitive("nil".into()));
            Ok(SchemaType::Union(members))
        }
        "vector" => {
            require_count(head, arguments, 1)?;
            Ok(SchemaType::Vector(Box::new(normalize_schema(
                &arguments[0],
            )?)))
        }
        "tuple" => arguments
            .iter()
            .map(normalize_schema)
            .collect::<Result<Vec<_>, _>>()
            .map(SchemaType::Tuple),
        "map" => {
            let mut fields = Vec::with_capacity(arguments.len());
            for argument in arguments {
                let Form::Vector(pair) = argument else {
                    return Err(":map schema fields must be [name type] pairs".into());
                };
                if pair.len() != 2 {
                    return Err(":map schema fields must be [name type] pairs".into());
                }
                fields.push(SchemaField {
                    name: pair[0].clone(),
                    value_type: normalize_schema(&pair[1])?,
                });
            }
            Ok(SchemaType::Map(fields))
        }
        "fn" => normalize_function(items).map(|arity| SchemaType::Function(vec![arity])),
        "function" => {
            if arguments.is_empty() {
                return Err(":function schema requires at least one :fn schema".into());
            }
            arguments
                .iter()
                .map(|argument| {
                    let Form::Vector(function) = argument else {
                        return Err(":function members must be :fn schemas".into());
                    };
                    normalize_function(function)
                })
                .collect::<Result<Vec<_>, _>>()
                .map(SchemaType::Function)
        }
        "enum" => Ok(SchemaType::Enum(arguments.to_vec())),
        _ => Ok(SchemaType::Extension {
            head: head.clone(),
            arguments: arguments.to_vec(),
        }),
    }
}

fn normalize_function(items: &[Form]) -> Result<FunctionSchema, String> {
    if !matches!(items.first(), Some(Form::Keyword(head)) if head == "fn") || items.len() != 3 {
        return Err(":fn schema must be [:fn [inputs ...] output]".into());
    }
    let Form::Vector(inputs) = &items[1] else {
        return Err(":fn schema inputs must be a vector".into());
    };
    let mut fixed = Vec::new();
    let mut rest = None;
    let mut index = 0;
    while index < inputs.len() {
        if matches!(&inputs[index], Form::Symbol(marker) if marker == "&") {
            if rest.is_some() || index + 2 != inputs.len() {
                return Err(":fn schema & must precede exactly one rest type".into());
            }
            rest = Some(Box::new(normalize_schema(&inputs[index + 1])?));
            index += 2;
        } else {
            fixed.push(normalize_schema(&inputs[index])?);
            index += 1;
        }
    }
    Ok(FunctionSchema {
        fixed,
        rest,
        output: Box::new(normalize_schema(&items[2])?),
    })
}

fn require_count(head: &str, arguments: &[Form], expected: usize) -> Result<(), String> {
    if arguments.len() == expected {
        Ok(())
    } else {
        Err(format!(
            ":{head} schema expects {expected} argument{}, got {}",
            if expected == 1 { "" } else { "s" },
            arguments.len()
        ))
    }
}

fn push_unique(output: &mut Vec<SchemaType>, value: SchemaType) {
    if !output.contains(&value) {
        output.push(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::parse;

    #[test]
    fn normalizes_nested_named_function_schemas() {
        assert_eq!(
            normalize_schema(&parse("[:fn [#'demo/Customer & :int] [:maybe :str]]").unwrap())
                .unwrap(),
            SchemaType::Function(vec![FunctionSchema {
                fixed: vec![SchemaType::Reference("demo/Customer".into())],
                rest: Some(Box::new(SchemaType::Primitive("int".into()))),
                output: Box::new(SchemaType::Union(vec![
                    SchemaType::Primitive("str".into()),
                    SchemaType::Primitive("nil".into()),
                ])),
            }])
        );
    }

    #[test]
    fn rejects_malformed_known_schema_forms() {
        assert!(normalize_schema(&parse("[:map [:name]]").unwrap()).is_err());
        assert!(normalize_schema(&parse("[:fn [:str & :int :bool] :str]").unwrap()).is_err());
        assert!(normalize_schema(&parse("[:maybe]").unwrap()).is_err());
    }

    #[test]
    fn infers_body_results_without_replacing_declared_contracts() {
        let forms = crate::kernel::parse_forms(
            "(ns demo)\n\
             (def Unary [:fn [:int] :number])\n\
             (defn ^{:schema #'Unary} choose [value]\n\
               (let [next (+ value 1)] (if true next 0)))\n\
             (defn labels [] {:name \"Ada\" :active true})\n\
             (defn select ([value] value) ([left right] right))",
        )
        .unwrap();
        let declarations = HashMap::from([(
            "demo/choose".into(),
            SchemaType::Reference("demo/Unary".into()),
        )]);
        let definitions = HashMap::from([(
            "demo/Unary".into(),
            normalize_schema(&parse("[:fn [:int] :number]").unwrap()).unwrap(),
        )]);
        let inferred = infer_function_types("demo", &forms, &declarations, &definitions);

        assert!(matches!(
            inferred.get("demo/choose"),
            Some(SchemaType::Function(arities))
                if arities[0].fixed == vec![SchemaType::Primitive("int".into())]
                    && *arities[0].output == SchemaType::Primitive("int".into())
        ));
        assert!(matches!(
            inferred.get("demo/labels"),
            Some(SchemaType::Function(arities))
                if matches!(arities[0].output.as_ref(), SchemaType::Map(fields) if fields.len() == 2)
        ));
        assert!(matches!(
            inferred.get("demo/select"),
            Some(SchemaType::Function(arities)) if arities.len() == 2
        ));
    }
}
