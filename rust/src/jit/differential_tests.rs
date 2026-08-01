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
    let program =
        crate::compile_bytecode("(loop [i 0 acc 0] (if (< i 5000) (recur (+ i 1) (+ acc i)) acc))")
            .unwrap();
    assert_eq!(crate::execute_bytecode(&program).unwrap(), "12497500");
    assert!(crate::vm::machine::cached_trace_count(&program) > 0);
    assert_eq!(crate::execute_bytecode(&program).unwrap(), "12497500");
    assert!(crate::vm::machine::cached_trace_count(&program) > 0);
}

#[test]
fn indexed_numeric_vectors_trace_from_constants_and_locals() {
    for source in [
        "(loop [i 0 acc 0] (if (< i 5000) (recur (+ i 1) (+ acc (nth [3 5 7 11] (mod i 4)))) acc))",
        "(let [values [3 5 7 11]] (loop [i 0 acc 0] (if (< i 5000) (recur (+ i 1) (+ acc (nth values (mod i 4)))) acc)))",
    ] {
        agrees(source);
        let program = crate::compile_bytecode(source).unwrap();
        let function = &program.functions[usize::from(program.entry)];
        let (backedge, header) = function
            .code
            .iter()
            .enumerate()
            .rev()
            .find_map(|(ip, instruction)| match instruction {
                crate::vm::Instruction::Jump(target) if usize::try_from(*target).ok()? <= ip => {
                    Some((ip as u32, *target))
                }
                _ => None,
            })
            .unwrap();
        let mut locals = vec![crate::jit::TraceValue::I64(64); usize::from(function.local_count)];
        if source.starts_with("(let") {
            locals[0] = crate::jit::TraceValue::Indexed(Box::new(crate::core::Value::Vector(
                [3, 5, 7, 11]
                    .into_iter()
                    .map(crate::core::Value::Number)
                    .collect(),
            )));
        }
        let recorded = crate::jit::TraceRecorder::new(4096).record_loop(
            &program,
            program.entry,
            header,
            backedge,
            &locals,
        );
        assert!(
            recorded.is_ok(),
            "vector loop was rejected: {recorded:?}; constants: {:?}",
            program.constants
        );
        assert_eq!(crate::execute_bytecode(&program).unwrap(), "32500");
        assert!(
            crate::vm::machine::cached_trace_count(&program) > 0,
            "vector loop did not compile: {source}"
        );
    }
}

#[test]
fn unsupported_vectors_and_late_bounds_errors_fall_back_to_vm_semantics() {
    agrees(
        "(loop [i 0 acc 0] (if (< i 5000) (recur (+ i 1) (+ acc (count (nth [\"ab\"] 0)))) acc))",
    );

    let values = (0..256)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    let source = format!("(loop [i 0] (if (< i 5000) (do (nth [{values}] i) (recur (+ i 1))) i))");
    let evaluator = Runtime::new().eval_native(&source).unwrap_err();
    let vm = eval_bytecode_native(&source).unwrap_err();
    assert!(evaluator.contains("nth index out of bounds"), "{evaluator}");
    assert!(vm.contains("nth index out of bounds"), "{vm}");
}
