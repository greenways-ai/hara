const encoder = new TextEncoder();
const decoder = new TextDecoder();
const OAUTH_COOKIE = "hara_oauth";
const SESSION_COOKIE = "hara_session";
const COOKIE_TTL = 60 * 60 * 24 * 30;
const OAUTH_TTL = 10 * 60;
const GITHUB_API = "https://api.github.com";

function configured(env) {
  return Boolean(env.GITHUB_CLIENT_ID && env.GITHUB_CLIENT_SECRET && env.SESSION_SECRET);
}

function allowedOrigins(env) {
  return new Set(String(env.ALLOWED_ORIGINS || "https://www.hara-lang.org")
    .split(",")
    .map((value) => value.trim())
    .filter(Boolean));
}

function requestOrigin(request) {
  return request.headers.get("Origin") || "";
}

function corsHeaders(request, env) {
  const origin = requestOrigin(request);
  if (!allowedOrigins(env).has(origin)) return {};
  return {
    "Access-Control-Allow-Origin": origin,
    "Access-Control-Allow-Credentials": "true",
    "Access-Control-Allow-Headers": "Content-Type, X-Hara-Request",
    "Access-Control-Allow-Methods": "GET, POST, PATCH, OPTIONS",
    Vary: "Origin"
  };
}

function json(request, env, value, status = 200, headers = {}) {
  return Response.json(value, {
    status,
    headers: {
      ...corsHeaders(request, env),
      "Cache-Control": "no-store",
      "Content-Security-Policy": "default-src 'none'; frame-ancestors 'none'",
      ...headers
    }
  });
}

function base64url(bytes) {
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary).replaceAll("+", "-").replaceAll("/", "_").replace(/=+$/, "");
}

function fromBase64url(value) {
  const normalized = value.replaceAll("-", "+").replaceAll("_", "/");
  const binary = atob(normalized.padEnd(Math.ceil(normalized.length / 4) * 4, "="));
  return Uint8Array.from(binary, (character) => character.charCodeAt(0));
}

function randomToken(size = 32) {
  const bytes = new Uint8Array(size);
  crypto.getRandomValues(bytes);
  return base64url(bytes);
}

async function encryptionKey(secret) {
  const digest = await crypto.subtle.digest("SHA-256", encoder.encode(secret));
  return crypto.subtle.importKey("raw", digest, "AES-GCM", false, ["encrypt", "decrypt"]);
}

async function seal(value, secret) {
  const iv = new Uint8Array(12);
  crypto.getRandomValues(iv);
  const key = await encryptionKey(secret);
  const ciphertext = await crypto.subtle.encrypt(
    { name: "AES-GCM", iv },
    key,
    encoder.encode(JSON.stringify(value))
  );
  const packed = new Uint8Array(iv.length + ciphertext.byteLength);
  packed.set(iv);
  packed.set(new Uint8Array(ciphertext), iv.length);
  return base64url(packed);
}

async function unseal(value, secret) {
  try {
    const packed = fromBase64url(value);
    if (packed.length < 29) return null;
    const key = await encryptionKey(secret);
    const cleartext = await crypto.subtle.decrypt(
      { name: "AES-GCM", iv: packed.slice(0, 12) },
      key,
      packed.slice(12)
    );
    const payload = JSON.parse(decoder.decode(cleartext));
    if (!payload?.expiresAt || payload.expiresAt < Date.now()) return null;
    return payload;
  } catch {
    return null;
  }
}

function cookies(request) {
  return Object.fromEntries(
    (request.headers.get("Cookie") || "")
      .split(";")
      .map((item) => item.trim().split("="))
      .filter(([name, value]) => name && value)
  );
}

function cookie(name, value, { maxAge = COOKIE_TTL } = {}) {
  return `${name}=${value}; Path=/; Max-Age=${maxAge}; HttpOnly; Secure; SameSite=None`;
}

function clearCookie(name) {
  return `${name}=; Path=/; Max-Age=0; HttpOnly; Secure; SameSite=None`;
}

function safeReturnTo(value, env) {
  try {
    const url = new URL(value);
    if (allowedOrigins(env).has(url.origin)) return url.href;
  } catch {
    // Fall through to the canonical site.
  }
  return "https://www.hara-lang.org/";
}

async function oauthStart(request, env) {
  if (!configured(env)) return json(request, env, { error: "GITHUB_AUTH_NOT_CONFIGURED" }, 503);
  const requestUrl = new URL(request.url);
  const state = randomToken();
  const verifier = randomToken(48);
  const challenge = base64url(new Uint8Array(await crypto.subtle.digest("SHA-256", encoder.encode(verifier))));
  const callback = new URL("/github/callback", requestUrl.origin).href;
  const payload = await seal({
    state,
    verifier,
    returnTo: safeReturnTo(requestUrl.searchParams.get("returnTo"), env),
    expiresAt: Date.now() + OAUTH_TTL * 1000
  }, env.SESSION_SECRET);
  const authorize = new URL("https://github.com/login/oauth/authorize");
  authorize.searchParams.set("client_id", env.GITHUB_CLIENT_ID);
  authorize.searchParams.set("redirect_uri", callback);
  authorize.searchParams.set("state", state);
  authorize.searchParams.set("code_challenge", challenge);
  authorize.searchParams.set("code_challenge_method", "S256");
  return new Response(null, {
    status: 302,
    headers: {
      Location: authorize.href,
      "Set-Cookie": cookie(OAUTH_COOKIE, payload, { maxAge: OAUTH_TTL }),
      "Cache-Control": "no-store"
    }
  });
}

async function oauthCallback(request, env) {
  if (!configured(env)) return json(request, env, { error: "GITHUB_AUTH_NOT_CONFIGURED" }, 503);
  const requestUrl = new URL(request.url);
  const oauth = await unseal(cookies(request)[OAUTH_COOKIE] || "", env.SESSION_SECRET);
  if (!oauth || requestUrl.searchParams.get("state") !== oauth.state) {
    return json(request, env, { error: "GITHUB_OAUTH_STATE" }, 400, {
      "Set-Cookie": clearCookie(OAUTH_COOKIE)
    });
  }
  const tokenResponse = await fetch("https://github.com/login/oauth/access_token", {
    method: "POST",
    headers: { Accept: "application/json", "Content-Type": "application/json" },
    body: JSON.stringify({
      client_id: env.GITHUB_CLIENT_ID,
      client_secret: env.GITHUB_CLIENT_SECRET,
      code: requestUrl.searchParams.get("code"),
      code_verifier: oauth.verifier,
      redirect_uri: new URL("/github/callback", requestUrl.origin).href
    })
  });
  const tokenValue = await tokenResponse.json();
  if (!tokenResponse.ok || !tokenValue.access_token) {
    return json(request, env, { error: "GITHUB_TOKEN_EXCHANGE" }, 502, {
      "Set-Cookie": clearCookie(OAUTH_COOKIE)
    });
  }
  const profileResponse = await fetch(`${GITHUB_API}/user`, {
    headers: githubHeaders(tokenValue.access_token)
  });
  const profileValue = await profileResponse.json();
  if (!profileResponse.ok || !profileValue.login) {
    return json(request, env, { error: "GITHUB_PROFILE" }, 502, {
      "Set-Cookie": clearCookie(OAUTH_COOKIE)
    });
  }
  const profile = {
    login: profileValue.login,
    name: profileValue.name || profileValue.login,
    avatarUrl: profileValue.avatar_url || "",
    htmlUrl: profileValue.html_url || `https://github.com/${profileValue.login}`
  };
  const session = await seal({
    accessToken: tokenValue.access_token,
    profile,
    expiresAt: Date.now() + COOKIE_TTL * 1000
  }, env.SESSION_SECRET);
  return new Response(null, {
    status: 302,
    headers: [
      ["Location", oauth.returnTo],
      ["Set-Cookie", cookie(SESSION_COOKIE, session)],
      ["Set-Cookie", clearCookie(OAUTH_COOKIE)],
      ["Cache-Control", "no-store"]
    ]
  });
}

function githubHeaders(token) {
  return {
    Accept: "application/vnd.github+json",
    Authorization: `Bearer ${token}`,
    "X-GitHub-Api-Version": "2022-11-28",
    "User-Agent": "Hara-Studio"
  };
}

async function sessionFor(request, env) {
  if (!configured(env)) return null;
  return unseal(cookies(request)[SESSION_COOKIE] || "", env.SESSION_SECRET);
}

async function sessionResponse(request, env) {
  if (!configured(env)) {
    return json(request, env, { authenticated: false, configured: false, profile: null });
  }
  const session = await sessionFor(request, env);
  return json(request, env, {
    authenticated: Boolean(session),
    configured: true,
    profile: session?.profile || null
  });
}

async function githubProxy(request, env, pathname) {
  const session = await sessionFor(request, env);
  if (!session) return json(request, env, { error: "GITHUB_AUTH_REQUIRED" }, 401);
  if (!/^\/github\/gists(?:\/[A-Za-z0-9]+)?$/.test(pathname) ||
      !["GET", "POST", "PATCH"].includes(request.method)) {
    return json(request, env, { error: "GITHUB_ROUTE_NOT_ALLOWED" }, 404);
  }
  const target = `${GITHUB_API}${pathname.replace(/^\/github/, "")}`;
  const response = await fetch(target, {
    method: request.method,
    headers: {
      ...githubHeaders(session.accessToken),
      ...(request.method === "GET" ? {} : { "Content-Type": "application/json" })
    },
    body: request.method === "GET" ? undefined : request.body
  });
  return new Response(response.body, {
    status: response.status,
    headers: {
      ...corsHeaders(request, env),
      "Cache-Control": "no-store",
      "Content-Type": response.headers.get("Content-Type") || "application/json"
    }
  });
}

function allowedBrowserRequest(request, env) {
  const origin = requestOrigin(request);
  return allowedOrigins(env).has(origin) && request.headers.get("X-Hara-Request");
}

async function handle(request, env) {
  const url = new URL(request.url);
  if (request.method === "OPTIONS") {
    return new Response(null, { status: 204, headers: corsHeaders(request, env) });
  }
  if (url.pathname === "/github/start" && request.method === "GET") return oauthStart(request, env);
  if (url.pathname === "/github/callback" && request.method === "GET") return oauthCallback(request, env);
  if (url.pathname === "/session" && request.method === "GET") return sessionResponse(request, env);
  if (url.pathname === "/logout" && request.method === "POST") {
    if (!allowedBrowserRequest(request, env)) return json(request, env, { error: "ORIGIN_NOT_ALLOWED" }, 403);
    return json(request, env, { ok: true }, 200, { "Set-Cookie": clearCookie(SESSION_COOKIE) });
  }
  if (url.pathname.startsWith("/github/")) {
    if (!allowedBrowserRequest(request, env)) return json(request, env, { error: "ORIGIN_NOT_ALLOWED" }, 403);
    return githubProxy(request, env, url.pathname);
  }
  return json(request, env, { error: "NOT_FOUND" }, 404);
}

export const internals = { configured, seal, unseal, safeReturnTo };

export default {
  async fetch(request, env) {
    try {
      return await handle(request, env);
    } catch (error) {
      console.error(JSON.stringify({
        message: "auth request failed",
        path: new URL(request.url).pathname,
        error: error instanceof Error ? error.message : String(error)
      }));
      return json(request, env, { error: "INTERNAL_ERROR" }, 500);
    }
  }
};
