import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const pkg = JSON.parse(await readFile(new URL("../package.json", import.meta.url)));
test("exports the public theme and Astro contract", () => {
  for (const path of ["./tokens.css", "./theme.css", "./motifs.css", "./theme.js",
    "./astro/ThemeToggle.astro", "./astro/HaraMark.astro", "./astro/Motif.astro", "./astro/Surface.astro"])
    assert.ok(pkg.exports[path], `missing ${path}`);
  assert.equal(pkg.license, "MIT");
});

test("theme contract supports cross-domain system, light, and dark preferences", async () => {
  const source = await readFile(new URL("../src/theme.js", import.meta.url), "utf8");
  assert.match(source, /hara-theme/);
  assert.match(source, /Domain=hara-lang\.org/);
  assert.match(source, /hara:theme-change/);
});

test("motifs are resolution-independent and do not load the raster reference", async () => {
  const motifs = await readFile(new URL("../src/motifs.css", import.meta.url), "utf8");
  assert.doesNotMatch(motifs, /precision-motifs\.png|background-size:\s*300%/);
  for (const primitive of ["linear-gradient", "radial-gradient", "conic-gradient"])
    assert.match(motifs, new RegExp(primitive));
});
