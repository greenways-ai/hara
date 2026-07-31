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
