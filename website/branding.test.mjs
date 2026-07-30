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

test("the main story plays and edits the real Amp pipeline", async () => {
  const [source, app] = await Promise.all([
    websiteFile("story.js"),
    websiteFile("app.js")
  ]);

  assert.match(source, /PLAY THE PROGRAM/);
  assert.match(source, /EDIT THE SYSTEM/);
  assert.match(source, /data-amp-node-graph/);
  assert.match(source, /createNodeGraph/);
  assert.match(source, /patchAmpParameter/);
  assert.match(source, /data-step-slots/);
  assert.match(source, /data-note-bank/);
  assert.match(source, /AMP REPL · ACTIVE DOCUMENT/);
  assert.match(source, /data-repl-completions/);
  assert.match(source, /instance\.eval/);
  assert.match(source, /NODE VIEW/);
  assert.match(source, /TEXT VIEW/);
  assert.match(source, /src\/amp\.hal/);
  assert.match(source, /APPLY \+ REBUILD/);
  assert.match(source, /data-story-source/);
  assert.match(source, /CREATE THIS WORKSPACE/);
  assert.match(app, /setWorkspace\(0, \{ reloadBackground: false \}\)/);
  assert.doesNotMatch(source, /One program\.<br>Every medium/);
  assert.doesNotMatch(source, /03 \/\/ GREENWAYS OS/);
});
