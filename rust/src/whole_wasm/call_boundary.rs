use crate::vm::{FunctionId, FunctionPrototype, Program};

use super::ir::{MirOp, Rep};

pub(super) fn lower_static(
    operations: &mut Vec<MirOp>,
    program: &Program,
    representations: &[Rep],
    stack_base: u16,
    height: usize,
    prototype: FunctionId,
    argc: u8,
    caller: FunctionId,
    ip: usize,
) -> Result<(), String> {
    let target = program
        .functions
        .get(usize::from(prototype))
        .ok_or_else(|| unsupported(caller, ip, "call target"))?;
    if target.async_function
        || target.variadic
        || target.capture_count != 0
        || target.arity != u16::from(argc)
    {
        return Err(unsupported(caller, ip, "static call shape"));
    }
    lower_call(
        operations,
        program,
        representations,
        stack_base,
        height - usize::from(argc),
        height - usize::from(argc),
        prototype,
        target,
        caller,
        ip,
    )
}

pub(super) fn lower_dynamic(
    operations: &mut Vec<MirOp>,
    program: &Program,
    representations: &[Rep],
    stack_base: u16,
    height: usize,
    argc: u8,
    caller: FunctionId,
    ip: usize,
) -> Result<(), String> {
    let base = height - usize::from(argc) - 1;
    let Rep::FunctionRef(prototype) = representations[base] else {
        return Err(unsupported(caller, ip, "dynamic call target"));
    };
    let target = program
        .functions
        .get(usize::from(prototype))
        .ok_or_else(|| unsupported(caller, ip, "dynamic call target"))?;
    lower_call(
        operations,
        program,
        representations,
        stack_base,
        base,
        base + 1,
        prototype,
        target,
        caller,
        ip,
    )
}

#[allow(clippy::too_many_arguments)]
fn lower_call(
    operations: &mut Vec<MirOp>,
    program: &Program,
    representations: &[Rep],
    stack_base: u16,
    base: usize,
    argument_base: usize,
    prototype: FunctionId,
    target: &FunctionPrototype,
    caller: FunctionId,
    ip: usize,
) -> Result<(), String> {
    let arguments = (0..usize::from(target.arity))
        .map(|index| stack(stack_base, argument_base + index))
        .collect::<Result<Vec<_>, _>>()?;
    for (index, argument) in arguments.iter().copied().enumerate() {
        if let Some(expected) =
            super::reps::declared_parameter_rep(program, prototype, target, index)
        {
            coerce_representation(
                operations,
                argument,
                representations[argument_base + index],
                expected,
                caller,
                ip,
            )?;
        }
    }
    operations.push(MirOp::CallStatic {
        destination: stack(stack_base, base)?,
        function: prototype,
        arguments,
    });
    Ok(())
}

pub(super) fn lower_return(
    operations: &mut Vec<MirOp>,
    program: &Program,
    representations: &[Rep],
    stack_base: u16,
    height: usize,
    function: FunctionId,
    ip: usize,
) -> Result<u16, String> {
    let value = stack(stack_base, height - 1)?;
    let actual = representations
        .last()
        .copied()
        .ok_or_else(|| unsupported(function, ip, "return value"))?;
    if let Some(expected) = super::reps::declared_result_rep(program, function) {
        coerce_representation(operations, value, actual, expected, function, ip)?;
    } else {
        if actual == Rep::TruthyHandle {
            operations.push(MirOp::UnboxI64 {
                destination: value,
                source: value,
            });
        }
        if actual == Rep::TaggedRef {
            operations.push(MirOp::TaggedUnboxI64 {
                destination: value,
                source: value,
            });
        }
    }
    Ok(value)
}

fn coerce_representation(
    operations: &mut Vec<MirOp>,
    slot: u16,
    actual: Rep,
    expected: Rep,
    function: FunctionId,
    ip: usize,
) -> Result<(), String> {
    if actual == expected
        || expected == Rep::Unknown
        || matches!(
            (actual, expected),
            (Rep::Bool, Rep::I64) | (Rep::I64, Rep::Bool)
        )
    {
        return Ok(());
    }
    match (actual, expected) {
        (Rep::I64, Rep::TruthyHandle) => operations.push(MirOp::BoxI64 {
            destination: slot,
            source: slot,
        }),
        (Rep::TruthyHandle, Rep::I64) => operations.push(MirOp::UnboxI64 {
            destination: slot,
            source: slot,
        }),
        (Rep::TaggedRef, Rep::I64) => operations.push(MirOp::TaggedUnboxI64 {
            destination: slot,
            source: slot,
        }),
        _ => {
            return Err(unsupported(
                function,
                ip,
                &format!("cannot coerce {actual:?} to {expected:?}"),
            ));
        }
    }
    Ok(())
}

fn stack(stack_base: u16, offset: usize) -> Result<u16, String> {
    stack_base
        .checked_add(u16::try_from(offset).map_err(|_| "operand stack exceeds u16")?)
        .ok_or_else(|| "whole-Wasm slot index overflow".to_string())
}

fn unsupported(function: FunctionId, ip: usize, detail: &str) -> String {
    format!("whole-Wasm function {function} instruction {ip} unsupported: {detail}")
}
