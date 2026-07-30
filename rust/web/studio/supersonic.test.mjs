import assert from "node:assert/strict";
import test from "node:test";
import { SupersonicProvider, normalizeGraph } from "./supersonic.js";

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
