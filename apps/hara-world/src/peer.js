const encoder = new TextEncoder();

export function randomToken(bytes = 18) {
  const value = crypto.getRandomValues(new Uint8Array(bytes));
  return base64url(value);
}

export function parseInvite() {
  const values = new URLSearchParams(location.hash.slice(1));
  return {
    room: values.get("room"),
    key: values.get("key"),
    role: values.get("role")
  };
}

export function inviteUrl(room, key) {
  const url = new URL(location.href);
  url.hash = new URLSearchParams({ room, key, role: "join" });
  return url.toString();
}

export async function proof(secret, nonce, peer) {
  const key = await crypto.subtle.importKey(
    "raw",
    fromBase64url(secret),
    { name: "HMAC", hash: "SHA-256" },
    false,
    ["sign"]
  );
  const signature = await crypto.subtle.sign(
    "HMAC",
    key,
    encoder.encode(`hara-world-v1:${nonce}:${peer}`)
  );
  return base64url(new Uint8Array(signature));
}

export function sameProof(left, right) {
  const a = encoder.encode(left ?? "");
  const b = encoder.encode(right ?? "");
  if (a.length !== b.length) return false;
  let difference = 0;
  for (let index = 0; index < a.length; index++) difference |= a[index] ^ b[index];
  return difference === 0;
}

function base64url(bytes) {
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary).replaceAll("+", "-").replaceAll("/", "_").replace(/=+$/, "");
}

function fromBase64url(value) {
  const base64 = value.replaceAll("-", "+").replaceAll("_", "/");
  const padded = base64.padEnd(Math.ceil(base64.length / 4) * 4, "=");
  const binary = atob(padded);
  return Uint8Array.from(binary, character => character.charCodeAt(0));
}
