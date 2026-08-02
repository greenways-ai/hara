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
    for parameter in 0..usize::from(function.arity) {
        entry.locals[parameter] = if program.function_has_i64_parameters(function_id) {
            Rep::I64
        } else {
            Rep::Unknown
        };
    }
    let mut states = vec![None; function.code.len()];
    states[0] = Some(entry);
    let mut work = VecDeque::from([0usize]);
    while let Some(ip) = work.pop_front() {
        let mut output = states[ip].clone().expect("queued state");
        transfer(program, function_id, ip, &function.code[ip], &mut output)?;
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

fn transfer(
    program: &Program,
    function: FunctionId,
    ip: usize,
    instruction: &Instruction,
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
            state.stack.push(primitive_rep(*op, &arguments));
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
            state.stack.push(Rep::Unknown);
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

fn constant_rep(value: &Value) -> Rep {
    match value {
        Value::Number(_) => Rep::I64,
        Value::Bool(_) => Rep::Bool,
        Value::Nil => Rep::Unknown,
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
        Primitive::ArrayNew | Primitive::ArraySet | Primitive::ObjectNew | Primitive::Assoc => {
            Rep::TruthyHandle
        }
        Primitive::ArrayGet => Rep::I64,
        _ => Rep::Unknown,
    }
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
