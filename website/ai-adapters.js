const STORAGE_KEY = "hara.ai.adapters.v1";
const SECRET_PREFIX = "hara.ai.secret.";
const KINDS = new Set(["openai-compatible", "anthropic", "custom-json"]);

function identifier() {
  if (crypto.randomUUID) return `ai-${crypto.randomUUID()}`;
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  return `ai-${[...bytes].map((value) => value.toString(16).padStart(2, "0")).join("")}`;
}

export class AiAdapterRepository {
  constructor({ storage = globalThis.localStorage, secrets = globalThis.sessionStorage, fetch = globalThis.fetch?.bind(globalThis) } = {}) {
    this.storage = storage;
    this.secrets = secrets;
    this.fetch = fetch;
  }

  all() {
    try {
      const value = JSON.parse(this.storage?.getItem(STORAGE_KEY) ?? "[]");
      return Array.isArray(value) ? value : [];
    } catch {
      return [];
    }
  }

  list(workspaceId) {
    return this.all().filter((adapter) => adapter.workspaceId === workspaceId);
  }

  save(input) {
    const adapter = normalizeAdapter(input);
    const records = this.all().filter((item) => item.id !== adapter.id);
    records.push(adapter);
    this.storage?.setItem(STORAGE_KEY, JSON.stringify(records));
    if (input.apiKey) this.secrets?.setItem(`${SECRET_PREFIX}${adapter.id}`, input.apiKey);
    return adapter;
  }

  remove(id) {
    this.storage?.setItem(STORAGE_KEY, JSON.stringify(this.all().filter((item) => item.id !== id)));
    this.secrets?.removeItem(`${SECRET_PREFIX}${id}`);
  }

  hasSecret(id) { return Boolean(this.secrets?.getItem(`${SECRET_PREFIX}${id}`)); }

  async chat(workspaceId, adapterId, messages, options = {}) {
    const adapter = this.list(workspaceId).find((item) => item.id === adapterId);
    if (!adapter) throw new Error("AI_ADAPTER_NOT_FOUND");
    if (!Array.isArray(messages) || !messages.length) throw new Error("AI_MESSAGES_REQUIRED");
    if (!this.fetch) throw new Error("AI_FETCH_UNAVAILABLE");
    const key = this.secrets?.getItem(`${SECRET_PREFIX}${adapter.id}`) ?? "";
    const request = buildRequest(adapter, key, messages, options);
    const response = await this.fetch(request.url, request.init);
    if (!response.ok) throw new Error(`AI_ADAPTER_HTTP ${response.status}`);
    return parseResponse(adapter.kind, await response.json());
  }
}

export function createAiCapability(repository, { workspaceForSession }) {
  return Object.freeze({
    forNode({ sessionId }) {
      const workspaceId = workspaceForSession(sessionId);
      if (!workspaceId) throw new Error("AI_WORKSPACE_SCOPE_UNAVAILABLE");
      return Object.freeze({
        list: () => repository.list(workspaceId).map(publicAdapter),
        chat: (adapterId, messages, options = {}) => repository.chat(workspaceId, String(adapterId), messages, options)
      });
    }
  });
}

function normalizeAdapter(input) {
  const kind = String(input.kind ?? "");
  if (!KINDS.has(kind)) throw new Error("AI_ADAPTER_KIND");
  const workspaceId = String(input.workspaceId ?? "").trim();
  const name = String(input.name ?? "").trim();
  const model = String(input.model ?? "").trim();
  if (!workspaceId || !name || !model) throw new Error("AI_ADAPTER_FIELDS");
  const endpoint = normalizeEndpoint(input.endpoint);
  return {
    id: String(input.id || identifier()),
    workspaceId, name, kind, endpoint, model,
    createdAt: input.createdAt ?? new Date().toISOString()
  };
}

function normalizeEndpoint(value) {
  const url = new URL(String(value ?? ""));
  if (url.protocol !== "https:" && !(url.protocol === "http:" && ["localhost", "127.0.0.1"].includes(url.hostname))) {
    throw new Error("AI_ADAPTER_HTTPS_REQUIRED");
  }
  return url.href.replace(/\/$/, "");
}

function buildRequest(adapter, key, messages, options) {
  const headers = { "Content-Type": "application/json", Accept: "application/json" };
  let body;
  if (adapter.kind === "anthropic") {
    if (key) headers["x-api-key"] = key;
    headers["anthropic-version"] = "2023-06-01";
    body = { model: adapter.model, messages, max_tokens: Number(options.maxTokens ?? 1024) };
  } else {
    if (key) headers.Authorization = `Bearer ${key}`;
    body = adapter.kind === "custom-json"
      ? { adapter: adapter.name, model: adapter.model, messages, options }
      : { model: adapter.model, messages, temperature: options.temperature };
  }
  return { url: adapter.endpoint, init: { method: "POST", headers, body: JSON.stringify(body) } };
}

function parseResponse(kind, value) {
  if (kind === "anthropic") {
    return { text: value.content?.map((item) => item.text ?? "").join("") ?? "", model: value.model, usage: value.usage ?? null };
  }
  const text = value.choices?.[0]?.message?.content ?? value.text ?? value.output ?? "";
  return { text, model: value.model ?? null, usage: value.usage ?? null };
}

function publicAdapter(adapter) {
  const { createdAt, ...value } = adapter;
  return value;
}
