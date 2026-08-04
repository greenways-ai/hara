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

test("uses the compact ecosystem navigation and sign-in button", async () => {
  const layout = await readFile(new URL("../src/layouts/SiteLayout.astro", import.meta.url), "utf8");
  const shell = await readFile(new URL("../src/styles/shell.css", import.meta.url), "utf8");
  assert.match(layout, /Benchmarks[\s\S]*Docs[\s\S]*Specs/);
  assert.doesNotMatch(layout, />Source<\/a>/);
  assert.match(layout, /https:\/\/specs\.hara-lang\.org\//);
  assert.ok(layout.includes('href="https://id.hara-lang.org/">Sign in</a>'));
  assert.doesNotMatch(layout, /api\/session|auth\/github|return_to/);
  assert.match(shell, /grid-template-columns: auto minmax\(0, 1fr\) auto/);
  assert.doesNotMatch(shell, /nav\s*\{[^}]*display:\s*none/);
});

test("leads with the language and renders benchmark evidence from committed data", async () => {
  const page = await readFile(new URL("../src/pages/index.astro", import.meta.url), "utf8");
  assert.match(page, /Simple to pick up\.[\s\S]*Fast enough to keep\./);
  assert.match(page, /Hara is Lisp/);
  assert.match(page, /language-reference\.json/);
  assert.match(page, /reference-v2\.json/);
  assert.match(page, /formatRatio\("python-prepared"\)/);
  assert.match(page, /hoplite-request/);
  assert.doesNotMatch(page, /One language\.[\s\S]*Fit to its environment\./);
});

test("keeps the three runtime modes after the learning path", async () => {
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

test("orders the embedded docs around a first learning journey", async () => {
  const config = await readFile(new URL("../astro.config.mjs", import.meta.url), "utf8");
  assert.match(config, /Start here[\s\S]*Why Hara\?[\s\S]*Read Hara and build from scratch[\s\S]*Try Hara in the browser[\s\S]*Build Tic Tac Toe[\s\S]*Choose your setup/);
  assert.match(config, /Interactive courses[\s\S]*Choose a learning path[\s\S]*First Contact[\s\S]*Protocols for Builders[\s\S]*Collection Protocols[\s\S]*Protocol Atlas/);
  assert.match(config, /Hara language course/);
  assert.match(config, /Guides & reference/);
});

test("publishes the interactive syllabus controller and styles with docs", async () => {
  const config = await readFile(new URL("../astro.config.mjs", import.meta.url), "utf8");
  const prepare = await readFile(new URL("../scripts/prepare-docs.mjs", import.meta.url), "utf8");
  assert.match(config, /\/docs-assets\/stylesheets\/syllabus\.css/);
  assert.match(config, /\/docs-assets\/javascripts\/syllabus\.js/);
  assert.match(prepare, /javascripts\/syllabus\.js/);
  assert.match(prepare, /stylesheets\/syllabus\.css/);
  assert.match(prepare, /mkdir\(join\(runtimeDestination, "stylesheets"\)/);
});

test("hydrates Tic Tac Toe stages as REPL-attached canvas outputs", async () => {
  const repl = await readFile(new URL("../public/assets/docs-repl.js", import.meta.url), "utf8");
  const styles = await readFile(new URL("../src/styles/docs.css", import.meta.url), "utf8");
  assert.match(repl, /data-hara-canvas-stage/);
  assert.match(repl, /compileAnonymousDocument/);
  assert.match(repl, /registerCanvas\(runtime\)/);
  assert.match(repl, /waitForFirstRender/);
  assert.match(repl, /runner\.button\.click\(\)/);
  assert.match(repl, /"studio\.draw": "\/runtime\/studio\/hal\/draw\.hal"/);
  assert.match(styles, /\.hara-canvas-stage \.hara-repl/);
  assert.match(styles, /\.hara-live-canvas/);
  assert.match(styles, /aspect-ratio:4 \/ 3/);
});
