import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

test("publishes docs below /docs and uses the visual package", async () => {
  const content = await readFile(new URL("../src/content.config.ts", import.meta.url), "utf8");
  const prepare = await readFile(new URL("../scripts/prepare-docs.mjs", import.meta.url), "utf8");
  const layout = await readFile(new URL("../src/layouts/SiteLayout.astro", import.meta.url), "utf8");
  assert.match(content, /docsLoader/);
  assert.match(prepare, /src\/content\/docs\/docs/);
  assert.match(layout, /@hara-lang\/visual-language/);
  assert.doesNotMatch(layout, /docs\.hara-lang\.org/);
});
