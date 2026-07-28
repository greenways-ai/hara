/**
 * Host-side verified package cache. The WASM evaluator is intentionally never
 * given fetch authority; callers register returned bytes before evaluation.
 */
const encoder = new TextEncoder();

export async function sha256(bytes) {
  const digest = await crypto.subtle.digest("SHA-256", bytes);
  return `sha256:${[...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, "0")).join("")}`;
}

export async function verifyEd25519({ publicKey, statement, signature }) {
  const key = await crypto.subtle.importKey("raw", publicKey, { name: "Ed25519" }, false, ["verify"]);
  return crypto.subtle.verify({ name: "Ed25519" }, key, signature, encoder.encode(statement));
}

export async function fetchVerifiedPackage({ url, digest, publisher, registry, fetchImpl = fetch, cache }) {
  const cacheKey = `hara-package/${digest}`;
  const cached = cache && await cache.match(cacheKey);
  const response = cached || await fetchImpl(url);
  if (!response?.ok) throw new Error(`package/fetch-failed: ${response?.status ?? "network"}`);
  const bytes = await response.arrayBuffer();
  if (await sha256(bytes) !== digest) throw new Error("package/digest-mismatch");
  for (const signature of [publisher, registry]) {
    if (!signature || !await verifyEd25519(signature)) throw new Error("package/signature-invalid");
  }
  if (!cached && cache) await cache.put(cacheKey, new Response(bytes, { headers: { "content-type": "application/vnd.hara.harp" } }));
  return bytes;
}
