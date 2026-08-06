import assert from "node:assert/strict";
import test from "node:test";
import "fake-indexeddb/auto";

import {
  BACKEND_ID,
  MemoryArtifactCache,
  NoirBrowserLoader,
  NOIR_VERSION
} from "./dist/noir-loader.js";

const program = {
  name: "first_cut",
  source: "fn main(value: u8) { assert(value > 0); }",
  noirVersion: NOIR_VERSION,
  backendVersion: BACKEND_ID
};

test("program identity matches the Truffle cache-key vector", async () => {
  const loader = new NoirBrowserLoader({ cache: new MemoryArtifactCache() });
  assert.equal(
    await loader.cacheKey(program),
    "6db358723409593229a47cd5271d5073dd67f38c55bc0a643857afb2206e8c5f"
  );
});

test("the browser WASM compiler produces and caches ACIR", async () => {
  const loader = new NoirBrowserLoader({ cache: new MemoryArtifactCache() });
  const artifact = await loader.compile(program);
  const cached = await loader.compile(program);

  assert.equal(artifact.format, "hara/ledger.noir/v1");
  assert.equal(artifact.programKey, await loader.cacheKey(program));
  assert.deepEqual(Object.keys(artifact.circuit), ["program", "warnings"]);
  assert.strictEqual(cached, artifact);
});

test("the WASM backend proves and verifies a private square root", { timeout: 120_000 }, async () => {
  const loader = new NoirBrowserLoader({ cache: new MemoryArtifactCache() });
  const artifact = await loader.compile({
    ...program,
    name: "proof_demo",
    source:
      "fn main(secret: Field, expected: pub Field) { assert(secret * secret == expected); }"
  });
  const proof = await loader.prove(artifact, { secret: "7", expected: "49" });

  assert.equal(proof.format, "hara.noir.proof/v1");
  assert.equal(proof.programKey, artifact.programKey);
  assert.equal(Buffer.from(proof.proof, "base64").length, 14_656);
  assert.deepEqual(proof.publicInputs, [
    "0x0000000000000000000000000000000000000000000000000000000000000031"
  ]);
  assert.equal(await loader.verify(artifact, proof), true);

  const tampered = {
    ...proof,
    proof:
      Buffer.from(proof.proof, "base64").map((byte, index) => index === 64 ? byte ^ 1 : byte)
        .toString("base64")
  };
  assert.equal(await loader.verify(artifact, tampered), false);
});

test("an invalid private witness cannot produce a proof", async () => {
  const loader = new NoirBrowserLoader({ cache: new MemoryArtifactCache() });
  const artifact = await loader.compile({
    ...program,
    name: "proof_demo",
    source:
      "fn main(secret: Field, expected: pub Field) { assert(secret * secret == expected); }"
  });

  await assert.rejects(
    loader.prove(artifact, { secret: "6", expected: "49" }),
    /Cannot satisfy constraint/
  );
});

test("compiler version mismatches fail closed", async () => {
  const loader = new NoirBrowserLoader({ cache: new MemoryArtifactCache() });
  await assert.rejects(
    loader.compile({ ...program, noirVersion: "1.0.0-beta.24" }),
    /noir\/version-mismatch/
  );
});
