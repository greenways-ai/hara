import assert from "node:assert/strict";
import test from "node:test";
import { highlightHara } from "../src/highlight.js";

test("wraps parens in depth-cycled classes", () => {
  assert.equal(
    highlightHara("(+)"),
    '<span class="paren-0">(</span>+<span class="paren-0">)</span>'
  );
  assert.equal(
    highlightHara("[()]"),
    '<span class="paren-0">[</span><span class="paren-1">(</span><span class="paren-1">)</span><span class="paren-0">]</span>'
  );
});

test("marks unmatched closers", () => {
  assert.equal(highlightHara(")"), '<span class="unmatched">)</span>');
});

test("highlights strings with escapes and comments", () => {
  assert.equal(
    highlightHara('"a\\"b" ; note'),
    '<span class="string">"</span><span class="string">a</span><span class="string">\\</span><span class="string">"</span><span class="string">b</span><span class="string">"</span> <span class="comment">;</span><span class="comment"> </span><span class="comment">n</span><span class="comment">o</span><span class="comment">t</span><span class="comment">e</span>'
  );
});

test("a comment runs to the end of the line", () => {
  const html = highlightHara("; (ignored)\n(+ 1)");
  assert.ok(html.includes('<span class="comment">(</span>'));
  assert.ok(html.endsWith('<span class="paren-0">)</span>'));
});

test("highlights keywords as single spans", () => {
  assert.equal(
    highlightHara(":score"),
    '<span class="keyword">:score</span>'
  );
});

test("escapes HTML in source", () => {
  assert.equal(highlightHara("<>&"), "&lt;&gt;&amp;");
});

test("marks the eval range with eval-target", () => {
  const html = highlightHara("(+ 1 2)", { evalRange: { start: 0, end: 3 } });
  assert.equal(
    html,
    '<span class="paren-0 eval-target">(</span><span class="eval-target">+</span><span class="eval-target"> </span>1 2<span class="paren-0">)</span>'
  );
});
