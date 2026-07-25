import { test } from "node:test";
import assert from "node:assert/strict";
import { createHostCalls, mergeHostCalls } from "../src/host-bridge.js";
import { createHostServices } from "../vendor/studio/host-services.js";

function echoPort(sent) {
  const listeners = [];
  return {
    onMessage: { addListener: (fn) => listeners.push(fn) },
    onDisconnect: { addListener: () => {} },
    postMessage: (msg) => {
      sent.push(msg);
      queueMicrotask(() =>
        listeners.forEach((fn) => fn({ id: msg.id, ok: true, value: msg.args[0] ?? null })),
      );
    },
  };
}

test("mergeHostCalls exposes the studio services as own keys", () => {
  const services = createHostServices();
  const merged = mergeHostCalls(services, createHostCalls(echoPort([])));
  assert.deepEqual(Object.keys(merged).sort(), Object.keys(services).sort());
  for (const key of ["store/get", "store/put", "store/del", "store/keys", "http/get", "json/parse"]) {
    assert.equal(typeof merged[key], "function", key);
  }
});

test("studio service keys never collide with the chrome/hara port services", () => {
  for (const key of Object.keys(createHostServices())) {
    const service = key.slice(0, key.lastIndexOf("/"));
    assert.ok(!service.startsWith("chrome.") && service !== "hara", key);
  }
});

test("merged host calls: studio answered in-panel, chrome routed over the port", async () => {
  const sent = [];
  const merged = mergeHostCalls(createHostServices(), createHostCalls(echoPort(sent)));

  // json/parse is a studio service: resolved locally, no port traffic.
  const parsed = await merged["json/parse"]('{"a": 1}');
  assert.ok(parsed instanceof Map);
  assert.equal(parsed.get("a"), 1);
  assert.equal(sent.length, 0);

  // hara/echo is not a studio key: falls through to the port proxy.
  const echoed = await merged["hara/echo"](42);
  assert.equal(sent.length, 1);
  assert.equal(sent[0].service, "hara");
  assert.equal(sent[0].method, "echo");
  assert.equal(echoed, 42);

  // Arbitrary chrome.* services route the same way.
  await merged["chrome.tabs/query"]({});
  assert.equal(sent[1].service, "chrome.tabs");
  assert.equal(sent[1].method, "query");
});
