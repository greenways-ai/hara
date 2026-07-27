import assert from "node:assert/strict";
import test from "node:test";

import {
  canonicalEdn,
  contentHash,
  WorkspaceConflictError,
  WorkspaceState
} from "./studio/workspace.js";

function manifest() {
  return {
    "hara/type": "workspace",
    "hara/version": "1.0.0",
    "workspace/id": "test",
    "workspace/layout": { "layout/type": "area", "layout/area": "area/editor" },
    "workspace/documents": [],
    "workspace/areas": [{ "area/id": "area/editor", "area/type": "code-editor" }],
    "workspace/nodes": [],
    "workspace/connections": [],
    "workspace/links": [],
    "workspace/customizations": {}
  };
}

test("canonical EDN is stable regardless of object insertion order", () => {
  const left = { "z/value": 2, "a/value": 1 };
  const right = { "a/value": 1, "z/value": 2 };
  assert.equal(canonicalEdn(left), canonicalEdn(right));
  assert.equal(canonicalEdn(left), "{\n  :a/value 1\n  :z/value 2\n}");
});

test("canvas creation and removal are structural and undoable", () => {
  const state = new WorkspaceState(manifest());
  state.addCanvas({ id: "canvas/spectrum", areaId: "area/spectrum", title: "Spectrum" });
  assert.equal(state.dirty, true);
  assert.equal(state.manifest["workspace/areas"].length, 2);
  assert.equal(state.manifest["workspace/layout"]["layout/type"], "split");
  assert.equal(state.undo(), true);
  assert.equal(state.manifest["workspace/areas"].length, 1);
  assert.equal(state.redo(), true);
  assert.equal(state.manifest["workspace/areas"].length, 2);
  state.removeCanvas("canvas/spectrum");
  assert.equal(state.manifest["workspace/areas"].length, 1);
  assert.equal(state.manifest["workspace/layout"]["layout/type"], "area");
});

test("node and connection mutations persist only structural metadata", () => {
  const state = new WorkspaceState(manifest());
  state.addNode({ "node/id": "node/fft", "node/type": "wasm/transform", "node/module": "fft.wasm" });
  state.connect({
    "connection/id": "connection/fft-viz",
    "connection/from": ["node/fft", "fft/bins"],
    "connection/to": ["node/viz", "fft/bins"],
    "connection/delivery": "latest"
  });
  const source = canonicalEdn(state.manifest);
  assert.ok(source.includes(':node/id "node/fft"'));
  assert.doesNotMatch(source, /audio-buffer|anonymous-module/);
});

test("save refuses to overwrite an externally changed workspace", async () => {
  const initial = `${canonicalEdn(manifest())}\n`;
  let disk = initial;
  const state = new WorkspaceState(manifest());
  state.baseHash = await contentHash(initial);
  state.addCanvas({ id: "canvas/a" });
  disk = `${initial}; external\n`;
  await assert.rejects(state.save({
    read: async () => disk,
    write: async (_path, source) => { disk = source; }
  }), (error) => {
    assert.ok(error instanceof WorkspaceConflictError);
    assert.notEqual(error.details.expectedHash, error.details.actualHash);
    return true;
  });
});

test("journal captures changes, restores recovery, and clears after save", async () => {
  const records = new Map();
  const journal = {
    read: async (path) => records.get(path) ?? null,
    write: async (path, value) => records.set(path, value),
    clear: async (path) => records.delete(path)
  };
  const initial = `${canonicalEdn(manifest())}\n`;
  const first = new WorkspaceState(manifest(), { journal });
  first.baseHash = await contentHash(initial);
  first.addCanvas({ id: "canvas/recovered" });
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.equal(records.get("workspace.edn").revision, 1);

  const second = new WorkspaceState(manifest(), { journal });
  second.baseHash = first.baseHash;
  second.recovery = await journal.read("workspace.edn");
  assert.equal(await second.recover(), true);
  assert.equal(second.manifest["workspace/areas"].length, 2);

  let disk = initial;
  await second.save({
    read: async () => disk,
    write: async (_path, source) => { disk = source; }
  });
  assert.equal(records.has("workspace.edn"), false);
  assert.equal(second.dirty, false);
});
