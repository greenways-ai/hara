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
            "== fn {index} {name} (arity={}, captures={}, locals={}, max_stack={}) ==\n",
            function.arity, function.capture_count, function.local_count, function.max_stack
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
                Instruction::Closure { prototype, .. }
                | Instruction::CallStatic { prototype, .. } => {
                    let mut line = format!("{ip:04}  {instruction}");
                    if let Some(target) = program.functions.get(usize::from(*prototype)) {
                        let name = target.name.as_deref().unwrap_or("<anonymous>");
                        line.push_str(&format!("  ; fn {prototype:04} {name}"));
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
        for entry in &function.handlers {
            out.push_str(&format!(
                "  try [{:04}..{:04}) depth={}\n",
                entry.start, entry.end, entry.depth
            ));
            for catch in &entry.catches {
                out.push_str(&format!(
                    "    catch {} -> slot {} @ {:04}\n",
                    catch.class, catch.binding, catch.target
                ));
            }
            if let Some(finally) = entry.finally {
                out.push_str(&format!(
                    "    finally @ {finally:04} pending=({}, {})\n",
                    entry.pending_value.expect("validated"),
                    entry.pending_error.expect("validated")
                ));
            }
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
