use std::collections::VecDeque;

use crate::core::{Primitive, Value};
use crate::vm::{FunctionId, FunctionPrototype, Instruction, Program};

use super::ir::Rep;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepState {
    pub locals: Vec<Rep>,
    pub stack: Vec<Rep>,
}

/// Point-sensitive representation analysis over bytecode control flow.
/// Facts attach to instruction entry states because VM stack positions are
/// reused for values with different representations.
pub(crate) fn analyze_function(
    program: &Program,
    function_id: FunctionId,
    function: &FunctionPrototype,
) -> Result<Vec<RepState>, String> {
    let mut entry = RepState {
        locals: vec![Rep::Unknown; usize::from(function.local_count)],
        stack: Vec::new(),
    };
    let native_collection = function_native_collection_parameter(function);
    let scalar_kernel = function_is_scalar_kernel(function);
    for parameter in 0..usize::from(function.arity) {
        entry.locals[parameter] = if parameter == 0 {
            native_collection.unwrap_or_else(|| {
                if scalar_kernel || program.function_has_i64_parameters(function_id) {
                    Rep::I64
                } else if function_uses_tagged_collections(function) {
                    Rep::TaggedRef
                } else {
                    Rep::TruthyHandle
                }
            })
        } else if scalar_kernel
            || native_collection == Some(Rep::ArrayRef)
            || program.function_has_i64_parameters(function_id)
        {
            Rep::I64
        } else if function_uses_tagged_collections(function) {
            Rep::TaggedRef
        } else {
            Rep::TruthyHandle
        };
    }
    let mut states = vec![None; function.code.len()];
    states[0] = Some(entry);
    let mut work = VecDeque::from([0usize]);
    while let Some(ip) = work.pop_front() {
        let mut output = states[ip].clone().expect("queued state");
        transfer(
            program,
            function_id,
            ip,
            &function.code[ip],
            function.code.get(ip + 1),
            &mut output,
        )?;
        for successor in successors(ip, &function.code[ip], function.code.len()) {
            let changed = match &mut states[successor] {
                Some(existing) => join(existing, &output)?,
                slot @ None => {
                    *slot = Some(output.clone());
                    true
                }
            };
            if changed {
                work.push_back(successor);
            }
        }
    }
    states
        .into_iter()
        .enumerate()
        .map(|(ip, state)| {
            state.ok_or_else(|| {
                format!("whole-Wasm representation analysis cannot reach instruction {ip}")
            })
        })
        .collect()
}

fn function_is_scalar_kernel(function: &FunctionPrototype) -> bool {
    let mut saw_numeric_operation = false;
    for instruction in &function.code {
        let Some(op) = (match instruction {
            Instruction::Primitive { op, .. } | Instruction::PrimitiveLocalConst { op, .. } => {
                Some(*op)
            }
            _ => None,
        }) else {
            continue;
        };
        if !matches!(
            op,
            Primitive::Add
                | Primitive::Subtract
                | Primitive::Multiply
                | Primitive::Divide
                | Primitive::Remainder
                | Primitive::Equal
                | Primitive::Less
                | Primitive::LessOrEqual
                | Primitive::Greater
                | Primitive::GreaterOrEqual
                | Primitive::NumberPredicate
        ) {
            return false;
        }
        saw_numeric_operation = true;
    }
    saw_numeric_operation
}

/// Native collection primitives take their collection as their first
/// argument. Hara collection algorithms conventionally preserve that value
/// as the first function parameter, including across recursive calls. Treat
/// those operations as a representation constraint on that parameter so a
/// linear-memory reference is never mistaken for a boxed runtime handle.
fn function_native_collection_parameter(function: &FunctionPrototype) -> Option<Rep> {
    let mut inferred = None;
    for instruction in &function.code {
        let candidate = match instruction {
            Instruction::Primitive {
                op: Primitive::ArrayGet | Primitive::ArraySet,
                ..
            }
            | Instruction::PrimitiveLocalConst {
                op: Primitive::ArrayGet | Primitive::ArraySet,
                ..
            } => Some(Rep::ArrayRef),
            Instruction::Primitive {
                op: Primitive::ObjectGet | Primitive::ObjectSet,
                ..
            }
            | Instruction::PrimitiveLocalConst {
                op: Primitive::ObjectGet | Primitive::ObjectSet,
                ..
            } => Some(Rep::ObjectRef),
            _ => None,
        };
        if let Some(candidate) = candidate {
            match inferred {
                None => inferred = Some(candidate),
                Some(current) if current == candidate => {}
                Some(_) => return None,
            }
        }
    }
    inferred
}

fn transfer(
    program: &Program,
    function: FunctionId,
    ip: usize,
    instruction: &Instruction,
    next_instruction: Option<&Instruction>,
    state: &mut RepState,
) -> Result<(), String> {
    let pop = |state: &mut RepState| {
        state.stack.pop().ok_or_else(|| {
            format!("whole-Wasm representation stack underflow in function {function} at {ip}")
        })
    };
    match instruction {
        Instruction::Constant(index) => state.stack.push(constant_rep(
            program
                .constants
                .get(*index as usize)
                .ok_or_else(|| format!("whole-Wasm representation constant {index} is missing"))?,
        )),
        Instruction::Nil => state.stack.push(Rep::Unknown),
        Instruction::True | Instruction::False => state.stack.push(Rep::Bool),
        Instruction::LoadLocal(local) => state.stack.push(state.locals[usize::from(*local)]),
        Instruction::StoreLocal(local) => state.locals[usize::from(*local)] = pop(state)?,
        Instruction::Dup => state.stack.push(*state.stack.last().ok_or_else(|| {
            format!("whole-Wasm representation stack underflow in function {function} at {ip}")
        })?),
        Instruction::Pop | Instruction::JumpIfFalse(_) => {
            pop(state)?;
        }
        Instruction::Primitive { op, argc } => {
            let start = state.stack.len() - usize::from(*argc);
            let arguments = state.stack.split_off(start);
            state.stack.push(
                if *op == Primitive::Get && next_instruction.is_some_and(is_scalar_numeric_consumer)
                {
                    Rep::I64
                } else {
                    primitive_rep(*op, &arguments)
                },
            );
        }
        Instruction::PrimitiveLocalConst {
            op,
            local,
            constant,
        } => {
            let arguments = [
                state.locals[usize::from(*local)],
                constant_rep(&program.constants[*constant as usize]),
            ];
            state.stack.push(primitive_rep(*op, &arguments));
        }
        Instruction::CallStatic { argc, .. } => {
            let start = state.stack.len() - usize::from(*argc);
            state.stack.truncate(start);
            state.stack.push(Rep::I64);
        }
        Instruction::Closure {
            prototype,
            captures: 0,
        } => state.stack.push(Rep::FunctionRef(*prototype)),
        Instruction::DefGlobal { .. } => {}
        Instruction::GetGlobal(index) | Instruction::VarGlobal(index) => {
            state
                .stack
                .push(resolve_function(program, *index).map_or(Rep::Unknown, Rep::FunctionRef));
        }
        Instruction::Call { argc } => {
            let start = state.stack.len() - usize::from(*argc) - 1;
            state.stack.truncate(start);
            state.stack.push(Rep::I64);
        }
        Instruction::BuildVector(count) => {
            let start = state.stack.len() - usize::from(*count);
            let values = state.stack.split_off(start);
            state.stack.push(
                if function_enables_tagged_vectors(program, function)
                    && values
                        .iter()
                        .all(|rep| matches!(rep, Rep::I64 | Rep::TaggedRef))
                {
                    Rep::TaggedRef
                } else {
                    Rep::TruthyHandle
                },
            );
        }
        Instruction::BuildMap(pairs) => {
            let start = state.stack.len() - usize::from(*pairs) * 2;
            state.stack.truncate(start);
            state.stack.push(Rep::TruthyHandle);
        }
        Instruction::Jump(_) | Instruction::Return => {}
        _ => {
            // Preserve point-state shape through instructions that the MIR
            // lowering will subsequently reject with its more specific
            // eligibility diagnostic. Every nonterminal VM instruction has
            // a statically validated net stack effect.
            let effect = instruction.stack_effect().ok_or_else(|| {
                format!(
                    "whole-Wasm function {function} instruction {ip} has no representation transfer: {instruction}"
                )
            })?;
            let next = usize::try_from(state.stack.len() as i32 + effect).map_err(|_| {
                format!("whole-Wasm representation stack underflow in function {function} at {ip}")
            })?;
            state.stack.resize(next, Rep::Unknown);
        }
    }
    Ok(())
}

fn is_scalar_numeric_consumer(instruction: &Instruction) -> bool {
    matches!(
        instruction,
        Instruction::Primitive {
            op: Primitive::Add
                | Primitive::Subtract
                | Primitive::Multiply
                | Primitive::Divide
                | Primitive::Remainder,
            ..
        } | Instruction::PrimitiveLocalConst {
            op: Primitive::Add
                | Primitive::Subtract
                | Primitive::Multiply
                | Primitive::Divide
                | Primitive::Remainder,
            ..
        }
    )
}

fn constant_rep(value: &Value) -> Rep {
    match value {
        Value::Number(_) => Rep::I64,
        Value::Bool(_) => Rep::Bool,
        Value::Nil => Rep::Bool,
        Value::String(_) => Rep::KeyRef,
        _ => Rep::TruthyHandle,
    }
}

fn primitive_rep(op: Primitive, arguments: &[Rep]) -> Rep {
    match op {
        Primitive::Add
        | Primitive::Subtract
        | Primitive::Multiply
        | Primitive::Divide
        | Primitive::Remainder => {
            if arguments.iter().all(|rep| *rep == Rep::I64) {
                Rep::I64
            } else {
                Rep::Unknown
            }
        }
        Primitive::Equal
        | Primitive::Less
        | Primitive::LessOrEqual
        | Primitive::Greater
        | Primitive::GreaterOrEqual
        | Primitive::NumberPredicate => Rep::Bool,
        Primitive::ArrayNew | Primitive::ArraySet => Rep::ArrayRef,
        Primitive::ObjectNew | Primitive::ObjectSet => Rep::ObjectRef,
        Primitive::ObjectGet | Primitive::Count => Rep::I64,
        Primitive::Nth => {
            if arguments.first() == Some(&Rep::TaggedRef) {
                Rep::TaggedRef
            } else {
                Rep::TruthyHandle
            }
        }
        Primitive::Assoc | Primitive::Get => Rep::TruthyHandle,
        Primitive::ArrayGet => Rep::I64,
        _ => Rep::Unknown,
    }
}

fn function_uses_tagged_collections(function: &FunctionPrototype) -> bool {
    function.arity == 1
        && [Primitive::NumberPredicate, Primitive::Count, Primitive::Nth]
            .into_iter()
            .all(|required| {
                function.code.iter().any(|instruction| {
                matches!(instruction, Instruction::Primitive { op, .. } if *op == required)
            })
            })
}

pub(crate) fn function_enables_tagged_vectors(program: &Program, function: FunctionId) -> bool {
    let Some(current) = program.functions.get(usize::from(function)) else {
        return false;
    };
    function_uses_tagged_collections(current)
        || (current
            .code
            .iter()
            .any(|instruction| matches!(instruction, Instruction::Call { .. }))
            && program
                .functions
                .iter()
                .any(function_uses_tagged_collections))
        || current.code.iter().any(|instruction| {
            let Instruction::CallStatic { prototype, .. } = instruction else {
                return false;
            };
            program
                .functions
                .get(usize::from(*prototype))
                .is_some_and(function_uses_tagged_collections)
        })
}

fn resolve_function(program: &Program, constant: u32) -> Option<FunctionId> {
    let Value::String(name) = program.constants.get(constant as usize)? else {
        return None;
    };
    program
        .functions
        .iter()
        .position(|function| {
            function.name.as_deref().is_some_and(|candidate| {
                candidate == name || candidate.rsplit('/').next() == name.rsplit('/').next()
            })
        })
        .and_then(|id| u16::try_from(id).ok())
}

fn successors(ip: usize, instruction: &Instruction, code_len: usize) -> Vec<usize> {
    match instruction {
        Instruction::Jump(target) => vec![*target as usize],
        Instruction::JumpIfFalse(target) => vec![ip + 1, *target as usize],
        Instruction::Return | Instruction::Throw | Instruction::Rethrow => Vec::new(),
        _ if ip + 1 < code_len => vec![ip + 1],
        _ => Vec::new(),
    }
}

fn join(existing: &mut RepState, incoming: &RepState) -> Result<bool, String> {
    if existing.locals.len() != incoming.locals.len()
        || existing.stack.len() != incoming.stack.len()
    {
        return Err("whole-Wasm representation state shape mismatch".into());
    }
    let mut changed = false;
    for (current, next) in existing
        .locals
        .iter_mut()
        .chain(existing.stack.iter_mut())
        .zip(incoming.locals.iter().chain(&incoming.stack))
    {
        let joined = if current == next {
            *current
        } else {
            Rep::Unknown
        };
        if *current != joined {
            *current = joined;
            changed = true;
        }
    }
    Ok(changed)
}
