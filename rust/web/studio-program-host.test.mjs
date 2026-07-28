import assert from "node:assert/strict";
import test from "node:test";

import { ProgramError } from "./studio/module-codec.js";
import { ProgramHost } from "./studio/program-host.js";

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
  assert.deepEqual(await host.deliver("node/increment", "input", { id: "evt-1" }), { accepted: true });
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
