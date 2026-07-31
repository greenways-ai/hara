//! Tests for the compiler and the synchronous machine: every instruction
//! exercised end to end, literal/local/control-flow semantics, arithmetic
//! and comparison success and failure, loop/recur behavior, and
//! source-aware diagnostics.

use super::error::CompileErrorKind;
use super::{compile_source, disassemble, eval_source, execute_program};
use crate::core::Value;

fn eval(source: &str) -> String {
    eval_source(source)
        .map(|value| value.display())
        .expect("evaluation must succeed")
}

fn eval_error(source: &str) -> String {
    eval_source(source).expect_err("evaluation must fail")
}

/// Runtime errors append `(instruction NNNN)` to the display; compare the
/// stable message-and-position prefix.
fn assert_eval_error(source: &str, expected_prefix: &str) {
    let message = eval_error(source);
    assert!(
        message.starts_with(expected_prefix),
        "{source}: {message} does not start with {expected_prefix}"
    );
}

fn compile_error(source: &str) -> (CompileErrorKind, String) {
    match compile_source(source) {
        Ok(program) => panic!("expected compile error, got {}", disassemble(&program)),
        Err(error) => (error.kind(), error.to_string()),
    }
}

#[test]
fn literals() {
    assert_eq!(eval("nil"), "nil");
    assert_eq!(eval("true"), "true");
    assert_eq!(eval("false"), "false");
    assert_eq!(eval("42"), "42");
    assert_eq!(eval("-7"), "-7");
    assert_eq!(eval("1.5"), "1.5");
    // `Value::display` renders whole floats without a trailing fraction,
    // matching the existing evaluator's output.
    assert_eq!(eval("2.0"), "2");
    assert_eq!(eval("\"hello\""), "\"hello\"");
    assert_eq!(eval(":hara/name"), ":hara/name");
    assert_eq!(eval("\\a"), "\\a");
    // BigInteger/Decimal literals compile as constants but are not
    // readable by this parser ("Invalid number: 42N").
    assert_eq!(eval("#\"\\d+\""), "#\"\\d+\"");
    assert_eq!(eval("()"), "nil");
    assert_eq!(eval("^:private (+ 1 2)"), "3");
}

#[test]
fn multiple_top_level_forms() {
    assert_eq!(eval("1 2 3"), "3");
    assert_eq!(eval("(+ 1 2) (+ 3 4)"), "7");
}

#[test]
fn local_load_and_store() {
    assert_eq!(eval("(let [x 19] x)"), "19");
    assert_eq!(eval("(let [x 19 y 23] (+ x y))"), "42");
    assert_eq!(eval("(let [x 1] (let [y 2] (+ x y)))"), "3");
}

#[test]
fn sequential_let_bindings_observe_earlier_names() {
    assert_eq!(eval("(let [x 19 y (+ x 23)] y)"), "42");
    assert_eq!(eval("(let [x 1 x (+ x 1)] x)"), "2");
}

#[test]
fn lexical_shadowing() {
    assert_eq!(eval("(let [x 1] (let [x 2] x))"), "2");
    assert_eq!(eval("(let [x 1] (do (let [x 2] x) x))"), "1");
    assert_eq!(eval("(let [x 1 y 2] (+ (let [x 10] x) y))"), "12");
}

#[test]
fn if_branches() {
    assert_eq!(eval("(if true 1 2)"), "1");
    assert_eq!(eval("(if false 1 2)"), "2");
    assert_eq!(eval("(if nil 1 2)"), "2");
    // Everything except nil and false is truthy, including 0 and "".
    assert_eq!(eval("(if 0 1 2)"), "1");
    assert_eq!(eval("(if \"\" 1 2)"), "1");
    assert_eq!(eval("(if false 1)"), "nil");
    assert_eq!(eval("(if (< 19 20) 42 0)"), "42");
}

#[test]
fn do_sequences() {
    assert_eq!(eval("(do)"), "nil");
    assert_eq!(eval("(do 1)"), "1");
    assert_eq!(eval("(do 1 2 3)"), "3");
    assert_eq!(eval("(do (do 1 2) (do 3 4))"), "4");
}

#[test]
fn arithmetic() {
    assert_eq!(eval("(+ 19 23)"), "42");
    assert_eq!(eval("(+ 1 2 3 4)"), "10");
    assert_eq!(eval("(+ 5)"), "5");
    assert_eq!(eval("(- 10 3)"), "7");
    assert_eq!(eval("(* 6 7)"), "42");
    assert_eq!(eval("(/ 17 5)"), "3");
    assert_eq!(eval("(/ -17 5)"), "-3");
    assert_eq!(eval("(% 17 5)"), "2");
    assert_eq!(eval("(mod 17 5)"), "2");
}

#[test]
fn arithmetic_errors() {
    assert_eval_error("(+)", "+ expects arguments [line 1, column 1]");
    assert_eval_error("(/ 1 0)", "division by zero [line 1, column 1]");
    assert_eval_error("(% 1 0)", "division by zero [line 1, column 1]");
    assert_eval_error("(mod 1 0)", "division by zero [line 1, column 1]");
    assert_eval_error(
        "(+ 9223372036854775807 1)",
        "integer overflow [line 1, column 1]",
    );
    assert_eval_error(
        "(- -9223372036854775808 1)",
        "integer overflow [line 1, column 1]",
    );
    assert_eval_error(
        "(* 9223372036854775807 2)",
        "integer overflow [line 1, column 1]",
    );
    assert_eval_error("(+ 1 \"a\")", "+ expects numbers [line 1, column 1]");
    assert_eval_error("(+ 1 1.5)", "+ expects numbers [line 1, column 1]");
    // `mod` reports its operator as `%`, matching the evaluator.
    assert_eval_error("(mod \"a\" 1)", "% expects numbers [line 1, column 1]");
}

#[test]
fn comparisons() {
    assert_eq!(eval("(< 1 2)"), "true");
    assert_eq!(eval("(< 2 1)"), "false");
    assert_eq!(eval("(< 1 2 3)"), "true");
    assert_eq!(eval("(< 1 3 2)"), "false");
    assert_eq!(eval("(<= 1 1)"), "true");
    assert_eq!(eval("(> 2 1)"), "true");
    assert_eq!(eval("(>= 2 3)"), "false");
}

#[test]
fn comparison_errors() {
    assert_eval_error("(< 1)", "< expects at least two arguments [line 1, column 1]");
    assert_eval_error("(< 1 \"a\")", "< expects numbers [line 1, column 1]");
    assert_eval_error("(= 1)", "= expects at least 2 arguments [line 1, column 1]");
}

#[test]
fn equality() {
    assert_eq!(eval("(= 1 1)"), "true");
    assert_eq!(eval("(= 1 2)"), "false");
    assert_eq!(eval("(= 1 1 1 1)"), "true");
    assert_eq!(eval("(= nil nil)"), "true");
    assert_eq!(eval("(= nil false)"), "false");
    assert_eq!(eval("(= \"a\" \"a\")"), "true");
    assert_eq!(eval("(= :a :a)"), "true");
    assert_eq!(eval("(= \\a \\a)"), "true");
    assert_eq!(eval("(= 1.5 1.5)"), "true");
    // Number and Float are distinct values, matching the evaluator.
    assert_eq!(eval("(= 1 1.0)"), "false");
}

#[test]
fn loop_zero_iterations() {
    assert_eq!(eval("(loop [i 0] (if (< i 0) (recur (+ i 1)) i))"), "0");
}

#[test]
fn loop_iterations() {
    assert_eq!(eval("(loop [i 0] (if (< i 1) (recur (+ i 1)) i))"), "1");
    assert_eq!(eval("(loop [i 0] (if (< i 100) (recur (+ i 1)) i))"), "100");
}

#[test]
fn loop_multiple_bindings() {
    assert_eq!(
        eval("(loop [i 0 acc 0] (if (< i 10) (recur (+ i 1) (+ acc i)) acc))"),
        "45"
    );
}

#[test]
fn recur_updates_are_simultaneous() {
    // Each iteration must compute both new values from the old bindings.
    assert_eq!(
        eval("(loop [x 0 y 1] (if (< x 3) (recur (+ x 1) (+ x y)) y))"),
        "4"
    );
    // Swapping two bindings through recur: one swap exchanges them.
    assert_eq!(
        eval("(loop [x 1 y 2 n 0] (if (< n 1) (recur y x (+ n 1)) (- x y)))"),
        "1"
    );
}

#[test]
fn nested_loops() {
    // Inner loop sums i*j for j in 0..3 per outer step: 3i; total 18.
    assert_eq!(
        eval("(loop [i 0 t 0] (if (< i 4) (recur (+ i 1) (+ t (loop [j 0 s 0] (if (< j 3) (recur (+ j 1) (+ s (* i j))) s)))) t))"),
        "18"
    );
}

#[test]
fn loop_body_sequences_like_do() {
    assert_eq!(eval("(loop [i 0] 1 2)"), "2");
    assert_eq!(eval("(loop [i 0] (+ i 1) i)"), "0");
    assert_eq!(eval("(loop [] 7)"), "7");
}

#[test]
fn recur_through_tail_positions() {
    // Tail `let` and `do` bodies and `if` branches are recur positions.
    assert_eq!(
        eval("(loop [i 0] (let [next (+ i 1)] (if (< i 5) (do (recur next)) i)))"),
        "5"
    );
}

#[test]
fn recur_errors() {
    let (kind, message) = compile_error("(recur 1)");
    assert_eq!(kind, CompileErrorKind::Recur);
    assert!(message.contains("recur must be inside loop"), "{message}");
    assert!(message.contains("[line 1, column 1]"), "{message}");

    let (kind, message) = compile_error("(recur)");
    assert_eq!(kind, CompileErrorKind::Recur);
    assert!(message.contains("recur expects values"), "{message}");

    let (kind, message) = compile_error("(loop [i 0] (recur 1 2))");
    assert_eq!(kind, CompileErrorKind::Recur);
    assert!(message.contains("loop recur arity mismatch"), "{message}");

    let (kind, message) = compile_error("(loop [i 0] (+ 1 (recur 2)))");
    assert_eq!(kind, CompileErrorKind::Recur);
    assert!(message.contains("recur must be in tail position"), "{message}");

    let (kind, message) = compile_error("(loop [i 0] (if (recur 1) 2 3))");
    assert_eq!(kind, CompileErrorKind::Recur);
    assert!(message.contains("recur must be in tail position"), "{message}");

    let (kind, message) = compile_error("(loop [i 0] (do (recur 1) i))");
    assert_eq!(kind, CompileErrorKind::Recur);
    assert!(message.contains("recur must be in tail position"), "{message}");
}

#[test]
fn unsupported_forms_are_typed_compile_errors() {
    let cases = [
        ("(def x 1)", "unsupported operator: def"),
        ("(defn f [x] x)", "defn in result position requires var semantics"),
        ("(quote a)", "unsupported operator: quote"),
        ("[1 2 3]", "unsupported form: [1 2 3]"),
        ("{:a 1}", "unsupported form: {:a 1}"),
        ("#{1 2}", "unsupported form: #{1 2}"),
        ("(let [[a b] [1 2]] a)", "let destructuring is not supported"),
        ("(loop [[a b] [1 2]] a)", "loop destructuring is not supported"),
    ];
    for (source, expected) in cases {
        let (kind, message) = compile_error(source);
        assert_eq!(kind, CompileErrorKind::UnsupportedForm, "{source}");
        assert!(message.contains(expected), "{source}: {message}");
        assert!(message.contains("[line 1, column"), "{source}: {message}");
    }
}

#[test]
fn compile_arity_errors_match_evaluator_messages() {
    for (source, expected) in [
        ("(if)", "if expects 2 or 3 arguments [line 1, column 1]"),
        ("(if 1 2 3 4)", "if expects 2 or 3 arguments [line 1, column 1]"),
        ("(let)", "let expects bindings and a body [line 1, column 1]"),
        ("(let [x 1])", "let expects bindings and a body [line 1, column 1]"),
        ("(let 1 x)", "let expects a binding list or vector [line 1, column 6]"),
        ("(let [x] x)", "let bindings require name/value pairs [line 1, column 6]"),
        ("(loop [i 0])", "loop expects bindings and a body [line 1, column 1]"),
        ("(loop 1 2)", "loop expects a binding list or vector [line 1, column 7]"),
        ("(loop [i] i)", "loop bindings require name/value pairs [line 1, column 7]"),
    ] {
        let (kind, message) = compile_error(source);
        assert_eq!(kind, CompileErrorKind::Arity, "{source}");
        assert_eq!(message, expected, "{source}");
    }
}

#[test]
fn unbound_symbols_are_compile_errors_with_positions() {
    let (kind, message) = compile_error("unknown");
    assert_eq!(kind, CompileErrorKind::UnboundSymbol);
    assert_eq!(message, "unbound symbol: unknown [line 1, column 1]");
    let (kind, message) = compile_error("(let [x 1] (+ x y))");
    assert_eq!(kind, CompileErrorKind::UnboundSymbol);
    assert_eq!(message, "unbound symbol: y [line 1, column 17]");
    // Names outside the curated unsupported-operator list report as
    // unbound symbols, matching the evaluator.
    let (kind, message) = compile_error("(first [1 2])");
    assert_eq!(kind, CompileErrorKind::UnboundSymbol);
    assert_eq!(message, "unbound symbol: first [line 1, column 1]");
}

#[test]
fn fn_values_and_direct_calls() {
    assert_eq!(eval("(fn [x] x)"), "<fn>");
    assert_eq!(eval("((fn [x] x) 1)"), "1");
    assert_eq!(eval("((fn [x y] (+ x y)) 19 23)"), "42");
    assert_eq!(eval("(let [f (fn [x] (+ x 1))] (f 41))"), "42");
    // Zero-argument functions.
    assert_eq!(eval("((fn [] 42))"), "42");
}

#[test]
fn closures_capture_lexical_environment() {
    assert_eq!(eval("(let [x 19] ((fn [y] (+ x y)) 23))"), "42");
    // Captures are by value at closure-creation time.
    assert_eq!(
        eval("(let [x 1 f (fn [] x)] (let [x 2] (+ (f) x)))"),
        "3"
    );
    // Nested closures capture through intermediate scopes.
    assert_eq!(
        eval("(((fn [x] (fn [y] (+ x y))) 19) 23)"),
        "42"
    );
    // Loop bindings are capturable.
    assert_eq!(
        eval("(loop [i 0 acc 0] (if (< i 5) (recur (+ i 1) ((fn [x] (+ x i)) acc)) acc))"),
        "10"
    );
}

#[test]
fn defn_lowering_binds_direct_calls() {
    assert_eq!(eval("(do (defn f [x] (+ x 1)) (f 41))"), "42");
    // Later defns shadow earlier ones under early binding.
    assert_eq!(
        eval("(do (defn f [x] (+ x 1)) (defn f [x] (+ x 2)) (f 40))"),
        "42"
    );
    // A defn body sees earlier defns.
    assert_eq!(
        eval("(do (defn g [x] (* x 2)) (defn h [x] (+ (g x) 1)) (h 20))"),
        "41"
    );
    // Self-recursion compiles to a direct static call.
    assert_eq!(
        eval("(do (defn countdown [n] (if (< n 1) 0 (+ 1 (countdown (- n 1))))) (countdown 100))"),
        "100"
    );
}

#[test]
fn call_errors() {
    // Arity mismatch reports through the shared native-function boundary.
    assert_eval_error(
        "((fn [x] x) 1 2)",
        "function expects 1 arguments [line 1, column 1]",
    );
    // Calling a non-function value.
    assert_eval_error("(1 2)", "value is not callable [line 1, column 1]");
}

#[test]
fn fn_shape_errors_are_compile_errors() {
    let (kind, message) = compile_error("(fn x x)");
    assert_eq!(kind, CompileErrorKind::Arity);
    assert!(
        message.contains("function parameters must be a vector"),
        "{message}"
    );
}

#[test]
fn parse_errors_are_compile_errors() {
    let (kind, message) = compile_error("(+ 1");
    assert_eq!(kind, CompileErrorKind::Parse);
    assert!(message.contains("EOF while reading list"), "{message}");
}

#[test]
fn runtime_errors_carry_instruction_and_position() {
    let program = compile_source("(+ 1 2) (loop [i 0] (if (< i 3) (recur (/ 1 0)) i))")
        .expect("compiles");
    let error = execute_program(std::rc::Rc::new(program)).expect_err("division by zero");
    let text = error.to_string();
    // The runtime error points at the failing primitive call, not the
    // enclosing `recur`.
    assert!(text.starts_with("division by zero [line 1, column 40]"), "{text}");
    assert!(text.contains("(instruction"), "{text}");
    let position = error.position.expect("source position");
    assert_eq!((position.line, position.column), (1, 40));
}

#[test]
fn loop_workload_executes() {
    assert_eq!(
        eval("(loop [i 0 acc 0] (if (< i 5000) (recur (+ i 1) (+ acc (mod i 17))) acc))"),
        "39985"
    );
}

#[test]
fn multiline_source_positions() {
    let (_, message) = compile_error("(let [x 1]\n  (+ x y))");
    assert!(message.contains("[line 2, column 8]"), "{message}");
}

#[test]
fn compiled_programs_are_reusable() {
    let program = std::rc::Rc::new(compile_source("(let [x 19 y 23] (+ x y))").expect("compiles"));
    for _ in 0..3 {
        let value = execute_program(program.clone()).expect("executes");
        assert!(matches!(value, Value::Number(42)));
    }
}

#[test]
fn declare_opts_in_to_foundation_replacement() {
    assert_eq!(eval("(declare count)"), "nil");
    assert_eq!(eval("(declare count) (defn count [n] 42) (count 5)"), "42");
    assert_eq!(
        eval("(declare count other) (defn count [n] (+ n 1)) (count 41)"),
        "42"
    );
    // Undeclared replacement of a foundation builtin is a compile error.
    let (kind, message) = compile_error("(defn count [n] 42) (count 5)");
    assert_eq!(kind, CompileErrorKind::UnsupportedForm);
    assert!(
        message.contains("defn replaces std.foundation var: count"),
        "{message}"
    );
    // The VM's own primitives are foundation names too.
    let (_, message) = compile_error("(defn mod [a b] 100) 1");
    assert!(
        message.contains("defn replaces std.foundation var: mod"),
        "{message}"
    );
    // declare is top-level only and takes name symbols.
    let (_, message) = compile_error("(let [x 1] (declare y) x)");
    assert!(
        message.contains("declare is only supported as a top-level statement"),
        "{message}"
    );
    let (_, message) = compile_error("(declare 1)");
    assert!(message.contains("declare expects name symbols"), "{message}");
}

#[test]
fn workload_disassembly_is_deterministic() {
    let source = "(loop [i 0 acc 0] (if (< i 5000) (recur (+ i 1) (+ acc (mod i 17))) acc))";
    let first = disassemble(&compile_source(source).expect("compiles"));
    let second = disassemble(&compile_source(source).expect("compiles"));
    assert_eq!(first, second);
    assert!(first.contains("JumpIfFalse ->"), "{first}");
    assert!(first.contains("StoreLocal 1"), "{first}");
    assert!(first.contains("Primitive < 2"), "{first}");
}

// ------------------------------------------------------------------
// Exceptions (issue #203): try/catch/finally and guest throw.
// ------------------------------------------------------------------

#[test]
fn throw_and_catch_basics() {
    assert_eq!(
        eval("(try (throw 41) (catch Exception error (+ error 1)))"),
        "42"
    );
    // The implicit (catch name body) form matches Exception.
    assert_eq!(eval("(try (throw :failed) (catch error error))"), ":failed");
    // First matching catch wins; later clauses do not run.
    assert_eq!(
        eval("(try (throw 41) (catch Exception a 41) (catch Exception b 42))"),
        "41"
    );
    // A non-matching class falls through to the next clause.
    assert_eq!(
        eval("(try (throw 41) (catch Problem error 0) (catch Exception error (+ error 1)))"),
        "42"
    );
    // A body value passes through an unmatched-catch try unchanged.
    assert_eq!(eval("(try 7 (catch Exception e 0))"), "7");
}

#[test]
fn catch_binds_runtime_error_messages() {
    // Runtime errors bind the message string.
    assert_eq!(
        eval("(try (/ 1 0) (catch Exception error error))"),
        "\"division by zero\""
    );
    // Errors crossing a closure call bind the bare message string, not a
    // rendered composite.
    assert_eq!(
        eval("(try ((fn [] (/ 1 0))) (catch Exception e e))"),
        "\"division by zero\""
    );
}

#[test]
fn uncaught_throws_propagate() {
    assert_eval_error("(throw :failed)", "thrown: :failed");
    assert_eval_error("(try (throw 41) (catch Problem error 0))", "thrown: 41");
    assert_eval_error(
        "(try (try (throw 41) (catch Problem error 0)) (catch Problem error 0))",
        "thrown: 41"
    );
}

#[test]
fn finally_semantics() {
    // Finally results are discarded on the success path.
    assert_eq!(eval("(try 42 (finally 0))"), "42");
    assert_eq!(eval("(try 42 43 (finally 0 1))"), "43");
    // Finally runs after a caught error without changing the outcome.
    assert_eq!(
        eval("(try (throw 41) (catch Exception error (+ error 1)) (finally 0))"),
        "42"
    );
    // An in-flight error rethrows with its identity after finally.
    assert_eq!(
        eval("(try (try (throw :original) (finally 0)) (catch Exception e e))"),
        ":original"
    );
    // An error in finally replaces the in-flight outcome (first error
    // short-circuits, matching the fiber).
    assert_eval_error("(try 1 (finally (throw 2)))", "thrown: 2");
    assert_eval_error("(try (throw 1) (catch Exception e (throw 2)))", "thrown: 2");
    assert_eval_error("(try (throw 1) (finally (throw 2)))", "thrown: 2");
}

#[test]
fn exceptions_cross_function_boundaries() {
    // try inside a function body.
    assert_eq!(eval("((fn [] (try (throw 1) (catch Exception e 42))))"), "42");
    // A throw inside a called function unwinds to the caller's catch.
    assert_eq!(
        eval("(try ((fn [] (throw 41))) (catch Exception e (+ e 1)))"),
        "42"
    );
}

#[test]
fn recur_through_catch_only_try() {
    // recur in the body of a catch-only try stays in tail position.
    assert_eq!(
        eval("(loop [i 0] (try (if (< i 3) (recur (+ i 1)) i) (catch Exception e -1)))"),
        "3"
    );
    // recur in a catch body of a catch-only try.
    assert_eq!(
        eval("(loop [i 0] (try (throw 1) (catch Exception e (if (< i 3) (recur (+ i 1)) i))))"),
        "3"
    );
}

#[test]
fn try_compile_errors() {
    // Body forms cannot follow catch/finally clauses.
    let (kind, message) = compile_error("(try 1 (catch Exception e 2) 3)");
    assert_eq!(kind, CompileErrorKind::Arity);
    assert!(message.contains("try clauses must follow body"), "{message}");
    // Malformed catch clauses are compile errors. The evaluator silently
    // treats a non-symbol class as non-matching; the VM rejects the
    // source instead (documented divergence).
    let (_, message) = compile_error("(try 1 (catch 42 e 0))");
    assert!(
        message.contains("catch class must be symbol [line 1, column 15]"),
        "{message}"
    );
    let (_, message) = compile_error("(try 1 (catch Exception 42 0))");
    assert!(message.contains("catch name must be symbol"), "{message}");
    let (_, message) = compile_error("(try 1 (catch))");
    assert!(
        message.contains("catch expects class, name, and body"),
        "{message}"
    );
    // throw takes exactly one value.
    let (kind, message) = compile_error("(throw)");
    assert_eq!(kind, CompileErrorKind::Arity);
    assert!(message.contains("throw expects one value"), "{message}");
    // recur cannot cross a finally boundary (checked before the tail
    // check, because the try itself suppresses tail propagation).
    let (kind, message) =
        compile_error("(loop [i 0] (try (if (< i 3) (recur (+ i 1)) i) (finally 0)))");
    assert_eq!(kind, CompileErrorKind::Recur);
    assert!(message.contains("recur cannot cross a finally boundary"), "{message}");
}

#[test]
fn uncaught_throw_carries_position() {
    let program = compile_source("(try 1 (finally 0)) (throw :failed)").expect("compiles");
    let error = execute_program(std::rc::Rc::new(program)).expect_err("uncaught throw");
    let text = error.to_string();
    assert!(text.starts_with("thrown: :failed [line 1, column 21]"), "{text}");
    assert!(text.contains("(instruction"), "{text}");
}
