function plain(value) {
  if (value == null || typeof value !== "object") return value;
  if (Array.isArray(value)) return value.map(plain);
  if (value instanceof Set) return new Set([...value].map(plain));
  if (value instanceof Map) {
    return Object.fromEntries([...value].map(([key, entry]) => [keyName(key), plain(entry)]));
  }
  return Object.fromEntries(Object.entries(value).map(([key, entry]) => [key, plain(entry)]));
}

function keyName(value) {
  if (typeof value === "string") return value.startsWith(":") ? value.slice(1) : value;
  if (value?.constructor?.name === "HtaKeyword") return value.name;
  if (value?.constructor?.name === "HtaSymbol") return value.name;
  return String(value);
}

function clone(value) {
  if (globalThis.structuredClone) return structuredClone(value);
  return JSON.parse(JSON.stringify(value));
}

export function canonicalEdn(value, depth = 0) {
  if (value === null || value === undefined) return "nil";
  if (value === true || value === false || typeof value === "number") return String(value);
  if (typeof value === "string") return JSON.stringify(value);
  if (Array.isArray(value)) return `[${value.map((entry) => canonicalEdn(entry, depth + 1)).join(" ")}]`;
  if (value instanceof Set) {
    return `#{${[...value].map((entry) => canonicalEdn(entry, depth + 1)).sort().join(" ")}}`;
  }
  if (typeof value === "object") {
    const entries = value instanceof Map ? [...value] : Object.entries(value);
    entries.sort(([left], [right]) => keyName(left).localeCompare(keyName(right)));
    if (!entries.length) return "{}";
    const indent = " ".repeat((depth + 1) * 2);
    const closing = " ".repeat(depth * 2);
    return `{\n${entries.map(([key, entry]) =>
      `${indent}${ednKey(key)} ${canonicalEdn(entry, depth + 1)}`
    ).join("\n")}\n${closing}}`;
  }
  throw new TypeError(`workspace value is not EDN-serializable: ${typeof value}`);
}

function ednKey(value) {
  const name = keyName(value);
  if (/^[A-Za-z*+!?._/-][A-Za-z0-9*+!?._/-]*$/.test(name)) return `:${name}`;
  return JSON.stringify(name);
}

export async function contentHash(text) {
  const bytes = new TextEncoder().encode(text);
  if (globalThis.crypto?.subtle) {
    const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", bytes));
    return [...digest].map((byte) => byte.toString(16).padStart(2, "0")).join("");
  }
  // FNV-1a is only a deterministic fallback for runtimes without WebCrypto.
  let hash = 0x811c9dc5;
  for (const byte of bytes) {
    hash ^= byte;
    hash = Math.imul(hash, 0x01000193);
  }
  return `fnv1a-${(hash >>> 0).toString(16).padStart(8, "0")}`;
}

export class WorkspaceConflictError extends Error {
  constructor(message, details) {
    super(message);
    this.name = "WorkspaceConflictError";
    this.code = "workspace/conflict";
    this.details = details;
  }
}

export class WorkspaceState extends EventTarget {
  constructor(manifest, { path = "workspace.edn", journal = null, historyLimit = 100 } = {}) {
    super();
    this.path = path;
    this.journal = journal;
    this.historyLimit = historyLimit;
    this.manifest = validateWorkspace(plain(manifest));
    this.history = [];
    this.future = [];
    this.dirty = false;
    this.baseHash = null;
    this.revision = 0;
  }

  static async load({ read, evaluate, root = "." , journal = null }) {
    const projectPath = join(root, "project.edn");
    const workspacePath = join(root, "workspace.edn");
    const projectSource = await read(projectPath);
    const project = validateProject(plain(await evaluate(projectSource, projectPath)));
    const workspaceSource = await read(workspacePath);
    const manifest = plain(await evaluate(workspaceSource, workspacePath));
    const state = new WorkspaceState(manifest, { path: workspacePath, journal });
    state.project = project;
    state.baseHash = await contentHash(workspaceSource);
    const recovery = await journal?.read?.(workspacePath);
    if (recovery && recovery.baseHash === state.baseHash && recovery.revision > 0) {
      state.recovery = recovery;
    }
    return state;
  }

  snapshot() {
    return clone(this.manifest);
  }

  apply(label, mutate) {
    const before = this.snapshot();
    const candidate = this.snapshot();
    mutate(candidate);
    validateWorkspace(candidate);
    this.history.push({ label, value: before });
    if (this.history.length > this.historyLimit) this.history.shift();
    this.future.length = 0;
    this.manifest = candidate;
    this.revision += 1;
    this.markDirty(label);
    return this.snapshot();
  }

  undo() {
    const entry = this.history.pop();
    if (!entry) return false;
    this.future.push({ label: entry.label, value: this.snapshot() });
    this.manifest = entry.value;
    this.revision += 1;
    this.markDirty(`undo:${entry.label}`);
    return true;
  }

  redo() {
    const entry = this.future.pop();
    if (!entry) return false;
    this.history.push({ label: entry.label, value: this.snapshot() });
    this.manifest = entry.value;
    this.revision += 1;
    this.markDirty(`redo:${entry.label}`);
    return true;
  }

  addCanvas({ id, areaId = `area/${id.split("/").pop()}`, title = "Canvas" }) {
    return this.apply("canvas/add", (workspace) => {
      ensureUnique(workspace["workspace/areas"], "area/id", areaId, "area");
      workspace["workspace/areas"].push({
        "area/id": areaId,
        "area/type": "visual-canvas",
        "area/title": title,
        "area/canvas": id
      });
      workspace["workspace/layout"] = {
        "layout/type": "split",
        "layout/direction": "horizontal",
        "layout/ratio": 0.65,
        "layout/first": workspace["workspace/layout"],
        "layout/second": { "layout/type": "area", "layout/area": areaId }
      };
    });
  }

  removeCanvas(id) {
    return this.apply("canvas/remove", (workspace) => {
      const removed = new Set(
        workspace["workspace/areas"]
          .filter((area) => area["area/canvas"] === id)
          .map((area) => area["area/id"])
      );
      workspace["workspace/areas"] = workspace["workspace/areas"]
        .filter((area) => !removed.has(area["area/id"]));
      workspace["workspace/layout"] = pruneLayout(workspace["workspace/layout"], removed) ??
        { "layout/type": "empty" };
      workspace["workspace/links"] = workspace["workspace/links"]
        .filter((link) => !removed.has(link["link/area"]));
    });
  }

  addNode(node) {
    const id = node["node/id"] ?? node.id;
    return this.apply("node/add", (workspace) => {
      ensureUnique(workspace["workspace/nodes"], "node/id", id, "node");
      workspace["workspace/nodes"].push({ ...node, "node/id": id });
    });
  }

  connect(connection) {
    const id = connection["connection/id"] ?? connection.id;
    return this.apply("connection/add", (workspace) => {
      ensureUnique(workspace["workspace/connections"], "connection/id", id, "connection");
      workspace["workspace/connections"].push({ ...connection, "connection/id": id });
    });
  }

  async save({ read, write }) {
    const currentSource = await read(this.path);
    const currentHash = await contentHash(currentSource);
    if (this.baseHash !== null && currentHash !== this.baseHash) {
      throw new WorkspaceConflictError("workspace.edn changed outside Studio", {
        path: this.path,
        expectedHash: this.baseHash,
        actualHash: currentHash
      });
    }
    const source = `${canonicalEdn(this.manifest)}\n`;
    await write(this.path, source);
    this.baseHash = await contentHash(source);
    this.dirty = false;
    await this.journal?.clear?.(this.path);
    this.dispatchEvent(new CustomEvent("save", { detail: { path: this.path, hash: this.baseHash } }));
    return source;
  }

  async recover() {
    if (!this.recovery) return false;
    this.manifest = validateWorkspace(clone(this.recovery.manifest));
    this.revision = this.recovery.revision;
    this.dirty = true;
    this.recovery = null;
    this.dispatchEvent(new CustomEvent("change", { detail: { label: "recovery", revision: this.revision } }));
    return true;
  }

  markDirty(label) {
    this.dirty = true;
    const record = {
      version: 1,
      path: this.path,
      baseHash: this.baseHash,
      revision: this.revision,
      manifest: this.snapshot(),
      updatedAt: new Date().toISOString()
    };
    Promise.resolve(this.journal?.write?.(this.path, record)).catch(() => {});
    this.dispatchEvent(new CustomEvent("change", { detail: { label, revision: this.revision } }));
  }
}

export function createStoreJournal(host, prefix = "workspace/recovery/") {
  return {
    read: (path) => host["store/get"](`${prefix}${path}`),
    write: (path, record) => host["store/put"](`${prefix}${path}`, record),
    clear: (path) => host["store/del"](`${prefix}${path}`)
  };
}

export function validateProject(value) {
  if (value?.["hara/type"] !== "project") throw new Error("project.edn must declare :hara/type :project");
  for (const key of ["hara/version", "project/id", "project/version", "project/source-paths",
    "project/test-paths", "project/extension-paths", "project/capabilities"]) {
    if (value[key] === undefined) throw new Error(`project.edn missing :${key}`);
  }
  return value;
}

export function validateWorkspace(value) {
  if (value?.["hara/type"] !== "workspace") throw new Error("workspace.edn must declare :hara/type :workspace");
  for (const key of ["hara/version", "workspace/id", "workspace/layout", "workspace/documents",
    "workspace/areas", "workspace/nodes", "workspace/connections", "workspace/links",
    "workspace/customizations"]) {
    if (value[key] === undefined) throw new Error(`workspace.edn missing :${key}`);
  }
  for (const key of ["workspace/documents", "workspace/areas", "workspace/nodes",
    "workspace/connections", "workspace/links"]) {
    if (!Array.isArray(value[key])) throw new Error(`:${key} must be a vector`);
  }
  return value;
}

function ensureUnique(items, key, id, label) {
  if (!id) throw new Error(`${label} id is required`);
  if (items.some((item) => item[key] === id)) throw new Error(`${label} already exists: ${id}`);
}

function pruneLayout(layout, removed) {
  if (!layout || typeof layout !== "object") return layout;
  if (layout["layout/type"] === "area") return removed.has(layout["layout/area"]) ? null : layout;
  if (layout["layout/type"] !== "split") return layout;
  const first = pruneLayout(layout["layout/first"], removed);
  const second = pruneLayout(layout["layout/second"], removed);
  if (!first) return second;
  if (!second) return first;
  return { ...layout, "layout/first": first, "layout/second": second };
}

function join(root, name) {
  return root === "." || root === "" ? name : `${root.replace(/\/$/, "")}/${name}`;
}
