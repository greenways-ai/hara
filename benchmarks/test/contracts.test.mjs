import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const catalog = JSON.parse(await readFile(new URL("../../lib/bench/catalog.json", import.meta.url)));
const page = await readFile(new URL("../src/pages/index.astro", import.meta.url), "utf8");
const language = await readFile(new URL("../src/components/LanguagePanel.astro", import.meta.url), "utf8");
const reference = await readFile(new URL("../src/components/RuntimeReference.astro", import.meta.url), "utf8");
const header = await readFile(new URL("../src/components/SiteHeader.astro", import.meta.url), "utf8");
const data = await readFile(new URL("../src/lib/benchmark-data.ts", import.meta.url), "utf8");
const source = [page, language, reference, header, data].join("\n");
const indexOf = (value) => { const index = language.indexOf(value); assert.notEqual(index, -1, `Expected language panel to contain ${value}`); return index; };

test("uses the nine canonical internal artifacts", () => {
  assert.deepEqual(catalog.artifacts.map(({ id }) => id), ["hara-wasm-core","hara-rust-vm","hara-rust-full","hara-wasm-vm","hara-wasm-full","hara-truffle-vm","hara-truffle-full","hara-jvm-vm","hara-jvm-full"]);
});
test("opens on a three-tab language-first presentation", () => {
  assert.equal((page.match(/<button role="tab"/g) ?? []).length, 3);
  assert.match(page, /aria-selected="true" aria-controls="language-shootout"/);
  assert.doesNotMatch(source, /aria-controls="methodology"|id="methodology"/);
});
test("puts overview and insights before the drill-down matrix", () => {
  const overview = indexOf("Overview");
  const topInsights = indexOf("Top insights");
  const matrix = indexOf("Hara comparison matrix");
  const explanation = indexOf("What just happened");
  const haraInsights = indexOf("Hara insights");
  assert.ok(overview < topInsights && topInsights < matrix && matrix < explanation && explanation < haraInsights);
  assert.match(language, /data-comparison-cell/);
  assert.match(language, /data-matrix-detail/);
});
test("explains methodology below each result instead of in a separate tab", () => {
  assert.match(language, /Equivalent work[\s\S]*Prepared execution[\s\S]*One Hara baseline[\s\S]*Geometric mean/);
  assert.match(page, /RuntimePanel/);
  assert.match(page, /HttpPanel/);
});
test("moves the product-mode table to a collapsed reference at the end", () => {
  assert.match(reference, /<details>/);
  assert.match(reference, /<th>Java<\/th>[\s\S]*<th>Native<\/th>[\s\S]*<th>Web<\/th>/);
  assert.ok(page.indexOf("RuntimeReference") > page.indexOf("LanguagePanel"));
});
test("uses the shared Hara navigation and sign-in button", () => {
  assert.match(header, /aria-current="page" aria-disabled="true">Benchmarks/);
  assert.match(header, /Benchmarks[\s\S]*Docs[\s\S]*Specs/);
  assert.doesNotMatch(header, />Source<\/a>/);
  assert.match(header, /https:\/\/specs\.hara-lang\.org\//);
  assert.ok(header.includes('href="https://id.hara-lang.org/">Sign in</a>'));
  assert.doesNotMatch(source, /api\/session|auth\/github|return_to/);
});
test("uses a dedicated maximum-resolution benchmark social card", () => {
  assert.match(page, /og-hara-benchmarks\.jpg/);
  assert.match(page, /og:image:width" content="3840"/);
  assert.match(page, /og:image:height" content="2016"/);
});
test("external comparisons use only the native full Hara tier", () => {
  assert.match(data, /haraRuntime = "hara-rust-full"/);
  assert.match(language, /Every external row is compared only with/);
  assert.doesNotMatch(source, /hara-rust-vm[^<]*compet/);
});
