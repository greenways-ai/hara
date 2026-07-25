import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";
import "fake-indexeddb/auto";

import { HtaContext, HtaKeyword } from "./hta.js";
import { KernelBroker } from "./studio/broker.js";
import { defaultBootstrap } from "./studio/boot.js";
import { createHostServices } from "./studio/host-services.js";

// Real-wasm integration tests for the studio hara libraries
// (rust/web/studio/hal/*.hal): store/fs/space/boot behaviour is asserted by
// evaluating hara source in actual HTA kernels. Skipped when the raw wasm
// artifact has not been built.
const wasmUrl = new URL("../raw/target/wasm32-unknown-unknown/release/hara_wasm_raw.wasm", import.meta.url);
const wasmBytes = await readFile(wasmUrl).catch(() => null);
const hal = (name) => readFile(new URL(`./studio/hal/${name}.hal`, import.meta.url), "utf8");
const resources = wasmBytes === null
  ? null
  : {
      "studio.store": await hal("store"),
      "studio.fs": await hal("fs"),
      "studio.space": await hal("space"),
      "studio.boot": await hal("boot")
    };

const LISTING_URL = "https://data.jsdelivr.com/v1/packages/gh/octo/lessons@main";
const CDN_PREFIX = "https://cdn.jsdelivr.net/gh/octo/lessons@main";
const LISTING = JSON.stringify({
  type: "package",
  name: "octo/lessons",
  version: "main",
  files: [
    { type: "file", name: "/README.md", size: 10 },
    {
      type: "directory",
      name: "/src",
      files: [
        { type: "file", name: "/src/intro.hal", size: 7 },
        { type: "file", name: "/src/advanced.hal", size: 7 }
      ]
    }
  ]
});
const FILES = {
  "/README.md": "# Lessons",
  "/src/intro.hal": "(+ 1 2)",
  "/src/advanced.hal": "(+ 3 4)"
};

function mockFetch({ failFor } = {}) {
  return async (url) => {
    if (url === LISTING_URL) return { ok: true, status: 200, text: async () => LISTING };
    if (url.startsWith(CDN_PREFIX)) {
      const path = url.slice(CDN_PREFIX.length);
      if (failFor !== path && path in FILES) {
        return { ok: true, status: 200, text: async () => FILES[path] };
      }
    }
    return { ok: false, status: 404, text: async () => "not found" };
  };
}

// Each kernel needs its own hta-worker instance behind its own `self` bridge;
// the cache-busting query forces node to evaluate the module afresh so it
// binds to the bridge installed just before the import.
let kernelCounter = 0;
async function spawnRealKernel(hostCalls) {
  kernelCounter += 1;
  const bridge = { listeners: {}, selfListeners: {} };
  bridge.self = {
    addEventListener: (type, handler) => {
      bridge.selfListeners[type] = handler;
    },
    postMessage: (data) => bridge.listeners.message?.({ data }),
    close: () => {}
  };
  globalThis.self = bridge.self;
  await import(`./hta-worker.js?kernel=${kernelCounter}`);
  const worker = {
    terminated: false,
    addEventListener: (type, handler) => {
      bridge.listeners[type] = handler;
    },
    postMessage: (message) => bridge.selfListeners.message({ data: message }),
    terminate() {
      this.terminated = true;
    }
  };
  return { context: new HtaContext({ worker, moduleBytes: wasmBytes, hostCalls }), worker };
}

function makeBroker({ fetch } = {}) {
  const hostCalls = createHostServices({ dbName: "hara-studio-hal-test", fetch: fetch ?? mockFetch() });
  return new KernelBroker({ resources, spawn: () => spawnRealKernel(hostCalls) });
}

const REQUIRE_ALL =
  "(require [studio.store :as store]) " +
  "(require [studio.fs :as fs]) " +
  "(require [studio.space :as space]) " +
  "(require [studio.boot :as boot])";
const evaluate = (broker, source) => broker.eval("ROOT", `(do ${REQUIRE_ALL} ${source})`);

// hta maps decode to JS Maps with HtaKeyword (or string) keys.
function mapGet(map, name) {
  for (const [key, value] of map) {
    if (key instanceof HtaKeyword && key.name === name) return value;
    if (key === name) return value;
  }
  return undefined;
}

test("defaultBootstrap renders the shared bootstrap template", { skip: wasmBytes === null }, () => {
  assert.equal(
    defaultBootstrap("boot-space"),
    '(do (require [studio.boot :as boot]) (boot/boot! "boot-space"))'
  );
});

test("studio.store round trips string values and lists keys", { skip: wasmBytes === null }, async () => {
  const broker = makeBroker();
  assert.equal(await evaluate(broker, '(store/put! "test-store/alpha" "one")'), true);
  assert.equal(await evaluate(broker, '(store/put! "test-store/beta" "two")'), true);
  assert.equal(await evaluate(broker, '(store/get "test-store/alpha")'), "one");
  assert.equal(await evaluate(broker, '(store/get "test-store/missing")'), null);
  assert.deepEqual(await evaluate(broker, '(store/keys "test-store/")'), ["test-store/alpha", "test-store/beta"]);
  assert.equal(await evaluate(broker, '(store/del! "test-store/alpha")'), true);
  assert.equal(await evaluate(broker, '(store/get "test-store/alpha")'), null);
  assert.deepEqual(await evaluate(broker, '(store/keys "test-store/")'), ["test-store/beta"]);
  assert.ok((await evaluate(broker, "(count (store/keys))")) > 0);
});

test("studio.fs scopes files per space and lists by directory", { skip: wasmBytes === null }, async () => {
  const broker = makeBroker();
  await evaluate(broker, '(fs/write! "alpha" "/intro.hal" "alpha-intro")');
  await evaluate(broker, '(fs/write! "alpha" "/docs/day1.hal" "day-one")');
  await evaluate(broker, '(fs/write! "beta" "/intro.hal" "beta-intro")');

  assert.equal(await evaluate(broker, '(fs/read "alpha" "/intro.hal")'), "alpha-intro");
  assert.equal(await evaluate(broker, '(fs/read "beta" "/intro.hal")'), "beta-intro");
  assert.equal(await evaluate(broker, '(fs/read "alpha" "/absent.hal")'), null);
  assert.equal(await evaluate(broker, '(fs/exists? "alpha" "/intro.hal")'), true);
  assert.equal(await evaluate(broker, '(fs/exists? "alpha" "/absent.hal")'), false);

  // Space A's listing never sees space B's files.
  assert.deepEqual(await evaluate(broker, '(fs/list "alpha" "/")'), ["/docs/day1.hal", "/intro.hal"]);
  assert.deepEqual(await evaluate(broker, '(fs/list "alpha" "/docs")'), ["/docs/day1.hal"]);
  assert.deepEqual(await evaluate(broker, '(fs/list "beta" "/")'), ["/intro.hal"]);

  await evaluate(broker, '(fs/delete! "alpha" "/intro.hal")');
  assert.equal(await evaluate(broker, '(fs/exists? "alpha" "/intro.hal")'), false);
  assert.equal(await evaluate(broker, '(fs/read "beta" "/intro.hal")'), "beta-intro");
});

test("studio.space creates, checks, and lists spaces and their files", { skip: wasmBytes === null }, async () => {
  const broker = makeBroker();
  assert.equal(await evaluate(broker, '(space/create! "gamma")'), true);
  assert.equal(await evaluate(broker, '(space/exists? "gamma")'), true);
  assert.equal(await evaluate(broker, '(space/exists? "ghost-space")'), false);

  await evaluate(broker, '(fs/write! "gamma" "/a.hal" "a")');
  await evaluate(broker, '(fs/write! "delta" "/b.hal" "b")'); // files before meta
  assert.deepEqual(await evaluate(broker, '(space/files "gamma")'), ["/a.hal"]);
  assert.deepEqual(await evaluate(broker, '(space/tree "gamma")'), ["/a.hal"]);

  const spaces = await evaluate(broker, "(space/list-spaces)");
  assert.ok(spaces.includes("gamma"));
  assert.ok(spaces.includes("delta")); // discovered from file keys too
  assert.ok(!spaces.includes("ghost-space"));
  assert.equal(new Set(spaces).size, spaces.length); // no duplicates
});

test("studio.space import-github! fetches the listing and writes every file", { skip: wasmBytes === null }, async () => {
  const broker = makeBroker();
  const summary = await evaluate(broker, '(space/import-github! "imported" "octo/lessons" {:ref "main"})');
  assert.equal(mapGet(summary, "space"), "imported");
  assert.equal(mapGet(summary, "repo"), "octo/lessons");
  assert.equal(mapGet(summary, "ref"), "main");
  assert.equal(mapGet(summary, "imported"), 3);

  assert.equal(await evaluate(broker, '(space/exists? "imported")'), true);
  assert.equal(await evaluate(broker, '(fs/read "imported" "/README.md")'), "# Lessons");
  assert.equal(await evaluate(broker, '(fs/read "imported" "/src/intro.hal")'), "(+ 1 2)");
  assert.deepEqual(await evaluate(broker, '(fs/list "imported" "/src")'), ["/src/advanced.hal", "/src/intro.hal"]);
});

test("studio.space import-github! defaults the ref to main", { skip: wasmBytes === null }, async () => {
  const broker = makeBroker();
  const summary = await evaluate(broker, '(space/import-github! "imported-default" "octo/lessons")');
  assert.equal(mapGet(summary, "ref"), "main");
  assert.equal(mapGet(summary, "imported"), 3);
});

test("studio.space import-github! fails the whole import when one file fetch fails", { skip: wasmBytes === null }, async () => {
  const broker = makeBroker({ fetch: mockFetch({ failFor: "/src/advanced.hal" }) });
  await assert.rejects(
    evaluate(broker, '(space/import-github! "imported-broken" "octo/lessons")'),
    /import-github! failed to fetch \/src\/advanced\.hal/
  );
});

test("default bootstrap boots the space in a fresh kernel", { skip: wasmBytes === null }, async () => {
  const broker = makeBroker();
  await broker.create("booted", { bootstrap: defaultBootstrap("boot-space") });

  assert.equal(await broker.eval("booted", '(do (require [studio.space :as space]) (space/exists? "boot-space"))'), true);
  const summary = await broker.eval(
    "booted",
    '(do (require [studio.boot :as boot]) (boot/boot! "boot-space"))'
  );
  assert.equal(mapGet(summary, "space"), "boot-space");
  assert.equal(mapGet(summary, "created"), false); // the bootstrap already created it
  assert.equal(mapGet(summary, "files"), 0);
});

test("boot! creates a missing space and reports the summary", { skip: wasmBytes === null }, async () => {
  const broker = makeBroker();
  const summary = await evaluate(broker, '(boot/boot! "fresh-space")');
  assert.equal(mapGet(summary, "space"), "fresh-space");
  assert.equal(mapGet(summary, "created"), true);
  assert.equal(mapGet(summary, "files"), 0);
  assert.equal(await evaluate(broker, '(space/exists? "fresh-space")'), true);
});

test("a custom bootstrap can build its own store layout without studio.fs", { skip: wasmBytes === null }, async () => {
  const broker = makeBroker();
  const bootstrap = [
    "(do",
    "  (require [studio.store :as store])",
    '  (store/put! "custom/layout/note" "custom-value")',
    '  (store/put! "custom/layout/index" "note"))'
  ].join("\n");
  await broker.create("custom", { bootstrap });

  assert.equal(
    await broker.eval("custom", '(do (require [studio.store :as store]) (store/get "custom/layout/note"))'),
    "custom-value"
  );
  assert.deepEqual(
    await broker.eval("custom", '(do (require [studio.store :as store]) (store/keys "custom/"))'),
    ["custom/layout/index", "custom/layout/note"]
  );
});
