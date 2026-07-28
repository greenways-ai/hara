import { createBrowserBroker } from "./studio/broker.js";
import { createHostServices } from "./studio/host-services.js";
import { GraphHost } from "./studio/graph-host.js";
import { SessionRouter } from "./studio/session-router.js";
import { CapabilityRegistry } from "./studio/capability-registry.js";
import { createCanvasCapability } from "./studio/capabilities/canvas.js";
import { createClockCapability } from "./studio/capabilities/clock.js";
import { CanvasRuntime } from "./studio/canvas-runtime.js";
import { mountStudio } from "./studio/ui.js";

// Smoke-page bootstrap: same wiring as the website's studio.js, with paths
// resolved against the repo-root server used by playwright.config.js.
const bytes = new Uint8Array(
  await (await fetch("/rust/raw/target/wasm32-unknown-unknown/release/hara_wasm_raw.wasm")).arrayBuffer()
);
const resources = {};
for (const name of ["store", "fs", "space", "boot", "node", "draw", "program", "graph", "session"]) {
  resources[`studio.${name}`] = await (await fetch(`./studio/hal/${name}.hal`)).text();
}
for (const name of ["protocol", "frame"]) {
  resources[`std.substrate.${name}`] = await (await fetch(`../../lib/src/std/substrate/${name}.hal`)).text();
}
resources["std.substrate"] = await (await fetch("../../lib/src/std/substrate.hal")).text();
const sessionRouter = new SessionRouter();
const canvasRuntime = new CanvasRuntime();
const capabilityRegistry = new CapabilityRegistry({ adapters: {
  "surface/canvas-2d": createCanvasCapability(canvasRuntime),
  "clock/frame": createClockCapability()
} });
const graphHost = new GraphHost({ workerUrl: "./studio/program-worker.js", sessionRouter, capabilityRegistry });
const broker = createBrowserBroker({
  workerUrl: "./hta-worker.js",
  moduleBytes: bytes,
  hostCalls: createHostServices({ canvasRuntime, graphHost, graphHostOptions: { sessionRouter } }),
  resources,
  onKernelCreated: async (kernel) => sessionRouter.register(kernel.name, kernel.context, {
    onRelease: (sessionId) => graphHost.releaseSession(sessionId)
  }),
  onKernelClosed: (kernel) => sessionRouter.unregister(kernel.name)
});
window.studio = mountStudio(document.getElementById("hara-studio-mount"), { broker });
