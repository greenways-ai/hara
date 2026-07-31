const GRAPH_ID = "graph/id";

/**
 * Session-owned audio graph service. The HAL graph is authoritative: clients
 * consume snapshots and parameter metadata instead of maintaining a parallel
 * diagram or control schema.
 */
export class SupersonicProvider {
  constructor({ engine = null, storage = null, onSnapshot = null } = {}) {
    this.engine = engine;
    this.storage = storage;
    this.onSnapshot = onSnapshot;
    this.graphs = new Map();
    this.generations = new Map();
  }

  async start(input) {
    const graph = normalizeGraph(input);
    const id = graph[GRAPH_ID];
    const previous = this.graphs.get(id);
    const generation = (this.generations.get(id) ?? 0) + 1;
    const overlay = this.readOverlay(id);
    applyOverlay(graph, overlay);

    // The engine must prepare the complete replacement before it becomes live.
    // A rejected prepare leaves the previous graph untouched.
    const prepared = await this.engine?.prepare?.(graph, previous?.graph);
    try {
      await prepared?.commit?.();
      if (!prepared && this.engine?.start) await this.engine.start(graph, previous?.graph);
    } catch (error) {
      await prepared?.discard?.();
      throw error;
    }

    const state = { graph, generation, revision: 1, status: "running", pending: [] };
    this.graphs.set(id, state);
    this.generations.set(id, generation);
    return this.publish(id);
  }

  async update(graphId, nodeId, parameter, value) {
    const state = requiredGraph(this.graphs, graphId);
    const node = state.graph.nodes.find((candidate) => candidate.id === String(nodeId));
    if (!node) throw new Error(`supersonic/node-not-found:${nodeId}`);
    const control = node.controls.find((candidate) => candidate.parameter === String(parameter));
    if (!control) throw new Error(`supersonic/parameter-not-found:${nodeId}/${parameter}`);
    const normalized = normalizeControlValue(control, value);
    const result = await this.engine?.update?.(state.graph, node, control, normalized);
    node.params[control.parameter] = normalized;
    state.pending = state.pending.filter((entry) =>
      entry.node !== node.id || entry.parameter !== control.parameter);
    if (result?.pending) {
      state.pending.push({
        node: node.id,
        parameter: control.parameter,
        effectiveAt: result.effectiveAt ?? null
      });
    } else state.revision += 1;
    this.writeOverlay(String(graphId), node.id, control.parameter, normalized);
    return this.publish(String(graphId));
  }

  effective(graphId, nodeId, parameter) {
    const state = requiredGraph(this.graphs, graphId);
    state.pending = state.pending.filter((entry) =>
      entry.node !== String(nodeId) ||
      parameter != null && entry.parameter !== String(parameter));
    state.revision += 1;
    return this.publish(String(graphId));
  }

  status(graphId) {
    return this.snapshot(requiredGraph(this.graphs, graphId));
  }

  async stop(graphId) {
    const id = String(graphId);
    const state = requiredGraph(this.graphs, id);
    await this.engine?.stop?.(state.graph);
    state.status = "stopped";
    return this.publish(id);
  }

  snapshot(state) {
    return clone({
      "graph/id": state.graph[GRAPH_ID],
      generation: state.generation,
      "active/revision": state.revision,
      status: state.status,
      pending: state.pending,
      title: state.graph.title,
      nodes: state.graph.nodes,
      connections: state.graph.connections
    });
  }

  publish(graphId) {
    const snapshot = this.status(graphId);
    this.onSnapshot?.(snapshot);
    return snapshot;
  }

  readOverlay(graphId) {
    try {
      return JSON.parse(this.storage?.getItem?.(`supersonic:${graphId}`) ?? "{}");
    } catch {
      return {};
    }
  }

  writeOverlay(graphId, nodeId, parameter, value) {
    if (!this.storage?.setItem) return;
    const overlay = this.readOverlay(graphId);
    overlay[nodeId] ??= {};
    overlay[nodeId][parameter] = value;
    this.storage.setItem(`supersonic:${graphId}`, JSON.stringify(overlay));
  }
}

export function normalizeGraph(input) {
  const value = plain(input);
  if (!value || typeof value !== "object") throw new Error("supersonic/graph-required");
  const id = text(value[GRAPH_ID] ?? value.id);
  if (!id) throw new Error("supersonic/graph-id-required");
  const nodes = array(value.nodes).map(normalizeNode);
  const nodeIds = new Set();
  for (const node of nodes) {
    if (nodeIds.has(node.id)) throw new Error(`supersonic/node-id-duplicate:${node.id}`);
    nodeIds.add(node.id);
  }
  const connections = array(value.connections).map((connection, index) =>
    normalizeConnection(connection, index, nodeIds));
  return {
    [GRAPH_ID]: id,
    title: text(value.title) || id,
    nodes,
    connections
  };
}

function normalizeNode(value, index) {
  const node = plain(value);
  const id = text(node.id);
  const type = text(node.type);
  if (!id) throw new Error(`supersonic/node-id-required:${index}`);
  if (!type) throw new Error(`supersonic/node-type-required:${id}`);
  const params = { ...(plain(node.params) ?? {}) };
  const controls = array(node.controls).map((control) => normalizeControl(control, id, params));
  return {
    id,
    type,
    label: text(node.label) || id,
    summary: text(node.summary),
    input: text(node.input),
    output: text(node.output),
    runtime: text(node.runtime),
    params,
    controls
  };
}

function normalizeControl(value, nodeId, params) {
  const control = plain(value);
  const parameter = text(control.parameter ?? control.id);
  const type = text(control.type) || "number";
  if (!parameter) throw new Error(`supersonic/control-parameter-required:${nodeId}`);
  if (!["number", "boolean", "choice", "steps"].includes(type)) {
    throw new Error(`supersonic/control-type-unsupported:${nodeId}/${parameter}`);
  }
  const normalized = {
    parameter,
    type,
    label: text(control.label) || parameter,
    min: finite(control.min),
    max: finite(control.max),
    step: finite(control.step),
    integer: Boolean(control.integer),
    choices: array(control.choices).map(plain)
  };
  if (!(parameter in params) && "default" in control) params[parameter] = plain(control.default);
  if (parameter in params) params[parameter] = normalizeControlValue(normalized, params[parameter]);
  return normalized;
}

function normalizeConnection(value, index, nodeIds) {
  const connection = plain(value);
  const from = endpoint(connection.from);
  const to = endpoint(connection.to);
  if (!nodeIds.has(from[0]) || !nodeIds.has(to[0])) {
    throw new Error(`supersonic/connection-node-not-found:${index}`);
  }
  return {
    id: text(connection.id) || `connection/${index + 1}`,
    from,
    to,
    kind: text(connection.kind) || "audio"
  };
}

function normalizeControlValue(control, value) {
  if (control.type === "steps") {
    if (!Array.isArray(value) || value.length < 1 || value.length > 64) {
      throw new Error(`supersonic/control-steps-length-invalid:${control.parameter}`);
    }
    return value.map((step) => {
      if (step == null) return null;
      const note = Number(step);
      if (!Number.isInteger(note) || note < -48 || note > 48) {
        throw new Error(`supersonic/control-step-invalid:${control.parameter}`);
      }
      return note;
    });
  }
  if (control.type === "boolean") return Boolean(value);
  if (control.type === "choice") {
    const choices = control.choices.map((choice) =>
      typeof choice === "object" ? choice.value : choice);
    if (!choices.some((choice) => choice === value)) {
      throw new Error(`supersonic/control-choice-invalid:${control.parameter}`);
    }
    return value;
  }
  const number = Number(value);
  if (!Number.isFinite(number)) throw new Error(`supersonic/control-number-invalid:${control.parameter}`);
  if (control.integer && !Number.isInteger(number)) {
    throw new Error(`supersonic/control-integer-invalid:${control.parameter}`);
  }
  if (control.min != null && number < control.min ||
      control.max != null && number > control.max) {
    throw new Error(`supersonic/control-range-invalid:${control.parameter}`);
  }
  return number;
}

function applyOverlay(graph, overlay) {
  for (const node of graph.nodes) {
    for (const control of node.controls) {
      const value = overlay?.[node.id]?.[control.parameter];
      if (value !== undefined) node.params[control.parameter] = normalizeControlValue(control, value);
    }
  }
}

function requiredGraph(graphs, graphId) {
  const state = graphs.get(String(graphId));
  if (!state) throw new Error(`supersonic/graph-not-found:${graphId}`);
  return state;
}

function endpoint(value) {
  const parts = array(value).map(text);
  if (parts.length !== 2 || !parts[0] || !parts[1]) throw new Error("supersonic/endpoint-invalid");
  return parts;
}

function plain(value) {
  if (value instanceof Map) {
    return Object.fromEntries([...value].map(([key, item]) => [text(key), plain(item)]));
  }
  if (Array.isArray(value)) return value.map(plain);
  if (value && typeof value === "object" && value.constructor === Object) {
    return Object.fromEntries(Object.entries(value).map(([key, item]) => [text(key), plain(item)]));
  }
  return value;
}

function text(value) {
  return value == null ? "" : String(value?.name ?? value).replace(/^:/, "");
}

function array(value) {
  return Array.isArray(value) ? value : [];
}

function finite(value) {
  if (value == null) return null;
  const number = Number(value);
  return Number.isFinite(number) ? number : null;
}

function clone(value) {
  return globalThis.structuredClone ? structuredClone(value) : JSON.parse(JSON.stringify(value));
}
