const REQUIRED = [
  "program/id",
  "program/hash",
  "program/language",
  "program/source",
  "program/export",
  "program/capabilities"
];

const SUPPORTED = new Set(["javascript/module", "javascript/audio-worklet"]);

export class ProgramError extends Error {
  constructor(code, message, details = {}) {
    super(message);
    this.name = "ProgramError";
    this.code = code;
    Object.assign(this, details);
  }
}

/** Normalise a plain, Map, or HTA-decoded program descriptor at the browser
 * boundary. Internal code always works with stable string-keyed fields. */
export function normalizeProgramDescriptor(value, { maxSourceBytes = 1048576 } = {}) {
  const input = plain(value);
  if (!input || typeof input !== "object" || Array.isArray(input)) {
    throw new ProgramError("program/invalid", "program descriptor must be a map");
  }
  for (const key of REQUIRED) {
    if (input[key] === undefined) throw new ProgramError("program/field-missing", `program missing ${key}`);
  }
  const language = keywordName(input["program/language"]);
  if (!SUPPORTED.has(language)) {
    throw new ProgramError("program/language", `unsupported program language: ${language}`);
  }
  const source = input["program/source"];
  if (typeof source !== "string") throw new ProgramError("program/source", "program source must be a string");
  const bytes = new TextEncoder().encode(source).byteLength;
  if (bytes > maxSourceBytes) {
    throw new ProgramError("program/source-too-large", `program source exceeds ${maxSourceBytes} bytes`, {
      maxSourceBytes,
      sourceBytes: bytes
    });
  }
  const capabilities = [...asSet(input["program/capabilities"])].map(keywordName);
  return Object.freeze({
    id: String(input["program/id"]),
    hash: String(input["program/hash"]),
    language,
    source,
    exportName: String(input["program/export"]),
    capabilities: Object.freeze(capabilities.sort()),
    sourceMap: input["program/source-map"] ?? null,
    meta: plain(input["program/meta"] ?? {})
  });
}

export function normalizeNodeDescriptor(value) {
  const input = plain(value);
  if (!input || typeof input !== "object" || Array.isArray(input)) {
    throw new ProgramError("node/invalid", "node descriptor must be a map");
  }
  for (const key of ["node/id", "node/session", "node/program"]) {
    if (input[key] === undefined) throw new ProgramError("node/field-missing", `node missing ${key}`);
  }
  return Object.freeze({
    id: String(input["node/id"]),
    sessionId: String(input["node/session"]),
    programId: String(input["node/program"]),
    generation: Number(input["node/generation"] ?? 1),
    config: plain(input["node/config"] ?? {}),
    ports: plain(input["node/ports"] ?? {}),
    actions: plain(input["node/actions"] ?? {})
  });
}

export function assertCapabilities(program, allowed) {
  const grants = new Set([...asSet(allowed)].map(keywordName));
  for (const capability of program.capabilities) {
    if (!grants.has(capability)) {
      throw new ProgramError("program/capability-denied", `capability denied: ${capability}`, {
        programId: program.id,
        capability
      });
    }
  }
}

function asSet(value) {
  if (value instanceof Set) return value;
  if (Array.isArray(value)) return new Set(value);
  if (value == null) return new Set();
  throw new ProgramError("program/capabilities", "program capabilities must be a set or vector");
}

function keywordName(value) {
  if (typeof value === "string") return value.startsWith(":") ? value.slice(1) : value;
  if (value?.constructor?.name === "HtaKeyword") return value.name;
  return String(value);
}

function plain(value) {
  if (value instanceof Map) return Object.fromEntries([...value].map(([key, entry]) => [keywordName(key), plain(entry)]));
  if (value instanceof Set) return new Set([...value].map(plain));
  if (Array.isArray(value)) return value.map(plain);
  if (value && typeof value === "object" && !ArrayBuffer.isView(value)) {
    return Object.fromEntries(Object.entries(value).map(([key, entry]) => [key, plain(entry)]));
  }
  return value;
}
