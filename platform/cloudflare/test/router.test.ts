import assert from "node:assert/strict";
import test from "node:test";
import { objectKey, repositoryRequest } from "../src/router.ts";

const env = {
  IDENTITY_REPOSITORY: "hara-lang/hara-identity",
  PACKAGES_REPOSITORY: "hara-lang/hara-packages",
} as Env;

test("Git reads are restricted to canonical documents and refs", () => {
  assert.match(repositoryRequest("identity", "a".repeat(40), env).url, /hara-identity/);
  assert.match(repositoryRequest("packages", "main", env).url, /registry\.edn/);
  assert.throws(() => repositoryRequest("packages", "../../main", env));
});

test("object paths are content-addressed only", () => {
  assert.equal(objectKey(`/objects/sha256/${"a".repeat(64)}`), `sha256/${"a".repeat(64)}`);
  assert.equal(objectKey("/objects/latest/package.zip"), null);
  assert.equal(objectKey(`/objects/sha256/${"g".repeat(64)}`), null);
});
