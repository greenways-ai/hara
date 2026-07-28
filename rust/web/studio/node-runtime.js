const VERSION = "substrate.v1";
const BINARY_TAG = "hara.bytes.v1";

export class NodeProtocolError extends Error {
  constructor(code, message, frame = null) {
    super(message);
    this.name = "NodeProtocolError";
    this.code = code;
    this.frame = frame;
  }
}

export function frameId(prefix = "evt") {
  const random = globalThis.crypto?.randomUUID?.() ??
    `${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}`;
  return `${prefix}-${random}`;
}

export function normalizeFrame(value) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new NodeProtocolError("frame/invalid", "substrate frame must be an object");
  }
  const kind = value.kind;
  if (!["request", "response", "stream", "subscribe", "unsubscribe", "cancel", "error"].includes(kind)) {
    throw new NodeProtocolError("frame/kind", `unsupported substrate frame kind: ${String(kind)}`);
  }
  if (typeof value.id !== "string" || value.id.length === 0) {
    throw new NodeProtocolError("frame/id", "substrate frame id must be a non-empty string");
  }
  return {
    version: value.version ?? VERSION,
    kind,
    id: value.id,
    source: value.source ?? null,
    target: value.target ?? null,
    space: value.space ?? null,
    ...(value.action === undefined ? {} : { action: value.action }),
    ...(value.args === undefined ? {} : { args: value.args }),
    ...(value.reply_to === undefined ? {} : { reply_to: value.reply_to }),
    ...(value.status === undefined ? {} : { status: value.status }),
    ...(value.data === undefined ? {} : { data: value.data }),
    ...(value.error === undefined ? {} : { error: value.error }),
    ...(value.signal === undefined ? {} : { signal: value.signal }),
    ...(value.cause === undefined ? {} : { cause: value.cause }),
    meta: value.meta ?? {}
  };
}

export function encodeFrameJson(frame) {
  return JSON.stringify(normalizeFrame(frame), jsonReplacer);
}

export function decodeFrameJson(text) {
  if (typeof text !== "string") {
    throw new NodeProtocolError("json/type", "substrate JSON input must be a string");
  }
  let value;
  try {
    value = JSON.parse(text, jsonReviver);
  } catch (error) {
    throw new NodeProtocolError("json/parse", `invalid substrate JSON: ${error.message}`);
  }
  return normalizeFrame(value);
}

function jsonReplacer(_key, value) {
  if (value instanceof Uint8Array) {
    return {
      "$hara/type": BINARY_TAG,
      encoding: "base64",
      data: bytesToBase64(value)
    };
  }
  if (value instanceof ArrayBuffer) {
    return {
      "$hara/type": BINARY_TAG,
      encoding: "base64",
      data: bytesToBase64(new Uint8Array(value))
    };
  }
  if (ArrayBuffer.isView(value)) {
    return {
      "$hara/type": BINARY_TAG,
      encoding: "base64",
      array: value.constructor.name,
      data: bytesToBase64(new Uint8Array(value.buffer, value.byteOffset, value.byteLength))
    };
  }
  if (value instanceof Map) return Object.fromEntries(value);
  if (value instanceof Set) return [...value];
  return value;
}

function jsonReviver(_key, value) {
  if (value?.["$hara/type"] !== BINARY_TAG) return value;
  if (value.encoding !== "base64" || typeof value.data !== "string") {
    throw new Error("invalid hara byte descriptor");
  }
  const bytes = base64ToBytes(value.data);
  if (!value.array || value.array === "Uint8Array") return bytes;
  const constructors = {
    Int8Array, Uint8ClampedArray, Int16Array, Uint16Array,
    Int32Array, Uint32Array, Float32Array, Float64Array
  };
  const Constructor = constructors[value.array];
  if (!Constructor || bytes.byteLength % Constructor.BYTES_PER_ELEMENT !== 0) return bytes;
  return new Constructor(bytes.buffer);
}

function bytesToBase64(bytes) {
  if (typeof Buffer !== "undefined") {
    return Buffer.from(bytes.buffer, bytes.byteOffset, bytes.byteLength).toString("base64");
  }
  let binary = "";
  for (let index = 0; index < bytes.length; index += 0x8000) {
    binary += String.fromCharCode(...bytes.subarray(index, index + 0x8000));
  }
  return btoa(binary);
}

function base64ToBytes(value) {
  if (typeof Buffer !== "undefined") return new Uint8Array(Buffer.from(value, "base64"));
  const binary = atob(value);
  return Uint8Array.from(binary, (character) => character.charCodeAt(0));
}

class PortQueue {
  constructor({ delivery = "ordered", capacity = 16 } = {}) {
    this.delivery = delivery;
    this.capacity = Math.max(1, capacity);
    this.values = [];
    this.waiters = [];
    this.closed = null;
  }

  accept(frame) {
    if (this.closed) throw this.closed;
    const waiter = this.waiters.shift();
    if (waiter) {
      waiter.resolve(frame);
      return { accepted: true, dropped: 0 };
    }
    if (this.delivery === "latest") {
      const dropped = this.values.length;
      this.values.splice(0, this.values.length, frame);
      return { accepted: true, dropped };
    }
    if (this.values.length >= this.capacity) {
      throw new NodeProtocolError("queue/overflow", `node input queue capacity ${this.capacity} exceeded`, frame);
    }
    this.values.push(frame);
    return { accepted: true, dropped: 0 };
  }

  take({ signal } = {}) {
    if (this.values.length) return Promise.resolve(this.values.shift());
    if (this.closed) return Promise.reject(this.closed);
    return new Promise((resolve, reject) => {
      const waiter = { resolve, reject };
      this.waiters.push(waiter);
      if (signal) {
        const cancel = () => {
          const index = this.waiters.indexOf(waiter);
          if (index >= 0) this.waiters.splice(index, 1);
          reject(signal.reason ?? new NodeProtocolError("task/cancelled", "node input cancelled"));
        };
        if (signal.aborted) cancel();
        else signal.addEventListener("abort", cancel, { once: true });
      }
    });
  }

  close(reason) {
    this.closed = reason;
    for (const waiter of this.waiters.splice(0)) waiter.reject(reason);
    this.values.length = 0;
  }
}

/**
 * A Hara-owned reimplementation of the xt.substrate workflows. It deliberately
 * uses no foundation-base code. JSON frames are the canonical portable
 * contract; direct and binary transports carry the same normalized frame.
 */
export class NodeRuntime {
  constructor({ space = "workspace/default", transport = "direct" } = {}) {
    this.space = space;
    this.transport = transport;
    this.nodes = new Map();
    this.connections = new Map();
    this.pending = new Map();
    this.observers = new Set();
    // A kernel context may evaluate a candidate document before its generation
    // is public. Keep its callbacks private until activateDocument commits it.
    this.stagedKernelHandlers = new WeakMap();
  }

  registerNode(descriptor) {
    const id = descriptor?.id ?? descriptor?.["node/id"];
    if (typeof id !== "string" || !id) throw new NodeProtocolError("node/id", "node id is required");
    const existing = this.nodes.get(id);
    if (existing) {
      existing.descriptor = { ...existing.descriptor, ...descriptor, id };
      return existing.publicInfo();
    }
    const state = new NodeState(this, { ...descriptor, id });
    this.nodes.set(id, state);
    return state.publicInfo();
  }

  releaseNode(nodeId) {
    const node = this.nodes.get(nodeId);
    if (!node) return false;
    node.active?.close(new NodeProtocolError("node/released", `node ${nodeId} released`));
    for (const [id, connection] of this.connections) {
      if (connection.from[0] === nodeId || connection.to[0] === nodeId) this.connections.delete(id);
    }
    this.nodes.delete(nodeId);
    return true;
  }

  /**
   * Atomically binds a successful private ns+ document generation to a public
   * node instance. The prior generation is cancelled only after `prepare`
   * succeeds, providing reload rollback.
   */
  async activateDocument(nodeId, { documentId, generation, moduleId, kernelContext, prepare } = {}) {
    const node = this.requireNode(nodeId);
    if (typeof documentId !== "string" || !documentId) {
      throw new NodeProtocolError("document/id", "document id is required");
    }
    const candidate = new GenerationScope(node, {
      documentId,
      generation: generation ?? ((node.active?.generation ?? 0) + 1),
      moduleId: moduleId ?? `anonymous:${documentId}:${generation ?? ((node.active?.generation ?? 0) + 1)}`
    });
    try {
      await prepare?.(candidate.api());
      this.commitKernelHandlers(nodeId, kernelContext);
    } catch (error) {
      this.discardKernelHandlers(kernelContext);
      candidate.close(new NodeProtocolError("generation/rollback", String(error?.message ?? error)));
      throw error;
    }
    const previous = node.active;
    node.active = candidate;
    previous?.close(new NodeProtocolError("generation/reloaded", `document ${documentId} reloaded`));
    return candidate.info();
  }

  releaseDocument(documentId) {
    let released = 0;
    for (const node of this.nodes.values()) {
      if (node.active?.documentId === documentId) {
        node.active.close(new NodeProtocolError("document/released", `document ${documentId} released`));
        node.active = null;
        node.kernelHandlers.clear();
        released += 1;
      }
    }
    return released;
  }

  connect(descriptor) {
    const id = descriptor?.id ?? descriptor?.["connection/id"] ?? frameId("connection");
    const from = descriptor?.from ?? descriptor?.["connection/from"];
    const to = descriptor?.to ?? descriptor?.["connection/to"];
    if (!Array.isArray(from) || from.length !== 2 || !Array.isArray(to) || to.length !== 2) {
      throw new NodeProtocolError("connection/ports", "connection endpoints must be [node signal]");
    }
    this.requireNode(from[0]);
    this.requireNode(to[0]);
    const connection = {
      id,
      from,
      to,
      transport: descriptor.transport ?? descriptor["connection/transport"] ?? "json",
      delivery: descriptor.delivery ?? descriptor["connection/delivery"] ?? "ordered",
      capacity: descriptor.capacity ?? descriptor["connection/capacity"] ?? 16
    };
    this.connections.set(id, connection);
    this.requireNode(to[0]).input(to[1], connection);
    return id;
  }

  disconnect(id) {
    return this.connections.delete(id);
  }

  handle(nodeId, action, handler, meta = {}) {
    if (typeof handler !== "function") throw new NodeProtocolError("handler/type", "node handler must be a function");
    const node = this.requireNode(nodeId);
    node.handlers.set(action, { handler, meta });
    return () => node.handlers.delete(action);
  }

  stageKernelHandler(context, nodeId, action, handler, meta = {}) {
    if (!context) throw new NodeProtocolError("handler/context", "kernel handler requires a document context");
    if (typeof handler !== "function") throw new NodeProtocolError("handler/type", "node handler must be a function");
    this.requireNode(nodeId);
    let staged = this.stagedKernelHandlers.get(context);
    if (!staged) {
      staged = new Map();
      this.stagedKernelHandlers.set(context, staged);
    }
    let handlers = staged.get(nodeId);
    if (!handlers) {
      handlers = new Map();
      staged.set(nodeId, handlers);
    }
    handlers.set(action, { handler, meta });
    return () => handlers.delete(action);
  }

  commitKernelHandlers(nodeId, context) {
    const node = this.requireNode(nodeId);
    const staged = context ? this.stagedKernelHandlers.get(context)?.get(nodeId) : null;
    node.kernelHandlers = new Map(staged ?? []);
    this.discardKernelHandlers(context);
  }

  discardKernelHandlers(context) {
    if (context) this.stagedKernelHandlers.delete(context);
  }

  async call(source, target, action, args = [], opts = {}) {
    return this.callFrame(source, {
      version: VERSION,
      kind: "request",
      id: opts.id ?? frameId("req"),
      target,
      space: opts.space ?? this.space,
      action,
      args,
      cause: opts.cause ?? null,
      meta: opts.meta ?? {}
    });
  }

  /**
   * Browser adapter for a request constructed by `std.substrate.frame`.
   * The portable layer owns the frame shape; Studio supplies only its local
   * source and workspace defaults before dispatching it to a document node.
   */
  async callFrame(source, frame) {
    const request = normalizeFrame({
      ...frame,
      version: frame?.version ?? VERSION,
      source: frame?.source ?? source,
      space: frame?.space ?? this.space,
      meta: frame?.meta ?? {}
    });
    if (request.kind !== "request") {
      throw new NodeProtocolError("frame/kind", "node/call-frame expects a request frame", request);
    }
    const opts = request.meta ?? {};
    const controller = new AbortController();
    const pending = { request, controller };
    this.pending.set(request.id, pending);
    let timer = null;
    if (opts.timeout > 0) {
      timer = setTimeout(() => controller.abort(new NodeProtocolError("call/timeout", `node call timed out: ${request.action}`, request)), opts.timeout);
    }
    try {
      this.publish(request);
      const targetNode = this.requireNode(request.target);
      const entry = targetNode.kernelHandlers.get(request.action) ?? targetNode.handlers.get(request.action);
      if (!entry) throw new NodeProtocolError("handler/missing", `no handler for ${request.target} ${request.action}`, request);
      const data = await abortable(entry.handler(request.args, request, controller.signal), controller.signal);
      const response = normalizeFrame({
        version: VERSION,
        kind: "response",
        id: frameId("res"),
        source: request.target,
        target: request.source,
        space: request.space,
        reply_to: request.id,
        status: "ok",
        data,
        cause: request.id,
        meta: {}
      });
      this.publish(response);
      return response;
    } catch (error) {
      const errorFrame = normalizeFrame({
        version: VERSION,
        kind: "error",
        id: frameId("err"),
        source: request.target,
        target: request.source,
        space: request.space,
        reply_to: request.id,
        status: "error",
        error: {
          code: error?.code ?? "call/error",
          message: String(error?.message ?? error)
        },
        cause: request.id,
        meta: {}
      });
      this.publish(errorFrame);
      throw Object.assign(error instanceof Error ? error : new Error(String(error)), { frame: errorFrame });
    } finally {
      clearTimeout(timer);
      this.pending.delete(request.id);
    }
  }

  cancel(requestId, reason = "cancelled") {
    const pending = this.pending.get(requestId);
    if (!pending) return false;
    pending.controller.abort(new NodeProtocolError("call/cancelled", reason, pending.request));
    this.publish(normalizeFrame({
      version: VERSION,
      kind: "cancel",
      id: frameId("cancel"),
      source: pending.request.source,
      target: pending.request.target,
      space: pending.request.space,
      cause: requestId,
      meta: {}
    }));
    return true;
  }

  async emit(source, signal, data, meta = {}) {
    return this.emitFrame(source, {
      version: VERSION,
      kind: "stream",
      id: frameId("evt"),
      target: null,
      space: this.space,
      signal,
      data,
      cause: meta.cause ?? null,
      meta
    });
  }

  /**
   * Browser adapter for a stream frame constructed by `std.substrate.frame`.
   * Queues and graph edges remain Studio concerns; envelope validation and
   * wire-key normalization stay portable.
   */
  async emitFrame(source, frame) {
    frame = normalizeFrame({
      ...frame,
      version: frame?.version ?? VERSION,
      source: frame?.source ?? source,
      space: frame?.space ?? this.space,
      meta: frame?.meta ?? {}
    });
    if (frame.kind !== "stream") {
      throw new NodeProtocolError("frame/kind", "node/emit-frame expects a stream frame", frame);
    }
    if (typeof frame.signal !== "string" || frame.signal.length === 0) {
      throw new NodeProtocolError("frame/signal", "stream frames require a non-empty signal", frame);
    }
    const deliveries = [];
    for (const edge of this.connections.values()) {
      if (edge.from[0] !== frame.source || edge.from[1] !== frame.signal) continue;
      const target = this.requireNode(edge.to[0]);
      const queue = target.input(edge.to[1], edge);
      deliveries.push({ connection: edge.id, ...queue.accept({ ...frame, target: edge.to[0], signal: edge.to[1] }) });
    }
    this.publish(frame);
    return { accepted: true, deliveries };
  }

  inFrame(nodeId, signal, options = {}) {
    const node = this.requireNode(nodeId);
    return node.input(signal, options).take(options);
  }

  async in(nodeId, signal, options = {}) {
    return (await this.inFrame(nodeId, signal, options)).data;
  }

  info(nodeId) {
    return this.requireNode(nodeId).publicInfo();
  }

  stop(nodeId, task) {
    return this.requireNode(nodeId).active?.stop(task) ?? false;
  }

  subscribe(observer) {
    this.observers.add(observer);
    return () => this.observers.delete(observer);
  }

  publish(frame) {
    for (const observer of this.observers) observer(frame);
  }

  requireNode(id) {
    const node = this.nodes.get(id);
    if (!node) throw new NodeProtocolError("node/missing", `unknown node: ${id}`);
    return node;
  }
}

class NodeState {
  constructor(runtime, descriptor) {
    this.runtime = runtime;
    this.descriptor = descriptor;
    this.handlers = new Map();
    this.kernelHandlers = new Map();
    this.inputs = new Map();
    this.active = null;
  }

  input(signal, options = {}) {
    if (!this.inputs.has(signal)) this.inputs.set(signal, new PortQueue(options));
    return this.inputs.get(signal);
  }

  publicInfo() {
    return {
      id: this.descriptor.id,
      type: this.descriptor.type ?? this.descriptor["node/type"] ?? null,
      ports: this.descriptor.ports ?? this.descriptor["node/ports"] ?? [],
      transport: {
        protocol: VERSION,
        active: this.runtime.transport,
        capabilities: ["direct", "json", "structured-clone", "transferable"]
      },
      generation: this.active?.info() ?? null,
      tasks: this.active?.tasks.size ?? 0
    };
  }
}

class GenerationScope {
  constructor(node, { documentId, generation, moduleId }) {
    this.node = node;
    this.documentId = documentId;
    this.generation = generation;
    this.moduleId = moduleId;
    this.controller = new AbortController();
    this.tasks = new Map();
    this.handlerDisposers = [];
  }

  api() {
    return Object.freeze({
      start: (fn) => this.start(fn),
      in: (signal) => this.node.runtime.in(this.node.descriptor.id, signal, { signal: this.controller.signal }),
      inFrame: (signal) => this.node.runtime.inFrame(this.node.descriptor.id, signal, { signal: this.controller.signal }),
      emit: (signal, value, meta) => this.node.runtime.emit(this.node.descriptor.id, signal, value, meta),
      call: (target, action, args, opts) => this.node.runtime.call(this.node.descriptor.id, target, action, args, opts),
      handle: (action, fn, meta) => {
        const dispose = this.node.runtime.handle(this.node.descriptor.id, action, fn, meta);
        this.handlerDisposers.push(dispose);
        return dispose;
      },
      stop: (task) => this.stop(task),
      info: () => this.node.publicInfo()
    });
  }

  start(fn) {
    if (typeof fn !== "function") throw new NodeProtocolError("task/type", "node/start expects a function");
    const id = frameId("task");
    const controller = new AbortController();
    const task = { id, controller, state: "running", promise: null };
    task.promise = Promise.resolve().then(() => fn(controller.signal)).then(
      (value) => { task.state = "completed"; return value; },
      (error) => { task.state = controller.signal.aborted ? "cancelled" : "failed"; throw error; }
    ).finally(() => this.tasks.delete(id));
    task.promise.catch(() => {});
    this.tasks.set(id, task);
    return Object.freeze({
      id,
      settled: task.promise,
      stop: () => this.stop(id),
      get state() { return task.state; }
    });
  }

  stop(handle) {
    const id = typeof handle === "string" ? handle : handle?.id;
    const task = this.tasks.get(id);
    if (!task) return false;
    task.controller.abort(new NodeProtocolError("task/cancelled", `task ${id} cancelled`));
    return true;
  }

  close(reason) {
    this.controller.abort(reason);
    for (const task of this.tasks.values()) task.controller.abort(reason);
    this.tasks.clear();
    for (const dispose of this.handlerDisposers.splice(0)) dispose();
  }

  info() {
    return {
      documentId: this.documentId,
      generation: this.generation,
      moduleId: this.moduleId,
      private: true,
      tasks: this.tasks.size
    };
  }
}

function abortable(value, signal) {
  return new Promise((resolve, reject) => {
    const onAbort = () => reject(signal.reason ?? new NodeProtocolError("call/cancelled", "cancelled"));
    if (signal.aborted) return onAbort();
    signal.addEventListener("abort", onAbort, { once: true });
    Promise.resolve(value).then(resolve, reject).finally(() => signal.removeEventListener("abort", onAbort));
  });
}

export { VERSION as SUBSTRATE_VERSION };
