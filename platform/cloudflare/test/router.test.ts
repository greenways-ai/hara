import assert from "node:assert/strict";
import test from "node:test";
import { objectKey, repositoryRequest, route } from "../src/router.ts";

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

test("both public services expose the shared Hara favicon", async () => {
  for (const host of ["id.hara-lang.org", "packages.hara-lang.org"]) {
    const response = await route(new Request(`https://${host}/favicon.ico`), env);
    assert.equal(response.status, 200);
    assert.equal(response.headers.get("content-type"), "image/svg+xml");
    assert.match(await response.text(), /M10 8h13v18h18V8h13/);
  }
});
