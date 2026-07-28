import assert from "node:assert/strict";
import test from "node:test";

import { GraphHost } from "./studio/graph-host.js";
import { SessionRouter } from "./studio/session-router.js";

function executor() {
  const calls = [];
  return {
    calls,
    async install(value) { calls.push(["install", value]); },
    async spawn(value) { calls.push(["spawn", value]); },
    async deliver(nodeId, port, frame) { calls.push(["deliver", nodeId, port, frame]); return { accepted: true }; },
    async call() { return null; },
    async releaseNode() {},
    async releaseProgram() {}
  };
}

const program = (id) => ({
  "program/id": id,
  "program/hash": `sha256:${id}`,
  "program/language": ":javascript/module",
  "program/source": "export function createNode() { return {}; }",
  "program/export": "createNode",
  "program/capabilities": []
});

const node = (id, programId) => ({
  "node/id": id,
  "node/session": "UI",
  "node/program": programId
});

test("host-to-host streams stay in GraphHost and retain substrate framing", async () => {
  const active = executor();
  const graph = new GraphHost({ executor: active });
  await graph.install(program("example/source"), { capabilities: [] });
  await graph.install(program("example/target"), { capabilities: [] });
  await graph.spawn(node("node/source", "example/source"), { capabilities: [] });
  await graph.spawn(node("node/target", "example/target"), { capabilities: [] });
  graph.connect({ id: "source-target", from: ["node/source", "out"], to: ["node/target", "in"], delivery: "latest", capacity: 1 });

  const result = await graph.sendFrame("node/source", {
    version: "substrate.v1", kind: "stream", id: "evt-1", signal: "out", data: 41, meta: {}
  });
  assert.equal(result.deliveries[0].accepted, true);
  const delivery = active.calls.find(([kind]) => kind === "deliver");
  assert.deepEqual(delivery.slice(0, 3), ["deliver", "node/target", "in"]);
  assert.equal(delivery[3].id, "evt-1");
  assert.equal(delivery[3].data, 41);
  assert.equal(graph.info("node/target").execution, "host");
});

test("a graph session target routes a selected event into its addressed Hara session", async () => {
  const active = executor();
  const calls = [];
  const sessions = new SessionRouter();
  sessions.register("UI", {
    call: async (op, args) => calls.push([op, args])
  });
  sessions.subscribe("UI", "selected", "callback/selected");

  const graph = new GraphHost({ executor: active, sessionRouter: sessions });
  await graph.install(program("example/source"), { capabilities: [] });
  await graph.spawn(node("node/source", "example/source"), { capabilities: [] });
  graph.registerSessionNode({ "node/id": "node/ui", "node/session": "UI" });
  graph.connect({
    id: "source-ui",
    from: ["node/source", "selected"],
    to: ["node/ui", "selected"],
    delivery: "latest",
    capacity: 1
  });

  const result = await graph.sendFrame("node/source", {
    version: "substrate.v1", kind: "stream", id: "evt-selected", signal: "selected", data: { id: "row-7" }, meta: {}
  });
  assert.equal(result.deliveries[0].accepted, true);
  assert.equal(calls.length, 1);
  assert.equal(calls[0][0], "eval-bound");
  const delivered = calls[0][1][1][0];
  assert.equal(delivered.get("id"), "evt-selected");
  assert.equal(delivered.get("meta").get("session/callback"), "callback/selected");
});

test("releasing a session removes both generated and Hara ingress graph nodes", async () => {
  const active = executor();
  const sessions = new SessionRouter();
  sessions.register("UI", { call: async () => null });
  const graph = new GraphHost({ executor: active, sessionRouter: sessions });
  await graph.install(program("example/source"), { capabilities: [] });
  await graph.spawn(node("node/source", "example/source"), { capabilities: [] });
  graph.registerSessionNode({ "node/id": "node/ui", "node/session": "UI" });
  graph.connect({ id: "source-ui", from: ["node/source", "out"], to: ["node/ui", "in"] });

  assert.equal(await graph.releaseSession("UI"), 2);
  assert.throws(() => graph.info("node/source"), /unknown node/);
  assert.throws(() => graph.info("node/ui"), /unknown node/);
});
