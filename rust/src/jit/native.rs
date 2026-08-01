//! Optional machine-code backend. Trace IR is lowered to a tiny wasm module
//! and Wasmtime/Cranelift compiles that module to host code. This reuses the
//! runtime's existing native Cranelift dependency and adds nothing to wasm or
//! default builds.

use super::{ExitReason, ExitSnapshot, Trace, TraceBackend, TraceOp, TraceOutcome, TraceValue};
use crate::core::Primitive;
use wasmtime::{Engine, Instance, Memory, Module, Store, TypedFunc};

pub struct NativeTrace {
    store: Store<()>,
    memory: Memory,
    run: TypedFunc<i32, i32>,
    trace: Trace,
    local_count: usize,
}

pub struct NativeBackend {
    engine: Engine,
}

impl Default for NativeBackend {
    fn default() -> Self {
        Self {
            engine: Engine::default(),
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
                | TraceOp::LoadLocal { local }
                | TraceOp::StoreLocal { local } => Some(*local as usize + 1),
                _ => None,
            })
            .max()
            .unwrap_or(0);
        let wasm = lower(trace, local_count)?;
        let module = Module::new(&self.engine, wasm).map_err(|error| format!("{error:?}"))?;
        let mut store = Store::new(&self.engine, ());
        let instance =
            Instance::new(&mut store, &module, &[]).map_err(|error| error.to_string())?;
        let memory = instance
            .get_memory(&mut store, "locals")
            .ok_or("native trace has no locals memory")?;
        let run = instance
            .get_typed_func::<i32, i32>(&mut store, "run")
            .map_err(|error| error.to_string())?;
        Ok(NativeTrace {
            store,
            memory,
            run,
            trace: trace.clone(),
            local_count,
        })
    }

    fn enter(
        &mut self,
        compiled: &mut NativeTrace,
        locals: &mut [TraceValue],
        max_iterations: u32,
    ) -> TraceOutcome {
        for operation in &compiled.trace.operations {
            if let TraceOp::GuardLocalI64 { local } = operation {
                if !matches!(locals.get(*local as usize), Some(TraceValue::I64(_))) {
                    return side_exit(&compiled.trace, ExitReason::WrongTag, locals);
                }
            }
        }
        if locals.len() < compiled.local_count {
            return side_exit(&compiled.trace, ExitReason::WrongTag, locals);
        }
        {
            let data = compiled.memory.data_mut(&mut compiled.store);
            for (index, value) in locals.iter().take(compiled.local_count).enumerate() {
                let bits = match value {
                    TraceValue::I64(value) => *value,
                    TraceValue::Bool(value) => i64::from(*value),
                    TraceValue::Nil => 0,
                };
                data[index * 8..index * 8 + 8].copy_from_slice(&bits.to_le_bytes());
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

fn lower(trace: &Trace, local_count: usize) -> Result<Vec<u8>, String> {
    let mut body = vec![0x02, 0x01, 0x7f, 0x03, 0x7e]; // counter i32; a,b,result i64
    body.extend([0x41, 0x00, 0x21, 0x01, 0x02, 0x40, 0x03, 0x40]);
    for operation in &trace.operations {
        match *operation {
            TraceOp::GuardLocalI64 { .. } => {}
            TraceOp::LoadLocal { local } => {
                i32_const(&mut body, i32::from(local) * 8);
                body.extend([0x29, 0x03, 0x00]);
            }
            TraceOp::ConstantI64(value) => i64_const(&mut body, value),
            TraceOp::StoreLocal { local } => {
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
    if local_count * 8 > 65536 {
        return Err("native trace locals exceed one page".into());
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
}
