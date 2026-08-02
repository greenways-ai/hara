import assert from "node:assert/strict";
import { generateKeyPairSync, sign } from "node:crypto";
import test from "node:test";
import { dispatchVerifiedPublication, verifyPublication } from "../lib/publication.mjs";

const intent = '{:intent/format 3 :intent/kind :package :intent/source "gh:alice/graph" :intent/coordinate "hara:alice/graph" :intent/version "1.0.0" :intent/repository-id 1234 :intent/commit "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" :intent/recipe-sha256 "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb" :intent/identity-revision "cccccccccccccccccccccccccccccccccccccccc"}\n';

function fixture() {
  const { privateKey, publicKey } = generateKeyPairSync("ed25519");
  const rawPublicKey = publicKey.export({ format: "der", type: "spki" }).subarray(-32).toString("hex");
  const signature = sign(null, Buffer.from(intent), privateKey).toString("hex");
  return {
    request: { intent, keyId: "key-1", signature },
    policy: { publisherKeys: { "key-1": { publicKey: rawPublicKey, coordinates: ["hara:alice/graph"], revoked: false } } },
  };
}

test("accepts an authorized publisher signature", () => {
  const { request, policy } = fixture();
  assert.deepEqual(verifyPublication(request, policy), { coordinate: "hara:alice/graph", keyId: "key-1" });
});

test("does not dispatch GitHub when the signature does not match", async () => {
  const { request, policy } = fixture();
  request.intent = request.intent.replace('"1.0.0"', '"1.0.1"');
  let dispatched = false;
  await assert.rejects(
    dispatchVerifiedPublication(request, policy, async () => { dispatched = true; }),
    /signature does not match/,
  );
  assert.equal(dispatched, false);
});

test("does not dispatch GitHub for revoked or cross-namespace keys", async () => {
  const { request, policy } = fixture();
  policy.publisherKeys["key-1"].revoked = true;
  await assert.rejects(dispatchVerifiedPublication(request, policy, async () => assert.fail("dispatched")), /revoked/);
  policy.publisherKeys["key-1"].revoked = false;
  policy.publisherKeys["key-1"].coordinates = ["hara:bob/graph"];
  await assert.rejects(dispatchVerifiedPublication(request, policy, async () => assert.fail("dispatched")), /not authorized/);
});
