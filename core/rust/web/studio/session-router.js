import { normalizeFrame, NodeProtocolError } from "./node-runtime.js";

/** Session-qualified compatibility ingress. It deliberately uses eval-bound
 * only at this edge; host-local graph edges never pass through this router. */
export class SessionRouter {
  constructor({ ingressSource = "(studio.session/invoke-ingress __hta_arg_0)" } = {}) {
    this.ingressSource = ingressSource;
    this.sessions = new Map();
    this.subscriptions = new Map();
    this.nextSubscription = 0;
  }

  register(sessionId, context, { capabilities = [], onRelease = null } = {}) {
    if (!sessionId || !context?.call) throw new NodeProtocolError("session/invalid", "session requires an id and HtaContext");
    const existing = this.sessions.get(sessionId);
    if (existing) {
      if (existing.context !== context) {
        throw new NodeProtocolError("session/already-exists", `session already registered: ${sessionId}`);
      }
      for (const capability of capabilities) existing.capabilities.add(capability);
      existing.onRelease = onRelease ?? existing.onRelease;
      return this.info(sessionId);
    }
    this.sessions.set(sessionId, { id: sessionId, context, capabilities: new Set(capabilities), onRelease });
    return this.info(sessionId);
  }

  async unregister(sessionId) {
    const session = this.sessions.get(sessionId);
    if (!session) return false;
    this.sessions.delete(sessionId);
    for (const [id, subscription] of this.subscriptions) {
      if (subscription.sessionId === sessionId) this.subscriptions.delete(id);
    }
    await session.onRelease?.(sessionId);
    return true;
  }

  subscribe(sessionId, signal, callbackId) {
    this.require(sessionId);
    if (typeof signal !== "string" || !signal) throw new NodeProtocolError("session/signal", "session signal is required");
    if (typeof callbackId !== "string" || !callbackId) throw new NodeProtocolError("session/callback", "session callback id is required");
    const id = `subscription-${++this.nextSubscription}`;
    this.subscriptions.set(id, { id, sessionId, signal, callbackId });
    return id;
  }

  unsubscribe(id) { return this.subscriptions.delete(id); }

  async deliver(sessionId, frame) {
    const session = this.require(sessionId);
    const normalized = normalizeFrame(frame);
    const matching = [...this.subscriptions.values()]
      .filter((subscription) => subscription.sessionId === sessionId && subscription.signal === normalized.signal);
    if (!matching.length) return { accepted: true, delivered: 0 };
    await Promise.all(matching.map((subscription) => session.context.call("eval-bound", [
      this.ingressSource,
      [toHta({ ...normalized, meta: { ...normalized.meta, "session/callback": subscription.callbackId } })]
    ])));
    return { accepted: true, delivered: matching.length };
  }

  info(sessionId) {
    const session = this.require(sessionId);
    return {
      sessionId: session.id,
      capabilities: [...session.capabilities].sort(),
      subscriptions: [...this.subscriptions.values()].filter((entry) => entry.sessionId === sessionId).length
    };
  }

  list() { return [...this.sessions.keys()].map((id) => this.info(id)); }

  require(sessionId) {
    const session = this.sessions.get(sessionId);
    if (!session) throw new NodeProtocolError("session/not-found", `unknown session: ${sessionId}`);
    return session;
  }
}

// HtaContext only accepts the codec's Map/array/scalar value vocabulary.
// Keep this conversion at the compatibility boundary so host graph transport
// itself remains plain normalized substrate data.
function toHta(value) {
  if (Array.isArray(value)) return value.map(toHta);
  if (value !== null && typeof value === "object" &&
      !(value instanceof Uint8Array) && !(value instanceof ArrayBuffer) && !ArrayBuffer.isView(value)) {
    return new Map(Object.entries(value).map(([key, entry]) => [key, toHta(entry)]));
  }
  return value;
}
