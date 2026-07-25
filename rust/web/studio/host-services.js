const DEFAULT_DATABASE = "hara-studio";
const STORE = "kv";

/**
 * Generic host services for studio kernels: an IndexedDB key/value store and
 * fetch-backed HTTP. Returns a handler map for the `hostCalls` option of
 * `HtaContext`, keyed "service/method"; handlers are async, take plain
 * decoded HTA arguments, and return encodeable values (null -> nil).
 */
export function createHostServices(options = {}) {
  const dbName = options.dbName ?? DEFAULT_DATABASE;
  const fetchImpl = options.fetch ?? globalThis.fetch.bind(globalThis);
  let opening = null;

  function open() {
    opening ??= new Promise((resolve, reject) => {
      const request = indexedDB.open(dbName, 1);
      request.onupgradeneeded = () => request.result.createObjectStore(STORE);
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
    });
    const pending = opening;
    pending.catch(() => { if (opening === pending) opening = null; });
    return pending;
  }

  async function store(mode) {
    const db = await open();
    return db.transaction(STORE, mode).objectStore(STORE);
  }

  return {
    "store/get": async (key) => request(await store("readonly"), "get", key),
    "store/put": async (key, value) => {
      await request(await store("readwrite"), "put", value, key);
      return true;
    },
    "store/del": async (key) => {
      await request(await store("readwrite"), "delete", key);
      return true;
    },
    "store/keys": async (prefix) => {
      const keys = await request(await store("readonly"), "getAllKeys");
      return prefix === undefined || prefix === null
        ? keys
        : keys.filter((key) => key.startsWith(prefix));
    },
    "http/get": async (url) => {
      const response = await fetchImpl(url);
      if (!response.ok) throw new Error(`http/get failed with status ${response.status}`);
      return response.text();
    },
    "json/parse": async (text) => fromJson(JSON.parse(text))
  };
}

// Decoded shape: objects -> Maps with string keys, arrays -> arrays, scalars
// pass through (null -> nil on the hara side). String keys keep host-call
// arguments and store keys free of opaque keyword objects.
function fromJson(value) {
  if (Array.isArray(value)) return value.map(fromJson);
  if (value !== null && typeof value === "object") {
    return new Map(Object.entries(value).map(([key, item]) => [key, fromJson(item)]));
  }
  return value;
}

async function request(store, method, ...arguments_) {
  return new Promise((resolve, reject) => {
    const operation = store[method](...arguments_);
    operation.onsuccess = () => resolve(operation.result ?? null);
    operation.onerror = () => reject(operation.error);
  });
}
