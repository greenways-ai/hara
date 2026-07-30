import { HtaKeyword } from "./runtime/hta.js";
import { createBrowserBroker } from "./runtime/studio/broker.js";
import { createHostServices } from "./runtime/studio/host-services.js";
import { NodeRuntime } from "./runtime/studio/node-runtime.js";
import { SupersonicProvider } from "./runtime/studio/supersonic.js";

export const AMP_FREQUENCIES = Object.freeze([31, 62, 125, 250, 500, 1000, 2000, 4000, 8000, 16000]);
export const AMP_PRESETS = Object.freeze({
  flat: Object.freeze([0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
  hara: Object.freeze([3, 2, 0, -1, 1, 3, 5, 4, 2, 3]),
  bass: Object.freeze([8, 7, 5, 2, 0, -1, -2, -2, -1, 0]),
  voice: Object.freeze([-3, -2, 0, 3, 5, 5, 3, 1, 0, -2])
});

const ROOT = "ROOT";
const DOCUMENT_ID = "document/amp";
const NOOP = () => {};

export class HaraAmpRuntime {
  constructor({
    canvas = null,
    dbName = "hara-amp",
    onStatus = NOOP,
    onFrame = NOOP,
    onPlayback = NOOP,
    onTime = NOOP,
    onTelemetry = NOOP,
    onGraph = NOOP
  } = {}) {
    this.canvas = canvas;
    this.dbName = dbName;
    this.onStatus = onStatus;
    this.onFrame = onFrame;
    this.onPlayback = onPlayback;
    this.onTime = onTime;
    this.onTelemetry = onTelemetry;
    this.onGraph = onGraph;
    this.runtime = new NodeRuntime({ space: `workspace/${dbName}` });
    this.broker = null;
    this.audio = null;
    this.supersonic = null;
    this.graphSnapshot = null;
    this.synth = null;
    this.fft = null;
    this.visualMode = "spectrum";
    this.preset = "flat";
    this.preamp = 0;
    this.lastFrame = null;
    this.frameCount = 0;
    this.emittedFrames = 0;
    this.peaks = new Float32Array(96);
    this.frameWaiters = [];
    this.lastHalError = null;
    this.source = "";
    this.originalSource = "";
    this.generation = 0;
    this.bootPromise = null;
    this.ready = false;
    this.disposed = false;

    for (const node of [
      { id: "node/synth", type: "wasm/audio-source" },
      { id: "node/fft", type: "wasm/transform" },
      { id: "node/visualizer", type: "hal/transform" }
    ]) this.runtime.registerNode(node);
    this.runtime.connect({
      id: "connection/fft-visualizer",
      from: ["node/fft", "fft/bins"],
      to: ["node/visualizer", "fft/bins"],
      transport: "hta",
      delivery: "latest",
      capacity: 1
    });
  }

  status(stage, state, detail = "") {
    this.onStatus({ stage, state, detail });
  }

  async boot() {
    if (this.ready) return this;
    if (this.bootPromise) return this.bootPromise;
    this.disposed = false;
    this.bootPromise = this.bootRuntime();
    try {
      await this.bootPromise;
      this.ready = true;
      return this;
    } catch (error) {
      this.bootPromise = null;
      this.status("runtime", "error", friendlyError(error));
      throw error;
    }
  }

  async bootRuntime() {
    this.status("runtime", "loading", "Loading the browser kernel");
    this.status("synth", "loading", "Loading synth WASM");
    this.status("fft", "loading", "Loading FFT WASM");
    const [runtimeBytes, synth, fft, nodeSource, drawSource, sonicSource, protocolSource, frameSource, substrateSource, ampSource] =
      await Promise.all([
        bytes("./runtime/hara.wasm"),
        wasm("./assets/wasm/demo-synth.wasm"),
        wasm("./assets/wasm/demo-fft.wasm"),
        text("./runtime/studio/hal/node.hal"),
        text("./runtime/studio/hal/draw.hal"),
        text("./runtime/studio/hal/supersonic.hal"),
        text("./runtime/std/lib/substrate/protocol.hal"),
        text("./runtime/std/lib/substrate/frame.hal"),
        text("./runtime/std/lib/substrate.hal"),
        text("./examples/hara-amp/src/amp.hal")
      ]);
    if (this.disposed) throw new Error("Amp runtime was closed");

    this.synth = synth;
    this.fft = fft;
    this.source = ampSource;
    this.originalSource = ampSource;
    this.audio = new AmpAudio(this, synth, fft);
    this.supersonic = new SupersonicProvider({
      storage: globalThis.localStorage,
      engine: {
        prepare: async (graph) => this.audio.prepareGraph(graph),
        update: async (_graph, node, control, value) =>
          this.audio.updateGraphParameter(node.id, control.parameter, value, node),
        stop: async () => this.audio.stop()
      },
      onSnapshot: (snapshot) => {
        this.graphSnapshot = snapshot;
        this.onGraph(snapshot);
      }
    });
    this.status("synth", "ready", "Rust/WASM oscillator ready");
    this.status("audio", "gesture", "Play authorizes Web Audio");
    this.status("fft", "ready", "Rust/WASM analysis ready");
    this.status("hta", "loading", "Opening latest-value transport");

    const hostCalls = createHostServices({
      dbName: this.dbName,
      nodeRuntime: this.runtime,
      supersonic: this.supersonic,
      renderCanvas: (canvasId, scene) => this.renderCanvas(canvasId, scene)
    });
    this.broker = createBrowserBroker({
      workerUrl: new URL("./runtime/hta-worker.js", import.meta.url),
      moduleBytes: runtimeBytes,
      hostCalls,
      resources: {
        "studio.node": nodeSource,
        "studio.draw": drawSource,
        "gw.audio.supersonic": sonicSource,
        "std.lib.substrate.protocol": protocolSource,
        "std.lib.substrate.frame": frameSource,
        "std.lib.substrate": substrateSource
      }
    });
    await this.activateVisualizer(ampSource);

    this.status("hta", "ready", "HTA latest-value connection open");
    this.status("hal", "ready", `HAL visualizer generation ${this.generation} armed`);
    await this.probe();
    this.status("canvas", "ready", "Canvas received a real probe frame");
    this.status("runtime", "ready", "WASM · HTA · HAL live");
  }

  async activateVisualizer(source) {
    this.lastHalError = null;
    const prepared = await this.broker.prepareDocument(ROOT, DOCUMENT_ID, source, {
      nodeId: "node/visualizer"
    });
    try {
      await this.runtime.activateDocument("node/visualizer", {
        documentId: DOCUMENT_ID,
        generation: prepared.generation,
        moduleId: prepared.moduleId,
        kernelContext: prepared.context,
        prepare: (node) => {
          node.start(() => this.broker.evalPreparedDocument(prepared, "(run-visualizer)")
            .catch((error) => {
              this.lastHalError = error;
              const active = this.runtime.nodes.get("node/visualizer")?.active;
              if (!this.disposed && active?.generation === prepared.generation) {
                this.status("hal", "error", friendlyError(error));
              }
              throw error;
            }));
        }
      });
      this.broker.commitDocument(prepared);
      this.generation = prepared.generation;
      return prepared.generation;
    } catch (error) {
      this.broker.discardDocument(prepared);
      throw error;
    }
  }

  async rebuild(source) {
    if (typeof source !== "string" || !source.trim()) throw new Error("HAL source is required");
    await this.boot();
    this.status("hal", "loading", "Preparing a new HAL generation");
    const previousFrame = this.frameCount;
    try {
      const generation = await this.activateVisualizer(source);
      this.source = source;
      if (this.audio?.playing) await this.waitForFrame(previousFrame);
      else await this.probe();
      this.status("hal", "ready", `HAL visualizer generation ${generation} live`);
      return { generation, frame: this.frameCount };
    } catch (error) {
      this.status("hal", "error", friendlyError(error));
      throw error;
    }
  }

  async probe() {
    if (!this.synth || !this.fft) return;
    const previousFrame = this.frameCount;
    const samples = this.synthSamples(1024, 48000, 0);
    await this.emitSamples(samples);
    await this.waitForFrame(previousFrame);
  }

  synthSamples(frameCount, sampleRate, offset = 0) {
    const samples = new Float32Array(frameCount);
    const { memory, synth_buffer, synth_capacity, synth_fill } = this.synth.exports;
    const capacity = Number(synth_capacity());
    for (let start = 0; start < frameCount; start += capacity) {
      const count = Number(synth_fill(
        BigInt(offset + start),
        Math.min(capacity, frameCount - start),
        sampleRate
      ));
      samples.set(new Float32Array(memory.buffer, Number(synth_buffer()), count), start);
    }
    return samples;
  }

  async emitSamples(samples) {
    const inputFrames = Math.min(1024, samples.length);
    const { memory, fft_input, fft_output, fft_compute } = this.fft.exports;
    new Float32Array(memory.buffer, Number(fft_input()), inputFrames)
      .set(samples.subarray(0, inputFrames));
    const count = Number(fft_compute(inputFrames, 96));
    const magnitudes = new Float32Array(memory.buffer, Number(fft_output()), count);
    const bins = [...magnitudes].map((value) => Math.min(255, Math.round(value * 510)));
    const wave = [...samples.subarray(0, inputFrames).filter((_, index) => index % 8 === 0)]
      .map((value) => Math.max(-127, Math.min(127, Math.round(value * 127))));
    this.emittedFrames += 1;
    await this.runtime.emit(
      "node/fft",
      "fft/bins",
      new Map([["bins", bins], ["wave", wave]])
    );
    this.reportTelemetry();
  }

  emitLiveSamples(samples) {
    this.emitSamples(samples).catch((error) => {
      this.status("hta", "error", friendlyError(error));
    });
  }

  renderCanvas(canvasId, scene) {
    if (canvasId !== "canvas/visualizer") return;
    const bins = mapValue(scene, "bins") ?? [];
    const wave = mapValue(scene, "wave") ?? [];
    const palette = mapValue(scene, "palette") ?? ["#2fffe0", "#149df2", "#9b35ff"];
    this.lastFrame = { bins, wave, palette };
    this.frameCount += 1;
    for (const waiter of this.frameWaiters.splice(0)) waiter(this.frameCount);
    this.drawFrame();
    this.onFrame({ count: this.frameCount, bins, wave, palette });
    this.reportTelemetry();
  }

  waitForFrame(previousFrame, timeout = 2500) {
    if (this.frameCount > previousFrame) return Promise.resolve(this.frameCount);
    return new Promise((resolve, reject) => {
      const done = (count) => {
        clearTimeout(timer);
        resolve(count);
      };
      const timer = setTimeout(() => {
        const index = this.frameWaiters.indexOf(done);
        if (index >= 0) this.frameWaiters.splice(index, 1);
        reject(this.lastHalError ??
          new Error("HAL visualizer did not produce a probe frame; the active task produced no canvas scene"));
      }, timeout);
      this.frameWaiters.push(done);
    });
  }

  reportTelemetry() {
    const queue = this.runtime.nodes.get("node/visualizer")?.inputs.get("fft/bins");
    this.onTelemetry({
      emittedFrames: this.emittedFrames,
      renderedFrames: this.frameCount,
      nodeWaiters: queue?.waiters.length ?? 0,
      nodeQueued: queue?.values.length ?? 0
    });
  }

  setCanvas(canvas) {
    this.canvas = canvas;
    this.drawFrame();
  }

  setVisualMode(mode) {
    if (!["spectrum", "scope", "artwork"].includes(mode)) return false;
    this.visualMode = mode;
    void this.update("visualizer", "mode", mode);
    this.drawFrame();
    return true;
  }

  setPreset(name) {
    if (!AMP_PRESETS[name]) return false;
    this.preset = name;
    this.audio?.setEq(AMP_PRESETS[name]);
    void this.update("eq", "character", name);
    return true;
  }

  setPreamp(decibels) {
    this.preamp = Number(decibels) || 0;
    this.audio?.setPreamp(this.preamp);
    void this.update("gain", "preamp", this.preamp);
  }

  setEqEnabled(enabled) {
    if (!this.audio) return;
    this.audio.eqEnabled = Boolean(enabled);
    this.audio.setEq(AMP_PRESETS[this.preset]);
  }

  setVolume(value) {
    this.audio?.setVolume(value);
    void this.update("pan", "volume", Number(value));
  }

  setBalance(value) {
    this.audio?.setBalance(value);
    void this.update("pan", "balance", Number(value));
  }

  async update(nodeId, parameter, value) {
    if (!this.supersonic?.graphs.has("hara-amp")) return null;
    return this.supersonic.update("hara-amp", nodeId, parameter, value);
  }

  async eval(form) {
    await this.boot();
    if (typeof form !== "string" || !form.trim()) throw new Error("A HAL form is required");
    return this.broker.evalForm(ROOT, DOCUMENT_ID, form);
  }

  async play(offset) {
    await this.boot();
    await this.audio.play(offset);
    this.audio.setEq(AMP_PRESETS[this.preset]);
    this.audio.setPreamp(this.preamp);
    this.status("audio", "ready", "Web Audio playing");
  }

  pause() {
    this.audio?.pause();
  }

  stop() {
    this.audio?.stop();
  }

  seek(offset) {
    if (!this.audio) return;
    if (this.audio.playing) void this.audio.play(offset);
    else this.audio.offset = offset;
  }

  async loadFile(file) {
    await this.boot();
    await this.audio.loadFile(file);
  }

  drawFrame() {
    if (!this.canvas || !this.lastFrame) return;
    const rect = this.canvas.getBoundingClientRect();
    const ratio = Math.min(2, globalThis.devicePixelRatio || 1);
    const width = Math.max(1, Math.round(rect.width));
    const height = Math.max(1, Math.round(rect.height));
    if (this.canvas.width !== width * ratio || this.canvas.height !== height * ratio) {
      this.canvas.width = width * ratio;
      this.canvas.height = height * ratio;
    }
    const context = this.canvas.getContext("2d");
    context.setTransform(ratio, 0, 0, ratio, 0, 0);
    context.clearRect(0, 0, width, height);
    const { bins, wave, palette } = this.lastFrame;
    if (this.visualMode === "scope") drawScope(context, width, height, wave, palette);
    else drawSpectrum(context, width, height, bins, palette, this.peaks, this.visualMode === "artwork");
  }

  async dispose() {
    this.disposed = true;
    this.ready = false;
    this.audio?.dispose();
    this.audio = null;
    this.frameWaiters.splice(0);
    if (this.broker) {
      this.broker.releaseDocument(ROOT, DOCUMENT_ID);
      const kernel = this.broker.kernels?.get(ROOT);
      kernel?.context?.close?.();
      kernel?.worker?.terminate?.();
      this.broker.kernels?.delete(ROOT);
    }
    this.broker = null;
    this.bootPromise = null;
  }
}

class AmpAudio {
  constructor(owner, synth, fft) {
    this.owner = owner;
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

  async prepareGraph(graph) {
    const previous = this.graph;
    return {
      commit: async () => {
        this.graph = graph;
        for (const node of graph.nodes) {
          for (const [parameter, value] of Object.entries(node.params)) {
            if (!TUNE_PARAMETERS.has(parameter)) {
              await this.updateGraphParameter(node.id, parameter, value, node);
            }
          }
          if (node.id === "synth") await this.updateTune(node.params);
        }
      },
      discard: async () => { this.graph = previous; }
    };
  }

  async updateGraphParameter(nodeId, parameter, value, node = null) {
    if (nodeId === "synth" && TUNE_PARAMETERS.has(parameter)) {
      return this.updateTune({ ...(node?.params ?? {}), [parameter]: value }, parameter);
    }
    if (nodeId === "synth" && parameter === "waveform") {
      const waveform = { sine: 0, square: 1, saw: 2 }[value];
      if (waveform == null || this.synth.exports.synth_set_waveform(waveform) !== 1) {
        throw new Error(`supersonic/waveform-unsupported:${value}`);
      }
      if (this.context) {
        this.buffer = this.renderSynth();
        if (this.playing) await this.play(this.position());
      }
    } else if (nodeId === "eq" && parameter === "character") {
      this.owner.preset = value;
      this.setEq(AMP_PRESETS[value] ?? AMP_PRESETS.flat);
    } else if (nodeId === "gain" && parameter === "preamp") {
      this.owner.preamp = Number(value);
      this.setPreamp(Number(value));
    } else if (nodeId === "pan" && parameter === "volume") {
      this.setVolume(value);
    } else if (nodeId === "pan" && parameter === "balance") {
      this.setBalance(value);
    } else if (nodeId === "visualizer" && parameter === "mode") {
      this.owner.visualMode = value;
      this.owner.drawFrame();
    }
  }

  async initialize() {
    if (this.context) return;
    this.context = new AudioContext({ latencyHint: "interactive" });
    this.preamp = new GainNode(this.context, { gain: 1 });
    this.filters = AMP_FREQUENCIES.map((frequency) =>
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
    for (const node of this.graph?.nodes ?? []) {
      for (const [parameter, value] of Object.entries(node.params)) {
        if (!TUNE_PARAMETERS.has(parameter)) {
          await this.updateGraphParameter(node.id, parameter, value, node);
        }
      }
      if (node.id === "synth") await this.updateTune(node.params);
    }
    this.buffer = this.renderSynth();
    this.duration = this.buffer.duration;
    this.visualLoop();
  }

  renderSynth() {
    const sampleRate = this.context.sampleRate;
    const frames = Math.round(sampleRate * this.tuneDuration());
    const buffer = new AudioBuffer({ length: frames, sampleRate, numberOfChannels: 2 });
    const mono = this.owner.synthSamples(frames, sampleRate);
    buffer.copyToChannel(mono, 0);
    buffer.copyToChannel(mono, 1);
    return buffer;
  }

  tuneDuration(tune = this.tune) {
    if (!tune) return 4;
    return tune.steps.length * 60 / tune.tempo / tune["steps-per-beat"];
  }

  configureTune(tune) {
    const exports = this.synth.exports;
    exports.synth_tune_begin();
    const checks = [
      exports.synth_tune_set_tempo(Number(tune.tempo)),
      exports.synth_tune_set_root(Number(tune.root)),
      exports.synth_tune_set_gate(Number(tune.gate)),
      exports.synth_tune_set_steps_per_beat(Number(tune["steps-per-beat"])),
      exports.synth_tune_set_length(tune.steps.length)
    ];
    tune.steps.forEach((step, index) => {
      checks.push(exports.synth_tune_set_step(index, step ?? 0, step == null));
    });
    if (checks.some((accepted) => accepted !== 1)) {
      throw new Error("supersonic/tune-wasm-rejected");
    }
    exports.synth_tune_commit();
  }

  async updateTune(tune, parameter = "tune") {
    this.configureTune(tune);
    const nextTune = structuredClone(tune);
    if (!this.context || !this.playing) {
      this.tune = nextTune;
      if (this.context) {
        this.buffer = this.renderSynth();
        this.duration = this.buffer.duration;
      }
      return { pending: false };
    }

    const boundary = this.pendingTune?.boundary ??
      this.context.currentTime + Math.max(0.001, this.duration - this.position());
    try { this.pendingTune?.source?.stop(); } catch {}
    this.tune = nextTune;
    const buffer = this.renderSynth();
    const source = new AudioBufferSourceNode(this.context, { buffer, loop: true });
    source.connect(this.preamp);
    source.start(boundary);
    if (!this.pendingTune) this.source?.stop(boundary);
    clearTimeout(this.pendingTune?.timer);
    const timer = setTimeout(() => {
      this.source = source;
      this.buffer = buffer;
      this.duration = buffer.duration;
      this.startedAt = boundary;
      this.offset = 0;
      this.pendingTune = null;
      this.owner.supersonic?.effective("hara-amp", "synth", null);
    }, Math.max(0, (boundary - this.context.currentTime) * 1000));
    this.pendingTune = { boundary, source, timer };
    return { pending: true, effectiveAt: boundary };
  }

  async play(offset = this.offset) {
    await this.initialize();
    await this.context.resume();
    this.stopSource();
    this.source = new AudioBufferSourceNode(this.context, { buffer: this.buffer, loop: this.loop });
    this.source.connect(this.preamp);
    this.offset = Math.max(0, Math.min(offset ?? 0, this.duration - .001));
    this.startedAt = this.context.currentTime - this.offset;
    this.source.start(0, this.offset);
    this.source.onended = () => {
      if (!this.loop && this.playing) this.stop();
    };
    this.playing = true;
    this.owner.onPlayback({ state: "playing" });
  }

  pause() {
    if (!this.playing) return;
    this.offset = this.position();
    this.cancelPendingTune({ install: true });
    this.stopSource();
    this.playing = false;
    this.owner.onPlayback({ state: "paused" });
  }

  stop() {
    this.cancelPendingTune({ install: true });
    this.stopSource();
    this.offset = 0;
    this.playing = false;
    this.owner.onPlayback({ state: "stopped" });
    this.owner.onTime({ position: 0, duration: this.duration });
  }

  stopSource() {
    if (!this.source) return;
    try { this.source.stop(); } catch {}
    this.source = null;
  }

  cancelPendingTune({ install = false } = {}) {
    if (!this.pendingTune) return;
    clearTimeout(this.pendingTune.timer);
    try { this.pendingTune.source.stop(); } catch {}
    this.pendingTune = null;
    if (install && this.context) {
      this.buffer = this.renderSynth();
      this.duration = this.buffer.duration;
      this.owner.supersonic?.effective("hara-amp", "synth", null);
    }
  }

  position() {
    if (!this.context || !this.playing) return this.offset;
    const elapsed = this.context.currentTime - this.startedAt;
    return this.loop ? elapsed % this.duration : Math.min(elapsed, this.duration);
  }

  setEq(values) {
    if (!this.context) return;
    this.filters.forEach((filter, index) => {
      filter.gain.setTargetAtTime(this.eqEnabled ? values[index] : 0, this.context.currentTime, .02);
    });
  }

  setPreamp(decibels) {
    if (!this.context) return;
    this.preamp.gain.setTargetAtTime(10 ** (decibels / 20), this.context.currentTime, .02);
  }

  setVolume(value) {
    if (this.master) this.master.gain.value = Math.max(0, Math.min(1, Number(value)));
  }

  setBalance(value) {
    if (this.balance) this.balance.pan.value = Math.max(-1, Math.min(1, Number(value)));
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
      const position = this.position();
      this.owner.onTime({ position, duration: this.duration });
      if (!this.playing) return;
      this.analyser.getFloatTimeDomainData(this.timeData);
      this.owner.emitLiveSamples(this.timeData.subarray(0, 1024));
    };
    tick();
  }

  dispose() {
    this.cancelPendingTune();
    this.stopSource();
    cancelAnimationFrame(this.raf);
    this.context?.close?.();
    this.context = null;
    this.playing = false;
  }
}

const TUNE_PARAMETERS = new Set(["tempo", "root", "gate", "steps-per-beat", "steps"]);

function drawSpectrum(context, width, height, bins, palette, peaks, overlay) {
  if (!bins.length) return;
  const gradient = context.createLinearGradient(0, height, width, 0);
  palette.forEach((color, index) => gradient.addColorStop(index / Math.max(1, palette.length - 1), color));
  const gap = 2;
  const barWidth = Math.max(1, width / bins.length - gap);
  bins.forEach((value, index) => {
    const normalized = value / 255;
    const barHeight = Math.max(2, normalized * height * .84);
    const x = index * width / bins.length;
    peaks[index] = Math.max(normalized, peaks[index] - .018);
    context.globalAlpha = overlay ? .58 : .92;
    context.fillStyle = gradient;
    context.fillRect(x, height - barHeight, barWidth, barHeight);
    context.globalAlpha = 1;
    context.fillRect(x, height - peaks[index] * height * .84 - 2, barWidth, 2);
  });
}

function drawScope(context, width, height, wave, palette) {
  if (!wave.length) return;
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

function mapValue(map, name) {
  if (!(map instanceof Map)) return map?.[name];
  for (const [key, value] of map) {
    if (key === name || key instanceof HtaKeyword && key.name === name) return value;
  }
}

function friendlyError(error) {
  const message = String(error?.message ?? error ?? "Unknown error");
  return message.replace(/^Error:\s*/, "").slice(0, 180);
}

async function text(url) {
  const response = await fetch(new URL(url, import.meta.url));
  if (!response.ok) throw new Error(`${url}: ${response.status}`);
  return response.text();
}

async function bytes(url) {
  const response = await fetch(new URL(url, import.meta.url));
  if (!response.ok) throw new Error(`${url}: ${response.status}`);
  return new Uint8Array(await response.arrayBuffer());
}

async function wasm(url) {
  return (await WebAssembly.instantiate(await bytes(url), {})).instance;
}
