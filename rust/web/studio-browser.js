import { createBrowserBroker } from "./studio/broker.js";
import { createHostServices } from "./studio/host-services.js";
import { mountStudio } from "./studio/ui.js";

// Smoke-page bootstrap: same wiring as the website's studio.js, with paths
// resolved against the repo-root server used by playwright.config.js.
const bytes = new Uint8Array(
  await (await fetch("/rust/raw/target/wasm32-unknown-unknown/release/hara_wasm_raw.wasm")).arrayBuffer()
);
const resources = {};
for (const name of ["store", "fs", "space", "boot", "node", "draw"]) {
  resources[`studio.${name}`] = await (await fetch(`./studio/hal/${name}.hal`)).text();
}
const broker = createBrowserBroker({
  workerUrl: "./hta-worker.js",
  moduleBytes: bytes,
  hostCalls: createHostServices(),
  resources
});
window.studio = mountStudio(document.getElementById("hara-studio-mount"), { broker });
