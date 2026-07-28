// Generated programs run in this module Worker. Do not add window/document or
// privileged host objects here; all browser access is capability RPC in the
// GraphHost layer.

const programs = new Map();
const nodes = new Map();

self.addEventListener("message", (event) => {
  handle(event.data).catch((error) => replyError(event.data?.id, error));
});

async function handle(message) {
  switch (message?.type) {
    case "install": return reply(message.id, await install(message.program));
    case "spawn": return reply(message.id, await spawn(message.node));
    case "deliver": return reply(message.id, await deliver(message));
    case "call": return reply(message.id, await call(message));
    case "release-node": return reply(message.id, await releaseNode(message.nodeId, message.generation));
    case "release-program": return reply(message.id, await releaseProgram(message.programId, message.generation));
    default: throw structured("program/worker-command", `unknown program worker command: ${message?.type}`);
  }
}

async function install(program) {
  const key = programKey(program.id, program.generation);
  if (programs.has(key)) return { status: "cached", programId: program.id, generation: program.generation };
  if (program.language !== "javascript/module") {
    throw structured("program/language", `worker cannot install ${program.language}`);
  }
  const blob = new Blob([program.source], { type: "text/javascript" });
  const url = URL.createObjectURL(blob);
  try {
    const module = await import(url);
    const createNode = module[program.exportName];
    if (typeof createNode !== "function") {
      throw structured("program/export-missing", `program must export ${program.exportName}`);
    }
    programs.set(key, { ...program, createNode });
    return { status: "ready", programId: program.id, generation: program.generation };
  } finally {
    URL.revokeObjectURL(url);
  }
}

async function spawn(descriptor) {
  if (nodes.has(descriptor.nodeId)) throw structured("node/already-exists", `node already exists: ${descriptor.nodeId}`);
  const program = programs.get(programKey(descriptor.programId, descriptor.programGeneration));
  if (!program) throw structured("program/not-found", `unknown program: ${descriptor.programId}`);
  const node = await program.createNode(apiFor(descriptor), descriptor.config);
  if (!node || typeof node !== "object") throw structured("node/invalid", "createNode must return an object");
  for (const method of ["start", "receive", "call", "dispose"]) {
    if (node[method] !== undefined && typeof node[method] !== "function") {
      throw structured("node/invalid", `node ${method} must be a function`);
    }
  }
  const state = { descriptor, node, lane: Promise.resolve(), disposed: false };
  nodes.set(descriptor.nodeId, state);
  await enqueue(state, () => node.start?.());
  return { status: "ready", nodeId: descriptor.nodeId, generation: descriptor.generation };
}

async function deliver({ nodeId, port, frame, generation }) {
  const state = requireNode(nodeId, generation);
  await enqueue(state, () => state.node.receive?.(port, frame));
  return { accepted: true };
}

async function call({ nodeId, action, args, frame, generation }) {
  const state = requireNode(nodeId, generation);
  if (!state.node.call) throw structured("node/action-missing", `node has no action handler: ${action}`);
  return enqueue(state, () => state.node.call(action, args, frame));
}

async function releaseNode(nodeId, generation) {
  const state = nodes.get(nodeId);
  if (!state || state.descriptor.generation !== generation) return false;
  nodes.delete(nodeId);
  state.disposed = true;
  await enqueue(state, () => state.node.dispose?.());
  return true;
}

async function releaseProgram(programId, generation) {
  const key = programKey(programId, generation);
  for (const [nodeId, state] of nodes) {
    if (state.descriptor.programId === programId && state.descriptor.programGeneration === generation) {
      await releaseNode(nodeId, state.descriptor.generation);
    }
  }
  return programs.delete(key);
}

function apiFor(descriptor) {
  return Object.freeze({
    nodeId: descriptor.nodeId,
    sessionId: descriptor.sessionId,
    emit: (signal, data, meta = {}) => postMessage({ type: "emission", nodeId: descriptor.nodeId, sessionId: descriptor.sessionId, signal, data, meta }),
    call: (target, action, args, options = {}) => {
      throw structured("node/call-unavailable", `host call routing is not installed for ${target}/${action}`, { options, args });
    },
    capability: (name) => { throw structured("program/capability-unavailable", `capability unavailable in worker: ${name}`); },
    schedule: (callback, delayMs = 0) => setTimeout(callback, delayMs),
    cancelSchedule: (token) => clearTimeout(token),
    frame: () => { throw structured("program/frame-unavailable", "frame scheduling requires a surface capability"); },
    cancelFrame: () => {},
    log: (level, message, data = null) => postMessage({ type: "log", nodeId: descriptor.nodeId, sessionId: descriptor.sessionId, level, message, data })
  });
}

function enqueue(state, task) {
  const next = state.lane.then(async () => {
    if (state.disposed) throw structured("node/released", `node released: ${state.descriptor.nodeId}`);
    return task();
  });
  state.lane = next.catch(() => {});
  return next;
}

function requireNode(nodeId, generation) {
  const state = nodes.get(nodeId);
  if (!state) throw structured("node/not-found", `unknown node: ${nodeId}`);
  if (state.descriptor.generation !== generation) throw structured("node/stale-generation", `stale node generation: ${nodeId}`);
  return state;
}

function programKey(id, generation) { return `${id}@${generation}`; }
function reply(id, value) { postMessage({ type: "result", id, value }); }
function replyError(id, error) { postMessage({ type: "error", id, error: { code: error?.code ?? "program/worker-error", message: String(error?.message ?? error) } }); }
function structured(code, message, details = {}) { return Object.assign(new Error(message), { code, ...details }); }
