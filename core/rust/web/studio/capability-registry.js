import { ProgramError } from "./module-codec.js";

/**
 * Single authority for host capability discovery and per-session grants.
 * Adapters are deliberately registered separately from generated-program
 * execution: this lets a browser report only facilities that actually exist
 * while the ProgramWorker remains isolated from window and document.
 */
export class CapabilityRegistry {
  constructor({ capabilities = [], adapters = {} } = {}) {
    this.adapters = new Map();
    this.grants = new Map();
    for (const capability of capabilities) this.register(capability);
    for (const [capability, adapter] of Object.entries(adapters)) this.register(capability, adapter);
  }

  register(capability, adapter = null) {
    capability = normalize(capability);
    this.adapters.set(capability, adapter);
    return capability;
  }

  available() { return [...this.adapters.keys()].sort(); }
  has(capability) { return this.adapters.has(normalize(capability)); }

  grant(sessionId, capabilities) {
    const requested = normalizeAll(capabilities);
    for (const capability of requested) this.requireAvailable(capability);
    let grants = this.grants.get(sessionId);
    if (!grants) this.grants.set(sessionId, grants = new Set());
    for (const capability of requested) grants.add(capability);
    return this.forSession(sessionId);
  }

  revokeSession(sessionId) { return this.grants.delete(sessionId); }
  forSession(sessionId) { return [...(this.grants.get(sessionId) ?? [])].sort(); }

  assert(sessionId, capabilities) {
    const granted = this.grants.get(sessionId) ?? new Set();
    for (const capability of normalizeAll(capabilities)) {
      this.requireAvailable(capability);
      if (!granted.has(capability)) {
        throw new ProgramError("program/capability-denied", `capability denied: ${capability}`, { sessionId, capability });
      }
    }
  }

  async invoke(sessionId, capability, method, ...args) {
    capability = normalize(capability);
    this.assert(sessionId, [capability]);
    const adapter = this.adapters.get(capability);
    if (!adapter || typeof adapter[method] !== "function") {
      throw new ProgramError("capability/method", `capability ${capability} does not implement ${method}`, { capability, method });
    }
    return adapter[method](...args);
  }

  async invokeForNode(sessionId, nodeId, capability, method, ...args) {
    capability = normalize(capability);
    this.assert(sessionId, [capability]);
    const adapter = this.adapters.get(capability);
    if (!adapter) {
      throw new ProgramError("capability/method", `capability ${capability} has no adapter`, { capability, method });
    }
    const scoped = typeof adapter.forNode === "function"
      ? adapter.forNode({ sessionId, nodeId })
      : adapter;
    if (!scoped || typeof scoped[method] !== "function") {
      throw new ProgramError("capability/method", `capability ${capability} does not implement ${method}`, { capability, method });
    }
    return scoped[method](...args);
  }

  requireAvailable(capability) {
    if (!this.adapters.has(capability)) {
      throw new ProgramError("program/capability-unavailable", `capability unavailable: ${capability}`, { capability });
    }
  }
}

function normalize(value) {
  const capability = String(value).replace(/^:/, "");
  if (!capability) throw new ProgramError("program/capability", "capability is required");
  return capability;
}

function normalizeAll(values) {
  if (values == null) return [];
  if (!(Array.isArray(values) || values instanceof Set)) {
    throw new ProgramError("program/capabilities", "capabilities must be a vector or set");
  }
  return [...new Set([...values].map(normalize))].sort();
}
