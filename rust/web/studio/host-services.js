const DEFAULT_DATABASE = "hara-studio";
const STORE = "kv";

/**
 * Generic host services for studio kernels: an IndexedDB key/value store and
 * fetch-backed HTTP. Returns a handler map for the `hostCalls` option of
 * `HtaContext`, keyed "service/method"; handlers are async, take plain
 * decoded HTA arguments, and return encodeable values (null -> nil).
 */
export function createHostServices(options = {}) {
  const dbName = options.dbName ?? DEFAULT_DATABASE;
  const fetchImpl = options.fetch ?? globalThis.fetch.bind(globalThis);
  let opening = null;

  function open() {
    opening ??= new Promise((resolve, reject) => {
      const request = indexedDB.open(dbName, 1);
      request.onupgradeneeded = () => request.result.createObjectStore(STORE);
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
    });
    const pending = opening;
    pending.catch(() => { if (opening === pending) opening = null; });
    return pending;
  }

  async function store(mode) {
    const db = await open();
    return db.transaction(STORE, mode).objectStore(STORE);
  }

  const services = {
    "store/get": async (key) => request(await store("readonly"), "get", key),
    "store/put": async (key, value) => {
      await request(await store("readwrite"), "put", value, key);
      return true;
    },
    "store/del": async (key) => {
      await request(await store("readwrite"), "delete", key);
      return true;
    },
    "store/keys": async (prefix) => {
      const keys = await request(await store("readonly"), "getAllKeys");
      return prefix === undefined || prefix === null
        ? keys
        : keys.filter((key) => key.startsWith(prefix));
    },
    "http/get": async (url) => {
      const response = await fetchImpl(url);
      if (!response.ok) throw new Error(`http/get failed with status ${response.status}`);
      return response.text();
    },
    "json/parse": async (text) => fromJson(JSON.parse(text))
  };
  if (options.nodeRuntime) Object.assign(services, createNodeHostServices(options.nodeRuntime));
  if (options.canvasRuntime) {
    services["studio.canvas/next-frame"] = (nodeId, canvasId) =>
      options.canvasRuntime.nextFrame(nodeId, canvasId);
    services["studio.canvas/render"] = (nodeId, canvasId, frame) =>
      options.canvasRuntime.render(nodeId, canvasId, frame);
  }
  if (options.renderCanvas && !options.canvasRuntime) {
    services["studio.canvas/render"] = async (canvas, scene) => {
      await options.renderCanvas(canvas, scene);
      return true;
    };
  }
  return services;
}

export function createNodeHostServices(runtime) {
  return {
    "node/in": async (nodeId, signal) => toHta(await runtime.in(nodeId, signal)),
    "node/in-frame": async (nodeId, signal) => toHta(await runtime.inFrame(nodeId, signal)),
    // Legacy value-oriented calls remain for existing Studio documents. New
    // documents use the frame forms below, which originate in
    // std.substrate.frame before reaching this browser adapter.
    "node/emit": async (nodeId, signal, value, meta) =>
      toHta(await runtime.emit(nodeId, signal, value, toPlain(meta))),
    "node/call": async (nodeId, target, action, args, opts) =>
      toHta((await runtime.call(nodeId, target, action, args, toPlain(opts))).data),
    "node/emit-frame": async (nodeId, frame) =>
      toHta(await runtime.emitFrame(nodeId, toPlain(frame))),
    "node/call-frame": async (nodeId, frame) =>
      toHta((await runtime.callFrame(nodeId, toPlain(frame))).data),
    "node/handle": function(nodeId, action, handlerId, meta) {
      const invocation = this;
      if (typeof handlerId !== "string" || handlerId.length === 0 || !invocation.context) {
        throw new Error("node/handle requires a kernel callback id");
      }
      const source = `(studio.node/invoke-handler ${JSON.stringify(handlerId)} __hta_arg_0 __hta_arg_1)`;
      runtime.stageKernelHandler(invocation.context, nodeId, action, (args, frame) => invocation.context.call(
        "eval-bound",
        [source, [toHta(args), toHta(frame)]]
      ), toPlain(meta));
      return handlerId;
    },
    "node/stop": (nodeId, task) => runtime.stop(nodeId, task),
    "node/info": (nodeId) => toHta(runtime.info(nodeId))
  };
}

// Decoded shape: objects -> Maps with string keys, arrays -> arrays, scalars
// pass through (null -> nil on the hara side). String keys keep host-call
// arguments and store keys free of opaque keyword objects.
function fromJson(value) {
  if (Array.isArray(value)) return value.map(fromJson);
  if (value !== null && typeof value === "object") {
    return new Map(Object.entries(value).map(([key, item]) => [key, fromJson(item)]));
  }
  return value;
}

function toPlain(value) {
  if (value instanceof Map) {
    return Object.fromEntries([...value].map(([key, entry]) => [
      key?.constructor?.name === "HtaKeyword" ? key.name : String(key),
      toPlain(entry)
    ]));
  }
  if (Array.isArray(value)) return value.map(toPlain);
  return value;
}

function toHta(value) {
  if (Array.isArray(value)) return value.map(toHta);
  if (value !== null && typeof value === "object" &&
      !(value instanceof Uint8Array) && !(value instanceof ArrayBuffer) && !ArrayBuffer.isView(value)) {
    return new Map(Object.entries(value).map(([key, entry]) => [key, toHta(entry)]));
  }
  return value;
}

async function request(store, method, ...arguments_) {
  return new Promise((resolve, reject) => {
    const operation = store[method](...arguments_);
    operation.onsuccess = () => resolve(operation.result ?? null);
    operation.onerror = () => reject(operation.error);
  });
}
