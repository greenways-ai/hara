import assert from "node:assert/strict";
import test from "node:test";
import { pathToFileURL } from "node:url";
import {
  createLiveKernel,
  createProgressFetch,
  resetLiveKernelCache
} from "../src/kernel.js";

const fixtureModuleUrl = pathToFileURL(
  new URL("./fixtures/fake-kernel-module.js", import.meta.url).pathname
).href;

const manifestResponse = () => new Response(JSON.stringify({
  variants: { core: { url: "https://example.test/hara.wasm" } }
}), { status: 200, headers: { "content-type": "application/json" } });

test("injected createKernel factory receives the resolved configuration", async () => {
  const configs = [];
  const kernel = await createLiveKernel({
    runtimeBase: "/rt",
    docsAssetsBase: "/assets",
    createKernel: async (config) => { configs.push(config); return { config }; },
    fetchAsset: async () => manifestResponse()
  });
  assert.equal(configs.length, 1);
  assert.equal(kernel.config.wasmUrl, "https://example.test/hara.wasm");
  assert.equal(kernel.config.workerUrl, "/rt/hta-worker.js");
  assert.equal(kernel.config.manifest.variants.core.url, "https://example.test/hara.wasm");
  assert.equal(kernel.config.resources["studio.node"], "/rt/studio/hal/node.hal");
  assert.equal(kernel.config.resources["studio.draw"], "/rt/studio/hal/draw.hal");
  assert.equal(kernel.config.resources["studio.store"], "/assets/rust/studio/hal/store.hal");
  assert.equal(kernel.config.resources["std.lib.substrate.frame"], "/rt/std/lib/substrate/frame.hal");
});

test("kernelModuleUrl boots are cached per configuration", async () => {
  resetLiveKernelCache();
  const module = await import(fixtureModuleUrl);
  const before = module.boots;
  const options = {
    kernelModuleUrl: fixtureModuleUrl,
    fetchAsset: async () => manifestResponse()
  };
  const [first, second] = await Promise.all([
    createLiveKernel(options),
    createLiveKernel(options)
  ]);
  assert.equal(module.boots, before + 1, "kernel module should boot once");
  assert.equal(first, second);
});

test("a failed boot is evicted so a later call retries", async () => {
  resetLiveKernelCache();
  let fetches = 0;
  const options = {
    kernelModuleUrl: fixtureModuleUrl,
    fetchAsset: async () => {
      fetches += 1;
      return fetches === 1
        ? new Response("missing", { status: 404 })
        : manifestResponse();
    }
  };
  await assert.rejects(createLiveKernel(options), /kernel manifest: 404/);
  const kernel = await createLiveKernel(options);
  assert.equal(fetches, 2);
  assert.ok(kernel.config);
});

test("createProgressFetch reports byte-level progress", async () => {
  const events = [];
  const body = new TextEncoder().encode("kernel-bytes");
  const progress = createProgressFetch((message, percent) => events.push([message, percent]),
    async () => new Response(body, {
      status: 200,
      headers: { "content-length": String(body.byteLength) }
    }));
  const response = await progress("https://example.test/hara.wasm");
  const text = await response.text();
  assert.equal(text, "kernel-bytes");
  assert.deepEqual(events, [["Loading Hara kernel", 99]]);
});

test("createProgressFetch passes through without an onProgress callback", async () => {
  const base = async () => new Response("ok", { status: 200 });
  assert.equal(createProgressFetch(null, base), base);
});
