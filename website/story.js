import { HaraAmpRuntime } from "./amp-runtime.js";

const query = (selector, root = document) => root.querySelector(selector);
const queryAll = (selector, root = document) => [...root.querySelectorAll(selector)];

const stylesheet = document.createElement("link");
stylesheet.rel = "stylesheet";
stylesheet.href = "./story.css?v=amp-story-2";
document.head.append(stylesheet);

const story = document.createElement("div");
story.className = "kernel-story";
story.dataset.kernelStory = "";
story.hidden = true;
story.setAttribute("aria-hidden", "true");
story.innerHTML = `
  <section class="story-screen" data-story-screen="1" aria-labelledby="connect-story-title">
    <div class="story-layout story-connect">
      <header class="story-heading">
        <p class="story-step">02 // CONNECT THE SYSTEM</p>
        <h2 id="connect-story-title">Sound becomes<br>a live program.</h2>
        <p>
          Follow one signal from generated samples to browser pixels. Select a
          block to see exactly what enters it, what it does, and what leaves it.
          This page runs a silent probe; audible playback is on Page 03.
        </p>
        <output class="story-runtime-state" data-amp-runtime-state aria-live="polite">
          WAITING FOR RUNTIME
        </output>
      </header>

      <div class="story-pipeline" data-story-pipeline
        aria-label="Interactive Hara Amp signal pipeline"></div>

      <aside class="story-node-detail" aria-live="polite">
        <header>
          <span data-amp-node-label>LOADING GRAPH</span>
          <p data-amp-node-copy>The blocks are read from the live HAL graph.</p>
          <output data-amp-node-error hidden></output>
        </header>
        <dl>
          <div><dt>INPUT</dt><dd data-amp-node-input>—</dd></div>
          <div><dt>OUTPUT</dt><dd data-amp-node-output>—</dd></div>
          <div><dt>RUNS IN</dt><dd data-amp-node-runtime>—</dd></div>
        </dl>
        <small>SELECT EACH BLOCK · PAGE 03 PLAYS THE SIGNAL</small>
      </aside>
    </div>
  </section>

  <section class="story-screen" data-story-screen="2" aria-labelledby="play-story-title">
    <div class="story-layout story-play">
      <header class="story-heading">
        <p class="story-step">03 // PLAY THE SYSTEM</p>
        <h2 id="play-story-title">Change the signal.<br>Keep it live.</h2>
        <p>
          Play the WASM synth, alter its Web Audio character, and switch the HAL
          output. The counters come from the running node connection—not an animation.
        </p>
      </header>

      <section class="story-source" aria-label="Live Hara visualizer source">
        <header>
          <span>src/amp.hal</span>
          <output data-story-source-status>LOADING SOURCE</output>
        </header>
        <textarea data-story-source spellcheck="false" wrap="off"
          aria-label="Editable Hara visualizer source">;; Loading the live .hal file…</textarea>
        <footer>
          <span>CHANGE A PALETTE COLOUR, THEN REBUILD</span>
          <div>
            <button type="button" data-story-source-reset>RESET</button>
            <button type="button" class="story-source-apply" data-story-source-apply>APPLY + REBUILD</button>
          </div>
        </footer>
        <output class="story-source-error" data-story-source-error aria-live="polite" hidden></output>
      </section>

      <section class="story-repl" aria-label="Live Hara Amp REPL">
        <header><span>AMP REPL · ACTIVE DOCUMENT</span><button type="button" data-story-repl-clear>CLEAR</button></header>
        <div data-story-repl-history aria-live="polite"></div>
        <form data-story-repl-form>
          <label for="story-repl-input">HAL</label>
          <input id="story-repl-input" data-story-repl-input
            value="(sonic/status &quot;hara-amp&quot;)" autocomplete="off" spellcheck="false">
          <button type="submit">EVAL</button>
        </form>
      </section>

      <section class="story-amp" aria-label="Compact Hara Amp instrument">
        <div class="story-visual">
          <canvas data-story-visualizer aria-label="Live Hara Amp visualizer"></canvas>
          <img src="./assets/hara-amp/hara-amp-artwork-original.png" alt="" aria-hidden="true">
          <div class="story-no-signal" data-story-no-signal>
            <strong>SILENT PROBE READY</strong>
            <span>PRESS PLAY TO AUTHORIZE AUDIO</span>
          </div>
          <output data-story-frame-status>HAL · PROBE</output>
        </div>

        <div class="story-controls">
          <button type="button" class="story-play-toggle" data-story-play aria-pressed="false">
            <i aria-hidden="true">▶</i><span>PLAY SIGNAL</span>
          </button>
          <div data-story-controls></div>
        </div>

        <dl class="story-telemetry" aria-label="Live kernel telemetry">
          <div><dt>FFT → HTA</dt><dd data-story-emitted>0000</dd></div>
          <div><dt>HAL → CANVAS</dt><dd data-story-rendered>0000</dd></div>
          <div><dt>QUEUE</dt><dd data-story-queue>0 / LATEST</dd></div>
          <div><dt>AUDIO</dt><dd data-story-audio>GESTURE REQUIRED</dd></div>
        </dl>
      </section>

      <footer class="story-closeout">
        <p>
          <strong>MAKE IT YOURS.</strong>
          Create the complete Hara Amp workspace with this EQ, visual mode, and edited HAL file.
          Greenways OS can carry the same live project into the page where you work.
        </p>
        <div class="story-actions">
          <button type="button" class="story-primary" data-story-create>CREATE THIS WORKSPACE</button>
          <a href="./hara-amp.html" target="_blank" rel="noopener">OPEN FULL AMP</a>
        </div>
        <output class="story-inline-error" data-story-error aria-live="polite" hidden></output>
      </footer>
    </div>
  </section>`;

document.body.append(story);

const start = query("[data-start]");
const previous = query("[data-workspace-prev]");
const next = query("[data-workspace-next]");
let activeScreen = 0;
let amp = null;
let ampBoot = null;
let selectedPreset = "hara";
let selectedMode = "spectrum";
let selectedNode = null;

function runtimeReady() {
  return document.body.dataset.kernel === "live";
}

function updateNavigation() {
  if (activeScreen === 0) {
    previous.disabled = true;
    next.disabled = !runtimeReady();
    start.disabled = !runtimeReady();
    return;
  }
  previous.disabled = false;
  next.disabled = activeScreen >= 2;
  start.disabled = false;
}

function selectNode(name) {
  const detail = amp?.graphSnapshot?.nodes.find((node) => node.id === name);
  if (!detail) return;
  queryAll("[data-amp-node]", story).forEach((button) => {
    button.setAttribute("aria-pressed", String(button.dataset.ampNode === name));
  });
  query("[data-amp-node-label]", story).textContent = detail.label;
  query("[data-amp-node-copy]", story).textContent = detail.summary;
  query("[data-amp-node-input]", story).textContent = detail.input;
  query("[data-amp-node-output]", story).textContent = detail.output;
  query("[data-amp-node-runtime]", story).textContent = detail.runtime;
  const error = query("[data-amp-node-error]", story);
  const stage = query(`[data-amp-node="${name}"]`, story);
  const message = stage?.dataset.error;
  error.hidden = !message;
  error.textContent = message ? `ERROR · ${message}` : "";
}

function renderGraph(snapshot) {
  const pipeline = query("[data-story-pipeline]", story);
  pipeline.replaceChildren();
  snapshot.nodes.forEach((node, index) => {
    if (index) {
      const wire = document.createElement("span");
      wire.className = "story-wire";
      wire.setAttribute("aria-hidden", "true");
      wire.innerHTML = "<b></b>";
      pipeline.append(wire);
    }
    const button = document.createElement("button");
    button.type = "button";
    button.dataset.ampNode = node.id;
    button.dataset.state = snapshot.status === "running" ? "ready" : snapshot.status;
    button.setAttribute("aria-pressed", String((selectedNode ?? snapshot.nodes[0]?.id) === node.id));
    button.innerHTML =
      `<i>${String(index + 1).padStart(2, "0")}</i>` +
      `<strong>${escapeHtml(node.label.replace(/^\\d+\\s*·\\s*/, ""))}</strong>` +
      `<span>${escapeHtml(node.type.toUpperCase())}</span>` +
      `<em data-amp-node-state="${escapeHtml(node.id)}">${snapshot.status.toUpperCase()}</em>`;
    pipeline.append(button);
  });
  selectedNode = snapshot.nodes.some((node) => node.id === selectedNode)
    ? selectedNode : snapshot.nodes[0]?.id;
  selectedPreset = snapshot.nodes.find((node) => node.id === "eq")?.params.character
    ?? selectedPreset;
  selectedMode = snapshot.nodes.find((node) => node.id === "visualizer")?.params.mode
    ?? selectedMode;
  if (selectedNode) selectNode(selectedNode);
  renderControls(snapshot);
}

function renderControls(snapshot) {
  const root = query("[data-story-controls]", story);
  root.replaceChildren();
  for (const node of snapshot.nodes) {
    for (const control of node.controls ?? []) {
      const label = document.createElement("label");
      label.dataset.graphNode = node.id;
      label.dataset.graphParameter = control.parameter;
      const title = document.createElement("span");
      title.textContent = control.label;
      const value = node.params[control.parameter];
      let input;
      if (control.type === "choice") {
        input = document.createElement("select");
        for (const candidate of control.choices) {
          const option = document.createElement("option");
          option.value = typeof candidate === "object" ? candidate.value : candidate;
          option.textContent = typeof candidate === "object"
            ? candidate.label : String(candidate).toUpperCase();
          input.append(option);
        }
      } else if (control.type === "boolean") {
        input = document.createElement("input");
        input.type = "checkbox";
      } else if (control.type === "steps") {
        const grid = document.createElement("div");
        grid.className = "story-step-grid";
        grid.dataset.graphNode = node.id;
        grid.dataset.graphParameter = control.parameter;
        value.forEach((step, index) => {
          const input = document.createElement("input");
          input.type = "number";
          input.min = -48;
          input.max = 48;
          input.step = 1;
          input.value = step ?? "";
          input.placeholder = "—";
          input.title = `Step ${index + 1}; blank is a rest`;
          input.setAttribute("aria-label", `Sequence step ${index + 1}`);
          grid.append(input);
        });
        const heading = document.createElement("span");
        heading.textContent = `${control.label} · BLANK = REST`;
        const remove = document.createElement("button");
        remove.type = "button";
        remove.dataset.stepAction = "remove";
        remove.textContent = "− STEP";
        remove.disabled = value.length <= 1;
        const add = document.createElement("button");
        add.type = "button";
        add.dataset.stepAction = "add";
        add.textContent = "+ STEP";
        add.disabled = value.length >= 64;
        grid.prepend(heading, remove, add);
        root.append(grid);
        continue;
      } else {
        input = document.createElement("input");
        input.type = "range";
        if (control.min != null) input.min = control.min;
        if (control.max != null) input.max = control.max;
        if (control.step != null) input.step = control.step;
      }
      if (input.type === "checkbox") input.checked = Boolean(value);
      else input.value = value;
      label.append(title, input);
      root.append(label);
    }
  }
}

function updateNode(stage, state, detail) {
  const output = query(`[data-amp-node-state="${stage}"]`, story);
  const button = query(`[data-amp-node="${stage}"]`, story);
  if (output) {
    output.textContent = state === "gesture" ? "GESTURE" : state.toUpperCase();
  }
  if (button) button.dataset.state = state;
  if (button) {
    if (state === "error" && detail) button.dataset.error = detail;
    else delete button.dataset.error;
    if (button.getAttribute("aria-pressed") === "true") selectNode(stage);
  }
  if (stage === "runtime") {
    const runtimeState = query("[data-amp-runtime-state]", story);
    runtimeState.textContent =
      state === "ready" ? "LIVE // SILENT PROBE COMPLETED" :
      state === "error" ? `UNAVAILABLE // ${detail}` :
      detail?.toUpperCase() || "STARTING";
    runtimeState.dataset.state = state;
  }
}

function createAmp() {
  const instance = new HaraAmpRuntime({
    canvas: query("[data-story-visualizer]", story),
    dbName: "hara-story-amp",
    onStatus({ stage, state, detail }) {
      updateNode(stage, state, detail);
      if (stage === "hal") {
        const sourceStatus = query("[data-story-source-status]", story);
        sourceStatus.textContent =
          state === "ready" ? `GEN ${instance.generation} // LIVE` :
          state === "error" ? "REBUILD FAILED // PREVIOUS GEN LIVE" :
          detail?.toUpperCase() || "REBUILDING";
        sourceStatus.dataset.state = state;
      }
      if (state === "error") showError(detail);
    },
    onFrame({ count }) {
      query("[data-story-frame-status]", story).textContent = `HAL · FRAME ${count}`;
    },
    onPlayback({ state }) {
      const playing = state === "playing";
      const control = query("[data-story-play]", story);
      control.setAttribute("aria-pressed", String(playing));
      query("i", control).textContent = playing ? "Ⅱ" : "▶";
      query("span", control).textContent = playing ? "PAUSE SIGNAL" : "PLAY SIGNAL";
      query("[data-story-no-signal]", story).classList.toggle("is-hidden", playing);
      query("[data-story-audio]", story).textContent =
        playing ? "PLAYING / WASM" : state.toUpperCase();
    },
    onTelemetry({ emittedFrames, renderedFrames, nodeQueued }) {
      query("[data-story-emitted]", story).textContent = String(emittedFrames).padStart(4, "0");
      query("[data-story-rendered]", story).textContent = String(renderedFrames).padStart(4, "0");
      query("[data-story-queue]", story).textContent = `${nodeQueued} / LATEST`;
    },
    onGraph(snapshot) { renderGraph(snapshot); }
  });
  return instance;
}

function ensureAmp() {
  if (!amp) amp = createAmp();
  if (!ampBoot) {
    ampBoot = amp.boot().catch((error) => {
      showError(`The live Amp could not start: ${String(error?.message ?? error)}`);
      throw error;
    });
  }
  return ampBoot.then((instance) => {
    const editor = query("[data-story-source]", story);
    if (!editor.dataset.loaded) {
      editor.value = instance.source;
      editor.dataset.loaded = "true";
      query("[data-story-source-status]", story).textContent = `GEN ${instance.generation} // LIVE`;
    }
    return instance;
  });
}

async function disposeAmp() {
  const closing = amp;
  amp = null;
  ampBoot = null;
  if (closing) await closing.dispose();
}

function showError(message) {
  const output = query("[data-story-error]", story);
  output.hidden = false;
  output.textContent = message;
}

async function rebuildSource({ reset = false } = {}) {
  const editor = query("[data-story-source]", story);
  const apply = query("[data-story-source-apply]", story);
  const resetButton = query("[data-story-source-reset]", story);
  const status = query("[data-story-source-status]", story);
  const errorOutput = query("[data-story-source-error]", story);
  apply.disabled = true;
  resetButton.disabled = true;
  errorOutput.hidden = true;
  status.textContent = "PREPARING NEW GENERATION";
  status.dataset.state = "loading";
  try {
    const instance = await ensureAmp();
    if (reset) editor.value = instance.originalSource;
    const result = await instance.rebuild(editor.value);
    status.textContent = `GEN ${result.generation} // LIVE`;
    status.dataset.state = "ready";
  } catch (error) {
    status.textContent = `GEN ${amp?.generation ?? 0} // PREVIOUS VERSION LIVE`;
    status.dataset.state = "error";
    errorOutput.hidden = false;
    errorOutput.textContent = String(error?.message ?? error);
  } finally {
    apply.disabled = false;
    resetButton.disabled = false;
  }
}

function showScreen(index) {
  const previousScreen = activeScreen;
  activeScreen = Math.max(0, Math.min(2, index));
  const open = activeScreen > 0;
  story.hidden = !open;
  story.setAttribute("aria-hidden", String(!open));

  if (previousScreen === 2 && activeScreen !== 2) amp?.pause();
  if (!open) {
    delete document.body.dataset.storyScreen;
    queryAll("[data-story-screen]", story).forEach((screen) => screen.classList.remove("is-active"));
    void disposeAmp();
    updateNavigation();
    start?.focus();
    return;
  }

  document.body.dataset.storyScreen = String(activeScreen);
  queryAll("[data-story-screen]", story).forEach((screen) => {
    screen.classList.toggle("is-active", Number(screen.dataset.storyScreen) === activeScreen);
  });
  updateNavigation();
  void ensureAmp()
    .then(() => amp?.setCanvas(query("[data-story-visualizer]", story)))
    .catch(() => {});
  query(".story-screen.is-active .story-heading", story)?.focus({ preventScroll: true });
}

start.addEventListener("click", (event) => {
  if (!runtimeReady()) return;
  event.preventDefault();
  event.stopImmediatePropagation();
  showScreen(1);
}, true);

previous.addEventListener("click", (event) => {
  if (!activeScreen) return;
  event.preventDefault();
  event.stopImmediatePropagation();
  showScreen(activeScreen - 1);
}, true);

next.addEventListener("click", (event) => {
  if (!activeScreen && runtimeReady()) {
    event.preventDefault();
    event.stopImmediatePropagation();
    showScreen(1);
    return;
  }
  if (activeScreen === 1) {
    event.preventDefault();
    event.stopImmediatePropagation();
    showScreen(2);
  }
}, true);

story.addEventListener("click", (event) => {
  const stepAction = event.target.closest("[data-step-action]");
  if (stepAction) {
    const grid = stepAction.closest(".story-step-grid");
    const inputs = queryAll("input", grid);
    if (stepAction.dataset.stepAction === "add" && inputs.length < 64) {
      const input = inputs.at(-1).cloneNode();
      input.value = "";
      input.setAttribute("aria-label", `Sequence step ${inputs.length + 1}`);
      grid.append(input);
    } else if (stepAction.dataset.stepAction === "remove" && inputs.length > 1) {
      inputs.at(-1).remove();
    }
    void applyStepGrid(grid);
    return;
  }

  const node = event.target.closest("[data-amp-node]");
  if (node) {
    selectedNode = node.dataset.ampNode;
    selectNode(node.dataset.ampNode);
    return;
  }

  if (event.target.closest("[data-story-play]")) {
    const control = query("[data-story-play]", story);
    control.disabled = true;
    const action = amp?.audio?.playing ? Promise.resolve(amp.pause()) : ensureAmp().then(() => amp.play());
    action.catch((error) => showError(`Audio could not start: ${String(error?.message ?? error)}`))
      .finally(() => { control.disabled = false; });
    return;
  }

  if (event.target.closest("[data-story-source-apply]")) {
    void rebuildSource();
    return;
  }

  if (event.target.closest("[data-story-source-reset]")) {
    void rebuildSource({ reset: true });
    return;
  }

  if (event.target.closest("[data-story-create]")) {
    const control = query("[data-story-create]", story);
    control.disabled = true;
    control.textContent = "CREATING…";
    document.dispatchEvent(new CustomEvent("hara:create-amp-workspace", {
      detail: {
        preset: selectedPreset,
        mode: selectedMode,
        source: query("[data-story-source]", story).value
      }
    }));
  }
});

query("[data-story-controls]", story).addEventListener("change", async (event) => {
  const label = event.target.closest("[data-graph-node]");
  if (!label || !amp) return;
  if (label.classList.contains("story-step-grid")) {
    await applyStepGrid(label);
    return;
  }
  const value = event.target.type === "checkbox" ? event.target.checked
    : event.target.type === "range" ? Number(event.target.value) : event.target.value;
  await amp.update(label.dataset.graphNode, label.dataset.graphParameter, value);
  if (label.dataset.graphNode === "visualizer") {
    selectedMode = String(value);
    query(".story-visual", story).classList.toggle("is-artwork", selectedMode === "artwork");
  }
  if (label.dataset.graphNode === "eq") selectedPreset = String(value);
});

async function applyStepGrid(grid) {
  if (!amp) return;
  const value = queryAll("input", grid)
    .map((input) => input.value === "" ? null : Number(input.value));
  await amp.update(grid.dataset.graphNode, grid.dataset.graphParameter, value);
}

query("[data-story-repl-form]", story).addEventListener("submit", async (event) => {
  event.preventDefault();
  const input = query("[data-story-repl-input]", story);
  const form = input.value.trim();
  if (!form) return;
  const history = query("[data-story-repl-history]", story);
  const entry = document.createElement("div");
  entry.innerHTML = `<code>${escapeHtml(form)}</code><output>…</output>`;
  history.append(entry);
  try {
    const value = await ensureAmp().then((instance) => instance.eval(form));
    query("output", entry).textContent = renderReplValue(value);
    entry.dataset.state = "ready";
  } catch (error) {
    query("output", entry).textContent = String(error?.message ?? error);
    entry.dataset.state = "error";
  }
  history.scrollTop = history.scrollHeight;
});

query("[data-story-repl-clear]", story).addEventListener("click", () => {
  query("[data-story-repl-history]", story).replaceChildren();
});

query("[data-story-source]", story).addEventListener("input", () => {
  const status = query("[data-story-source-status]", story);
  status.textContent = `GEN ${amp?.generation ?? 0} // CHANGED`;
  status.dataset.state = "changed";
});

document.addEventListener("hara:amp-workspace-error", (event) => {
  const control = query("[data-story-create]", story);
  control.disabled = false;
  control.textContent = "CREATE THIS WORKSPACE";
  showError(`Workspace could not be created: ${event.detail?.message ?? "Unknown error"}`);
});

document.addEventListener("keydown", (event) => {
  if (!activeScreen || event.defaultPrevented || event.metaKey || event.ctrlKey || event.altKey) return;
  if (event.target.closest?.("button, a, input, select, textarea")) return;
  if (event.key === "ArrowLeft") {
    event.preventDefault();
    showScreen(activeScreen - 1);
  } else if (event.key === "ArrowRight" && activeScreen < 2) {
    event.preventDefault();
    showScreen(activeScreen + 1);
  } else if (event.key === "Escape") {
    event.preventDefault();
    showScreen(0);
  }
}, true);

new MutationObserver(() => {
  if (document.body.dataset.workspace === "1" && activeScreen) {
    activeScreen = 0;
    story.hidden = true;
    story.setAttribute("aria-hidden", "true");
    delete document.body.dataset.storyScreen;
    void disposeAmp();
  }
  updateNavigation();
}).observe(document.body, {
  attributes: true,
  attributeFilter: ["data-kernel", "data-workspace"]
});

updateNavigation();

function escapeHtml(value) {
  return String(value).replace(/[&<>"']/g, (character) => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;"
  })[character]);
}

function renderReplValue(value) {
  if (value instanceof Map) {
    return JSON.stringify(Object.fromEntries([...value].map(([key, item]) =>
      [String(key?.name ?? key), replPlain(item)])));
  }
  if (typeof value === "string") return value;
  return JSON.stringify(replPlain(value)) ?? String(value);
}

function replPlain(value) {
  if (value instanceof Map) {
    return Object.fromEntries([...value].map(([key, item]) =>
      [String(key?.name ?? key), replPlain(item)]));
  }
  if (Array.isArray(value)) return value.map(replPlain);
  return value;
}
