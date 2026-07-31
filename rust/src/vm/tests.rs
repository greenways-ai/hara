//! Unit tests for the program model, validator, and disassembler.
//! Compiler, machine, and differential tests live in later modules.

use super::disassemble::disassemble;
use super::error::ValidationError;
use super::opcode::Instruction;
use super::program::{FunctionPrototype, Program, MAX_OPERAND_STACK};
use super::source_map::SourceMap;
use super::validate::validate;
use crate::core::{Primitive, Value};
use crate::kernel::Position;

fn source_map(len: usize) -> SourceMap {
    let mut map = SourceMap::default();
    for _ in 0..len {
        map.record(None);
    }
    map
}

fn program(
    code: Vec<Instruction>,
    constants: Vec<Value>,
    local_count: u16,
    max_stack: u16,
) -> Program {
    let source_map = source_map(code.len());
    Program {
        constants,
        functions: vec![FunctionPrototype {
            name: None,
            arity: 0,
            capture_count: 0,
            local_count,
            max_stack,
            code,
            source_map,
        }],
        entry: 0,
    }
}

/// `(+ 1 2)` compiled by hand.
fn add_program() -> Program {
    program(
        vec![
            Instruction::Constant(0),
            Instruction::Constant(1),
            Instruction::Primitive {
                op: Primitive::Add,
                argc: 2,
            },
            Instruction::Return,
        ],
        vec![Value::Number(1), Value::Number(2)],
        0,
        2,
    )
}

/// One prototype of a multi-function program. `max_stack` is 1: enough
/// for the hand-built shapes below, and the validator recomputes it.
fn prototype(
    name: Option<&str>,
    arity: u16,
    capture_count: u16,
    code: Vec<Instruction>,
) -> FunctionPrototype {
    FunctionPrototype {
        name: name.map(str::to_string),
        arity,
        capture_count,
        local_count: arity + capture_count,
        max_stack: 1,
        source_map: source_map(code.len()),
        code,
    }
}

/// Entry creates and calls a zero-argument, zero-capture closure:
/// `Closure 1 / Call 0 / Return` with the target `Nil / Return`.
fn closure_call_program() -> Program {
    Program {
        constants: vec![],
        functions: vec![
            prototype(
                None,
                0,
                0,
                vec![
                    Instruction::Closure {
                        prototype: 1,
                        captures: 0,
                    },
                    Instruction::Call { argc: 0 },
                    Instruction::Return,
                ],
            ),
            prototype(Some("f"), 0, 0, vec![Instruction::Nil, Instruction::Return]),
        ],
        entry: 0,
    }
}

/// Hand-compiled `if` shape: `True / JumpIfFalse else / then / Jump end /
/// else / end / Return`.
fn if_program() -> Program {
    program(
        vec![
            Instruction::True,
            Instruction::JumpIfFalse(4),
            Instruction::Constant(0),
            Instruction::Jump(5),
            Instruction::Constant(1),
            Instruction::Return,
        ],
        vec![Value::Number(42), Value::Number(0)],
        0,
        1,
    )
}

fn invalid(program: &Program) -> String {
    validate(program)
        .expect_err("program must be rejected")
        .to_string()
}

#[test]
fn instruction_display_and_shape() {
    assert_eq!(Instruction::Constant(7).to_string(), "Constant 7");
    assert_eq!(Instruction::LoadLocal(3).to_string(), "LoadLocal 3");
    assert_eq!(Instruction::StoreLocal(3).to_string(), "StoreLocal 3");
    assert_eq!(
        Instruction::Primitive {
            op: Primitive::Remainder,
            argc: 2
        }
        .to_string(),
        "Primitive % 2"
    );
    assert_eq!(Instruction::Jump(12).to_string(), "Jump 0012");
    assert_eq!(
        Instruction::Closure {
            prototype: 1,
            captures: 2
        }
        .to_string(),
        "Closure 0001 captures 2"
    );
    assert_eq!(Instruction::Call { argc: 3 }.to_string(), "Call 3");
    assert_eq!(
        Instruction::CallStatic {
            prototype: 2,
            argc: 1
        }
        .to_string(),
        "CallStatic 0002 1"
    );
    assert_eq!(Instruction::JumpIfFalse(4).jump_target(), Some(4));
    assert_eq!(Instruction::Return.jump_target(), None);
    assert!(!Instruction::Jump(0).falls_through());
    assert!(!Instruction::Return.falls_through());
    assert!(Instruction::Pop.falls_through());
    assert_eq!(
        Instruction::Primitive { op: Primitive::Add, argc: 3 }.stack_effect(),
        Some(-2)
    );
    assert_eq!(
        Instruction::Closure {
            prototype: 0,
            captures: 2
        }
        .stack_effect(),
        Some(-1)
    );
    assert_eq!(Instruction::Call { argc: 3 }.stack_effect(), Some(-3));
    assert_eq!(
        Instruction::CallStatic {
            prototype: 0,
            argc: 2
        }
        .stack_effect(),
        Some(-1)
    );
    assert_eq!(Instruction::Return.stack_effect(), None);
}

#[test]
fn valid_programs_pass_validation() {
    validate(&add_program()).expect("add program");
    validate(&if_program()).expect("if program");
    validate(&closure_call_program()).expect("closure call program");
}

#[test]
fn validator_rejects_bad_closure_prototype() {
    let mut program = closure_call_program();
    program.functions[0].code[0] = Instruction::Closure {
        prototype: 9,
        captures: 0,
    };
    assert!(invalid(&program).contains("closure prototype 9 out of range"));
}

#[test]
fn validator_rejects_closure_capture_mismatch() {
    let mut program = closure_call_program();
    program.functions[0].code[0] = Instruction::Closure {
        prototype: 1,
        captures: 1,
    };
    let message = invalid(&program);
    assert!(
        message.contains("closure captures 1 but prototype expects 0"),
        "{message}"
    );
}

#[test]
fn validator_rejects_bad_callstatic_target() {
    let mut program = closure_call_program();
    program.functions[0].code[1] = Instruction::CallStatic {
        prototype: 9,
        argc: 0,
    };
    assert!(invalid(&program).contains("callstatic target 9 out of range"));
}

#[test]
fn validator_rejects_callstatic_arity_mismatch() {
    let mut program = closure_call_program();
    program.functions[1].arity = 1;
    program.functions[1].local_count = 1;
    program.functions[0].code[1] = Instruction::CallStatic {
        prototype: 1,
        argc: 0,
    };
    let message = invalid(&program);
    assert!(
        message.contains("callstatic argc 0 but prototype expects 1"),
        "{message}"
    );
}

#[test]
fn validator_rejects_callstatic_capture_mismatch() {
    // Entry directly self-calls a prototype with a different capture
    // count; no Closure instruction masks the CallStatic check.
    let program = Program {
        constants: vec![],
        functions: vec![
            prototype(
                None,
                0,
                0,
                vec![
                    Instruction::CallStatic {
                        prototype: 1,
                        argc: 0,
                    },
                    Instruction::Return,
                ],
            ),
            prototype(Some("f"), 0, 1, vec![Instruction::Nil, Instruction::Return]),
        ],
        entry: 0,
    };
    let message = invalid(&program);
    assert!(
        message.contains("callstatic capture count differs from current function"),
        "{message}"
    );
}

#[test]
fn validator_rejects_bad_constant_index() {
    let mut program = add_program();
    program.functions[0].code[0] = Instruction::Constant(9);
    assert!(invalid(&program).contains("constant index 9 out of range"));
}

#[test]
fn validator_rejects_bad_local_slots() {
    for instruction in [Instruction::LoadLocal(1), Instruction::StoreLocal(1)] {
        let mut program = add_program();
        program.functions[0].code[0] = instruction;
        assert!(invalid(&program).contains("local slot 1 out of range"));
    }
}

#[test]
fn validator_rejects_bad_jump_targets() {
    for (index, instruction) in [(1, Instruction::JumpIfFalse(9)), (3, Instruction::Jump(9))] {
        let mut program = if_program();
        program.functions[0].code[index] = instruction;
        assert!(invalid(&program).contains("jump target 9 out of range"));
    }
}

#[test]
fn validator_rejects_stack_underflow() {
    let program = program(
        vec![Instruction::Pop, Instruction::Nil, Instruction::Return],
        vec![],
        0,
        1,
    );
    assert!(invalid(&program).contains("stack underflow"));
}

#[test]
fn validator_rejects_primitive_underflow() {
    let program = program(
        vec![
            Instruction::True,
            Instruction::Primitive {
                op: Primitive::Add,
                argc: 3,
            },
            Instruction::Return,
        ],
        vec![],
        0,
        1,
    );
    assert!(invalid(&program).contains("stack underflow"));
}

#[test]
fn validator_rejects_inconsistent_join_heights() {
    // `then` falls into the join with an extra value on the stack.
    let program = program(
        vec![
            Instruction::True,
            Instruction::JumpIfFalse(3),
            Instruction::Constant(0),
            Instruction::Return,
        ],
        vec![Value::Number(1)],
        0,
        2,
    );
    assert!(invalid(&program).contains("inconsistent stack heights"));
}

#[test]
fn validator_rejects_missing_return() {
    let program = program(vec![Instruction::True], vec![], 0, 1);
    assert!(invalid(&program).contains("missing return"));
}

#[test]
fn validator_rejects_return_at_wrong_height() {
    let program = program(
        vec![
            Instruction::True,
            Instruction::Constant(0),
            Instruction::Return,
        ],
        vec![Value::Number(1)],
        0,
        2,
    );
    assert!(invalid(&program).contains("return with stack height 2"));
}

#[test]
fn validator_rejects_unreachable_instructions() {
    let program = program(
        vec![
            Instruction::True,
            Instruction::Jump(3),
            Instruction::Pop,
            Instruction::Return,
        ],
        vec![],
        0,
        1,
    );
    let message = invalid(&program);
    assert!(message.contains("unreachable instruction"), "{message}");
    assert!(message.contains("0002"), "{message}");
}

#[test]
fn validator_rejects_empty_code() {
    let program = program(vec![], vec![], 0, 0);
    assert!(invalid(&program).contains("function has no code"));
}

#[test]
fn validator_rejects_operand_stack_overflow() {
    let mut code = vec![Instruction::Nil; MAX_OPERAND_STACK + 1];
    code.push(Instruction::Return);
    let program = program(code, vec![], 0, 0);
    assert!(invalid(&program).contains("operand stack exceeds limit"));
}

#[test]
fn validator_rejects_max_stack_mismatch() {
    let mut program = add_program();
    program.functions[0].max_stack = 3;
    assert!(invalid(&program).contains("declared max_stack 3 disagrees with computed 2"));
}

#[test]
fn validator_rejects_source_map_mismatch() {
    let mut program = add_program();
    program.functions[0].source_map = SourceMap::default();
    assert!(invalid(&program).contains("source map length"));
}

#[test]
fn validator_rejects_missing_entry_function() {
    let mut program = add_program();
    program.entry = 7;
    assert_eq!(invalid(&program), "validation failed: entry function index out of range");
    program = add_program();
    program.functions.clear();
    assert_eq!(invalid(&program), "validation failed: program has no functions");
}

#[test]
fn validation_error_display_includes_instruction() {
    let error = ValidationError::new("stack underflow", Some(12));
    assert_eq!(error.to_string(), "validation failed at 0012: stack underflow");
}

#[test]
fn disassembler_renders_offsets_operands_and_positions() {
    let mut program = if_program();
    let position = Position {
        offset: 5,
        line: 1,
        column: 6,
    };
    let mut map = SourceMap::default();
    for index in 0..program.functions[0].code.len() {
        map.record((index == 1).then_some(position));
    }
    program.functions[0].source_map = map;
    let expected = "\
== program: 2 constants, 1 functions, entry 0 ==
== fn 0 <anonymous> (arity=0, captures=0, locals=0, max_stack=1) ==
0000  True
0001  JumpIfFalse -> 0004  [line 1, column 6]
0002  Constant 0  ; 42
0003  Jump -> 0005
0004  Constant 1  ; 0
0005  Return
";
    assert_eq!(disassemble(&program), expected);
}

#[test]
fn disassembler_resolves_closure_targets() {
    let expected = "\
== program: 0 constants, 2 functions, entry 0 ==
== fn 0 <anonymous> (arity=0, captures=0, locals=0, max_stack=1) ==
0000  Closure 0001 captures 0  ; fn 0001 f
0001  Call 0
0002  Return
== fn 1 f (arity=0, captures=0, locals=0, max_stack=1) ==
0000  Nil
0001  Return
";
    assert_eq!(disassemble(&closure_call_program()), expected);
}

#[test]
fn disassembler_truncates_long_constant_previews() {
    let long = "a".repeat(64);
    let program = program(
        vec![Instruction::Constant(0), Instruction::Return],
        vec![Value::String(long)],
        0,
        1,
    );
    let text = disassemble(&program);
    let preview = text.lines().nth(2).expect("constant line");
    assert!(preview.contains('…'), "{preview}");
}
