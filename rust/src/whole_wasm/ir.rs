use std::collections::{BTreeMap, BTreeSet};

use crate::core::{Primitive, Value};
use crate::vm::{FunctionId, Instruction, Program};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rep {
    I64,
    Bool,
    ArrayRef,
    TruthyHandle,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MirOp {
    Constant {
        destination: u16,
        value: i64,
        rep: Rep,
    },
    Move {
        destination: u16,
        source: u16,
    },
    Binary {
        destination: u16,
        left: u16,
        right: u16,
        op: Primitive,
    },
    BinaryConstant {
        destination: u16,
        left: u16,
        right: i64,
        op: Primitive,
    },
    ArrayNew {
        destination: u16,
        values: Vec<u16>,
    },
    ArrayGetI64 {
        destination: u16,
        array: u16,
        index: u16,
    },
    ArrayGetI64Constant {
        destination: u16,
        array: u16,
        index: i64,
    },
    ArraySetI64 {
        destination: u16,
        array: u16,
        index: u16,
        value: u16,
    },
    CallStatic {
        destination: u16,
        function: FunctionId,
        arguments: Vec<u16>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MirTerminator {
    Goto(u16),
    BranchZero {
        condition: u16,
        rep: Rep,
        zero: u16,
        nonzero: u16,
    },
    Return(u16),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirBlock {
    pub id: u16,
    pub start: u32,
    pub operations: Vec<MirOp>,
    pub terminator: MirTerminator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirFunction {
    pub id: FunctionId,
    pub name: Option<String>,
    pub arity: u16,
    pub local_count: u16,
    pub stack_count: u16,
    pub blocks: Vec<MirBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirProgram {
    pub entry: FunctionId,
    pub functions: Vec<MirFunction>,
}

/// Converts every eligible bytecode function into whole-function block IR.
/// This first representation slice is deliberately strict: it accepts only
/// synchronous, capture-free, fixed-arity scalar functions and fails the
/// complete compilation if a reachable operation has no exact lowering.
pub fn lower_program(program: &Program) -> Result<MirProgram, String> {
    crate::vm::validate(program).map_err(|error| error.to_string())?;
    let functions = program
        .functions
        .iter()
        .enumerate()
        .map(|(id, function)| lower_function(program, id as FunctionId, function))
        .collect::<Result<Vec<_>, _>>()?;
    let mir = MirProgram {
        entry: program.entry,
        functions,
    };
    verify(&mir)?;
    Ok(mir)
}

fn lower_function(
    program: &Program,
    id: FunctionId,
    function: &crate::vm::FunctionPrototype,
) -> Result<MirFunction, String> {
    if function.async_function || function.variadic || function.capture_count != 0 {
        return Err(unsupported(id, 0, "function shape"));
    }
    let heights =
        crate::vm::validate::stack_heights(program, function).map_err(|error| error.to_string())?;
    let representations = super::reps::analyze_function(program, id, function)?;
    let mut leaders = BTreeSet::from([0usize]);
    for (ip, instruction) in function.code.iter().enumerate() {
        if let Some(target) = instruction.jump_target() {
            leaders.insert(target as usize);
        }
        if matches!(
            instruction,
            Instruction::Jump(_)
                | Instruction::JumpIfFalse(_)
                | Instruction::Return
                | Instruction::Throw
                | Instruction::Rethrow
        ) && ip + 1 < function.code.len()
        {
            leaders.insert(ip + 1);
        }
    }
    let starts = leaders.into_iter().collect::<Vec<_>>();
    let ids = starts
        .iter()
        .enumerate()
        .map(|(id, start)| (*start, id as u16))
        .collect::<BTreeMap<_, _>>();
    let stack_base = function.local_count;
    let mut blocks = Vec::with_capacity(starts.len());
    for (block_index, start) in starts.iter().copied().enumerate() {
        let end = starts
            .get(block_index + 1)
            .copied()
            .unwrap_or(function.code.len());
        let mut operations = Vec::new();
        let mut terminator = None;
        for ip in start..end {
            let instruction = &function.code[ip];
            let height = usize::from(heights[ip]);
            let stack = |offset: usize| -> Result<u16, String> {
                stack_base
                    .checked_add(u16::try_from(offset).map_err(|_| "operand stack exceeds u16")?)
                    .ok_or_else(|| "whole-Wasm slot index overflow".to_string())
            };
            match instruction {
                Instruction::Constant(index) => {
                    let (value, rep) = scalar_constant(program.constants.get(*index as usize))
                        .ok_or_else(|| unsupported(id, ip, "non-scalar constant"))?;
                    operations.push(MirOp::Constant {
                        destination: stack(height)?,
                        value,
                        rep,
                    });
                }
                Instruction::True => operations.push(MirOp::Constant {
                    destination: stack(height)?,
                    value: 1,
                    rep: Rep::Bool,
                }),
                Instruction::False => operations.push(MirOp::Constant {
                    destination: stack(height)?,
                    value: 0,
                    rep: Rep::Bool,
                }),
                Instruction::LoadLocal(source) => operations.push(MirOp::Move {
                    destination: stack(height)?,
                    source: *source,
                }),
                Instruction::StoreLocal(destination) => operations.push(MirOp::Move {
                    destination: *destination,
                    source: stack(height - 1)?,
                }),
                Instruction::Dup => operations.push(MirOp::Move {
                    destination: stack(height)?,
                    source: stack(height - 1)?,
                }),
                Instruction::Pop => {}
                Instruction::Primitive { op, argc: 2 } if scalar_binary(*op) => {
                    operations.push(MirOp::Binary {
                        destination: stack(height - 2)?,
                        left: stack(height - 2)?,
                        right: stack(height - 1)?,
                        op: *op,
                    });
                }
                Instruction::Primitive {
                    op: Primitive::ArrayNew,
                    argc,
                } => {
                    let base = height - usize::from(*argc);
                    let values = (0..usize::from(*argc))
                        .map(|index| stack(base + index))
                        .collect::<Result<Vec<_>, _>>()?;
                    if values
                        .iter()
                        .enumerate()
                        .any(|(index, _)| representations[ip].stack[base + index] != Rep::I64)
                    {
                        return Err(unsupported(id, ip, "array constructor requires i64 values"));
                    }
                    operations.push(MirOp::ArrayNew {
                        destination: stack(base)?,
                        values,
                    });
                }
                Instruction::Primitive {
                    op: Primitive::ArrayGet,
                    argc: 2,
                } => operations.push(MirOp::ArrayGetI64 {
                    destination: stack(height - 2)?,
                    array: stack(height - 2)?,
                    index: stack(height - 1)?,
                }),
                Instruction::Primitive {
                    op: Primitive::ArraySet,
                    argc: 3,
                } => operations.push(MirOp::ArraySetI64 {
                    destination: stack(height - 3)?,
                    array: stack(height - 3)?,
                    index: stack(height - 2)?,
                    value: stack(height - 1)?,
                }),
                Instruction::PrimitiveLocalConst {
                    op,
                    local,
                    constant,
                } if scalar_binary(*op) => {
                    let (right, _) = scalar_constant(program.constants.get(*constant as usize))
                        .ok_or_else(|| unsupported(id, ip, "non-scalar primitive constant"))?;
                    operations.push(MirOp::BinaryConstant {
                        destination: stack(height)?,
                        left: *local,
                        right,
                        op: *op,
                    });
                }
                Instruction::PrimitiveLocalConst {
                    op: Primitive::ArrayGet,
                    local,
                    constant,
                } => {
                    let (index, rep) =
                        scalar_constant(program.constants.get(*constant as usize))
                            .ok_or_else(|| unsupported(id, ip, "array index constant"))?;
                    if rep != Rep::I64 {
                        return Err(unsupported(id, ip, "array index must be i64"));
                    }
                    operations.push(MirOp::ArrayGetI64Constant {
                        destination: stack(height)?,
                        array: *local,
                        index,
                    });
                }
                Instruction::CallStatic { prototype, argc } => {
                    let target = program
                        .functions
                        .get(usize::from(*prototype))
                        .ok_or_else(|| unsupported(id, ip, "call target"))?;
                    if target.async_function
                        || target.variadic
                        || target.capture_count != 0
                        || target.arity != u16::from(*argc)
                    {
                        return Err(unsupported(id, ip, "static call shape"));
                    }
                    let base = height - usize::from(*argc);
                    operations.push(MirOp::CallStatic {
                        destination: stack(base)?,
                        function: *prototype,
                        arguments: (0..usize::from(*argc))
                            .map(|index| stack(base + index))
                            .collect::<Result<Vec<_>, _>>()?,
                    });
                }
                Instruction::Jump(target) => {
                    terminator = Some(MirTerminator::Goto(block_id(&ids, *target)?));
                }
                Instruction::JumpIfFalse(target) => {
                    let rep =
                        representations[ip].stack.last().copied().ok_or_else(|| {
                            unsupported(id, ip, "condition has no representation")
                        })?;
                    if rep == Rep::Unknown {
                        return Err(unsupported(id, ip, "unproven dynamic truthiness"));
                    }
                    let fallthrough = u32::try_from(ip + 1)
                        .map_err(|_| "whole-Wasm instruction index overflow")?;
                    terminator = Some(MirTerminator::BranchZero {
                        condition: stack(height - 1)?,
                        rep,
                        zero: block_id(&ids, *target)?,
                        nonzero: block_id(&ids, fallthrough)?,
                    });
                }
                Instruction::Return => {
                    terminator = Some(MirTerminator::Return(stack(height - 1)?));
                }
                other => return Err(unsupported(id, ip, &other.to_string())),
            }
            if terminator.is_some() {
                break;
            }
        }
        let terminator = terminator.unwrap_or_else(|| {
            MirTerminator::Goto(ids.get(&end).copied().expect("validated fallthrough block"))
        });
        blocks.push(MirBlock {
            id: block_index as u16,
            start: start as u32,
            operations,
            terminator,
        });
    }
    Ok(MirFunction {
        id,
        name: function.name.clone(),
        arity: function.arity,
        local_count: function.local_count,
        stack_count: function.max_stack,
        blocks,
    })
}

fn scalar_constant(value: Option<&Value>) -> Option<(i64, Rep)> {
    match value? {
        Value::Number(value) => Some((*value, Rep::I64)),
        Value::Bool(value) => Some((i64::from(*value), Rep::Bool)),
        _ => None,
    }
}

fn scalar_binary(op: Primitive) -> bool {
    matches!(
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
    )
}

fn block_id(ids: &BTreeMap<usize, u16>, target: u32) -> Result<u16, String> {
    ids.get(&(target as usize))
        .copied()
        .ok_or_else(|| format!("whole-Wasm target {target} is not a block leader"))
}

fn unsupported(function: FunctionId, instruction: usize, detail: &str) -> String {
    format!("whole-Wasm function {function} instruction {instruction} unsupported: {detail}")
}

pub fn verify(program: &MirProgram) -> Result<(), String> {
    if program.functions.is_empty() || usize::from(program.entry) >= program.functions.len() {
        return Err("whole-Wasm MIR has an invalid entry".into());
    }
    for (expected, function) in program.functions.iter().enumerate() {
        if usize::from(function.id) != expected || function.blocks.is_empty() {
            return Err(format!("whole-Wasm MIR function {expected} is malformed"));
        }
        let slot_count = function
            .local_count
            .checked_add(function.stack_count)
            .ok_or("whole-Wasm MIR slot count overflow")?;
        let valid_slot = |slot: u16| slot < slot_count;
        for (expected_block, block) in function.blocks.iter().enumerate() {
            if usize::from(block.id) != expected_block {
                return Err(format!(
                    "whole-Wasm MIR block id mismatch in function {expected}"
                ));
            }
            for operation in &block.operations {
                let valid = match operation {
                    MirOp::Constant { destination, .. } => valid_slot(*destination),
                    MirOp::Move {
                        destination,
                        source,
                    } => valid_slot(*destination) && valid_slot(*source),
                    MirOp::Binary {
                        destination,
                        left,
                        right,
                        ..
                    } => valid_slot(*destination) && valid_slot(*left) && valid_slot(*right),
                    MirOp::BinaryConstant {
                        destination, left, ..
                    } => valid_slot(*destination) && valid_slot(*left),
                    MirOp::ArrayNew {
                        destination,
                        values,
                    } => valid_slot(*destination) && values.iter().all(|slot| valid_slot(*slot)),
                    MirOp::ArrayGetI64 {
                        destination,
                        array,
                        index,
                    } => valid_slot(*destination) && valid_slot(*array) && valid_slot(*index),
                    MirOp::ArrayGetI64Constant {
                        destination, array, ..
                    } => valid_slot(*destination) && valid_slot(*array),
                    MirOp::ArraySetI64 {
                        destination,
                        array,
                        index,
                        value,
                    } => {
                        valid_slot(*destination)
                            && valid_slot(*array)
                            && valid_slot(*index)
                            && valid_slot(*value)
                    }
                    MirOp::CallStatic {
                        destination,
                        function: target,
                        arguments,
                    } => {
                        valid_slot(*destination)
                            && program
                                .functions
                                .get(usize::from(*target))
                                .is_some_and(|callee| {
                                    usize::from(callee.arity) == arguments.len()
                                        && arguments.iter().all(|slot| valid_slot(*slot))
                                })
                    }
                };
                if !valid {
                    return Err(format!(
                        "whole-Wasm MIR function {expected} block {expected_block} has invalid operands"
                    ));
                }
            }
            let block_count = function.blocks.len();
            let valid = match block.terminator {
                MirTerminator::Goto(target) => usize::from(target) < block_count,
                MirTerminator::BranchZero {
                    condition,
                    rep: _,
                    zero,
                    nonzero,
                } => {
                    valid_slot(condition)
                        && usize::from(zero) < block_count
                        && usize::from(nonzero) < block_count
                }
                MirTerminator::Return(value) => valid_slot(value),
            };
            if !valid {
                return Err(format!(
                    "whole-Wasm MIR function {expected} block {expected_block} has an invalid terminator"
                ));
            }
        }
    }
    Ok(())
}
