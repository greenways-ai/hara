import test from "node:test";
import assert from "node:assert/strict";
import { GitHubAuthClient } from "../../website/github-auth.js";
import { AiAdapterRepository, createAiCapability } from "../../website/ai-adapters.js";

class MemoryStorage {
  constructor() { this.values = new Map(); }
  getItem(key) { return this.values.get(key) ?? null; }
  setItem(key, value) { this.values.set(key, String(value)); }
  removeItem(key) { this.values.delete(key); }
}

test("GitHub auth keeps token exchange behind the configured service", async () => {
  const calls = [];
  const client = new GitHubAuthClient({
    baseUrl: "https://auth.hara.test/",
    fetch: async (url, init) => {
      calls.push({ url, init });
      return { ok: true, status: 200, json: async () => ({ authenticated: true, profile: { login: "hara-user" } }) };
    }
  });
  assert.deepEqual(await client.session(), {
    authenticated: true, configured: true, profile: { login: "hara-user" }
  });
  assert.equal(calls[0].init.credentials, "include");
  assert.equal(calls[0].url, "https://auth.hara.test/session");
});

test("unconfigured GitHub auth remains explicit and does not navigate", async () => {
  const client = new GitHubAuthClient();
  assert.deepEqual(await client.session(), { authenticated: false, configured: false, profile: null });
  assert.throws(() => client.signIn("https://www.hara-lang.org/"), /NOT_CONFIGURED/);
});

test("GitHub auth respects an unconfigured server response", async () => {
  const client = new GitHubAuthClient({
    baseUrl: "https://auth.hara.test",
    fetch: async () => ({
      ok: true,
      status: 200,
      json: async () => ({ authenticated: false, configured: false, profile: null })
    })
  });
  assert.deepEqual(await client.session(), { authenticated: false, configured: false, profile: null });
});

test("AI adapter definitions persist while API keys stay session-only", () => {
  const storage = new MemoryStorage();
  const secrets = new MemoryStorage();
  const repository = new AiAdapterRepository({ storage, secrets });
  const adapter = repository.save({
    workspaceId: "canvas-one",
    name: "My model",
    kind: "openai-compatible",
    endpoint: "https://models.example/v1/chat/completions",
    model: "example-1",
    apiKey: "secret-value"
  });
  assert.equal(repository.list("canvas-one").length, 1);
  assert.equal(repository.hasSecret(adapter.id), true);
  assert.doesNotMatch(storage.getItem("hara.ai.adapters.v1"), /secret-value/);
});

test("AI capability scopes adapters to their owning workspace", async () => {
  const storage = new MemoryStorage();
  const secrets = new MemoryStorage();
  let request;
  const repository = new AiAdapterRepository({
    storage, secrets,
    fetch: async (url, init) => {
      request = { url, init };
      return { ok: true, json: async () => ({ model: "example-1", choices: [{ message: { content: "hello" } }] }) };
    }
  });
  const adapter = repository.save({
    workspaceId: "alpha", name: "Example", kind: "openai-compatible",
    endpoint: "https://models.example/v1/chat/completions", model: "example-1", apiKey: "session-key"
  });
  const capability = createAiCapability(repository, {
    workspaceForSession: (sessionId) => sessionId === "workspace.alpha" ? "alpha" : "beta"
  });
  const alpha = capability.forNode({ sessionId: "workspace.alpha" });
  assert.equal((await alpha.list()).length, 1);
  assert.equal((await alpha.chat(adapter.id, [{ role: "user", content: "hi" }])).text, "hello");
  assert.equal(request.init.headers.Authorization, "Bearer session-key");
  assert.equal(capability.forNode({ sessionId: "workspace.beta" }).list().length, 0);
});
