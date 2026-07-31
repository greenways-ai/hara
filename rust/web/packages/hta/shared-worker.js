import { decodeHta, encodeHta } from "./index.js";

// One raw HTA instance shared by every same-origin tab. Each MessagePort owns
// its request IDs and host calls, while the task table associates kernel work
// with the port that initiated it.
let instance;
let boot;
const clients = new Map();
const tasks = new Map();

self.onconnect = (event) => {
  const port = event.ports[0];
  clients.set(port, new Map());
  port.addEventListener("message", (message) => receive(port, message.data));
  port.start();
};

async function receive(port, message) {
  try {
    if (message.type === "init") {
      boot ??= instantiate(message);
      await boot;
      port.postMessage({ type: "ready" });
    } else if (message.type === "call") {
      await boot;
      const session = requestSession(message.frame);
      const task = Number(callFrame(instance.exports.hta_start, message.frame));
      clients.get(port)?.set(message.id, task);
      tasks.set(task, { port, id: message.id, session });
      pump();
    } else if (message.type === "delivery") {
      await boot;
      callFrame(instance.exports.hta_deliver, encodeHta([message.call, message.ok ? 0 : 1, decodeHta(message.frame)]));
      pump();
    } else if (message.type === "cancel") {
      const task = clients.get(port)?.get(message.id);
      if (task !== undefined) { instance.exports.hta_cancel(BigInt(task)); pump(); }
    } else if (message.type === "release") {
      await boot;
      callFrame(instance.exports.hta_release, message.frame);
      pump();
    } else if (message.type === "close") {
      clients.delete(port);
      port.close();
    }
  } catch (error) {
    port.postMessage({ type: "fatal", error: { message: String(error?.message ?? error) } });
  }
}

async function instantiate(message) {
  const bytes = message.moduleBytes ?? await (await fetch(message.moduleUrl)).arrayBuffer();
  instance = (await WebAssembly.instantiate(bytes, {})).instance;
  for (const name of ["memory", "hta_abi_version", "hta_alloc", "hta_dealloc", "hta_start", "hta_next_event", "hta_deliver", "hta_cancel", "hta_drop_task", "hta_release"]) {
    if (!(name in instance.exports)) throw new Error(`hta/export-missing: ${name}`);
  }
  if (instance.exports.hta_abi_version() !== 2) throw new Error("hta/version-unsupported");
}

function callFrame(fn, frame) {
  const bytes = frame instanceof Uint8Array ? frame : new Uint8Array(frame);
  const pointer = Number(instance.exports.hta_alloc(bytes.length));
  new Uint8Array(instance.exports.memory.buffer, pointer, bytes.length).set(bytes);
  try { return fn(pointer, bytes.length); } finally { instance.exports.hta_dealloc(pointer, bytes.length); }
}

function next() {
  const packed = instance.exports.hta_next_event();
  if (packed === 0n) return null;
  const pointer = Number(packed >> 32n);
  const size = Number(packed & 0xffff_ffffn);
  const frame = new Uint8Array(instance.exports.memory.buffer, pointer, size).slice();
  instance.exports.hta_dealloc(pointer, size);
  return decodeHta(frame);
}

function requestSession(frame) {
  const [target, args] = decodeHta(frame);
  return typeof target === "string" &&
    (target === "session/eval" || target === "session/eval-bound" || target === "session/trace-eval") &&
    typeof args?.[0] === "string"
    ? args[0]
    : "ROOT";
}

function pump() {
  for (let event; (event = next()) !== null;) {
    const kind = Number(event[0]);
    if (kind === 0 || kind === 1) {
      const task = Number(event[1]);
      const request = tasks.get(task);
      if (!request) continue;
      tasks.delete(task);
      clients.get(request.port)?.delete(request.id);
      instance.exports.hta_drop_task(BigInt(task));
      request.port.postMessage({ type: "result", id: request.id, ok: kind === 0, frame: encodeHta(event[2]) });
    } else if (kind === 2) {
      const request = tasks.get(Number(event[2]));
      if (request) request.port.postMessage({ type: "host-call", call: Number(event[1]), task: Number(event[2]), session: event[3], mount: event[4] ?? null, service: event[5], method: event[6], frame: encodeHta(event[7]) });
    } else throw new Error("hta/event-unknown");
  }
}
