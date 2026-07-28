import { assertCapabilities, normalizeNodeDescriptor, normalizeProgramDescriptor, ProgramError } from "./module-codec.js";

/**
 * Owns generated host programs and their browser-local node instances.
 * Execution is delegated to an executor (normally ProgramWorkerClient), so
 * this class remains the authority for descriptor validation, ownership,
 * generation replacement and lifecycle.
 */
export class ProgramHost {
  constructor({ executor, maxPrograms = 1024, maxNodes = 1024, diagnostics = () => {} } = {}) {
    if (!executor) throw new Error("ProgramHost requires an executor");
    this.executor = executor;
    this.maxPrograms = maxPrograms;
    this.maxNodes = maxNodes;
    this.diagnostics = diagnostics;
    this.programs = new Map();
    this.nodes = new Map();
    this.sessionNodes = new Map();
  }

  async install(descriptor, { sessionId = "ROOT", capabilities = [], maxSourceBytes } = {}) {
    const program = normalizeProgramDescriptor(descriptor, { maxSourceBytes });
    assertCapabilities(program, capabilities);
    const active = this.programs.get(program.id);
    if (active?.hash === program.hash) return active.info("cached");
    if (!active && this.programs.size >= this.maxPrograms) {
      throw new ProgramError("program/limit", `program limit ${this.maxPrograms} reached`);
    }
    const generation = (active?.generation ?? 0) + 1;
    const candidate = { ...program, generation, ownerSession: sessionId };
    try {
      await this.executor.install(candidate);
    } catch (error) {
      throw programError("program/import-failed", error, candidate);
    }
    const entry = new ProgramEntry(candidate);
    this.programs.set(program.id, entry);
    this.emitDiagnostic("program/installed", entry.info(active ? "replaced" : "ready"));
    return entry.info(active ? "replaced" : "ready");
  }

  info(programId) {
    const program = this.programs.get(programId);
    if (!program) throw new ProgramError("program/not-found", `unknown program: ${programId}`);
    return program.info();
  }

  async release(programId) {
    const program = this.programs.get(programId);
    if (!program) return false;
    for (const nodeId of [...program.nodes]) await this.releaseNode(nodeId);
    await this.executor.releaseProgram?.(programId, program.generation);
    this.programs.delete(programId);
    this.emitDiagnostic("program/released", { programId });
    return true;
  }

  async spawn(descriptor, { capabilities = [] } = {}) {
    const node = normalizeNodeDescriptor(descriptor);
    if (this.nodes.has(node.id)) throw new ProgramError("node/already-exists", `node already exists: ${node.id}`);
    if (this.nodes.size >= this.maxNodes) throw new ProgramError("node/limit", `node limit ${this.maxNodes} reached`);
    const program = this.programs.get(node.programId);
    if (!program) throw new ProgramError("program/not-found", `unknown program: ${node.programId}`);
    assertCapabilities(program, capabilities);
    const instance = new NodeEntry({ ...node, programGeneration: program.generation });
    try {
      await this.executor.spawn(instance.descriptor());
    } catch (error) {
      throw programError("node/start-failed", error, instance);
    }
    this.nodes.set(node.id, instance);
    program.nodes.add(node.id);
    let owned = this.sessionNodes.get(node.sessionId);
    if (!owned) this.sessionNodes.set(node.sessionId, owned = new Set());
    owned.add(node.id);
    this.emitDiagnostic("node/spawned", instance.info());
    return instance.info();
  }

  async deliver(nodeId, port, frame) {
    const node = this.requireNode(nodeId);
    try {
      return await this.executor.deliver(nodeId, port, frame, node.generation);
    } catch (error) {
      throw programError("node/receive-failed", error, node, frame);
    }
  }

  async call(nodeId, action, args, frame = null) {
    const node = this.requireNode(nodeId);
    try {
      return await this.executor.call(nodeId, action, args, frame, node.generation);
    } catch (error) {
      throw programError("node/action-missing", error, node, frame);
    }
  }

  async releaseNode(nodeId) {
    const node = this.nodes.get(nodeId);
    if (!node) return false;
    this.nodes.delete(nodeId);
    this.programs.get(node.programId)?.nodes.delete(nodeId);
    const owned = this.sessionNodes.get(node.sessionId);
    owned?.delete(nodeId);
    if (owned?.size === 0) this.sessionNodes.delete(node.sessionId);
    try {
      await this.executor.releaseNode(nodeId, node.generation);
    } finally {
      this.emitDiagnostic("node/released", node.info());
    }
    return true;
  }

  async releaseSession(sessionId) {
    const ids = [...(this.sessionNodes.get(sessionId) ?? [])];
    for (const id of ids) await this.releaseNode(id);
    this.emitDiagnostic("session/released", { sessionId, nodes: ids });
    return ids.length;
  }

  list({ sessionId = null } = {}) {
    const ids = sessionId ? this.sessionNodes.get(sessionId) ?? [] : this.nodes.keys();
    return [...ids].map((id) => this.nodes.get(id)?.info()).filter(Boolean);
  }

  requireNode(nodeId) {
    const node = this.nodes.get(nodeId);
    if (!node) throw new ProgramError("node/not-found", `unknown node: ${nodeId}`);
    return node;
  }

  emitDiagnostic(kind, detail) {
    this.diagnostics({ kind, ...detail });
  }
}

/** Browser executor for ordinary generated JavaScript. It keeps source module
 * loading in a module Worker and exposes only the command protocol used by
 * ProgramHost. Capability RPC is deliberately left to GraphHost in phase 2. */
export class ProgramWorkerExecutor {
  constructor({ workerUrl, WorkerImpl = globalThis.Worker, onEmission = () => {}, onLog = () => {} } = {}) {
    if (!workerUrl || !WorkerImpl) throw new Error("ProgramWorkerExecutor requires workerUrl and Worker");
    this.worker = new WorkerImpl(workerUrl, { type: "module" });
    this.onEmission = onEmission;
    this.onLog = onLog;
    this.nextId = 0;
    this.pending = new Map();
    this.worker.addEventListener("message", (event) => this.receive(event.data));
    this.worker.addEventListener("error", (event) => this.failAll(event.error ?? new Error(event.message)));
  }

  install(program) { return this.command("install", { program }); }
  spawn(node) { return this.command("spawn", { node }); }
  deliver(nodeId, port, frame, generation) { return this.command("deliver", { nodeId, port, frame, generation }); }
  call(nodeId, action, args, frame, generation) { return this.command("call", { nodeId, action, args, frame, generation }); }
  releaseNode(nodeId, generation) { return this.command("release-node", { nodeId, generation }); }
  releaseProgram(programId, generation) { return this.command("release-program", { programId, generation }); }

  close() {
    this.worker.terminate();
    this.failAll(new ProgramError("program/worker-closed", "program worker closed"));
  }

  command(type, payload) {
    const id = `program-command-${++this.nextId}`;
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      this.worker.postMessage({ type, id, ...payload });
    });
  }

  receive(message) {
    if (message?.type === "emission") return this.onEmission(message);
    if (message?.type === "log") return this.onLog(message);
    const pending = this.pending.get(message?.id);
    if (!pending) return;
    this.pending.delete(message.id);
    if (message.type === "error") {
      pending.reject(new ProgramError(message.error?.code ?? "program/worker-error", message.error?.message ?? "program worker error", message.error));
    } else {
      pending.resolve(message.value);
    }
  }

  failAll(error) {
    for (const { reject } of this.pending.values()) reject(error);
    this.pending.clear();
  }
}

class ProgramEntry {
  constructor(program) {
    Object.assign(this, program);
    this.nodes = new Set();
  }

  info(status = "ready") {
    return {
      programRef: `program:${this.id}@${this.generation}`,
      programId: this.id,
      hash: this.hash,
      generation: this.generation,
      status,
      ownerSession: this.ownerSession,
      capabilities: [...this.capabilities]
    };
  }
}

class NodeEntry {
  constructor(node) { Object.assign(this, node); }
  descriptor() {
    return {
      nodeId: this.id,
      sessionId: this.sessionId,
      programId: this.programId,
      generation: this.generation,
      programGeneration: this.programGeneration,
      config: this.config,
      ports: this.ports,
      actions: this.actions
    };
  }
  info() {
    return {
      nodeId: this.id,
      sessionId: this.sessionId,
      programId: this.programId,
      generation: this.generation,
      programGeneration: this.programGeneration
    };
  }
}

function programError(code, error, nodeOrProgram, frame = null) {
  if (error instanceof ProgramError) return error;
  return new ProgramError(code, String(error?.message ?? error), {
    programId: nodeOrProgram.programId ?? nodeOrProgram.id,
    nodeId: nodeOrProgram.nodeId ?? nodeOrProgram.id ?? null,
    sessionId: nodeOrProgram.sessionId ?? nodeOrProgram.ownerSession ?? null,
    cause: frame?.id ?? null
  });
}
