use crate::{eval_bytecode_native, Runtime};

fn agrees(source: &str) {
    let expected = Runtime::new().eval_native(source).unwrap();
    assert_eq!(eval_bytecode_native(source).unwrap(), expected, "{source}");
}

#[test]
fn hot_arithmetic_branch_and_nested_loops_match_the_evaluator() {
    for source in [
        "(loop [i 0 acc 0] (if (< i 5000) (recur (+ i 1) (+ acc i)) acc))",
        "(loop [i 0 acc 0] (if (< i 5000) (recur (+ i 1) (+ acc (if (< (mod i 2) 1) 3 7))) acc))",
        "(loop [i 0 acc 1] (if (< i 5000) (recur (+ i 1) (mod (+ acc i) 1000003)) acc))",
        "(loop [i 0 total 0] (if (< i 100) (recur (+ i 1) (+ total (loop [j 0 subtotal 0] (if (< j 100) (recur (+ j 1) (+ subtotal j)) subtotal)))) total))",
    ] {
        agrees(source);
    }
}

#[test]
fn hot_loop_errors_remain_interpreter_errors() {
    let source = "(loop [i 0 x 1] (if (< i 100) (recur (+ i 1) (* x 1000000000)) x))";
    let evaluator = Runtime::new().eval_native(source).unwrap_err();
    let vm = eval_bytecode_native(source).unwrap_err();
    assert!(evaluator.contains("integer overflow"));
    assert!(vm.contains("integer overflow"));
}

#[test]
fn compiled_traces_survive_repeated_execution_of_one_program() {
    let program = crate::compile_bytecode(
        "(loop [i 0 acc 0] (if (< i 5000) (recur (+ i 1) (+ acc i)) acc))",
    )
    .unwrap();
    assert_eq!(crate::execute_bytecode(&program).unwrap(), "12497500");
    assert!(crate::vm::machine::cached_trace_count(&program) > 0);
    assert_eq!(crate::execute_bytecode(&program).unwrap(), "12497500");
    assert!(crate::vm::machine::cached_trace_count(&program) > 0);
}
