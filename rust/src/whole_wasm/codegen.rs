use wasm_encoder::{
    BlockType, CodeSection, ConstExpr, EntityType, ExportKind, ExportSection, Function,
    FunctionSection, GlobalSection, GlobalType, ImportSection, Instruction, Module, TypeSection,
    ValType,
};

use crate::core::Primitive;

use super::ir::{lower_program, MirFunction, MirOp, MirProgram, MirTerminator};
use crate::vm::Program;

/// Error codes published through the `hara_error` Wasm global before a trap.
pub const ERROR_INTEGER_OVERFLOW: i32 = 1;
pub const ERROR_DIVISION_BY_ZERO: i32 = 2;
const HOST_FUNCTION_COUNT: u32 = 4;
const HOST_ARRAY_EMPTY: u32 = 0;
const HOST_ARRAY_PUSH_I64: u32 = 1;
const HOST_ARRAY_GET_I64: u32 = 2;
const HOST_ARRAY_SET_I64: u32 = 3;

/// Compiles a complete eligible bytecode program into deterministic Wasm.
pub fn compile_program(program: &Program) -> Result<Vec<u8>, String> {
    emit_program(&lower_program(program)?)
}

pub(crate) fn emit_program(program: &MirProgram) -> Result<Vec<u8>, String> {
    super::ir::verify(program)?;
    let mut module = Module::new();
    let mut types = TypeSection::new();
    let mut functions = FunctionSection::new();
    types.function([], [ValType::I64]);
    types.function([ValType::I64, ValType::I64], [ValType::I64]);
    types.function([ValType::I64, ValType::I64], [ValType::I64]);
    types.function([ValType::I64, ValType::I64, ValType::I64], [ValType::I64]);
    for function in &program.functions {
        types.function(
            std::iter::repeat(ValType::I64).take(usize::from(function.arity)),
            [ValType::I64],
        );
        functions.function(HOST_FUNCTION_COUNT + u32::from(function.id));
    }
    module.section(&types);
    let mut imports = ImportSection::new();
    imports.import(
        "hara",
        "array_empty",
        EntityType::Function(HOST_ARRAY_EMPTY),
    );
    imports.import(
        "hara",
        "array_push_i64",
        EntityType::Function(HOST_ARRAY_PUSH_I64),
    );
    imports.import(
        "hara",
        "array_get_i64",
        EntityType::Function(HOST_ARRAY_GET_I64),
    );
    imports.import(
        "hara",
        "array_set_i64",
        EntityType::Function(HOST_ARRAY_SET_I64),
    );
    module.section(&imports);
    module.section(&functions);

    let mut globals = GlobalSection::new();
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
        },
        &ConstExpr::i32_const(0),
    );
    module.section(&globals);

    let mut exports = ExportSection::new();
    for function in &program.functions {
        exports.export(
            &format!("hara_fn_{}", function.id),
            ExportKind::Func,
            HOST_FUNCTION_COUNT + u32::from(function.id),
        );
    }
    exports.export("hara_error", ExportKind::Global, 0);
    module.section(&exports);

    let mut code = CodeSection::new();
    for function in &program.functions {
        code.function(&emit_function(function)?);
    }
    module.section(&code);
    Ok(module.finish())
}

fn emit_function(mir: &MirFunction) -> Result<Function, String> {
    let slots = mir
        .local_count
        .checked_add(mir.stack_count)
        .ok_or("whole-Wasm local count overflow")?;
    if slots < mir.arity {
        return Err(format!("whole-Wasm function {} has too few slots", mir.id));
    }
    let temp_a = u32::from(slots);
    let temp_b = temp_a + 1;
    let result = temp_a + 2;
    let pc = temp_a + 3;
    let scalar_locals = u32::from(slots - mir.arity) + 3;
    let mut declarations = Vec::new();
    if scalar_locals != 0 {
        declarations.push((scalar_locals, ValType::I64));
    }
    declarations.push((1, ValType::I32));
    let mut out = Function::new(declarations);
    out.instruction(&Instruction::I32Const(0));
    out.instruction(&Instruction::LocalSet(pc));
    out.instruction(&Instruction::Loop(BlockType::Empty));
    for block in &mir.blocks {
        out.instruction(&Instruction::LocalGet(pc));
        out.instruction(&Instruction::I32Const(i32::from(block.id)));
        out.instruction(&Instruction::I32Eq);
        out.instruction(&Instruction::If(BlockType::Empty));
        for operation in &block.operations {
            emit_operation(&mut out, operation, temp_a, temp_b, result)?;
        }
        emit_terminator(&mut out, &block.terminator, pc);
        out.instruction(&Instruction::End);
    }
    out.instruction(&Instruction::Unreachable);
    out.instruction(&Instruction::End);
    out.instruction(&Instruction::Unreachable);
    out.instruction(&Instruction::End);
    Ok(out)
}

fn emit_operation(
    out: &mut Function,
    operation: &MirOp,
    temp_a: u32,
    temp_b: u32,
    result: u32,
) -> Result<(), String> {
    match operation {
        MirOp::Constant {
            destination, value, ..
        } => {
            out.instruction(&Instruction::I64Const(*value));
            out.instruction(&Instruction::LocalSet(u32::from(*destination)));
        }
        MirOp::Move {
            destination,
            source,
        } => {
            out.instruction(&Instruction::LocalGet(u32::from(*source)));
            out.instruction(&Instruction::LocalSet(u32::from(*destination)));
        }
        MirOp::Binary {
            destination,
            left,
            right,
            op,
        } => emit_binary(
            out,
            *destination,
            |out| out.instruction(&Instruction::LocalGet(u32::from(*left))),
            |out| out.instruction(&Instruction::LocalGet(u32::from(*right))),
            *op,
            temp_a,
            temp_b,
            result,
        )?,
        MirOp::BinaryConstant {
            destination,
            left,
            right,
            op,
        } => emit_binary(
            out,
            *destination,
            |out| out.instruction(&Instruction::LocalGet(u32::from(*left))),
            |out| out.instruction(&Instruction::I64Const(*right)),
            *op,
            temp_a,
            temp_b,
            result,
        )?,
        MirOp::ArrayNew {
            destination,
            values,
        } => {
            out.instruction(&Instruction::Call(HOST_ARRAY_EMPTY));
            // Keep the handle in a temporary until every constructor operand
            // has been consumed: destination aliases the first operand stack
            // slot in stack-machine lowering.
            out.instruction(&Instruction::LocalSet(result));
            for value in values {
                out.instruction(&Instruction::LocalGet(result));
                out.instruction(&Instruction::LocalGet(u32::from(*value)));
                out.instruction(&Instruction::Call(HOST_ARRAY_PUSH_I64));
                out.instruction(&Instruction::LocalSet(result));
            }
            out.instruction(&Instruction::LocalGet(result));
            out.instruction(&Instruction::LocalSet(u32::from(*destination)));
        }
        MirOp::ArrayGetI64 {
            destination,
            array,
            index,
        } => {
            out.instruction(&Instruction::LocalGet(u32::from(*array)));
            out.instruction(&Instruction::LocalGet(u32::from(*index)));
            out.instruction(&Instruction::Call(HOST_ARRAY_GET_I64));
            out.instruction(&Instruction::LocalSet(u32::from(*destination)));
        }
        MirOp::ArrayGetI64Constant {
            destination,
            array,
            index,
        } => {
            out.instruction(&Instruction::LocalGet(u32::from(*array)));
            out.instruction(&Instruction::I64Const(*index));
            out.instruction(&Instruction::Call(HOST_ARRAY_GET_I64));
            out.instruction(&Instruction::LocalSet(u32::from(*destination)));
        }
        MirOp::ArraySetI64 {
            destination,
            array,
            index,
            value,
        } => {
            out.instruction(&Instruction::LocalGet(u32::from(*array)));
            out.instruction(&Instruction::LocalGet(u32::from(*index)));
            out.instruction(&Instruction::LocalGet(u32::from(*value)));
            out.instruction(&Instruction::Call(HOST_ARRAY_SET_I64));
            out.instruction(&Instruction::LocalSet(u32::from(*destination)));
        }
        MirOp::CallStatic {
            destination,
            function,
            arguments,
        } => {
            for argument in arguments {
                out.instruction(&Instruction::LocalGet(u32::from(*argument)));
            }
            out.instruction(&Instruction::Call(
                HOST_FUNCTION_COUNT + u32::from(*function),
            ));
            out.instruction(&Instruction::LocalSet(u32::from(*destination)));
        }
    }
    Ok(())
}

fn emit_binary<L, R>(
    out: &mut Function,
    destination: u16,
    left: L,
    right: R,
    op: Primitive,
    a: u32,
    b: u32,
    result: u32,
) -> Result<(), String>
where
    L: Fn(&mut Function) -> &mut Function,
    R: Fn(&mut Function) -> &mut Function,
{
    left(out);
    out.instruction(&Instruction::LocalSet(a));
    right(out);
    out.instruction(&Instruction::LocalSet(b));
    match op {
        Primitive::Add | Primitive::Subtract | Primitive::Multiply => {
            out.instruction(&Instruction::LocalGet(a));
            out.instruction(&Instruction::LocalGet(b));
            out.instruction(&match op {
                Primitive::Add => Instruction::I64Add,
                Primitive::Subtract => Instruction::I64Sub,
                Primitive::Multiply => Instruction::I64Mul,
                _ => unreachable!(),
            });
            out.instruction(&Instruction::LocalSet(result));
            emit_overflow_check(out, op, a, b, result);
            out.instruction(&Instruction::LocalGet(result));
        }
        Primitive::Divide | Primitive::Remainder => {
            out.instruction(&Instruction::LocalGet(b));
            out.instruction(&Instruction::I64Eqz);
            out.instruction(&Instruction::If(BlockType::Empty));
            emit_error(out, ERROR_DIVISION_BY_ZERO);
            out.instruction(&Instruction::End);
            out.instruction(&Instruction::LocalGet(a));
            out.instruction(&Instruction::I64Const(i64::MIN));
            out.instruction(&Instruction::I64Eq);
            out.instruction(&Instruction::LocalGet(b));
            out.instruction(&Instruction::I64Const(-1));
            out.instruction(&Instruction::I64Eq);
            out.instruction(&Instruction::I32And);
            out.instruction(&Instruction::If(BlockType::Empty));
            emit_error(out, ERROR_INTEGER_OVERFLOW);
            out.instruction(&Instruction::End);
            out.instruction(&Instruction::LocalGet(a));
            out.instruction(&Instruction::LocalGet(b));
            out.instruction(&if op == Primitive::Divide {
                Instruction::I64DivS
            } else {
                Instruction::I64RemS
            });
        }
        Primitive::Equal
        | Primitive::Less
        | Primitive::LessOrEqual
        | Primitive::Greater
        | Primitive::GreaterOrEqual => {
            out.instruction(&Instruction::LocalGet(a));
            out.instruction(&Instruction::LocalGet(b));
            out.instruction(&match op {
                Primitive::Equal => Instruction::I64Eq,
                Primitive::Less => Instruction::I64LtS,
                Primitive::LessOrEqual => Instruction::I64LeS,
                Primitive::Greater => Instruction::I64GtS,
                Primitive::GreaterOrEqual => Instruction::I64GeS,
                _ => unreachable!(),
            });
            out.instruction(&Instruction::I64ExtendI32U);
        }
        _ => return Err(format!("whole-Wasm cannot emit primitive {op:?}")),
    }
    out.instruction(&Instruction::LocalSet(u32::from(destination)));
    Ok(())
}

fn emit_overflow_check(out: &mut Function, op: Primitive, a: u32, b: u32, result: u32) {
    match op {
        Primitive::Add => {
            out.instruction(&Instruction::LocalGet(a));
            out.instruction(&Instruction::LocalGet(result));
            out.instruction(&Instruction::I64Xor);
            out.instruction(&Instruction::LocalGet(b));
            out.instruction(&Instruction::LocalGet(result));
            out.instruction(&Instruction::I64Xor);
            out.instruction(&Instruction::I64And);
            out.instruction(&Instruction::I64Const(0));
            out.instruction(&Instruction::I64LtS);
        }
        Primitive::Subtract => {
            out.instruction(&Instruction::LocalGet(a));
            out.instruction(&Instruction::LocalGet(b));
            out.instruction(&Instruction::I64Xor);
            out.instruction(&Instruction::LocalGet(a));
            out.instruction(&Instruction::LocalGet(result));
            out.instruction(&Instruction::I64Xor);
            out.instruction(&Instruction::I64And);
            out.instruction(&Instruction::I64Const(0));
            out.instruction(&Instruction::I64LtS);
        }
        Primitive::Multiply => {
            out.instruction(&Instruction::LocalGet(b));
            out.instruction(&Instruction::I64Eqz);
            out.instruction(&Instruction::I32Eqz);
            out.instruction(&Instruction::If(BlockType::Result(ValType::I32)));
            out.instruction(&Instruction::LocalGet(a));
            out.instruction(&Instruction::I64Const(i64::MIN));
            out.instruction(&Instruction::I64Eq);
            out.instruction(&Instruction::LocalGet(b));
            out.instruction(&Instruction::I64Const(-1));
            out.instruction(&Instruction::I64Eq);
            out.instruction(&Instruction::I32And);
            out.instruction(&Instruction::If(BlockType::Result(ValType::I32)));
            out.instruction(&Instruction::I32Const(1));
            out.instruction(&Instruction::Else);
            out.instruction(&Instruction::LocalGet(result));
            out.instruction(&Instruction::LocalGet(b));
            out.instruction(&Instruction::I64DivS);
            out.instruction(&Instruction::LocalGet(a));
            out.instruction(&Instruction::I64Ne);
            out.instruction(&Instruction::End);
            out.instruction(&Instruction::Else);
            out.instruction(&Instruction::I32Const(0));
            out.instruction(&Instruction::End);
        }
        _ => unreachable!(),
    }
    out.instruction(&Instruction::If(BlockType::Empty));
    emit_error(out, ERROR_INTEGER_OVERFLOW);
    out.instruction(&Instruction::End);
}

fn emit_error(out: &mut Function, code: i32) {
    out.instruction(&Instruction::I32Const(code));
    out.instruction(&Instruction::GlobalSet(0));
    out.instruction(&Instruction::Unreachable);
}

fn emit_terminator(out: &mut Function, terminator: &MirTerminator, pc: u32) {
    match terminator {
        MirTerminator::Goto(target) => {
            out.instruction(&Instruction::I32Const(i32::from(*target)));
            out.instruction(&Instruction::LocalSet(pc));
            out.instruction(&Instruction::Br(1));
        }
        MirTerminator::BranchZero {
            condition,
            rep,
            zero,
            nonzero,
        } => {
            match rep {
                super::ir::Rep::Bool => {
                    out.instruction(&Instruction::LocalGet(u32::from(*condition)));
                    out.instruction(&Instruction::I64Eqz);
                }
                super::ir::Rep::I64 | super::ir::Rep::TruthyHandle => {
                    out.instruction(&Instruction::I32Const(0));
                }
                super::ir::Rep::Unknown => unreachable!("unknown truthiness rejected by MIR"),
            }
            out.instruction(&Instruction::If(BlockType::Result(ValType::I32)));
            out.instruction(&Instruction::I32Const(i32::from(*zero)));
            out.instruction(&Instruction::Else);
            out.instruction(&Instruction::I32Const(i32::from(*nonzero)));
            out.instruction(&Instruction::End);
            out.instruction(&Instruction::LocalSet(pc));
            out.instruction(&Instruction::Br(1));
        }
        MirTerminator::Return(value) => {
            out.instruction(&Instruction::LocalGet(u32::from(*value)));
            out.instruction(&Instruction::Return);
        }
    }
}
