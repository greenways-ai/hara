import { HaraAmpRuntime } from "./amp-runtime.js";

const query = (selector, root = document) => root.querySelector(selector);
const queryAll = (selector, root = document) => [...root.querySelectorAll(selector)];

const stylesheet = document.createElement("link");
stylesheet.rel = "stylesheet";
stylesheet.href = "./story.css?v=amp-story-2";
document.head.append(stylesheet);

const pipelineCopy = {
  synth: {
    label: "01 · SYNTH WASM",
    summary: "Rust generates the sound signal locally using the compiled oscillator shared by the silent probe and the playable instrument.",
    input: "Note, waveform, and playback position",
    output: "Stereo PCM audio samples",
    runtime: "Rust compiled to WebAssembly"
  },
  audio: {
    label: "02 · WEB AUDIO + EQ",
    summary: "After you press Play on the next page, the browser authorizes sound and routes the samples through gain, ten-band EQ, and an analyser.",
    input: "Stereo samples from Synth",
    output: "Audible sound plus analyser samples",
    runtime: "Browser Web Audio API · click required"
  },
  fft: {
    label: "03 · FFT WASM",
    summary: "A second Rust module converts the analyser waveform into frequency-energy bins without uploading any audio.",
    input: "Time-domain analyser samples",
    output: "Frequency bins for one frame",
    runtime: "Rust compiled to WebAssembly"
  },
  hta: {
    label: "04 · HTA TRANSPORT",
    summary: "HTA carries only the newest FFT frame across the runtime boundary, keeping visuals responsive without slowing the audio clock.",
    input: "Successive FFT frames",
    output: "Latest available frame",
    runtime: "Session-local latest-value channel"
  },
  hal: {
    label: "05 · HAL LIVE DOCUMENT",
    summary: "The editable Hara program receives each FFT frame as ordinary data, selects a visual mode, and produces a host-neutral scene description.",
    input: "Latest FFT frame plus visual settings",
    output: "Canvas drawing commands",
    runtime: "Live src/visualizer.hal generation"
  },
  canvas: {
    label: "06 · CANVAS OUTPUT",
    summary: "The browser host executes those drawing commands as the spectrum, scope, or artwork view shown on the next page.",
    input: "Host-neutral drawing commands",
    output: "Pixels in the browser",
    runtime: "Browser Canvas host capability"
  }
};

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

      <div class="story-pipeline" aria-label="Interactive Hara Amp signal pipeline">
        <button type="button" data-amp-node="synth" aria-pressed="true">
          <i>01</i><strong>SYNTH</strong><span>RUST / WASM</span><em data-amp-node-state="synth">WAITING</em>
        </button>
        <span class="story-wire" aria-hidden="true"><b></b></span>
        <button type="button" data-amp-node="audio" aria-pressed="false">
          <i>02</i><strong>AUDIO + EQ</strong><span>WEB AUDIO</span><em data-amp-node-state="audio">WAITING</em>
        </button>
        <span class="story-wire" aria-hidden="true"><b></b></span>
        <button type="button" data-amp-node="fft" aria-pressed="false">
          <i>03</i><strong>FFT</strong><span>RUST / WASM</span><em data-amp-node-state="fft">WAITING</em>
        </button>
        <span class="story-wire" aria-hidden="true"><b></b></span>
        <button type="button" data-amp-node="hta" aria-pressed="false">
          <i>04</i><strong>HTA</strong><span>LATEST VALUE</span><em data-amp-node-state="hta">WAITING</em>
        </button>
        <span class="story-wire" aria-hidden="true"><b></b></span>
        <button type="button" data-amp-node="hal" aria-pressed="false">
          <i>05</i><strong>HAL</strong><span>LIVE DOCUMENT</span><em data-amp-node-state="hal">WAITING</em>
        </button>
        <span class="story-wire" aria-hidden="true"><b></b></span>
        <button type="button" data-amp-node="canvas" aria-pressed="false">
          <i>06</i><strong>CANVAS</strong><span>BROWSER HOST</span><em data-amp-node-state="canvas">WAITING</em>
        </button>
      </div>

      <aside class="story-node-detail" aria-live="polite">
        <header>
          <span data-amp-node-label>${pipelineCopy.synth.label}</span>
          <p data-amp-node-copy>${pipelineCopy.synth.summary}</p>
          <output data-amp-node-error hidden></output>
        </header>
        <dl>
          <div><dt>INPUT</dt><dd data-amp-node-input>${pipelineCopy.synth.input}</dd></div>
          <div><dt>OUTPUT</dt><dd data-amp-node-output>${pipelineCopy.synth.output}</dd></div>
          <div><dt>RUNS IN</dt><dd data-amp-node-runtime>${pipelineCopy.synth.runtime}</dd></div>
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
          <span>src/visualizer.hal</span>
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
          <label>
            <span>EQ CHARACTER</span>
            <select data-story-preset>
              <option value="flat">FLAT</option>
              <option value="hara" selected>HARA GLOW</option>
              <option value="bass">BASS ARC</option>
              <option value="voice">VOICE</option>
            </select>
          </label>
          <fieldset>
            <legend>HAL OUTPUT</legend>
            <button type="button" data-story-mode="spectrum" aria-pressed="true">SPECTRUM</button>
            <button type="button" data-story-mode="scope" aria-pressed="false">SCOPE</button>
            <button type="button" data-story-mode="artwork" aria-pressed="false">ARTWORK</button>
          </fieldset>
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
  const detail = pipelineCopy[name];
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
    }
  });
  instance.setPreset(selectedPreset);
  instance.setVisualMode(selectedMode);
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
  const node = event.target.closest("[data-amp-node]");
  if (node) {
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

  const mode = event.target.closest("[data-story-mode]");
  if (mode) {
    selectedMode = mode.dataset.storyMode;
    amp?.setVisualMode(selectedMode);
    query(".story-visual", story).classList.toggle("is-artwork", selectedMode === "artwork");
    queryAll("[data-story-mode]", story).forEach((button) => {
      button.setAttribute("aria-pressed", String(button === mode));
    });
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

query("[data-story-preset]", story).addEventListener("change", (event) => {
  selectedPreset = event.target.value;
  amp?.setPreset(selectedPreset);
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
