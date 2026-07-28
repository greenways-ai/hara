import { NodeRuntime, normalizeFrame } from "./node-runtime.js";
import { ProgramHost, ProgramWorkerExecutor } from "./program-host.js";
import { ProgramError } from "./module-codec.js";

/**
 * First active graph slice: ProgramHost owns executable JS nodes while
 * NodeRuntime retains substrate envelopes, fan-out, ordering and latest-value
 * policy. Session targets are intentionally reserved for SessionRouter.
 */
export class GraphHost {
  constructor({ executor = null, workerUrl = null, nodeRuntime = null, sessionRouter = null, diagnostics = () => {} } = {}) {
    this.diagnostics = diagnostics;
    const activeExecutor = executor ?? new ProgramWorkerExecutor({
      workerUrl,
      onEmission: (message) => this.receiveEmission(message),
      onLog: (message) => this.diagnostics({ kind: "program/log", ...message })
    });
    this.programs = new ProgramHost({ executor: activeExecutor, diagnostics });
    this.sessionRouter = sessionRouter;
    this.runtime = nodeRuntime ?? new NodeRuntime({
      deliver: (delivery) => this.deliver(delivery)
    });
  }

  install(descriptor, options) { return this.programs.install(descriptor, options); }
  programInfo(id) { return this.programs.info(id); }
  listPrograms() { return [...this.programs.programs.values()].map((program) => program.info()); }

  async spawn(descriptor, options) {
    const node = await this.programs.spawn(descriptor, options);
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
    return ids.size;
  }

  async deliver({ targetNode, port, frame, connection }) {
    if (targetNode.execution === "host") {
      return this.programs.deliver(targetNode.id, port, frame);
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
}
