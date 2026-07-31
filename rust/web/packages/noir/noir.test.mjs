import assert from "node:assert/strict";
import test from "node:test";
import { callNoir } from "./index.js";

const loaderUrl = new URL("./test/mock-loader.mjs", import.meta.url).toString();

test("Noir preserves compile, proof, and verification envelopes", async () => {
  const artifact = await callNoir(loaderUrl, "compile", [
    new Map([
      [{ name: "name" }, "balance"],
      [{ name: "source" }, "fn main() {}"]
    ])
  ]);

  assert.equal(artifact.format, "hara/ledger.noir/v1");
  assert.equal(artifact.programKey, "program:balance");
  assert.equal(artifact.compilerVersion, "test-noir");
  assert.deepEqual(JSON.parse(artifact.circuitJson), { source: "fn main() {}" });

  const proof = await callNoir(loaderUrl, "prove", [artifact, { amount: 7 }]);
  assert.equal(proof.format, "hara.noir.proof/v1");
  assert.deepEqual(proof.publicInputs, [7]);
  assert.equal(await callNoir(loaderUrl, "verify", [artifact, proof]), true);
});
