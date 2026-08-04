import { createDocsKernel } from "/docs-assets/javascripts/kernel.js";
import {
  createDocsSessionRegistry,
  describeDocsSession
} from "./docs-repl-state.js";

const print = (value) => {
  if (value === null) return "nil";
  if (typeof value === "string") return JSON.stringify(value);
  if (Array.isArray(value)) return `[${value.map(print).join(" ")}]`;
  if (value instanceof Map) return `{${[...value].map(([key, item]) => `${print(key)} ${print(item)}`).join(" ")}}`;
  return String(value);
};

function createKernelProgress() {
  const toast = document.createElement("div");
  toast.className = "hara-kernel-toast";
  toast.setAttribute("role", "status");
  toast.setAttribute("aria-live", "polite");
  toast.innerHTML = `<i></i><span>Preparing Hara kernel</span><b>0%</b>`;
  document.body.append(toast);

  let loaded = 0;
  let expected = 0;
  const report = (message) => {
    const percent = expected ? Math.min(99, Math.round(loaded / expected * 100)) : 0;
    toast.querySelector("span").textContent = message;
    toast.querySelector("b").textContent = `${percent}%`;
    toast.style.setProperty("--kernel-progress", `${percent}%`);
  };

  return {
    toast,
    async fetch(input, init) {
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
          report("Loading Hara kernel");
          controller.enqueue(value);
        }
      });
      return new Response(stream, {
        status: response.status,
        statusText: response.statusText,
        headers: response.headers
      });
    }
  };
}

function createKernelPromise(progress) {
  return fetch("/runtime/kernel-manifest.json")
    .then(async (response) => {
      if (!response.ok) throw new Error(`kernel manifest: ${response.status}`);
      const manifest = await response.json();
      return createDocsKernel({
        wasmUrl: manifest.variants.core.url,
        workerUrl: "/runtime/hta-worker.js",
        manifest,
        resources: {
          "studio.store": "/docs-assets/rust/studio/hal/store.hal",
          "studio.fs": "/docs-assets/rust/studio/hal/fs.hal",
          "studio.node": "/runtime/studio/hal/node.hal",
          "studio.draw": "/runtime/studio/hal/draw.hal",
          "std.lib.substrate.frame": "/runtime/std/lib/substrate/frame.hal"
        },
        fetchAsset: progress.fetch
      });
    })
    .then((kernel) => {
      progress.toast.remove();
      document.dispatchEvent(new CustomEvent("hara:kernel-ready", {
        detail: { artifact: "hara-wasm-core" }
      }));
      return kernel;
    })
    .catch((error) => {
      progress.toast.dataset.state = "error";
      progress.toast.querySelector("span").textContent = "Kernel unavailable";
      progress.toast.querySelector("b").textContent = "";
      console.error(error);
      throw error;
    });
}

function installRepl(frame, descriptor, sessions, { evaluate = null, source: suppliedSource = null } = {}) {
  const code = frame.querySelector("pre > code");
  if (!code) return null;
  const pre = code.parentElement;
  if (!pre || pre.closest(".hara-repl")) return null;
  const source = suppliedSource ?? (frame.dataset.haraSource
    ? decodeURIComponent(frame.dataset.haraSource)
    : code.textContent.replace(/\n$/, ""));

  const cell = document.createElement("section");
  cell.className = "hara-repl";
  cell.dataset.connectionState = "loading";
  cell.dataset.haraSessionId = descriptor.id;
  if (descriptor.groupName) cell.dataset.haraSessionGroup = descriptor.groupName;
  cell.innerHTML = `
    <header>
      <span class="hara-repl-brand">Hara</span>
      <details class="hara-repl-details">
        <summary>
          <i class="hara-repl-connection" aria-hidden="true"></i>
          <small data-hara-session-label></small>
        </summary>
        <div class="hara-repl-details-panel">
          <dl>
            <div><dt>Connection</dt><dd data-hara-connection-label>Connecting…</dd></div>
            <div><dt>Runtime</dt><dd>hara-wasm-core</dd></div>
            <div><dt>Shared with</dt><dd data-hara-shared-with></dd></div>
            <div><dt>Session</dt><dd data-hara-session-id></dd></div>
          </dl>
        </div>
      </details>
      <button type="button">Run</button>
    </header>
    <textarea spellcheck="false"></textarea>
    <output hidden aria-live="polite"></output>`;

  const editor = cell.querySelector("textarea");
  const output = cell.querySelector("output");
  const button = cell.querySelector("button");
  const details = cell.querySelector(".hara-repl-details");
  const summary = details.querySelector("summary");
  const status = cell.querySelector("[data-hara-connection-label]");
  cell.querySelector("[data-hara-session-label]").textContent = descriptor.label;
  cell.querySelector("[data-hara-shared-with]").textContent = descriptor.sharedWith;
  cell.querySelector("[data-hara-session-id]").textContent = descriptor.id;
  editor.value = source;
  editor.rows = Math.min(24, Math.max(2, source.split("\n").length));
  frame.replaceWith(cell);

  const connectionText = {
    loading: "Connecting",
    ready: "Connected",
    busy: "Connected, evaluating",
    error: "Unavailable"
  };
  const setConnection = (state, error = null) => {
    cell.dataset.connectionState = state;
    const label = connectionText[state] ?? state;
    status.textContent = error ? `${label}: ${String(error?.message ?? error)}` : label;
    summary.setAttribute("aria-label", `${descriptor.label}; ${label}; connection details`);
  };

  let connectedSession = null;
  let connectedRevision = -1;
  let connectionGeneration = 0;
  let operation = 0;

  const connect = async () => {
    const desiredRevision = sessions.revision(descriptor);
    if (connectedSession && connectedRevision === desiredRevision) return connectedSession;

    const generation = connectionGeneration;
    setConnection("loading");
    try {
      const session = await sessions.get(descriptor);
      if (generation === connectionGeneration) {
        connectedSession = session;
        connectedRevision = sessions.revision(descriptor);
        setConnection("ready");
      }
      return session;
    } catch (error) {
      if (generation === connectionGeneration) setConnection("error", error);
      throw error;
    }
  };

  connect().catch(() => {});

  button.addEventListener("click", async () => {
    const currentOperation = ++operation;
    button.disabled = true;
    output.hidden = false;
    output.dataset.state = "pending";
    output.textContent = "Evaluating…";
    let session = null;
    try {
      session = await connect();
      if (currentOperation !== operation) return;
      setConnection("busy");
      const result = evaluate
        ? await evaluate(session, editor.value)
        : await session.eval(editor.value);
      if (currentOperation !== operation) return;
      setConnection("ready");
      output.dataset.state = "ready";
      output.textContent = result.label ?? print(result.value);
    } catch (error) {
      if (currentOperation !== operation) return;
      if (session) setConnection("ready");
      output.dataset.state = "error";
      output.textContent = String(error?.message ?? error);
    } finally {
      if (currentOperation === operation) button.disabled = false;
    }
  });

  editor.addEventListener("keydown", (event) => {
    if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
      event.preventDefault();
      button.click();
    }
  });

  return {
    descriptor,
    cell,
    editor,
    button,
    output,
    connect,
    setConnection,
    beginReset() {
      operation += 1;
      connectionGeneration += 1;
      connectedSession = null;
      connectedRevision = -1;
      button.disabled = true;
      output.hidden = true;
      output.textContent = "";
      delete output.dataset.state;
      setConnection("loading");
    },
    finishReset(session) {
      connectedSession = session;
      connectedRevision = sessions.revision(descriptor);
      button.disabled = false;
      setConnection("ready");
    },
    failReset(error) {
      connectedSession = null;
      connectedRevision = -1;
      button.disabled = false;
      setConnection("error", error);
    }
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

const errorMessage = (error) => String(error?.message ?? error).replace(/^Error: /, "");

function createCanvasController(stage) {
  const canvas = document.createElement("canvas");
  canvas.className = "hara-live-canvas";
  canvas.width = 960;
  canvas.height = 600;
  canvas.tabIndex = 0;
  canvas.setAttribute("aria-label", "Live Hara canvas output");

  const panel = document.createElement("section");
  panel.className = "hara-live-canvas-panel";
  panel.innerHTML = `
    <div class="hara-live-canvas-meta">
      <span>ISOLATED · CANVAS/2D</span>
      <output aria-live="polite">Waiting to run</output>
    </div>`;
  panel.append(canvas);

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
        import("/runtime/studio/broker.js"),
        import("/runtime/studio/canvas-runtime.js")
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
    const nodeId = `docs-tictactoe-${stage.dataset.haraCanvasStage}-${currentGeneration}`;
    setStatus("Starting canvas", "loading");
    await ensureRuntime(session);
    runtime.stage(nodeId, canvasId);
    try {
      const document = compileAnonymousDocument(source, {
        documentId: `${location.pathname}/canvas-${stage.dataset.haraCanvasStage}`,
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

  const loadSource = async (fallback) => {
    const program = stage.dataset.haraCanvasProgram;
    if (!program) return fallback;
    const response = await fetch(new URL(program, document.baseURI));
    if (!response.ok) throw new Error(`unable to load tutorial source (${response.status})`);
    return response.text();
  };

  const close = () => {
    if (closed) return;
    closed = true;
    generation += 1;
    if (activeNode) runtime?.release(activeNode, canvasId);
    unregisterCanvas?.();
    runtime?.close();
  };
  window.addEventListener("pagehide", close, { once: true });

  return {
    evaluate,
    loadSource,
    mount(cell) {
      cell.classList.add("hara-canvas-repl");
      cell.after(panel);
    },
    setStatus
  };
}

const frames = [...document.querySelectorAll("main [data-hara-eval]")];
const canvasStages = [...document.querySelectorAll("main [data-hara-canvas-stage]")];
if (frames.length > 0 || canvasStages.length > 0) {
  const progress = createKernelProgress();
  const kernelPromise = createKernelPromise(progress);
  const sessions = createDocsSessionRegistry(kernelPromise);
  const runners = frames.map((frame, index) => {
    const descriptor = describeDocsSession({
      scope: frame.dataset.haraScope,
      groupName: frame.dataset.haraGroup,
      pagePath: location.pathname,
      sequence: index + 1
    });
    return installRepl(frame, descriptor, sessions);
  }).filter(Boolean);

  for (const [index, stage] of canvasStages.entries()) {
    const frame = stage.querySelector(".expressive-code, .highlight, pre");
    if (!frame) continue;
    const controller = createCanvasController(stage);
    const descriptor = describeDocsSession({
      pagePath: `${location.pathname}/canvas`,
      sequence: index + 1
    });
    const runner = installRepl(frame, descriptor, sessions, {
      evaluate: controller.evaluate
    });
    if (!runner) continue;
    runners.push(runner);
    controller.mount(runner.cell);
    controller.loadSource(runner.editor.value).then((source) => {
      runner.editor.value = source;
      runner.editor.rows = Math.min(28, Math.max(5, source.split("\n").length));
      runner.button.click();
    }).catch((error) => controller.setStatus(errorMessage(error), "error"));
  }

  document.addEventListener("hara:reset-session", async (event) => {
    const groupName = String(event.detail?.groupName ?? "").trim();
    if (!groupName) return;

    const matching = runners.filter(({ descriptor }) =>
      descriptor.scope === "group" && descriptor.groupName === groupName);
    if (!matching.length) return;

    const descriptor = matching[0].descriptor;
    matching.forEach((runner) => runner.beginReset());
    try {
      const session = await sessions.reset(descriptor);
      matching.forEach((runner) => runner.finishReset(session));
      document.dispatchEvent(new CustomEvent("hara:session-reset", {
        detail: { groupName, sessionId: descriptor.id }
      }));
    } catch (error) {
      matching.forEach((runner) => runner.failReset(error));
    }
  });
}
