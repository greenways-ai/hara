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

test("the main story connects and plays the real Amp pipeline", async () => {
  const source = await websiteFile("story.js");

  assert.match(source, /CONNECT THE SYSTEM/);
  assert.match(source, /SYNTH WASM/);
  assert.match(source, /WEB AUDIO \+ EQ/);
  assert.match(source, /FFT WASM/);
  assert.match(source, /HTA TRANSPORT/);
  assert.match(source, /HAL PROGRAM/);
  assert.match(source, /CANVAS OUTPUT/);
  assert.match(source, /PLAY THE SYSTEM/);
  assert.match(source, /CREATE THIS WORKSPACE/);
  assert.doesNotMatch(source, /One program\.<br>Every medium/);
  assert.doesNotMatch(source, /03 \/\/ GREENWAYS OS/);
});
