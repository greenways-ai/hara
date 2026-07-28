import assert from "node:assert/strict";
import test from "node:test";
import { CanvasRuntime, resolutionUniform } from "./studio/canvas-runtime.js";

function fixture() {
  const callbacks = new Map();
  let next = 0;
  const listeners = new Map();
  const calls = [];
  const gradient = {
    addColorStop: (...args) => calls.push(["addColorStop", ...args])
  };
  const context = new Proxy({}, {
    get(target, property) {
      if (property === "createRadialGradient") {
        return (...args) => {
          calls.push([property, ...args]);
          return gradient;
        };
      }
      if (!(property in target)) target[property] = (...args) => calls.push([property, ...args]);
      return target[property];
    },
    set(target, property, value) {
      calls.push([property, value]);
      target[property] = value;
      return true;
    }
  });
  const canvas = {
    width: 1,
    height: 1,
    clientWidth: 320,
    clientHeight: 180,
    getContext: (kind) => kind === "2d" ? context : null
  };
  const window = {
    devicePixelRatio: 2,
    addEventListener: (name, handler) => listeners.set(name, handler),
    removeEventListener: (name) => listeners.delete(name),
    document: { createElement: () => ({ getContext: () => null }) }
  };
  const runtime = new CanvasRuntime({
    window,
    requestFrame: (callback) => { const id = ++next; callbacks.set(id, callback); return id; },
    cancelFrame: (id) => callbacks.delete(id)
  });
  runtime.register("canvas/background", canvas);
  return { runtime, canvas, calls, callbacks, listeners };
}

test("next-frame returns integer geometry, timing, and active input", async () => {
  const { runtime, callbacks, listeners } = fixture();
  runtime.claim("node/tron@1", "canvas/background");
  listeners.get("keydown")({
    key: "ArrowLeft", code: "ArrowLeft", repeat: false,
    ctrlKey: false, altKey: false, shiftKey: false, metaKey: false
  });
  const pending = runtime.nextFrame("node/tron@1", "canvas/background");
  callbacks.values().next().value(18.75);
  const frame = await pending;
  assert.equal(frame.get("frame/time-ms"), 18);
  assert.equal(frame.get("canvas/width"), 320);
  assert.equal(frame.get("canvas/pixel-ratio-milli"), 2000);
  assert.equal(frame.get("input/events").length, 1);
});

test("WebGL resolution uniforms use the physical Retina backing store", () => {
  assert.deepEqual(
    resolutionUniform("u_resolution", [320, 180], 640, 360),
    [640, 360]
  );
  assert.deepEqual(
    resolutionUniform("iResolution", [320, 180, 1], 640, 360),
    [640, 360, 1]
  );
  assert.deepEqual(
    resolutionUniform("u_pointer", [20, 30], 640, 360),
    [20, 30]
  );
});

test("only the active generation can render and replacement cancels its frame", async () => {
  const { runtime } = fixture();
  runtime.claim("node/tron@1", "canvas/background");
  const pending = runtime.nextFrame("node/tron@1", "canvas/background");
  runtime.claim("node/tron@2", "canvas/background");
  await assert.rejects(pending, /ownership replaced/);
  assert.throws(
    () => runtime.render("node/tron@1", "canvas/background", new Map()),
    /does not own/
  );
});

test("semantic canvas aliases cannot be owned by competing generations", async () => {
  const { runtime, canvas } = fixture();
  runtime.register("canvas/visualizer", canvas);
  runtime.claim("node/tron@1", "canvas/background");
  const pending = runtime.nextFrame("node/tron@1", "canvas/background");
  runtime.claim("node/fft@1", "canvas/visualizer");
  await assert.rejects(pending, /canvas surface ownership replaced/);
  assert.throws(
    () => runtime.render("node/tron@1", "canvas/background", new Map()),
    /does not own/
  );
});

test("Canvas2D frames execute declared commands without game-specific state", () => {
  const { runtime, calls } = fixture();
  runtime.claim("node/grid@1", "canvas/background");
  runtime.render("node/grid@1", "canvas/background", new Map([
    ["type", { constructor: { name: "HtaKeyword" }, name: "canvas-2d" }],
    ["background", "#020408"],
    ["commands", [
      [{ constructor: { name: "HtaKeyword" }, name: "grid" }, 24, "#123", 1],
      [{ constructor: { name: "HtaKeyword" }, name: "circle" }, 20, 30, 4, "#41f5e4"],
      [{ constructor: { name: "HtaKeyword" }, name: "mist" }, 80, 70, 24, "#31ff8d", 0.16]
    ]]
  ]));
  assert.ok(calls.some(([name]) => name === "fillRect"));
  assert.ok(calls.some(([name]) => name === "arc"));
  assert.ok(calls.some(([name]) => name === "createRadialGradient"));
  assert.equal(calls.filter(([name]) => name === "addColorStop").length, 3);
});

test("hiding a workspace rejects outstanding animation frames", async () => {
  const { runtime } = fixture();
  runtime.claim("node/fire@1", "canvas/background");
  const pending = runtime.nextFrame("node/fire@1", "canvas/background");
  runtime.setVisible(false);
  await assert.rejects(pending, /workspace hidden/);
});
