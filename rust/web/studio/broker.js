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

  async close(name) {
    name = KernelBroker.normalizeName(name);
    if (name === ROOT) throw new Error("ROOT_CANNOT_CLOSE");
    const kernel = this.kernels.get(name);
    if (!kernel) throw new Error(`NO_SESSION ${name}`);
    this.kernels.delete(name);
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

/**
 * Production wiring for the website and hara-chrome: a broker whose spawn
 * creates a module Worker plus an `HtaContext`. `hostCalls` is passed through
 * as-is (a shared map, e.g. `createHostServices()` output, possibly merged
 * with extra calls by the caller); `resources` registers into every kernel.
 */
export function createBrowserBroker({ workerUrl, moduleBytes, hostCalls = {}, resources }) {
  return new KernelBroker({
    resources,
    spawn: async (name) => {
      const worker = new Worker(workerUrl, { type: "module", name: `hara-kernel-${name}` });
      const context = new HtaContext({ worker, moduleBytes, hostCalls });
      return { context, worker };
    }
  });
}
