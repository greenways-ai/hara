import assert from "node:assert/strict";
import test from "node:test";

import { NodeProtocolError } from "./studio/node-runtime.js";
import { SessionRouter } from "./studio/session-router.js";

const mapGet = (value, key) => value instanceof Map ? value.get(key) : value?.[key];

function context() {
  return { calls: [], async call(target, args) { this.calls.push([target, args]); return true; } };
}

test("session router delivers subscribed substrate frames only to the addressed session", async () => {
  const ui = context();
  const market = context();
  const router = new SessionRouter();
  router.register("UI", ui, { capabilities: ["surface/canvas-2d"] });
  router.register("MARKET", market);
  router.subscribe("UI", "selection", "callback-ui");
  router.subscribe("MARKET", "price", "callback-market");

  const result = await router.deliver("UI", {
    version: "substrate.v1", kind: "stream", id: "evt-select", source: "node/renderer", signal: "selection", data: { id: 42 }, meta: {}
  });
  assert.deepEqual(result, { accepted: true, delivered: 1 });
  assert.equal(ui.calls.length, 1);
  assert.equal(market.calls.length, 0);
  assert.equal(ui.calls[0][0], "eval-bound");
  assert.equal(mapGet(mapGet(ui.calls[0][1][1][0], "meta"), "session/callback"), "callback-ui");
});

test("unregister closes subscriptions and rejects later delivery", async () => {
  let released = 0;
  const router = new SessionRouter();
  router.register("UI", context(), { onRelease: () => { released += 1; } });
  const id = router.subscribe("UI", "selection", "callback-ui");
  assert.equal(await router.unregister("UI"), true);
  assert.equal(released, 1);
  assert.equal(router.unsubscribe(id), false);
  await assert.rejects(router.deliver("UI", {
    version: "substrate.v1", kind: "stream", id: "evt", signal: "selection", meta: {}
  }), (error) => error instanceof NodeProtocolError && error.code === "session/not-found");
});

test("a lifecycle registration can be augmented by explicit Hara capabilities", () => {
  const ui = context();
  const router = new SessionRouter();
  router.register("UI", ui, { capabilities: ["input/keyboard"] });
  const info = router.register("UI", ui, { capabilities: ["surface/canvas-2d"] });
  assert.deepEqual(info.capabilities, ["input/keyboard", "surface/canvas-2d"]);
});
