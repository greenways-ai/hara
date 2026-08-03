import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
const catalog = JSON.parse(await readFile(new URL("../../lib/bench/catalog.json", import.meta.url)));
const page = await readFile(new URL("../src/pages/index.astro", import.meta.url), "utf8");
test("uses exactly eight canonical artifacts", () => assert.deepEqual(catalog.artifacts.map(({id}) => id), ["hara-rust-vm","hara-rust-full","hara-wasm-vm","hara-wasm-full","hara-truffle-vm","hara-truffle-full","hara-jvm-vm","hara-jvm-full"]));
test("has one summary and four non-nested result tabs", () => { assert.equal((page.match(/class="summary hara-motif"/g) ?? []).length, 1); assert.equal((page.match(/role="tabpanel"/g) ?? []).length, 4); assert.doesNotMatch(page, /<select/); });
test("external comparisons name only rust full", () => { assert.match(page, /highest-performance native tier/); assert.doesNotMatch(page, /hara-rust-vm[^<]*compet/); });
