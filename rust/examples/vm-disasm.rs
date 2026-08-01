//! Prints the bytecode disassembly of a source string (issue #195/#202
//! diagnostics). Usage: cargo run --features bytecode-vm --example vm-disasm -- '<source>'
#[cfg(not(feature = "bytecode-vm"))]
fn main() {
    eprintln!("build with --features bytecode-vm");
    std::process::exit(2);
}

#[cfg(feature = "bytecode-vm")]
fn main() {
    let source = std::env::args().nth(1).unwrap_or_else(|| {
        "(loop [i 0 acc 0] (if (< i 2500) (recur (+ i 1) ((fn [x] (+ x 1)) acc)) acc))".to_string()
    });
    let program = hara_wasm::vm::compile_source(&source).expect("compiles");
    print!("{}", hara_wasm::vm::disassemble(&program));
}
