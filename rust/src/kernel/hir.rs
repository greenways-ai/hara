use super::Form;
use sha2::{Digest, Sha256};

const MAGIC: &[u8] = b"HIR\0";
const FORMAT_VERSION: u16 = 1;
const EXECUTABLE_FOUNDATION_FLAG: u16 = 1;
const HASH_BYTES: usize = 32;
const MAX_PAYLOAD_BYTES: usize = 64 * 1024 * 1024;
const MAX_COLLECTION_ITEMS: i32 = 1_000_000;

const NIL: u8 = 0;
const FALSE: u8 = 1;
const TRUE: u8 = 2;
const LONG: u8 = 3;
const DOUBLE: u8 = 4;
const BIG_INTEGER: u8 = 5;
const BIG_DECIMAL: u8 = 6;
const STRING: u8 = 7;
const CHARACTER: u8 = 8;
const SYMBOL: u8 = 9;
const KEYWORD: u8 = 10;
const LIST: u8 = 11;
const VECTOR: u8 = 12;
const MAP: u8 = 13;
const SET: u8 = 14;
const ORDERED_MAP: u8 = 15;
const ORDERED_SET: u8 = 16;
const REGEX: u8 = 17;

#[derive(Debug, Clone)]
pub struct HirModule {
    pub namespace: String,
    pub resource: String,
    pub source_hash: Vec<u8>,
    pub forms: Vec<Form>,
}

pub fn decode_hir(bytes: &[u8]) -> Result<HirModule, String> {
    let payload = decode_envelope(bytes)?;
    let mut reader = ByteReader::new(&payload);
    let namespace = reader.read_string()?;
    let resource = reader.read_string()?;
    let source_hash = reader.read_bytes(HASH_BYTES)?;
    let form_count = reader.read_count()?;
    let mut forms = Vec::with_capacity(form_count as usize);
    for _ in 0..form_count {
        forms.push(reader.read_value()?);
    }
    if !reader.is_empty() {
        return Err("trailing payload bytes".into());
    }
    Ok(HirModule {
        namespace,
        resource,
        source_hash,
        forms,
    })
}

fn decode_envelope(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut reader = ByteReader::new(bytes);
    let magic = reader.read_bytes(MAGIC.len())?;
    if magic != MAGIC {
        return Err("bad magic".into());
    }
    let version = reader.read_u16()?;
    if version != FORMAT_VERSION {
        return Err(format!("unsupported format version {version}"));
    }
    let flags = reader.read_u16()?;
    if flags != EXECUTABLE_FOUNDATION_FLAG {
        return Err(format!("unsupported flags {flags}"));
    }
    let payload_length = reader.read_u32()? as usize;
    if payload_length > MAX_PAYLOAD_BYTES {
        return Err(format!("invalid payload length {payload_length}"));
    }
    let expected_hash = reader.read_bytes(HASH_BYTES)?;
    let payload = reader.read_bytes(payload_length)?;
    if !reader.is_empty() {
        return Err("trailing bytes".into());
    }
    let actual_hash = Sha256::digest(&payload);
    if actual_hash.as_slice() != expected_hash.as_slice() {
        return Err("payload checksum mismatch".into());
    }
    Ok(payload)
}

struct ByteReader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> ByteReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn is_empty(&self) -> bool {
        self.position >= self.bytes.len()
    }

    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.position)
    }

    fn read_byte(&mut self) -> Result<u8, String> {
        if self.position >= self.bytes.len() {
            return Err("truncated artifact".into());
        }
        let byte = self.bytes[self.position];
        self.position += 1;
        Ok(byte)
    }

    fn read_bytes(&mut self, count: usize) -> Result<Vec<u8>, String> {
        if self.remaining() < count {
            return Err("truncated artifact".into());
        }
        let bytes = self.bytes[self.position..self.position + count].to_vec();
        self.position += count;
        Ok(bytes)
    }

    fn read_u16(&mut self) -> Result<u16, String> {
        let bytes = self.read_bytes(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    fn read_u32(&mut self) -> Result<u32, String> {
        let bytes = self.read_bytes(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_i64(&mut self) -> Result<i64, String> {
        let bytes = self.read_bytes(8)?;
        Ok(i64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn read_f64(&mut self) -> Result<f64, String> {
        let bytes = self.read_bytes(8)?;
        Ok(f64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn read_string(&mut self) -> Result<String, String> {
        let length = self.read_u32()? as usize;
        if length > MAX_PAYLOAD_BYTES {
            return Err(format!("invalid string length {length}"));
        }
        let bytes = self.read_bytes(length)?;
        String::from_utf8(bytes).map_err(|_| "invalid UTF-8 in string".to_string())
    }

    fn read_nullable_string(&mut self) -> Result<Option<String>, String> {
        let present = self.read_byte()? != 0;
        if present {
            Ok(Some(self.read_string()?))
        } else {
            Ok(None)
        }
    }

    fn read_count(&mut self) -> Result<i32, String> {
        let count = self.read_u32()? as i32;
        if count < 0 || count > MAX_COLLECTION_ITEMS {
            return Err(format!("invalid collection count {count}"));
        }
        Ok(count)
    }

    fn read_metadata(&mut self) -> Result<Option<Form>, String> {
        let present = self.read_byte()? != 0;
        if present {
            Ok(Some(self.read_value()?))
        } else {
            Ok(None)
        }
    }

    fn read_value(&mut self) -> Result<Form, String> {
        let opcode = self.read_byte()?;
        match opcode {
            NIL => Ok(Form::Nil),
            FALSE => Ok(Form::Bool(false)),
            TRUE => Ok(Form::Bool(true)),
            LONG => Ok(Form::Number(self.read_i64()?)),
            DOUBLE => Ok(Form::Float(self.read_f64()?)),
            BIG_INTEGER => Ok(Form::BigInteger(self.read_string()?)),
            BIG_DECIMAL => Ok(Form::Decimal(self.read_string()?)),
            STRING => Ok(Form::String(self.read_string()?)),
            CHARACTER => Ok(Form::Character(
                char::from_u32(self.read_u32()?).ok_or("invalid character code point")?,
            )),
            SYMBOL => {
                let namespace = self.read_nullable_string()?;
                let name = self.read_string()?;
                Ok(with_metadata(
                    Form::Symbol(namespaced(namespace, name)),
                    self.read_metadata()?,
                ))
            }
            KEYWORD => {
                let namespace = self.read_nullable_string()?;
                let name = self.read_string()?;
                Ok(with_metadata(
                    Form::Keyword(namespaced(namespace, name)),
                    self.read_metadata()?,
                ))
            }
            LIST => {
                let count = self.read_count()?;
                let items = self.read_values(count)?;
                Ok(with_metadata(Form::List(items), self.read_metadata()?))
            }
            VECTOR => {
                let count = self.read_count()?;
                let items = self.read_values(count)?;
                Ok(with_metadata(Form::Vector(items), self.read_metadata()?))
            }
            MAP | ORDERED_MAP => {
                let count = self.read_count()?;
                let mut entries = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    let key = self.read_value()?;
                    let value = self.read_value()?;
                    entries.push((key, value));
                }
                Ok(with_metadata(Form::Map(entries), self.read_metadata()?))
            }
            SET | ORDERED_SET => {
                let count = self.read_count()?;
                let items = self.read_values(count)?;
                Ok(with_metadata(Form::Set(items), self.read_metadata()?))
            }
            REGEX => Ok(Form::Regex(self.read_string()?)),
            _ => Err(format!("unknown value opcode {opcode}")),
        }
    }

    fn read_values(&mut self, count: i32) -> Result<Vec<Form>, String> {
        let mut values = Vec::with_capacity(count as usize);
        for _ in 0..count {
            values.push(self.read_value()?);
        }
        Ok(values)
    }
}

fn with_metadata(value: Form, metadata: Option<Form>) -> Form {
    match metadata {
        Some(metadata) => Form::Metadata(Box::new(metadata), Box::new(value)),
        None => value,
    }
}

fn namespaced(namespace: Option<String>, name: String) -> String {
    match namespace {
        Some(ns) => format!("{ns}/{name}"),
        None => name,
    }
}

#[cfg(any(test, feature = "hir-encoder"))]
fn write_string(output: &mut Vec<u8>, value: &str) {
    output.extend_from_slice(&(value.len() as u32).to_be_bytes());
    output.extend_from_slice(value.as_bytes());
}

#[cfg(any(test, feature = "hir-encoder"))]
fn write_count(output: &mut Vec<u8>, count: i32) {
    output.extend_from_slice(&count.to_be_bytes());
}

#[cfg(any(test, feature = "hir-encoder"))]
fn write_namespaced(output: &mut Vec<u8>, symbol: &str) {
    if let Some((ns, name)) = symbol.rsplit_once('/') {
        output.push(1);
        write_string(output, ns);
        write_string(output, name);
    } else {
        output.push(0);
        write_string(output, symbol);
    }
}

#[cfg(any(test, feature = "hir-encoder"))]
fn write_values(output: &mut Vec<u8>, values: &[Form]) {
    write_count(output, values.len() as i32);
    for value in values {
        write_value(output, value);
    }
}

#[cfg(any(test, feature = "hir-encoder"))]
fn write_value(output: &mut Vec<u8>, form: &Form) {
    match form {
        Form::Metadata(metadata, value) => write_value_with_metadata(output, value, Some(metadata)),
        _ => write_value_with_metadata(output, form, None),
    }
}

#[cfg(any(test, feature = "hir-encoder"))]
fn write_metadata(output: &mut Vec<u8>, metadata: Option<&Form>) {
    match metadata {
        Some(metadata) => {
            output.push(1);
            write_value(output, metadata);
        }
        None => output.push(0),
    }
}

#[cfg(any(test, feature = "hir-encoder"))]
fn write_value_with_metadata(output: &mut Vec<u8>, form: &Form, metadata: Option<&Form>) {
    match form {
        Form::Nil => output.push(NIL),
        Form::Bool(false) => output.push(FALSE),
        Form::Bool(true) => output.push(TRUE),
        Form::Number(n) => {
            output.push(LONG);
            output.extend_from_slice(&n.to_be_bytes());
        }
        Form::Float(f) => {
            output.push(DOUBLE);
            output.extend_from_slice(&f.to_be_bytes());
        }
        Form::BigInteger(s) => {
            output.push(BIG_INTEGER);
            write_string(output, s);
        }
        Form::Decimal(s) => {
            output.push(BIG_DECIMAL);
            write_string(output, s);
        }
        Form::String(s) => {
            output.push(STRING);
            write_string(output, s);
        }
        Form::Character(c) => {
            output.push(CHARACTER);
            output.extend_from_slice(&(*c as u32).to_be_bytes());
        }
        Form::Symbol(s) => {
            output.push(SYMBOL);
            write_namespaced(output, s);
            write_metadata(output, metadata);
        }
        Form::Keyword(s) => {
            output.push(KEYWORD);
            write_namespaced(output, s);
            write_metadata(output, metadata);
        }
        Form::List(items) => {
            output.push(LIST);
            write_values(output, items);
            write_metadata(output, metadata);
        }
        Form::Vector(items) => {
            output.push(VECTOR);
            write_values(output, items);
            write_metadata(output, metadata);
        }
        Form::Map(entries) => {
            output.push(MAP);
            write_count(output, entries.len() as i32);
            for (key, value) in entries {
                write_value(output, key);
                write_value(output, value);
            }
            write_metadata(output, metadata);
        }
        Form::Set(items) => {
            output.push(SET);
            write_values(output, items);
            write_metadata(output, metadata);
        }
        Form::Regex(s) => {
            output.push(REGEX);
            write_string(output, s);
        }
        Form::Tagged(_, _) | Form::Metadata(_, _) => {
            panic!("test encoder does not support tagged/metadata forms")
        }
    }
}

/// Test-only helper that encodes a minimal HIR artifact from parsed forms.
///
/// This mirrors the v1 format used by `decode_hir` so that integration tests
/// can construct artifacts without depending on an external encoder.
#[cfg(any(test, feature = "hir-encoder"))]
pub fn encode_hir_module(
    namespace: &str,
    resource: &str,
    source: &str,
    forms: Vec<Form>,
) -> Vec<u8> {
    let mut payload = Vec::new();
    write_string(&mut payload, namespace);
    write_string(&mut payload, resource);
    payload.extend_from_slice(Sha256::digest(source.as_bytes()).as_slice());
    write_count(&mut payload, forms.len() as i32);
    for form in forms {
        write_value(&mut payload, &form);
    }
    let mut artifact = Vec::new();
    artifact.extend_from_slice(MAGIC);
    artifact.extend_from_slice(&FORMAT_VERSION.to_be_bytes());
    artifact.extend_from_slice(&EXECUTABLE_FOUNDATION_FLAG.to_be_bytes());
    artifact.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    artifact.extend_from_slice(Sha256::digest(&payload).as_slice());
    artifact.extend_from_slice(&payload);
    artifact
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::parse;

    fn artifact_payload(forms: Vec<Form>) -> Vec<u8> {
        encode_hir_module("demo.ns", "demo.hal", "", forms)
    }

    #[test]
    fn round_trips_primitive_values() {
        let cases = [
            "nil",
            "true",
            "false",
            "42",
            "-7",
            "3.14",
            "\"hello\"",
            "\\x",
            ":key",
            ":ns/key",
            "symbol",
            "ns/symbol",
        ];
        for source in cases {
            let original = parse(source).unwrap();
            let bytes = artifact_payload(vec![original.clone()]);
            let decoded = decode_hir(&bytes).unwrap();
            assert_eq!(decoded.forms.len(), 1);
            assert_eq!(decoded.forms[0], original, "{source}");
        }
    }

    #[test]
    fn round_trips_collections() {
        let original = parse("(do {:a [1 2] :b #{3 4}})").unwrap();
        let bytes = artifact_payload(vec![original.clone()]);
        let decoded = decode_hir(&bytes).unwrap();
        assert_eq!(decoded.forms.len(), 1);
        assert_eq!(decoded.forms[0], original);
    }

    #[test]
    fn round_trips_metadata() {
        let original = parse("^:dynamic *value*").unwrap();
        let bytes = artifact_payload(vec![original.clone()]);
        let decoded = decode_hir(&bytes).unwrap();
        assert_eq!(decoded.forms, vec![original]);
    }

    #[test]
    fn rejects_bad_magic() {
        let mut bytes = artifact_payload(vec![Form::Nil]);
        bytes[0] = 0;
        assert!(decode_hir(&bytes).unwrap_err().contains("bad magic"));
    }

    #[test]
    fn rejects_checksum_mismatch() {
        let mut bytes = artifact_payload(vec![Form::Nil]);
        let last = bytes.len() - 1;
        bytes[last] = bytes[last].wrapping_add(1);
        assert!(decode_hir(&bytes).unwrap_err().contains("checksum"));
    }

    #[test]
    fn decodes_the_truffle_portable_format_golden_artifact() {
        // This is the canonical v1 artifact emitted by Truffle's
        // HirArtifactTest.goldenBytesLockThePortableFormat.  Keep this test
        // independent of Rust's test-only encoder: it is the cross-runtime
        // compatibility boundary, rather than a Rust encoder/decoder
        // round-trip.
        let bytes = hex_bytes(concat!(
            "48495200000100010000014b7640e14591506ea3c5e004467edc15b2ea8bb319",
            "3b48a4596d99c242ca5531a000000001740000000174e3b0c44298fc1c149afb",
            "f4c8996fb92427ae41e4649b934ca495991b7852b85500000012000102030000",
            "00000000002a044004000000000000050000001e313233343536373839303132",
            "3334353637383930313233343536373839300600000007332e31343135390700",
            "00000668c3a172c3a008000000780901000000056d792e6e73000000066d792d",
            "73796d000a00000000026b77000b000000020300000000000000010700000001",
            "61000c00000002030000000000000001070000000161000d0000000203000000",
            "0000000001070000000161030000000000000002070000000162000e00000002",
            "030000000000000001030000000000000002000f000000020300000000000000",
            "0207000000016203000000000000000107000000016100100000000203000000",
            "0000000002030000000000000001001100000003612b62",
        ));

        let module = decode_hir(&bytes).unwrap();
        assert_eq!(module.namespace, "t");
        assert_eq!(module.resource, "t");
        assert_eq!(module.forms.len(), 18);
        assert_eq!(module.forms[0], Form::Nil);
        assert_eq!(module.forms[3], Form::Number(42));
        assert_eq!(module.forms[7], Form::String("hárà".into()));
        assert_eq!(module.forms[8], Form::Character('x'));
        assert_eq!(module.forms[9], Form::Symbol("my.ns/my-sym".into()));
        assert_eq!(module.forms[10], Form::Keyword("kw".into()));
        assert_eq!(module.forms[17], Form::Regex("a+b".into()));
    }

    fn hex_bytes(hex: &str) -> Vec<u8> {
        (0..hex.len())
            .step_by(2)
            .map(|index| u8::from_str_radix(&hex[index..index + 2], 16).unwrap())
            .collect()
    }
}
