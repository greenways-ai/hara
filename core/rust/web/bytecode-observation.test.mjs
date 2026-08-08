import assert from "node:assert/strict";
import test from "node:test";
import {
  BytecodeObservationRuntime,
  BytecodeObservationWasmTransport,
} from "./host/bytecode-observation.js";

const createFakeExports = (handler) => {
  const memory = new WebAssembly.Memory({ initial: 2 });
  let cursor = 1024;
  const allocations = [];
  const allocate = (length) => {
    const pointer = cursor;
    cursor += Math.max(1, length) + 16;
    allocations.push([pointer, length]);
    return pointer;
  };
  return {
    memory,
    allocations,
    observation_abi_version: () => 1,
    observation_alloc: allocate,
    observation_dealloc() {},
    observation_invoke(pointer, length) {
      const request = JSON.parse(new TextDecoder().decode(
        new Uint8Array(memory.buffer, pointer, length),
      ));
      const response = new TextEncoder().encode(JSON.stringify(handler(request)));
      const responsePointer = allocate(response.byteLength);
      new Uint8Array(memory.buffer, responsePointer, response.byteLength).set(response);
      return (BigInt(responsePointer) << 32n) | BigInt(response.byteLength);
    },
  };
};

test("plain-C observation transport copies JSON responses and surfaces stable errors", () => {
  const exports = createFakeExports((request) => request.op === "fail"
    ? { ok: false, error: { code: "bytecode-observation/example", message: "broken" } }
    : { ok: true, value: { operation: request.op } });
  const transport = new BytecodeObservationWasmTransport(exports);
  assert.deepEqual(transport.invoke({ op: "metrics" }), { operation: "metrics" });
  assert.throws(
    () => transport.invoke({ op: "fail" }),
    (error) => error.code === "bytecode-observation/example" && error.message === "broken",
  );
  assert.ok(exports.allocations.length >= 4);
});

test("runtime owns opaque sessions and keeps evidence as plain serializable documents", () => {
  const requests = [];
  let status = "ready";
  let trace = 0;
  const invoke = (request) => {
    requests.push(request);
    switch (request.op) {
      case "compile":
        return {
          handle: 1,
          sessionId: request.sessionId,
          sourceId: request.sourceId,
          traceId: "lesson/trace-0",
          status,
          sequence: trace,
        };
      case "info":
        return {
          handle: 1,
          sessionId: "lesson",
          sourceId: "example/core.hal",
          traceId: "lesson/trace-0",
          status,
          sequence: trace,
        };
      case "step":
        status = "running";
        trace += 1;
        return { schema: "hal.bytecode-trace/v1", steps: [{ sequence: trace }] };
      case "run":
        status = "returned";
        trace += 6;
        return { schema: "hal.bytecode-trace/v1", steps: [{ sequence: trace }] };
      case "metrics":
        return { schema: "hal.bytecode-metrics/v1", instructions: 7 };
      case "events":
        return { schema: "hal.bytecode-events/v1", events: [] };
      case "result-display":
        return "7";
      case "dispose":
        return true;
      case "dispose-all":
        return 0;
      default:
        return true;
    }
  };

  const runtime = new BytecodeObservationRuntime({ invoke });
  const session = runtime.compileNamed("lesson", "example/core.hal", "(+ 1 (* 2 3))");
  assert.equal(session.status, "ready");
  assert.equal(session.step().schema, "hal.bytecode-trace/v1");
  assert.equal(session.status, "running");
  assert.equal(session.run(100).schema, "hal.bytecode-trace/v1");
  assert.equal(session.status, "returned");
  assert.deepEqual(session.metrics(), {
    schema: "hal.bytecode-metrics/v1",
    instructions: 7,
  });
  assert.deepEqual(session.events(), {
    schema: "hal.bytecode-events/v1",
    events: [],
  });
  assert.equal(session.resultDisplay(), "7");
  assert.equal(session.dispose(), true);
  assert.throws(() => session.metrics(), /disposed/);
  assert.equal(requests.some((request) => request.handle === 1), true);
  assert.equal(runtime.dispose(), true);
});

test("settlement and limits remain JSON-safe scalar requests", () => {
  const requests = [];
  const invoke = (request) => {
    requests.push(request);
    if (request.op === "compile") {
      return {
        handle: 3,
        sessionId: "suspend",
        sourceId: "suspend.hal",
        traceId: "suspend/trace-0",
        status: "suspended",
        sequence: 4,
      };
    }
    if (request.op === "info" || request.op.startsWith("set-")) {
      return {
        handle: 3,
        sessionId: "suspend",
        sourceId: "suspend.hal",
        traceId: "suspend/trace-0",
        status: "suspended",
        sequence: 4,
      };
    }
    if (request.op === "resume") return { schema: "hal.bytecode-trace/v1", steps: [] };
    if (request.op === "dispose-all") return 1;
    return true;
  };
  const runtime = new BytecodeObservationRuntime({ invoke });
  const session = runtime.compileNamed("suspend", "suspend.hal", "(await value)");
  session.setObservationLimits({ stack: 12, locals: 10, calls: 8, handlers: 4, displayChars: 256 });
  session.setRetentionLimits({ events: 24, trace: 12 });
  session.resume({ status: "fulfilled", value: { answer: 42 } });
  assert.deepEqual(
    requests.find((request) => request.op === "resume").settlement,
    { status: "fulfilled", value: { answer: 42 } },
  );
  runtime.dispose();
});
