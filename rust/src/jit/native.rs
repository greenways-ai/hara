//! Optional machine-code backend. Trace IR is lowered to a tiny wasm module
//! and Wasmtime/Cranelift compiles that module to host code. This reuses the
//! runtime's existing native Cranelift dependency and adds nothing to wasm or
//! default builds.

use super::{ExitReason, ExitSnapshot, Trace, TraceBackend, TraceOp, TraceOutcome, TraceValue};
use crate::core::Primitive;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use wasmtime::{Engine, Instance, Memory, Module, Store, TypedFunc};

const MAX_CACHED_MODULES: usize = 128;
const WASM_PAGE_BYTES: usize = 65_536;
const MAX_TRACE_MEMORY_BYTES: usize = 16 * 1024 * 1024;

thread_local! {
    static NATIVE_ENGINE: Engine = Engine::default();
    static MODULE_CACHE: RefCell<HashMap<Vec<u8>, Module>> = RefCell::new(HashMap::new());
}

pub struct NativeTrace {
    store: Store<()>,
    memory: Memory,
    run: TypedFunc<i32, i32>,
    trace: Trace,
    local_count: usize,
    heap_start: usize,
}

pub struct NativeBackend {
    engine: Engine,
}

impl Default for NativeBackend {
    fn default() -> Self {
        Self {
            engine: NATIVE_ENGINE.with(Clone::clone),
        }
    }
}

impl TraceBackend for NativeBackend {
    type Compiled = NativeTrace;

    fn compile(&mut self, trace: &Trace) -> Result<NativeTrace, String> {
        let local_count = trace
            .operations
            .iter()
            .filter_map(|operation| match operation {
                TraceOp::GuardLocalI64 { local }
                | TraceOp::GuardLocalVectorI64 { local }
                | TraceOp::LoadLocal { local }
                | TraceOp::StoreLocal { local } => Some(*local as usize + 1),
                _ => None,
            })
            .max()
            .unwrap_or(0);
        let (constant_offsets, heap_start) = constant_layout(trace, local_count)?;
        let wasm = lower(trace, local_count, &constant_offsets)?;
        let module = MODULE_CACHE.with(|cache| -> Result<Module, String> {
            if let Some(module) = cache.borrow().get(&wasm) {
                return Ok(module.clone());
            }
            let module = Module::new(&self.engine, &wasm).map_err(|error| format!("{error:?}"))?;
            let mut cache = cache.borrow_mut();
            if cache.len() >= MAX_CACHED_MODULES {
                cache.clear();
            }
            cache.insert(wasm, module.clone());
            Ok(module)
        })?;
        let mut store = Store::new(&self.engine, ());
        let instance =
            Instance::new(&mut store, &module, &[]).map_err(|error| error.to_string())?;
        let memory = instance
            .get_memory(&mut store, "locals")
            .ok_or("native trace has no locals memory")?;
        let run = instance
            .get_typed_func::<i32, i32>(&mut store, "run")
            .map_err(|error| error.to_string())?;
        ensure_memory(&memory, &mut store, heap_start)?;
        {
            let data = memory.data_mut(&mut store);
            for (values, offset) in trace.vectors.iter().zip(&constant_offsets) {
                write_vector(data, *offset, values)?;
            }
        }
        Ok(NativeTrace {
            store,
            memory,
            run,
            trace: trace.clone(),
            local_count,
            heap_start,
        })
    }

    fn enter(
        &mut self,
        compiled: &mut NativeTrace,
        locals: &mut [TraceValue],
        max_iterations: u32,
    ) -> TraceOutcome {
        let mut vector_locals = HashMap::new();
        let mut heap_cursor = compiled.heap_start;
        for operation in &compiled.trace.operations {
            match operation {
                TraceOp::GuardLocalI64 { local }
                    if !matches!(locals.get(usize::from(*local)), Some(TraceValue::I64(_))) =>
                {
                    return side_exit(&compiled.trace, ExitReason::WrongTag, locals)
                }
                TraceOp::GuardLocalVectorI64 { local } if !vector_locals.contains_key(local) => {
                    let Some(values) = locals
                        .get(usize::from(*local))
                        .and_then(numeric_vector_values)
                    else {
                        return side_exit(&compiled.trace, ExitReason::WrongTag, locals);
                    };
                    let bytes = match vector_bytes(values.len()) {
                        Ok(bytes) => bytes,
                        Err(_) => {
                            return side_exit(&compiled.trace, ExitReason::Unsupported, locals)
                        }
                    };
                    vector_locals.insert(*local, (heap_cursor, values));
                    heap_cursor = match heap_cursor.checked_add(bytes) {
                        Some(cursor) if cursor <= MAX_TRACE_MEMORY_BYTES => cursor,
                        _ => return side_exit(&compiled.trace, ExitReason::Unsupported, locals),
                    };
                }
                _ => {}
            }
        }
        if locals.len() < compiled.local_count {
            return side_exit(&compiled.trace, ExitReason::WrongTag, locals);
        }
        if ensure_memory(&compiled.memory, &mut compiled.store, heap_cursor).is_err() {
            return side_exit(&compiled.trace, ExitReason::Unsupported, locals);
        }
        {
            let data = compiled.memory.data_mut(&mut compiled.store);
            for (index, value) in locals.iter().take(compiled.local_count).enumerate() {
                let bits = match value {
                    TraceValue::I64(value) => *value,
                    TraceValue::Bool(value) => i64::from(*value),
                    TraceValue::Nil => 0,
                    TraceValue::Indexed(_) => vector_locals
                        .get(&(index as u16))
                        .map_or(0, |(offset, _)| *offset as i64),
                    TraceValue::Unsupported => 0,
                };
                data[index * 8..index * 8 + 8].copy_from_slice(&bits.to_le_bytes());
            }
            for (offset, values) in vector_locals.values() {
                if write_vector(data, *offset, values).is_err() {
                    return side_exit(&compiled.trace, ExitReason::Unsupported, locals);
                }
            }
        }
        let result = match compiled
            .run
            .call(&mut compiled.store, max_iterations as i32)
        {
            Ok(result) => result,
            Err(_) => return side_exit(&compiled.trace, ExitReason::Unsupported, locals),
        };
        {
            let data = compiled.memory.data(&compiled.store);
            for (index, value) in locals.iter_mut().take(compiled.local_count).enumerate() {
                let bits = i64::from_le_bytes(data[index * 8..index * 8 + 8].try_into().unwrap());
                if matches!(value, TraceValue::I64(_)) {
                    *value = TraceValue::I64(bits);
                }
            }
        }
        match result {
            -1 => side_exit(&compiled.trace, ExitReason::Overflow, locals),
            -2 => side_exit(&compiled.trace, ExitReason::DivisionByZero, locals),
            -3 => side_exit(&compiled.trace, ExitReason::IndexOutOfBounds, locals),
            value if value >= 0 && value < max_iterations as i32 => {
                side_exit(&compiled.trace, ExitReason::BranchChanged, locals)
            }
            _ => TraceOutcome::Completed {
                iterations: max_iterations,
            },
        }
    }
}

fn side_exit(trace: &Trace, reason: ExitReason, locals: &[TraceValue]) -> TraceOutcome {
    TraceOutcome::SideExit {
        reason,
        snapshot: ExitSnapshot {
            function: trace.function,
            instruction: trace.resume_ip,
            locals: locals.to_vec(),
            stack: Vec::new(),
        },
    }
}

fn numeric_vector_values(value: &TraceValue) -> Option<Vec<i64>> {
    let TraceValue::Indexed(value) = value else {
        return None;
    };
    let values: Box<dyn Iterator<Item = &crate::core::Value> + '_> = match value.as_ref() {
        crate::core::Value::Tuple(values) => Box::new(values.iter()),
        crate::core::Value::Vector(values) => Box::new(values.iter()),
        _ => return None,
    };
    values
        .map(|value| match value {
            crate::core::Value::Number(value) => Some(*value),
            _ => None,
        })
        .collect()
}

fn vector_bytes(length: usize) -> Result<usize, String> {
    length
        .checked_add(1)
        .and_then(|words| words.checked_mul(8))
        .ok_or_else(|| "trace vector is too large".into())
}

fn constant_layout(trace: &Trace, local_count: usize) -> Result<(Vec<usize>, usize), String> {
    let mut cursor = local_count
        .checked_mul(8)
        .ok_or_else(|| "trace locals exceed the memory limit".to_string())?;
    let mut offsets = Vec::with_capacity(trace.vectors.len());
    for vector in &trace.vectors {
        offsets.push(cursor);
        cursor = cursor
            .checked_add(vector_bytes(vector.len())?)
            .ok_or_else(|| "trace constants exceed the memory limit".to_string())?;
    }
    if cursor > MAX_TRACE_MEMORY_BYTES {
        return Err("trace constants exceed the memory limit".into());
    }
    Ok((offsets, cursor))
}

fn ensure_memory(memory: &Memory, store: &mut Store<()>, required: usize) -> Result<(), String> {
    if required > MAX_TRACE_MEMORY_BYTES {
        return Err("trace memory exceeds the limit".into());
    }
    let current = memory.data_size(&mut *store);
    if required <= current {
        return Ok(());
    }
    let pages = (required - current).div_ceil(WASM_PAGE_BYTES);
    memory
        .grow(
            &mut *store,
            u64::try_from(pages).map_err(|_| "trace memory growth exceeds u64")?,
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn write_vector(data: &mut [u8], offset: usize, values: &[i64]) -> Result<(), String> {
    let end = offset
        .checked_add(vector_bytes(values.len())?)
        .ok_or_else(|| "trace vector address overflow".to_string())?;
    let target = data
        .get_mut(offset..end)
        .ok_or_else(|| "trace vector exceeds native memory".to_string())?;
    target[..8].copy_from_slice(&(values.len() as i64).to_le_bytes());
    for (index, value) in values.iter().enumerate() {
        let start = 8 + index * 8;
        target[start..start + 8].copy_from_slice(&value.to_le_bytes());
    }
    Ok(())
}

fn lower(trace: &Trace, local_count: usize, constant_offsets: &[usize]) -> Result<Vec<u8>, String> {
    let vector_locals = trace
        .operations
        .iter()
        .filter_map(|operation| match operation {
            TraceOp::GuardLocalVectorI64 { local } => Some(*local),
            _ => None,
        })
        .collect::<HashSet<_>>();
    let mut body = vec![0x02, 0x01, 0x7f, 0x03, 0x7e]; // counter i32; a,b,result i64
    body.extend([0x41, 0x00, 0x21, 0x01, 0x02, 0x40, 0x03, 0x40]);
    for operation in &trace.operations {
        match *operation {
            TraceOp::GuardLocalI64 { .. } | TraceOp::GuardLocalVectorI64 { .. } => {}
            TraceOp::LoadLocal { local } => {
                i32_const(&mut body, i32::from(local) * 8);
                body.extend([0x29, 0x03, 0x00]);
                if vector_locals.contains(&local) {
                    body.push(0xa7); // i32.wrap_i64
                }
            }
            TraceOp::ConstantI64(value) => i64_const(&mut body, value),
            TraceOp::ConstantVectorI64 { vector } => {
                let offset = constant_offsets
                    .get(usize::from(vector))
                    .ok_or_else(|| format!("trace vector {vector} is out of range"))?;
                i32_const(
                    &mut body,
                    i32::try_from(*offset).map_err(|_| "trace vector offset exceeds i32")?,
                );
            }
            TraceOp::StoreLocal { local } => {
                if vector_locals.contains(&local) {
                    body.push(0xad); // i64.extend_i32_u
                }
                body.extend([0x21, 0x04]);
                i32_const(&mut body, i32::from(local) * 8);
                body.extend([0x20, 0x04, 0x37, 0x03, 0x00]);
            }
            TraceOp::Pop => {
                body.push(0x1a);
            }
            TraceOp::GuardTruthy { expected: true } => body.extend([0x45, 0x0d, 0x01]),
            TraceOp::GuardTruthy { expected: false } => body.extend([0x0d, 0x01]),
            TraceOp::BinaryI64(op) => binary(&mut body, op)?,
            TraceOp::VectorNthI64 => vector_nth(&mut body),
            TraceOp::LoopBackedge => {
                body.extend([
                    0x20, 0x01, 0x41, 0x01, 0x6a, 0x21, 0x01, 0x20, 0x01, 0x20, 0x00, 0x48, 0x0d,
                    0x00,
                ]);
            }
        }
    }
    body.extend([0x0b, 0x0b, 0x20, 0x01, 0x0b]);
    let mut module = b"\0asm\x01\0\0\0".to_vec();
    section(&mut module, 1, vec![0x01, 0x60, 0x01, 0x7f, 0x01, 0x7f]);
    section(&mut module, 3, vec![0x01, 0x00]);
    section(&mut module, 5, vec![0x01, 0x00, 0x01]);
    let mut exports = vec![0x02, 0x06];
    exports.extend(b"locals");
    exports.extend([0x02, 0x00, 0x03]);
    exports.extend(b"run");
    exports.extend([0x00, 0x00]);
    section(&mut module, 7, exports);
    let mut code = vec![0x01];
    uleb(&mut code, body.len() as u32);
    code.extend(body);
    section(&mut module, 10, code);
    if local_count
        .checked_mul(8)
        .is_none_or(|bytes| bytes > MAX_TRACE_MEMORY_BYTES)
    {
        return Err("native trace locals exceed the memory limit".into());
    }
    Ok(module)
}

fn binary(body: &mut Vec<u8>, op: Primitive) -> Result<(), String> {
    body.extend([0x21, 0x03, 0x21, 0x02]);
    match op {
        Primitive::Add | Primitive::Subtract => {
            body.extend([
                0x20,
                0x02,
                0x20,
                0x03,
                if op == Primitive::Add { 0x7c } else { 0x7d },
                0x21,
                0x04,
            ]);
            if op == Primitive::Add {
                body.extend([0x20, 0x02, 0x20, 0x04, 0x85, 0x20, 0x03, 0x20, 0x04, 0x85]);
            } else {
                body.extend([0x20, 0x02, 0x20, 0x03, 0x85, 0x20, 0x02, 0x20, 0x04, 0x85]);
            }
            body.extend([0x83]);
            i64_const(body, 0);
            body.extend([0x53, 0x04, 0x40]);
            i32_const(body, -1);
            body.extend([0x21, 0x01, 0x0c, 0x02, 0x0b, 0x20, 0x04]);
        }
        Primitive::Remainder => {
            body.extend([0x20, 0x03, 0x50, 0x04, 0x40]);
            i32_const(body, -2);
            body.extend([0x21, 0x01, 0x0c, 0x02, 0x0b, 0x20, 0x02, 0x20, 0x03, 0x81]);
        }
        Primitive::Less
        | Primitive::LessOrEqual
        | Primitive::Greater
        | Primitive::GreaterOrEqual
        | Primitive::Equal => {
            body.extend([
                0x20,
                0x02,
                0x20,
                0x03,
                match op {
                    Primitive::Less => 0x53,
                    Primitive::LessOrEqual => 0x57,
                    Primitive::Greater => 0x55,
                    Primitive::GreaterOrEqual => 0x59,
                    _ => 0x51,
                },
            ]);
        }
        _ => return Err(format!("native trace does not support {op:?}")),
    }
    Ok(())
}

fn vector_nth(body: &mut Vec<u8>) {
    body.extend([0x21, 0x02]); // index i64
    body.extend([0xad, 0x21, 0x03]); // vector pointer i32 -> i64

    body.extend([0x20, 0x02]);
    i64_const(body, 0);
    body.extend([0x53, 0x04, 0x40]); // index < 0
    native_exit(body, -3);
    body.push(0x0b);

    body.extend([0x20, 0x02, 0x20, 0x03, 0xa7, 0x29, 0x03, 0x00, 0x5a]);
    body.extend([0x04, 0x40]); // index >= vector length
    native_exit(body, -3);
    body.push(0x0b);

    body.extend([0x20, 0x03, 0xa7]);
    i32_const(body, 8);
    body.push(0x6a);
    body.extend([0x20, 0x02, 0xa7]);
    i32_const(body, 8);
    body.extend([0x6c, 0x6a, 0x29, 0x03, 0x00]);
}

fn native_exit(body: &mut Vec<u8>, code: i32) {
    i32_const(body, code);
    body.extend([0x21, 0x01, 0x0c, 0x02]);
}

fn section(module: &mut Vec<u8>, id: u8, payload: Vec<u8>) {
    module.push(id);
    uleb(module, payload.len() as u32);
    module.extend(payload);
}
fn uleb(output: &mut Vec<u8>, mut value: u32) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        output.push(byte);
        if value == 0 {
            break;
        }
    }
}
fn i32_const(output: &mut Vec<u8>, value: i32) {
    output.push(0x41);
    sleb(output, value as i64);
}
fn i64_const(output: &mut Vec<u8>, value: i64) {
    output.push(0x42);
    sleb(output, value);
}
fn sleb(output: &mut Vec<u8>, mut value: i64) {
    loop {
        let byte = (value & 0x7f) as u8;
        value >>= 7;
        let done = (value == 0 && byte & 0x40 == 0) || (value == -1 && byte & 0x40 != 0);
        output.push(if done { byte } else { byte | 0x80 });
        if done {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cranelift_backend_executes_guarded_host_code() {
        let trace = Trace {
            function: 0,
            header: 0,
            resume_ip: 0,
            operations: vec![
                TraceOp::GuardLocalI64 { local: 0 },
                TraceOp::LoadLocal { local: 0 },
                TraceOp::ConstantI64(1),
                TraceOp::BinaryI64(Primitive::Add),
                TraceOp::StoreLocal { local: 0 },
                TraceOp::LoopBackedge,
            ],
            vectors: Vec::new(),
        };
        let mut backend = NativeBackend::default();
        let mut compiled = backend.compile(&trace).unwrap();
        let mut locals = [TraceValue::I64(2)];
        assert_eq!(
            backend.enter(&mut compiled, &mut locals, 10),
            TraceOutcome::Completed { iterations: 10 }
        );
        assert_eq!(locals[0], TraceValue::I64(12));
    }

    #[test]
    fn cranelift_backend_indexes_vector_constants_and_exits_on_bounds() {
        let trace = Trace {
            function: 0,
            header: 0,
            resume_ip: 0,
            operations: vec![
                TraceOp::ConstantVectorI64 { vector: 0 },
                TraceOp::LoadLocal { local: 0 },
                TraceOp::VectorNthI64,
                TraceOp::StoreLocal { local: 1 },
                TraceOp::LoopBackedge,
            ],
            vectors: vec![vec![10, 20, 30]],
        };
        let mut backend = NativeBackend::default();
        let mut compiled = backend.compile(&trace).unwrap();
        let mut locals = [TraceValue::I64(2), TraceValue::I64(0)];
        assert_eq!(
            backend.enter(&mut compiled, &mut locals, 1),
            TraceOutcome::Completed { iterations: 1 }
        );
        assert_eq!(locals[1], TraceValue::I64(30));

        locals[0] = TraceValue::I64(-1);
        assert!(matches!(
            backend.enter(&mut compiled, &mut locals, 1),
            TraceOutcome::SideExit {
                reason: ExitReason::IndexOutOfBounds,
                ..
            }
        ));
    }

    #[test]
    fn cranelift_backend_indexes_guarded_vector_locals() {
        let trace = Trace {
            function: 0,
            header: 0,
            resume_ip: 0,
            operations: vec![
                TraceOp::GuardLocalVectorI64 { local: 0 },
                TraceOp::LoadLocal { local: 0 },
                TraceOp::LoadLocal { local: 1 },
                TraceOp::VectorNthI64,
                TraceOp::StoreLocal { local: 2 },
                TraceOp::LoopBackedge,
            ],
            vectors: Vec::new(),
        };
        let mut backend = NativeBackend::default();
        let mut compiled = backend.compile(&trace).unwrap();
        let mut locals = [
            TraceValue::Indexed(Box::new(crate::core::Value::Vector(
                [10, 20, 30]
                    .into_iter()
                    .map(crate::core::Value::Number)
                    .collect(),
            ))),
            TraceValue::I64(1),
            TraceValue::I64(0),
        ];
        assert_eq!(
            backend.enter(&mut compiled, &mut locals, 1),
            TraceOutcome::Completed { iterations: 1 }
        );
        assert_eq!(locals[2], TraceValue::I64(20));
        assert!(matches!(locals[0], TraceValue::Indexed(_)));
    }
}
