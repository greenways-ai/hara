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

test("ships a runnable core kernel and three product modes without hero size claims", async () => {
  const page = await readFile(new URL("../src/pages/index.astro", import.meta.url), "utf8");
  const repl = await readFile(new URL("../public/assets/docs-repl.js", import.meta.url), "utf8");
  const prepare = await readFile(new URL("../scripts/prepare-docs.mjs", import.meta.url), "utf8");
  assert.match(page, /id="kernel-mode"/);
  assert.match(page, /Java[\s\S]*Native[\s\S]*Web/);
  assert.doesNotMatch(page, /compressed browser VM/);
  assert.match(repl, /manifest\.variants\.core\.url/);
  assert.match(repl, /data-hara-eval/);
  assert.match(prepare, /clojure\$1/);
});
