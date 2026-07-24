import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";

import { HtaContext } from "./hta.js";
import { createBrowserBroker, KernelBroker } from "./studio/broker.js";

// Mock HtaContext-shaped kernel: records calls, echoes evals with the kernel name.
function mockSpawn({ failFor } = {}) {
  const spawned = [];
  const spawn = async (name) => {
    if (failFor === name) throw new Error(`spawn failed for ${name}`);
    const kernel = {
      name,
      context: {
        calls: [],
        closed: false,
        async call(target, args) {
          this.calls.push([target, args]);
          if (target === "eval") return `${name}:${args[0]}`;
          return true;
        },
        close() {
          this.closed = true;
        }
      },
      worker: {
        terminated: false,
        terminate() {
          this.terminated = true;
        }
      }
    };
    spawned.push(kernel);
    return kernel;
  };
  return { spawn, spawned };
}

test("create, list, size, and eval route to the named kernel", async () => {
  const { spawn } = mockSpawn();
  const broker = new KernelBroker({ spawn });
  const alpha = await broker.create("alpha");
  const beta = await broker.create("beta");

  assert.equal(await broker.eval("alpha", "(+ 1 2)"), "alpha:(+ 1 2)");
  assert.equal(await broker.eval("beta", "(* 3 4)"), "beta:(* 3 4)");
  assert.deepEqual(alpha.context.calls, [["eval", ["(+ 1 2)"]]]);
  assert.deepEqual(beta.context.calls, [["eval", ["(* 3 4)"]]]);

  const names = broker.list();
  assert.ok(names.includes("ROOT"));
  assert.ok(names.includes("alpha"));
  assert.ok(names.includes("beta"));
  assert.equal(broker.size(), 3);
});

test("resources are registered before the bootstrap eval", async () => {
  const { spawn } = mockSpawn();
  const broker = new KernelBroker({
    spawn,
    resources: { "lib/a.hal": "(ns lib.a)", "lib/b.hal": "(ns lib.b)" }
  });
  const kernel = await broker.create("booted", { bootstrap: "(require 'lib.a)" });
  assert.deepEqual(kernel.context.calls, [
    ["register-resource", ["lib/a.hal", "(ns lib.a)"]],
    ["register-resource", ["lib/b.hal", "(ns lib.b)"]],
    ["eval", ["(require 'lib.a)"]]
  ]);
});

test("create without bootstrap only registers resources", async () => {
  const { spawn } = mockSpawn();
  const broker = new KernelBroker({ spawn, resources: { "lib/a.hal": "(ns lib.a)" } });
  const kernel = await broker.create("plain");
  assert.deepEqual(kernel.context.calls, [["register-resource", ["lib/a.hal", "(ns lib.a)"]]]);
});

test("duplicate names are rejected, including concurrent creates", async () => {
  const { spawn, spawned } = mockSpawn();
  const broker = new KernelBroker({ spawn });
  await broker.create("alpha");
  await assert.rejects(broker.create("alpha"), /^Error: SESSION_EXISTS alpha$/);

  const [first, second] = await Promise.allSettled([broker.create("race"), broker.create("race")]);
  assert.equal(first.status === "fulfilled" ? second.status : first.status, "rejected");
  const rejection = first.status === "rejected" ? first : second;
  assert.match(rejection.reason.message, /^SESSION_EXISTS race$/);
  assert.equal(spawned.filter((kernel) => kernel.name === "race").length, 1);
});

test("invalid names are rejected like the JVM broker", async () => {
  const { spawn } = mockSpawn();
  const broker = new KernelBroker({ spawn });
  for (const bad of ["", "bad name", "bad/name", "bad:name", null, undefined]) {
    await assert.rejects(broker.create(bad), /^Error: INVALID_SESSION_NAME$/);
    await assert.rejects(broker.close(bad), /^Error: INVALID_SESSION_NAME$/);
  }
  // The JVM broker does not lowercase; valid names pass through unchanged.
  const kernel = await broker.create("Mixed_Case-1.2");
  assert.equal(kernel.name, "Mixed_Case-1.2");
});

test("require returns the kernel record and rejects unknown names", async () => {
  const { spawn } = mockSpawn();
  const broker = new KernelBroker({ spawn });
  const created = await broker.create("alpha");
  assert.equal(await broker.require("alpha"), created);
  await assert.rejects(broker.require("ghost"), /^Error: NO_SESSION ghost$/);
  await assert.rejects(broker.eval("ghost", "(+ 1 2)"), /^Error: NO_SESSION ghost$/);
});

test("close terminates context and worker and removes the kernel", async () => {
  const { spawn } = mockSpawn();
  const broker = new KernelBroker({ spawn });
  const kernel = await broker.create("alpha");
  await broker.close("alpha");

  assert.equal(kernel.context.closed, true);
  assert.equal(kernel.worker.terminated, true);
  assert.ok(!broker.list().includes("alpha"));
  await assert.rejects(broker.require("alpha"), /^Error: NO_SESSION alpha$/);
  await assert.rejects(broker.close("alpha"), /^Error: NO_SESSION alpha$/);
});

test("ROOT exists without explicit creation and cannot be closed or recreated", async () => {
  const { spawn } = mockSpawn();
  const broker = new KernelBroker({ spawn });

  assert.equal(await broker.eval("ROOT", "(+ 1 2)"), "ROOT:(+ 1 2)");
  const root = await broker.require("ROOT");
  assert.equal(root.name, "ROOT");
  assert.ok(broker.list().includes("ROOT"));

  await assert.rejects(broker.close("ROOT"), /^Error: ROOT_CANNOT_CLOSE$/);
  await assert.rejects(broker.create("ROOT"), /^Error: SESSION_EXISTS ROOT$/);
  // A failed close attempt leaves ROOT alive.
  assert.equal(await broker.eval("ROOT", "still-here"), "ROOT:still-here");
});

test("concurrent first access spawns ROOT exactly once", async () => {
  const { spawn, spawned } = mockSpawn();
  const broker = new KernelBroker({ spawn });
  const results = await Promise.all([
    broker.eval("ROOT", "a"),
    broker.require("ROOT"),
    broker.eval("ROOT", "b"),
    broker.eval("ROOT", "c")
  ]);
  assert.equal(results[0], "ROOT:a");
  assert.equal(results[1].name, "ROOT");
  assert.equal(results[2], "ROOT:b");
  assert.equal(results[3], "ROOT:c");
  assert.equal(spawned.filter((kernel) => kernel.name === "ROOT").length, 1);
});

test("a failed ROOT spawn does not poison later access", async () => {
  let attempts = 0;
  const { spawn, spawned } = mockSpawn();
  const broker = new KernelBroker({
    spawn: async (name) => {
      if (name === "ROOT" && attempts++ === 0) throw new Error("transient");
      return spawn(name);
    }
  });
  await assert.rejects(broker.eval("ROOT", "a"), /transient/);
  assert.equal(await broker.eval("ROOT", "b"), "ROOT:b");
  assert.equal(spawned.filter((kernel) => kernel.name === "ROOT").length, 1);
});

test("a failed create terminates the half-booted kernel", async () => {
  const { spawn, spawned } = mockSpawn();
  const broker = new KernelBroker({
    spawn: async (name) => {
      const kernel = await spawn(name);
      kernel.context.call = async function (target, args) {
        this.calls.push([target, args]);
        if (target === "register-resource") throw new Error("bad resource");
        return true;
      };
      return kernel;
    },
    resources: { "lib/broken.hal": "not hara" }
  });
  await assert.rejects(broker.create("alpha"), /bad resource/);
  const kernel = spawned.find((candidate) => candidate.name === "alpha");
  assert.equal(kernel.context.closed, true);
  assert.equal(kernel.worker.terminated, true);
  await assert.rejects(broker.require("alpha"), /^Error: NO_SESSION alpha$/);
});

test("createBrowserBroker wires Worker and HtaContext", async () => {
  const originalWorker = globalThis.Worker;
  const made = [];
  globalThis.Worker = class {
    constructor(url, options) {
      this.url = url;
      this.options = options;
      this.listeners = {};
      made.push(this);
    }
    addEventListener(type, handler) {
      this.listeners[type] = handler;
    }
    postMessage(message) {
      if (message.type === "init") {
        this.initMessage = message;
        queueMicrotask(() => this.listeners.message({ data: { type: "ready" } }));
      }
    }
    terminate() {
      this.terminated = true;
    }
  };
  try {
    const hostCalls = { "store/get": async () => null };
    const moduleBytes = new Uint8Array([1, 2, 3]);
    const broker = createBrowserBroker({ workerUrl: "/kernel.js", moduleBytes, hostCalls });
    const root = await broker.require("ROOT");

    assert.equal(made.length, 1);
    assert.equal(made[0].url, "/kernel.js");
    assert.deepEqual(made[0].options, { type: "module", name: "hara-kernel-ROOT" });
    assert.ok(root.context instanceof HtaContext);
    assert.equal(root.context.hostCalls, hostCalls);
    assert.equal(made[0].initMessage.moduleBytes, moduleBytes);
  } finally {
    globalThis.Worker = originalWorker;
  }
});

// Optional integration: drive rust/web/hta-worker.js (browser worker script) in
// node via a `self` shim, against the real raw wasm. Skipped when the artifact
// has not been built.
const wasmUrl = new URL("../raw/target/wasm32-unknown-unknown/release/hara_wasm_raw.wasm", import.meta.url);
const wasmBytes = await readFile(wasmUrl).catch(() => null);

test("real wasm kernel evals hara source through the broker", { skip: wasmBytes === null }, async () => {
  const bridge = { listeners: {}, selfListeners: {} };
  bridge.self = {
    addEventListener: (type, handler) => {
      bridge.selfListeners[type] = handler;
    },
    postMessage: (data) => bridge.listeners.message?.({ data }),
    close: () => {}
  };
  globalThis.self = bridge.self;
  await import("./hta-worker.js");

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
  const broker = new KernelBroker({
    resources: { "lib/probe.hal": "(ns lib.probe)" },
    spawn: async () => ({ context: new HtaContext({ worker, moduleBytes: wasmBytes }), worker })
  });

  assert.equal(await broker.eval("ROOT", "(+ 1 2)"), 3);
  const root = await broker.require("ROOT");
  assert.equal(await root.context.call("register-resource", ["lib/extra.hal", "(ns lib.extra)"]), true);
  assert.ok(broker.list().includes("ROOT"));
});
