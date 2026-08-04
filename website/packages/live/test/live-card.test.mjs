import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { print } from "../src/live-card.js";

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

test("live card exposes the compact Eval/Run/example header and touch eval", async () => {
  const source = await readFile(new URL("../src/live-card.js", import.meta.url), "utf8");
  assert.match(source, /data-live-eval/);
  assert.match(source, /data-live-run/);
  assert.match(source, /data-live-example/);
  assert.match(source, /pointerType !== "touch"/);
  assert.match(source, /Open in Playground/);
  assert.doesNotMatch(source, /hara-live-card-brand/);
  assert.doesNotMatch(source, /data-live-reset/);
});
