use hara_wasm::Runtime;
use std::time::Instant;

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 6 {
        eprintln!("benchmark expects RUNTIME ID SOURCE_HEX EXPECTED WINDOWS CALLS");
        std::process::exit(2);
    }
    let runtime_name = &args[0];
    let id = &args[1];
    let source = decode_hex(&args[2]).unwrap_or_else(|error| fail(id, &error));
    let expected = &args[3];
    let windows: usize = args[4]
        .parse()
        .unwrap_or_else(|_| fail(id, "invalid windows"));
    let calls: usize = args[5]
        .parse()
        .unwrap_or_else(|_| fail(id, "invalid calls"));
    let mut runtime = Runtime::new();
    let started = Instant::now();
    let first = runtime
        .eval_native(&source)
        .unwrap_or_else(|error| fail(id, &error));
    let first_ns = started.elapsed().as_nanos();
    assert_value(id, expected, &first);
    let mut samples = Vec::with_capacity(windows);
    for _ in 0..windows {
        let started = Instant::now();
        for _ in 0..calls {
            let value = runtime
                .eval_native(&source)
                .unwrap_or_else(|error| fail(id, &error));
            assert_value(id, expected, &value);
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
