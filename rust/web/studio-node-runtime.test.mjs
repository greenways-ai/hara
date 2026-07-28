import assert from "node:assert/strict";
import test from "node:test";

import {
  decodeFrameJson,
  encodeFrameJson,
  NodeProtocolError,
  NodeRuntime
} from "./studio/node-runtime.js";

function graph(delivery = "ordered", capacity = 4) {
  const runtime = new NodeRuntime({ space: "workspace/test" });
  runtime.registerNode({ id: "node/a", type: "test/source" });
  runtime.registerNode({ id: "node/b", type: "test/transform" });
  runtime.connect({
    id: "connection/a-b",
    from: ["node/a", "signal/out"],
    to: ["node/b", "signal/in"],
    delivery,
    capacity
  });
  return runtime;
}

test("canonical JSON round-trips substrate frames and binary payloads", () => {
  const input = {
    version: "substrate.v1",
    kind: "stream",
    id: "evt-123",
    source: "node/a",
    target: null,
    space: "workspace/test",
    signal: "signal/out",
    data: new Float32Array([0.25, -0.5, 1]),
    cause: "evt-122",
    meta: { transport: "json" }
  };
  const text = encodeFrameJson(input);
  assert.match(text, /"version":"substrate.v1"/);
  assert.ok(text.includes('"$hara/type":"hara.bytes.v1"'));
  const output = decodeFrameJson(text);
  assert.ok(output.data instanceof Float32Array);
  assert.deepEqual([...output.data], [0.25, -0.5, 1]);
  assert.equal(output.cause, "evt-122");
});

test("request response preserves correlation, cause, space, and metadata", async () => {
  const runtime = graph();
  const frames = [];
  runtime.subscribe((frame) => frames.push(frame));
  runtime.handle("node/b", "transform", async ([value]) => value * 2, { stable: true });
  const response = await runtime.call("node/a", "node/b", "transform", [21], {
    id: "req-1",
    cause: "evt-0",
    meta: { trace: "trace-1" }
  });
  assert.equal(response.data, 42);
  assert.equal(response.reply_to, "req-1");
  assert.equal(response.cause, "req-1");
  assert.deepEqual(frames.map((frame) => frame.kind), ["request", "response"]);
  assert.equal(frames[0].meta.trace, "trace-1");
});

test("stream fan-out delivers complete frames and values", async () => {
  const runtime = graph();
  runtime.registerNode({ id: "node/c" });
  runtime.connect({
    id: "connection/a-c",
    from: ["node/a", "signal/out"],
    to: ["node/c", "signal/in"],
    delivery: "ordered",
    capacity: 2
  });
  const accepted = await runtime.emit("node/a", "signal/out", { answer: 42 }, { cause: "req-1" });
  assert.equal(accepted.deliveries.length, 2);
  assert.deepEqual(await runtime.in("node/b", "signal/in"), { answer: 42 });
  const frame = await runtime.inFrame("node/c", "signal/in");
  assert.equal(frame.kind, "stream");
  assert.equal(frame.cause, "req-1");
  assert.deepEqual(frame.data, { answer: 42 });
});

test("latest delivery drops stale visualizer frames", async () => {
  const runtime = graph("latest", 1);
  await runtime.emit("node/a", "signal/out", 1);
  const result = await runtime.emit("node/a", "signal/out", 2);
  assert.equal(result.deliveries[0].dropped, 1);
  assert.equal(await runtime.in("node/b", "signal/in"), 2);
});

test("ordered delivery reports queue overflow", async () => {
  const runtime = graph("ordered", 1);
  await runtime.emit("node/a", "signal/out", 1);
  await assert.rejects(runtime.emit("node/a", "signal/out", 2), (error) => {
    assert.ok(error instanceof NodeProtocolError);
    assert.equal(error.code, "queue/overflow");
    return true;
  });
});

test("anonymous document generation activation rolls back and cancels old scope", async () => {
  const runtime = graph();
  let oldSignal;
  const first = await runtime.activateDocument("node/b", {
    documentId: "document/visualizer",
    generation: 1,
    prepare(api) {
      api.start((signal) => new Promise((resolve) => {
        oldSignal = signal;
        signal.addEventListener("abort", resolve, { once: true });
      }));
    }
  });
  assert.equal(first.private, true);
  assert.equal(runtime.info("node/b").generation.generation, 1);

  await assert.rejects(runtime.activateDocument("node/b", {
    documentId: "document/visualizer",
    generation: 2,
    prepare() { throw new Error("bad reload"); }
  }), /bad reload/);
  assert.equal(runtime.info("node/b").generation.generation, 1);
  assert.equal(oldSignal.aborted, false);

  await runtime.activateDocument("node/b", {
    documentId: "document/visualizer",
    generation: 3
  });
  assert.equal(oldSignal.aborted, true);
  assert.equal(runtime.info("node/b").generation.generation, 3);
  assert.equal(runtime.releaseDocument("document/visualizer"), 1);
  assert.equal(runtime.info("node/b").generation, null);
});

test("kernel handlers become public only with their committed document generation", async () => {
  const runtime = graph();
  const firstContext = {};
  runtime.stageKernelHandler(firstContext, "node/b", "double", ([value]) => value * 2);
  await runtime.activateDocument("node/b", {
    documentId: "document/handler",
    generation: 1,
    kernelContext: firstContext
  });
  assert.equal((await runtime.call("node/a", "node/b", "double", [21])).data, 42);

  const failedContext = {};
  runtime.stageKernelHandler(failedContext, "node/b", "double", ([value]) => value * 3);
  await assert.rejects(runtime.activateDocument("node/b", {
    documentId: "document/handler",
    generation: 2,
    kernelContext: failedContext,
    prepare() { throw new Error("bad reload"); }
  }), /bad reload/);
  assert.equal((await runtime.call("node/a", "node/b", "double", [21])).data, 42);

  const nextContext = {};
  runtime.stageKernelHandler(nextContext, "node/b", "double", ([value]) => value * 3);
  await runtime.activateDocument("node/b", {
    documentId: "document/handler",
    generation: 3,
    kernelContext: nextContext
  });
  assert.equal((await runtime.call("node/a", "node/b", "double", [21])).data, 63);
});

test("missing handlers return substrate error frames", async () => {
  const runtime = graph();
  await assert.rejects(runtime.call("node/a", "node/b", "missing", []), (error) => {
    assert.equal(error.code, "handler/missing");
    assert.equal(error.frame.kind, "error");
    assert.equal(error.frame.error.code, "handler/missing");
    return true;
  });
});
