import { AMP_FREQUENCIES, AMP_PRESETS, HaraAmpRuntime } from "./amp-runtime.js";

const $ = (selector, root = document) => root.querySelector(selector);
const $$ = (selector, root = document) => [...root.querySelectorAll(selector)];

const ui = {
  tracks: [{ title: "Hara Signal", detail: "SYNTH.WASM", kind: "synth", duration: 4 }],
  activeTrack: 0
};

const amp = new HaraAmpRuntime({
  canvas: $("[data-visualizer]"),
  dbName: "hara-amp",
  onStatus({ stage, state, detail }) {
    if (stage === "runtime") {
      const status = $("[data-runtime-status]");
      status.textContent = state === "ready" ? "WASM · LIVE / NS+ G1" :
        state === "error" ? `WASM · ERROR / ${detail}` : "WASM · BOOTING";
      status.classList.toggle("is-live", state === "ready");
      status.classList.toggle("is-error", state === "error");
    }
    if (stage === "hal") {
      $("[data-frame-status]").textContent =
        state === "ready" ? "HAL · ARMED" : state === "error" ? "HAL · ERROR" : "HAL · WAITING";
    }
  },
  onFrame({ count }) {
    $("[data-frame-status]").textContent = `HAL · FRAME ${count}`;
    document.documentElement.dataset.renderedFrames = String(count);
  },
  onPlayback({ state }) {
    $("[data-audio-status]").textContent =
      state === "playing" ? "PLAYING / WASM" : state.toUpperCase();
    $("[data-visual-empty]").classList.toggle("is-hidden", state === "playing");
  },
  onTime: updateTime,
  onTelemetry(telemetry) {
    for (const [key, value] of Object.entries(telemetry)) {
      document.documentElement.dataset[key] = String(value);
    }
  }
});

function installUi() {
  buildEq();
  $("[data-play]").addEventListener("click", () => amp.play().catch(showError));
  $("[data-pause]").addEventListener("click", () => amp.pause());
  $("[data-stop]").addEventListener("click", () => amp.stop());
  $("[data-previous]").addEventListener("click", () => selectRelative(-1));
  $("[data-next]").addEventListener("click", () => selectRelative(1));
  $("[data-repeat]").addEventListener("click", (event) => {
    const active = event.currentTarget.getAttribute("aria-pressed") !== "true";
    event.currentTarget.setAttribute("aria-pressed", String(active));
    if (amp.audio) amp.audio.loop = active;
  });
  $("[data-shuffle]").addEventListener("click", (event) => {
    const active = event.currentTarget.getAttribute("aria-pressed") !== "true";
    event.currentTarget.setAttribute("aria-pressed", String(active));
  });
  $("[data-volume]").addEventListener("input", (event) => {
    amp.setVolume(event.target.valueAsNumber / 100);
  });
  $("[data-balance]").addEventListener("input", (event) => {
    amp.setBalance(event.target.valueAsNumber / 100);
  });
  $("[data-seek]").addEventListener("change", (event) => {
    const duration = amp.audio?.duration ?? 4;
    amp.seek(event.target.valueAsNumber / 1000 * duration);
  });
  $("[data-local-file]").addEventListener("change", addLocalFiles);
  $("[data-clear-local]").addEventListener("click", clearLocal);
  $$("[data-visual-mode]").forEach((button) => button.addEventListener("click", () => {
    amp.setVisualMode(button.dataset.visualMode);
    $$("[data-visual-mode]").forEach((item) =>
      item.setAttribute("aria-pressed", String(item === button))
    );
    $(".visual-wrap").classList.toggle("is-artwork", amp.visualMode === "artwork");
  }));
  addEventListener("resize", () => amp.drawFrame());
}

function buildEq() {
  const root = $("[data-eq-bands]");
  const values = [0, ...AMP_PRESETS.flat];
  const labels = ["PRE", ...AMP_FREQUENCIES.map((frequency) =>
    frequency >= 1000 ? `${frequency / 1000}K` : String(frequency)
  )];
  values.forEach((value, index) => {
    const label = document.createElement("label");
    label.className = "eq-band";
    label.innerHTML = `<span>${labels[index]}</span><input type="range" min="-12" max="12" step=".5" value="${value}" data-eq-index="${index}"><output>0</output>`;
    root.append(label);
  });
  root.addEventListener("input", applyEq);
  $("[data-eq-enable]").addEventListener("click", (event) => {
    const enabled = event.currentTarget.getAttribute("aria-pressed") !== "true";
    event.currentTarget.setAttribute("aria-pressed", String(enabled));
    event.currentTarget.textContent = enabled ? "ENABLED" : "BYPASS";
    amp.setEqEnabled(enabled);
  });
  $("[data-eq-preset]").addEventListener("change", (event) => {
    $$("[data-eq-index]").slice(1).forEach((input, index) => {
      input.value = AMP_PRESETS[event.target.value][index];
    });
    amp.setPreset(event.target.value);
    applyEq();
  });
}

function applyEq() {
  const inputs = $$("[data-eq-index]");
  inputs.forEach((input) => {
    input.nextElementSibling.textContent = Number(input.value).toFixed(1);
  });
  amp.setPreamp(inputs[0].valueAsNumber);
  if (!amp.audio?.context) return;
  amp.audio.setEq(inputs.slice(1).map((input) => input.valueAsNumber));
}

async function addLocalFiles(event) {
  for (const file of event.target.files) {
    ui.tracks.push({
      title: file.name.replace(/\.[^.]+$/, ""),
      detail: file.type || "LOCAL AUDIO",
      kind: "file",
      file
    });
  }
  persistPlaylistMetadata();
  renderPlaylist();
}

function clearLocal() {
  ui.tracks = ui.tracks.filter((track) => track.kind === "synth");
  ui.activeTrack = 0;
  persistPlaylistMetadata();
  renderPlaylist();
}

function persistPlaylistMetadata() {
  localStorage.setItem("hara-amp.playlist.v1", JSON.stringify(
    ui.tracks.filter((track) => track.kind === "file")
      .map(({ title, detail }) => ({ title, detail }))
  ));
}

async function selectTrack(index) {
  const track = ui.tracks[index];
  if (!track) return;
  amp.stop();
  ui.activeTrack = index;
  if (track.kind === "file") await amp.loadFile(track.file);
  else {
    await amp.boot();
    await amp.audio.initialize();
    amp.audio.buffer = amp.audio.renderSynth();
    amp.audio.duration = amp.audio.buffer.duration;
  }
  $("[data-track-title]").textContent = track.title;
  $("[data-track-detail]").textContent = track.detail;
  renderPlaylist();
  await amp.play();
}

function selectRelative(direction) {
  if (!ui.tracks.length) return;
  const shuffle = $("[data-shuffle]").getAttribute("aria-pressed") === "true";
  const index = shuffle
    ? Math.floor(Math.random() * ui.tracks.length)
    : (ui.activeTrack + direction + ui.tracks.length) % ui.tracks.length;
  selectTrack(index).catch(showError);
}

function renderPlaylist() {
  const root = $("[data-playlist]");
  root.replaceChildren();
  ui.tracks.forEach((track, index) => {
    const item = document.createElement("li");
    item.classList.toggle("is-active", index === ui.activeTrack);
    item.innerHTML = `<b>${String(index + 1).padStart(2, "0")}</b><span>${escapeHtml(track.title)}</span><small>${escapeHtml(track.detail)}</small>`;
    item.addEventListener("click", () => selectTrack(index).catch(showError));
    root.append(item);
  });
}

function updateTime({ position = 0, duration = 4 } = {}) {
  $("[data-elapsed]").textContent = time(position);
  $("[data-remaining]").textContent = `-${time(Math.max(0, duration - position))}`;
  $("[data-seek]").value = duration ? Math.round(position / duration * 1000) : 0;
}

function showError(error) {
  const message = String(error?.message ?? error);
  console.error("[hara amp]", error);
  $("[data-runtime-status]").textContent = `WASM · ERROR / ${message}`;
  $("[data-runtime-status]").classList.add("is-error");
}

const time = (seconds) =>
  `${String(Math.floor(seconds / 60)).padStart(2, "0")}:${String(Math.floor(seconds % 60)).padStart(2, "0")}`;
const escapeHtml = (value) => String(value).replace(/[&<>"']/g, (character) =>
  ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[character]
);

installUi();
globalThis.haraAmp = amp;
amp.boot().catch(showError);
