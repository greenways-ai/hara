import { HtaKeyword } from "./runtime/hta.js";
import { createBrowserBroker } from "./runtime/studio/broker.js";
import { createHostServices } from "./runtime/studio/host-services.js";
import { NodeRuntime } from "./runtime/studio/node-runtime.js";

const $ = (selector, root = document) => root.querySelector(selector);
const $$ = (selector, root = document) => [...root.querySelectorAll(selector)];
const FREQUENCIES = [31, 62, 125, 250, 500, 1000, 2000, 4000, 8000, 16000];
const PRESETS = {
  flat: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
  hara: [3, 2, 0, -1, 1, 3, 5, 4, 2, 3],
  bass: [8, 7, 5, 2, 0, -1, -2, -2, -1, 0],
  voice: [-3, -2, 0, 3, 5, 5, 3, 1, 0, -2]
};

const state = {
  runtime: new NodeRuntime({ space: "workspace/hara-amp" }),
  broker: null,
  engine: null,
  visualizerRunning: false,
  visualMode: "spectrum",
  tracks: [],
  activeTrack: 0,
  peaks: new Float32Array(96),
  lastFrame: null,
  frameCount: 0,
  emittedFrames: 0
};

for (const node of [
  { id: "node/synth", type: "wasm/audio-source" },
  { id: "node/fft", type: "wasm/transform" },
  { id: "node/visualizer", type: "hal/transform" }
]) state.runtime.registerNode(node);
state.runtime.connect({
  id: "connection/fft-visualizer",
  from: ["node/fft", "fft/bins"],
  to: ["node/visualizer", "fft/bins"],
  transport: "hta",
  delivery: "latest",
  capacity: 1
});

class AmpAudio {
  constructor(synth, fft) {
    this.synth = synth;
    this.fft = fft;
    this.context = null;
    this.buffer = null;
    this.source = null;
    this.startedAt = 0;
    this.offset = 0;
    this.duration = 4;
    this.playing = false;
    this.loop = true;
    this.eqEnabled = true;
    this.raf = 0;
  }

  async initialize() {
    if (this.context) return;
    this.context = new AudioContext({ latencyHint: "interactive" });
    this.preamp = new GainNode(this.context, { gain: 1 });
    this.filters = FREQUENCIES.map((frequency) =>
      new BiquadFilterNode(this.context, { type: "peaking", frequency, Q: 1.15, gain: 0 })
    );
    this.balance = new StereoPannerNode(this.context, { pan: 0 });
    this.master = new GainNode(this.context, { gain: .78 });
    this.analyser = new AnalyserNode(this.context, { fftSize: 2048, smoothingTimeConstant: 0 });
    let previous = this.preamp;
    for (const filter of this.filters) {
      previous.connect(filter);
      previous = filter;
    }
    previous.connect(this.balance).connect(this.master).connect(this.analyser).connect(this.context.destination);
    this.timeData = new Float32Array(this.analyser.fftSize);
    this.buffer = this.renderSynth();
    this.duration = this.buffer.duration;
    this.visualLoop();
  }

  renderSynth() {
    const sampleRate = this.context.sampleRate;
    const frames = Math.round(sampleRate * 4);
    const buffer = new AudioBuffer({ length: frames, sampleRate, numberOfChannels: 2 });
    const mono = new Float32Array(frames);
    const { memory, synth_buffer, synth_capacity, synth_fill } = this.synth.exports;
    const capacity = Number(synth_capacity());
    for (let start = 0; start < frames; start += capacity) {
      const count = Number(synth_fill(BigInt(start), Math.min(capacity, frames - start), sampleRate));
      mono.set(new Float32Array(memory.buffer, Number(synth_buffer()), count), start);
    }
    buffer.copyToChannel(mono, 0);
    buffer.copyToChannel(mono, 1);
    return buffer;
  }

  async play(offset = this.offset) {
    await this.initialize();
    await this.context.resume();
    if (this.source) this.source.stop();
    this.source = new AudioBufferSourceNode(this.context, { buffer: this.buffer, loop: this.loop });
    this.source.connect(this.preamp);
    this.offset = Math.max(0, Math.min(offset, this.duration - .001));
    this.startedAt = this.context.currentTime - this.offset;
    this.source.start(0, this.offset);
    this.source.onended = () => {
      if (!this.loop && this.playing) this.stop();
    };
    this.playing = true;
    $("[data-audio-status]").textContent = "PLAYING / WASM";
    $("[data-visual-empty]").classList.add("is-hidden");
  }

  pause() {
    if (!this.playing) return;
    this.offset = this.position();
    this.source?.stop();
    this.source = null;
    this.playing = false;
    $("[data-audio-status]").textContent = "PAUSED";
  }

  stop() {
    this.source?.stop();
    this.source = null;
    this.offset = 0;
    this.playing = false;
    $("[data-audio-status]").textContent = "STOPPED";
    updateTime();
  }

  position() {
    if (!this.context || !this.playing) return this.offset;
    const elapsed = this.context.currentTime - this.startedAt;
    return this.loop ? elapsed % this.duration : Math.min(elapsed, this.duration);
  }

  setEq(values) {
    this.filters?.forEach((filter, index) => {
      filter.gain.setTargetAtTime(this.eqEnabled ? values[index] : 0, this.context.currentTime, .02);
    });
  }

  async loadFile(file) {
    await this.initialize();
    this.buffer = await this.context.decodeAudioData(await file.arrayBuffer());
    this.duration = this.buffer.duration;
    this.offset = 0;
  }

  visualLoop() {
    const tick = () => {
      this.raf = requestAnimationFrame(tick);
      updateTime();
      if (!this.playing || !state.visualizerRunning) return;
      this.analyser.getFloatTimeDomainData(this.timeData);
      const inputFrames = 1024;
      const { memory, fft_input, fft_output, fft_compute } = this.fft.exports;
      new Float32Array(memory.buffer, Number(fft_input()), inputFrames)
        .set(this.timeData.subarray(0, inputFrames));
      const count = Number(fft_compute(inputFrames, 96));
      const magnitudes = new Float32Array(memory.buffer, Number(fft_output()), count);
      const bins = [...magnitudes].map((value) => Math.min(255, Math.round(value * 510)));
      const wave = [...this.timeData.subarray(0, inputFrames).filter((_, index) => index % 8 === 0)]
        .map((value) => Math.max(-127, Math.min(127, Math.round(value * 127))));
      // Never await visualization: latest delivery may drop stale frames and
      // audio continues independently.
      state.emittedFrames += 1;
      document.documentElement.dataset.emittedFrames = String(state.emittedFrames);
      state.runtime.emit("node/fft", "fft/bins", new Map([["bins", bins], ["wave", wave]])).catch((error) => {
        console.error("[hara amp node emit]", error);
      });
      const queue = state.runtime.nodes.get("node/visualizer")?.inputs.get("fft/bins");
      document.documentElement.dataset.nodeWaiters = String(queue?.waiters.length ?? 0);
      document.documentElement.dataset.nodeQueued = String(queue?.values.length ?? 0);
    };
    tick();
  }
}

async function boot() {
  const status = $("[data-runtime-status]");
  try {
    const [runtimeBytes, synth, fft, nodeSource, drawSource, protocolSource, frameSource, substrateSource, visualizerSource] = await Promise.all([
      bytes("./runtime/hara.wasm"),
      wasm("./assets/wasm/demo-synth.wasm"),
      wasm("./assets/wasm/demo-fft.wasm"),
      text("./runtime/studio/hal/node.hal"),
      text("./runtime/studio/hal/draw.hal"),
      text("./runtime/std/substrate/protocol.hal"),
      text("./runtime/std/substrate/frame.hal"),
      text("./runtime/std/substrate.hal"),
      text("./examples/hara-amp/src/visualizer.hal")
    ]);
    state.engine = new AmpAudio(synth, fft);
    const hostCalls = createHostServices({
      dbName: "hara-amp",
      nodeRuntime: state.runtime,
      renderCanvas
    });
    state.broker = createBrowserBroker({
      workerUrl: new URL("./runtime/hta-worker.js", import.meta.url),
      moduleBytes: runtimeBytes,
      hostCalls,
      resources: {
        "studio.node": nodeSource,
        "studio.draw": drawSource,
        "std.substrate.protocol": protocolSource,
        "std.substrate.frame": frameSource,
        "std.substrate": substrateSource
      }
    });
    const prepared = await state.broker.prepareDocument("ROOT", "document/visualizer", visualizerSource, {
      nodeId: "node/visualizer"
    });
    try {
      await state.runtime.activateDocument("node/visualizer", {
        documentId: "document/visualizer",
        generation: prepared.generation,
        moduleId: prepared.moduleId,
        kernelContext: prepared.context,
        prepare: (node) => {
          node.start(() => state.broker.evalPreparedDocument(
            prepared,
            "(run-visualizer)"
          ).catch((error) => {
            document.documentElement.dataset.taskError = String(error?.message ?? error);
            throw error;
          }));
        }
      });
      state.broker.commitDocument(prepared);
    } catch (error) {
      state.broker.discardDocument(prepared);
      throw error;
    }
    state.visualizerRunning = true;
    status.textContent = "WASM · LIVE / NS+ G1";
    status.classList.add("is-live");
    $("[data-frame-status]").textContent = "HAL · ARMED";
  } catch (error) {
    console.error("[hara amp]", error);
    status.textContent = `WASM · ERROR / ${error.message}`;
    status.classList.add("is-error");
    $("[data-frame-status]").textContent = "HAL · ERROR";
  }
}

function renderCanvas(canvasId, scene) {
  if (canvasId !== "canvas/visualizer") return;
  const bins = mapValue(scene, "bins") ?? [];
  const wave = mapValue(scene, "wave") ?? [];
  const palette = mapValue(scene, "palette") ?? ["#2fffe0", "#149df2", "#9b35ff"];
  state.lastFrame = { bins, wave, palette };
  state.frameCount += 1;
  document.documentElement.dataset.renderedFrames = String(state.frameCount);
  drawFrame();
  $("[data-frame-status]").textContent = `HAL · FRAME ${state.frameCount}`;
}

function drawFrame() {
  if (!state.lastFrame) return;
  const canvas = $("[data-visualizer]");
  const rect = canvas.getBoundingClientRect();
  const ratio = Math.min(2, devicePixelRatio || 1);
  const width = Math.max(1, Math.round(rect.width));
  const height = Math.max(1, Math.round(rect.height));
  if (canvas.width !== width * ratio || canvas.height !== height * ratio) {
    canvas.width = width * ratio;
    canvas.height = height * ratio;
  }
  const context = canvas.getContext("2d");
  context.setTransform(ratio, 0, 0, ratio, 0, 0);
  context.clearRect(0, 0, width, height);
  const { bins, wave, palette } = state.lastFrame;
  if (state.visualMode === "scope") drawScope(context, width, height, wave, palette);
  else drawSpectrum(context, width, height, bins, palette, state.visualMode === "artwork");
}

function drawSpectrum(context, width, height, bins, palette, overlay) {
  const gradient = context.createLinearGradient(0, height, width, 0);
  palette.forEach((color, index) => gradient.addColorStop(index / Math.max(1, palette.length - 1), color));
  const gap = 2;
  const barWidth = Math.max(1, width / bins.length - gap);
  bins.forEach((value, index) => {
    const normalized = value / 255;
    const barHeight = Math.max(2, normalized * height * .84);
    const x = index * width / bins.length;
    state.peaks[index] = Math.max(normalized, state.peaks[index] - .018);
    context.globalAlpha = overlay ? .58 : .92;
    context.fillStyle = gradient;
    context.fillRect(x, height - barHeight, barWidth, barHeight);
    context.globalAlpha = 1;
    context.fillRect(x, height - state.peaks[index] * height * .84 - 2, barWidth, 2);
  });
}

function drawScope(context, width, height, wave, palette) {
  context.strokeStyle = palette[0];
  context.shadowColor = palette[1];
  context.shadowBlur = 12;
  context.lineWidth = 2;
  context.beginPath();
  wave.forEach((value, index) => {
    const x = index / Math.max(1, wave.length - 1) * width;
    const y = height * .5 - value / 127 * height * .42;
    if (index === 0) context.moveTo(x, y); else context.lineTo(x, y);
  });
  context.stroke();
  context.shadowBlur = 0;
}

function installUi() {
  buildEq();
  state.tracks = [{ title: "Hara Signal", detail: "SYNTH.WASM", kind: "synth", duration: 4 }];
  $("[data-play]").addEventListener("click", () => state.engine?.play());
  $("[data-pause]").addEventListener("click", () => state.engine?.pause());
  $("[data-stop]").addEventListener("click", () => state.engine?.stop());
  $("[data-previous]").addEventListener("click", () => selectRelative(-1));
  $("[data-next]").addEventListener("click", () => selectRelative(1));
  $("[data-repeat]").addEventListener("click", (event) => {
    const active = event.currentTarget.getAttribute("aria-pressed") !== "true";
    event.currentTarget.setAttribute("aria-pressed", String(active));
    if (state.engine) state.engine.loop = active;
  });
  $("[data-shuffle]").addEventListener("click", (event) => {
    const active = event.currentTarget.getAttribute("aria-pressed") !== "true";
    event.currentTarget.setAttribute("aria-pressed", String(active));
  });
  $("[data-volume]").addEventListener("input", (event) => {
    if (state.engine?.master) state.engine.master.gain.value = event.target.valueAsNumber / 100;
  });
  $("[data-balance]").addEventListener("input", (event) => {
    if (state.engine?.balance) state.engine.balance.pan.value = event.target.valueAsNumber / 100;
  });
  $("[data-seek]").addEventListener("change", (event) => {
    if (!state.engine) return;
    const offset = event.target.valueAsNumber / 1000 * state.engine.duration;
    if (state.engine.playing) state.engine.play(offset); else state.engine.offset = offset;
  });
  $("[data-local-file]").addEventListener("change", addLocalFiles);
  $("[data-clear-local]").addEventListener("click", clearLocal);
  $$("[data-visual-mode]").forEach((button) => button.addEventListener("click", () => {
    state.visualMode = button.dataset.visualMode;
    $$("[data-visual-mode]").forEach((item) =>
      item.setAttribute("aria-pressed", String(item === button))
    );
    $(".visual-wrap").classList.toggle("is-artwork", state.visualMode === "artwork");
    drawFrame();
  }));
  addEventListener("resize", drawFrame);
}

function buildEq() {
  const root = $("[data-eq-bands]");
  const values = [0, ...PRESETS.flat];
  const labels = ["PRE", ...FREQUENCIES.map((frequency) => frequency >= 1000 ? `${frequency / 1000}K` : String(frequency))];
  values.forEach((value, index) => {
    const label = document.createElement("label");
    label.className = "eq-band";
    label.innerHTML = `<span>${labels[index]}</span><input type="range" min="-12" max="12" step=".5" value="${value}" data-eq-index="${index}"><output>0</output>`;
    root.append(label);
  });
  root.addEventListener("input", applyEq);
  $("[data-eq-enable]").addEventListener("click", (event) => {
    if (!state.engine) return;
    state.engine.eqEnabled = !state.engine.eqEnabled;
    event.currentTarget.setAttribute("aria-pressed", String(state.engine.eqEnabled));
    event.currentTarget.textContent = state.engine.eqEnabled ? "ENABLED" : "BYPASS";
    applyEq();
  });
  $("[data-eq-preset]").addEventListener("change", (event) => {
    $$("[data-eq-index]").slice(1).forEach((input, index) => {
      input.value = PRESETS[event.target.value][index];
    });
    applyEq();
  });
}

function applyEq() {
  const inputs = $$("[data-eq-index]");
  inputs.forEach((input) => input.nextElementSibling.textContent = Number(input.value).toFixed(1));
  if (!state.engine?.context) return;
  state.engine.preamp.gain.setTargetAtTime(10 ** (inputs[0].valueAsNumber / 20), state.engine.context.currentTime, .02);
  state.engine.setEq(inputs.slice(1).map((input) => input.valueAsNumber));
}

async function addLocalFiles(event) {
  for (const file of event.target.files) {
    const track = { title: file.name.replace(/\.[^.]+$/, ""), detail: file.type || "LOCAL AUDIO", kind: "file", file };
    state.tracks.push(track);
  }
  persistPlaylistMetadata();
  renderPlaylist();
}

function clearLocal() {
  state.tracks = state.tracks.filter((track) => track.kind === "synth");
  state.activeTrack = 0;
  persistPlaylistMetadata();
  renderPlaylist();
}

function persistPlaylistMetadata() {
  localStorage.setItem("hara-amp.playlist.v1", JSON.stringify(
    state.tracks.filter((track) => track.kind === "file").map(({ title, detail }) => ({ title, detail }))
  ));
}

async function selectTrack(index) {
  const track = state.tracks[index];
  if (!track || !state.engine) return;
  state.engine.stop();
  state.activeTrack = index;
  if (track.kind === "file") await state.engine.loadFile(track.file);
  else {
    await state.engine.initialize();
    state.engine.buffer = state.engine.renderSynth();
    state.engine.duration = state.engine.buffer.duration;
  }
  $("[data-track-title]").textContent = track.title;
  $("[data-track-detail]").textContent = track.detail;
  renderPlaylist();
  await state.engine.play();
}

function selectRelative(direction) {
  if (!state.tracks.length) return;
  const shuffle = $("[data-shuffle]").getAttribute("aria-pressed") === "true";
  const index = shuffle
    ? Math.floor(Math.random() * state.tracks.length)
    : (state.activeTrack + direction + state.tracks.length) % state.tracks.length;
  selectTrack(index);
}

function renderPlaylist() {
  const root = $("[data-playlist]");
  root.replaceChildren();
  state.tracks.forEach((track, index) => {
    const item = document.createElement("li");
    item.classList.toggle("is-active", index === state.activeTrack);
    item.innerHTML = `<b>${String(index + 1).padStart(2, "0")}</b><span>${escapeHtml(track.title)}</span><small>${escapeHtml(track.detail)}</small>`;
    item.addEventListener("click", () => selectTrack(index));
    root.append(item);
  });
}

function updateTime() {
  const engine = state.engine;
  const position = engine?.position() ?? 0;
  const duration = engine?.duration ?? 4;
  $("[data-elapsed]").textContent = time(position);
  $("[data-remaining]").textContent = `-${time(Math.max(0, duration - position))}`;
  $("[data-seek]").value = duration ? Math.round(position / duration * 1000) : 0;
}

function mapValue(map, name) {
  if (!(map instanceof Map)) return map?.[name];
  for (const [key, value] of map) {
    if (key === name || key instanceof HtaKeyword && key.name === name) return value;
  }
}

const time = (seconds) => `${String(Math.floor(seconds / 60)).padStart(2, "0")}:${String(Math.floor(seconds % 60)).padStart(2, "0")}`;
const escapeHtml = (value) => String(value).replace(/[&<>"']/g, (character) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[character]);
async function text(url) { const response = await fetch(url); if (!response.ok) throw new Error(`${url}: ${response.status}`); return response.text(); }
async function bytes(url) { return new Uint8Array(await (await fetch(url)).arrayBuffer()); }
async function wasm(url) { return (await WebAssembly.instantiate(await bytes(url), {})).instance; }

installUi();
globalThis.haraAmp = state;
boot();
