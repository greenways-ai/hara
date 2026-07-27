import assert from "node:assert/strict";
import test from "node:test";
import { applyParedit } from "./editor.js";

function editor(value, start = value.length, end = start) {
  return {
    value, selectionStart: start, selectionEnd: end,
    setRangeText(text, from, to) {
      this.value = this.value.slice(0, from) + text + this.value.slice(to);
      this.selectionStart = this.selectionEnd = from + text.length;
    },
    setSelectionRange(from, to) { this.selectionStart = from; this.selectionEnd = to; },
    dispatchEvent() {}
  };
}

test("paredit inserts balanced pairs", () => {
  const input = editor("(def x ");
  assert.equal(applyParedit(input, "["), true);
  assert.equal(input.value, "(def x []");
  assert.equal(input.selectionStart, 8);
});

test("paredit wraps a selection", () => {
  const input = editor("hello", 0, 5);
  applyParedit(input, "(");
  assert.equal(input.value, "(hello)");
  assert.deepEqual([input.selectionStart, input.selectionEnd], [1, 6]);
});

test("paredit skips an existing closer and removes empty pairs", () => {
  const input = editor("()", 1);
  applyParedit(input, ")");
  assert.equal(input.selectionStart, 2);
  input.setSelectionRange(1, 1);
  applyParedit(input, "Backspace");
  assert.equal(input.value, "");
});
