use hara_wasm::Runtime;

#[test]
fn generated_catalog_loads_portable_hal_namespaces() {
    let mut runtime = Runtime::new();
    assert_eq!(
        runtime
            .eval_native(
                "(require [std.logic :as logic]) \
                 (logic/run* (fn [query] (logic/== query 42)))"
            )
            .unwrap(),
        "[42]"
    );
    assert_eq!(
        runtime
            .eval_native(
                "(require [std.lib.simple :as simple]) \
                 (simple/foo 41)"
            )
            .unwrap(),
        "42"
    );
    assert_eq!(
        runtime
            .eval_native(
                "(require [std.typed.schema :as schema]) \
                 (schema/valid? [:tuple :keyword :int] [:age 42])"
            )
            .unwrap(),
        "true"
    );
    assert_eq!(
        runtime
            .eval_native(
                "(require [std.logic.datalog :as datalog]) \
                 (def db (datalog/database {} \
                   [[:requirement :demo/missing :must []]])) \
                 (datalog/query db \
                   '{:find [?id] \
                     :where [[:requirement ?id :must ?path]]})"
            )
            .unwrap(),
        "[[:demo/missing]]"
    );
    assert_eq!(
        runtime
            .eval_native(
                "(require [std.logic.kanren :as kanren]) \
                 (kanren/query* \
                   (fn [query] \
                     (kanren/relationo \
                       [[:color :sky :blue]] \
                       [:color query :blue])))"
            )
            .unwrap(),
        "[:sky]"
    );
}

#[test]
fn host_resource_replaces_embedded_hal_source() {
    let mut runtime = Runtime::new();
    runtime.require_resource("std.lib.simple").unwrap();
    assert_eq!(runtime.eval_native("(std.lib.simple/foo 1)").unwrap(), "2");

    runtime.register_resource(
        "std.lib.simple",
        "(ns std.lib.simple) (defn foo [value] (+ value 40))",
    );
    runtime.require_resource("std.lib.simple").unwrap();
    assert_eq!(runtime.eval_native("(std.lib.simple/foo 2)").unwrap(), "42");
}
