import { HtaKeyword } from "./index.js";

export function toHta(value) {
  if (value === null || value === undefined || typeof value !== "object") {
    return value ?? null;
  }
  if (Array.isArray(value)) return value.map(toHta);
  const result = new Map();
  for (const [key, item] of Object.entries(value)) {
    result.set(new HtaKeyword(key), toHta(item));
  }
  return result;
}

export function providerError(error, origin, fallbackCode = "provider/error") {
  const message = String(error?.message ?? error);
  const separator = message.indexOf(":");
  const code = separator > 0 ? message.slice(0, separator) : fallbackCode;
  return new Map([
    [new HtaKeyword("code"), new HtaKeyword(code)],
    [new HtaKeyword("message"), message],
    [new HtaKeyword("origin"), new HtaKeyword(origin)],
    [new HtaKeyword("retryable"), false]
  ]);
}
