import { NodeRuntime, normalizeFrame } from "./node-runtime.js";
import { CapabilityRegistry } from "./capability-registry.js";
import { ProgramHost, ProgramWorkerExecutor } from "./program-host.js";
import { ProgramError } from "./module-codec.js";

/**
 * First active graph slice: ProgramHost owns executable JS nodes while
 * NodeRuntime retains substrate envelopes, fan-out, ordering and latest-value
 * policy. Session targets are intentionally reserved for SessionRouter.
 */
export class GraphHost {
  constructor({
    executor = null, workerUrl = null, nodeRuntime = null, sessionRouter = null,
    capabilityRegistry = new CapabilityRegistry(), diagnostics = () => {}
  } = {}) {
    this.diagnostics = diagnostics;
    const activeExecutor = executor ?? new ProgramWorkerExecutor({
      workerUrl,
      onEmission: (message) => this.receiveEmission(message),
      onLog: (message) => this.diagnostics({ kind: "program/log", ...message }),
      onCapability: (message) => this.invokeCapability(message),
      onCall: (message) => this.invokeProgramCall(message)
    });
    this.programs = new ProgramHost({ executor: activeExecutor, diagnostics });
    this.sessionRouter = sessionRouter;
    this.capabilities = capabilityRegistry;
    this.runtime = nodeRuntime ?? new NodeRuntime({
      deliver: (delivery) => this.deliver(delivery)
    });
  }

  install(descriptor, options = {}) {
    const sessionId = options.sessionId ?? "ROOT";
    this.capabilities.assert(sessionId, descriptor?.["program/capabilities"] ?? []);
    return this.programs.install(descriptor, {
      ...options, sessionId, capabilities: this.capabilities.forSession(sessionId)
    });
  }
  programInfo(id) { return this.programs.info(id); }
  listPrograms() { return [...this.programs.programs.values()].map((program) => program.info()); }
  availableCapabilities() { return this.capabilities.available(); }

  async spawn(descriptor, options) {
    const sessionId = descriptor?.["node/session"] ?? descriptor?.sessionId;
    const node = await this.programs.spawn(descriptor, {
      ...options, capabilities: this.capabilities.forSession(sessionId)
    });
    this.runtime.registerNode({ id: node.nodeId, type: "generated/javascript", execution: "host" });
    return node;
  }

  registerSessionNode(descriptor) {
    if (!this.sessionRouter) throw new ProgramError("session/ingress-unavailable", "GraphHost has no SessionRouter");
    const id = descriptor?.id ?? descriptor?.["node/id"];
    const sessionId = descriptor?.sessionId ?? descriptor?.["node/session"];
    this.sessionRouter.require(sessionId);
    return this.runtime.registerNode({ ...descriptor, id, sessionId, execution: "session" });
  }

  connect(descriptor) { return this.runtime.connect(descriptor); }
  disconnect(id) { return this.runtime.disconnect(id); }
  info(id) { return this.runtime.info(id); }
  list() { return this.programs.list(); }

  async sendFrame(source, frame) { return this.runtime.emitFrame(source, normalizeFrame(frame)); }
  async callFrame(source, frame) { return this.runtime.callFrame(source, normalizeFrame(frame)); }

  async release(nodeId) {
    this.runtime.releaseNode(nodeId);
    return this.programs.releaseNode(nodeId);
  }

  async releaseSession(sessionId) {
    const hostNodes = this.programs.list({ sessionId }).map((entry) => entry.nodeId);
    const sessionNodes = [...this.runtime.nodes.values()]
      .map((node) => node.publicInfo())
      .filter((node) => node.sessionId === sessionId)
      .map((node) => node.id);
    const ids = new Set([...hostNodes, ...sessionNodes]);
    for (const nodeId of ids) this.runtime.releaseNode(nodeId);
    await this.programs.releaseSession(sessionId);
    // Grants are process-local authority, not persisted workspace state. A
    // closed document/kernel must never leave them available to a future
    // session reusing the same public id.
    this.capabilities.revokeSession(sessionId);
    return ids.size;
  }

  async deliver({ targetNode, port, frame, connection }) {
    if (targetNode.execution === "host") {
      return this.programs.deliver(targetNode.id, port, frame, {
        delivery: connection.delivery,
        capacity: connection.capacity
      });
    }
    if (targetNode.execution === "session") {
      return this.sessionRouter?.deliver(targetNode.sessionId, frame) ??
        Promise.reject(new ProgramError("session/ingress-unavailable", "GraphHost has no SessionRouter"));
    }
    throw new ProgramError("session/ingress-unavailable", `active delivery is not implemented for ${targetNode.execution}`, {
      nodeId: targetNode.id,
      connectionId: connection.id,
      cause: frame.id
    });
  }

  async receiveEmission({ nodeId, signal, data, meta = {} }) {
    return this.runtime.emit(nodeId, signal, data, meta);
  }

  async invokeCapability({ nodeId, sessionId, capability, method, args = [] }) {
    const node = this.programs.requireNode(nodeId);
    if (node.sessionId !== sessionId) {
      throw new ProgramError("program/session-mismatch", `node ${nodeId} is not owned by ${sessionId}`, { nodeId, sessionId });
    }
    return this.capabilities.invokeForNode(sessionId, nodeId, capability, method, ...args);
  }

  async invokeProgramCall({ nodeId, sessionId, target, action, args = [], options = {} }) {
    const node = this.programs.requireNode(nodeId);
    if (node.sessionId !== sessionId) {
      throw new ProgramError("program/session-mismatch", `node ${nodeId} is not owned by ${sessionId}`, { nodeId, sessionId });
    }
    const targetNode = this.runtime.requireNode(String(target));
    if (targetNode.execution === "host") {
      return this.programs.call(String(target), String(action), args, null);
    }
    const response = await this.runtime.call(nodeId, String(target), String(action), args, options);
    return response.data;
  }
}
