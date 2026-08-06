import { HtaKeyword, loadHtaExtension } from "./hta.js";

const descriptorUrl = new URL(
  "../extensions/ledger-noir/target/package/ledger/noir/hara.extension.edn",
  import.meta.url
).toString();
const context = await loadHtaExtension({ descriptorUrl });
window.noirContext = context;
window.noirSmoke = (async () => {
  const artifact = await context.call("compile", [map({
    name: "proof_demo",
    source: "fn main(secret: Field, expected: pub Field) { assert(secret * secret == expected); }"
  })]);
  const proof = await context.call("prove", [artifact, map({ secret: "7", expected: "49" })]);
  const verified = await context.call("verify", [artifact, proof]);
  return {
    identity: context.manifest.identity,
    namespace: context.manifest.namespace,
    artifact: field(artifact, "format"),
    proof: field(proof, "format"),
    verified
  };
})();

function map(value) {
  return new Map(Object.entries(value).map(([key, item]) => [new HtaKeyword(key), item]));
}

function field(value, name) {
  for (const [key, item] of value) if (key instanceof HtaKeyword && key.name === name) return item;
}
