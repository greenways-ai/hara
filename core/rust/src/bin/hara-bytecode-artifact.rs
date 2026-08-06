//! Compile one Hara source expression to a portable HBC4 artifact.

use hara_wasm::Runtime;
use std::path::Path;

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 {
        eprintln!("usage: hara-bytecode-artifact SOURCE_HEX OUTPUT.hbc");
        std::process::exit(2);
    }
    let source = decode_hex(&args[0]).unwrap_or_else(|error| fail(error));
    let runtime = Runtime::new();
    let artifact = runtime
        .compile_bytecode_artifact(&source)
        .unwrap_or_else(|error| fail(error));
    std::fs::write(Path::new(&args[1]), artifact).unwrap_or_else(|error| fail(error.to_string()));
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

fn fail(message: impl std::fmt::Display) -> ! {
    eprintln!("{message}");
    std::process::exit(1);
}
