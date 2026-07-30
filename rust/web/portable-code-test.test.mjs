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

test("portable tasks emit structured reports through browser wasm", () => {
  const runtime = new Runtime();
  const result = runtime.eval(
    "(ns std-task-browser-probe" +
      " (:require [std.task :as task] [std.task.bulk :as bulk]))" +
      " (task/deftask double-task" +
      "   {:template :default :main {:fn (fn [value] (* 2 value))}})" +
      " (let [reporter (bulk/event-reporter)" +
      "       output (task/invoke double-task [1 2 3]" +
      "                           {:reporter reporter :return :all})]" +
      " [(get output :summary)" +
      "  (vec (map (fn [item] (get item :data)) (get output :results)))" +
      "  (count (bulk/reporter-events reporter))])",
  );

  assert.equal(
    result,
    "[{:items 3 :results 3 :warnings 0 :errors 0} [2 4 6] 8]",
  );
});

test("portable blocks preserve source, value, and structure through browser wasm", () => {
  const runtime = new Runtime();
  const result = runtime.eval(
    "(ns std-block-browser-probe" +
      " (:require [std.block :as block]))" +
      ' (let [parsed (block/parse-string "[1 2 3]")' +
      '       first-block (block/parse-first "[1 2 3]")' +
      "       spaces (block/spaces 3)]" +
      " [(block/string parsed)" +
      "  (block/value parsed)" +
      "  (block/type first-block)" +
      "  (block/tag first-block)" +
      "  (vec (map block/value (block/children first-block)))" +
      "  (block/string spaces)" +
      "  (block/space? spaces)])",
  );

  assert.equal(
    result,
    '["[1 2 3]" [1 2 3] :container :vector [1 2 3] "   " true]',
  );
});
