import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import {
  ampLineDecorations,
  completionOptions,
  completionPrefix,
  patchAmpParameter,
  projectAmpSource
} from "./amp-source-model.js";

const source = await readFile(new URL("./examples/hara-amp/src/amp.hal", import.meta.url), "utf8");

test("projects real Amp nodes, parameters, connections, and colored lines", () => {
  const model = projectAmpSource(source);
  assert.ok(model.nodes.length >= 10);
  assert.deepEqual(
    model.nodes.slice(0, 3).map(({ id }) => id),
    ["playlist", "transport", "sequence"]
  );
  assert.equal(model.nodes.find(({ id }) => id === "visualizer").color, "magenta");
  assert.ok(model.nodes.find(({ id }) => id === "transport").params.tempo);
  assert.ok(ampLineDecorations(source, model).some(({ nodeId, color }) =>
    nodeId === "fft" && color === "violet"));
});

test("patches one parameter literal without rewriting unrelated source", () => {
  const patched = patchAmpParameter(source, "transport", "tempo", 93);
  assert.equal(projectAmpSource(patched).nodes.find(({ id }) => id === "transport").params.tempo != null, true);
  assert.match(patched, /"tempo" 93/);
  assert.equal(patched.replace('"tempo" 93', '"tempo" 120'), source);
});

test("rejects malformed source before replacing the current graph projection", () => {
  assert.throws(() => projectAmpSource(source.replace('"connections"', '"connections" [')), /delimiter|collection/i);
});

test("completion combines recipes with graph-aware selectable values", () => {
  const graph = {
    nodes: [{ id: "visualizer", controls: [{ parameter: "mode", choices: ["scope"] }] }]
  };
  assert.ok(completionOptions({ graph, prefix: "vis" }).some(({ label }) => label.includes("visualizer")));
  assert.deepEqual(completionPrefix("(sonic/sta", 10), { start: 1, end: 10, value: "sonic/sta" });
});
