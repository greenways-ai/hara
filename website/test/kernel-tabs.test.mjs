import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const homepageUrl = new URL("../src/pages/index.astro", import.meta.url);
const stylesUrl = new URL("../src/styles/site.css", import.meta.url);

test("kernel modes are exposed as accessible tabs rather than a dropdown", async () => {
  const homepage = await readFile(homepageUrl, "utf8");
  assert.match(homepage, /class="kernel-mode-tabs" role="tablist"/);
  assert.equal((homepage.match(/role="tab" data-kernel-tab=/g) ?? []).length, 3);
  assert.match(homepage, /data-kernel-tab="java"/);
  assert.match(homepage, /data-kernel-tab="native"/);
  assert.match(homepage, /data-kernel-tab="web"/);
  assert.match(homepage, /role="tabpanel"/);
  assert.match(homepage, /ArrowUp/);
  assert.match(homepage, /ArrowRight/);
  assert.doesNotMatch(homepage, /<select id="kernel-mode"/);
});

test("kernel tabs use the approved vertical selector on wide screens", async () => {
  const styles = await readFile(stylesUrl, "utf8");
  assert.match(styles, /\.kernel-picker\s*\{[\s\S]*grid-template-columns:\s*minmax\(210px/);
  assert.match(styles, /\.kernel-mode-tabs\s*\{[\s\S]*display:\s*grid/);
  assert.match(styles, /\.kernel-mode-tabs button\[aria-selected="true"\]/);
  assert.match(styles, /\.kernel-mode-tabs button\[aria-selected="true"\]::before/);
});
