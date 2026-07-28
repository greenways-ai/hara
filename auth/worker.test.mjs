import test from "node:test";
import assert from "node:assert/strict";
import worker, { internals } from "./worker.js";

const env = {
  GITHUB_CLIENT_ID: "client-id",
  GITHUB_CLIENT_SECRET: "client-secret",
  SESSION_SECRET: "a-test-session-secret-that-is-long-enough",
  ALLOWED_ORIGINS: "https://www.hara-lang.org,http://localhost:8002"
};

test("encrypted sessions round-trip and reject expired payloads", async () => {
  const value = { profile: { login: "hara" }, expiresAt: Date.now() + 60_000 };
  const sealed = await internals.seal(value, env.SESSION_SECRET);
  assert.notEqual(sealed, JSON.stringify(value));
  assert.deepEqual(await internals.unseal(sealed, env.SESSION_SECRET), value);
  const expired = await internals.seal({ expiresAt: Date.now() - 1 }, env.SESSION_SECRET);
  assert.equal(await internals.unseal(expired, env.SESSION_SECRET), null);
});

test("session reports whether the server is configured without exposing secrets", async () => {
  const response = await worker.fetch(new Request("https://auth.hara-lang.org/session", {
    headers: { Origin: "https://www.hara-lang.org" }
  }), env);
  assert.equal(response.status, 200);
  assert.deepEqual(await response.json(), {
    authenticated: false,
    configured: true,
    profile: null
  });
  assert.equal(response.headers.get("Access-Control-Allow-Origin"), "https://www.hara-lang.org");
});

test("oauth start uses PKCE, an encrypted state cookie, and an allowlisted return URL", async () => {
  const response = await worker.fetch(new Request(
    "https://auth.hara-lang.org/github/start?returnTo=https%3A%2F%2Fevil.example%2Fsteal"
  ), env);
  assert.equal(response.status, 302);
  const redirect = new URL(response.headers.get("Location"));
  assert.equal(redirect.origin, "https://github.com");
  assert.equal(redirect.searchParams.get("code_challenge_method"), "S256");
  assert.ok(redirect.searchParams.get("code_challenge"));
  const cookie = response.headers.get("Set-Cookie");
  assert.match(cookie, /HttpOnly/);
  assert.match(cookie, /Secure/);
  assert.doesNotMatch(cookie, /client-secret/);
  assert.equal(internals.safeReturnTo("https://evil.example/steal", env), "https://www.hara-lang.org/");
});

test("oauth callback stores the token only in an encrypted HttpOnly session", async () => {
  const start = await worker.fetch(new Request(
    "https://auth.hara-lang.org/github/start?returnTo=https%3A%2F%2Fwww.hara-lang.org%2F"
  ), env);
  const authorize = new URL(start.headers.get("Location"));
  const oauthCookie = start.headers.get("Set-Cookie").split(";")[0];
  const originalFetch = globalThis.fetch;
  const calls = [];
  globalThis.fetch = async (url, init) => {
    calls.push({ url: String(url), init });
    if (String(url).includes("login/oauth/access_token")) {
      return Response.json({ access_token: "github-user-token" });
    }
    if (String(url) === "https://api.github.com/user") {
      return Response.json({
        login: "hara-user",
        name: "Hara User",
        avatar_url: "https://avatars.example/hara.png",
        html_url: "https://github.com/hara-user"
      });
    }
    if (String(url) === "https://api.github.com/gists") {
      return Response.json({ id: "gist-1", html_url: "https://gist.github.com/gist-1" });
    }
    throw new Error(`unexpected fetch ${url}`);
  };
  try {
    const callback = await worker.fetch(new Request(
      `https://auth.hara-lang.org/github/callback?code=code-1&state=${authorize.searchParams.get("state")}`,
      { headers: { Cookie: oauthCookie } }
    ), env);
    assert.equal(callback.status, 302);
    assert.equal(callback.headers.get("Location"), "https://www.hara-lang.org/");
    const setCookies = callback.headers.getSetCookie();
    assert.equal(setCookies.length, 2);
    assert.ok(setCookies.every((value) => !value.includes("github-user-token")));
    const sessionCookie = setCookies.find((value) => value.startsWith("hara_session=")).split(";")[0];

    const session = await worker.fetch(new Request("https://auth.hara-lang.org/session", {
      headers: { Cookie: sessionCookie, Origin: "https://www.hara-lang.org" }
    }), env);
    assert.deepEqual(await session.json(), {
      authenticated: true,
      configured: true,
      profile: {
        login: "hara-user",
        name: "Hara User",
        avatarUrl: "https://avatars.example/hara.png",
        htmlUrl: "https://github.com/hara-user"
      }
    });

    const gist = await worker.fetch(new Request("https://auth.hara-lang.org/github/gists", {
      method: "POST",
      headers: {
        Cookie: sessionCookie,
        Origin: "https://www.hara-lang.org",
        "Content-Type": "application/json",
        "X-Hara-Request": "studio"
      },
      body: JSON.stringify({ description: "Hara workspace", files: {} })
    }), env);
    assert.equal(gist.status, 200);
    assert.deepEqual(await gist.json(), { id: "gist-1", html_url: "https://gist.github.com/gist-1" });
    assert.equal(calls.at(-1).init.headers.Authorization, "Bearer github-user-token");
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test("credentialed mutation routes reject untrusted origins", async () => {
  const response = await worker.fetch(new Request("https://auth.hara-lang.org/logout", {
    method: "POST",
    headers: { Origin: "https://evil.example", "X-Hara-Request": "sign-out" }
  }), env);
  assert.equal(response.status, 403);
});

test("unconfigured server does not begin OAuth", async () => {
  const response = await worker.fetch(new Request("https://auth.hara-lang.org/github/start"), {
    ALLOWED_ORIGINS: env.ALLOWED_ORIGINS
  });
  assert.equal(response.status, 503);
  assert.deepEqual(await response.json(), { error: "GITHUB_AUTH_NOT_CONFIGURED" });
});
