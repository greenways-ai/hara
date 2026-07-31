//! Versioned persistent encoding for validated VM programs.

use std::rc::Rc;

use sha2::{Digest, Sha256};

use super::opcode::Instruction;
use super::program::{CatchEntry, FunctionPrototype, Program, TryEntry};
use super::source_map::SourceMap;
use crate::core::Primitive;
#[cfg(test)]
use crate::core::Value;
use crate::kernel::Position;
use crate::lang::data::{Keyword, Metadata, MetadataValue, Symbol};

const MAGIC: &[u8; 4] = b"HBC1";

/// Encodes a program after validating it. Constants use the portable HTA
/// value codec; unsupported runtime-only values are rejected explicitly.
pub fn encode_program(program: &Program) -> Result<Vec<u8>, String> {
    super::validate::validate(program).map_err(|error| error.to_string())?;
    let mut payload = Writer::default();
    payload.u16(program.entry);
    payload.len(program.constants.len())?;
    for value in &program.constants {
        payload.bytes(&crate::hta::encode(value)?)?;
    }
    payload.len(program.var_metadata.len())?;
    for metadata in &program.var_metadata {
        write_metadata(&mut payload, metadata)?;
    }
    payload.len(program.functions.len())?;
    for function in &program.functions {
        write_function(&mut payload, function)?;
    }
    let digest = Sha256::digest(&payload.bytes);
    let mut output = MAGIC.to_vec();
    output.extend_from_slice(
        &u32::try_from(payload.bytes.len())
            .map_err(|_| "bytecode artifact is too large")?
            .to_be_bytes(),
    );
    output.extend_from_slice(&payload.bytes);
    output.extend_from_slice(&digest);
    Ok(output)
}

/// Decodes, authenticates, and validates a persistent VM program.
pub fn decode_program(bytes: &[u8]) -> Result<Program, String> {
    if !bytes.starts_with(MAGIC) {
        return Err("bytecode artifact has invalid magic".into());
    }
    if bytes.len() < 8 + 32 {
        return Err("bytecode artifact is truncated".into());
    }
    let payload_len = u32::from_be_bytes(bytes[4..8].try_into().unwrap()) as usize;
    let payload_end = 8usize
        .checked_add(payload_len)
        .ok_or("bytecode artifact length overflow")?;
    if payload_end.checked_add(32) != Some(bytes.len()) {
        return Err("bytecode artifact length mismatch".into());
    }
    let payload = &bytes[8..payload_end];
    if Sha256::digest(payload).as_slice() != &bytes[payload_end..] {
        return Err("bytecode artifact checksum mismatch".into());
    }
    let mut reader = Reader::new(payload);
    let entry = reader.u16()?;
    let constants = reader.many(|reader| crate::hta::decode(reader.bytes()?))?;
    let var_metadata = reader.many(|reader| read_metadata(reader))?;
    let functions = reader.many(|reader| read_function(reader))?;
    reader.finish()?;
    let program = Program {
        constants,
        var_metadata,
        functions,
        entry,
    };
    super::validate::validate(&program).map_err(|error| error.to_string())?;
    Ok(program)
}

fn write_function(out: &mut Writer, function: &FunctionPrototype) -> Result<(), String> {
    out.option_string(function.name.as_deref())?;
    out.u16(function.arity);
    out.byte(u8::from(function.variadic));
    out.u16(function.capture_count);
    out.u16(function.local_count);
    out.u16(function.max_stack);
    out.len(function.code.len())?;
    for instruction in &function.code {
        write_instruction(out, instruction);
    }
    out.len(function.source_map.len())?;
    for index in 0..function.source_map.len() {
        match function.source_map.position(index) {
            Some(position) => {
                out.byte(1);
                out.usize32(position.offset)?;
                out.usize32(position.line)?;
                out.usize32(position.column)?;
            }
            None => out.byte(0),
        }
    }
    out.len(function.handlers.len())?;
    for handler in &function.handlers {
        out.u32(handler.start);
        out.u32(handler.end);
        out.u16(handler.depth);
        out.len(handler.catches.len())?;
        for catch in &handler.catches {
            out.string(&catch.class)?;
            out.u16(catch.binding);
            out.u32(catch.target);
        }
        out.option_u32(handler.finally);
        out.option_u16(handler.pending_value);
        out.option_u16(handler.pending_error);
    }
    Ok(())
}

fn read_function(reader: &mut Reader<'_>) -> Result<FunctionPrototype, String> {
    let name = reader.option_string()?;
    let arity = reader.u16()?;
    let variadic = reader.boolean()?;
    let capture_count = reader.u16()?;
    let local_count = reader.u16()?;
    let max_stack = reader.u16()?;
    let code = reader.many(read_instruction)?;
    let positions = reader.many(|reader| {
        if reader.boolean()? {
            Ok(Some(Position {
                offset: reader.u32()? as usize,
                line: reader.u32()? as usize,
                column: reader.u32()? as usize,
            }))
        } else {
            Ok(None)
        }
    })?;
    let mut source_map = SourceMap::default();
    for position in positions {
        source_map.record(position);
    }
    let handlers = reader.many(|reader| {
        let start = reader.u32()?;
        let end = reader.u32()?;
        let depth = reader.u16()?;
        let catches = reader.many(|reader| {
            Ok(CatchEntry {
                class: reader.string()?,
                binding: reader.u16()?,
                target: reader.u32()?,
            })
        })?;
        Ok(TryEntry {
            start,
            end,
            depth,
            catches,
            finally: reader.option_u32()?,
            pending_value: reader.option_u16()?,
            pending_error: reader.option_u16()?,
        })
    })?;
    Ok(FunctionPrototype {
        name,
        arity,
        variadic,
        capture_count,
        local_count,
        max_stack,
        code,
        source_map,
        handlers,
    })
}

fn write_instruction(out: &mut Writer, instruction: &Instruction) {
    use Instruction::*;
    match instruction {
        Constant(value) => {
            out.byte(0);
            out.u32(*value);
        }
        Nil => out.byte(1),
        True => out.byte(2),
        False => out.byte(3),
        LoadLocal(value) => {
            out.byte(4);
            out.u16(*value);
        }
        StoreLocal(value) => {
            out.byte(5);
            out.u16(*value);
        }
        Pop => out.byte(6),
        Primitive { op, argc } => {
            out.byte(7);
            out.byte(primitive_id(*op));
            out.byte(*argc);
        }
        PrimitiveLocalConst {
            op,
            local,
            constant,
        } => {
            out.byte(25);
            out.byte(primitive_id(*op));
            out.u16(*local);
            out.u32(*constant);
        }
        Jump(value) => {
            out.byte(8);
            out.u32(*value);
        }
        JumpIfFalse(value) => {
            out.byte(9);
            out.u32(*value);
        }
        Closure {
            prototype,
            captures,
        } => {
            out.byte(10);
            out.u16(*prototype);
            out.byte(*captures);
        }
        Call { argc } => {
            out.byte(11);
            out.byte(*argc);
        }
        CallStatic { prototype, argc } => {
            out.byte(12);
            out.u16(*prototype);
            out.byte(*argc);
        }
        Throw => out.byte(13),
        Rethrow => out.byte(14),
        GetGlobal(value) => {
            out.byte(15);
            out.u32(*value);
        }
        DefGlobal { name, metadata } => {
            out.byte(16);
            out.u32(*name);
            out.option_u16(*metadata);
        }
        SetGlobal(value) => {
            out.byte(17);
            out.u32(*value);
        }
        VarGlobal(value) => {
            out.byte(18);
            out.u32(*value);
        }
        DeclareGlobal(value) => {
            out.byte(19);
            out.u32(*value);
        }
        DefStruct { name, fields } => {
            out.byte(20);
            out.u32(*name);
            out.u32(*fields);
        }
        StructField(value) => {
            out.byte(21);
            out.u32(*value);
        }
        InstanceOf => out.byte(22),
        MakeMultiArity { name, count } => {
            out.byte(23);
            out.u32(*name);
            out.byte(*count);
        }
        Return => out.byte(24),
    }
}

fn read_instruction(reader: &mut Reader<'_>) -> Result<Instruction, String> {
    Ok(match reader.byte()? {
        0 => Instruction::Constant(reader.u32()?),
        1 => Instruction::Nil,
        2 => Instruction::True,
        3 => Instruction::False,
        4 => Instruction::LoadLocal(reader.u16()?),
        5 => Instruction::StoreLocal(reader.u16()?),
        6 => Instruction::Pop,
        7 => Instruction::Primitive {
            op: primitive(reader.byte()?)?,
            argc: reader.byte()?,
        },
        8 => Instruction::Jump(reader.u32()?),
        9 => Instruction::JumpIfFalse(reader.u32()?),
        10 => Instruction::Closure {
            prototype: reader.u16()?,
            captures: reader.byte()?,
        },
        11 => Instruction::Call {
            argc: reader.byte()?,
        },
        12 => Instruction::CallStatic {
            prototype: reader.u16()?,
            argc: reader.byte()?,
        },
        13 => Instruction::Throw,
        14 => Instruction::Rethrow,
        15 => Instruction::GetGlobal(reader.u32()?),
        16 => Instruction::DefGlobal {
            name: reader.u32()?,
            metadata: reader.option_u16()?,
        },
        17 => Instruction::SetGlobal(reader.u32()?),
        18 => Instruction::VarGlobal(reader.u32()?),
        19 => Instruction::DeclareGlobal(reader.u32()?),
        20 => Instruction::DefStruct {
            name: reader.u32()?,
            fields: reader.u32()?,
        },
        21 => Instruction::StructField(reader.u32()?),
        22 => Instruction::InstanceOf,
        23 => Instruction::MakeMultiArity {
            name: reader.u32()?,
            count: reader.byte()?,
        },
        24 => Instruction::Return,
        25 => Instruction::PrimitiveLocalConst {
            op: primitive(reader.byte()?)?,
            local: reader.u16()?,
            constant: reader.u32()?,
        },
        _ => return Err("bytecode artifact contains an unknown opcode".into()),
    })
}

fn primitive_id(value: Primitive) -> u8 {
    match value {
        Primitive::Add => 0,
        Primitive::Subtract => 1,
        Primitive::Multiply => 2,
        Primitive::Divide => 3,
        Primitive::Remainder => 4,
        Primitive::Equal => 5,
        Primitive::Less => 6,
        Primitive::LessOrEqual => 7,
        Primitive::Greater => 8,
        Primitive::GreaterOrEqual => 9,
        Primitive::Count => 10,
        Primitive::Get => 11,
        Primitive::Meta => 12,
    }
}

fn primitive(value: u8) -> Result<Primitive, String> {
    Ok(match value {
        0 => Primitive::Add,
        1 => Primitive::Subtract,
        2 => Primitive::Multiply,
        3 => Primitive::Divide,
        4 => Primitive::Remainder,
        5 => Primitive::Equal,
        6 => Primitive::Less,
        7 => Primitive::LessOrEqual,
        8 => Primitive::Greater,
        9 => Primitive::GreaterOrEqual,
        10 => Primitive::Count,
        11 => Primitive::Get,
        12 => Primitive::Meta,
        _ => return Err("bytecode artifact contains an unknown primitive".into()),
    })
}

fn write_metadata(out: &mut Writer, metadata: &Metadata) -> Result<(), String> {
    out.len(metadata.entries().len())?;
    for (key, value) in metadata.entries() {
        write_metadata_value(out, key)?;
        write_metadata_value(out, value)?;
    }
    Ok(())
}

fn read_metadata(reader: &mut Reader<'_>) -> Result<Rc<Metadata>, String> {
    let entries =
        reader.many(|reader| Ok((read_metadata_value(reader)?, read_metadata_value(reader)?)))?;
    Ok(Metadata::new(entries))
}

fn write_metadata_value(out: &mut Writer, value: &MetadataValue) -> Result<(), String> {
    use MetadataValue::*;
    match value {
        Nil => out.byte(0),
        Boolean(v) => {
            out.byte(1);
            out.byte(u8::from(*v));
        }
        Number(v) => {
            out.byte(2);
            out.i64(*v);
        }
        Float(v) => {
            out.byte(3);
            out.u64(v.to_bits());
        }
        BigInteger(v) => {
            out.byte(4);
            out.string(v)?;
        }
        Decimal(v) => {
            out.byte(5);
            out.string(v)?;
        }
        Character(v) => {
            out.byte(6);
            out.u32(*v as u32);
        }
        Regex(v) => {
            out.byte(7);
            out.string(v)?;
        }
        Tagged(tag, value) => {
            out.byte(8);
            out.string(tag)?;
            write_metadata_value(out, value)?;
        }
        String(v) => {
            out.byte(9);
            out.string(v)?;
        }
        Keyword(v) => {
            out.byte(10);
            out.string(v.as_str())?;
        }
        Symbol(v) => {
            out.byte(11);
            out.string(v.as_str())?;
        }
        Vector(values) => {
            out.byte(12);
            write_metadata_values(out, values)?;
        }
        List(values) => {
            out.byte(13);
            write_metadata_values(out, values)?;
        }
        Set(values) => {
            out.byte(14);
            write_metadata_values(out, values)?;
        }
        Map(values) => {
            out.byte(15);
            out.len(values.len())?;
            for (k, v) in values {
                write_metadata_value(out, k)?;
                write_metadata_value(out, v)?;
            }
        }
    }
    Ok(())
}

fn write_metadata_values(out: &mut Writer, values: &[MetadataValue]) -> Result<(), String> {
    out.len(values.len())?;
    for value in values {
        write_metadata_value(out, value)?;
    }
    Ok(())
}

fn read_metadata_value(reader: &mut Reader<'_>) -> Result<MetadataValue, String> {
    Ok(match reader.byte()? {
        0 => MetadataValue::Nil,
        1 => MetadataValue::Boolean(reader.boolean()?),
        2 => MetadataValue::Number(reader.i64()?),
        3 => MetadataValue::Float(f64::from_bits(reader.u64()?)),
        4 => MetadataValue::BigInteger(reader.string()?),
        5 => MetadataValue::Decimal(reader.string()?),
        6 => MetadataValue::Character(
            char::from_u32(reader.u32()?).ok_or("invalid metadata character")?,
        ),
        7 => MetadataValue::Regex(reader.string()?),
        8 => MetadataValue::Tagged(reader.string()?, Box::new(read_metadata_value(reader)?)),
        9 => MetadataValue::String(reader.string()?),
        10 => MetadataValue::Keyword(Keyword::from(reader.string()?)),
        11 => MetadataValue::Symbol(Symbol::from(reader.string()?)),
        12 => MetadataValue::Vector(reader.many(read_metadata_value)?),
        13 => MetadataValue::List(reader.many(read_metadata_value)?),
        14 => MetadataValue::Set(reader.many(read_metadata_value)?),
        15 => MetadataValue::Map(
            reader.many(|r| Ok((read_metadata_value(r)?, read_metadata_value(r)?)))?,
        ),
        _ => return Err("bytecode artifact contains unknown metadata".into()),
    })
}

#[derive(Default)]
struct Writer {
    bytes: Vec<u8>,
}
impl Writer {
    fn byte(&mut self, value: u8) {
        self.bytes.push(value);
    }
    fn u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }
    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }
    fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }
    fn i64(&mut self, value: i64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }
    fn usize32(&mut self, value: usize) -> Result<(), String> {
        self.u32(u32::try_from(value).map_err(|_| "bytecode field is too large")?);
        Ok(())
    }
    fn len(&mut self, value: usize) -> Result<(), String> {
        self.usize32(value)
    }
    fn bytes(&mut self, value: &[u8]) -> Result<(), String> {
        self.len(value.len())?;
        self.bytes.extend_from_slice(value);
        Ok(())
    }
    fn string(&mut self, value: &str) -> Result<(), String> {
        self.bytes(value.as_bytes())
    }
    fn option_string(&mut self, value: Option<&str>) -> Result<(), String> {
        match value {
            Some(v) => {
                self.byte(1);
                self.string(v)?;
            }
            None => self.byte(0),
        };
        Ok(())
    }
    fn option_u16(&mut self, value: Option<u16>) {
        match value {
            Some(v) => {
                self.byte(1);
                self.u16(v);
            }
            None => self.byte(0),
        }
    }
    fn option_u32(&mut self, value: Option<u32>) {
        match value {
            Some(v) => {
                self.byte(1);
                self.u32(v);
            }
            None => self.byte(0),
        }
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    cursor: usize,
}
impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }
    fn take(&mut self, size: usize) -> Result<&'a [u8], String> {
        let end = self
            .cursor
            .checked_add(size)
            .ok_or("bytecode artifact length overflow")?;
        if end > self.bytes.len() {
            return Err("bytecode artifact is truncated".into());
        }
        let value = &self.bytes[self.cursor..end];
        self.cursor = end;
        Ok(value)
    }
    fn byte(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    fn boolean(&mut self) -> Result<bool, String> {
        match self.byte()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err("bytecode artifact contains invalid boolean".into()),
        }
    }
    fn u16(&mut self) -> Result<u16, String> {
        Ok(u16::from_be_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64, String> {
        Ok(u64::from_be_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn i64(&mut self) -> Result<i64, String> {
        Ok(i64::from_be_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn bytes(&mut self) -> Result<&'a [u8], String> {
        let size = self.u32()? as usize;
        self.take(size)
    }
    fn string(&mut self) -> Result<String, String> {
        String::from_utf8(self.bytes()?.to_vec())
            .map_err(|_| "bytecode artifact contains invalid UTF-8".into())
    }
    fn option_string(&mut self) -> Result<Option<String>, String> {
        if self.boolean()? {
            Ok(Some(self.string()?))
        } else {
            Ok(None)
        }
    }
    fn option_u16(&mut self) -> Result<Option<u16>, String> {
        if self.boolean()? {
            Ok(Some(self.u16()?))
        } else {
            Ok(None)
        }
    }
    fn option_u32(&mut self) -> Result<Option<u32>, String> {
        if self.boolean()? {
            Ok(Some(self.u32()?))
        } else {
            Ok(None)
        }
    }
    fn many<T>(
        &mut self,
        mut read: impl FnMut(&mut Reader<'a>) -> Result<T, String>,
    ) -> Result<Vec<T>, String> {
        let size = self.u32()? as usize;
        let mut values = Vec::with_capacity(size.min(4096));
        for _ in 0..size {
            values.push(read(self)?);
        }
        Ok(values)
    }
    fn finish(&self) -> Result<(), String> {
        if self.cursor == self.bytes.len() {
            Ok(())
        } else {
            Err("bytecode artifact has trailing payload bytes".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::{compile_source, disassemble, execute_program};

    #[test]
    fn programs_round_trip_and_execute() {
        let source = "(do (defn add-one [x] (+ x 1)) (add-one 41))";
        let program = compile_source(source).unwrap();
        let encoded = encode_program(&program).unwrap();
        let decoded = decode_program(&encoded).unwrap();
        assert_eq!(disassemble(&decoded), disassemble(&program));
        assert_eq!(
            execute_program(Rc::new(decoded)).unwrap(),
            Value::Number(42)
        );
    }

    #[test]
    fn corruption_is_rejected_before_decode() {
        let program = compile_source("(+ 19 23)").unwrap();
        let mut encoded = encode_program(&program).unwrap();
        encoded[12] ^= 1;
        assert_eq!(
            decode_program(&encoded).unwrap_err(),
            "bytecode artifact checksum mismatch"
        );
    }
}
