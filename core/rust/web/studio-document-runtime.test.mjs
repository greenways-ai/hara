import assert from "node:assert/strict";
import test from "node:test";
import { activateStudioDocument } from "./studio/document-runtime.js";

function fixture({ firstFrame = Promise.resolve() } = {}) {
  const calls = [];
  const candidate = { value: "task-7", generation: 2 };
  const broker = {
    async prepareDocument(...args) { calls.push(["prepare", ...args]); return candidate; },
    evalPreparedDocument(...args) { calls.push(["run", ...args]); return new Promise(() => {}); },
    commitDocument(value) { calls.push(["commit", value]); return { ...value, committed: true }; },
    discardDocument(value) { calls.push(["discard", value]); }
  };
  const canvasRuntime = {
    stage(...args) { calls.push(["stage", ...args]); },
    waitForFirstRender(...args) { calls.push(["wait", ...args]); return firstFrame; },
    commit(...args) { calls.push(["canvas-commit", ...args]); },
    discard(...args) { calls.push(["canvas-discard", ...args]); }
  };
  return { broker, canvasRuntime, calls };
}

test("canvas document activates only after its candidate renders", async () => {
  let render;
  const firstFrame = new Promise((resolve) => { render = resolve; });
  const { broker, canvasRuntime, calls } = fixture({ firstFrame });
  const pending = activateStudioDocument({
    broker, canvasRuntime, kernel: "page", documentId: "p:s:/a.hal",
    source: "(ns+)", nodeId: "node/p", canvasId: "canvas/background",
    requireFirstFrame: true
  });
  await Promise.resolve();
  assert.equal(calls.some(([name]) => name === "commit"), false);
  render();
  const result = await pending;
  assert.equal(result.committed, true);
  assert.deepEqual(calls.slice(-2).map(([name]) => name), ["commit", "canvas-commit"]);
});

test("failed first frame discards both document and canvas candidates", async () => {
  const { broker, canvasRuntime, calls } = fixture({
    firstFrame: Promise.reject(new Error("first frame failed"))
  });
  await assert.rejects(activateStudioDocument({
    broker, canvasRuntime, kernel: "page", documentId: "p:s:/a.hal",
    source: "(ns+)", nodeId: "node/p", canvasId: "canvas/background",
    requireFirstFrame: true
  }), /first frame failed/);
  assert.deepEqual(calls.slice(-2).map(([name]) => name), ["discard", "canvas-discard"]);
});

test("candidate task failure before first frame rolls activation back immediately", async () => {
  const { broker, canvasRuntime, calls } = fixture({ firstFrame: new Promise(() => {}) });
  broker.evalPreparedDocument = () => Promise.reject(new Error("task crashed"));
  await assert.rejects(activateStudioDocument({
    broker, canvasRuntime, kernel: "page", documentId: "p:s:/a.hal",
    source: "(ns+)", nodeId: "node/p", canvasId: "canvas/background",
    requireFirstFrame: true
  }), /task crashed/);
  assert.deepEqual(calls.slice(-2).map(([name]) => name), ["discard", "canvas-discard"]);
});

test("non-canvas document commits without a render gate", async () => {
  const { broker, canvasRuntime, calls } = fixture();
  await activateStudioDocument({
    broker, canvasRuntime, kernel: "page", documentId: "p:s:/a.hal",
    source: "(ns+)", nodeId: "node/p"
  });
  assert.equal(calls.some(([name]) => name === "stage"), false);
  assert.equal(calls.some(([name]) => name === "commit"), true);
});
