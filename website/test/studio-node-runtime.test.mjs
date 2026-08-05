import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const nodeHalUrl = new URL("../../rust/web/studio/hal/node.hal", import.meta.url);

test("studio.node reads task and handler registries from their atoms once", async () => {
  const source = await readFile(nodeHalUrl, "utf8");
  assert.match(source, /\(defn run-task \[id\]\n  \(let \[entry \(deref \*active-task\*\)\]/);
  assert.match(source, /\(find-handler \(deref \*handlers\*\) handler-id 0\)/);
  assert.doesNotMatch(source, /\(deref \(deref \*active-task\*\)\)/);
  assert.doesNotMatch(source, /\(deref \(deref \*handlers\*\)\)/);
});
