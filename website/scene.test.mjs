import assert from "node:assert/strict";
import test from "node:test";

import { mapValue, renderScene, validateScene } from "./scene.js";

const keyword = (name) => ({ name });

function validScene() {
  return new Map([
    [keyword("version"), 1],
    [keyword("width"), 960],
    [keyword("height"), 600],
    [keyword("background"), "#020408"],
    [keyword("commands"), [
      [keyword("line"), 1, 2, 3, 4, "#41f5e4", 2],
      [keyword("circle"), 12, 14, 5, "#ff2e88"],
      [keyword("rect"), 20, 30, 40, 50, "#9c7bff"],
      [keyword("polyline"), [[0, 0], [10, 10]], "#f5d742", 3]
    ]]
  ]);
}

test("mapValue reads keyword-like HTA map keys", () => {
  assert.equal(mapValue(validScene(), "width"), 960);
});

test("validateScene normalizes all v1 commands", () => {
  const scene = validateScene(validScene());
  assert.equal(scene.version, 1);
  assert.equal(scene.commands.length, 4);
  assert.deepEqual(scene.commands[0], {
    type: "line",
    x1: 1,
    y1: 2,
    x2: 3,
    y2: 4,
    color: "#41f5e4",
    width: 2
  });
  assert.deepEqual(scene.commands[3].points, [[0, 0], [10, 10]]);
});

test("validateScene rejects malformed and unsupported output", () => {
  assert.throws(() => validateScene(null), /scene map/);
  const scene = validScene();
  for (const [key] of scene) {
    if (key.name === "commands") scene.set(key, [[keyword("text"), 1, 2]]);
  }
  assert.throws(() => validateScene(scene), /unsupported type/);
});

test("renderScene fits the logical scene into the canvas", () => {
  const calls = [];
  const context = new Proxy({}, {
    get(target, property) {
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
    width: 0,
    height: 0,
    getBoundingClientRect: () => ({ width: 480, height: 300 }),
    getContext: () => context
  };
  const output = renderScene(canvas, validateScene(validScene()), { pixelRatio: 2 });
  assert.equal(canvas.width, 960);
  assert.equal(canvas.height, 600);
  assert.equal(output.scale, .5);
  assert.ok(calls.some(([name]) => name === "arc"));
  assert.ok(calls.some(([name]) => name === "fillRect"));
});
