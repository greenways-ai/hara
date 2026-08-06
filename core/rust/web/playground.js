// The generated wasm-bindgen package is copied beside this file by the wasm build.
import init, { Runtime, version, target_profile } from "./pkg/hara_wasm.js";
import {
  BACKEND_ID,
  MemoryArtifactCache,
  NoirBrowserLoader,
  NOIR_VERSION
} from "./noir-loader.js";

const source = document.querySelector("#source");
const result = document.querySelector("#result");
const button = document.querySelector("#run");
const proveButton = document.querySelector("#prove");
const proofResult = document.querySelector("#proof-result");

await init();
const runtime = new Runtime();
runtime.install_memory_file_provider("/browser");
runtime.install_loopback_socket_provider();
result.textContent = `${version()} / ${target_profile()}\nfile=${runtime.file_supported()} socket=${runtime.socket_supported()}\nready`;
button.addEventListener("click", () => {
  try { result.textContent = runtime.eval(source.value); }
  catch (error) { result.textContent = `error: ${error}`; }
});

proveButton.addEventListener("click", async () => {
  proveButton.disabled = true;
  proofResult.textContent = "compiling circuit…";
  try {
    const loader = new NoirBrowserLoader({ cache: new MemoryArtifactCache() });
    const artifact = await loader.compile({
      name: "proof_demo",
      source:
        "fn main(secret: Field, expected: pub Field) { assert(secret * secret == expected); }",
      noirVersion: NOIR_VERSION,
      backendVersion: BACKEND_ID
    });
    proofResult.textContent = "generating proof for private secret 7…";
    const proof = await loader.prove(artifact, { secret: "7", expected: "49" });
    proofResult.textContent = "verifying proof…";
    const verified = await loader.verify(artifact, proof);
    const padding = proof.proof.endsWith("==") ? 2 : proof.proof.endsWith("=") ? 1 : 0;
    const proofBytes = proof.proof.length * 3 / 4 - padding;
    proofResult.textContent =
      `verified=${verified}\npublic expected=49\nproof=${proofBytes} bytes\nprogram=${proof.programKey}`;
  } catch (error) {
    proofResult.textContent = `proof error: ${error}`;
  } finally {
    proveButton.disabled = false;
  }
});
