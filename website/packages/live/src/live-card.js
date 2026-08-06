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

/** Prefer an actual canvas-task failure over a generic first-frame timeout. */
export async function waitForCanvasFirstFrame(rendered, task) {
  return Promise.race([
    rendered,
    task.then(() => {
      throw new Error("canvas task stopped before rendering its first frame");
    })
  ]);
}

/** Cancel an in-flight HTA evaluation without destroying the shared kernel. */
export function cancelEvaluation(task) {
  if (typeof task?.cancel !== "function") return false;
  try {
    return task.cancel() !== false;
  } catch (_) {
    return false;
  }
}

/**
 * Run after a textarea's click/default action has committed its new caret.
 * Mobile browsers may expose the previous selection during pointerup.
 */
export function afterCaretPlacement(callback, {
  requestFrame = globalThis.requestAnimationFrame?.bind(globalThis),
  setTimer = globalThis.setTimeout?.bind(globalThis)
} = {}) {
  let cancelled = false;
  const run = () => {
    if (!cancelled) callback();
  };
  if (typeof requestFrame === "function") requestFrame(run);
  else setTimer?.(run, 0);
  return () => { cancelled = true; };
}

const CONNECTION_TEXT = {
  idle: "Idle",
  loading: "Connecting",
  ready: "Connected",
  busy: "Evaluating",
  error: "Unavailable"
};

const clamp = (value, minimum, maximum) =>
  Math.min(maximum, Math.max(minimum, value));

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

/** Touch- and keyboard-accessible vertical resize handle. */
function createVerticalResizer(target, {
  label,
  initialHeight,
  minimumHeight,
  maximumHeight = () => Math.max(minimumHeight, Math.round((globalThis.innerHeight || 900) * 0.8)),
  onResize = () => {}
}) {
  const handle = document.createElement("div");
  handle.className = "hara-live-card-resizer";
  handle.tabIndex = 0;
  handle.setAttribute("role", "separator");
  handle.setAttribute("aria-label", label);
  handle.setAttribute("aria-orientation", "horizontal");
  target.append(handle);

  let activePointer = null;
  let startY = 0;
  let startHeight = 0;

  const maximum = () => typeof maximumHeight === "function" ? maximumHeight() : maximumHeight;
  const setHeight = (height) => {
    const resolved = Math.round(clamp(height, minimumHeight, Math.max(minimumHeight, maximum())));
    target.style.height = `${resolved}px`;
    handle.setAttribute("aria-valuemin", String(minimumHeight));
    handle.setAttribute("aria-valuemax", String(Math.round(maximum())));
    handle.setAttribute("aria-valuenow", String(resolved));
    onResize(resolved);
    return resolved;
  };

  const finish = (event) => {
    if (activePointer === null || (event && event.pointerId !== activePointer)) return;
    try { handle.releasePointerCapture?.(activePointer); } catch (_) { /* already released */ }
    activePointer = null;
    delete target.dataset.resizing;
  };

  handle.addEventListener("pointerdown", (event) => {
    if (event.pointerType !== "touch" && event.button !== 0) return;
    event.preventDefault();
    activePointer = event.pointerId;
    startY = event.clientY;
    startHeight = target.getBoundingClientRect().height;
    target.dataset.resizing = "true";
    handle.setPointerCapture?.(event.pointerId);
  });
  handle.addEventListener("pointermove", (event) => {
    if (event.pointerId !== activePointer) return;
    event.preventDefault();
    setHeight(startHeight + event.clientY - startY);
  });
  handle.addEventListener("pointerup", finish);
  handle.addEventListener("pointercancel", finish);
  handle.addEventListener("lostpointercapture", finish);
  handle.addEventListener("keydown", (event) => {
    if (!["ArrowUp", "ArrowDown", "Home", "End"].includes(event.key)) return;
    event.preventDefault();
    const current = target.getBoundingClientRect().height;
    const step = event.shiftKey ? 48 : 16;
    if (event.key === "ArrowUp") setHeight(current - step);
    else if (event.key === "ArrowDown") setHeight(current + step);
    else if (event.key === "Home") setHeight(minimumHeight);
    else setHeight(maximum());
  });
  handle.addEventListener("dblclick", () => setHeight(initialHeight));

  setHeight(initialHeight);
  return { handle, setHeight, destroy: () => handle.remove() };
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
function createCanvasController(card, { runtimeBase, onRunningChange = () => {} }) {
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
  let stagedNode = null;
  let activeNode = null;
  let activeTask = null;
  let closed = false;

  const setStatus = (text, state = "") => {
    status.textContent = text;
    status.dataset.state = state;
  };

  const clearSurface = () => {
    const context = canvas.getContext?.("2d");
    if (!context) return;
    context.setTransform(1, 0, 0, 1, 0, 0);
    context.clearRect(0, 0, canvas.width, canvas.height);
    context.fillStyle = "#02050b";
    context.fillRect(0, 0, canvas.width, canvas.height);
  };

  const canvasResizer = createVerticalResizer(panel, {
    label: "Resize canvas",
    initialHeight: Math.min(500, Math.max(300, Math.round((globalThis.innerHeight || 800) * 0.52))),
    minimumHeight: 220,
    onResize: () => runtime?.resize(canvas)
  });

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
      runtime.resize(canvas);
    }
    unregisterCanvas ??= session.registerCanvas(runtime);
  };

  for (const type of ["pointerdown", "pointermove", "pointerup"]) {
    canvas.addEventListener(type, (event) => {
      if (type === "pointerdown") canvas.setPointerCapture?.(event.pointerId);
      runtime?.pushEvent(localPointer(event, canvas));
    });
  }

  const interrupt = ({ clear = true, statusText = "Stopped" } = {}) => {
    if (closed) return false;
    generation += 1;
    const nodes = [...new Set([stagedNode, activeNode].filter(Boolean))];
    stagedNode = null;
    activeNode = null;
    for (const nodeId of nodes) runtime?.release(nodeId, canvasId);
    const task = activeTask;
    activeTask = null;
    const cancelled = cancelEvaluation(task);
    if (clear) clearSurface();
    if (statusText !== null) setStatus(statusText, "idle");
    onRunningChange(false);
    return cancelled || nodes.length > 0;
  };

  const evaluate = async (session, source) => {
    if (closed) throw new Error("canvas stage is closed");
    interrupt({ clear: false, statusText: null });
    const currentGeneration = ++generation;
    const nodeId = `live-card-${currentGeneration}`;
    stagedNode = nodeId;
    setStatus("Starting canvas", "loading");
    await ensureRuntime(session);
    runtime.stage(nodeId, canvasId);
    try {
      const document = compileAnonymousDocument(source, {
        documentId: `${location.pathname}/live-card`,
        nodeId
      });
      const taskId = await session.evalRaw(document.source);
      if (typeof taskId !== "string" || !taskId.startsWith("task-")) {
        throw new Error(`canvas program did not start a node task: ${print(taskId)}`);
      }
      const rendered = runtime.waitForFirstRender(nodeId, canvasId, 8000);
      const task = session.evalRaw(`(studio.node/run-task ${JSON.stringify(taskId)})`);
      activeTask = task;
      task.then(
        () => {
          if (task !== activeTask || currentGeneration !== generation) return;
          activeTask = null;
          runtime.release(nodeId, canvasId);
          if (activeNode === nodeId) activeNode = null;
          if (stagedNode === nodeId) stagedNode = null;
          setStatus("Stopped", "idle");
          onRunningChange(false);
        },
        (error) => {
          if (task !== activeTask || currentGeneration !== generation) return;
          activeTask = null;
          runtime.release(nodeId, canvasId);
          if (activeNode === nodeId) activeNode = null;
          if (stagedNode === nodeId) stagedNode = null;
          setStatus(errorMessage(error), "error");
          onRunningChange(false);
        }
      );
      await waitForCanvasFirstFrame(rendered, task);
      if (currentGeneration !== generation) {
        runtime.release(nodeId, canvasId);
        return { value: null, label: "Canvas interrupted" };
      }
      runtime.commit(nodeId, canvasId);
      stagedNode = null;
      activeNode = nodeId;
      setStatus("Live · Stop or Esc to interrupt", "ready");
      onRunningChange(true);
      return { value: null, label: "Canvas live" };
    } catch (error) {
      runtime.release(nodeId, canvasId);
      if (currentGeneration !== generation) {
        return { value: null, label: "Canvas interrupted" };
      }
      stagedNode = null;
      activeNode = null;
      if (activeTask && currentGeneration === generation) activeTask = null;
      setStatus(errorMessage(error), "error");
      onRunningChange(false);
      throw error;
    }
  };

  return {
    evaluate,
    interrupt,
    isRunning: () => Boolean(activeTask || stagedNode || activeNode),
    setStatus,
    show() { panel.hidden = false; },
    hide() { panel.hidden = true; },
    close() {
      if (closed) return;
      interrupt({ clear: false, statusText: null });
      closed = true;
      unregisterCanvas?.();
      runtime?.close();
      canvasResizer.destroy();
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

/** Prefer the complete expression beginning on a clicked line, then local form. */
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
 * @returns {{ destroy: () => void, eval: () => Promise<void>, run: () => Promise<void>, interrupt: () => boolean, reset: () => void }}
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
  card.dataset.instarepl = "true";
  card.innerHTML = `
    <header class="hara-live-card-header">
      <span class="hara-live-card-status" title="Kernel status">
        <i class="hara-live-card-connection" aria-hidden="true"></i>
        <small data-live-connection-label>Idle</small>
      </span>
      <button type="button" class="hara-live-card-eval" data-live-eval>Eval</button>
      <button type="button" class="hara-live-card-run" data-live-run>Run</button>
      <button type="button" class="hara-live-card-eval hara-live-card-reset" data-live-reset hidden>Reset</button>
      <div class="hara-live-card-tabs" role="tablist" aria-label="Examples"></div>
      <a class="hara-live-card-playground" target="_blank" rel="noopener">Open in Playground</a>
    </header>
    <div class="hara-live-card-editor">
      <pre class="code-highlight" aria-hidden="true"><code></code></pre>
      <textarea spellcheck="false" wrap="off" aria-label="Hara source editor"></textarea>
    </div>
    <output class="hara-live-card-output" aria-live="polite" hidden></output>`;
  root.append(card);

  const tabs = card.querySelector(".hara-live-card-tabs");
  const playgroundLink = card.querySelector(".hara-live-card-playground");
  const editorSurface = card.querySelector(".hara-live-card-editor");
  const highlight = card.querySelector(".code-highlight");
  const highlightContent = highlight.querySelector("code");
  const editor = card.querySelector("textarea");
  const evalButton = card.querySelector("[data-live-eval]");
  const runButton = card.querySelector("[data-live-run]");
  const resetButton = card.querySelector("[data-live-reset]");
  const output = card.querySelector(".hara-live-card-output");
  const connectionLabel = card.querySelector("[data-live-connection-label]");
  playgroundLink.href = playgroundUrl;

  const byId = new Map(snippets.map((snippet) => [snippet.id, snippet]));
  let active = byId.get(activeSnippet) ?? snippets[0] ?? null;
  let canvasRunning = false;

  const updateCanvasControls = (running) => {
    canvasRunning = Boolean(running);
    card.dataset.canvasRunning = String(canvasRunning);
    runButton.textContent = canvasRunning ? "Stop" : "Run";
    runButton.setAttribute("aria-label", canvasRunning ? "Interrupt running canvas" : "Run example");
    runButton.classList.toggle("hara-live-card-run", !canvasRunning);
    runButton.classList.toggle("hara-live-card-eval", canvasRunning);
    resetButton.hidden = active?.kind !== "canvas" && !canvasRunning;
  };

  const toast = createCardToast(card);
  const canvas = createCanvasController(card, {
    runtimeBase,
    onRunningChange: updateCanvasControls
  });

  const sessionId = `live-${Math.random().toString(36).slice(2)}`;
  let kernelPromise = kernel ? Promise.resolve(kernel) : null;
  let sessionPromise = null;
  let evalRange = null;
  let operation = 0;
  let destroyed = false;
  let pointerGesture = null;
  let pendingPointerEvaluation = false;
  let cancelPendingPointerEvaluation = null;
  let lastPointerEvaluation = 0;

  const setConnection = (state, error = null) => {
    card.dataset.connectionState = state;
    const label = CONNECTION_TEXT[state] ?? state;
    connectionLabel.textContent = error ? `${label}: ${errorMessage(error)}` : label;
    card.querySelector(".hara-live-card-status")
      .setAttribute("aria-label", error ? `${label}: ${errorMessage(error)}` : `Kernel ${label}`);
  };

  const tabButtons = () => [...tabs.querySelectorAll("button")];
  const setControlsDisabled = (disabled) => {
    evalButton.disabled = disabled;
    runButton.disabled = disabled && !canvasRunning;
    resetButton.disabled = disabled;
    for (const tab of tabButtons()) tab.disabled = disabled;
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

  const editorResizer = createVerticalResizer(editorSurface, {
    label: "Resize editor",
    initialHeight: 230,
    minimumHeight: 150,
    onResize: syncHighlight
  });
  const resizeObserver = typeof ResizeObserver === "function"
    ? new ResizeObserver(syncHighlight)
    : null;
  resizeObserver?.observe(editorSurface);

  const syncOutputMode = () => {
    output.hidden = true;
    delete output.dataset.state;
    delete output.dataset.mode;
    output.textContent = "";
    if (active?.kind === "canvas") canvas.show();
    else canvas.hide();
    updateCanvasControls(canvas.isRunning());
  };

  const stopCanvas = ({ clear = true, statusText = "Stopped" } = {}) => {
    operation += 1;
    const stopped = canvas.interrupt({ clear, statusText });
    if (sessionPromise) setConnection("ready");
    setControlsDisabled(false);
    return stopped;
  };

  const reset = () => {
    operation += 1;
    canvas.interrupt({ clear: true, statusText: active?.kind === "canvas" ? "Waiting to run" : null });
    evalRange = null;
    if (active) editor.value = active.source;
    syncHighlight();
    syncOutputMode();
    if (sessionPromise) setConnection("ready");
  };

  const selectSnippet = (id, { focus = false } = {}) => {
    const next = byId.get(id);
    if (!next) return;
    if (next !== active) {
      operation += 1;
      canvas.interrupt({ clear: true, statusText: null });
      active = next;
      evalRange = null;
      editor.value = next.source;
      syncHighlight();
      syncOutputMode();
    }
    for (const tab of tabButtons()) {
      const selected = tab.dataset.snippetId === next.id;
      tab.setAttribute("aria-selected", String(selected));
      tab.tabIndex = selected ? 0 : -1;
      if (selected && focus) tab.focus();
    }
  };

  for (const snippet of snippets) {
    const tab = document.createElement("button");
    tab.type = "button";
    tab.setAttribute("role", "tab");
    tab.dataset.snippetId = snippet.id;
    tab.textContent = snippet.title;
    tab.setAttribute("aria-selected", String(snippet === active));
    tab.tabIndex = snippet === active ? 0 : -1;
    tab.addEventListener("click", () => selectSnippet(snippet.id));
    tabs.append(tab);
  }

  tabs.addEventListener("keydown", (event) => {
    if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
    const buttons = tabButtons();
    if (!buttons.length) return;
    event.preventDefault();
    const current = Math.max(0, buttons.indexOf(document.activeElement));
    const nextIndex = event.key === "Home" ? 0
      : event.key === "End" ? buttons.length - 1
      : (current + (event.key === "ArrowRight" ? 1 : -1) + buttons.length) % buttons.length;
    selectSnippet(buttons[nextIndex].dataset.snippetId, { focus: true });
  });

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
      output.textContent = "Click or tap inside a form, or select source to evaluate.";
      return;
    }
    await evaluate({ source: form.source, mode: "eval", range: form });
  };

  const run = () => evaluate({ source: editor.value, mode: "run", range: null });
  const runOrStop = () => canvas.isRunning() ? stopCanvas() : run();

  // Structural editing and evaluation shortcuts.
  editor.addEventListener("keydown", (event) => {
    const modifier = event.metaKey || event.ctrlKey;
    if (modifier && event.key === "Enter") {
      event.preventDefault();
      runOrStop();
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

  card.addEventListener("keydown", (event) => {
    const interruptKey = event.key === "Escape" ||
      (event.ctrlKey && !event.metaKey && event.key === ".");
    if (!interruptKey || !canvas.isRunning()) return;
    event.preventDefault();
    stopCanvas();
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

  // InstaREPL interaction: pointerup only records a direct tap. Evaluation is
  // deferred until click/default handling has committed the new textarea caret.
  // This avoids evaluating the previous cursor position on Android and iOS.
  editor.addEventListener("pointerdown", (event) => {
    if (!event.isPrimary || (event.pointerType !== "touch" && event.button !== 0)) return;
    cancelPendingPointerEvaluation?.();
    cancelPendingPointerEvaluation = null;
    pendingPointerEvaluation = false;
    pointerGesture = {
      id: event.pointerId,
      x: event.clientX,
      y: event.clientY
    };
  });
  editor.addEventListener("pointerup", (event) => {
    const gesture = pointerGesture;
    pointerGesture = null;
    if (!gesture || gesture.id !== event.pointerId) return;
    if (Math.hypot(event.clientX - gesture.x, event.clientY - gesture.y) > 8) return;
    pendingPointerEvaluation = true;
  });
  editor.addEventListener("click", () => {
    if (!pendingPointerEvaluation) return;
    pendingPointerEvaluation = false;
    cancelPendingPointerEvaluation?.();
    cancelPendingPointerEvaluation = afterCaretPlacement(() => {
      cancelPendingPointerEvaluation = null;
      if (destroyed || editor.selectionStart !== editor.selectionEnd) return;
      const now = Date.now();
      if (now - lastPointerEvaluation < 280) return;
      lastPointerEvaluation = now;
      evalCurrent({ preferLine: true });
    });
  });
  editor.addEventListener("pointercancel", () => {
    pointerGesture = null;
    pendingPointerEvaluation = false;
    cancelPendingPointerEvaluation?.();
    cancelPendingPointerEvaluation = null;
  });

  evalButton.addEventListener("click", () => evalCurrent({ preferLine: true }));
  runButton.addEventListener("click", runOrStop);
  resetButton.addEventListener("click", reset);

  if (active) editor.value = active.source;
  selectSnippet(active?.id ?? "");
  syncHighlight();
  syncOutputMode();

  return {
    eval: evalCurrent,
    run,
    interrupt: stopCanvas,
    reset,
    destroy() {
      if (destroyed) return;
      destroyed = true;
      operation += 1;
      cancelPendingPointerEvaluation?.();
      cancelPendingPointerEvaluation = null;
      resizeObserver?.disconnect();
      editorResizer.destroy();
      canvas.close();
      const stale = sessionPromise;
      sessionPromise = null;
      if (stale) stale.then((session) => session.close?.()).catch(() => {});
      card.remove();
    }
  };
}
