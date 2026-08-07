pub use hara_wasm::{core, kernel, lang, vm, whole_wasm};

#[path = "hara-code-analyzer/json.rs"]
mod json;
#[path = "../source_analyzer.rs"]
mod source_analyzer;

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(module) = args.next() else {
        eprintln!("usage: hara-code-analyzer MODULE.hal");
        std::process::exit(2);
    };
    if args.next().is_some() {
        eprintln!("hara-code-analyzer accepts exactly one MODULE.hal path");
        std::process::exit(2);
    }
    if let Err(error) = source_analyzer::run_jsonl(std::path::Path::new(&module)) {
        eprintln!("hara-code-analyzer: {error}");
        std::process::exit(1);
    }
}
