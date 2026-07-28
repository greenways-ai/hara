import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";
import "fake-indexeddb/auto";

import { HtaContext, HtaKeyword } from "./hta.js";
import { KernelBroker } from "./studio/broker.js";
import { defaultBootstrap } from "./studio/boot.js";
import { createHostServices } from "./studio/host-services.js";
import { NodeRuntime } from "./studio/node-runtime.js";
import { SessionRouter } from "./studio/session-router.js";

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
      "studio.boot": await hal("boot"),
      "studio.node": await hal("node"),
      "studio.program": await hal("program"),
      "studio.graph": await hal("graph"),
      "studio.session": await hal("session"),
      "std.substrate.protocol": await readFile(new URL("../../lib/src/std/substrate/protocol.hal", import.meta.url), "utf8"),
      "std.substrate.frame": await readFile(new URL("../../lib/src/std/substrate/frame.hal", import.meta.url), "utf8"),
      "std.substrate": await readFile(new URL("../../lib/src/std/substrate.hal", import.meta.url), "utf8")
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
let brokerCounter = 0;
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

function makeBroker({ fetch, nodeRuntime } = {}) {
  const hostCalls = createHostServices({
    dbName: `hara-studio-hal-test-${++brokerCounter}`,
    fetch: fetch ?? mockFetch(),
    nodeRuntime
  });
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

test("studio.node sends std.substrate.frame envelopes through the browser adapter", { skip: wasmBytes === null }, async () => {
  const runtime = new NodeRuntime({ space: "workspace/studio-hal" });
  runtime.registerNode({ id: "node/a" });
  runtime.registerNode({ id: "node/b" });
  runtime.connect({
    id: "a-to-b",
    from: ["node/a", "signal/out"],
    to: ["node/b", "signal/in"]
  });
  runtime.handle("node/b", "double", ([value]) => value * 2);
  const broker = makeBroker({ nodeRuntime: runtime });

  const document = await broker.evalDocument(
    "ROOT",
    "document/substrate-node",
    '(ns+) (node/emit "signal/out" {:answer 42} {:cause "evt-0"})',
    { nodeId: "node/a" }
  );
  const frame = await runtime.inFrame("node/b", "signal/in");
  assert.equal(frame.version, "substrate.v1");
  assert.equal(frame.kind, "stream");
  assert.equal(frame.source, "node/a");
  assert.equal(frame.cause, "evt-0");
  assert.deepEqual(frame.data, { answer: 42 });

  const value = await broker.evalForm(
    "ROOT",
    "document/substrate-node",
    '(node/call "node/b" "double" [21] {:id "req-1" :meta {:trace "studio"}})'
  );
  assert.equal(value, 42);
});

test("studio.program and studio.graph bridge their host-call operations", { skip: wasmBytes === null }, async () => {
  const calls = [];
  const graphHost = {
    programs: { release: async (id) => { calls.push(["program/release", id]); return true; } },
    install: async (descriptor, options) => { calls.push(["program/install", descriptor, options]); return { programId: descriptor["program/id"] }; },
    programInfo: (id) => ({ programId: id }),
    spawn: async (descriptor) => ({ nodeId: descriptor["node/id"] }),
    release: async () => true,
    connect: () => "connection-1",
    disconnect: () => true,
    sendFrame: async (_source, frame) => ({ accepted: true, frame }),
    callFrame: async () => ({ data: 42 }),
    info: (id) => ({ nodeId: id }),
    list: () => []
  };
  const hostCalls = createHostServices({
    dbName: `hara-studio-graph-test-${++brokerCounter}`,
    fetch: mockFetch(),
    graphHost
  });
  const broker = new KernelBroker({ resources, spawn: () => spawnRealKernel(hostCalls) });
  const value = await broker.eval("ROOT", "(do " +
    "(require [studio.program :as program]) " +
    "(require [studio.graph :as graph]) " +
    '(program/install {"program/id" "example/transform"} {"sessionId" "ROOT"}) ' +
    '(graph/send-frame "node/source" {"kind" "stream" "id" "evt-1" "signal" "out" "data" 42}))');
  assert.equal(mapGet(value, "accepted"), true);
  assert.deepEqual(calls[0], ["program/install", { "program/id": "example/transform" }, { sessionId: "ROOT" }]);
});

test("studio.session registers a callback and receives only its subscribed frame", { skip: wasmBytes === null }, async () => {
  const sessions = new SessionRouter();
  const released = [];
  const graphHost = { releaseSession: async (id) => released.push(id) };
  const hostCalls = createHostServices({
    dbName: `hara-studio-session-test-${++brokerCounter}`,
    fetch: mockFetch(),
    graphHost,
    graphHostOptions: { sessionRouter: sessions }
  });
  const broker = new KernelBroker({ resources, spawn: () => spawnRealKernel(hostCalls) });
  const callbackId = await broker.eval("ROOT", "(do " +
    "(require [studio.session :as session]) " +
    '(session/register-ingress! "ROOT") ' +
    '(session/on "ROOT" "selected" (fn [event] (get event "data"))))');
  assert.equal(typeof callbackId, "string");
  const delivered = await sessions.deliver("ROOT", {
    version: "substrate.v1", kind: "stream", id: "evt-selected", signal: "selected", data: 7,
    meta: { "session/callback": callbackId }
  });
  assert.deepEqual(delivered, { accepted: true, delivered: 1 });
  assert.equal(await broker.eval("ROOT", '(session/unregister-ingress! "ROOT")'), true);
  assert.deepEqual(released, ["ROOT"]);
});

test("studio.node registers kernel-owned request handlers", { skip: wasmBytes === null }, async () => {
  const runtime = new NodeRuntime({ space: "workspace/studio-hal" });
  runtime.registerNode({ id: "node/a" });
  runtime.registerNode({ id: "node/b" });
  const broker = makeBroker({ nodeRuntime: runtime });

  const prepared = await broker.prepareDocument(
    "ROOT",
    "document/substrate-handler",
    '(ns+) (node/handle "double" (fn [args] (* 2 (nth args 0))))',
    { nodeId: "node/b" }
  );
  await runtime.activateDocument("node/b", {
    documentId: "document/substrate-handler",
    generation: prepared.generation,
    moduleId: prepared.moduleId,
    kernelContext: prepared.context
  });
  broker.commitDocument(prepared);
  assert.equal(prepared.value, "handler-1");
  assert.equal(await broker.evalForm("ROOT", "document/substrate-handler", '(studio.node/invoke-handler "handler-1" [21] nil)'), 42);
  const response = await runtime.call("node/a", "node/b", "double", [21], { id: "handler-req" });
  assert.equal(response.data, 42);
  assert.equal(response.reply_to, "handler-req");

  const failed = await broker.prepareDocument(
    "ROOT",
    "document/substrate-handler",
    '(ns+) (node/handle "double" (fn [args] (* 3 (nth args 0))))',
    { nodeId: "node/b" }
  );
  await assert.rejects(runtime.activateDocument("node/b", {
    documentId: "document/substrate-handler",
    generation: failed.generation,
    moduleId: failed.moduleId,
    kernelContext: failed.context,
    prepare() { throw new Error("candidate failed"); }
  }), /candidate failed/);
  broker.discardDocument(failed);
  assert.equal((await runtime.call("node/a", "node/b", "double", [21])).data, 42);
});

test("Studio kernels load the atom-backed std.substrate node", { skip: wasmBytes === null }, async () => {
  const broker = makeBroker();
  const value = await broker.eval(
    "ROOT",
    "(do " +
      "(require [std.substrate :as substrate]) " +
      "(require [std.substrate.protocol :as protocol]) " +
      '(def node (substrate/node-create "node/studio")) ' +
      '(protocol-call protocol/IService set-service node "answer" 42) ' +
      '(protocol-call protocol/IService get-service node "answer"))'
  );
  assert.equal(value, 42);
});

test("Studio kernels run the atom-backed substrate request stream and cancellation lifecycle", { skip: wasmBytes === null }, async () => {
  const broker = makeBroker();
  const fixture = await readFile(
    new URL("../../lib/test-fixtures/std/substrate/node_lifecycle_conformance.hal", import.meta.url),
    "utf8"
  );
  assert.deepEqual(await broker.eval("ROOT", fixture), [84, 42, new HtaKeyword("rejected")]);
});

test("Studio runs the shared substrate protocol fixture", { skip: wasmBytes === null }, async () => {
  const broker = makeBroker();
  const protocolFixture = await readFile(
    new URL("../../lib/test-fixtures/std/substrate/protocol_conformance.hal", import.meta.url),
    "utf8"
  );
  assert.deepEqual(await broker.eval("ROOT", protocolFixture), [40, 42]);
});

test("Studio runs the shared substrate frame fixture", { skip: wasmBytes === null }, async () => {
  const broker = makeBroker();
  const frameFixture = await readFile(
    new URL("../../lib/test-fixtures/std/substrate/frame_conformance.hal", import.meta.url),
    "utf8"
  );
  assert.equal(
    await broker.eval("ROOT", frameFixture),
    '{"version":"substrate.v1","kind":"request","id":"req-1","source":"client/a","target":"server/b","space":"workspace/main","meta":{"trace":"trace-1"},"action":"math/add","args":[19,23],"reply_to":null,"status":null,"data":null,"error":null,"signal":null,"cause":null}'
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
