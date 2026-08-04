/**
 * Generalized Hara WASM kernel boot, extracted from the docs REPL
 * (website/public/assets/docs-repl.js `createKernelPromise`).
 *
 * The kernel module itself (`createDocsKernel`) is NOT vendored into this
 * package. It is resolved in one of two ways:
 *
 *   - `createKernel`: an injected factory with the same signature as
 *     `createDocsKernel` — preferred for consumers that vendor their own copy.
 *   - `kernelModuleUrl`: dynamically imported and its `createDocsKernel`
 *     export used. Defaults to the docs-assets path served by hara-lang.org.
 *
 * Boot promises are cached per page, keyed on the resolved configuration, so
 * multiple live cards share one kernel. A failed boot evicts its cache entry
 * so a later call can retry.
 */

function defaultResources(runtimeBase, docsAssetsBase) {
  return {
    "studio.store": `${docsAssetsBase}/rust/studio/hal/store.hal`,
    "studio.fs": `${docsAssetsBase}/rust/studio/hal/fs.hal`,
    "studio.node": `${runtimeBase}/studio/hal/node.hal`,
    "studio.draw": `${runtimeBase}/studio/hal/draw.hal`,
    "std.lib.substrate.frame": `${runtimeBase}/std/lib/substrate/frame.hal`
  };
}

/**
 * Wrap a fetch implementation so kernel asset downloads report byte-level
 * progress: `onProgress(message, percent)` with percent in [0, 99].
 */
export function createProgressFetch(onProgress, baseFetch = fetch) {
  if (typeof onProgress !== "function") return baseFetch;
  let loaded = 0;
  let expected = 0;
  return async (input, init) => {
    const response = await baseFetch(input, init);
    const total = Number(response.headers.get("content-length")) || 0;
    expected += total;
    if (!response.body) return response;
    const reader = response.body.getReader();
    const stream = new ReadableStream({
      async pull(controller) {
        const { done, value } = await reader.read();
        if (done) { controller.close(); return; }
        loaded += value.byteLength;
        const percent = expected ? Math.min(99, Math.round(loaded / expected * 100)) : 0;
        onProgress("Loading Hara kernel", percent);
        controller.enqueue(value);
      }
    });
    return new Response(stream, {
      status: response.status,
      statusText: response.statusText,
      headers: response.headers
    });
  };
}

async function resolveKernelFactory({ createKernel, kernelModuleUrl }) {
  if (createKernel) return createKernel;
  const module = await import(kernelModuleUrl);
  if (typeof module.createDocsKernel !== "function") {
    throw new Error(`kernel module ${kernelModuleUrl} does not export createDocsKernel`);
  }
  return module.createDocsKernel;
}

const bootCache = new Map();

/**
 * Boot (or return the in-flight boot of) the shared page kernel.
 * Returns a promise for a kernel facade with `createSession(name)` — see
 * docs/docs/javascripts/kernel.js for the full facade contract.
 */
export function createLiveKernel({
  runtimeBase = "/runtime",
  docsAssetsBase = "/docs-assets",
  kernelModuleUrl = null,
  createKernel = null,
  manifestUrl = null,
  workerUrl = null,
  resources = null,
  fetchAsset = null,
  onProgress = null
} = {}) {
  const resolved = {
    runtimeBase,
    docsAssetsBase,
    kernelModuleUrl: kernelModuleUrl ?? `${docsAssetsBase}/javascripts/kernel.js`,
    manifestUrl: manifestUrl ?? `${runtimeBase}/kernel-manifest.json`,
    workerUrl: workerUrl ?? `${runtimeBase}/hta-worker.js`,
    resources: resources ?? defaultResources(runtimeBase, docsAssetsBase)
  };
  const cacheKey = createKernel ? null : JSON.stringify(resolved);
  if (cacheKey && bootCache.has(cacheKey)) return bootCache.get(cacheKey);

  const progressFetch = createProgressFetch(onProgress, fetchAsset ?? fetch);
  const boot = Promise.resolve()
    .then(() => progressFetch(resolved.manifestUrl))
    .then(async (response) => {
      if (!response.ok) throw new Error(`kernel manifest: ${response.status}`);
      const manifest = await response.json();
      const factory = await resolveKernelFactory({ createKernel, kernelModuleUrl: resolved.kernelModuleUrl });
      return factory({
        wasmUrl: manifest.variants.core.url,
        workerUrl: resolved.workerUrl,
        manifest,
        resources: resolved.resources,
        fetchAsset: progressFetch
      });
    });

  if (cacheKey) {
    bootCache.set(cacheKey, boot);
    boot.catch(() => {
      if (bootCache.get(cacheKey) === boot) bootCache.delete(cacheKey);
    });
  }
  return boot;
}

/** Test hook: drop all cached kernel boot promises. */
export function resetLiveKernelCache() {
  bootCache.clear();
}
