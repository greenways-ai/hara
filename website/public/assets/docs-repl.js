import { createDocsKernel } from "/docs-assets/javascripts/kernel.js";

const toast = document.createElement("div");
toast.className = "hara-kernel-toast";
toast.setAttribute("role", "status");
toast.setAttribute("aria-live", "polite");
toast.innerHTML = `<i></i><span>Preparing hara-wasm-vm</span><b>0%</b>`;
document.body.append(toast);

let loaded = 0;
let expected = 0;
const report = (message) => {
  const percent = expected ? Math.min(99, Math.round(loaded / expected * 100)) : 0;
  toast.querySelector("span").textContent = message;
  toast.querySelector("b").textContent = `${percent}%`;
  toast.style.setProperty("--kernel-progress", `${percent}%`);
};

const fetchWithProgress = async (input, init) => {
  const response = await fetch(input, init);
  const total = Number(response.headers.get("content-length")) || 0;
  expected += total;
  if (!response.body) return response;
  const reader = response.body.getReader();
  const stream = new ReadableStream({
    async pull(controller) {
      const { done, value } = await reader.read();
      if (done) { controller.close(); return; }
      loaded += value.byteLength;
      report("Loading hara-wasm-vm");
      controller.enqueue(value);
    }
  });
  return new Response(stream, { status: response.status, statusText: response.statusText, headers: response.headers });
};

const kernelPromise = fetch("/runtime/kernel-manifest.json")
  .then(async (response) => {
    if (!response.ok) throw new Error(`kernel manifest: ${response.status}`);
    const manifest = await response.json();
    return createDocsKernel({
      wasmUrl: manifest.variants.core.url,
      workerUrl: "/runtime/hta-worker.js",
      manifest,
      resources: {
        "studio.store": "/docs-assets/rust/studio/hal/store.hal",
        "studio.fs": "/docs-assets/rust/studio/hal/fs.hal"
      },
      fetchAsset: fetchWithProgress
    });
  }).then((kernel) => {
  toast.dataset.state = "ready";
  toast.querySelector("span").textContent = "hara-wasm-core ready";
  toast.querySelector("b").textContent = "100%";
  toast.style.setProperty("--kernel-progress", "100%");
  setTimeout(() => { toast.hidden = true; }, 2400);
  document.dispatchEvent(new CustomEvent("hara:kernel-ready", { detail: { artifact: "hara-wasm-core" } }));
  return kernel;
}).catch((error) => {
  toast.dataset.state = "error";
  toast.querySelector("span").textContent = "Kernel unavailable";
  toast.querySelector("b").textContent = "";
  console.error(error);
  throw error;
});

const print = (value) => {
  if (value === null) return "nil";
  if (typeof value === "string") return JSON.stringify(value);
  if (Array.isArray(value)) return `[${value.map(print).join(" ")}]`;
  if (value instanceof Map) return `{${[...value].map(([key, item]) => `${print(key)} ${print(item)}`).join(" ")}}`;
  return String(value);
};

let sequence = 0;
for (const frame of document.querySelectorAll("main [data-hara-eval]")) {
  const code = frame.querySelector("pre > code");
  if (!code) continue;
  const pre = code.parentElement;
  if (!pre || pre.closest(".hara-repl")) continue;
  const source = frame.dataset.haraSource
    ? decodeURIComponent(frame.dataset.haraSource)
    : code.textContent.replace(/\n$/, "");
  const cell = document.createElement("section");
  cell.className = "hara-repl";
  cell.innerHTML = `<header><span>Hara</span><small>hara-wasm-core · isolated session</small><button type="button">Run</button></header><textarea spellcheck="false"></textarea><output aria-live="polite">Kernel loading…</output>`;
  const editor = cell.querySelector("textarea");
  const output = cell.querySelector("output");
  const button = cell.querySelector("button");
  editor.value = source;
  editor.rows = Math.min(24, Math.max(2, source.split("\n").length));
  frame.replaceWith(cell);
  const sessionName = `docs-${location.pathname.replace(/\W+/g, "-")}-${++sequence}`;
  button.addEventListener("click", async () => {
    button.disabled = true;
    output.dataset.state = "pending";
    output.textContent = "Evaluating…";
    try {
      const kernel = await kernelPromise;
      const session = await kernel.createSession(sessionName, { filesystem: `memory:${sessionName}` });
      const result = await session.eval(editor.value);
      output.dataset.state = "ready";
      output.textContent = result.label ?? print(result.value);
    } catch (error) {
      output.dataset.state = "error";
      output.textContent = String(error?.message ?? error);
    } finally { button.disabled = false; }
  });
  editor.addEventListener("keydown", (event) => {
    if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
      event.preventDefault();
      button.click();
    }
  });
}
