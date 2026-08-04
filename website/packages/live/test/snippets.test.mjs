import assert from "node:assert/strict";
import test from "node:test";
import { LIVE_SNIPPETS, getLiveSnippet } from "../src/snippets.js";

const KINDS = new Set(["console", "canvas"]);

test("every snippet has a valid registry shape", () => {
  assert.ok(LIVE_SNIPPETS.length >= 4, "expected a curated set of snippets");
  const ids = new Set();
  for (const snippet of LIVE_SNIPPETS) {
    assert.equal(typeof snippet.id, "string");
    assert.match(snippet.id, /^[a-z0-9-]+$/);
    assert.ok(!ids.has(snippet.id), `duplicate snippet id ${snippet.id}`);
    ids.add(snippet.id);
    assert.equal(typeof snippet.title, "string");
    assert.ok(snippet.title.length > 0);
    assert.ok(KINDS.has(snippet.kind), `${snippet.id}: unknown kind ${snippet.kind}`);
    assert.equal(typeof snippet.source, "string");
    assert.ok(snippet.source.trim().length > 0, `${snippet.id}: empty source`);
  }
});

test("required demo snippets are present", () => {
  for (const id of ["first-eval", "collections", "state", "tictactoe-move"]) {
    assert.ok(getLiveSnippet(id), `missing snippet ${id}`);
  }
  assert.ok(LIVE_SNIPPETS.some((snippet) => snippet.kind === "canvas"),
    "expected at least one canvas snippet");
});

test("canvas snippets follow the docs canvas-stage contract", () => {
  for (const snippet of LIVE_SNIPPETS.filter((entry) => entry.kind === "canvas")) {
    assert.ok(snippet.source.includes("(:require [studio.draw :as draw])"),
      `${snippet.id}: missing studio.draw require`);
    assert.ok(snippet.source.includes('"canvas/background"'),
      `${snippet.id}: must render to the canvas/background canvas id`);
    assert.ok(snippet.source.includes("(node/start"),
      `${snippet.id}: must start a node task`);
  }
});

test("getLiveSnippet returns null for unknown ids", () => {
  assert.equal(getLiveSnippet("nope"), null);
});
