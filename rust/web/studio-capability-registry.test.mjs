import assert from "node:assert/strict";
import test from "node:test";

import { CapabilityRegistry } from "./studio/capability-registry.js";
import { ProgramError } from "./studio/module-codec.js";

test("capability registry exposes only declared browser facilities", () => {
  const registry = new CapabilityRegistry({ capabilities: ["surface/canvas-2d", "input/keyboard"] });
  assert.deepEqual(registry.available(), ["input/keyboard", "surface/canvas-2d"]);
  assert.equal(registry.has(":surface/canvas-2d"), true);
  assert.equal(registry.has("audio/worklet"), false);
});

test("capability grants are isolated by session and reject unavailable facilities", () => {
  const registry = new CapabilityRegistry({ capabilities: ["surface/canvas-2d"] });
  assert.deepEqual(registry.grant("UI", ["surface/canvas-2d"]), ["surface/canvas-2d"]);
  registry.assert("UI", ["surface/canvas-2d"]);
  assert.throws(() => registry.assert("MARKET", ["surface/canvas-2d"]),
    (error) => error instanceof ProgramError && error.code === "program/capability-denied");
  assert.throws(() => registry.grant("UI", ["audio/worklet"]),
    (error) => error instanceof ProgramError && error.code === "program/capability-unavailable");
});

test("registered adapters run only through a session grant", async () => {
  const registry = new CapabilityRegistry({
    adapters: { "asset/load": { load: async (path) => `asset:${path}` } }
  });
  await assert.rejects(registry.invoke("UI", "asset/load", "load", "cover.png"), /capability denied/);
  registry.grant("UI", ["asset/load"]);
  assert.equal(await registry.invoke("UI", "asset/load", "load", "cover.png"), "asset:cover.png");
});
