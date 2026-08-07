pub use hara_wasm::{core, kernel, lang, vm, whole_wasm};

#[path = "hara-code-analyzer/json.rs"]
mod json;
#[path = "../source_analyzer.rs"]
mod source_analyzer;

fn main() {
    let mut args = std::env::args().skip(1);
    match (args.next().as_deref(), args.next(), args.next()) {
        (Some("analyzer"), Some(module), None) => {
            if let Err(error) = source_analyzer::run_jsonl(std::path::Path::new(&module)) {
                eprintln!("hara-rust-full analyzer: {error}");
                std::process::exit(1);
            }
        }
        _ => {
            eprintln!("usage: hara-rust-full analyzer MODULE.hal");
            std::process::exit(2);
        }
    }
}
