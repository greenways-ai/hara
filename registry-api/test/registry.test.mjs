import test from "node:test";
import assert from "node:assert/strict";
import { requestFor } from "../netlify/functions/registry.mjs";

test("only permits canonical immutable registry sources", () => {
  assert.match(requestFor({ queryStringParameters: { kind: "identity", ref: "a".repeat(40) } }), /hara-lang\/hara-identity/);
  assert.throws(() => requestFor({ queryStringParameters: { kind: "packages", ref: "main" } }));
  assert.throws(() => requestFor({ queryStringParameters: { kind: "registry", ref: "../../etc/passwd" } }));
});
