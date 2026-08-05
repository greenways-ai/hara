import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const pongUrl = new URL("../sources/pong.hal", import.meta.url);
const homepageUrl = new URL("../src/pages/index.astro", import.meta.url);
const nodeHalUrl = new URL("../../rust/web/studio/hal/node.hal", import.meta.url);

function assertBalanced(source) {
  const stack = [];
  const pairs = { ")": "(", "]": "[", "}": "{" };
  let string = false;
  let escaped = false;
  let comment = false;
  for (const character of source) {
    if (comment) {
      if (character === "\n") comment = false;
      continue;
    }
    if (string) {
      if (escaped) escaped = false;
      else if (character === "\\") escaped = true;
      else if (character === '"') string = false;
      continue;
    }
    if (character === ";") comment = true;
    else if (character === '"') string = true;
    else if (["(", "[", "{"].includes(character)) stack.push(character);
    else if (character in pairs) assert.equal(stack.pop(), pairs[character]);
  }
  assert.deepEqual(stack, []);
}

test("homepage Pong uses the canonical editable source", async () => {
  const homepage = await readFile(homepageUrl, "utf8");
  assert.match(homepage, /import pongSource from "\.\.\/\.\.\/sources\/pong\.hal\?raw"/);
  assert.match(homepage, /kind: "canvas", source: pongSource/);
  assert.doesNotMatch(homepage, /getLiveSnippet\("canvas-pong"\)/);
});

test("Pong keeps namespace setup locally evaluable and sequences its frame loop", async () => {
  const source = await readFile(pongUrl, "utf8");
  assertBalanced(source);
  assert.match(source, /^\(ns\+\)\n\n\(require \[studio\.draw :as draw\]\)/);
  assert.doesNotMatch(source, /\(ns\+[\s\S]*?:require \[studio\.draw/);
  assert.match(source, /\(let \[delta[\s\S]*?\]\n    \(let \[step/);
  assert.match(source, /\(let \[tracked[\s\S]*?\]\n    \(let \[moved/);
  assert.match(source, /\(let \[frame[\s\S]*?\]\n        \(let \[width/);
  assert.match(source, /\(node\/start/);
  assert.doesNotMatch(source, /\(loop \[state \(initial-state\) tick/);
});

test("studio node task and handler registries are dereferenced exactly once", async () => {
  const source = await readFile(nodeHalUrl, "utf8");
  assert.match(source, /\(let \[entry \(deref \*active-task\*\)\]/);
  assert.match(source, /find-handler \(deref \*handlers\*\)/);
  assert.doesNotMatch(source, /\(deref \(deref \*active-task\*\)\)/);
  assert.doesNotMatch(source, /\(deref \(deref \*handlers\*\)\)/);
});
