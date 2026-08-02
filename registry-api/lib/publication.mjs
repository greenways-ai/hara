import { createPublicKey, verify } from "node:crypto";

const ED25519_SPKI_PREFIX = Buffer.from("302a300506032b6570032100", "hex");

export function verifyPublication({ intent, keyId, signature }, policy) {
  if (typeof intent !== "string" || !intent.endsWith("\n")) {
    throw new Error("publication intent must be canonical newline-terminated UTF-8");
  }
  if (!/^[0-9a-f]{128}$/.test(signature ?? "")) {
    throw new Error("publication signature must be 64-byte lowercase hex");
  }
  const key = policy?.publisherKeys?.[keyId];
  if (!key) throw new Error(`publisher key is not authorized: ${keyId}`);
  if (key.revoked) throw new Error(`publisher key is revoked: ${keyId}`);
  if (!/^[0-9a-f]{64}$/.test(key.publicKey ?? "")) {
    throw new Error(`publisher key is invalid: ${keyId}`);
  }
  const coordinate = ednString(intent, "intent/coordinate");
  if (!key.coordinates?.includes(coordinate)) {
    throw new Error(`publisher key is not authorized for ${coordinate}`);
  }
  const publicKey = createPublicKey({
    key: Buffer.concat([ED25519_SPKI_PREFIX, Buffer.from(key.publicKey, "hex")]),
    format: "der",
    type: "spki",
  });
  if (!verify(null, Buffer.from(intent, "utf8"), publicKey, Buffer.from(signature, "hex"))) {
    throw new Error("publication signature does not match publisher key");
  }
  return { coordinate, keyId };
}

export async function dispatchVerifiedPublication(request, policy, dispatch) {
  const verified = verifyPublication(request, policy);
  return dispatch({ ...request, verified });
}

function ednString(source, keyword) {
  const match = source.match(new RegExp(`:${keyword.replace("/", "\\/")}\\s+\"([^\"]+)\"`));
  if (!match) throw new Error(`publication intent is missing :${keyword}`);
  return match[1];
}
