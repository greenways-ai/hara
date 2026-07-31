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

#[test]
fn metaspec_conformance_reports_match_the_hal_contract() {
    let mut runtime = Runtime::new();
    let pass = runtime
        .eval_native(
            "(require [hara.metaspec.core :as metaspec]) \
             (def meta-document \
               {:document/id :demo/meta \
                :document/version \"1.0.0\" \
                :spec/conforms-to {:spec/id :demo/meta :spec/version \"1.0.0\"} \
                :meta/document-schema \
                {:schema/id :demo/document :schema/type :map \
                 :schema/required [:document/id :document/version]} \
                :meta/schemas [] :meta/cross-references [] :meta/checkers []}) \
             (:report/status (metaspec/conforms meta-document))",
        )
        .unwrap();
    assert_eq!(pass, ":pass");

    let blocked = runtime
        .eval_native(
            "(:report/status \
               (hara.metaspec.core/conforms \
                 (assoc meta-document :spec/conforms-to \
                   {:spec/id :missing/meta :spec/version \"1.0.0\"})))",
        )
        .unwrap();
    assert_eq!(blocked, ":blocked");
}
