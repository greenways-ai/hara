//! Deterministic human-readable disassembler. Used in tests and in
//! benchmark diagnostics.

use super::opcode::Instruction;
use super::program::Program;

/// Renders a program with instruction offsets, operands, constant
/// previews, jump destinations, and source positions where available.
pub fn disassemble(program: &Program) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "== program: {} constants, {} functions, entry {} ==\n",
        program.constants.len(),
        program.functions.len(),
        program.entry
    ));
    for (index, function) in program.functions.iter().enumerate() {
        let name = function.name.as_deref().unwrap_or("<anonymous>");
        out.push_str(&format!(
            "== fn {index} {name} (arity={}, locals={}, max_stack={}) ==\n",
            function.arity, function.local_count, function.max_stack
        ));
        for (ip, instruction) in function.code.iter().enumerate() {
            let mut line = match instruction {
                Instruction::Jump(target) => format!("{ip:04}  Jump -> {target:04}"),
                Instruction::JumpIfFalse(target) => {
                    format!("{ip:04}  JumpIfFalse -> {target:04}")
                }
                Instruction::Constant(constant) => {
                    let mut line = format!("{ip:04}  Constant {constant}");
                    if let Some(value) = program.constants.get(*constant as usize) {
                        line.push_str(&format!("  ; {}", preview(&value.display())));
                    }
                    line
                }
                _ => format!("{ip:04}  {instruction}"),
            };
            if let Some(position) = function.source_map.position(ip) {
                line.push_str(&format!(
                    "  [line {}, column {}]",
                    position.line, position.column
                ));
            }
            line.push('\n');
            out.push_str(&line);
        }
    }
    out
}

fn preview(text: &str) -> String {
    const LIMIT: usize = 32;
    if text.chars().count() > LIMIT {
        let truncated: String = text.chars().take(LIMIT - 1).collect();
        format!("{truncated}…")
    } else {
        text.to_string()
    }
}
