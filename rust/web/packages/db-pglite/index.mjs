function keywordName(value) {
  return value && typeof value === "object" && typeof value.name === "string"
    ? value.name
    : value;
}

function option(options, name, fallback = undefined) {
  if (!(options instanceof Map)) return fallback;
  for (const [key, value] of options) {
    if (keywordName(key) === name) return value;
  }
  return fallback;
}

function fromHta(value) {
  if (Array.isArray(value)) return value.map(fromHta);
  if (value instanceof Uint8Array) return value;
  if (value instanceof Map) {
    const output = Object.create(null);
    for (const [key, item] of value) {
      output[String(keywordName(key))] = fromHta(item);
    }
    return output;
  }
  const keyword = keywordName(value);
  return keyword === value ? value : keyword;
}

function resultShape(result) {
  return {
    columns: (result.fields ?? []).map(field => field.name),
    rows: result.rows ?? [],
    affected: result.affectedRows ?? 0
  };
}

export function createPgliteProvider(PGlite) {
  let nextConnectionId = 0;
  const connections = new Map();

  function connection(id) {
    const database = connections.get(Number(id));
    if (!database) {
      throw new Error(`db/pglite-connection-missing: ${id}`);
    }
    return database;
  }

  async function openDatabase(options) {
    const storage = keywordName(option(options, "storage", "memory"));
    if (storage !== "memory" && storage !== "transient") {
      throw new Error(
        `db/pglite-storage-unsupported: ${storage}; the portable provider currently supports only memory`
      );
    }
    const database = await PGlite.create("memory://");
    const id = ++nextConnectionId;
    connections.set(id, database);
    return {
      id,
      engine: "postgresql",
      provider: "pglite",
      storage: "memory"
    };
  }

  async function execute(id, sql, parameters) {
    const database = connection(id);
    const result = await database.query(
      String(sql),
      fromHta(parameters ?? []),
      { rowMode: "array" }
    );
    return resultShape(result);
  }

  async function closeDatabase(id) {
    const key = Number(id);
    const database = connections.get(key);
    if (!database) return false;
    connections.delete(key);
    await database.close();
    return true;
  }

  async function engineVersion() {
    const database = await PGlite.create("memory://");
    try {
      const result = await database.query("select version()", [], { rowMode: "array" });
      return {
        engine: "postgresql",
        provider: "pglite",
        version: result.rows[0][0]
      };
    } finally {
      await database.close();
    }
  }

  async function call(environment, operation, args) {
    switch (operation) {
      case "version":
        return engineVersion();
      case "open":
        return openDatabase(args[0]);
      case "exec":
        return execute(args[0], args[1], args[2]);
      case "query":
        return execute(args[0], args[1], args[2]);
      case "close":
        return closeDatabase(args[0]);
      default:
        throw new Error(`db/pglite-operation-unknown: ${operation}`);
    }
  }

  async function closeAll() {
    for (const [id] of connections) await closeDatabase(id);
  }

  return Object.freeze({ call, closeAll });
}
