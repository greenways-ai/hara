use sha2::{Digest, Sha256};

use crate::vm::{decode_program, encode_program, FunctionId, Program};

use super::codegen::compile_program;

const MAGIC: &[u8; 4] = b"HNW1";
pub const HNW_ABI_VERSION: u16 = 2;

#[derive(Debug, Clone)]
pub struct NativeArtifact {
    pub abi_version: u16,
    pub program: Program,
    pub wasm: Vec<u8>,
    pub functions: Vec<(FunctionId, u16)>,
}

pub fn compile_artifact(program: &Program) -> Result<Vec<u8>, String> {
    let hbc = encode_program(program)?;
    let wasm = compile_program(program)?;
    let mut payload = Vec::new();
    put_u16(&mut payload, HNW_ABI_VERSION);
    put_u16(
        &mut payload,
        u16::try_from(program.functions.len()).map_err(|_| "too many HNW1 functions")?,
    );
    for (id, function) in program.functions.iter().enumerate() {
        put_u16(&mut payload, id as u16);
        put_u16(&mut payload, function.arity);
    }
    put_bytes(&mut payload, &hbc)?;
    put_bytes(&mut payload, &wasm)?;
    let digest = Sha256::digest(&payload);
    let mut output = MAGIC.to_vec();
    put_u32(
        &mut output,
        u32::try_from(payload.len()).map_err(|_| "HNW1 artifact is too large")?,
    );
    output.extend_from_slice(&payload);
    output.extend_from_slice(&digest);
    Ok(output)
}

pub fn decode_artifact(bytes: &[u8]) -> Result<NativeArtifact, String> {
    if !bytes.starts_with(MAGIC) {
        return Err("native artifact has invalid magic".into());
    }
    if bytes.len() < 8 + 32 {
        return Err("native artifact is truncated".into());
    }
    let length = u32::from_be_bytes(bytes[4..8].try_into().unwrap()) as usize;
    let end = 8usize
        .checked_add(length)
        .ok_or("native artifact length overflow")?;
    if end.checked_add(32) != Some(bytes.len()) {
        return Err("native artifact length mismatch".into());
    }
    let payload = &bytes[8..end];
    if Sha256::digest(payload).as_slice() != &bytes[end..] {
        return Err("native artifact checksum mismatch".into());
    }
    let mut reader = Reader {
        bytes: payload,
        offset: 0,
    };
    let abi_version = reader.u16()?;
    if abi_version != HNW_ABI_VERSION {
        return Err(format!("unsupported HNW ABI version {abi_version}"));
    }
    let count = usize::from(reader.u16()?);
    let mut functions = Vec::with_capacity(count);
    for expected in 0..count {
        let id = reader.u16()?;
        let arity = reader.u16()?;
        if usize::from(id) != expected {
            return Err("native artifact function table is not canonical".into());
        }
        functions.push((id, arity));
    }
    let program = decode_program(reader.bytes()?)?;
    let wasm = reader.bytes()?.to_vec();
    reader.finish()?;
    if wasm.get(..4) != Some(b"\0asm") {
        return Err("native artifact contains invalid Wasm".into());
    }
    if program.functions.len() != functions.len()
        || program
            .functions
            .iter()
            .zip(&functions)
            .any(|(function, (_, arity))| function.arity != *arity)
    {
        return Err("native artifact function metadata mismatch".into());
    }
    Ok(NativeArtifact {
        abi_version,
        program,
        wasm,
        functions,
    })
}

fn put_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_be_bytes());
}
fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_be_bytes());
}
fn put_bytes(out: &mut Vec<u8>, bytes: &[u8]) -> Result<(), String> {
    put_u32(
        out,
        u32::try_from(bytes.len()).map_err(|_| "HNW1 section is too large")?,
    );
    out.extend_from_slice(bytes);
    Ok(())
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], String> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or("native artifact offset overflow")?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or("native artifact is truncated")?;
        self.offset = end;
        Ok(value)
    }
    fn u16(&mut self) -> Result<u16, String> {
        Ok(u16::from_be_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn bytes(&mut self) -> Result<&'a [u8], String> {
        let n = self.u32()? as usize;
        self.take(n)
    }
    fn finish(self) -> Result<(), String> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err("native artifact has trailing payload".into())
        }
    }
}
