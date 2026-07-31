import assert from "node:assert/strict";
import test from "node:test";
import { SupersonicProvider, normalizeGraph } from "../src/js/supersonic.js";

const graph = () => ({
  "graph/id": "amp",
  title: "Amp",
  nodes: [
    {
      id: "gain", type: "audio/gain", params: { volume: .7 },
      controls: [{ parameter: "volume", type: "number", min: 0, max: 1, step: .01 }]
    },
    { id: "out", type: "audio/output" }
  ],
  connections: [{ from: ["gain", "audio"], to: ["out", "audio"] }]
});

test("normalizes a typed graph and rejects dangling connections", () => {
  assert.equal(normalizeGraph(graph()).nodes[0].params.volume, .7);
  const invalid = graph();
  invalid.connections[0].to[0] = "missing";
  assert.throws(() => normalizeGraph(invalid), /connection-node-not-found/);
});

test("start is atomic, update is typed, and overlay survives replacement", async () => {
  const writes = new Map();
  const storage = {
    getItem: (key) => writes.get(key) ?? null,
    setItem: (key, value) => writes.set(key, value)
  };
  let fail = false;
  const provider = new SupersonicProvider({
    storage,
    engine: {
      prepare: async () => ({
        commit: async () => { if (fail) throw new Error("engine/rejected"); }
      })
    }
  });
  assert.equal((await provider.start(graph())).generation, 1);
  assert.equal((await provider.update("amp", "gain", "volume", .4)).nodes[0].params.volume, .4);
  await assert.rejects(() => provider.update("amp", "gain", "volume", 2), /control-range-invalid/);
  fail = true;
  await assert.rejects(() => provider.start({ ...graph(), title: "Rejected" }), /engine\/rejected/);
  assert.equal(provider.status("amp").title, "Amp");
  fail = false;
  const replaced = await provider.start({ ...graph(), title: "Live" });
  assert.equal(replaced.generation, 2);
  assert.equal(replaced.nodes[0].params.volume, .4);
});

test("step controls validate compact tunes and report pending activation", async () => {
  const tune = graph();
  tune.nodes[0].params.steps = [0, null, 7, 12];
  tune.nodes[0].controls.push({ parameter: "steps", type: "steps" });
  const provider = new SupersonicProvider({
    engine: { update: async () => ({ pending: true, effectiveAt: 4 }) }
  });
  await provider.start(tune);
  const pending = await provider.update("amp", "gain", "steps", [0, 3, null, 10]);
  assert.deepEqual(pending.nodes[0].params.steps, [0, 3, null, 10]);
  assert.deepEqual(pending.pending, [{ node: "gain", parameter: "steps", effectiveAt: 4 }]);
  const effective = provider.effective("amp", "gain", "steps");
  assert.equal(effective.pending.length, 0);
  assert.equal(effective["active/revision"], 2);
  await assert.rejects(
    () => provider.update("amp", "gain", "steps", [49]),
    /control-step-invalid/
  );
  await assert.rejects(
    () => provider.update("amp", "gain", "steps", []),
    /control-steps-length-invalid/
  );
});

test("integer number controls reject fractional MIDI values", async () => {
  const tune = graph();
  tune.nodes[0].params.root = 57;
  tune.nodes[0].controls.push({
    parameter: "root", type: "number", min: 0, max: 127, integer: true
  });
  const provider = new SupersonicProvider();
  await provider.start(tune);
  await assert.rejects(
    () => provider.update("amp", "gain", "root", 57.5),
    /control-integer-invalid/
  );
});
