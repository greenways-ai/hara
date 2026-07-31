//! Differential tests: every supported form runs through the existing
//! evaluator (`Runtime::eval_native`) and the bytecode VM, and the
//! results must agree. Successes compare displayed values; failures
//! compare normalized error categories, because the two paths detect some
//! misuse at different stages (compile time vs runtime) and phrase
//! positions differently.

use super::error_category;
use super::eval_source;
use crate::core::Value;
use crate::Runtime;

fn differential(source: &str) {
    let reference = Runtime::new().eval_native(source);
    let vm = eval_source(source).map(|value| value.display());
    match (&reference, &vm) {
        (Ok(expected), Ok(actual)) => {
            assert_eq!(expected, actual, "value divergence for {source}")
        }
        (Err(expected), Err(actual)) => assert_eq!(
            error_category(expected),
            error_category(actual),
            "error category divergence for {source}: {expected} vs {actual}"
        ),
        _ => panic!("divergence for {source}: reference {reference:?} vs vm {vm:?}"),
    }
}

#[test]
fn supported_forms_match_the_existing_evaluator() {
    let sources = [
        // Required by the issue.
        "42",
        "(+ 19 23)",
        "(if (< 19 20) 42 0)",
        "(let [x 19 y 23] (+ x y))",
        "(loop [i 0 acc 0] (if (< i 5000) (recur (+ i 1) (+ acc (mod i 17))) acc))",
        // Literals.
        "nil",
        "true",
        "false",
        "-9223372036854775808",
        "9223372036854775807",
        "1.5",
        "\"hello world\"",
        ":keyword",
        ":hara/namespaced",
        "\\newline",
        "\\a",
        "()",
        "1 2 3",
        // Arithmetic and comparisons, including edge semantics.
        "(+ 1 2 3 4 5)",
        "(+ 7)",
        "(- 10 1 2 3)",
        "(* 2 3 4)",
        "(/ 100 4 5)",
        "(% 100 7)",
        "(mod -7 3)",
        "(< 1 2 3 4)",
        "(<= 1 1 2)",
        "(> 4 3 2 1)",
        "(>= 3 3 2)",
        "(= 42 42 42)",
        "(= 1 1.0)",
        "(= nil nil)",
        "(= :a :a)",
        "(= \"x\" \"x\" \"y\")",
        // Truthiness.
        "(if 0 1 2)",
        "(if \"\" 1 2)",
        "(if nil 1)",
        "(if false 1)",
        // Locals.
        "(let [x 1] (let [x 2] x))",
        "(let [x 1] (do (let [x 2] x) x))",
        "(let [x 1 y (+ x 1) z (+ y 1)] z)",
        "(let [x 1 x (+ x 1)] x)",
        "(do 1 2 3)",
        "(do)",
        "(let [a 1 b 2 c 3] (+ a (- b c)))",
        // Loops.
        "(loop [i 0] (if (< i 0) (recur (+ i 1)) i))",
        "(loop [i 0] (if (< i 1) (recur (+ i 1)) i))",
        "(loop [i 0 acc 1] (if (< i 10) (recur (+ i 1) (* acc 2)) acc))",
        "(loop [x 0 y 1] (if (< x 5) (recur (+ x 1) (+ x y)) y))",
        "(loop [x 1 y 2 n 0] (if (< n 3) (recur y x (+ n 1)) (- x y)))",
        "(loop [i 0] (let [next (+ i 1)] (if (< i 5) (do (recur next)) i)))",
        "(loop [i 0 t 0] (if (< i 4) (recur (+ i 1) (+ t (loop [j 0 s 0] (if (< j 3) (recur (+ j 1) (+ s (* i j))) s)))) t))",
        "(loop [i 0] (if (>= i 3) i (recur (+ i 2))))",
        "(loop [] 7)",
        "(loop [i 0] 1 2)",
        "(loop [i 0] (+ i 1) i)",
        // Functions, closures, and defn lowering.
        "((fn [x] x) 1)",
        "((fn [x y] (+ x y)) 19 23)",
        "((fn [] 42))",
        "(let [f (fn [x] (+ x 1))] (f 41))",
        "(let [x 19] ((fn [y] (+ x y)) 23))",
        "(let [x 1 f (fn [] x)] (let [x 2] (+ (f) x)))",
        "(((fn [x] (fn [y] (+ x y))) 19) 23)",
        "(loop [i 0 acc 0] (if (< i 5) (recur (+ i 1) ((fn [x] (+ x i)) acc)) acc))",
        "(do (defn f [x] (+ x 1)) (f 41))",
        "(do (defn f [x] (+ x 1)) (defn f [x] (+ x 2)) (f 40))",
        "(do (defn g [x] (* x 2)) (defn h [x] (+ (g x) 1)) (h 20))",
        "(do (defn countdown [n] (if (< n 1) 0 (+ 1 (countdown (- n 1))))) (countdown 100))",
        "(defn f [x] (+ x 1)) (f 41)",
        // Exceptions (issue #203).
        "(try (throw 41) (catch Exception error (+ error 1)))",
        "(try (throw :failed) (catch error error))",
        "(try (throw 7) (catch e (+ e 1)))",
        "(try (throw 41) (catch Exception a 41) (catch Exception b 42))",
        "(try (throw 41) (catch Problem error 0) (catch Exception error (+ error 1)))",
        "(try 7 (catch Exception e 0))",
        "(try (/ 1 0) (catch Exception error error))",
        "(try 42 (finally 0))",
        "(try 42 43 (finally 0 1))",
        "(try (throw 41) (catch Exception error (+ error 1)) (finally 0))",
        "(try (try (throw :original) (finally 0)) (catch Exception e e))",
        "(try (try (throw 41) (catch Problem error 0) (finally 0)) (catch Exception error (+ error 1)))",
        "(try ((fn [] (throw 41))) (catch Exception e (+ e 1)))",
        "(try ((fn [] (/ 1 0))) (catch Exception e e))",
        "((fn [] (try (throw 1) (catch Exception e 42))))",
        "(loop [i 0] (try (if (< i 3) (recur (+ i 1)) i) (catch Exception e -1)))",
        "(loop [i 0] (try (throw 1) (catch Exception e (if (< i 3) (recur (+ i 1)) i))))",
    ];
    for source in sources {
        differential(source);
    }
}

#[test]
fn supported_form_errors_match_the_existing_evaluator() {
    let sources = [
        "(/ 1 0)",
        "(% 1 0)",
        "(mod 1 0)",
        "(+ 9223372036854775807 1)",
        "(* 4611686018427387904 2)",
        "(- -9223372036854775808 1)",
        "(+ 1 1.5)",
        "(+ \"a\" 1)",
        "(< 1 \"a\")",
        "(< 1)",
        "(= 1)",
        "(+)",
        "(mod)",
        "(if)",
        "(if 1 2 3 4)",
        "(let [x] x)",
        "(let 1 x)",
        "(loop [i] i)",
        "(loop [i 0])",
        "(loop 1 2)",
        "(let)",
        "unknown",
        "(let [x 1] (+ x y))",
        "(recur 1)",
        "(loop [i 0] (recur 1 2))",
        "(+ 1",
        "42N",
        "((fn [x] x) 1 2)",
        "((fn [x] x))",
        "(1 2)",
        "(do (defn f [x y] (+ x y)) (f 1))",
        // Exceptions (issue #203).
        "(throw :failed)",
        "(try (throw 41) (catch Problem error 0))",
        "(try (try (throw 41) (catch Problem error 0)) (catch Problem error 0))",
        "(try 1 (finally (throw 2)))",
        "(try (throw 1) (finally (throw 2)))",
        "(throw)",
        "(try (throw 1) (catch Exception e (throw 2)))",
    ];
    for source in sources {
        differential(source);
    }
}

#[test]
fn recur_tail_tightening_is_a_documented_divergence() {
    // The langspec restricts recur to tail positions; the evaluator
    // detects some violations only at runtime (or, for truthiness
    // misuses, silently). The VM rejects them at compile time. Both
    // paths agree on the cases in the supported corpus above; these
    // cases are where detection timing legitimately differs.
    let reference = Runtime::new().eval_native("(loop [i 0] (+ 1 (recur 2)))");
    assert!(reference.is_err(), "{reference:?}");
    assert!(eval_source("(loop [i 0] (+ 1 (recur 2)))").is_err());
}

#[test]
fn defn_foundation_replacement_requires_declare() {
    // Ruling (issue #202): replacing a std.foundation builtin through
    // `defn` is an error unless the name was `declare`d first. The VM
    // makes undeclared replacement a compile error; the evaluator still
    // gives the builtin precedence and converges later.
    let undeclared = "(do (defn count [n] 42) (count 5))";
    let error = super::compile_source(undeclared).expect_err("must not compile");
    assert!(
        error.to_string().contains("replaces std.foundation var: count"),
        "{error}"
    );
    // With an explicit declare, the replacement lowers and takes effect
    // in the VM. The evaluator still resolves the builtin even after
    // declare — canonical behavior is the VM's; the evaluator converges.
    let declared = "(do (declare count) (defn count [n] 42) (count 5))";
    assert_eq!(
        eval_source(declared).map(|value| value.display()),
        Ok("42".into())
    );
    assert!(Runtime::new().eval_native(declared).is_err());
    // A bare declare already agrees on both paths.
    differential("(declare count)");
}

/// Reads the shared benchmark corpus and runs every workload whose
/// source is inside the supported subset, exactly as written in
/// `lib/bench/runtime/workloads.json`.
#[test]
fn shared_benchmark_workloads_match() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../lib/bench/runtime/workloads.json"
    );
    let text = std::fs::read_to_string(path).expect("workloads.json must exist");
    let parsed = crate::json::read(&text).expect("workloads.json parses");
    let entries = crate::core::map_entries(&parsed).expect("top-level object");
    let workloads = entries
        .iter()
        .find(|(key, _)| matches!(key, Value::String(name) if name == "workloads"))
        .map(|(_, value)| value)
        .expect("workloads key");
    let Value::Vector(workloads) = workloads else {
        panic!("workloads must be a vector")
    };
    let supported = ["noop", "arithmetic", "function-call"];
    let mut seen = Vec::new();
    for workload in workloads.iter() {
        let fields = crate::core::map_entries(workload).expect("workload object");
        let field = |name: &str| {
            fields
                .iter()
                .find(|(key, _)| matches!(key, Value::String(key) if key == name))
                .and_then(|(_, value)| match value {
                    Value::String(text) => Some(text.clone()),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("workload missing {name}"))
        };
        let id = field("id");
        if !supported.contains(&id.as_str()) {
            continue; // collections are outside this milestone
        }
        seen.push(id.clone());
        let source = field("source");
        let expected = field("expected");
        let reference = Runtime::new().eval_native(&source).expect("reference evaluates");
        let vm = eval_source(&source)
            .map(|value| value.display())
            .expect("vm evaluates");
        assert_eq!(reference, expected, "{id} reference mismatch");
        assert_eq!(vm, expected, "{id} vm mismatch");
    }
    assert_eq!(seen, supported, "corpus must contain the supported workloads");
}
