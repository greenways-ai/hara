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
  const scopeForContext = options.scopeForContext ?? null;
  const memoryFilesystems = options.memoryFilesystems ?? new Map();
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

  function scopedKey(invocation, key, { keys = false } = {}) {
    const filesystem = invocation?.context?.filesystemForSession?.(
      invocation?.sessionId ?? "ROOT"
    );
    if (filesystem) {
      if (typeof key !== "string" && !(keys && (key === undefined || key === null))) {
        throw new Error("file/path-invalid");
      }
      return { filesystem, key: key ?? "" };
    }
    if (!scopeForContext) return key;
    const space = scopeForContext(invocation?.context);
    if (!space) throw new Error("store/workspace-scope-unavailable");
    const prefix = `spaces/${space}/`;
    if (keys && (key === undefined || key === null)) return prefix;
    if (typeof key !== "string" || !key.startsWith(prefix)) {
      throw new Error(`store/workspace-scope-denied:${space}`);
    }
    return key;
  }

  function mount(invocation, key, options = {}) {
    const scoped = scopedKey(invocation, key, options);
    return scoped && typeof scoped === "object" && "filesystem" in scoped ? scoped : null;
  }

  function memory(filesystem) {
    let entries = memoryFilesystems.get(filesystem);
    if (!entries) memoryFilesystems.set(filesystem, entries = new Map());
    return entries;
  }

  function persistentKey(filesystem, key) {
    return `filesystems/${encodeURIComponent(filesystem)}/${key}`;
  }

  async function mountedGet({ filesystem, key }) {
    if (filesystem.startsWith("memory:")) return memory(filesystem).get(key) ?? null;
    return request(await store("readonly"), "get", persistentKey(filesystem, key));
  }

  async function mountedPut({ filesystem, key }, value) {
    if (filesystem.startsWith("memory:")) {
      memory(filesystem).set(key, value);
      return true;
    }
    await request(await store("readwrite"), "put", value, persistentKey(filesystem, key));
    return true;
  }

  async function mountedDelete({ filesystem, key }) {
    if (filesystem.startsWith("memory:")) return memory(filesystem).delete(key);
    await request(await store("readwrite"), "delete", persistentKey(filesystem, key));
    return true;
  }

  async function mountedKeys({ filesystem, key }) {
    if (filesystem.startsWith("memory:")) {
      return [...memory(filesystem).keys()].filter((entry) => entry.startsWith(key));
    }
    const prefix = persistentKey(filesystem, key);
    const root = persistentKey(filesystem, "");
    return (await request(await store("readonly"), "getAllKeys"))
      .filter((entry) => entry.startsWith(prefix))
      .map((entry) => entry.slice(root.length));
  }

  const services = {
    "store/get": async function(key) {
      const mounted = mount(this, key);
      if (mounted) return mountedGet(mounted);
      return request(await store("readonly"), "get", scopedKey(this, key));
    },
    "store/put": async function(key, value) {
      const mounted = mount(this, key);
      if (mounted) return mountedPut(mounted, value);
      key = scopedKey(this, key);
      await request(await store("readwrite"), "put", value, key);
      return true;
    },
    "store/del": async function(key) {
      const mounted = mount(this, key);
      if (mounted) return mountedDelete(mounted);
      key = scopedKey(this, key);
      await request(await store("readwrite"), "delete", key);
      return true;
    },
    "store/keys": async function(prefix) {
      const mounted = mount(this, prefix, { keys: true });
      if (mounted) return mountedKeys(mounted);
      prefix = scopedKey(this, prefix, { keys: true });
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
  if (options.graphHost) Object.assign(services, createGraphHostServices(options.graphHost, options.graphHostOptions));
  if (options.canvasRuntime || options.canvasRuntimeForSession) {
    const canvasFor = (invocation) =>
      options.canvasRuntimeForSession?.(invocation.sessionId ?? "ROOT") ??
      options.canvasRuntime;
    services["studio.canvas/next-frame"] = function(nodeId, canvasId) {
      const runtime = canvasFor(this);
      if (!runtime) throw new Error(`canvas/session-unavailable:${this.sessionId ?? "ROOT"}`);
      return runtime.nextFrame(nodeId, canvasId);
    };
    services["studio.canvas/render"] = function(nodeId, canvasId, frame) {
      const runtime = canvasFor(this);
      if (!runtime) throw new Error(`canvas/session-unavailable:${this.sessionId ?? "ROOT"}`);
      return runtime.render(nodeId, canvasId, frame);
    };
  }
  if (options.audioPipeline) {
    services["studio.audio/configure"] = async (spec) =>
      toHta(await options.audioPipeline.configure(toPlain(spec)));
    services["studio.audio/control"] = async (command, value) =>
      toHta(await options.audioPipeline.control(String(command), toPlain(value)));
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

/** Exact HTA host-call surface for generated programs and active graph nodes.
 * The compatibility session ingress methods are intentionally not registered
 * until SessionRouter owns their permission and lifecycle rules. */
export function createGraphHostServices(graph, options = {}) {
  const sessions = options.sessionRouter ?? graph.sessionRouter ?? null;
  const hostDescription = {
    "host/version": "hara.host.v1",
    "program/runtimes": options.programRuntimes ?? ["javascript/module", "javascript/audio-worklet"],
    capabilities: options.capabilities ?? graph.availableCapabilities?.() ?? [],
    limits: options.limits ?? {
      "program/max-source-bytes": 1048576,
      "graph/max-nodes": 1024,
      "graph/max-connections": 4096
    }
  };
  const services = {
    "program/install": async (descriptor, installOptions = {}) =>
      toHta(await graph.install(toPlain(descriptor), toPlain(installOptions))),
    "program/info": async (programId) => toHta(graph.programInfo(String(programId))),
    "program/release": async (programId) => graph.programs.release(String(programId)),
    "graph/spawn": async (descriptor, spawnOptions = {}) =>
      toHta(await graph.spawn(toPlain(descriptor), toPlain(spawnOptions))),
    "graph/release": async (nodeId) => graph.release(String(nodeId)),
    "graph/connect": async (descriptor) => graph.connect(toPlain(descriptor)),
    "graph/disconnect": async (connectionId) => graph.disconnect(String(connectionId)),
    "graph/send-frame": async (source, frame) => toHta(await graph.sendFrame(String(source), toPlain(frame))),
    "graph/call-frame": async (source, frame) => toHta(await graph.callFrame(String(source), toPlain(frame))),
    "graph/info": async (nodeId) => toHta(graph.info(String(nodeId))),
    "graph/list": async () => toHta(graph.list()),
    "host/describe": async () => toHta(hostDescription),
    "host/capabilities": async () => toHta(hostDescription.capabilities)
  };
  if (sessions) Object.assign(services, createSessionHostServices(graph, sessions));
  return services;
}

/** Compatibility ingress registration is intentionally a separate surface:
 * graph traffic never enters a Hara session unless that session explicitly
 * subscribes. The handler receives the owning HtaContext from HtaContext's
 * host-call invocation binding. */
export function createSessionHostServices(graph, sessions) {
  return {
    "session/register-ingress": function(sessionId, capabilities = []) {
      graph.capabilities?.grant(String(sessionId), toPlain(capabilities));
      return toHta(sessions.register(String(sessionId), this.context, {
        capabilities: toPlain(capabilities),
        onRelease: async (released) => graph.releaseSession(released)
      }));
    },
    "session/unregister-ingress": async (sessionId) => sessions.unregister(String(sessionId)),
    "session/subscribe": async (sessionId, signal, callbackId) =>
      sessions.subscribe(String(sessionId), String(signal), String(callbackId)),
    "session/unsubscribe": async (subscriptionId) => sessions.unsubscribe(String(subscriptionId))
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
