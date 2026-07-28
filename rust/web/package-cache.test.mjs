import assert from "node:assert/strict";
import test from "node:test";
import { webcrypto } from "node:crypto";

globalThis.crypto ??= webcrypto;

import { fetchVerifiedPackage, sha256 } from "./package-cache.js";

async function signaturePair(statement) {
  const pair = await crypto.subtle.generateKey({ name: "Ed25519" }, true, ["sign", "verify"]);
  return {
    publicKey: await crypto.subtle.exportKey("raw", pair.publicKey),
    statement,
    signature: await crypto.subtle.sign({ name: "Ed25519" }, pair.privateKey, new TextEncoder().encode(statement))
  };
}

test("verified packages require both detached signatures and the declared digest", async () => {
  const bytes = new TextEncoder().encode("deterministic harp").buffer;
  const publisher = await signaturePair("publisher intent");
  const registry = await signaturePair("registry attestation");
  const cache = new Map();
  const response = { ok: true, arrayBuffer: async () => bytes };
  cache.match = async (key) => cache.get(key);
  cache.put = async (key, value) => cache.set(key, value);
  const result = await fetchVerifiedPackage({
    url: "https://github.example/release.harp",
    digest: await sha256(bytes), publisher, registry, cache,
    fetchImpl: async () => response
  });
  assert.deepEqual(new Uint8Array(result), new Uint8Array(bytes));
});

test("a missing or invalid attestation rejects before cache insertion", async () => {
  const bytes = new TextEncoder().encode("archive").buffer;
  const publisher = await signaturePair("publisher intent");
  await assert.rejects(
    fetchVerifiedPackage({
      url: "https://github.example/release.harp", digest: await sha256(bytes), publisher,
      fetchImpl: async () => ({ ok: true, arrayBuffer: async () => bytes })
    }),
    /signature-invalid/
  );
});
