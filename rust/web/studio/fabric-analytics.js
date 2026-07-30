/**
 * Read-only Studio model for Hara Fabric analytics.
 *
 * The service deliberately emits metadata rather than report bodies. This
 * model preserves that boundary: it knows topology, counters, sizes, timings,
 * and error codes, but never asks an adapter for message payloads.
 */
export class FabricAnalyticsModel {
  constructor({ maxEvents = 2000 } = {}) {
    this.maxEvents = maxEvents;
    this.node = null;
    this.spaces = new Map();
    this.events = [];
    this.flows = new Map();
    this.cursor = 0;
  }

  ingestTopology(value) {
    const topology = parse(value, "topology");
    if (!topology || typeof topology.node !== "string" || !Array.isArray(topology.spaces)) {
      throw new TypeError("fabric topology must contain node and spaces");
    }
    this.node = topology.node;
    this.spaces.clear();
    for (const space of topology.spaces) {
      if (typeof space?.id !== "string" || !Array.isArray(space.sessions)) {
        throw new TypeError("fabric topology space must contain id and sessions");
      }
      this.spaces.set(space.id, {
        id: space.id,
        sessions: new Map(space.sessions.map((session) => [session.id, {
          id: session.id,
          shard: session.shard,
          home: session.home,
          namespaces: [...(session.namespaces ?? [])].sort()
        }]))
      });
    }
    return this.snapshot();
  }

  ingestEvents(values) {
    for (const input of values ?? []) {
      const event = parse(input, "event");
      if (!Number.isSafeInteger(event?.cursor) || typeof event.kind !== "string") {
        throw new TypeError("fabric event must contain cursor and kind");
      }
      if (event.cursor <= this.cursor) continue;
      this.cursor = event.cursor;
      this.events.push(event);
      if (event.kind === "report/accepted") this.recordFlow(event);
    }
    if (this.events.length > this.maxEvents) {
      this.events.splice(0, this.events.length - this.maxEvents);
    }
    return this.snapshot();
  }

  recordFlow(event) {
    const source = event.detail?.source;
    const target = event.session;
    const signal = event.detail?.signal;
    if (!event.space || !source || !target || !signal) return;
    const key = `${event.space}\0${source}\0${target}\0${signal}`;
    const active = this.flows.get(key) ?? {
      space: event.space, source, target, signal, reports: 0, bytes: 0,
      delivered: 0, retained: 0, lastCursor: 0, lastTimestamp: 0
    };
    active.reports += 1;
    active.bytes += integer(event.detail.bytes);
    active.delivered += integer(event.detail.delivered);
    active.retained += event.detail.retained === "true" ? 1 : 0;
    active.lastCursor = event.cursor;
    active.lastTimestamp = event.timestamp_ms;
    this.flows.set(key, active);
  }

  snapshot() {
    return {
      node: this.node,
      cursor: this.cursor,
      spaces: [...this.spaces.values()].map((space) => ({
        id: space.id,
        sessions: [...space.sessions.values()]
      })),
      flows: [...this.flows.values()].sort((a, b) => b.lastCursor - a.lastCursor),
      events: [...this.events]
    };
  }
}

/** Polls a caller-provided read-only adapter and keeps the model current. */
export class FabricAnalyticsController {
  constructor({ model = new FabricAnalyticsModel(), adapter, onUpdate = () => {} } = {}) {
    if (!adapter?.topology || !adapter?.events) {
      throw new TypeError("fabric analytics adapter requires topology() and events()");
    }
    this.model = model;
    this.adapter = adapter;
    this.onUpdate = onUpdate;
  }

  async refresh({ limit = 500 } = {}) {
    this.model.ingestTopology(await this.adapter.topology());
    this.model.ingestEvents(await this.adapter.events(this.model.cursor, limit));
    const snapshot = this.model.snapshot();
    this.onUpdate(snapshot);
    return snapshot;
  }
}

/**
 * Small dependency-free SVG view used by Studio or an embedding panel.
 * Spaces form columns, sessions are nodes, and accumulated report traffic is
 * rendered as weighted directed edges.
 */
export class FabricTopologyView {
  constructor(root, { document = globalThis.document } = {}) {
    if (!root || !document) throw new TypeError("FabricTopologyView requires a DOM root");
    this.root = root;
    this.document = document;
  }

  render(snapshot) {
    this.root.replaceChildren();
    const width = Math.max(640, snapshot.spaces.length * 320);
    const height = Math.max(320, ...snapshot.spaces.map((space) => 120 + space.sessions.length * 92));
    const svg = element(this.document, "svg", {
      viewBox: `0 0 ${width} ${height}`,
      role: "img",
      "aria-label": `Hara Fabric topology for node ${snapshot.node ?? "unknown"}`
    });
    const defs = element(this.document, "defs", {});
    const marker = element(this.document, "marker", {
      id: "hara-fabric-arrow", viewBox: "0 0 10 10", refX: 9, refY: 5,
      markerWidth: 6, markerHeight: 6, orient: "auto-start-reverse"
    });
    marker.append(element(this.document, "path", { d: "M 0 0 L 10 5 L 0 10 z" }));
    defs.append(marker);
    svg.append(defs);
    const positions = new Map();

    snapshot.spaces.forEach((space, spaceIndex) => {
      const x = 50 + spaceIndex * 310;
      svg.append(element(this.document, "text", {
        x, y: 36, class: "fabric-space-label"
      }, space.id));
      space.sessions.forEach((session, sessionIndex) => {
        const y = 72 + sessionIndex * 92;
        positions.set(`${space.id}\0${session.id}`, { x: x + 110, y: y + 25 });
        const group = element(this.document, "g", {
          class: "fabric-session",
          "data-space": space.id,
          "data-session": session.id
        });
        group.append(element(this.document, "rect", {
          x, y, width: 220, height: 64, rx: 8
        }));
        group.append(element(this.document, "text", {
          x: x + 12, y: y + 24, class: "fabric-session-label"
        }, session.id));
        group.append(element(this.document, "text", {
          x: x + 12, y: y + 46, class: "fabric-session-meta"
        }, `shard ${session.shard} · ${session.namespaces.length} namespaces`));
        svg.append(group);
      });
    });

    const edgeLayer = element(this.document, "g", { class: "fabric-flows" });
    for (const flow of [...snapshot.flows].reverse()) {
      const from = positions.get(`${flow.space}\0${flow.source}`);
      const to = positions.get(`${flow.space}\0${flow.target}`);
      if (!from || !to) continue;
      edgeLayer.append(element(this.document, "line", {
        x1: from.x, y1: from.y, x2: to.x, y2: to.y,
        class: "fabric-flow",
        "data-signal": flow.signal,
        "stroke-width": Math.min(8, 1 + Math.log2(flow.reports + 1))
      }));
    }
    svg.prepend(edgeLayer);
    this.root.append(svg);
    return svg;
  }
}

function parse(value, label) {
  if (typeof value !== "string") return value;
  try { return JSON.parse(value); }
  catch (error) { throw new TypeError(`invalid fabric ${label} JSON: ${error.message}`); }
}

function integer(value) {
  const parsed = Number.parseInt(value ?? "0", 10);
  return Number.isSafeInteger(parsed) ? parsed : 0;
}

function element(document, name, attributes, text = null) {
  const node = document.createElementNS("http://www.w3.org/2000/svg", name);
  for (const [key, value] of Object.entries(attributes)) node.setAttribute(key, String(value));
  if (text !== null) node.textContent = text;
  return node;
}
