import assert from "node:assert/strict";
import test from "node:test";
import { applyParedit, barfForward, killToFormEnd, localFormAt, slurpForward, structuralAlign } from "./editor.js";

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

test("structural navigation slurps the next sibling and barfs the final expression", () => {
  const input = editor("(+ 1) 2", 3);
  assert.equal(slurpForward(input), true);
  assert.equal(input.value, "(+ 1 2)");
  assert.equal(input.selectionStart, 3);

  input.value = "(+ 1 2)";
  input.setSelectionRange(3, 3);
  assert.equal(barfForward(input), true);
  assert.equal(input.value, "(+ 1) 2");
  assert.equal(input.selectionStart, 3);
});

test("structural kill preserves the enclosing closing delimiter", () => {
  const input = editor("(+ 1 2)", 3);
  assert.equal(killToFormEnd(input), true);
  assert.equal(input.value, "(+ )");
  assert.equal(input.selectionStart, 3);
});

test("structural alignment derives indentation from enclosing delimiters", () => {
  const input = editor("(do\nvalue\n  )", 4);
  assert.equal(structuralAlign(input), true);
  assert.equal(input.value, "(do\n  value\n  )");
  assert.equal(input.selectionStart, 6);

  input.setSelectionRange(input.value.length - 1, input.value.length - 1);
  assert.equal(structuralAlign(input), true);
  assert.equal(input.value, "(do\n  value\n)");
});

test("structural alignment indents a blank line at the caret", () => {
  const input = editor("(do\n\n)", 4);
  assert.equal(structuralAlign(input), true);
  assert.equal(input.value, "(do\n  \n)");
  assert.equal(input.selectionStart, 6);
});

test("local evaluation selects the innermost balanced form", () => {
  const source = "(do (def x (+ 1 2)) (* x 3))";
  const form = localFormAt(source, source.indexOf("1"));
  assert.equal(form.source, "(+ 1 2)");
});

test("local evaluation uses the completed form to the left in whitespace", () => {
  const source = "(do (+ 1 2)  (* 3 4))";
  const caret = source.indexOf("  (*") + 1;
  assert.equal(localFormAt(source, caret).source, "(+ 1 2)");
});

test("local evaluation ignores delimiters inside comments and strings", () => {
  const source = '(do ; ) ignored\n  (println "[") )';
  assert.equal(localFormAt(source, source.indexOf("println")).source, '(println "[")');
});

test("local evaluation uses the previous form after a trailing newline", () => {
  const source = "{:version 1 :commands []}\n";
  assert.equal(localFormAt(source, source.length).source, "{:version 1 :commands []}");
});
