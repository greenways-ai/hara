//! Deterministic container for the eagerly loaded Foundation bytecode modules.

use sha2::{Digest, Sha256};

use crate::{core, kernel, Runtime, EAGER_HAL_RESOURCES, EMBEDDED_HAL_RESOURCES};

const MAGIC: &[u8; 4] = b"HBB1";

pub struct ModuleSource<'a> {
    pub resource: &'a str,
    pub source: &'a str,
}

struct Module<'a> {
    resource: &'a str,
    namespace_form: &'a str,
    artifact: &'a [u8],
}

pub fn embedded_foundation_sources() -> Vec<ModuleSource<'static>> {
    std::iter::once("std.foundation")
        .chain(EAGER_HAL_RESOURCES.iter().copied())
        .map(|resource| {
            let source = EMBEDDED_HAL_RESOURCES
                .iter()
                .find_map(|(name, _, source)| (*name == resource).then_some(*source))
                .unwrap_or_else(|| panic!("missing embedded HAL resource: {resource}"));
            ModuleSource { resource, source }
        })
        .collect()
}

pub fn compile_bytecode_bundle(sources: &[ModuleSource<'_>]) -> Result<Vec<u8>, String> {
    let mut runtime = Runtime::core();
    let mut encoded = Vec::new();
    for source in sources {
        let (namespace_form, body) = split_namespace_form(source.source)?;
        runtime.eval_native(namespace_form)?;
        if source.resource == "std.foundation" {
            runtime.prepare_foundation_bytecode();
        }
        let artifact = runtime.compile_bytecode_artifact(body)?;
        core::with_definition_origin(kernel::VarOrigin::HalFallback, || {
            runtime.eval_bytecode_artifact(&artifact)
        })?;
        encoded.push((source.resource, namespace_form, artifact));
    }

    let modules = encoded
        .iter()
        .map(|(resource, namespace_form, artifact)| Module {
            resource,
            namespace_form,
            artifact,
        })
        .collect::<Vec<_>>();
    encode(&modules)
}

pub fn compile_embedded_foundation_bundle() -> Result<Vec<u8>, String> {
    compile_bytecode_bundle(&embedded_foundation_sources())
}

pub fn eval_bytecode_bundle(runtime: &mut Runtime, bytes: &[u8]) -> Result<(), String> {
    for module in decode(bytes)? {
        runtime.eval_native(&module.namespace_form)?;
        if module.resource == "std.foundation" {
            runtime.prepare_foundation_bytecode();
        }
        core::with_definition_origin(kernel::VarOrigin::HalFallback, || {
            runtime.eval_bytecode_artifact(&module.artifact)
        })?;
    }
    runtime.use_namespace("user");
    Ok(())
}

struct OwnedModule {
    resource: String,
    namespace_form: String,
    artifact: Vec<u8>,
}

fn encode(modules: &[Module<'_>]) -> Result<Vec<u8>, String> {
    let mut payload = Vec::new();
    put_u32(&mut payload, modules.len())?;
    for module in modules {
        put_bytes(&mut payload, module.resource.as_bytes())?;
        put_bytes(&mut payload, module.namespace_form.as_bytes())?;
        put_bytes(&mut payload, module.artifact)?;
    }
    let checksum = Sha256::digest(&payload);
    let mut output = Vec::with_capacity(4 + checksum.len() + payload.len());
    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&checksum);
    output.extend_from_slice(&payload);
    Ok(output)
}

fn decode(bytes: &[u8]) -> Result<Vec<OwnedModule>, String> {
    if bytes.len() < 36 || &bytes[..4] != MAGIC {
        return Err("invalid foundation bytecode bundle header".into());
    }
    let payload = &bytes[36..];
    if Sha256::digest(payload).as_slice() != &bytes[4..36] {
        return Err("foundation bytecode bundle checksum mismatch".into());
    }
    let mut input = payload;
    let count = take_u32(&mut input)? as usize;
    let mut modules = Vec::with_capacity(count);
    for _ in 0..count {
        modules.push(OwnedModule {
            resource: take_string(&mut input)?,
            namespace_form: take_string(&mut input)?,
            artifact: take_bytes(&mut input)?.to_vec(),
        });
    }
    if !input.is_empty() {
        return Err("trailing bytes in foundation bytecode bundle".into());
    }
    Ok(modules)
}

fn split_namespace_form(source: &str) -> Result<(&str, &str), String> {
    let start = source.find("(ns ").ok_or("HAL module is missing ns form")?;
    let mut depth = 0usize;
    let mut string = false;
    let mut escape = false;
    for (offset, ch) in source[start..].char_indices() {
        if string {
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                string = false;
            }
            continue;
        }
        match ch {
            '"' => string = true,
            '(' => depth += 1,
            ')' => {
                depth = depth.checked_sub(1).ok_or("invalid ns form")?;
                if depth == 0 {
                    let end = start + offset + ch.len_utf8();
                    return Ok((&source[start..end], &source[end..]));
                }
            }
            _ => {}
        }
    }
    Err("unterminated ns form".into())
}

fn put_u32(output: &mut Vec<u8>, value: usize) -> Result<(), String> {
    let value = u32::try_from(value).map_err(|_| "foundation bundle exceeds u32 limits")?;
    output.extend_from_slice(&value.to_le_bytes());
    Ok(())
}

fn put_bytes(output: &mut Vec<u8>, value: &[u8]) -> Result<(), String> {
    put_u32(output, value.len())?;
    output.extend_from_slice(value);
    Ok(())
}

fn take_u32(input: &mut &[u8]) -> Result<u32, String> {
    let bytes = take(input, 4)?;
    Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
}

fn take_bytes<'a>(input: &mut &'a [u8]) -> Result<&'a [u8], String> {
    let len = take_u32(input)? as usize;
    take(input, len)
}

fn take_string(input: &mut &[u8]) -> Result<String, String> {
    String::from_utf8(take_bytes(input)?.to_vec())
        .map_err(|_| "foundation bundle contains invalid UTF-8".into())
}

fn take<'a>(input: &mut &'a [u8], len: usize) -> Result<&'a [u8], String> {
    if input.len() < len {
        return Err("truncated foundation bytecode bundle".into());
    }
    let (value, rest) = input.split_at(len);
    *input = rest;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_bundle_round_trips_and_bootstraps() {
        let bytes = compile_embedded_foundation_bundle().expect("compile foundation bundle");
        let mut runtime = Runtime::core();
        eval_bytecode_bundle(&mut runtime, &bytes).expect("load foundation bundle");
        let publics = runtime
            .eval_native("(keys (ns-publics 'std.foundation.string))")
            .expect("inspect string namespace");
        assert!(publics.contains("upper"), "{publics}");
        assert!(runtime.use_namespace("std.foundation.string"));
        assert_eq!(runtime.eval_native("(upper \"hara\")").unwrap(), "\"HARA\"");
        assert!(runtime.use_namespace("std.foundation"));
        assert_eq!(runtime.eval_native("(if-not false 42)").unwrap(), "42");
    }
}
