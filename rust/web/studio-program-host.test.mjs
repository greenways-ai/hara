import assert from "node:assert/strict";
import test from "node:test";

import { ProgramError } from "./studio/module-codec.js";
import { ProgramHost, ProgramWorkerExecutor } from "./studio/program-host.js";

const program = (overrides = {}) => ({
  "program/id": "example/increment",
  "program/hash": "sha256:one",
  "program/language": ":javascript/module",
  "program/source": "export function createNode() { return {}; }",
  "program/export": "createNode",
  "program/capabilities": new Set(),
  ...overrides
});

const node = (overrides = {}) => ({
  "node/id": "node/increment",
  "node/session": "UI",
  "node/program": "example/increment",
  "node/config": { amount: 1 },
  ...overrides
});

function executor() {
  const calls = [];
  return {
    calls,
    async install(value) { calls.push(["install", value]); },
    async spawn(value) { calls.push(["spawn", value]); },
    async deliver(...value) { calls.push(["deliver", ...value]); return { accepted: true }; },
    async call(...value) { calls.push(["call", ...value]); return 42; },
    async releaseNode(...value) { calls.push(["release-node", ...value]); },
    async releaseProgram(...value) { calls.push(["release-program", ...value]); }
  };
}

test("installs a content-addressed program and caches an unchanged hash", async () => {
  const runtime = executor();
  const host = new ProgramHost({ executor: runtime });
  const first = await host.install(program(), { sessionId: "UI" });
  const cached = await host.install(program(), { sessionId: "UI" });
  assert.equal(first.status, "ready");
  assert.equal(cached.status, "cached");
  assert.equal(runtime.calls.filter(([kind]) => kind === "install").length, 1);
});

test("a changed program hash creates a new active generation", async () => {
  const runtime = executor();
  const host = new ProgramHost({ executor: runtime });
  await host.install(program(), { sessionId: "UI" });
  const replacement = await host.install(program({ "program/hash": "sha256:two" }), { sessionId: "UI" });
  assert.equal(replacement.status, "replaced");
  assert.equal(replacement.generation, 2);
});

test("a replacement retains existing node ownership until those nodes are released", async () => {
  const runtime = executor();
  const host = new ProgramHost({ executor: runtime });
  await host.install(program(), { sessionId: "UI" });
  await host.spawn(node());
  await host.install(program({ "program/hash": "sha256:two" }), { sessionId: "UI" });
  assert.equal(await host.release("example/increment"), true);
  assert.deepEqual(runtime.calls.map(([kind]) => kind), ["install", "spawn", "install", "release-node", "release-program"]);
});

test("a program id cannot be replaced by a different session", async () => {
  const host = new ProgramHost({ executor: executor() });
  await host.install(program(), { sessionId: "UI" });
  await assert.rejects(
    host.install(program({ "program/hash": "sha256:two" }), { sessionId: "MARKET" }),
    (error) => error instanceof ProgramError && error.code === "program/session-mismatch"
  );
});

test("capability grants and source limits are enforced before execution", async () => {
  const runtime = executor();
  const host = new ProgramHost({ executor: runtime });
  await assert.rejects(
    host.install(program({ "program/capabilities": new Set([":surface/canvas-2d"]) }), { capabilities: [] }),
    (error) => error instanceof ProgramError && error.code === "program/capability-denied"
  );
  await assert.rejects(
    host.install(program({ "program/source": "0123456789" }), { maxSourceBytes: 5 }),
    (error) => error instanceof ProgramError && error.code === "program/source-too-large"
  );
  assert.equal(runtime.calls.length, 0);
});

test("spawn, delivery, calls, and session release use the active program generation", async () => {
  const runtime = executor();
  const host = new ProgramHost({ executor: runtime });
  await host.install(program(), { sessionId: "UI" });
  const spawned = await host.spawn(node());
  assert.equal(spawned.sessionId, "UI");
  assert.deepEqual(await host.deliver("node/increment", "input", { id: "evt-1" }), { accepted: true, dropped: 0 });
  assert.equal(await host.call("node/increment", "status", []), 42);
  assert.equal(await host.releaseSession("UI"), 1);
  assert.deepEqual(host.list(), []);
  assert.equal(runtime.calls.map(([kind]) => kind).join(","), "install,spawn,deliver,call,release-node");
});

test("release removes program nodes before its executable generation", async () => {
  const runtime = executor();
  const host = new ProgramHost({ executor: runtime });
  await host.install(program(), { sessionId: "UI" });
  await host.spawn(node());
  assert.equal(await host.release("example/increment"), true);
  assert.deepEqual(runtime.calls.map(([kind]) => kind), ["install", "spawn", "release-node", "release-program"]);
});

test("worker capability requests are correlated and return structured denials", async () => {
  class WorkerStub {
    constructor() { this.listeners = new Map(); this.sent = []; }
    addEventListener(type, listener) { this.listeners.set(type, listener); }
    postMessage(message) { this.sent.push(message); }
    emit(message) { this.listeners.get("message")?.({ data: message }); }
    terminate() {}
  }
  const worker = new WorkerStub();
  const executor = new ProgramWorkerExecutor({
    workerUrl: "program-worker.js",
    WorkerImpl: class { constructor() { return worker; } },
    onCapability: async (message) => `${message.capability}/${message.method}:${message.args[0]}`,
    onCall: async (message) => `${message.target}/${message.action}:${message.args[0]}`
  });
  worker.emit({ type: "capability", requestId: "cap-1", nodeId: "node/a", sessionId: "UI",
    capability: "asset/load", method: "load", args: ["cover.png"] });
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.deepEqual(worker.sent.at(-1), {
    type: "capability-result", requestId: "cap-1", value: "asset/load/load:cover.png"
  });
  worker.emit({ type: "host-call", requestId: "call-1", nodeId: "node/a", sessionId: "UI",
    target: "node/b", action: "transform", args: [41], options: {} });
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.deepEqual(worker.sent.at(-1), {
    type: "host-call-result", requestId: "call-1", value: "node/b/transform:41"
  });

  const deniedWorker = new WorkerStub();
  new ProgramWorkerExecutor({ workerUrl: "program-worker.js", WorkerImpl: class { constructor() { return deniedWorker; } } });
  deniedWorker.emit({ type: "capability", requestId: "cap-2" });
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.equal(deniedWorker.sent.at(-1).type, "capability-error");
  assert.equal(deniedWorker.sent.at(-1).error.code, "program/capability-unavailable");
});

test("latest host mailboxes drop stale pending frames without delaying acknowledgement", async () => {
  let unblock;
  const received = [];
  const runtime = executor();
  runtime.deliver = async (_nodeId, _port, frame) => {
    received.push(frame.id);
    if (frame.id === "evt-1") await new Promise((resolve) => { unblock = resolve; });
    return { accepted: true };
  };
  const host = new ProgramHost({ executor: runtime });
  await host.install(program(), { sessionId: "UI" });
  await host.spawn(node());
  assert.equal((await host.deliver("node/increment", "visual", { id: "evt-1" }, { delivery: "latest", capacity: 1 })).dropped, 0);
  assert.equal((await host.deliver("node/increment", "visual", { id: "evt-2" }, { delivery: "latest", capacity: 1 })).dropped, 0);
  assert.equal((await host.deliver("node/increment", "visual", { id: "evt-3" }, { delivery: "latest", capacity: 1 })).dropped, 1);
  unblock();
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.deepEqual(received, ["evt-1", "evt-3"]);
});

test("ordered host mailboxes report overflow while a node is busy", async () => {
  let unblock;
  const runtime = executor();
  runtime.deliver = async (_nodeId, _port, frame) => {
    if (frame.id === "evt-1") await new Promise((resolve) => { unblock = resolve; });
    return { accepted: true };
  };
  const host = new ProgramHost({ executor: runtime });
  await host.install(program(), { sessionId: "UI" });
  await host.spawn(node());
  await host.deliver("node/increment", "event", { id: "evt-1" }, { delivery: "ordered", capacity: 1 });
  await host.deliver("node/increment", "event", { id: "evt-2" }, { delivery: "ordered", capacity: 1 });
  await assert.rejects(host.deliver("node/increment", "event", { id: "evt-3" }, { delivery: "ordered", capacity: 1 }),
    (error) => error instanceof ProgramError && error.code === "queue/overflow");
  unblock();
});
