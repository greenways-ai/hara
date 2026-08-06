import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import {
  afterCaretPlacement,
  cancelEvaluation,
  print,
  waitForCanvasFirstFrame
} from "../src/live-card.js";

class HtaKeyword { constructor(name) { this.name = name; } }
class HtaSymbol { constructor(name) { this.name = name; } }
class HtaAtom { constructor(value) { this.value = value; } }
class HtaArray { constructor(values) { this.values = values; } }
class HtaObject { constructor(entries) { this.entries = entries; } }

test("prints decoded HTA keywords, symbols and nested collections", () => {
  const value = new Map([
    [new HtaKeyword("turn"), new HtaSymbol("x")],
    [new HtaKeyword("moves"), [[1, 1]]]
  ]);
  assert.equal(print(value), "{:turn x :moves [[1 1]]}");
});

test("prints HTA wrappers without object placeholders", () => {
  assert.equal(print(new HtaAtom(new HtaKeyword("ready"))), "#atom <:ready>");
  assert.equal(print(new HtaArray([1, new HtaSymbol("x")])), "(array 1 x)");
  assert.equal(
    print(new HtaObject([["mode", new HtaKeyword("mobile")]])),
    '(object "mode" :mobile)'
  );
  assert.equal(print(new Set([new HtaKeyword("a"), 2])), "#{:a 2}");
  assert.equal(print({ ready: true }), '#js {"ready" true}');
});

test("guards against cyclic host values", () => {
  const value = [];
  value.push(value);
  assert.equal(print(value), "[#<cycle>]");
});

test("canvas startup reports the real task error before a generic timeout", async () => {
  const failure = new Error("unresolved symbol in Pong");
  await assert.rejects(
    waitForCanvasFirstFrame(new Promise(() => {}), Promise.reject(failure)),
    /unresolved symbol in Pong/
  );
});

test("canvas startup rejects tasks that stop without drawing", async () => {
  await assert.rejects(
    waitForCanvasFirstFrame(new Promise(() => {}), Promise.resolve(null)),
    /stopped before rendering its first frame/
  );
});

test("cancellable HTA evaluations can be interrupted without closing the kernel", () => {
  let calls = 0;
  assert.equal(cancelEvaluation({ cancel() { calls += 1; return true; } }), true);
  assert.equal(calls, 1);
  assert.equal(cancelEvaluation(Promise.resolve()), false);
});

test("mobile caret evaluation waits for the browser placement frame", () => {
  const frames = [];
  let calls = 0;
  afterCaretPlacement(() => { calls += 1; }, {
    requestFrame: (callback) => frames.push(callback)
  });
  assert.equal(calls, 0);
  assert.equal(frames.length, 1);
  frames.shift()();
  assert.equal(calls, 1);
});

test("a superseded mobile caret evaluation can be cancelled", () => {
  const frames = [];
  let calls = 0;
  const cancel = afterCaretPlacement(() => { calls += 1; }, {
    requestFrame: (callback) => frames.push(callback)
  });
  cancel();
  frames.shift()();
  assert.equal(calls, 0);
});

test("live card exposes tabs, InstaREPL, resizers, interrupt and reset", async () => {
  const source = await readFile(new URL("../src/live-card.js", import.meta.url), "utf8");
  const styles = await readFile(new URL("../src/style.css", import.meta.url), "utf8");
  assert.match(source, /data-live-eval/);
  assert.match(source, /data-live-run/);
  assert.match(source, /data-live-reset/);
  assert.match(source, /role="tablist"/);
  assert.match(source, /role", "tab"/);
  assert.match(source, /pointerdown/);
  assert.match(source, /pointerType !== "touch" && event\.button !== 0/);
  assert.match(source, /pendingPointerEvaluation = true/);
  assert.match(source, /addEventListener\("click"/);
  assert.match(source, /afterCaretPlacement/);
  assert.match(source, /createVerticalResizer\(editorSurface/);
  assert.match(source, /createVerticalResizer\(panel/);
  assert.match(source, /waitForCanvasFirstFrame\(rendered, task\)/);
  assert.match(source, /cancelEvaluation\(task\)/);
  assert.match(source, /Stop or Esc to interrupt/);
  assert.match(source, /interrupt: stopCanvas/);
  assert.match(source, /resetButton\.addEventListener\("click", reset\)/);
  assert.match(source, /Open in Playground/);
  assert.match(styles, /\.hara-live-card-tabs/);
  assert.match(styles, /\.hara-live-card-resizer/);
  assert.doesNotMatch(source, /setTimeout\(\(\) => evalCurrent/);
  assert.doesNotMatch(source, /data-live-example/);
  assert.doesNotMatch(source, /<select/);
});

test("demo tabs own the top row and kernel progress never overlays the card", async () => {
  const styles = await readFile(new URL("../src/style.css", import.meta.url), "utf8");
  assert.match(styles, /\.hara-live-card-header\s*\{[\s\S]*grid-template-rows:\s*auto auto/);
  assert.match(styles, /\.hara-live-card-tabs\s*\{[\s\S]*grid-row:\s*1/);
  assert.match(styles, /\.hara-live-card-tabs::-webkit-scrollbar\s*\{\s*display:\s*none/);
  assert.match(styles, /\.hara-live-card-toast\s*\{\s*display:\s*none !important/);
});
