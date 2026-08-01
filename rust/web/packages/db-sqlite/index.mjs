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

export function createSqliteProvider(sqlite3InitModule) {
  let sqlitePromise;
  let nextConnectionId = 0;
  const connections = new Map();

  async function sqlite() {
    if (!sqlitePromise) sqlitePromise = sqlite3InitModule();
    return sqlitePromise;
  }

  function connection(id) {
    const database = connections.get(Number(id));
    if (!database) {
      throw new Error(`db/sqlite-connection-missing: ${id}`);
    }
    return database;
  }

  async function openDatabase(options) {
    const sqlite3 = await sqlite();
    const storage = keywordName(option(options, "storage", "memory"));
    if (storage !== "memory" && storage !== "transient") {
      throw new Error(
        `db/sqlite-storage-unsupported: ${storage}; the portable provider currently supports only memory`
      );
    }
    const database = new sqlite3.oo1.DB(":memory:", "ct");
    const id = ++nextConnectionId;
    connections.set(id, database);
    return {
      id,
      engine: "sqlite",
      storage: "memory",
      filename: ":memory:"
    };
  }

  function execDatabase(id, sql, params) {
    const database = connection(id);
    const bind = fromHta(params ?? []);
    const options = { sql: String(sql) };
    if (Array.isArray(bind) ? bind.length > 0 : bind != null) options.bind = bind;
    database.exec(options);
    return { affected: database.changes() };
  }

  function queryDatabase(id, sql, params) {
    const database = connection(id);
    const columns = [];
    const rows = [];
    const bind = fromHta(params ?? []);
    const options = {
      sql: String(sql),
      rowMode: "array",
      columnNames: columns,
      resultRows: rows
    };
    if (Array.isArray(bind) ? bind.length > 0 : bind != null) options.bind = bind;
    database.exec(options);
    return {
      columns,
      rows,
      affected: database.changes()
    };
  }

  function closeDatabase(id) {
    const key = Number(id);
    const database = connections.get(key);
    if (!database) return false;
    connections.delete(key);
    database.close();
    return true;
  }

  async function call(environment, operation, args) {
    switch (operation) {
      case "version":
        return { engine: "sqlite", version: (await sqlite()).version.libVersion };
      case "open":
        return openDatabase(args[0]);
      case "exec":
        return execDatabase(args[0], args[1], args[2]);
      case "query":
        return queryDatabase(args[0], args[1], args[2]);
      case "close":
        return closeDatabase(args[0]);
      default:
        throw new Error(`db/sqlite-operation-unknown: ${operation}`);
    }
  }

  function closeAll() {
    for (const [id] of connections) closeDatabase(id);
  }

  return Object.freeze({ call, closeAll });
}
