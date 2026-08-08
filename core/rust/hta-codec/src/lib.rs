//! Dependency-free canonical HTA1 codec for portable Hara ABI values.
//!
//! `hara-hta` deliberately operates on [`hara_abi::Value`] rather than the
//! executable runtime value graph. It is suitable for native providers,
//! package tooling, embedding hosts, and durable state boundaries that need
//! canonical Hara bytes without linking the VM, Wasmtime, or host services.

use hara_abi::Value;
use std::collections::BTreeMap;

pub const MAGIC: &[u8; 4] = b"HTA1";
pub const MAX_FRAME_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_NESTING_DEPTH: usize = 256;

const NIL: u8 = 0;
const FALSE: u8 = 1;
const TRUE: u8 = 2;
const I64: u8 = 3;
const STRING: u8 = 4;
const BYTES: u8 = 5;
const KEYWORD: u8 = 6;
const SYMBOL: u8 = 7;
const LIST: u8 = 8;
const VECTOR: u8 = 9;
const SET: u8 = 10;
const MAP: u8 = 11;
const HANDLE: u8 = 12;
const NAMESPACE: u8 = 13;
const VAR: u8 = 14;
const F64: u8 = 15;
const ATOM: u8 = 16;
const ARRAY: u8 = 17;
const OBJECT: u8 = 18;
const CHARACTER: u8 = 19;
const BIG_INTEGER: u8 = 20;
const DECIMAL: u8 = 21;
const REGEX: u8 = 22;

/// Encode one portable value as an exact canonical HTA1 frame.
pub fn encode(value: &Value) -> Result<Vec<u8>, String> {
    let mut output = Vec::with_capacity(128);
    write(&mut output, MAGIC)?;
    encode_value(value, 0, &mut output)?;
    Ok(output)
}

/// Decode one exact canonical HTA1 frame into a portable value.
///
/// Runtime-only wire tags such as symbols, lists, sets, handles, namespaces,
/// vars, atoms, arrays, objects, characters, big integers, and regex values
/// fail closed. HTA maps decode only when every key is a unique keyword, which
/// maps directly to [`Value::Record`].
pub fn decode(bytes: &[u8]) -> Result<Value, String> {
    if bytes.len() > MAX_FRAME_BYTES {
        return Err(format!(
            "hta/frame-too-large: {} exceeds {} bytes",
            bytes.len(),
            MAX_FRAME_BYTES
        ));
    }
    if !bytes.starts_with(MAGIC) {
        return Err("hta/frame-invalid: expected HTA1 magic".into());
    }
    let mut reader = Reader {
        bytes: &bytes[MAGIC.len()..],
        cursor: 0,
    };
    let value = reader.value(0)?;
    if reader.cursor != reader.bytes.len() {
        return Err("hta/frame-invalid: trailing bytes".into());
    }
    Ok(value)
}

fn encode_value(value: &Value, depth: usize, output: &mut Vec<u8>) -> Result<(), String> {
    if depth > MAX_NESTING_DEPTH {
        return Err("hta/value-too-deep".into());
    }
    match value {
        Value::Nil => push(output, NIL),
        Value::Boolean(false) => push(output, FALSE),
        Value::Boolean(true) => push(output, TRUE),
        Value::Integer(value) => {
            push(output, I64)?;
            write(output, &value.to_be_bytes())
        }
        Value::Float(value) => {
            push(output, F64)?;
            write(output, &value.to_bits().to_be_bytes())
        }
        Value::String(value) => write_sized(output, STRING, value.as_bytes()),
        Value::Bytes(value) => write_sized(output, BYTES, value),
        Value::Keyword(value) => write_sized(output, KEYWORD, value.as_bytes()),
        Value::Decimal(value) => write_sized(output, DECIMAL, value.as_bytes()),
        Value::Vector(values) => {
            push(output, VECTOR)?;
            write_len(output, values.len())?;
            for value in values {
                encode_value(value, depth + 1, output)?;
            }
            Ok(())
        }
        Value::Record(values) => encode_record(values, depth, output),
    }
}

fn encode_record(
    values: &BTreeMap<String, Value>,
    depth: usize,
    output: &mut Vec<u8>,
) -> Result<(), String> {
    push(output, MAP)?;
    write_len(output, values.len())?;

    let mut entries = Vec::with_capacity(values.len());
    for (key, value) in values {
        let mut key_bytes = Vec::with_capacity(key.len() + 5);
        encode_value(&Value::Keyword(key.clone()), depth + 1, &mut key_bytes)?;
        let mut value_bytes = Vec::new();
        encode_value(value, depth + 1, &mut value_bytes)?;
        entries.push((key_bytes, value_bytes));
    }
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    for (key, value) in entries {
        write(output, &key)?;
        write(output, &value)?;
    }
    Ok(())
}

fn write_sized(output: &mut Vec<u8>, tag: u8, bytes: &[u8]) -> Result<(), String> {
    push(output, tag)?;
    write_len(output, bytes.len())?;
    write(output, bytes)
}

fn write_len(output: &mut Vec<u8>, len: usize) -> Result<(), String> {
    let len = u32::try_from(len)
        .map_err(|_| "hta/value-too-large: container or scalar length exceeds u32".to_string())?;
    write(output, &len.to_be_bytes())
}

fn push(output: &mut Vec<u8>, byte: u8) -> Result<(), String> {
    write(output, &[byte])
}

fn write(output: &mut Vec<u8>, bytes: &[u8]) -> Result<(), String> {
    let next = output
        .len()
        .checked_add(bytes.len())
        .ok_or_else(|| "hta/frame-too-large: length overflow".to_string())?;
    if next > MAX_FRAME_BYTES {
        return Err(format!(
            "hta/frame-too-large: encoded frame exceeds {} bytes",
            MAX_FRAME_BYTES
        ));
    }
    output.extend_from_slice(bytes);
    Ok(())
}

struct Reader<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl Reader<'_> {
    fn value(&mut self, depth: usize) -> Result<Value, String> {
        if depth > MAX_NESTING_DEPTH {
            return Err("hta/value-too-deep".into());
        }
        match self.byte()? {
            NIL => Ok(Value::Nil),
            FALSE => Ok(Value::Boolean(false)),
            TRUE => Ok(Value::Boolean(true)),
            I64 => Ok(Value::Integer(i64::from_be_bytes(
                self.take(8)?.try_into().expect("eight bytes"),
            ))),
            F64 => Ok(Value::Float(f64::from_bits(u64::from_be_bytes(
                self.take(8)?.try_into().expect("eight bytes"),
            )))),
            STRING => Ok(Value::String(self.text()?)),
            BYTES => Ok(Value::Bytes(self.sized()?.to_vec())),
            KEYWORD => Ok(Value::Keyword(self.text()?)),
            DECIMAL => Ok(Value::Decimal(self.text()?)),
            VECTOR => self.vector(depth),
            MAP => self.record(depth),
            tag @ (SYMBOL | LIST | SET | HANDLE | NAMESPACE | VAR | ATOM | ARRAY | OBJECT
            | CHARACTER | BIG_INTEGER | REGEX) => Err(format!(
                "hta/value-unsupported: runtime wire tag {tag} is not portable"
            )),
            tag => Err(format!("hta/value-malformed: unknown tag {tag}")),
        }
    }

    fn vector(&mut self, depth: usize) -> Result<Value, String> {
        let len = self.len()?;
        if len > self.remaining() {
            return Err("hta/value-malformed: impossible sequence length".into());
        }
        let mut values = Vec::with_capacity(len);
        for _ in 0..len {
            values.push(self.value(depth + 1)?);
        }
        Ok(Value::Vector(values))
    }

    fn record(&mut self, depth: usize) -> Result<Value, String> {
        let len = self.len()?;
        if len > self.remaining() / 2 {
            return Err("hta/value-malformed: impossible map length".into());
        }
        let mut values = BTreeMap::new();
        for _ in 0..len {
            let key = match self.value(depth + 1)? {
                Value::Keyword(key) => key,
                _ => {
                    return Err(
                        "hta/value-unsupported: portable records require keyword keys".into(),
                    )
                }
            };
            let value = self.value(depth + 1)?;
            if values.insert(key.clone(), value).is_some() {
                return Err(format!(
                    "hta/value-malformed: duplicate portable record key :{key}"
                ));
            }
        }
        Ok(Value::Record(values))
    }

    fn text(&mut self) -> Result<String, String> {
        String::from_utf8(self.sized()?.to_vec())
            .map_err(|_| "hta/value-malformed: invalid UTF-8".into())
    }

    fn sized(&mut self) -> Result<&[u8], String> {
        let len = self.len()?;
        self.take(len)
    }

    fn len(&mut self) -> Result<usize, String> {
        Ok(u32::from_be_bytes(
            self.take(4)?.try_into().expect("four bytes"),
        ) as usize)
    }

    fn byte(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }

    fn take(&mut self, size: usize) -> Result<&[u8], String> {
        let end = self
            .cursor
            .checked_add(size)
            .ok_or_else(|| "hta/value-malformed: length overflow".to_string())?;
        if end > self.bytes.len() {
            return Err("hta/value-malformed: truncated value".into());
        }
        let output = &self.bytes[self.cursor..end];
        self.cursor = end;
        Ok(output)
    }

    fn remaining(&self) -> usize {
        self.bytes.len() - self.cursor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(entries: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
        Value::Record(
            entries
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value))
                .collect(),
        )
    }

    #[test]
    fn canonical_portable_round_trip() {
        let value = record([
            (
                "a",
                Value::Vector(vec![Value::Boolean(true), Value::Nil]),
            ),
            ("b", Value::Integer(2)),
            ("bytes", Value::Bytes(vec![0, 1, 255])),
            ("decimal", Value::Decimal("1.2500".into())),
            ("float", Value::Float(0.28)),
            ("keyword", Value::Keyword("profile.primary".into())),
        ]);
        let encoded = encode(&value).unwrap();
        assert_eq!(decode(&encoded).unwrap(), value);
        assert_eq!(encode(&decode(&encoded).unwrap()).unwrap(), encoded);
    }

    #[test]
    fn records_are_canonical_independent_of_construction_order() {
        let first = record([("z", Value::Integer(1)), ("a", Value::Integer(2))]);
        let second = record([("a", Value::Integer(2)), ("z", Value::Integer(1))]);
        assert_eq!(encode(&first).unwrap(), encode(&second).unwrap());
    }

    #[test]
    fn floats_preserve_ieee_754_bits() {
        for value in [0.28, -0.0, f64::INFINITY, f64::NEG_INFINITY] {
            let decoded = decode(&encode(&Value::Float(value)).unwrap()).unwrap();
            let Value::Float(decoded) = decoded else {
                panic!("float value")
            };
            assert_eq!(decoded.to_bits(), value.to_bits());
        }
    }

    #[test]
    fn non_keyword_and_duplicate_record_keys_fail_closed() {
        let mut non_keyword = MAGIC.to_vec();
        non_keyword.extend_from_slice(&[MAP, 0, 0, 0, 1]);
        non_keyword.extend_from_slice(&[STRING, 0, 0, 0, 1, b'a', NIL]);
        assert!(decode(&non_keyword)
            .unwrap_err()
            .contains("portable records require keyword keys"));

        let mut duplicate = MAGIC.to_vec();
        duplicate.extend_from_slice(&[MAP, 0, 0, 0, 2]);
        for value in [NIL, TRUE] {
            duplicate.extend_from_slice(&[KEYWORD, 0, 0, 0, 1, b'a', value]);
        }
        assert!(decode(&duplicate)
            .unwrap_err()
            .contains("duplicate portable record key :a"));
    }

    #[test]
    fn runtime_only_tags_fail_closed() {
        for tag in [
            SYMBOL,
            LIST,
            SET,
            HANDLE,
            NAMESPACE,
            VAR,
            ATOM,
            ARRAY,
            OBJECT,
            CHARACTER,
            BIG_INTEGER,
            REGEX,
        ] {
            let bytes = [MAGIC.as_slice(), &[tag]].concat();
            assert!(decode(&bytes)
                .unwrap_err()
                .contains("runtime wire tag"));
        }
    }

    #[test]
    fn frame_shape_and_lengths_are_bounded() {
        assert!(decode(b"not-hta").unwrap_err().contains("expected HTA1 magic"));

        let trailing = [MAGIC.as_slice(), &[NIL, NIL]].concat();
        assert!(decode(&trailing).unwrap_err().contains("trailing bytes"));

        let mut impossible = MAGIC.to_vec();
        impossible.extend_from_slice(&[VECTOR, 0xff, 0xff, 0xff, 0xff]);
        assert!(decode(&impossible)
            .unwrap_err()
            .contains("impossible sequence length"));
    }

    #[test]
    fn nesting_depth_is_bounded_on_encode_and_decode() {
        let mut value = Value::Nil;
        for _ in 0..=MAX_NESTING_DEPTH {
            value = Value::Vector(vec![value]);
        }
        assert!(encode(&value).unwrap_err().contains("value-too-deep"));

        let mut bytes = MAGIC.to_vec();
        for _ in 0..=MAX_NESTING_DEPTH {
            bytes.extend_from_slice(&[VECTOR, 0, 0, 0, 1]);
        }
        bytes.push(NIL);
        assert!(decode(&bytes).unwrap_err().contains("value-too-deep"));
    }
}
