//! Benchmark driver for the experimental bytecode VM (issue #195).
//!
//! Same wire protocol as `hara-runtime-benchmark` plus a leading MODE:
//!
//! ```text
//! hara-bytecode-benchmark MODE ID SOURCE_HEX EXPECTED WINDOWS CALLS [RUNTIME]
//! ```
//!
//! Modes:
//!
//! - `existing`        — `Runtime::eval_native` baseline (parse + fiber
//!                       evaluation per call).
//! - `compile-execute` — parse + compile + validate + execute + display
//!                       per call.
//! - `execute-only`    — compile once; execute + display per call.
//!
//! Every call checks the result against EXPECTED (the correctness
//! checksum); a mismatch aborts the run. Output is one JSON line with
//! `first_ns` and the per-window `samples_ns`.

use hara_wasm::Runtime;
use std::time::Instant;

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if !(6..=7).contains(&args.len()) {
        eprintln!("benchmark expects MODE ID SOURCE_HEX EXPECTED WINDOWS CALLS [RUNTIME]");
        std::process::exit(2);
    }
    let mode = &args[0];
    let id = &args[1];
    let source = decode_hex(&args[2]).unwrap_or_else(|error| fail(id, &error));
    let expected = &args[3];
    let windows: usize = args[4]
        .parse()
        .unwrap_or_else(|_| fail(id, "invalid windows"));
    let calls: usize = args[5]
        .parse()
        .unwrap_or_else(|_| fail(id, "invalid calls"));
    let default_runtime_name = match mode.as_str() {
        "existing" => "hara-rust-existing",
        "compile-execute" => "hara-rust-bytecode-compile-execute",
        "execute-only" => "hara-rust-bytecode-execute-only",
        other => fail(id, &format!("unknown mode: {other}")),
    };
    let runtime_name = args.get(6).map(String::as_str).unwrap_or(default_runtime_name);

    let mut runtime = Runtime::new();
    // For execute-only the program is compiled once, outside the samples.
    let program = if mode == "execute-only" {
        Some(hara_wasm::compile_bytecode(&source).unwrap_or_else(|error| fail(id, &error)))
    } else {
        None
    };
    let mut call = || {
        let value = match mode.as_str() {
            "existing" => runtime.eval_native(&source),
            "compile-execute" => hara_wasm::eval_bytecode_native(&source),
            "execute-only" => hara_wasm::execute_bytecode(program.as_ref().expect("program")),
            _ => unreachable!(),
        };
        value.unwrap_or_else(|error| fail(id, &error))
    };

    let started = Instant::now();
    let first = call();
    let first_ns = started.elapsed().as_nanos();
    assert_value(id, expected, &first);
    let mut samples = Vec::with_capacity(windows);
    for _ in 0..windows {
        let started = Instant::now();
        for _ in 0..calls {
            assert_value(id, expected, &call());
        }
        samples.push(started.elapsed().as_nanos() / calls as u128);
    }
    let samples = samples
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",");
    println!(
        "{{\"runtime\":\"{}\",\"workload\":\"{}\",\"first_ns\":{},\"samples_ns\":[{}]}}",
        json(runtime_name),
        json(id),
        first_ns,
        samples
    );
}

fn decode_hex(value: &str) -> Result<String, String> {
    if value.len() % 2 != 0 {
        return Err("invalid source hex".into());
    }
    let bytes = (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "invalid source hex")?;
    String::from_utf8(bytes).map_err(|_| "source is not UTF-8".into())
}

fn assert_value(id: &str, expected: &str, actual: &str) {
    if expected != actual {
        fail(id, &format!("expected {expected}, got {actual}"));
    }
}

fn fail(id: &str, message: &str) -> ! {
    eprintln!("{id}: {message}");
    std::process::exit(1);
}

fn json(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
