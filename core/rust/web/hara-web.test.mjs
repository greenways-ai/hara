import assert from "node:assert/strict";
import test from "node:test";
import { createHaraWebHost } from "./hara-web.js";

test("hara-web exposes capability discovery and broker lifecycle without a Studio UI", async () => {
  const calls = [];
  const broker = {
    create: async (...args) => { calls.push(["create", ...args]); return { name: args[0] }; },
    require: async () => ({ name: "ROOT" }),
    close: async () => true,
    list: () => ["ROOT"],
    eval: async () => 42,
    createSession: async () => ({ name: "preview" }),
    closeSession: async () => true,
    listSessions: async () => ["ROOT"],
    evalSession: async () => 23
  };
  const host = createHaraWebHost({
    hostOptions: { capabilities: ["clock/frame"] },
    createBroker: (options) => {
      assert.equal(typeof options.hostCalls["host/describe"], "function");
      return broker;
    }
  });
  assert.equal((await host.describe())["host/version"], "hara.host.v1");
  assert.equal(await host.capability("filesystem"), true);
  assert.equal(await host.capability(":clock/frame"), true);
  assert.deepEqual(host.kernels.list(), ["ROOT"]);
  assert.deepEqual(await host.kernels.create("scratch"), { name: "scratch" });
  assert.deepEqual(calls, [["create", "scratch"]]);
});
