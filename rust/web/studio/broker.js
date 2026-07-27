import { HtaContext } from "../hta.js";

const ROOT = "ROOT";
const NAME_PATTERN = /^[A-Za-z0-9_.-]+$/;

/**
 * Owns studio kernel lifecycle: one kernel = one Web Worker running one raw
 * HTA wasm instance. Mirrors the JVM `HaraSessionBroker`
 * (java/src/main/java/hara/truffle/HaraSessionBroker.java) — same name
 * normalization (reject, never lowercase), same error codes
 * (INVALID_SESSION_NAME, SESSION_EXISTS, NO_SESSION, ROOT_CANNOT_CLOSE), and
 * an always-present ROOT kernel.
 *
 * ROOT is created lazily: the first async access (`eval`/`require`) or sync
 * listing (`list`/`size`) triggers the spawn, and the in-flight promise is
 * cached so concurrent first access cannot double-spawn. A failed ROOT spawn
 * is forgotten, so later access retries. `require` is async (unlike the JVM)
 * because spawning a worker is async; `list`/`size` are sync snapshots that
 * report ROOT once its spawn has been triggered.
 */
export class KernelBroker {
  constructor({ spawn, resources = {} }) {
    this.spawn = spawn;
    this.resources = resources;
    this.kernels = new Map(); // name -> { name, context, worker }
    this.pending = new Map(); // name -> in-flight create promise
    this.rootStart = null; // in-flight ROOT spawn promise, once triggered
    this.documents = new Map(); // kernel/document -> active private generation
    this.documentGenerations = new Map();
  }

  static normalizeName(value) {
    if (typeof value !== "string" || value.length === 0 || !NAME_PATTERN.test(value)) {
      throw new Error("INVALID_SESSION_NAME");
    }
    return value;
  }

  async create(name, { bootstrap } = {}) {
    name = KernelBroker.normalizeName(name);
    if (name === ROOT) await this.rootKernel(); // settle any in-flight ROOT first
    if (this.kernels.has(name) || this.pending.has(name)) throw new Error(`SESSION_EXISTS ${name}`);
    const boot = this.boot(name, bootstrap);
    this.pending.set(name, boot);
    try {
      const kernel = await boot;
      this.kernels.set(name, kernel);
      return kernel;
    } finally {
      this.pending.delete(name);
    }
  }

  async require(name) {
    if (name === ROOT) return this.rootKernel();
    const kernel = this.kernels.get(name);
    if (!kernel) throw new Error(`NO_SESSION ${name}`);
    return kernel;
  }

  async eval(name, source) {
    const kernel = await this.require(name);
    return kernel.context.call("eval", [source]);
  }

  /**
   * Evaluate an ns+ document in an isolated candidate kernel. Only a
   * successful candidate becomes active; the previous generation is then
   * terminated. Hidden document kernels never appear in list()/size().
   */
  async evalDocument(name, documentId, source, { nodeId = null } = {}) {
    const prepared = await this.prepareDocument(name, documentId, source, { nodeId });
    this.commitDocument(prepared);
    return documentResult(prepared);
  }

  /**
   * Evaluate a candidate anonymous generation without disturbing the active
   * document. Callers can run it until its first visible frame, then commit,
   * or discard it to preserve the previous generation.
   */
  async prepareDocument(name, documentId, source, { nodeId = null } = {}) {
    name = KernelBroker.normalizeName(name);
    if (typeof documentId !== "string" || documentId.length === 0) {
      throw new Error("INVALID_DOCUMENT_ID");
    }
    const compiled = compileAnonymousDocument(source, { documentId, nodeId });
    const key = `${name}\u0000${documentId}`;
    const generation = (this.documentGenerations.get(key) ?? 0) + 1;
    const hiddenName = `DOC.${safeName(name)}.${safeName(documentId)}.${generation}`;
    const candidate = await this.boot(hiddenName);
    try {
      const value = await candidate.context.call("eval", [compiled.source]);
      return {
        ...candidate,
        key,
        kernel: name,
        documentId,
        nodeId,
        generation,
        moduleId: compiled.moduleId,
        value,
        prepared: true
      };
    } catch (error) {
      candidate.context?.close?.();
      candidate.worker?.terminate?.();
      throw error;
    }
  }

  commitDocument(candidate) {
    if (!candidate?.prepared) throw new Error("INVALID_DOCUMENT_CANDIDATE");
    const previous = this.documents.get(candidate.key);
    candidate.prepared = false;
    this.documents.set(candidate.key, candidate);
    this.documentGenerations.set(candidate.key, candidate.generation);
    previous?.context?.close?.();
    previous?.worker?.terminate?.();
    return documentResult(candidate);
  }

  discardDocument(candidate) {
    if (!candidate?.prepared) return false;
    candidate.prepared = false;
    candidate.context?.close?.();
    candidate.worker?.terminate?.();
    return true;
  }

  async evalForm(name, documentId, source) {
    const document = this.requireDocument(name, documentId);
    return document.context.call("eval", [expandBuiltinAliases(source)]);
  }

  async evalPreparedDocument(candidate, source) {
    if (!candidate?.prepared) throw new Error("INVALID_DOCUMENT_CANDIDATE");
    return candidate.context.call("eval", [expandBuiltinAliases(source)]);
  }

  releaseDocument(name, documentId) {
    const key = `${name}\u0000${documentId}`;
    const document = this.documents.get(key);
    if (!document) return false;
    this.documents.delete(key);
    document.context?.close?.();
    document.worker?.terminate?.();
    return true;
  }

  requireDocument(name, documentId) {
    const document = this.documents.get(`${name}\u0000${documentId}`);
    if (!document) throw new Error(`NO_DOCUMENT ${documentId}`);
    return document;
  }

  async close(name) {
    name = KernelBroker.normalizeName(name);
    if (name === ROOT) throw new Error("ROOT_CANNOT_CLOSE");
    const kernel = this.kernels.get(name);
    if (!kernel) throw new Error(`NO_SESSION ${name}`);
    this.kernels.delete(name);
    for (const document of [...this.documents.values()]) {
      if (document.kernel !== name) continue;
      this.releaseDocument(name, document.documentId);
    }
    kernel.context?.close?.();
    kernel.worker?.terminate?.();
  }

  list() {
    this.rootKernel().catch(() => {}); // trigger; failure surfaces on await
    const names = [...this.kernels.keys()];
    return names.includes(ROOT) ? names : [ROOT, ...names];
  }

  size() {
    return this.list().length;
  }

  rootKernel() {
    const existing = this.kernels.get(ROOT);
    if (existing) return Promise.resolve(existing);
    this.rootStart ??= this.boot(ROOT).then(
      (kernel) => {
        this.kernels.set(ROOT, kernel);
        return kernel;
      },
      (error) => {
        this.rootStart = null; // allow a later access to retry
        throw error;
      }
    );
    return this.rootStart;
  }

  // Spawns, registers resources, then evals the bootstrap source. A boot
  // failure terminates the half-started kernel and leaves nothing stored.
  async boot(name, bootstrap) {
    const { context, worker } = await this.spawn(name);
    try {
      for (const [resourceName, source] of Object.entries(this.resources)) {
        await context.call("register-resource", [resourceName, source]);
      }
      if (bootstrap !== undefined) await context.call("eval", [bootstrap]);
    } catch (error) {
      context?.close?.();
      worker?.terminate?.();
      throw error;
    }
    return { name, context, worker };
  }
}

function documentResult(document) {
  return {
    value: document.value,
    generation: document.generation,
    moduleId: document.moduleId,
    private: true
  };
}

export function compileAnonymousDocument(source, { documentId, nodeId = null } = {}) {
  if (typeof source !== "string") throw new Error("DOCUMENT_SOURCE_MUST_BE_STRING");
  const form = firstEffectiveForm(source);
  if (!form || form.head !== "ns+") throw new Error("NS_PLUS_MUST_BE_FIRST_EFFECTIVE_FORM");
  const generationToken = `${safeName(documentId)}.${Math.random().toString(36).slice(2)}`;
  const moduleId = `anonymous:${generationToken}`;
  const nsSource = `${source.slice(form.start, form.headStart)}ns ${moduleId.replaceAll(":", ".")}${source.slice(form.headEnd, form.end)}`;
  const body = source.slice(form.end);
  const binding = bindNodeSource(`(do ${body})`, nodeId);
  return {
    moduleId,
    source: `${nsSource}\n(require [studio.node :as node])\n${binding}`
  };
}

function bindNodeSource(source, nodeId) {
  return `(do (set! node/*node-id* ${nodeId == null ? "nil" : JSON.stringify(nodeId)}) ${expandBuiltinAliases(source)})`;
}

// Raw HTA kernels implement the canonical coroutine special form directly.
// `co/` is nevertheless a language builtin, so document compilation resolves
// it without requiring or publicly registering a namespace.
function expandBuiltinAliases(source) {
  let output = "";
  let cursor = 0;
  let string = false;
  let comment = false;
  while (cursor < source.length) {
    const character = source[cursor];
    if (comment) {
      output += character;
      cursor += 1;
      if (character === "\n") comment = false;
      continue;
    }
    if (string) {
      output += character;
      cursor += 1;
      if (character === "\\") {
        output += source[cursor] ?? "";
        cursor += 1;
      } else if (character === '"') {
        string = false;
      }
      continue;
    }
    if (character === ";") {
      comment = true;
      output += character;
      cursor += 1;
      continue;
    }
    if (character === '"') {
      string = true;
      output += character;
      cursor += 1;
      continue;
    }
    if (source.startsWith("co/await", cursor) &&
        !/[A-Za-z0-9*+!?._/-]/.test(source[cursor - 1] ?? "") &&
        !/[A-Za-z0-9*+!?._/-]/.test(source[cursor + 8] ?? "")) {
      // co/ is a builtin alias. Lower it to the raw runtime's fiber-aware
      // special form without registering or requiring a public namespace.
      output += "std.foundation.coroutine/await";
      cursor += 8;
      continue;
    }
    output += character;
    cursor += 1;
  }
  return output;
}

function firstEffectiveForm(source) {
  let cursor = 0;
  while (cursor < source.length) {
    while (/\s|,/.test(source[cursor] ?? "")) cursor += 1;
    if (source[cursor] === ";") {
      const newline = source.indexOf("\n", cursor);
      cursor = newline < 0 ? source.length : newline + 1;
      continue;
    }
    if (source.startsWith("#_", cursor)) {
      cursor += 2;
      while (/\s|,/.test(source[cursor] ?? "")) cursor += 1;
      cursor = scanForm(source, cursor);
      continue;
    }
    break;
  }
  if (source[cursor] !== "(") return null;
  const start = cursor;
  cursor += 1;
  while (/\s|,/.test(source[cursor] ?? "")) cursor += 1;
  const headStart = cursor;
  while (cursor < source.length && !/[\s,()[\]{}";]/.test(source[cursor])) cursor += 1;
  const headEnd = cursor;
  return { start, headStart, headEnd, head: source.slice(headStart, headEnd), end: scanForm(source, start) };
}

function scanForm(source, start) {
  let cursor = start;
  const opening = source[cursor];
  if ('"'.includes(opening)) {
    cursor += 1;
    while (cursor < source.length) {
      if (source[cursor] === "\\") cursor += 2;
      else if (source[cursor++] === '"') return cursor;
    }
    throw new Error("UNTERMINATED_DOCUMENT_FORM");
  }
  const pairs = { "(": ")", "[": "]", "{": "}" };
  if (!pairs[opening]) {
    while (cursor < source.length && !/[\s,()[\]{}";]/.test(source[cursor])) cursor += 1;
    return cursor;
  }
  const stack = [pairs[opening]];
  cursor += 1;
  while (cursor < source.length && stack.length) {
    const character = source[cursor];
    if (character === ";") {
      const newline = source.indexOf("\n", cursor);
      cursor = newline < 0 ? source.length : newline + 1;
    } else if (character === '"') {
      cursor = scanForm(source, cursor);
    } else if (pairs[character]) {
      stack.push(pairs[character]);
      cursor += 1;
    } else if (character === stack.at(-1)) {
      stack.pop();
      cursor += 1;
    } else {
      cursor += 1;
    }
  }
  if (stack.length) throw new Error("UNTERMINATED_DOCUMENT_FORM");
  return cursor;
}

function safeName(value) {
  return String(value).replace(/[^A-Za-z0-9_.-]/g, ".");
}

/**
 * Production wiring for the website and hara-chrome: a broker whose spawn
 * creates a module Worker plus an `HtaContext`. `hostCalls` is passed through
 * as-is (a shared map, e.g. `createHostServices()` output, possibly merged
 * with extra calls by the caller); `resources` registers into every kernel.
 */
function sharedWorkerPort(url) {
  const shared = new SharedWorker(url, { type: "module", name: "hara-runtime" });
  const port = shared.port;
  port.start();
  return {
    postMessage(message) { port.postMessage(message); },
    addEventListener(type, listener) { port.addEventListener(type, listener); },
    terminate() { port.close(); }
  };
}

export function createBrowserBroker({ workerUrl, sharedWorkerUrl, moduleBytes, hostCalls = {}, resources }) {
  return new KernelBroker({
    resources,
    spawn: async (name) => {
      const worker = sharedWorkerUrl && typeof SharedWorker !== "undefined"
        ? sharedWorkerPort(sharedWorkerUrl)
        : new Worker(workerUrl, { type: "module", name: `hara-kernel-${name}` });
      const context = new HtaContext({ worker, moduleBytes, hostCalls });
      return { context, worker };
    }
  });
}
