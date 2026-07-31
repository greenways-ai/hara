//! Program validation: one abstract-interpretation pass over the code
//! vector before any execution. After validation the machine indexes
//! without re-checking, and malformed programs never reach a panic.

use super::error::ValidationError;
use super::opcode::Instruction;
use super::program::{
    FunctionPrototype, Program, MAX_CONSTANTS, MAX_INSTRUCTIONS, MAX_LOCALS, MAX_OPERAND_STACK,
};

/// Validates a whole program. See `notes/rust-bytecode-vm.md` §9 for the
/// rule list.
pub fn validate(program: &Program) -> Result<(), ValidationError> {
    if program.constants.len() > MAX_CONSTANTS {
        return Err(ValidationError::new(
            format!("constant pool exceeds limit of {MAX_CONSTANTS}"),
            None,
        ));
    }
    if program.functions.is_empty() {
        return Err(ValidationError::new("program has no functions", None));
    }
    if program.entry as usize >= program.functions.len() {
        return Err(ValidationError::new("entry function index out of range", None));
    }
    let multiple = program.functions.len() > 1;
    for (index, function) in program.functions.iter().enumerate() {
        validate_function(program, function).map_err(|mut error| {
            if multiple {
                error.message = format!("function {index}: {}", error.message);
            }
            error
        })?;
    }
    Ok(())
}

fn validate_function(
    program: &Program,
    function: &FunctionPrototype,
) -> Result<(), ValidationError> {
    if function.source_map.len() != function.code.len() {
        return Err(ValidationError::new(
            "source map length does not match code length",
            None,
        ));
    }
    let heights = stack_heights(program, function)?;
    let computed = heights.iter().copied().max().unwrap_or(0);
    if computed != function.max_stack {
        return Err(ValidationError::new(
            format!(
                "declared max_stack {} disagrees with computed {computed}",
                function.max_stack
            ),
            None,
        ));
    }
    Ok(())
}

/// Computes the unique operand-stack height at every instruction while
/// checking indexes, slots, jump targets, reachability, and termination.
/// Shared by the validator and by the compiler, which uses it to fill in
/// `max_stack` for code it just emitted.
pub(crate) fn stack_heights(
    program: &Program,
    function: &FunctionPrototype,
) -> Result<Vec<u16>, ValidationError> {
    let code = &function.code;
    if code.is_empty() {
        return Err(ValidationError::new("function has no code", None));
    }
    if code.len() > MAX_INSTRUCTIONS {
        return Err(ValidationError::new(
            format!("code exceeds limit of {MAX_INSTRUCTIONS} instructions"),
            None,
        ));
    }
    if usize::from(function.local_count) > MAX_LOCALS {
        return Err(ValidationError::new("local count exceeds slot limit", None));
    }
    let mut heights: Vec<Option<u16>> = vec![None; code.len()];
    let mut worklist: Vec<(usize, u16)> = vec![(0, 0)];
    while let Some((ip, height)) = worklist.pop() {
        if let Some(existing) = heights[ip] {
            if existing != height {
                return Err(ValidationError::new(
                    format!("inconsistent stack heights {existing} and {height} at join"),
                    Some(ip as u32),
                ));
            }
            continue;
        }
        heights[ip] = Some(height);
        let instruction = &code[ip];
        let at = Some(ip as u32);
        // Operand checks independent of control flow.
        match instruction {
            Instruction::Constant(index) if *index as usize >= program.constants.len() => {
                return Err(ValidationError::new(
                    format!("constant index {index} out of range"),
                    at,
                ));
            }
            Instruction::LoadLocal(slot) | Instruction::StoreLocal(slot)
                if *slot >= function.local_count =>
            {
                return Err(ValidationError::new(
                    format!("local slot {slot} out of range"),
                    at,
                ));
            }
            Instruction::Jump(target) | Instruction::JumpIfFalse(target)
                if *target as usize >= code.len() =>
            {
                return Err(ValidationError::new(
                    format!("jump target {target} out of range"),
                    at,
                ));
            }
            Instruction::Closure { prototype, captures } => {
                let Some(target) = program.functions.get(usize::from(*prototype)) else {
                    return Err(ValidationError::new(
                        format!("closure prototype {prototype} out of range"),
                        at,
                    ));
                };
                if usize::from(*captures) != usize::from(target.capture_count) {
                    return Err(ValidationError::new(
                        format!(
                            "closure captures {captures} but prototype expects {}",
                            target.capture_count
                        ),
                        at,
                    ));
                }
            }
            Instruction::CallStatic { prototype, argc } => {
                let Some(target) = program.functions.get(usize::from(*prototype)) else {
                    return Err(ValidationError::new(
                        format!("callstatic target {prototype} out of range"),
                        at,
                    ));
                };
                if usize::from(*argc) != usize::from(target.arity) {
                    return Err(ValidationError::new(
                        format!(
                            "callstatic argc {argc} but prototype expects {}",
                            target.arity
                        ),
                        at,
                    ));
                }
                if target.capture_count != function.capture_count {
                    return Err(ValidationError::new(
                        "callstatic capture count differs from current function",
                        at,
                    ));
                }
            }
            _ => {}
        }
        // Stack effects and successors.
        if let Instruction::Return = instruction {
            if height != 1 {
                return Err(ValidationError::new(
                    format!("return with stack height {height}, expected 1"),
                    at,
                ));
            }
            continue;
        }
        let effect = instruction.stack_effect().expect("non-terminal instruction");
        let next = height as i32 + effect;
        if next < 0 {
            return Err(ValidationError::new("stack underflow", at));
        }
        if next as usize > MAX_OPERAND_STACK {
            return Err(ValidationError::new(
                format!("operand stack exceeds limit of {MAX_OPERAND_STACK}"),
                at,
            ));
        }
        let next = next as u16;
        match instruction {
            Instruction::Jump(target) => worklist.push((*target as usize, next)),
            Instruction::JumpIfFalse(target) => {
                worklist.push((*target as usize, next));
                push_fallthrough(code, ip, next, &mut worklist)?;
            }
            _ => push_fallthrough(code, ip, next, &mut worklist)?,
        }
    }
    let mut result = Vec::with_capacity(code.len());
    for (ip, height) in heights.into_iter().enumerate() {
        match height {
            Some(height) => result.push(height),
            None => {
                return Err(ValidationError::new(
                    "unreachable instruction",
                    Some(ip as u32),
                ))
            }
        }
    }
    Ok(result)
}

fn push_fallthrough(
    code: &[Instruction],
    ip: usize,
    height: u16,
    worklist: &mut Vec<(usize, u16)>,
) -> Result<(), ValidationError> {
    if ip + 1 == code.len() {
        return Err(ValidationError::new(
            "missing return: control falls off the end of the function",
            Some(ip as u32),
        ));
    }
    debug_assert!(code[ip].falls_through());
    worklist.push((ip + 1, height));
    Ok(())
}
