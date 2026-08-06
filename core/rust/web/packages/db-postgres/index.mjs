function keywordName(value) {
  return value && typeof value === "object" && typeof value.name === "string"
    ? value.name
    : value;
}

function entries(options) {
  if (!(options instanceof Map)) return [];
  return [...options].map(([key, value]) => [String(keywordName(key)), value]);
}

function option(options, name) {
  return entries(options).find(([key]) => key === name)?.[1];
}

function selectedProvider(options, environment) {
  const explicit = keywordName(option(options, "provider"));
  if (explicit && explicit !== "auto") return String(explicit);
  const keys = new Set(entries(options).map(([key]) => key));
  const remote = ["url", "host", "endpoint", "database", "dbname", "user"].some(key => keys.has(key));
  const embedded = ["storage", "path", "database-name"].some(key => keys.has(key));
  if (remote && embedded) throw new Error("postgres/config-invalid: conflicting remote and embedded options");
  if (remote) return "postgres";
  if (embedded) return "pglite";
  return environment === "browser" ? "pglite" : "postgres";
}

/** Creates the exact host-call surface consumed by std.db.postgres.
 *
 * `pglite` and `remote` expose `call(environment, operation, args)`. A browser
 * Hestia may omit `remote` or provide an authenticated on-prem transport; this
 * adapter never attempts raw PostgreSQL TCP from the browser.
 */
export function createPostgresHostServices({ pglite = null, remote = null, environment = "browser" } = {}) {
  let nextConnectionId = 0;
  let nextSubscriptionId = 0;
  const connections = new Map();
  const subscriptions = new Map();

  function implementation(name) {
    const value = name === "pglite" ? pglite : remote;
    if (!value) throw new Error(`postgres/provider-unavailable: ${name}`);
    return value;
  }

  function connection(id) {
    const value = connections.get(Number(id));
    if (!value) throw new Error(`postgres/connection-closed: ${id}`);
    return value;
  }

  async function open(options = new Map()) {
    const provider = selectedProvider(options, environment);
    const backend = implementation(provider);
    const descriptor = await backend.call(environment, "open", [options]);
    const id = ++nextConnectionId;
    connections.set(id, { backend, rawId: descriptor.id, provider });
    return { ...descriptor, id, provider };
  }

  async function close(id) {
    const key = Number(id);
    const value = connections.get(key);
    if (!value) return false;
    for (const [subscriptionId, subscription] of subscriptions) {
      if (subscription.connection === key) await unlisten(subscriptionId);
    }
    connections.delete(key);
    return value.backend.call(environment, "close", [value.rawId]);
  }

  async function listen(id, channel) {
    const value = connection(id);
    const descriptor = await value.backend.call(environment, "listen", [value.rawId, channel]);
    const subscriptionId = ++nextSubscriptionId;
    subscriptions.set(subscriptionId, {
      connection: Number(id), backend: value.backend, rawId: descriptor.id
    });
    return { ...descriptor, id: subscriptionId };
  }

  async function unlisten(id) {
    const key = Number(id);
    const value = subscriptions.get(key);
    if (!value) return false;
    subscriptions.delete(key);
    return value.backend.call(environment, "unlisten", [value.rawId]);
  }

  async function call(operation, args) {
    switch (operation) {
      case "describe":
        return {
          engine: "postgresql",
          providers: [pglite && "pglite", remote && "postgres"].filter(Boolean),
          mode: environment
        };
      case "open": return open(args[0]);
      case "close": return close(args[0]);
      case "listen": return listen(args[0], args[1]);
      case "unlisten": return unlisten(args[0]);
      case "notification-next": {
        const value = subscriptions.get(Number(args[0]));
        if (!value) throw new Error(`postgres/subscription-closed: ${args[0]}`);
        return value.backend.call(environment, operation, [value.rawId]);
      }
      case "version":
      case "exec":
      case "query":
      case "notify": {
        const value = connection(args[0]);
        const backendOperation = operation === "query" && args.length > 3 && value.provider === "pglite"
          ? "query-options"
          : operation;
        return value.backend.call(environment, backendOperation, [value.rawId, ...args.slice(1)]);
      }
      case "wait-ready":
      case "database-create":
      case "database-drop":
      case "server-start":
      case "server-stop": {
        const provider = selectedProvider(args[0] instanceof Map ? args[0] : new Map(), environment);
        return implementation(provider).call(environment, operation, args);
      }
      default: throw new Error(`postgres/operation-unknown: ${operation}`);
    }
  }

  return Object.freeze(Object.fromEntries([
    "describe", "open", "close", "version", "exec", "query", "wait-ready",
    "database-create", "database-drop", "server-start", "server-stop", "listen",
    "notification-next", "unlisten", "notify"
  ].map(operation => [`std.db.postgres/${operation}`, (...args) => call(operation, args)])));
}
