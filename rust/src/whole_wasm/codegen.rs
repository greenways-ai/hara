use wasm_encoder::{
    BlockType, CodeSection, ConstExpr, EntityType, ExportKind, ExportSection, Function,
    FunctionSection, GlobalSection, GlobalType, ImportSection, Instruction, MemArg, MemorySection,
    MemoryType, Module, TypeSection, ValType,
};

use crate::core::Primitive;

use super::ir::{lower_program, MirFunction, MirOp, MirProgram, MirTerminator};
use crate::vm::Program;

/// Error codes published through the `hara_error` Wasm global before a trap.
pub const ERROR_INTEGER_OVERFLOW: i32 = 1;
pub const ERROR_DIVISION_BY_ZERO: i32 = 2;
pub const ERROR_ARRAY_BOUNDS: i32 = 3;
pub const ERROR_OBJECT_KEY: i32 = 4;
const HOST_TYPE_COUNT: u32 = 5;
const HOST_FUNCTION_COUNT: u32 = 15;
const HOST_CONSTANT: u32 = 0;
const HOST_BOX_I64: u32 = 1;
const HOST_UNBOX_I64: u32 = 2;
const HOST_VECTOR_EMPTY: u32 = 3;
const HOST_VECTOR_PUSH: u32 = 4;
const HOST_MAP_EMPTY: u32 = 5;
const HOST_MAP_ASSOC: u32 = 6;
const HOST_GET: u32 = 7;
const HOST_IS_NUMBER: u32 = 8;
const HOST_COUNT: u32 = 9;
const HOST_NTH: u32 = 10;
const HOST_MAP_I64_PAIR: u32 = 11;
const HOST_GET_I64: u32 = 12;
const HOST_GET_PATH_I64_CONSTANTS: u32 = 13;
const HOST_ASSOC_MAP_I64_PAIR: u32 = 14;
const ARRAY_MEMORY: u32 = 0;
const ARRAY_HEAP_GLOBAL: u32 = 1;
const I64_MEMORY: MemArg = MemArg {
    offset: 0,
    align: 3,
    memory_index: ARRAY_MEMORY,
};

/// Compiles a complete eligible bytecode program into deterministic Wasm.
pub fn compile_program(program: &Program) -> Result<Vec<u8>, String> {
    emit_program(&lower_program(program)?)
}

pub(crate) fn emit_program(program: &MirProgram) -> Result<Vec<u8>, String> {
    super::ir::verify(program)?;
    let mut module = Module::new();
    let mut types = TypeSection::new();
    let mut functions = FunctionSection::new();
    types.function([ValType::I64], [ValType::I64]);
    types.function([], [ValType::I64]);
    types.function([ValType::I64, ValType::I64], [ValType::I64]);
    types.function([ValType::I64, ValType::I64, ValType::I64], [ValType::I64]);
    types.function(
        [ValType::I64, ValType::I64, ValType::I64, ValType::I64],
        [ValType::I64],
    );
    for function in &program.functions {
        types.function(
            std::iter::repeat(ValType::I64).take(usize::from(function.arity)),
            [ValType::I64],
        );
        functions.function(HOST_TYPE_COUNT + u32::from(function.id));
    }
    module.section(&types);
    let mut imports = ImportSection::new();
    for (name, ty) in [
        ("constant_handle", 0),
        ("box_i64", 0),
        ("unbox_i64", 0),
        ("vector_empty", 1),
        ("vector_push", 2),
        ("map_empty", 1),
        ("map_assoc", 3),
        ("get", 2),
        ("is_number", 0),
        ("count", 0),
        ("nth", 2),
        ("map_i64_pair", 2),
        ("get_i64", 2),
        ("get_path_i64_constants", 3),
        ("assoc_map_i64_pair", 4),
    ] {
        imports.import("hara", name, EntityType::Function(ty));
    }
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
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
        },
        &ConstExpr::i32_const(0),
    );
    let mut memories = MemorySection::new();
    memories.memory(MemoryType {
        minimum: 1,
        maximum: Some(1),
        memory64: false,
        shared: false,
    });
    module.section(&memories);
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
    exports.export("hara_heap", ExportKind::Global, ARRAY_HEAP_GLOBAL);
    exports.export("hara_memory", ExportKind::Memory, ARRAY_MEMORY);
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
        MirOp::ConstantHandle {
            destination,
            constant,
        } => {
            out.instruction(&Instruction::I64Const(i64::from(*constant)));
            out.instruction(&Instruction::Call(HOST_CONSTANT));
            out.instruction(&Instruction::LocalSet(u32::from(*destination)));
        }
        MirOp::BoxI64 {
            destination,
            source,
        } => {
            out.instruction(&Instruction::LocalGet(u32::from(*source)));
            out.instruction(&Instruction::Call(HOST_BOX_I64));
            out.instruction(&Instruction::LocalSet(u32::from(*destination)));
        }
        MirOp::UnboxI64 {
            destination,
            source,
        } => {
            out.instruction(&Instruction::LocalGet(u32::from(*source)));
            out.instruction(&Instruction::Call(HOST_UNBOX_I64));
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
            let bytes = (values.len() + 1)
                .checked_mul(8)
                .and_then(|value| i32::try_from(value).ok())
                .ok_or("whole-Wasm array allocation is too large")?;
            out.instruction(&Instruction::GlobalGet(ARRAY_HEAP_GLOBAL));
            out.instruction(&Instruction::I64ExtendI32U);
            out.instruction(&Instruction::LocalSet(result));
            out.instruction(&Instruction::LocalGet(result));
            out.instruction(&Instruction::I32WrapI64);
            out.instruction(&Instruction::I64Const(values.len() as i64));
            out.instruction(&Instruction::I64Store(I64_MEMORY));
            for (index, value) in values.iter().enumerate() {
                out.instruction(&Instruction::LocalGet(result));
                out.instruction(&Instruction::I32WrapI64);
                out.instruction(&Instruction::LocalGet(u32::from(*value)));
                out.instruction(&Instruction::I64Store(MemArg {
                    offset: ((index + 1) * 8) as u64,
                    ..I64_MEMORY
                }));
            }
            out.instruction(&Instruction::GlobalGet(ARRAY_HEAP_GLOBAL));
            out.instruction(&Instruction::I32Const(bytes));
            out.instruction(&Instruction::I32Add);
            out.instruction(&Instruction::GlobalSet(ARRAY_HEAP_GLOBAL));
            out.instruction(&Instruction::LocalGet(result));
            out.instruction(&Instruction::LocalSet(u32::from(*destination)));
        }
        MirOp::ArrayGetI64 {
            destination,
            array,
            index,
        } => {
            emit_array_address(out, *array, |out| {
                out.instruction(&Instruction::LocalGet(u32::from(*index)))
            });
            out.instruction(&Instruction::I64Load(I64_MEMORY));
            out.instruction(&Instruction::LocalSet(u32::from(*destination)));
        }
        MirOp::ArrayGetI64Constant {
            destination,
            array,
            index,
        } => {
            emit_array_address(out, *array, |out| {
                out.instruction(&Instruction::I64Const(*index))
            });
            out.instruction(&Instruction::I64Load(I64_MEMORY));
            out.instruction(&Instruction::LocalSet(u32::from(*destination)));
        }
        MirOp::ArraySetI64 {
            destination,
            array,
            index,
            value,
        } => {
            emit_array_address(out, *array, |out| {
                out.instruction(&Instruction::LocalGet(u32::from(*index)))
            });
            out.instruction(&Instruction::LocalGet(u32::from(*value)));
            out.instruction(&Instruction::I64Store(I64_MEMORY));
            out.instruction(&Instruction::LocalGet(u32::from(*array)));
            out.instruction(&Instruction::LocalSet(u32::from(*destination)));
        }
        MirOp::ObjectNew {
            destination,
            entries,
        } => {
            let bytes = entries
                .len()
                .checked_mul(16)
                .and_then(|value| value.checked_add(8))
                .and_then(|value| i32::try_from(value).ok())
                .ok_or("whole-Wasm object allocation is too large")?;
            out.instruction(&Instruction::GlobalGet(ARRAY_HEAP_GLOBAL));
            out.instruction(&Instruction::I64ExtendI32U);
            out.instruction(&Instruction::LocalSet(result));
            out.instruction(&Instruction::LocalGet(result));
            out.instruction(&Instruction::I32WrapI64);
            out.instruction(&Instruction::I64Const(entries.len() as i64));
            out.instruction(&Instruction::I64Store(I64_MEMORY));
            for (index, (key, value)) in entries.iter().enumerate() {
                let key_offset = 8 + index * 16;
                out.instruction(&Instruction::LocalGet(result));
                out.instruction(&Instruction::I32WrapI64);
                out.instruction(&Instruction::LocalGet(u32::from(*key)));
                out.instruction(&Instruction::I64Store(MemArg {
                    offset: key_offset as u64,
                    ..I64_MEMORY
                }));
                out.instruction(&Instruction::LocalGet(result));
                out.instruction(&Instruction::I32WrapI64);
                out.instruction(&Instruction::LocalGet(u32::from(*value)));
                out.instruction(&Instruction::I64Store(MemArg {
                    offset: (key_offset + 8) as u64,
                    ..I64_MEMORY
                }));
            }
            out.instruction(&Instruction::GlobalGet(ARRAY_HEAP_GLOBAL));
            out.instruction(&Instruction::I32Const(bytes));
            out.instruction(&Instruction::I32Add);
            out.instruction(&Instruction::GlobalSet(ARRAY_HEAP_GLOBAL));
            out.instruction(&Instruction::LocalGet(result));
            out.instruction(&Instruction::LocalSet(u32::from(*destination)));
        }
        MirOp::ObjectGetI64 {
            destination,
            object,
            key,
        } => {
            emit_object_value_address(out, *object, *key, temp_a, result);
            out.instruction(&Instruction::I64Load(I64_MEMORY));
            out.instruction(&Instruction::LocalSet(u32::from(*destination)));
        }
        MirOp::ObjectSetI64 {
            destination,
            object,
            key,
            value,
        } => {
            emit_object_value_address(out, *object, *key, temp_a, result);
            out.instruction(&Instruction::LocalGet(u32::from(*value)));
            out.instruction(&Instruction::I64Store(I64_MEMORY));
            out.instruction(&Instruction::LocalGet(u32::from(*object)));
            out.instruction(&Instruction::LocalSet(u32::from(*destination)));
        }
        MirOp::BuildVector {
            destination,
            values,
        } => {
            out.instruction(&Instruction::Call(HOST_VECTOR_EMPTY));
            out.instruction(&Instruction::LocalSet(result));
            for value in values {
                out.instruction(&Instruction::LocalGet(result));
                out.instruction(&Instruction::LocalGet(u32::from(*value)));
                out.instruction(&Instruction::Call(HOST_VECTOR_PUSH));
                out.instruction(&Instruction::LocalSet(result));
            }
            out.instruction(&Instruction::LocalGet(result));
            out.instruction(&Instruction::LocalSet(u32::from(*destination)));
        }
        MirOp::NativeVector {
            destination,
            values,
        } => {
            let bytes = values
                .len()
                .checked_mul(16)
                .and_then(|value| value.checked_add(24))
                .and_then(|value| i32::try_from(value).ok())
                .ok_or("whole-Wasm tagged vector allocation is too large")?;
            out.instruction(&Instruction::GlobalGet(ARRAY_HEAP_GLOBAL));
            out.instruction(&Instruction::I64ExtendI32U);
            out.instruction(&Instruction::LocalSet(result));
            out.instruction(&Instruction::LocalGet(result));
            out.instruction(&Instruction::I32WrapI64);
            out.instruction(&Instruction::I64Const(1));
            out.instruction(&Instruction::I64Store(I64_MEMORY));
            out.instruction(&Instruction::LocalGet(result));
            out.instruction(&Instruction::I32WrapI64);
            out.instruction(&Instruction::LocalGet(result));
            out.instruction(&Instruction::I64Const(16));
            out.instruction(&Instruction::I64Add);
            out.instruction(&Instruction::I64Store(MemArg {
                offset: 8,
                ..I64_MEMORY
            }));
            out.instruction(&Instruction::LocalGet(result));
            out.instruction(&Instruction::I32WrapI64);
            out.instruction(&Instruction::I64Const(values.len() as i64));
            out.instruction(&Instruction::I64Store(MemArg {
                offset: 16,
                ..I64_MEMORY
            }));
            for (index, (value, rep)) in values.iter().enumerate() {
                let tag_offset = 24 + index * 16;
                let payload_offset = tag_offset + 8;
                out.instruction(&Instruction::LocalGet(result));
                out.instruction(&Instruction::I32WrapI64);
                match rep {
                    super::ir::Rep::I64 => out.instruction(&Instruction::I64Const(0)),
                    super::ir::Rep::TaggedRef => {
                        out.instruction(&Instruction::LocalGet(u32::from(*value)));
                        out.instruction(&Instruction::I32WrapI64);
                        out.instruction(&Instruction::I64Load(I64_MEMORY))
                    }
                    _ => unreachable!("native vector reps verified by MIR"),
                };
                out.instruction(&Instruction::I64Store(MemArg {
                    offset: tag_offset as u64,
                    ..I64_MEMORY
                }));
                out.instruction(&Instruction::LocalGet(result));
                out.instruction(&Instruction::I32WrapI64);
                match rep {
                    super::ir::Rep::I64 => {
                        out.instruction(&Instruction::LocalGet(u32::from(*value)))
                    }
                    super::ir::Rep::TaggedRef => {
                        out.instruction(&Instruction::LocalGet(u32::from(*value)));
                        out.instruction(&Instruction::I32WrapI64);
                        out.instruction(&Instruction::I64Load(MemArg {
                            offset: 8,
                            ..I64_MEMORY
                        }))
                    }
                    _ => unreachable!("native vector reps verified by MIR"),
                };
                out.instruction(&Instruction::I64Store(MemArg {
                    offset: payload_offset as u64,
                    ..I64_MEMORY
                }));
            }
            out.instruction(&Instruction::GlobalGet(ARRAY_HEAP_GLOBAL));
            out.instruction(&Instruction::I32Const(bytes));
            out.instruction(&Instruction::I32Add);
            out.instruction(&Instruction::GlobalSet(ARRAY_HEAP_GLOBAL));
            out.instruction(&Instruction::LocalGet(result));
            out.instruction(&Instruction::LocalSet(u32::from(*destination)));
        }
        MirOp::BuildMap {
            destination,
            entries,
        } => {
            out.instruction(&Instruction::Call(HOST_MAP_EMPTY));
            out.instruction(&Instruction::LocalSet(result));
            for (key, value) in entries {
                out.instruction(&Instruction::LocalGet(result));
                out.instruction(&Instruction::LocalGet(u32::from(*key)));
                out.instruction(&Instruction::LocalGet(u32::from(*value)));
                out.instruction(&Instruction::Call(HOST_MAP_ASSOC));
                out.instruction(&Instruction::LocalSet(result));
            }
            out.instruction(&Instruction::LocalGet(result));
            out.instruction(&Instruction::LocalSet(u32::from(*destination)));
        }
        MirOp::BuildMapI64Pair {
            destination,
            key,
            value,
        } => {
            out.instruction(&Instruction::LocalGet(u32::from(*key)));
            out.instruction(&Instruction::LocalGet(u32::from(*value)));
            out.instruction(&Instruction::Call(HOST_MAP_I64_PAIR));
            out.instruction(&Instruction::LocalSet(u32::from(*destination)));
        }
        MirOp::Assoc {
            destination,
            collection,
            key,
            value,
        } => {
            out.instruction(&Instruction::LocalGet(u32::from(*collection)));
            out.instruction(&Instruction::LocalGet(u32::from(*key)));
            out.instruction(&Instruction::LocalGet(u32::from(*value)));
            out.instruction(&Instruction::Call(HOST_MAP_ASSOC));
            out.instruction(&Instruction::LocalSet(u32::from(*destination)));
        }
        MirOp::AssocMapI64Pair {
            destination,
            collection,
            outer_key,
            inner_key,
            value,
        } => {
            out.instruction(&Instruction::LocalGet(u32::from(*collection)));
            out.instruction(&Instruction::LocalGet(u32::from(*outer_key)));
            out.instruction(&Instruction::LocalGet(u32::from(*inner_key)));
            out.instruction(&Instruction::LocalGet(u32::from(*value)));
            out.instruction(&Instruction::Call(HOST_ASSOC_MAP_I64_PAIR));
            out.instruction(&Instruction::LocalSet(u32::from(*destination)));
        }
        MirOp::Get {
            destination,
            collection,
            key,
        } => {
            out.instruction(&Instruction::LocalGet(u32::from(*collection)));
            out.instruction(&Instruction::LocalGet(u32::from(*key)));
            out.instruction(&Instruction::Call(HOST_GET));
            out.instruction(&Instruction::LocalSet(u32::from(*destination)));
        }
        MirOp::GetI64 {
            destination,
            collection,
            key,
        } => {
            out.instruction(&Instruction::LocalGet(u32::from(*collection)));
            out.instruction(&Instruction::LocalGet(u32::from(*key)));
            out.instruction(&Instruction::Call(HOST_GET_I64));
            out.instruction(&Instruction::LocalSet(u32::from(*destination)));
        }
        MirOp::GetPathI64Constants {
            destination,
            collection,
            first_key,
            second_key,
        } => {
            out.instruction(&Instruction::LocalGet(u32::from(*collection)));
            out.instruction(&Instruction::I64Const(i64::from(*first_key)));
            out.instruction(&Instruction::I64Const(i64::from(*second_key)));
            out.instruction(&Instruction::Call(HOST_GET_PATH_I64_CONSTANTS));
            out.instruction(&Instruction::LocalSet(u32::from(*destination)));
        }
        MirOp::IsNumber { destination, value } => {
            out.instruction(&Instruction::LocalGet(u32::from(*value)));
            out.instruction(&Instruction::Call(HOST_IS_NUMBER));
            out.instruction(&Instruction::LocalSet(u32::from(*destination)));
        }
        MirOp::TaggedIsNumber { destination, value } => {
            out.instruction(&Instruction::LocalGet(u32::from(*value)));
            out.instruction(&Instruction::I32WrapI64);
            out.instruction(&Instruction::I64Load(I64_MEMORY));
            out.instruction(&Instruction::I64Eqz);
            out.instruction(&Instruction::I64ExtendI32U);
            out.instruction(&Instruction::LocalSet(u32::from(*destination)));
        }
        MirOp::Count {
            destination,
            collection,
        } => {
            out.instruction(&Instruction::LocalGet(u32::from(*collection)));
            out.instruction(&Instruction::Call(HOST_COUNT));
            out.instruction(&Instruction::LocalSet(u32::from(*destination)));
        }
        MirOp::TaggedCount {
            destination,
            collection,
        } => {
            out.instruction(&Instruction::LocalGet(u32::from(*collection)));
            out.instruction(&Instruction::I32WrapI64);
            out.instruction(&Instruction::I64Load(MemArg {
                offset: 8,
                ..I64_MEMORY
            }));
            out.instruction(&Instruction::I32WrapI64);
            out.instruction(&Instruction::I64Load(I64_MEMORY));
            out.instruction(&Instruction::LocalSet(u32::from(*destination)));
        }
        MirOp::Nth {
            destination,
            collection,
            index,
        } => {
            out.instruction(&Instruction::LocalGet(u32::from(*collection)));
            out.instruction(&Instruction::LocalGet(u32::from(*index)));
            out.instruction(&Instruction::Call(HOST_NTH));
            out.instruction(&Instruction::LocalSet(u32::from(*destination)));
        }
        MirOp::TaggedNth {
            destination,
            collection,
            index,
        } => {
            out.instruction(&Instruction::LocalGet(u32::from(*collection)));
            out.instruction(&Instruction::I32WrapI64);
            out.instruction(&Instruction::I64Load(MemArg {
                offset: 8,
                ..I64_MEMORY
            }));
            out.instruction(&Instruction::LocalSet(temp_a));
            out.instruction(&Instruction::LocalGet(u32::from(*index)));
            out.instruction(&Instruction::I64Const(0));
            out.instruction(&Instruction::I64LtS);
            out.instruction(&Instruction::If(BlockType::Empty));
            emit_error(out, ERROR_ARRAY_BOUNDS);
            out.instruction(&Instruction::End);
            out.instruction(&Instruction::LocalGet(u32::from(*index)));
            out.instruction(&Instruction::LocalGet(temp_a));
            out.instruction(&Instruction::I32WrapI64);
            out.instruction(&Instruction::I64Load(I64_MEMORY));
            out.instruction(&Instruction::I64GeU);
            out.instruction(&Instruction::If(BlockType::Empty));
            emit_error(out, ERROR_ARRAY_BOUNDS);
            out.instruction(&Instruction::End);
            out.instruction(&Instruction::LocalGet(temp_a));
            out.instruction(&Instruction::LocalGet(u32::from(*index)));
            out.instruction(&Instruction::I64Const(16));
            out.instruction(&Instruction::I64Mul);
            out.instruction(&Instruction::I64Add);
            out.instruction(&Instruction::I64Const(8));
            out.instruction(&Instruction::I64Add);
            out.instruction(&Instruction::LocalSet(u32::from(*destination)));
        }
        MirOp::TaggedUnboxI64 {
            destination,
            source,
        } => {
            out.instruction(&Instruction::LocalGet(u32::from(*source)));
            out.instruction(&Instruction::I32WrapI64);
            out.instruction(&Instruction::I64Load(MemArg {
                offset: 8,
                ..I64_MEMORY
            }));
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

fn emit_array_address<I>(out: &mut Function, array: u16, index: I)
where
    I: Fn(&mut Function) -> &mut Function,
{
    index(out);
    out.instruction(&Instruction::I64Const(0));
    out.instruction(&Instruction::I64LtS);
    out.instruction(&Instruction::If(BlockType::Empty));
    emit_error(out, ERROR_ARRAY_BOUNDS);
    out.instruction(&Instruction::End);

    index(out);
    out.instruction(&Instruction::LocalGet(u32::from(array)));
    out.instruction(&Instruction::I32WrapI64);
    out.instruction(&Instruction::I64Load(I64_MEMORY));
    out.instruction(&Instruction::I64GeU);
    out.instruction(&Instruction::If(BlockType::Empty));
    emit_error(out, ERROR_ARRAY_BOUNDS);
    out.instruction(&Instruction::End);

    out.instruction(&Instruction::LocalGet(u32::from(array)));
    out.instruction(&Instruction::I32WrapI64);
    index(out);
    out.instruction(&Instruction::I32WrapI64);
    out.instruction(&Instruction::I32Const(8));
    out.instruction(&Instruction::I32Mul);
    out.instruction(&Instruction::I32Add);
    out.instruction(&Instruction::I32Const(8));
    out.instruction(&Instruction::I32Add);
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
                super::ir::Rep::I64
                | super::ir::Rep::ArrayRef
                | super::ir::Rep::ObjectRef
                | super::ir::Rep::KeyRef
                | super::ir::Rep::TaggedRef
                | super::ir::Rep::TruthyHandle
                | super::ir::Rep::FunctionRef(_) => {
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

fn emit_object_value_address(out: &mut Function, object: u16, key: u16, cursor: u32, address: u32) {
    out.instruction(&Instruction::I64Const(0));
    out.instruction(&Instruction::LocalSet(cursor));
    out.instruction(&Instruction::Block(BlockType::Empty));
    out.instruction(&Instruction::Loop(BlockType::Empty));

    out.instruction(&Instruction::LocalGet(cursor));
    out.instruction(&Instruction::LocalGet(u32::from(object)));
    out.instruction(&Instruction::I32WrapI64);
    out.instruction(&Instruction::I64Load(I64_MEMORY));
    out.instruction(&Instruction::I64GeU);
    out.instruction(&Instruction::If(BlockType::Empty));
    emit_error(out, ERROR_OBJECT_KEY);
    out.instruction(&Instruction::End);

    out.instruction(&Instruction::LocalGet(u32::from(object)));
    out.instruction(&Instruction::I32WrapI64);
    out.instruction(&Instruction::LocalGet(cursor));
    out.instruction(&Instruction::I32WrapI64);
    out.instruction(&Instruction::I32Const(16));
    out.instruction(&Instruction::I32Mul);
    out.instruction(&Instruction::I32Add);
    out.instruction(&Instruction::I64Load(MemArg {
        offset: 8,
        ..I64_MEMORY
    }));
    out.instruction(&Instruction::LocalGet(u32::from(key)));
    out.instruction(&Instruction::I64Eq);
    out.instruction(&Instruction::If(BlockType::Empty));
    out.instruction(&Instruction::LocalGet(u32::from(object)));
    out.instruction(&Instruction::LocalGet(cursor));
    out.instruction(&Instruction::I64Const(16));
    out.instruction(&Instruction::I64Mul);
    out.instruction(&Instruction::I64Add);
    out.instruction(&Instruction::I64Const(16));
    out.instruction(&Instruction::I64Add);
    out.instruction(&Instruction::LocalSet(address));
    out.instruction(&Instruction::Br(2));
    out.instruction(&Instruction::End);

    out.instruction(&Instruction::LocalGet(cursor));
    out.instruction(&Instruction::I64Const(1));
    out.instruction(&Instruction::I64Add);
    out.instruction(&Instruction::LocalSet(cursor));
    out.instruction(&Instruction::Br(0));
    out.instruction(&Instruction::End);
    out.instruction(&Instruction::End);
    out.instruction(&Instruction::LocalGet(address));
    out.instruction(&Instruction::I32WrapI64);
}
