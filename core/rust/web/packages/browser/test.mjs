import assert from "node:assert/strict";
import test from "node:test";
import { start as startVm } from "./dist/hara-wasm-vm/hara.mjs";
import { start as startFull } from "./dist/hara-wasm-full/hara.mjs";

test("browser SDK starts the embedded runtime and loads std.logic.kanren", async () => {
  const hara = await startVm({
    resources: {
      "app.config": "(ns app.config) (def answer 42)"
    }
  });

  assert.equal(
    hara.eval(
      "(require [std.logic.kanren :as logic]) " +
      "(logic/run* (fn [query] (logic/== query 42)))"
    ),
    "[42]"
  );
  assert.equal(hara.require("app.config"), "42");
  assert.equal(hara.eval("app.config/answer"), "42");
});

test("VM package compiles and executes persistent bytecode", async () => {
  const hara = await startVm();
  const artifact = hara.compileBytecode("(+ 19 23)");
  assert.equal(hara.evalBytecode(artifact), "42");
  await assert.rejects(() => hara.compileWholeWasm("(+ 19 23)"), /browser\/full/);
});

test("browser SDK compiles and executes whole-function WebAssembly", async () => {
  const hara = await startFull();
  const compiled = await hara.compileWholeWasm(
    "(loop [i 0 acc 0] (if (< i 5000) (recur (+ i 1) (+ acc i)) acc))"
  );
  assert.equal(compiled.call(), 12_497_500n);

  const division = await hara.compileWholeWasm("(/ 1 0)");
  assert.throws(() => division.call(), /division by zero/);
  hara.dispose();
});
