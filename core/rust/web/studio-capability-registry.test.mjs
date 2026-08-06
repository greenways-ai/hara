import assert from "node:assert/strict";
import test from "node:test";

import { CapabilityRegistry } from "./studio/capability-registry.js";
import { createCanvasCapability } from "./studio/capabilities/canvas.js";
import { createClockCapability } from "./studio/capabilities/clock.js";
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

test("canvas capability scopes every operation to the generated node", async () => {
  const calls = [];
  const runtime = {
    claim: (...args) => { calls.push(["claim", ...args]); return true; },
    render: (...args) => { calls.push(["render", ...args]); return true; },
    nextFrame: async (...args) => { calls.push(["nextFrame", ...args]); return { "frame/time-ms": 12 }; },
    stage: () => true, commit: () => true, discard: () => true, release: () => true,
    waitForFirstRender: async () => true
  };
  const registry = new CapabilityRegistry({ adapters: { "surface/canvas-2d": createCanvasCapability(runtime) } });
  registry.grant("UI", ["surface/canvas-2d"]);
  assert.equal(await registry.invokeForNode("UI", "node/renderer", "surface/canvas-2d", "claim", "canvas/main"), true);
  assert.equal(await registry.invokeForNode("UI", "node/renderer", "surface/canvas-2d", "render", "canvas/main", { type: "canvas-2d" }), true);
  assert.deepEqual(calls, [
    ["claim", "node/renderer", "canvas/main"],
    ["render", "node/renderer", "canvas/main", { type: "canvas-2d" }]
  ]);
});

test("clock capability exposes bounded millisecond time and sleep", async () => {
  const registry = new CapabilityRegistry({ adapters: { "clock/frame": createClockCapability({
    now: () => 42.9, sleep: async (milliseconds) => milliseconds
  }) } });
  registry.grant("UI", ["clock/frame"]);
  assert.equal(await registry.invokeForNode("UI", "node/timer", "clock/frame", "now"), 42);
  assert.equal(await registry.invokeForNode("UI", "node/timer", "clock/frame", "sleep", -10), 0);
});
