import {
  applyParedit,
  barfForward,
  insertIndent,
  killToFormEnd,
  localFormAt,
  slurpForward,
  structuralAlign
} from "./editor.js";
import { highlightHara } from "./highlight.js";
import { createLiveKernel } from "./kernel.js";
import { LIVE_SNIPPETS } from "./snippets.js";

const objectType = (value) => value?.constructor?.name ?? "";

/** Print a decoded HTA value as readable Hara data. */
export const print = (value, ancestors = new Set()) => {
  if (value === null || value === undefined) return "nil";
  if (typeof value === "string") return JSON.stringify(value);
  if (["number", "bigint", "boolean"].includes(typeof value)) return String(value);

  const type = objectType(value);
  if (type === "HtaKeyword") return `:${value.name}`;
  if (type === "HtaSymbol") return value.name;
  if (type === "HtaVar") return `#'${print(value.symbol, ancestors)}`;
  if (type === "HtaHandle") return String(value);

  if (typeof value === "object") {
    if (ancestors.has(value)) return "#<cycle>";
    ancestors.add(value);
  }

  let rendered;
  if (type === "HtaAtom") {
    rendered = `#atom <${print(value.value, ancestors)}>`;
  } else if (type === "HtaArray") {
    rendered = `(array${value.values?.length ? ` ${value.values.map((item) => print(item, ancestors)).join(" ")}` : ""})`;
  } else if (type === "HtaObject") {
    const entries = value.entries ?? [];
    rendered = `(object${entries.length ? ` ${entries.map(([key, item]) => `${JSON.stringify(key)} ${print(item, ancestors)}`).join(" ")}` : ""})`;
  } else if (value instanceof Uint8Array) {
    rendered = `#bytes[${[...value].join(" ")}]`;
  } else if (Array.isArray(value)) {
    rendered = `[${value.map((item) => print(item, ancestors)).join(" ")}]`;
  } else if (value instanceof Set) {
    rendered = `#{${[...value].map((item) => print(item, ancestors)).join(" ")}}`;
  } else if (value instanceof Map) {
    rendered = `{${[...value].map(([key, item]) => `${print(key, ancestors)} ${print(item, ancestors)}`).join(" ")}}`;
  } else if (typeof value === "object") {
    const custom = value.toString?.();
    rendered = custom && custom !== "[object Object]"
      ? custom
      : `#js {${Object.entries(value).map(([key, item]) => `${JSON.stringify(key)} ${print(item, ancestors)}`).join(" ")}}`;
  } else {
    rendered = String(value);
  }

  if (typeof value === "object") ancestors.delete(value);
  return rendered;
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

function trimmedSelection(editor) {
  const { value, selectionStart: start, selectionEnd: end } = editor;
  if (start === end) return null;
  const selected = value.slice(start, end);
  const leading = selected.match(/^\s*/)?.[0].length ?? 0;
  const trailing = selected.match(/\s*$/)?.[0].length ?? 0;
  const from = start + leading;
  const to = end - trailing;
  return from < to ? { source: value.slice(from, to), start: from, end: to } : null;
}

/** Prefer the complete expression beginning on a tapped line, then local form. */
function formAtEditor(editor, preferLine = false) {
  const selected = trimmedSelection(editor);
  if (selected) return selected;

  const { value, selectionStart: caret } = editor;
  if (preferLine) {
    const lineStart = value.lastIndexOf("\n", Math.max(0, caret - 1)) + 1;
    const first = lineStart + (value.slice(lineStart).match(/^\s*/)?.[0].length ?? 0);
    const lineForm = localFormAt(value, first);
    if (lineForm?.start === first && lineForm.end >= caret) return lineForm;
  }
  return localFormAt(value, caret);
}

/**
 * Mount an embeddable Hara live-coding card into `root`.
 *
 * @param {HTMLElement} root element the card is appended to
 * @param {object} [options]
 * @param {LiveSnippet[]} [options.snippets] registry entries offered as examples
 * @param {string | null} [options.activeSnippet] id of the initially selected snippet
 * @param {object | Promise<object> | null} [options.kernel] kernel facade (or
 *   promise); when omitted, the shared kernel is lazily booted on first Eval/Run
 * @param {string} [options.runtimeBase] base URL for /runtime assets (broker, canvas-runtime)
 * @param {string} [options.docsAssetsBase] base URL for docs-assets
 * @param {string | null} [options.kernelModuleUrl] passed to createLiveKernel
 * @param {Function | null} [options.createKernel] passed to createLiveKernel
 * @param {Function | null} [options.fetchAsset] passed to createLiveKernel
 * @param {string} [options.playgroundUrl] target of the "Open in Playground" link
 * @returns {{ destroy: () => void, eval: () => Promise<void>, run: () => Promise<void> }}
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
      <span class="hara-live-card-status" title="Kernel status">
        <i class="hara-live-card-connection" aria-hidden="true"></i>
        <small data-live-connection-label>Idle</small>
      </span>
      <button type="button" class="hara-live-card-eval" data-live-eval>Eval</button>
      <button type="button" class="hara-live-card-run" data-live-run>Run</button>
      <label class="hara-live-card-examples">
        <span class="hara-live-card-sr-only">Example</span>
        <select data-live-example aria-label="Example"></select>
      </label>
      <a class="hara-live-card-playground" target="_blank" rel="noopener">Open in Playground</a>
    </header>
    <div class="hara-live-card-editor">
      <pre class="code-highlight" aria-hidden="true"><code></code></pre>
      <textarea spellcheck="false" wrap="off" aria-label="Hara source editor"></textarea>
    </div>
    <output class="hara-live-card-output" aria-live="polite" hidden></output>`;
  root.append(card);

  const exampleSelect = card.querySelector("[data-live-example]");
  const playgroundLink = card.querySelector(".hara-live-card-playground");
  const highlight = card.querySelector(".code-highlight");
  const highlightContent = highlight.querySelector("code");
  const editor = card.querySelector("textarea");
  const evalButton = card.querySelector("[data-live-eval]");
  const runButton = card.querySelector("[data-live-run]");
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
  let evalRange = null;
  let operation = 0;
  let destroyed = false;
  let lastTouchEvaluation = 0;

  const setConnection = (state, error = null) => {
    card.dataset.connectionState = state;
    const label = CONNECTION_TEXT[state] ?? state;
    connectionLabel.textContent = error ? `${label}: ${errorMessage(error)}` : label;
    card.querySelector(".hara-live-card-status")
      .setAttribute("aria-label", error ? `${label}: ${errorMessage(error)}` : `Kernel ${label}`);
  };

  const setControlsDisabled = (disabled) => {
    evalButton.disabled = disabled;
    runButton.disabled = disabled;
    exampleSelect.disabled = disabled;
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
    highlightContent.innerHTML = highlightHara(editor.value, { evalRange });
    highlightContent.style.transform = `translate(${-editor.scrollLeft}px, ${-editor.scrollTop}px)`;
  };

  const syncOutputMode = () => {
    output.hidden = true;
    delete output.dataset.state;
    delete output.dataset.mode;
    output.textContent = "";
    if (active?.kind === "canvas") canvas.show();
    else canvas.hide();
  };

  for (const snippet of snippets) {
    const option = document.createElement("option");
    option.value = snippet.id;
    option.textContent = snippet.title;
    option.selected = snippet === active;
    exampleSelect.append(option);
  }

  const selectSnippet = (id) => {
    const next = byId.get(id);
    if (!next || next === active) return;
    operation += 1;
    active = next;
    evalRange = null;
    editor.value = next.source;
    exampleSelect.value = next.id;
    syncHighlight();
    syncOutputMode();
  };

  const evaluate = async ({ source, mode, range = null }) => {
    if (!active || !source?.trim()) return;
    const currentOperation = ++operation;
    const canvasRun = mode === "run" && active.kind === "canvas";
    evalRange = range;
    syncHighlight();
    setControlsDisabled(true);
    output.hidden = canvasRun;
    if (!canvasRun) {
      output.dataset.state = "pending";
      output.dataset.mode = mode;
      output.textContent = "Evaluating…";
    }

    let session = null;
    try {
      session = await connect();
      if (currentOperation !== operation || destroyed) return;
      setConnection("busy");
      const result = canvasRun
        ? await canvas.evaluate(session, source)
        : await session.eval(source);
      if (currentOperation !== operation || destroyed) return;
      setConnection("ready");
      if (!canvasRun) {
        output.hidden = false;
        output.dataset.state = "ready";
        output.textContent = result.label ?? print(result.value);
      }
    } catch (error) {
      if (currentOperation !== operation || destroyed) return;
      if (session) setConnection("ready");
      output.hidden = false;
      output.dataset.state = "error";
      output.textContent = errorMessage(error);
    } finally {
      if (currentOperation === operation && !destroyed) setControlsDisabled(false);
    }
  };

  const evalCurrent = async ({ preferLine = false } = {}) => {
    const form = formAtEditor(editor, preferLine);
    if (!form?.source) {
      output.hidden = false;
      output.dataset.state = "error";
      output.dataset.mode = "eval";
      output.textContent = "Tap inside a form or select source to evaluate.";
      return;
    }
    await evaluate({ source: form.source, mode: "eval", range: form });
  };

  const run = () => evaluate({ source: editor.value, mode: "run", range: null });

  // Structural editing and evaluation shortcuts.
  editor.addEventListener("keydown", (event) => {
    const modifier = event.metaKey || event.ctrlKey;
    if (modifier && event.key === "Enter") {
      event.preventDefault();
      run();
      return;
    }
    if ((event.altKey && event.key === "Enter") ||
        (event.ctrlKey && !event.metaKey && !event.altKey && event.key.toLowerCase() === "e")) {
      event.preventDefault();
      evalCurrent();
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

  editor.addEventListener("input", () => {
    evalRange = null;
    syncHighlight();
  });
  editor.addEventListener("scroll", syncHighlight);
  editor.addEventListener("select", () => {
    const form = formAtEditor(editor, true);
    evalRange = form ? { start: form.start, end: form.end } : null;
    syncHighlight();
  });

  // InstaREPL-style mobile interaction: a direct touch evaluates the complete
  // expression beginning on that line. Text selections are left untouched.
  editor.addEventListener("pointerup", (event) => {
    if (event.pointerType !== "touch" || editor.selectionStart !== editor.selectionEnd) return;
    const now = Date.now();
    if (now - lastTouchEvaluation < 350) return;
    lastTouchEvaluation = now;
    setTimeout(() => evalCurrent({ preferLine: true }), 0);
  });

  evalButton.addEventListener("click", () => evalCurrent({ preferLine: true }));
  runButton.addEventListener("click", run);
  exampleSelect.addEventListener("change", () => selectSnippet(exampleSelect.value));

  if (active) editor.value = active.source;
  syncHighlight();
  syncOutputMode();

  return {
    eval: evalCurrent,
    run,
    destroy() {
      if (destroyed) return;
      destroyed = true;
      operation += 1;
      canvas.close();
      const stale = sessionPromise;
      sessionPromise = null;
      if (stale) stale.then((session) => session.close?.()).catch(() => {});
      card.remove();
    }
  };
}
