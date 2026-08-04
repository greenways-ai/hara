import {
  applyParedit,
  barfForward,
  insertIndent,
  killToFormEnd,
  slurpForward,
  structuralAlign
} from "./editor.js";
import { highlightHara } from "./highlight.js";
import { createLiveKernel } from "./kernel.js";
import { LIVE_SNIPPETS } from "./snippets.js";

/** Print an evaluated kernel value the way the docs REPL does. */
export const print = (value) => {
  if (value === null) return "nil";
  if (typeof value === "string") return JSON.stringify(value);
  if (Array.isArray(value)) return `[${value.map(print).join(" ")}]`;
  if (value instanceof Map) return `{${[...value].map(([key, item]) => `${print(key)} ${print(item)}`).join(" ")}}`;
  return String(value);
};

const errorMessage = (error) => String(error?.message ?? error).replace(/^Error: /, "");

const CONNECTION_TEXT = {
  idle: "Idle",
  loading: "Connecting",
  ready: "Connected",
  busy: "Evaluating",
  error: "Unavailable"
};

/** Kernel progress toast, scoped to the card instead of document.body. */
function createCardToast(card) {
  const toast = document.createElement("div");
  toast.className = "hara-live-card-toast";
  toast.setAttribute("role", "status");
  toast.setAttribute("aria-live", "polite");
  toast.innerHTML = `<i></i><span>Preparing Hara kernel</span><b>0%</b>`;
  toast.hidden = true;
  card.append(toast);
  return {
    element: toast,
    show() { toast.hidden = false; },
    report(message, percent) {
      toast.querySelector("span").textContent = message;
      toast.querySelector("b").textContent = `${percent ?? 0}%`;
      toast.style.setProperty("--kernel-progress", `${percent ?? 0}%`);
    },
    fail(message) {
      toast.dataset.state = "error";
      toast.querySelector("span").textContent = message;
      toast.querySelector("b").textContent = "";
    },
    remove() { toast.remove(); }
  };
}

const localPointer = (event, canvas) => {
  const rect = canvas.getBoundingClientRect();
  return {
    type: "pointer",
    phase: event.type === "pointerup" ? "up" : event.type === "pointermove" ? "move" : "down",
    x: Math.round(event.clientX - rect.left),
    y: Math.round(event.clientY - rect.top),
    button: event.button ?? 0,
    pointer: event.pointerType ?? "mouse"
  };
};

/**
 * Live canvas stage, generalized from the docs REPL canvas controller
 * (website/public/assets/docs-repl.js `createCanvasController`).
 */
function createCanvasController(card, { runtimeBase }) {
  const canvas = document.createElement("canvas");
  canvas.className = "hara-live-card-canvas";
  canvas.width = 960;
  canvas.height = 600;
  canvas.tabIndex = 0;
  canvas.setAttribute("aria-label", "Live Hara canvas output");

  const panel = document.createElement("section");
  panel.className = "hara-live-card-canvas-panel";
  panel.hidden = true;
  panel.innerHTML = `
    <div class="hara-live-card-canvas-meta">
      <span>ISOLATED · CANVAS/2D</span>
      <output aria-live="polite">Waiting to run</output>
    </div>`;
  panel.append(canvas);
  card.append(panel);

  const status = panel.querySelector("output");
  const canvasId = "canvas/background";
  let runtime = null;
  let compileAnonymousDocument = null;
  let unregisterCanvas = null;
  let generation = 0;
  let activeNode = null;
  let closed = false;

  const setStatus = (text, state = "") => {
    status.textContent = text;
    status.dataset.state = state;
  };

  const ensureRuntime = async (session) => {
    if (!runtime) {
      const [broker, canvasModule] = await Promise.all([
        import(`${runtimeBase}/studio/broker.js`),
        import(`${runtimeBase}/studio/canvas-runtime.js`)
      ]);
      compileAnonymousDocument = broker.compileAnonymousDocument;
      runtime = new canvasModule.CanvasRuntime({
        capabilities: ["canvas/2d"],
        onDiagnostic: (error) => setStatus(errorMessage(error), "error")
      });
      runtime.register(canvasId, canvas);
    }
    unregisterCanvas ??= session.registerCanvas(runtime);
  };

  for (const type of ["pointerdown", "pointermove", "pointerup"]) {
    canvas.addEventListener(type, (event) => {
      if (type === "pointerdown") canvas.setPointerCapture?.(event.pointerId);
      runtime?.pushEvent(localPointer(event, canvas));
    });
  }

  const evaluate = async (session, source) => {
    if (closed) throw new Error("canvas stage is closed");
    const currentGeneration = ++generation;
    const nodeId = `live-card-${currentGeneration}`;
    setStatus("Starting canvas", "loading");
    await ensureRuntime(session);
    runtime.stage(nodeId, canvasId);
    try {
      const document = compileAnonymousDocument(source, {
        documentId: `${location.pathname}/live-card`,
        nodeId
      });
      const taskId = await session.evalRaw(document.source);
      const rendered = runtime.waitForFirstRender(nodeId, canvasId, 5000);
      session.evalRaw(`(studio.node/run-task ${JSON.stringify(taskId)})`)
        .catch((error) => setStatus(errorMessage(error), "error"));
      await rendered;
      if (currentGeneration !== generation) {
        runtime.discard(nodeId, canvasId);
        return { value: null, label: "Canvas superseded" };
      }
      runtime.commit(nodeId, canvasId);
      activeNode = nodeId;
      setStatus("Live · first frame rendered", "ready");
      return { value: null, label: "Canvas live" };
    } catch (error) {
      runtime.discard(nodeId, canvasId);
      setStatus(errorMessage(error), "error");
      throw error;
    }
  };

  return {
    evaluate,
    setStatus,
    show() { panel.hidden = false; },
    hide() { panel.hidden = true; },
    close() {
      if (closed) return;
      closed = true;
      generation += 1;
      if (activeNode) runtime?.release(activeNode, canvasId);
      unregisterCanvas?.();
      runtime?.close();
      panel.remove();
    }
  };
}

/**
 * @typedef {import("./snippets.js").LiveSnippet} LiveSnippet
 */

/**
 * Mount an embeddable Hara live-coding card into `root`.
 *
 * @param {HTMLElement} root element the card is appended to
 * @param {object} [options]
 * @param {LiveSnippet[]} [options.snippets] registry entries to offer as tabs
 * @param {string | null} [options.activeSnippet] id of the initially selected snippet
 * @param {object | Promise<object> | null} [options.kernel] kernel facade (or
 *   promise); when omitted, the shared kernel is lazily booted via
 *   createLiveKernel on first Run
 * @param {string} [options.runtimeBase] base URL for /runtime assets (broker, canvas-runtime)
 * @param {string} [options.docsAssetsBase] base URL for docs-assets
 * @param {string | null} [options.kernelModuleUrl] passed to createLiveKernel
 * @param {Function | null} [options.createKernel] passed to createLiveKernel
 * @param {Function | null} [options.fetchAsset] passed to createLiveKernel
 * @param {string} [options.playgroundUrl] target of the "Open in Playground" link
 * @returns {{ destroy: () => void, run: () => Promise<void> }}
 */
export function mountLiveCard(root, {
  snippets = LIVE_SNIPPETS,
  activeSnippet = null,
  kernel = null,
  runtimeBase = "/runtime",
  docsAssetsBase = "/docs-assets",
  kernelModuleUrl = null,
  createKernel = null,
  fetchAsset = null,
  playgroundUrl = "https://playground.hara-lang.org/"
} = {}) {
  const card = document.createElement("section");
  card.className = "hara-live-card";
  card.dataset.connectionState = "idle";
  card.innerHTML = `
    <header class="hara-live-card-header">
      <span class="hara-live-card-brand">Hara</span>
      <div class="hara-live-card-tabs" role="tablist" aria-label="Hara demos"></div>
      <span class="hara-live-card-status">
        <i class="hara-live-card-connection" aria-hidden="true"></i>
        <small data-live-connection-label>Idle</small>
      </span>
      <a class="hara-live-card-playground" target="_blank" rel="noopener">Open in Playground</a>
    </header>
    <div class="hara-live-card-editor">
      <pre class="code-highlight" aria-hidden="true"><code></code></pre>
      <textarea spellcheck="false" wrap="off" aria-label="Hara source editor"></textarea>
    </div>
    <div class="hara-live-card-toolbar">
      <button type="button" class="hara-live-card-run" data-live-run>Run</button>
      <button type="button" class="hara-live-card-reset" data-live-reset>Reset</button>
      <span class="hara-live-card-hint">Ctrl/Cmd+Enter to run</span>
    </div>
    <output class="hara-live-card-output" aria-live="polite" hidden></output>`;
  root.append(card);

  const tabs = card.querySelector(".hara-live-card-tabs");
  const playgroundLink = card.querySelector(".hara-live-card-playground");
  const highlight = card.querySelector(".code-highlight");
  const highlightContent = highlight.querySelector("code");
  const editor = card.querySelector("textarea");
  const runButton = card.querySelector("[data-live-run]");
  const resetButton = card.querySelector("[data-live-reset]");
  const output = card.querySelector(".hara-live-card-output");
  const connectionLabel = card.querySelector("[data-live-connection-label]");
  playgroundLink.href = playgroundUrl;

  const toast = createCardToast(card);
  const canvas = createCanvasController(card, { runtimeBase });

  const byId = new Map(snippets.map((snippet) => [snippet.id, snippet]));
  let active = byId.get(activeSnippet) ?? snippets[0] ?? null;
  const sessionId = `live-${Math.random().toString(36).slice(2)}`;
  let kernelPromise = kernel ? Promise.resolve(kernel) : null;
  let sessionPromise = null;
  let operation = 0;
  let destroyed = false;

  const setConnection = (state, error = null) => {
    card.dataset.connectionState = state;
    const label = CONNECTION_TEXT[state] ?? state;
    connectionLabel.textContent = error ? `${label}: ${errorMessage(error)}` : label;
  };

  const bootKernel = () => {
    kernelPromise ??= createLiveKernel({
      runtimeBase,
      docsAssetsBase,
      kernelModuleUrl,
      createKernel,
      fetchAsset,
      onProgress: (message, percent) => toast.report(message, percent)
    });
    return kernelPromise;
  };

  const connect = () => {
    if (sessionPromise) return sessionPromise;
    setConnection("loading");
    toast.show();
    toast.report("Preparing Hara kernel", 0);
    sessionPromise = bootKernel()
      .then((instance) => {
        toast.report("Starting session", 99);
        return instance.createSession(sessionId);
      })
      .then((session) => {
        if (destroyed) return session;
        toast.remove();
        setConnection("ready");
        return session;
      })
      .catch((error) => {
        sessionPromise = null;
        toast.fail("Kernel unavailable");
        setConnection("error", error);
        throw error;
      });
    return sessionPromise;
  };

  const syncHighlight = () => {
    highlightContent.innerHTML = highlightHara(editor.value);
    highlightContent.style.transform = `translate(${-editor.scrollLeft}px, ${-editor.scrollTop}px)`;
  };

  const syncOutputMode = () => {
    const isCanvas = active?.kind === "canvas";
    output.hidden = true;
    delete output.dataset.state;
    output.textContent = "";
    if (isCanvas) canvas.show();
    else canvas.hide();
  };

  const selectSnippet = (id) => {
    const next = byId.get(id);
    if (!next || next === active) return;
    active = next;
    editor.value = next.source;
    for (const tab of tabs.querySelectorAll("button")) {
      const selected = tab.dataset.snippetId === next.id;
      tab.setAttribute("aria-selected", String(selected));
    }
    syncHighlight();
    syncOutputMode();
  };

  for (const snippet of snippets) {
    const tab = document.createElement("button");
    tab.type = "button";
    tab.setAttribute("role", "tab");
    tab.dataset.snippetId = snippet.id;
    tab.textContent = snippet.title;
    tab.setAttribute("aria-selected", String(snippet === active));
    tab.addEventListener("click", () => selectSnippet(snippet.id));
    tabs.append(tab);
  }

  const run = async () => {
    if (!active) return;
    const currentOperation = ++operation;
    runButton.disabled = true;
    output.hidden = active.kind === "canvas";
    if (active.kind !== "canvas") {
      output.dataset.state = "pending";
      output.textContent = "Evaluating…";
    }
    let session = null;
    try {
      session = await connect();
      if (currentOperation !== operation || destroyed) return;
      setConnection("busy");
      const result = active.kind === "canvas"
        ? await canvas.evaluate(session, editor.value)
        : await session.eval(editor.value);
      if (currentOperation !== operation || destroyed) return;
      setConnection("ready");
      if (active.kind !== "canvas") {
        output.dataset.state = "ready";
        output.textContent = result.label ?? print(result.value);
      }
    } catch (error) {
      if (currentOperation !== operation || destroyed) return;
      if (session) setConnection("ready");
      if (active.kind !== "canvas") {
        output.dataset.state = "error";
        output.textContent = errorMessage(error);
      }
    } finally {
      if (currentOperation === operation) runButton.disabled = false;
    }
  };

  const reset = async () => {
    operation += 1;
    runButton.disabled = true;
    if (active) {
      editor.value = active.source;
      syncHighlight();
    }
    syncOutputMode();
    const stale = sessionPromise;
    sessionPromise = null;
    if (stale) {
      try {
        const session = await stale;
        await session.close?.();
      } catch (_) {
        // A failed or already-closed session must not prevent recovery.
      }
    }
    if (!destroyed) {
      setConnection("idle");
      runButton.disabled = false;
    }
  };

  // Editor wiring — the same Paredit keyhandling pattern as the workbench
  // (website/app.js), minus completion/undo/prefix features.
  editor.addEventListener("keydown", (event) => {
    const modifier = event.metaKey || event.ctrlKey;
    if (modifier && event.key === "Enter") {
      event.preventDefault();
      run();
      return;
    }
    if (event.ctrlKey && !event.metaKey && !event.altKey &&
        event.key.toLowerCase() === "k" && killToFormEnd(editor)) {
      event.preventDefault();
      return;
    }
    if (event.ctrlKey && !event.metaKey && !event.altKey) {
      const structuralEdit = event.key === "ArrowRight" ? slurpForward : event.key === "ArrowLeft" ? barfForward : null;
      if (structuralEdit?.(editor)) {
        event.preventDefault();
        return;
      }
    }
    if (!event.metaKey && !event.ctrlKey && !event.altKey &&
        applyParedit(editor, event.key)) {
      event.preventDefault();
      return;
    }
    if (event.key === "Tab") {
      event.preventDefault();
      if (event.shiftKey) insertIndent(editor, true);
      else structuralAlign(editor);
    }
  });
  editor.addEventListener("input", syncHighlight);
  editor.addEventListener("scroll", syncHighlight);
  runButton.addEventListener("click", run);
  resetButton.addEventListener("click", reset);

  if (active) editor.value = active.source;
  syncHighlight();
  syncOutputMode();

  return {
    run,
    destroy() {
      if (destroyed) return;
      destroyed = true;
      operation += 1;
      canvas.close();
      const stale = sessionPromise;
      sessionPromise = null;
      if (stale) {
        stale.then((session) => session.close?.()).catch(() => {});
      }
      card.remove();
    }
  };
}
