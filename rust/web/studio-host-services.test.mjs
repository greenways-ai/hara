import assert from "node:assert/strict";
import test from "node:test";
import "fake-indexeddb/auto";

import { createGraphHostServices, createHostServices } from "./studio/host-services.js";
import { SessionRouter } from "./studio/session-router.js";

test("store put/get round trips string values", async () => {
  const host = createHostServices({ dbName: "test-round-trip" });
  assert.equal(await host["store/put"]("alpha", "one"), true);
  assert.equal(await host["store/get"]("alpha"), "one");
  assert.equal(await host["store/put"]("alpha", "two"), true);
  assert.equal(await host["store/get"]("alpha"), "two");
});

test("store get of a missing key returns null", async () => {
  const host = createHostServices({ dbName: "test-missing" });
  assert.equal(await host["store/get"]("absent"), null);
});

test("store del removes a key and returns true", async () => {
  const host = createHostServices({ dbName: "test-del" });
  await host["store/put"]("gone", "value");
  assert.equal(await host["store/del"]("gone"), true);
  assert.equal(await host["store/get"]("gone"), null);
});

test("store keys lists all keys without a prefix and filters with one", async () => {
  const host = createHostServices({ dbName: "test-keys" });
  await host["store/put"]("notes/a", "a");
  await host["store/put"]("notes/b", "b");
  await host["store/put"]("scratch", "c");

  const all = await host["store/keys"]();
  assert.deepEqual([...all].sort(), ["notes/a", "notes/b", "scratch"]);
  assert.deepEqual(await host["store/keys"](null), [...all].sort());
  assert.deepEqual(await host["store/keys"]("notes/"), ["notes/a", "notes/b"]);
  assert.deepEqual(await host["store/keys"]("nothing/"), []);
});

test("two instances sharing a db name see the same IndexedDB data", async () => {
  const first = createHostServices({ dbName: "test-shared" });
  const second = createHostServices({ dbName: "test-shared" });
  await first["store/put"]("shared-key", "shared-value");
  assert.equal(await second["store/get"]("shared-key"), "shared-value");
  assert.deepEqual(await second["store/keys"](), ["shared-key"]);
});

test("http/get returns the response body as text", async () => {
  const calls = [];
  const fetch = async (url) => {
    calls.push(url);
    return { ok: true, status: 200, text: async () => "body text" };
  };
  const host = createHostServices({ dbName: "test-http-ok", fetch });
  assert.equal(await host["http/get"]("https://example.test/data"), "body text");
  assert.deepEqual(calls, ["https://example.test/data"]);
});

test("http/get rejects with the status code on HTTP errors", async () => {
  const fetch = async () => ({ ok: false, status: 404, text: async () => "nope" });
  const host = createHostServices({ dbName: "test-http-error", fetch });
  await assert.rejects(host["http/get"]("https://example.test/missing"), /404/);
});

test("json/parse decodes JSON text into maps, arrays, and scalars", async () => {
  const host = createHostServices({ dbName: "test-json" });
  const value = await host["json/parse"](
    '{"name":"x","count":2,"flag":true,"missing":null,"files":[{"path":"/a.hal"}]}'
  );
  assert.ok(value instanceof Map);
  assert.equal(value.get("name"), "x");
  assert.equal(value.get("count"), 2);
  assert.equal(value.get("flag"), true);
  assert.equal(value.get("missing"), null);
  assert.deepEqual(value.get("files"), [new Map([["path", "/a.hal"]])]);
});

test("json/parse rejects invalid JSON", async () => {
  const host = createHostServices({ dbName: "test-json-bad" });
  await assert.rejects(host["json/parse"]("{nope"));
});

test("graph host services expose exact generated-program host-call keys", async () => {
  const calls = [];
  const graph = {
    programs: { release: async (id) => { calls.push(["program/release", id]); return true; } },
    install: async (descriptor, options) => { calls.push(["program/install", descriptor, options]); return { programId: "example/node" }; },
    programInfo: (id) => ({ programId: id }),
    spawn: async (descriptor) => ({ nodeId: descriptor["node/id"] }),
    release: async () => true,
    connect: () => "connection-1",
    disconnect: () => true,
    sendFrame: async () => ({ accepted: true }),
    callFrame: async () => ({ data: 42 }),
    info: (id) => ({ id }),
    list: () => []
  };
  const services = createGraphHostServices(graph, { capabilities: ["surface/canvas-2d"] });
  const installed = await services["program/install"](
    new Map([["program/id", "example/node"]]),
    new Map([["sessionId", "UI"]])
  );
  assert.ok(installed instanceof Map);
  assert.equal(installed.get("programId"), "example/node");
  assert.deepEqual(calls[0], ["program/install", { "program/id": "example/node" }, { sessionId: "UI" }]);
  const described = await services["host/describe"]();
  assert.equal(described.get("host/version"), "hara.host.v1");
  assert.deepEqual(await services["host/capabilities"](), ["surface/canvas-2d"]);
});

test("session host calls register explicit ingress and release its graph partition", async () => {
  const released = [];
  const sessions = new SessionRouter();
  const graph = { releaseSession: async (id) => released.push(id) };
  const services = createGraphHostServices(graph, { sessionRouter: sessions });
  const context = { call: async () => null };
  const registered = services["session/register-ingress"].call(
    { context }, "UI", ["input/keyboard"]
  );
  assert.ok(registered instanceof Map);
  assert.equal(registered.get("sessionId"), "UI");
  const subscription = await services["session/subscribe"]("UI", "selected", "callback/1");
  assert.equal(typeof subscription, "string");
  assert.equal(await services["session/unsubscribe"](subscription), true);
  assert.equal(await services["session/unregister-ingress"]("UI"), true);
  assert.deepEqual(released, ["UI"]);
});
