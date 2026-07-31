import assert from "node:assert/strict";
import test from "node:test";
import { start } from "./dist/hara.mjs";

test("browser SDK starts the embedded runtime and loads std.logic.kanren", async () => {
  const hara = await start({
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
  hara.dispose();
});
