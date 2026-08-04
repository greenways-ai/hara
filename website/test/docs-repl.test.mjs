import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import {
  createDocsSessionRegistry,
  describeDocsSession
} from "../public/assets/docs-repl-state.js";
import remarkHaraEval, { parseHaraEvalScope } from "../scripts/remark-hara-eval.mjs";

test("describes isolated, page-global, and named group sessions", () => {
  const isolated = describeDocsSession({ pagePath: "/docs/start/orientation/", sequence: 2 });
  const global = describeDocsSession({ scope: "global", pagePath: "/docs/start/orientation/", sequence: 9 });
  const group = describeDocsSession({ scope: "group", groupName: "Board state", pagePath: "/docs/create/game/", sequence: 3 });

  assert.deepEqual(isolated, {
    scope: "isolated",
    label: "isolated",
    id: "docs-start-orientation-2",
    filesystem: "memory:docs-start-orientation-2",
    sharedWith: "this runner only"
  });
  assert.equal(global.label, "global");
  assert.equal(global.id, "docs-start-orientation-global");
  assert.equal(group.label, "group Board state");
  assert.equal(group.id, "docs-create-game-group-board-state");
});

test("reuses one session promise for repeated evaluations", async () => {
  const calls = [];
  const kernel = {
    async createSession(id, options) {
      calls.push([id, options]);
      return { id };
    }
  };
  const registry = createDocsSessionRegistry(Promise.resolve(kernel));
  const descriptor = describeDocsSession({ pagePath: "/docs/start/orientation/", sequence: 1 });

  const first = registry.get(descriptor);
  const second = registry.get(descriptor);
  assert.strictEqual(first, second);
  assert.deepEqual(await first, { id: descriptor.id });
  assert.deepEqual(calls, [[descriptor.id, { filesystem: descriptor.filesystem }]]);
});

test("shares global and named group sessions but not isolated sessions", () => {
  const firstGlobal = describeDocsSession({ scope: "global", pagePath: "/docs/start/", sequence: 1 });
  const secondGlobal = describeDocsSession({ scope: "global", pagePath: "/docs/start/", sequence: 2 });
  const firstGroup = describeDocsSession({ scope: "group", groupName: "lesson", pagePath: "/docs/start/", sequence: 1 });
  const secondGroup = describeDocsSession({ scope: "group", groupName: "lesson", pagePath: "/docs/start/", sequence: 2 });
  const firstIsolated = describeDocsSession({ pagePath: "/docs/start/", sequence: 1 });
  const secondIsolated = describeDocsSession({ pagePath: "/docs/start/", sequence: 2 });

  assert.equal(firstGlobal.id, secondGlobal.id);
  assert.equal(firstGroup.id, secondGroup.id);
  assert.notEqual(firstIsolated.id, secondIsolated.id);
});

test("parses documentation evaluator scopes", () => {
  assert.deepEqual(parseHaraEvalScope("eval"), { scope: "isolated", groupName: "" });
  assert.deepEqual(parseHaraEvalScope("eval global"), { scope: "global", groupName: "" });
  assert.deepEqual(parseHaraEvalScope("eval group=lesson"), { scope: "group", groupName: "lesson" });
  assert.deepEqual(parseHaraEvalScope('eval group "Board state"'), { scope: "group", groupName: "Board state" });
});

test("emits scope metadata on runnable Hara fences", () => {
  const tree = {
    type: "root",
    children: [
      { type: "code", lang: "clojure", meta: "eval global", value: "(+ 19 23)" },
      { type: "code", lang: "clojure", meta: "eval group=lesson", value: "(def answer 42)" }
    ]
  };

  remarkHaraEval()(tree);

  assert.match(tree.children[0].value, /data-hara-scope="global"/);
  assert.match(tree.children[1].value, /data-hara-scope="group"/);
  assert.match(tree.children[1].value, /data-hara-group="lesson"/);
});

test("keeps readiness in each runner instead of a persistent output or ready toast", async () => {
  const repl = await readFile(new URL("../public/assets/docs-repl.js", import.meta.url), "utf8");
  assert.match(repl, /dataset\.connectionState/);
  assert.match(repl, /<details class="hara-repl-details">/);
  assert.match(repl, /<output hidden/);
  assert.match(repl, /progress\.toast\.remove\(\)/);
  assert.doesNotMatch(repl, /Kernel loading/);
  assert.doesNotMatch(repl, /hara-wasm-core ready/);
});

test("shows the user-facing scope and keeps the runtime in details", async () => {
  const repl = await readFile(new URL("../public/assets/docs-repl.js", import.meta.url), "utf8");
  const state = await readFile(new URL("../public/assets/docs-repl-state.js", import.meta.url), "utf8");
  assert.match(repl, /data-hara-session-label/);
  assert.match(repl, /<div><dt>Runtime<\/dt><dd>hara-wasm-core<\/dd><\/div>/);
  assert.match(state, /label: "isolated"/);
  assert.match(state, /label: "global"/);
  assert.match(state, /label: `group \$\{normalizedGroup\}`/);
});
