/* Hara Studio bootstrap (live wasm runtime).

   Fetches the raw HTA wasm module and the four studio hal resources, creates
   the kernel broker with the browser host services, and mounts the shared
   studio UI (rust/web/studio/ui.js) into the mount point in studio.html.

   All paths resolve against the site root derived from import.meta.url, so
   the module works under any base path (the site deploys to a project page,
   e.g. /hara.lang/, where root-absolute "/rust/..." URLs would break).
   Deploy layout (see .github/workflows/pages.yml):
     /rust/hara.wasm            raw wasm module (hara_wasm_raw.wasm)
     /rust/hta.js               HTA codec + HtaContext
     /rust/hta-worker.js        kernel worker script
     /rust/studio/*.js          broker, host services, boot template, UI
     /rust/studio/studio.css    studio styles (@imported by hara.css)
     /rust/studio/hal/*.hal     store/fs/space/boot resources              */

const siteRoot = new URL("../", import.meta.url);
const asset = (path) => new URL(path, siteRoot);

const mount = document.getElementById("hara-studio-mount");

function fail(error) {
  const message = String(error?.message ?? error);
  const shell = document.createElement("div");
  shell.className = "hara-studio";
  const head = document.createElement("div");
  head.className = "hara-studio-head";
  const kicker = document.createElement("span");
  kicker.className = "hara-kicker";
  kicker.textContent = "HARA STUDIO";
  const index = document.createElement("span");
  index.className = "hara-index";
  index.textContent = "BOOT FAILED";
  head.append(kicker, index);
  const strip = document.createElement("div");
  strip.className = "hara-strip hara-studio-status";
  const runtime = document.createElement("span");
  const bold = document.createElement("b");
  bold.textContent = `WASM · ERROR — ${message}`;
  runtime.append("RUNTIME ", bold);
  strip.appendChild(runtime);
  shell.append(head, strip);
  mount.replaceChildren(shell);
}

async function fetchText(url) {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`fetch ${url.pathname} failed: ${response.status}`);
  return response.text();
}

try {
  const [{ createBrowserBroker }, { createHostServices }, { mountStudio }] = await Promise.all([
    import(asset("rust/studio/broker.js").href),
    import(asset("rust/studio/host-services.js").href),
    import(asset("rust/studio/ui.js").href)
  ]);

  const wasmResponse = await fetch(asset("rust/hara.wasm"));
  if (!wasmResponse.ok) throw new Error(`fetch hara.wasm failed: ${wasmResponse.status}`);
  const moduleBytes = new Uint8Array(await wasmResponse.arrayBuffer());

  const resources = {};
  for (const name of ["store", "fs", "space", "boot"]) {
    resources[`studio.${name}`] = await fetchText(asset(`rust/studio/hal/${name}.hal`));
  }

  const broker = createBrowserBroker({
    workerUrl: asset("rust/hta-worker.js"),
    moduleBytes,
    hostCalls: createHostServices(),
    resources
  });

  mountStudio(mount, { broker });
} catch (error) {
  fail(error);
}
