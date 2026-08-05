import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const redirects = new URL("../public/_redirects", import.meta.url);

test("www benchmark routes proxy the canonical observatory", async () => {
  const source = await readFile(redirects, "utf8");
  assert.match(source, /^\/benchmarks https:\/\/benchmarks\.hara-lang\.org\/ 200!$/m);
  assert.match(source, /^\/benchmarks\/ https:\/\/benchmarks\.hara-lang\.org\/ 200!$/m);
  assert.match(source, /^\/benchmarks\/\* https:\/\/benchmarks\.hara-lang\.org\/:splat 200!$/m);
  assert.match(source, /^https:\/\/docs\.hara-lang\.org\/\* https:\/\/www\.hara-lang\.org\/docs\/:splat 301!$/m);
});
