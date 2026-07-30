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
