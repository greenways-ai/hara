import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import init, { Runtime } from "../../website/docs/rust/pkg/hara_wasm.js";

const moduleBytes = await readFile(
  new URL("../../website/docs/rust/pkg/hara_wasm_bg.wasm", import.meta.url),
);
await init({ module_or_path: moduleBytes });

test("portable code.test runs through the browser wasm runtime", () => {
  const runtime = new Runtime();
  const result = runtime.eval(
    '(ns code.test-browser-probe (:use code.test))' +
      ' (fact "promise assertion" (promise/from 42) => 42)' +
      ' (let [summary (run {:namespace "code.test-browser-probe"})]' +
      ' [(:status summary) (:passed (:counts summary))])',
  );

  assert.equal(result, "[:passed 1]");
});

test("canonical component and context libraries run through browser wasm", () => {
  const runtime = new Runtime();
  const result = runtime.eval(
    "(ns std-lib-context-browser-probe" +
      " (:require [std.lib.component :as component]" +
      "           [std.lib.context :as context]))" +
      " (let [runtime (context/runtime-null)]" +
      " [(component/started? runtime) (context/call runtime :a :b)])",
  );

  assert.equal(result, "[true [:a :b]]");
  assert.throws(
    () => runtime.eval("(require [std.foundation.component :as old])"),
    /missing/,
  );
});
