import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const websiteFile = (name) => readFile(new URL(name, import.meta.url), "utf8");

test("the main site presents Hara as a programmable kernel", async () => {
  const source = await websiteFile("index.html");

  assert.match(source, /<title>Hara<\/title>/);
  assert.match(source, /<meta property="og:title" content="Hara">/);
  assert.match(source, /A Programmable Kernel for the Agentic Age\./);
  assert.doesNotMatch(source, /A Modern Lisp/);
});

test("the Amp demo uses the shared page identity", async () => {
  const source = await websiteFile("hara-amp.html");

  assert.match(source, /<title>Hara \/ Amp Demo<\/title>/);
  assert.match(source, /HARA \/ AMP DEMO/);
});
