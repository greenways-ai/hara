import { keywordName, mapValue, renderScene, validateScene } from "./scene.js";
import { CreativeRuntime, normalizeCreative } from "./creative.js";
import { applyParedit, barfForward, insertIndent, killToFormEnd, localFormAt, slurpForward, structuralAlign } from "./editor.js";
import { WorkspaceRepository, kernelName, workspaceTemplates } from "./workspaces.js";
import { downloadWorkspace, GitHubDeviceAuth, githubRequest, GistPublisher, workspaceBundle } from "./publishing.js";

const SPACE = "home";
const ROOT = "ROOT";
const ACTIVE_FILE_KEY = "hara-www.active-file.v1";
const WINDOWS_KEY = "hara-www.windows.v1";
const BACKGROUND_SOURCE_KEY = "hara-www.background-source.v2";
const BACKGROUND_WORKSPACE = "./examples/studio-backgrounds/";
const SIGNAL_RING_BACKGROUND = "document/background/signal-ring";
const HAL_FORMS = [
  ["def", "bind a named value"], ["defn", "define a function"], ["fn", "anonymous function"],
  ["let", "local bindings"], ["if", "conditional branch"], ["when", "conditional body"],
  ["do", "evaluate forms in sequence"], ["cond", "multi-branch conditional"],
  ["map", "transform a collection"], ["filter", "select collection values"], ["reduce", "fold a collection"],
  ["get", "read a value from a collection"], ["assoc", "associate map entries"],
  ["vec", "make a vector"], ["concat", "join collections"], ["println", "write a value"],
  [":version", "scene format version"], [":commands", "scene drawing commands"],
  [":background", "scene background colour"], [":width", "scene width"], [":height", "scene height"]
];

class HalAudioPipeline {
  constructor() {
    this.spec = null;
    this.context = null;
    this.master = null;
    this.timer = null;
    this.playing = false;
    this.step = 0;
  }

  configure(spec) {
    this.spec = { tempo: 86, volume: .16, root: 43.65, steps: [0], pulse: .42, wave: "sine", ...spec };
    if (this.master) this.master.gain.setTargetAtTime(this.spec.volume, this.context.currentTime, .04);
    return this.status();
  }

  async control(command, value) {
    if (command === "volume") {
      this.spec = { ...this.spec, volume: Math.max(0, Math.min(.32, Number(value) || 0)) };
      if (this.master) this.master.gain.setTargetAtTime(this.spec.volume, this.context.currentTime, .025);
    } else if (command === "toggle") {
      if (this.playing) this.stop(); else await this.play();
    } else if (command === "play") await this.play();
    else if (command === "stop") this.stop();
    return this.status();
  }

  async play() {
    if (!this.spec) throw new Error("audio pipeline is not configured by the active HAL background");
    const Context = globalThis.AudioContext ?? globalThis.webkitAudioContext;
    if (!Context) throw new Error("Web Audio is unavailable in this browser");
    this.context ??= new Context({ latencyHint: "interactive" });
    this.master ??= new GainNode(this.context, { gain: this.spec.volume });
    this.master.connect(this.context.destination);
    await this.context.resume();
    this.playing = true;
    this.tick();
    const milliseconds = 60000 / this.spec.tempo * this.spec.pulse;
    this.timer = setInterval(() => this.tick(), milliseconds);
  }

  stop() {
    clearInterval(this.timer);
    this.timer = null;
    this.playing = false;
  }

  tick() {
    if (!this.playing || !this.context || !this.master) return;
    const semitone = this.spec.steps[this.step++ % this.spec.steps.length];
    const frequency = this.spec.root * Math.pow(2, semitone / 12);
    const now = this.context.currentTime;
    const oscillator = new OscillatorNode(this.context, { type: this.spec.wave, frequency });
    const gain = new GainNode(this.context, { gain: 0.0001 });
    oscillator.connect(gain).connect(this.master);
    gain.gain.exponentialRampToValueAtTime(.34, now + .012);
    gain.gain.exponentialRampToValueAtTime(.0001, now + Math.max(.11, this.spec.pulse * .46));
    oscillator.start(now);
    oscillator.stop(now + Math.max(.14, this.spec.pulse * .52));
  }

  status() { return { playing: this.playing, volume: this.spec?.volume ?? 0 }; }
}

const audioPipeline = new HalAudioPipeline();

const DEFAULT_FILES = new Map([
  ["/sketches/neon-orbit.hal", `;; Put the cursor in this map and press Ctrl-E.
;; Scene commands are a finite vector so the browser runtime can transport it.
{:version 1
 :width 960
 :height 600
 :background "#020408"
 :commands
 [[:polyline [[170 300] [285 165] [480 105] [675 165] [790 300]
              [675 435] [480 495] [285 435] [170 300]] "#225f70" 3]
  [:circle 480 300 76 "#102d3d"]
  [:circle 480 300 16 "#bafff8"]
  [:circle 170 300 20 "#41f5e4"]
  [:circle 285 165 28 "#9c7bff"]
  [:circle 480 105 18 "#ff2e88"]
  [:circle 675 165 28 "#41f5e4"]
  [:circle 790 300 20 "#f5d742"]
  [:circle 675 435 28 "#9c7bff"]
  [:circle 480 495 18 "#41f5e4"]
  [:circle 285 435 28 "#ff2e88"]]}
`],
  ["/sketches/signal-field.hal", `;; A declarative canvas scene is ordinary Hara data.
{:version 1
 :width 960
 :height 600
 :background "#03050a"
 :commands
 [[:rect 80 84 800 2 "#17444d"]
  [:rect 80 514 800 2 "#17444d"]
  [:polyline [[80 420] [180 315] [280 370] [390 180]
              [500 340] [610 130] [720 280] [880 120]]
             "#41f5e4" 5]
  [:polyline [[80 465] [210 410] [330 455] [455 330]
              [570 420] [700 285] [880 365]]
             "#9c7bff" 3]
  [:circle 390 180 11 "#ff2e88"]
  [:circle 610 130 11 "#f5d742"]
  [:circle 880 120 11 "#bafff8"]]}
`],
  ["/sketches/rigged-cube.hal", `;; Creative scenes share the same local form evaluation workflow.
{:creative/version 1
 :background "#020408"
 :entities [{:id "mesh/hero"
             :mesh {:primitive :box}
             :material {:color "#41f5e4"}
             :transform {:rotation [0 0 0]}
             :rig {:bones [{:id "bone/root" :length 1}
                           {:id "bone/arm" :parent "bone/root" :length 1}]}}]
 :audio {:tempo 120 :midi true :voices []}}
`],
  ["/templates/3d-editor.hal", `;; 3D editor template — change the mesh, material, or rig, then run.
{:creative/version 1
 :background "#020408"
 :entities [{:id "mesh/hero"
             :mesh {:primitive :box}
             :material {:color "#41f5e4"}
             :transform {:rotation [0 0 0]}
             :rig {:bones [{:id "bone/root" :length 1}
                           {:id "bone/arm" :parent "bone/root" :length 1}]}}]
 :audio {:tempo 120 :midi true :voices []}}
`],
  ["/templates/graphing.hal", `;; Graphing template — edit the points or add another series, then run.
{:version 1
 :width 960
 :height 600
 :background "#020408"
 :commands
 [[:rect 72 72 816 456 "#07131d"]
  [:line 72 300 888 300 "#1a6070" 2]
  [:line 480 72 480 528 "#1a6070" 2]
  [:polyline [[92 450] [172 410] [252 355] [332 290] [412 245] [492 270] [572 205] [652 150] [732 175] [812 112]] "#41f5e4" 5]
  [:circle 492 270 10 "#ff2e88"]
  [:circle 812 112 10 "#9c7bff"]]}
`],
  ["/README.hal", `;; HARA VISUAL LAB
;;
;; Open a sketch from /sketches and press Run.
;; A runnable file returns a scene map with:
;;   :version, :width, :height, :background, :commands
;;
;; Commands:
;;   [:line x1 y1 x2 y2 color width]
;;   [:circle x y radius color]
;;   [:rect x y width height color]
;;   [:polyline [[x y] ...] color width]
;;
;; Files and window positions stay on this device.
nil
`]
]);

const query = (selector, root = document) => root.querySelector(selector);
const queryAll = (selector, root = document) => [...root.querySelectorAll(selector)];

const state = {
  broker: null,
  nodeRuntime: null,
  activeDocument: null,
  files: [],
  activeFile: null,
  dirty: false,
  savedSource: "",
  lastScene: null,
  creativeRuntime: null,
  editorPrefix: null,
  editorPrefixTimer: null,
  evalRange: null,
  editorHistory: { past: [], future: [], current: null, replaying: false },
  backgroundSource: localStorage.getItem(BACKGROUND_SOURCE_KEY) ?? SIGNAL_RING_BACKGROUND,
  backgroundDocuments: new Map(),
  activeBackground: null,
  canvasRuntime: null,
  sourceTimer: null,
  zIndex: 10,
  workspace: 0,
  workspaceRepository: new WorkspaceRepository(),
  workspaceRecords: new Map(),
  openWorkspaces: [],
  currentProject: null,
  activeKernel: ROOT,
  activeSpace: SPACE,
  kernelSpaces: new Map([[ROOT, SPACE]]),
  contextSpaces: new WeakMap(),
  defaultBootstrap: null
};

const elements = {
  launcher: query("[data-launcher]"),
  launcherToggle: query("[data-launcher-toggle]"),
  launcherScrim: query("[data-launcher-scrim]"),
  runtimeLed: query("[data-runtime-led]"),
  runtimeLabel: query("[data-runtime-label]"),
  backgroundSource: query("[data-background-source]"),
  backgroundAudio: query("[data-background-audio]"),
  audioToggle: query("[data-audio-toggle]"),
  audioVolume: query("[data-audio-volume]"),
  sourceToggle: query("[data-source-toggle]"),
  sourcePanel: query("[data-background-panel]"),
  sourceEditor: query("[data-background-editor]"),
  sourceHighlight: query("[data-background-highlight]"),
  sourceStatus: query("[data-background-status]"),
  sourceSave: query("[data-background-save]"),
  fileTree: query("[data-file-tree]"),
  fileCount: query("[data-file-count]"),
  editor: query("[data-editor]"),
  codeHighlight: query("[data-code-highlight]"),
  editorTitle: query("[data-editor-title]"),
  editorStatus: query("[data-editor-status]"),
  lineNumbers: query("[data-line-numbers]"),
  dirty: query("[data-dirty]"),
  save: query("[data-save]"),
  run: query("[data-run]"),
  paredit: query("[data-paredit]"),
  diff: query("[data-diff]"),
  inlineEval: query("[data-inline-eval]"),
  completions: query("[data-hal-completions]"),
  structuralDiff: query("[data-structural-diff]"),
  outputCanvas: query("[data-output-canvas]"),
  creativeCanvas: query("[data-creative-canvas]"),
  canvasEmpty: query("[data-canvas-empty]"),
  canvasStatus: query("[data-canvas-status]"),
  canvasSize: query("[data-canvas-size]"),
  canvasWrap: query("[data-canvas-wrap]"),
  dialog: query("[data-dialog]"),
  dialogForm: query("[data-dialog-form]"),
  dialogTitle: query("[data-dialog-title]"),
  dialogLabel: query("[data-dialog-label]"),
  dialogInput: query("[data-dialog-input]"),
  dialogMessage: query("[data-dialog-message]"),
  helpDialog: query("[data-help-dialog]"),
  templateDialog: query("[data-template-dialog]"),
  templateGrid: query("[data-template-grid]"),
  workspaceName: query("[data-workspace-name]"),
  projectTabs: query("[data-project-tabs]"),
  savedWorkspaces: query("[data-saved-workspaces]"),
  publish: query("[data-publish]"),
  publishDialog: query("[data-publish-dialog]"),
  publishName: query("[data-publish-name]"),
  publishFiles: query("[data-publish-files]"),
  publishGist: query("[data-publish-gist]"),
  publishAuth: query("[data-publish-auth]"),
  publishCode: query("[data-publish-code]"),
  publishNote: query("[data-publish-note]"),
  toasts: query("[data-toasts]")
};

const completionState = { entries: [], index: 0, start: 0 };
let backgroundLoadGeneration = 0;

function toast(message, error = false) {
  const node = document.createElement("div");
  node.className = `toast${error ? " is-error" : ""}`;
  node.textContent = message;
  elements.toasts.append(node);
  setTimeout(() => node.remove(), 4200);
}

function errorText(error) {
  return String(error?.message ?? error).replace(/^Error:\s*/, "");
}

function setRuntimeStatus(label, status) {
  elements.runtimeLabel.textContent = label;
  elements.runtimeLed.classList.toggle("is-live", status === "live");
  elements.runtimeLed.classList.toggle("is-error", status === "error");
}

async function loadBackgroundWorkspace() {
  const project = await fetch(new URL(`${BACKGROUND_WORKSPACE}project.edn`, import.meta.url));
  if (!project.ok) throw new Error(`project.edn fetch failed: ${project.status}`);
  await state.broker.eval(ROOT, `(quote ${await project.text()})`);
  const workspace = await fetch(new URL(`${BACKGROUND_WORKSPACE}workspace.edn`, import.meta.url));
  if (!workspace.ok) throw new Error(`workspace.edn fetch failed: ${workspace.status}`);
  const value = await state.broker.eval(ROOT, `(quote ${await workspace.text()})`);
  state.backgroundDocuments.clear();
  elements.backgroundSource.replaceChildren();
  for (const documentValue of mapValue(value, "workspace/documents") ?? []) {
    if (keywordName(mapValue(documentValue, "document/role")) !== "studio/background") continue;
    const descriptor = {
      id: String(mapValue(documentValue, "document/id")),
      title: String(mapValue(documentValue, "document/title")),
      path: String(mapValue(documentValue, "document/path")).replace("../../sources/", "./sources/"),
      node: String(mapValue(documentValue, "document/node")),
      canvas: String(mapValue(documentValue, "document/canvas"))
    };
    state.backgroundDocuments.set(descriptor.id, descriptor);
    const option = document.createElement("option");
    option.value = descriptor.id;
    option.textContent = `${descriptor.title.toUpperCase()}.HAL`;
    elements.backgroundSource.append(option);
  }
  if (!state.backgroundDocuments.has(state.backgroundSource)) {
    state.backgroundSource = state.backgroundDocuments.keys().next().value;
  }
}

function sourceStorageKey(documentId, kind) {
  return `hara-www.background.${kind}.v1:${documentId}`;
}

async function fetchBackgroundSource(descriptor) {
  const response = await fetch(new URL(descriptor.path, import.meta.url), { cache: "no-store" });
  if (!response.ok) throw new Error(`background source fetch failed: ${response.status}`);
  const bundled = await response.text();
  const saved = localStorage.getItem(sourceStorageKey(descriptor.id, "saved"));
  const recovery = localStorage.getItem(sourceStorageKey(descriptor.id, "recovery"));
  return {
    bundled,
    source: recovery ?? saved ?? bundled,
    recovered: recovery !== null
  };
}

async function activateBackground(descriptor, source, generation) {
  const nodeId = `${descriptor.node}@${generation}`;
  const prepared = await state.broker.prepareDocument(ROOT, descriptor.id, source, { nodeId });
  state.nodeRuntime.registerNode({ id: nodeId, type: "hal/background" });
  state.canvasRuntime.stage(nodeId, descriptor.canvas);
  const firstFrame = state.canvasRuntime.waitForFirstRender(nodeId, descriptor.canvas, 2500);
  try {
    const taskId = prepared.value;
    if (typeof taskId !== "string") throw new Error("background must return a node/start task handle");
    let taskSettled = null;
    await state.nodeRuntime.activateDocument(nodeId, {
      documentId: descriptor.id,
      generation: prepared.generation,
      moduleId: prepared.moduleId,
      kernelContext: prepared.context,
      prepare: (node) => {
        taskSettled = node.start(() => state.broker.evalPreparedDocument(
          prepared,
          `(node/run-task ${JSON.stringify(taskId)})`
        )).settled;
      }
    });
    await Promise.race([
      firstFrame,
      taskSettled.then(() => { throw new Error("background task stopped before its first frame"); })
    ]);
    const previous = state.activeBackground;
    state.canvasRuntime.commit(nodeId, descriptor.canvas);
    state.broker.commitDocument(prepared);
    state.activeBackground = { descriptor, nodeId, source };
    if (previous?.nodeId && previous.nodeId !== nodeId) state.nodeRuntime.releaseNode(previous.nodeId);
  } catch (error) {
    state.canvasRuntime.discard(nodeId, descriptor.canvas);
    state.nodeRuntime.releaseNode(nodeId);
    state.broker.discardDocument(prepared);
    throw error;
  }
}

async function loadBackgroundSource(name, sourceOverride = null) {
  const descriptor = state.backgroundDocuments.get(name);
  if (!descriptor) throw new Error(`unknown background document: ${name}`);
  state.backgroundSource = name;
  localStorage.setItem(BACKGROUND_SOURCE_KEY, name);
  elements.backgroundSource.value = name;
  if (!state.broker) return;
  const generation = ++backgroundLoadGeneration;
  try {
    const loaded = await fetchBackgroundSource(descriptor);
    const source = sourceOverride ?? loaded.source;
    await activateBackground(descriptor, source, generation);
    if (generation !== backgroundLoadGeneration) return;
    const canvas = query("[data-tron]");
    canvas.hidden = state.workspace === 1;
    canvas.dataset.backgroundName = descriptor.title.toLowerCase();
    elements.sourceEditor.value = source;
    elements.sourceEditor.dataset.baseSource = loaded.bundled;
    elements.sourceEditor.dataset.documentId = descriptor.id;
    elements.sourceStatus.textContent =
      `${loaded.recovered ? "RECOVERED" : "LIVE"} // GENERATION ${generation}`;
    const hasAudio = descriptor.id === SIGNAL_RING_BACKGROUND;
    elements.backgroundAudio.hidden = !hasAudio || state.workspace === 1;
    if (!hasAudio && audioPipeline.playing) audioPipeline.stop();
    elements.audioToggle.setAttribute("aria-pressed", String(audioPipeline.playing));
    elements.audioToggle.textContent = audioPipeline.playing ? "■ STOP" : "♪ PLAY";
    syncBackgroundHighlight();
  } catch (error) {
    if (generation !== backgroundLoadGeneration) return;
    elements.sourceStatus.textContent = `ERROR // ${errorText(error)}`;
    toast(`BACKGROUND SOURCE FAILED: ${errorText(error)}`, true);
    throw error;
  }
}

function setWorkspace(index) {
  state.workspace = index === 1 ? 1 : 0;
  document.body.dataset.workspace = String(state.workspace);
  queryAll("[data-home]").forEach((button) => {
    button.classList.toggle("is-active", state.workspace === 0);
    if (button.classList.contains("project-tab")) {
      button.toggleAttribute("aria-current", state.workspace === 0);
    }
  });
  if (state.workspace === 1) {
    state.canvasRuntime?.setVisible(false);
    query("[data-tron]").hidden = true;
    elements.backgroundAudio.hidden = true;
    showWorkspaceWindows();
  } else {
    state.canvasRuntime?.setVisible(true);
    query("[data-tron]").hidden = false;
    if (state.broker) loadBackgroundSource(state.backgroundSource).catch(() => {});
  }
  closeLauncher();
}

const workspacePresentation = {
  blank: { files: "EXPLORER", editor: "SOURCE", canvas: "OUTPUT", tabs: ["explorer", "source", "output"] },
  canvas: { files: "EXPLORER", editor: "SOURCE", canvas: "CANVAS", tabs: ["explorer", "source", "canvas"] },
  music: { files: "PLAYLIST", editor: "PLAYER / SOURCE", canvas: "SPECTRUM", tabs: ["playlist", "source", "spectrum"] },
  "3d": { files: "HIERARCHY", editor: "SOURCE", canvas: "3D VIEWPORT", tabs: ["hierarchy", "source", "viewport"] },
  graphs: { files: "SOURCE & DATA", editor: "SOURCE", canvas: "GRAPH", tabs: ["data", "source", "graph"] }
};

function applyWorkspacePresentation(record) {
  const template = record?.template ?? "blank";
  const presentation = workspacePresentation[template] ?? workspacePresentation.blank;
  const desktop = query(".desktop-workspace");
  desktop.dataset.workspaceTemplate = template;
  query('[data-area-title="files"]').textContent = presentation.files;
  query('[data-area-title="canvas"]').textContent = presentation.canvas;
  if (!state.activeFile) elements.editorTitle.textContent = presentation.editor;
  queryAll("[data-mobile-panels] [data-focus-window]").forEach((tab, index) => {
    tab.textContent = presentation.tabs[index];
  });
}

function renderProjectTabs() {
  elements.projectTabs.querySelectorAll("[data-project-id]").forEach((tab) => tab.remove());
  for (const id of state.openWorkspaces) {
    const record = state.workspaceRecords.get(id);
    if (!record) continue;
    const button = document.createElement("button");
    button.type = "button";
    button.className = `project-tab${state.currentProject?.id === id && state.workspace === 1 ? " is-active" : ""}`;
    button.dataset.projectId = id;
    button.title = record.name;
    button.append(document.createTextNode(record.name.toUpperCase()));
    const close = document.createElement("span");
    close.className = "project-tab-close";
    close.textContent = "×";
    close.setAttribute("role", "button");
    close.setAttribute("aria-label", `Close ${record.name}`);
    close.addEventListener("click", (event) => {
      event.stopPropagation();
      closeWorkspace(id).catch((error) => toast(errorText(error), true));
    });
    button.append(close);
    button.addEventListener("click", () => openWorkspace(record).catch((error) => toast(errorText(error), true)));
    elements.projectTabs.append(button);
  }
}

async function renderSavedWorkspaces() {
  const records = await state.workspaceRepository.list();
  state.workspaceRecords = new Map(records.map((record) => [record.id, record]));
  elements.savedWorkspaces.replaceChildren();
  if (!records.length) {
    const empty = document.createElement("span");
    empty.className = "launcher-empty";
    empty.textContent = "NO SAVED WORKSPACES";
    elements.savedWorkspaces.append(empty);
  }
  for (const record of records) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "saved-workspace";
    button.textContent = `${record.name.toUpperCase()}  //  ${record.template.toUpperCase()}`;
    button.addEventListener("click", () => openWorkspace(record).catch((error) => toast(errorText(error), true)));
    elements.savedWorkspaces.append(button);
  }
  renderProjectTabs();
}

async function syncProjectIntoKernel(record) {
  const files = await state.workspaceRepository.files(record.id);
  for (const [path, content] of files) {
    await state.broker.eval(kernelName(record.id), studioSource(
      `(fs/write! ${JSON.stringify(`workspace-${record.id}`)} ${JSON.stringify(path)} ${JSON.stringify(content)})`
    ));
  }
}

async function openWorkspace(record) {
  if (!state.broker || !state.defaultBootstrap) {
    toast("RUNTIME STILL BOOTING", true);
    return;
  }
  const name = kernelName(record.id);
  const space = `workspace-${record.id}`;
  if (!state.openWorkspaces.includes(record.id)) state.openWorkspaces.push(record.id);
  state.workspaceRecords.set(record.id, record);
  state.kernelSpaces.set(name, space);
  if (!state.broker.list().includes(name)) {
    await state.broker.create(name, { bootstrap: state.defaultBootstrap(space) });
    await syncProjectIntoKernel(record);
  }
  if (state.activeDocument) {
    state.broker.releaseDocument(state.activeKernel, state.activeDocument.id);
    state.activeDocument = null;
  }
  state.currentProject = record;
  state.activeKernel = name;
  state.activeSpace = space;
  state.activeFile = null;
  state.dirty = false;
  applyWorkspacePresentation(record);
  setWorkspace(1);
  renderProjectTabs();
  const files = await listFiles();
  const path = files.includes("/src/main.hal") ? "/src/main.hal" :
    files.includes("/workspace.edn") ? "/workspace.edn" : files[0];
  if (path) await openFile(path, true);
  elements.publish.disabled = false;
}

async function closeWorkspace(id) {
  const name = kernelName(id);
  if (state.broker?.list().includes(name)) await state.broker.close(name);
  state.openWorkspaces = state.openWorkspaces.filter((value) => value !== id);
  state.kernelSpaces.delete(name);
  if (state.currentProject?.id === id) {
    const next = state.workspaceRecords.get(state.openWorkspaces.at(-1));
    state.currentProject = null;
    state.activeKernel = ROOT;
    state.activeSpace = SPACE;
    state.activeDocument = null;
    if (next) await openWorkspace(next);
    else {
      elements.publish.disabled = true;
      setWorkspace(0);
    }
  }
  renderProjectTabs();
}

function setLauncher(open) {
  elements.launcher.classList.toggle("is-open", open);
  elements.launcherScrim.classList.toggle("is-open", open);
  elements.launcher.setAttribute("aria-hidden", String(!open));
  elements.launcherToggle.setAttribute("aria-expanded", String(open));
  if (open) query(".app-tile", elements.launcher)?.focus();
}

function closeLauncher() {
  setLauncher(false);
}

function focusWindow(windowNode) {
  if (!windowNode) return;
  state.zIndex += 1;
  for (const other of queryAll("[data-window]")) other.classList.remove("is-focused");
  windowNode.classList.remove("is-hidden");
  windowNode.classList.add("is-focused");
  windowNode.style.zIndex = String(state.zIndex);
  for (const tab of queryAll("[data-focus-window]")) {
    tab.setAttribute("aria-selected", String(tab.dataset.focusWindow === windowNode.dataset.window));
  }
  saveWindows();
}

function openWindow(name) {
  setWorkspace(1);
  const windowNode = query(`[data-window="${name}"]`);
  focusWindow(windowNode);
  closeLauncher();
}

function showWorkspaceWindows() {
  for (const windowNode of queryAll("[data-window]")) {
    windowNode.classList.remove("is-hidden", "is-maximized");
    for (const property of ["left", "top", "width", "height"]) windowNode.style[property] = "";
  }
  focusWindow(query('[data-window="editor"]'));
}

function serializeWindows() {
  return Object.fromEntries(queryAll("[data-window]").map((windowNode) => [
    windowNode.dataset.window,
    {
      left: windowNode.style.left || null,
      top: windowNode.style.top || null,
      width: windowNode.style.width || null,
      height: windowNode.style.height || null,
      zIndex: Number(windowNode.style.zIndex) || 10,
      hidden: windowNode.classList.contains("is-hidden"),
      maximized: windowNode.classList.contains("is-maximized")
    }
  ]));
}

function saveWindows() {
  localStorage.setItem(WINDOWS_KEY, JSON.stringify(serializeWindows()));
}

function restoreWindows() {
  let saved = null;
  try {
    saved = JSON.parse(localStorage.getItem(WINDOWS_KEY));
  } catch {
    localStorage.removeItem(WINDOWS_KEY);
  }
  if (!saved) {
    focusWindow(query('[data-window="editor"]'));
    return;
  }
  for (const windowNode of queryAll("[data-window]")) {
    const item = saved[windowNode.dataset.window];
    if (!item) continue;
    for (const property of ["left", "top", "width", "height"]) {
      if (item[property]) windowNode.style[property] = item[property];
    }
    windowNode.style.zIndex = String(item.zIndex || 10);
    windowNode.classList.toggle("is-hidden", Boolean(item.hidden));
    windowNode.classList.toggle("is-maximized", Boolean(item.maximized));
    state.zIndex = Math.max(state.zIndex, item.zIndex || 10);
  }
  const visible = queryAll("[data-window]:not(.is-hidden)")
    .sort((left, right) => Number(right.style.zIndex) - Number(left.style.zIndex));
  focusWindow(visible[0] ?? query('[data-window="editor"]'));
}

function installWindowManager() {
  let saveTimer = 0;
  const scheduleSave = () => {
    clearTimeout(saveTimer);
    saveTimer = setTimeout(saveWindows, 120);
  };

  for (const windowNode of queryAll("[data-window]")) {
    const handle = query("[data-drag-handle]", windowNode);
    windowNode.addEventListener("pointerdown", () => focusWindow(windowNode));
    handle.addEventListener("dblclick", () => {
      windowNode.classList.toggle("is-maximized");
      saveWindows();
    });
    handle.addEventListener("pointerdown", (event) => {
      if (event.target.closest("button") || innerWidth <= 900 ||
          windowNode.classList.contains("is-maximized")) return;
      event.preventDefault();
      focusWindow(windowNode);
      const desktop = query(".desktop-workspace").getBoundingClientRect();
      const rect = windowNode.getBoundingClientRect();
      const originX = event.clientX;
      const originY = event.clientY;
      const startLeft = rect.left - desktop.left;
      const startTop = rect.top - desktop.top;
      handle.setPointerCapture(event.pointerId);

      const move = (moveEvent) => {
        const maxLeft = Math.max(0, desktop.width - 120);
        const maxTop = Math.max(0, desktop.height - 70);
        windowNode.style.left = `${Math.max(0, Math.min(maxLeft, startLeft + moveEvent.clientX - originX))}px`;
        windowNode.style.top = `${Math.max(0, Math.min(maxTop, startTop + moveEvent.clientY - originY))}px`;
      };
      const end = () => {
        handle.removeEventListener("pointermove", move);
        handle.removeEventListener("pointerup", end);
        saveWindows();
      };
      handle.addEventListener("pointermove", move);
      handle.addEventListener("pointerup", end);
    });

    query("[data-window-close]", windowNode).addEventListener("click", () => {
      windowNode.classList.add("is-hidden");
      const next = queryAll("[data-window]:not(.is-hidden)")[0];
      if (next) focusWindow(next);
      saveWindows();
    });

    query("[data-window-maximize]", windowNode).addEventListener("click", () => {
      windowNode.classList.toggle("is-maximized");
      focusWindow(windowNode);
      saveWindows();
      if (state.lastScene) requestAnimationFrame(drawLastScene);
    });

    new ResizeObserver(() => {
      scheduleSave();
      if (windowNode.dataset.window === "canvas" && state.lastScene) drawLastScene();
    }).observe(windowNode);
  }
}

function installWorkspaceNavigation() {
  if (elements.backgroundSource) {
    elements.backgroundSource.value = state.backgroundSource;
    elements.backgroundSource.addEventListener("change", () => {
      loadBackgroundSource(elements.backgroundSource.value).catch(() => {});
    });
  }
  elements.audioToggle.addEventListener("click", () => {
    controlBackgroundAudio("toggle").catch((error) => toast(`AUDIO FAILED: ${errorText(error)}`, true));
  });
  elements.audioVolume.addEventListener("input", () => {
    controlBackgroundAudio("volume", Number(elements.audioVolume.value))
      .catch((error) => toast(`AUDIO FAILED: ${errorText(error)}`, true));
  });
  query("[data-start]").addEventListener("click", openTemplateDialog);
  queryAll("[data-home]").forEach((button) => button.addEventListener("click", () => setWorkspace(0)));
  query("[data-workspace-prev]").addEventListener("click", () => {
    if (state.workspace === 1) setWorkspace(0);
  });
  query("[data-workspace-next]").addEventListener("click", () => {
    const record = state.workspaceRecords.get(state.openWorkspaces[0]);
    if (record) openWorkspace(record).catch((error) => toast(errorText(error), true));
    else openTemplateDialog();
  });

  document.addEventListener("keydown", (event) => {
    if (elements.dialog.open || elements.launcher.classList.contains("is-open")) return;
    if (event.target.matches("input, textarea, select")) return;
    if (event.key === "ArrowRight" && state.workspace === 0) {
      const record = state.workspaceRecords.get(state.openWorkspaces[0]);
      if (record) openWorkspace(record).catch((error) => toast(errorText(error), true));
    }
    if (event.key === "ArrowLeft" && state.workspace === 1) setWorkspace(0);
  });

  const viewport = query(".workspace-viewport");
  let swipeStart = null;
  viewport.addEventListener("pointerdown", (event) => {
    if (event.target.closest(".app-window")) return;
    swipeStart = { x: event.clientX, y: event.clientY };
  });
  viewport.addEventListener("pointerup", (event) => {
    if (!swipeStart) return;
    const dx = event.clientX - swipeStart.x;
    const dy = event.clientY - swipeStart.y;
    swipeStart = null;
    if (Math.abs(dx) > 70 && Math.abs(dx) > Math.abs(dy) * 1.4) {
      if (dx > 0) setWorkspace(0);
    }
  });
}

function installHelp() {
  for (const button of queryAll("[data-help]")) {
    button.addEventListener("click", () => elements.helpDialog.showModal());
  }
  query("[data-help-close]").addEventListener("click", () => elements.helpDialog.close());
}

function installLauncher() {
  elements.launcherToggle.addEventListener("click", () => {
    setLauncher(!elements.launcher.classList.contains("is-open"));
  });
  elements.launcherScrim.addEventListener("click", closeLauncher);
  query("[data-new-workspace]").addEventListener("click", openTemplateDialog);
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && elements.launcher.classList.contains("is-open")) {
      closeLauncher();
      elements.launcherToggle.focus();
    }
  });
}

function openTemplateDialog() {
  closeLauncher();
  elements.workspaceName.value = "Untitled Workspace";
  elements.templateDialog.showModal();
  requestAnimationFrame(() => elements.workspaceName.select());
}

function installWorkspaceCreation() {
  const symbols = { blank: "◇", canvas: "◎", music: "♫", "3d": "3D", graphs: "∿" };
  for (const template of workspaceTemplates) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "template-option";
    button.dataset.template = template.id;
    button.dataset.symbol = symbols[template.id];
    button.textContent = template.label.toUpperCase();
    button.addEventListener("click", async () => {
      button.disabled = true;
      try {
        const record = await state.workspaceRepository.create({
          name: elements.workspaceName.value,
          template: template.id
        });
        elements.templateDialog.close();
        await renderSavedWorkspaces();
        await openWorkspace(record);
      } catch (error) {
        toast(`WORKSPACE FAILED: ${errorText(error)}`, true);
      } finally {
        button.disabled = false;
      }
    });
    elements.templateGrid.append(button);
  }
  query("[data-template-close]").addEventListener("click", () => elements.templateDialog.close());
}

function installPublishing() {
  let session = null;
  let loginGeneration = 0;
  const auth = new GitHubDeviceAuth({ clientId: globalThis.HARA_GITHUB_OAUTH_CLIENT_ID });
  const close = () => { loginGeneration += 1; elements.publishDialog.close(); };
  const status = (message, error = false) => {
    elements.publishNote.textContent = message;
    elements.publishNote.classList.toggle("is-error", error);
  };
  const render = async () => {
    if (!state.currentProject) return;
    const files = await state.workspaceRepository.files(state.currentProject.id);
    elements.publishName.textContent = state.currentProject.name;
    elements.publishFiles.textContent = `${files.size} ${files.size === 1 ? "FILE" : "FILES"} READY`;
    elements.publishGist.textContent = session ? "PUBLISH TO GITHUB GIST" : "SIGN IN WITH GITHUB";
    elements.publishGist.disabled = !auth.configured() && !session;
    elements.publishGist.title = auth.configured() || session ? "" : "GitHub login is not configured for this deployment.";
    elements.publishAuth.hidden = true;
    status(session ? `SIGNED IN AS ${session.user.login.toUpperCase()}.` : "CONNECT GITHUB TO PUBLISH A GIST.");
  };
  elements.publish.addEventListener("click", () => void render().then(() => elements.publishDialog.showModal()));
  query("[data-publish-close]").addEventListener("click", close);
  elements.publishDialog.addEventListener("close", () => { loginGeneration += 1; });
  query("[data-publish-provider='download']").addEventListener("click", async () => {
    if (!state.currentProject) return;
    const filename = downloadWorkspace(await workspaceBundle(state.workspaceRepository, state.currentProject.id));
    status(`${filename.toUpperCase()} SAVED TO DISK.`);
  });
  elements.publishGist.addEventListener("click", async () => {
    if (!state.currentProject) return;
    elements.publishGist.disabled = true;
    try {
      if (!session) {
        const generation = ++loginGeneration;
        const device = await auth.begin();
        elements.publishAuth.hidden = false;
        elements.publishCode.textContent = device.user_code;
        elements.publishAuth.href = device.verification_uri;
        elements.publishAuth.textContent = "OPEN GITHUB & AUTHORIZE ↗";
        status("ENTER THE CODE ON GITHUB. WAITING FOR AUTHORIZATION.");
        const token = await auth.authorize(device, { cancelled: () => generation !== loginGeneration });
        const user = await githubRequest("/user", { token: token.access_token });
        if (generation !== loginGeneration) return;
        session = { token: token.access_token, user };
        await render();
        return;
      }
      status("PUBLISHING WORKSPACE TO GITHUB…");
      const bundle = await workspaceBundle(state.workspaceRepository, state.currentProject.id);
      const gist = await new GistPublisher({
        request: (path, options) => githubRequest(path, { ...options, token: session.token })
      }).publish(bundle, {
        public: query("[data-publish-public]").checked,
        previous: state.currentProject.providers?.gist
      });
      await state.workspaceRepository.setProvider(state.currentProject.id, "gist", {
        id: gist.id, url: gist.html_url, public: gist.public
      });
      state.currentProject = await state.workspaceRepository.get(state.currentProject.id);
      status(`GIST PUBLISHED: ${gist.html_url}`);
      elements.publishGist.textContent = "UPDATE GITHUB GIST";
    } catch (error) {
      if (error.message !== "GITHUB_LOGIN_CANCELLED") status(`PUBLISH FAILED: ${errorText(error)}`, true);
    } finally {
      elements.publishGist.disabled = !session && !auth.configured();
    }
  });
}

function installWorkspaceTabs() {
  for (const tab of queryAll("[data-focus-window]")) {
    tab.addEventListener("click", () => openWindow(tab.dataset.focusWindow));
  }
}

function studioSource(form) {
  return `(do (require [studio.space :as space]) (require [studio.fs :as fs]) ${form})`;
}

function evalStudio(form) {
  return state.broker.eval(state.activeKernel, studioSource(form));
}

async function listFiles() {
  const result = await evalStudio(`(space/files ${JSON.stringify(state.activeSpace)})`);
  state.files = (Array.isArray(result) ? result.map(String) : []).sort();
  renderFiles();
  return state.files;
}

function renderFiles() {
  elements.fileTree.replaceChildren();
  const groups = new Map();
  for (const path of state.files) {
    const parts = path.replace(/^\//, "").split("/");
    const group = parts.length > 1 ? parts[0].toUpperCase() : "ROOT";
    if (!groups.has(group)) groups.set(group, []);
    groups.get(group).push({ path, name: parts.at(-1) });
  }
  for (const [group, files] of groups) {
    const label = document.createElement("div");
    label.className = "file-group";
    label.textContent = group;
    elements.fileTree.append(label);
    for (const file of files) {
      const button = document.createElement("button");
      button.type = "button";
      button.className = `file-row${file.path === state.activeFile ? " is-active" : ""}`;
      button.dataset.file = file.path;
      button.textContent = file.name;
      button.addEventListener("click", () => openFile(file.path));
      elements.fileTree.append(button);
    }
  }
  if (!state.files.length) {
    const empty = document.createElement("div");
    empty.className = "window-loading";
    empty.textContent = "EMPTY SPACE";
    elements.fileTree.append(empty);
  }
  elements.fileCount.textContent = `${state.files.length} FILE${state.files.length === 1 ? "" : "S"}`;
}

function updateEditorChrome() {
  elements.editorTitle.textContent = state.activeFile ? state.activeFile.toUpperCase() : "EDITOR";
  elements.dirty.classList.toggle("is-dirty", state.dirty);
  elements.editor.disabled = !state.activeFile;
  elements.save.disabled = !state.activeFile;
  elements.run.disabled = !state.activeFile;
  renderLineNumbers();
  renderFiles();
}

function changedLineNumbers(previous, current) {
  const before = previous.split("\n");
  const after = current.split("\n");
  let prefix = 0;
  while (prefix < before.length && prefix < after.length && before[prefix] === after[prefix]) prefix += 1;
  let suffix = 0;
  while (suffix < before.length - prefix && suffix < after.length - prefix &&
         before.at(-1 - suffix) === after.at(-1 - suffix)) suffix += 1;
  return new Set(Array.from({ length: Math.max(0, after.length - prefix - suffix) }, (_, index) => prefix + index));
}

function renderLineNumbers() {
  const lines = elements.editor.value.split("\n");
  const changed = changedLineNumbers(state.savedSource, elements.editor.value);
  elements.lineNumbers.innerHTML = lines.map((_, index) =>
    `<span class="${changed.has(index) ? "is-changed" : ""}">${index + 1}</span>`
  ).join("\n");
}

function topLevelForms(source) {
  const balanced = (() => {
    const stack = [], found = [];
    let string = false, comment = false, escaped = false;
    const pairs = { "(": ")", "[": "]", "{": "}" };
    for (let index = 0; index < source.length; index += 1) {
      const character = source[index];
      if (comment) { if (character === "\n") comment = false; continue; }
      if (string) { if (!escaped && character === '"') string = false; escaped = !escaped && character === "\\"; continue; }
      if (character === ";") { comment = true; continue; }
      if (character === '"') { string = true; escaped = false; continue; }
      if (pairs[character]) stack.push({ opener: character, start: index });
      else if (stack.length && pairs[stack.at(-1).opener] === character) {
        const form = stack.pop();
        found.push({ start: form.start, end: index + 1 });
      }
    }
    return found;
  })();
  return balanced.filter((form) => !balanced.some((outer) => outer !== form && outer.start < form.start && form.end < outer.end))
    .sort((left, right) => left.start - right.start)
    .map((form) => source.slice(form.start, form.end));
}

function structuralDiffText() {
  const before = topLevelForms(state.savedSource);
  const after = topLevelForms(elements.editor.value);
  const added = Math.max(0, after.length - before.length);
  const removed = Math.max(0, before.length - after.length);
  const changed = Math.min(before.length, after.length) - before.filter((form, index) => form === after[index]).length;
  const lines = changedLineNumbers(state.savedSource, elements.editor.value);
  const title = `STRUCTURAL DIFF // ${changed + added + removed ? "CHANGED" : "CLEAN"}`;
  const forms = [`~ ${changed} CHANGED`, `+ ${added} ADDED`, `- ${removed} REMOVED`];
  const lineLabel = lines.size ? `CHANGED LINES // ${[...lines].map((line) => line + 1).join(", ")}` : "CHANGED LINES // —";
  return [title, ...forms, lineLabel].join("\n");
}

function updateStructuralDiff() {
  elements.structuralDiff.textContent = structuralDiffText();
}

function editorSnapshot() {
  return {
    value: elements.editor.value,
    start: elements.editor.selectionStart,
    end: elements.editor.selectionEnd
  };
}

function sameSnapshot(left, right) {
  return left?.value === right?.value && left?.start === right?.start && left?.end === right?.end;
}

function resetEditorHistory() {
  state.editorHistory = { past: [], future: [], current: editorSnapshot(), replaying: false };
}

function recordEditorChange() {
  const history = state.editorHistory;
  if (history.replaying) return;
  const next = editorSnapshot();
  if (sameSnapshot(history.current, next)) return;
  if (history.current) history.past.push(history.current);
  history.future = [];
  history.current = next;
}

function restoreEditorSnapshot(snapshot) {
  const history = state.editorHistory;
  history.replaying = true;
  elements.editor.value = snapshot.value;
  elements.editor.setSelectionRange(snapshot.start, snapshot.end);
  history.current = snapshot;
  history.replaying = false;
  state.dirty = true;
  updateEditorChrome();
  updateCompletions();
  syncHighlight();
  updateStructuralDiff();
}

function undoEditor() {
  const history = state.editorHistory;
  const previous = history.past.pop();
  if (!previous) return false;
  if (history.current) history.future.push(history.current);
  restoreEditorSnapshot(previous);
  return true;
}

function redoEditor() {
  const history = state.editorHistory;
  const next = history.future.pop();
  if (!next) return false;
  if (history.current) history.past.push(history.current);
  restoreEditorSnapshot(next);
  return true;
}

async function openFile(path, force = false, activateWorkspace = true) {
  if (state.dirty && !force) {
    const discard = await confirmDialog("UNSAVED CHANGES", "Discard the current editor changes?");
    if (!discard) return;
  }
  const content = await evalStudio(`(fs/read ${JSON.stringify(state.activeSpace)} ${JSON.stringify(path)})`);
  if (state.activeDocument && state.activeDocument.path !== path) {
    state.broker.releaseDocument(state.activeKernel, state.activeDocument.id);
    state.nodeRuntime?.releaseDocument(state.activeDocument.id);
    state.activeDocument = null;
  }
  state.activeFile = path;
  state.dirty = false;
  elements.editor.value = content == null ? "" : String(content);
  state.savedSource = elements.editor.value;
  resetEditorHistory();
  elements.editorStatus.textContent = "READY";
  localStorage.setItem(ACTIVE_FILE_KEY, path);
  updateEditorChrome();
  syncHighlight();
  updateStructuralDiff();
  if (activateWorkspace) openWindow("editor");
}

async function saveFile(showToast = true) {
  if (!state.activeFile) return false;
  elements.editorStatus.textContent = "SAVING";
  if (state.activeFile === "/workspace.edn") {
    try {
      const manifest = await state.broker.eval(state.activeKernel, `(quote ${elements.editor.value})`);
      for (const key of ["workspace/id", "workspace/layout", "workspace/documents", "workspace/areas",
        "workspace/nodes", "workspace/connections", "workspace/links", "workspace/customizations"]) {
        if (mapValue(manifest, key) === undefined) throw new Error(`workspace.edn missing :${key}`);
      }
      const customizations = mapValue(manifest, "workspace/customizations");
      const template = keywordName(mapValue(customizations, "template"));
      if (workspacePresentation[template]) {
        state.currentProject = { ...state.currentProject, template };
        state.workspaceRecords.set(state.currentProject.id, state.currentProject);
        await state.workspaceRepository.setTemplate(state.currentProject.id, template);
        applyWorkspacePresentation(state.currentProject);
      }
    } catch (error) {
      elements.editorStatus.textContent = `WORKSPACE ERROR // ${errorText(error)}`;
      toast(`WORKSPACE LAYOUT NOT APPLIED: ${errorText(error)}`, true);
      return false;
    }
  }
  await evalStudio(
    `(fs/write! ${JSON.stringify(state.activeSpace)} ${JSON.stringify(state.activeFile)} ${JSON.stringify(elements.editor.value)})`
  );
  if (state.currentProject) {
    await state.workspaceRepository.writeFile(state.currentProject.id, state.activeFile, elements.editor.value);
  }
  state.dirty = false;
  state.savedSource = elements.editor.value;
  elements.editorStatus.textContent = "SAVED";
  updateEditorChrome();
  updateStructuralDiff();
  if (showToast) toast(`SAVED ${state.activeFile}`);
  return true;
}

function drawLastScene() {
  if (!state.lastScene || query('[data-window="canvas"]').classList.contains("is-hidden")) return;
  renderScene(elements.outputCanvas, state.lastScene);
}

function resultLabel(value) {
  if (value == null) return "NIL";
  if (typeof value === "string") return JSON.stringify(value).slice(0, 90);
  try { return JSON.stringify(value).slice(0, 90); } catch { return String(value).slice(0, 90); }
}

function html(value) {
  return value.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;");
}

function highlightHara(source) {
  let output = "";
  let depth = 0;
  let string = false;
  let comment = false;
  let escaped = false;
  const target = (index) => state.evalRange && index >= state.evalRange.start && index < state.evalRange.end ? " eval-target" : "";
  for (let index = 0; index < source.length; index += 1) {
    const character = source[index];
    if (comment) { output += `<span class="comment${target(index)}">${html(character)}</span>`; if (character === "\n") comment = false; continue; }
    if (string) {
      output += `<span class="string${target(index)}">${html(character)}</span>`;
      if (!escaped && character === '"') string = false;
      escaped = !escaped && character === "\\";
      continue;
    }
    if (character === ";") { comment = true; output += `<span class="comment${target(index)}">;</span>`; continue; }
    if (character === '"') { string = true; escaped = false; output += `<span class="string${target(index)}">"</span>`; continue; }
    if ("([{".includes(character)) { output += `<span class="paren-${depth % 6}${target(index)}">${character}</span>`; depth += 1; continue; }
    if (")] }".replace(" ", "").includes(character)) {
      depth -= 1;
      output += `<span class="${depth < 0 ? "unmatched" : `paren-${depth % 6}`}${target(index)}">${character}</span>`;
      continue;
    }
    if (character === ":") {
      const match = source.slice(index).match(/^:[A-Za-z*+!?._/-]+/);
      if (match) { output += `<span class="keyword${target(index)}">${html(match[0])}</span>`; index += match[0].length - 1; continue; }
    }
    output += target(index) ? `<span class="eval-target">${html(character)}</span>` : html(character);
  }
  return output;
}

function syncHighlight() {
  elements.codeHighlight.innerHTML = highlightHara(elements.editor.value);
  elements.codeHighlight.style.transform = `translate(${-elements.editor.scrollLeft}px, ${-elements.editor.scrollTop}px)`;
}

function syncBackgroundHighlight() {
  if (!elements.sourceEditor || !elements.sourceHighlight) return;
  elements.sourceHighlight.innerHTML = highlightHara(elements.sourceEditor.value);
  elements.sourceHighlight.style.transform =
    `translate(${-elements.sourceEditor.scrollLeft}px, ${-elements.sourceEditor.scrollTop}px)`;
}

function setSourcePanel(open) {
  elements.sourcePanel.classList.toggle("is-open", open);
  elements.sourcePanel.setAttribute("aria-hidden", String(!open));
  elements.sourceToggle.setAttribute("aria-expanded", String(open));
  document.body.classList.toggle("source-open", open);
}

function scheduleBackgroundPreview() {
  clearTimeout(state.sourceTimer);
  const documentId = elements.sourceEditor.dataset.documentId;
  if (!documentId) return;
  localStorage.setItem(sourceStorageKey(documentId, "recovery"), elements.sourceEditor.value);
  elements.sourceStatus.textContent = "EVALUATING CANDIDATE";
  state.sourceTimer = setTimeout(() => {
    loadBackgroundSource(documentId, elements.sourceEditor.value).catch(() => {});
  }, 320);
}

async function saveBackgroundSource() {
  const descriptor = state.backgroundDocuments.get(elements.sourceEditor.dataset.documentId);
  if (!descriptor) return;
  const current = await fetch(new URL(descriptor.path, import.meta.url), { cache: "no-store" });
  const currentSource = await current.text();
  if (currentSource !== elements.sourceEditor.dataset.baseSource) {
    elements.sourceStatus.textContent = "CONFLICT // BUNDLED SOURCE CHANGED";
    toast("SOURCE CONFLICT: RELOAD BEFORE SAVING", true);
    return;
  }
  localStorage.setItem(sourceStorageKey(descriptor.id, "saved"), elements.sourceEditor.value);
  localStorage.removeItem(sourceStorageKey(descriptor.id, "recovery"));
  elements.sourceStatus.textContent = "SAVED // INDEXEDDB OVERLAY";
  toast(`SAVED ${descriptor.title.toUpperCase()}.HAL LOCALLY`);
}

function installBackgroundEditor() {
  elements.sourceToggle.addEventListener("click", () =>
    setSourcePanel(!elements.sourcePanel.classList.contains("is-open")));
  elements.sourceEditor.addEventListener("input", () => {
    syncBackgroundHighlight();
    scheduleBackgroundPreview();
  });
  elements.sourceEditor.addEventListener("scroll", syncBackgroundHighlight);
  elements.sourceEditor.addEventListener("keydown", (event) => {
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") {
      event.preventDefault();
      void saveBackgroundSource();
      return;
    }
    if (event.key === "Tab") {
      event.preventDefault();
      structuralAlign(elements.sourceEditor);
      elements.sourceEditor.dispatchEvent(new Event("input", { bubbles: true }));
      return;
    }
    if (applyParedit(elements.sourceEditor, event.key)) {
      event.preventDefault();
      elements.sourceEditor.dispatchEvent(new Event("input", { bubbles: true }));
    }
  });
  elements.sourceSave.addEventListener("click", () => void saveBackgroundSource());
}

function syncAudioControls(result = null) {
  const playing = result instanceof Map ? result.get("playing") : result?.playing ?? audioPipeline.playing;
  const volume = result instanceof Map ? result.get("volume") : result?.volume;
  elements.audioToggle.setAttribute("aria-pressed", String(playing));
  elements.audioToggle.textContent = playing ? "■ STOP" : "♪ PLAY";
  if (typeof volume === "number") elements.audioVolume.value = String(volume);
}

async function controlBackgroundAudio(command, value = null) {
  const nodeId = state.activeBackground?.nodeId;
  if (!nodeId || state.activeBackground.descriptor.id !== SIGNAL_RING_BACKGROUND) return;
  const response = await state.nodeRuntime.call("ui/home", nodeId, "audio/control", [command, value]);
  syncAudioControls(response.data);
}

function positionEditorOverlay(node, offset) {
  const source = elements.editor.value.slice(0, offset);
  const line = source.split("\n").length - 1;
  const column = source.length - source.lastIndexOf("\n") - 1;
  node.style.top = `${14 + line * 18 - elements.editor.scrollTop}px`;
  node.style.left = `${62 + Math.min(column * 7.1, Math.max(30, elements.editor.clientWidth - 190))}px`;
}

function showInlineEval(form, label, error = false) {
  elements.inlineEval.textContent = error ? `ERROR => ${label}` : `=> ${label}`;
  elements.inlineEval.classList.toggle("is-error", error);
  elements.inlineEval.classList.remove("is-pending");
  elements.inlineEval.hidden = false;
  positionEditorOverlay(elements.inlineEval, form.end ?? elements.editor.selectionEnd);
}

function showScene(scene, started, target) {
  elements.creativeCanvas.hidden = true;
  elements.outputCanvas.hidden = false;
  state.lastScene = scene;
  query('[data-window="canvas"]').classList.remove("is-hidden");
  drawLastScene();
  elements.canvasEmpty.classList.add("is-hidden");
  elements.canvasStatus.textContent = `FRAME // ${Math.round(performance.now() - started)} MS`;
  elements.canvasSize.textContent = `${scene.width} × ${scene.height}`;
  elements.editorStatus.textContent = `${target} RENDERED`;
  if (innerWidth <= 900) focusWindow(query('[data-window="canvas"]'));
  toast(`${target} RENDERED`);
}

function showCreative(scene, started, target) {
  state.creativeRuntime ??= new CreativeRuntime(elements.creativeCanvas);
  elements.outputCanvas.hidden = true;
  elements.creativeCanvas.hidden = false;
  state.creativeRuntime.render(scene);
  query('[data-window="canvas"]').classList.remove("is-hidden");
  elements.canvasEmpty.classList.add("is-hidden");
  elements.canvasStatus.textContent = `3D // ${Math.round(performance.now() - started)} MS`;
  elements.canvasSize.textContent = `${scene.entities.length} ENTITY${scene.entities.length === 1 ? "" : "IES"}`;
  elements.editorStatus.textContent = `${target} CREATIVE`;
  toast(`${target} CREATIVE SCENE`);
}

function hideCompletions() {
  elements.completions.hidden = true;
  completionState.entries = [];
}

function completionPrefix() {
  const before = elements.editor.value.slice(0, elements.editor.selectionStart);
  const match = before.match(/[:A-Za-z*+!?._/-]+$/);
  return match ? { value: match[0], start: before.length - match[0].length } : null;
}

function renderCompletions() {
  elements.completions.replaceChildren();
  for (const [index, entry] of completionState.entries.entries()) {
    const [form, detail] = entry;
    const item = document.createElement("button");
    item.type = "button";
    item.className = `hal-completion${index === completionState.index ? " is-active" : ""}`;
    item.setAttribute("role", "option");
    item.setAttribute("aria-selected", String(index === completionState.index));
    item.innerHTML = `<strong>${form}</strong><small>${detail}</small>`;
    item.addEventListener("mousedown", (event) => { event.preventDefault(); acceptCompletion(index); });
    elements.completions.append(item);
  }
  positionEditorOverlay(elements.completions, elements.editor.selectionStart);
  elements.completions.style.left = `${Math.max(52, Number.parseFloat(elements.completions.style.left) - 10)}px`;
  elements.completions.style.top = `${Number.parseFloat(elements.completions.style.top) + 20}px`;
  elements.completions.hidden = !completionState.entries.length;
}

function updateCompletions() {
  const prefix = completionPrefix();
  if (!prefix || prefix.value.length < 2) return hideCompletions();
  const entries = HAL_FORMS.filter(([form]) => form.startsWith(prefix.value)).slice(0, 8);
  if (!entries.length) return hideCompletions();
  completionState.entries = entries;
  completionState.index = 0;
  completionState.start = prefix.start;
  renderCompletions();
}

function acceptCompletion(index = completionState.index) {
  const entry = completionState.entries[index];
  if (!entry) return;
  elements.editor.setRangeText(entry[0], completionState.start, elements.editor.selectionStart, "end");
  elements.editor.dispatchEvent(new Event("input", { bubbles: true }));
  hideCompletions();
}

function formAtSelection() {
  const start = elements.editor.selectionStart;
  const end = elements.editor.selectionEnd;
  const selection = elements.editor.value.slice(start, end).trim();
  return selection ? { source: selection, start, end } : localFormAt(elements.editor.value, start);
}

function haraLiteral(value) {
  if (value == null) return "nil";
  if (typeof value === "string") return JSON.stringify(value);
  if (typeof value === "number" || typeof value === "bigint" || typeof value === "boolean") return String(value);
  if (Array.isArray(value)) return `[${value.map(haraLiteral).join(" ")}]`;
  if (value instanceof Set) return `#{${[...value].map(haraLiteral).join(" ")}}`;
  if (value instanceof Map) return `{${[...value].map(([key, entry]) => `${haraLiteral(key)} ${haraLiteral(entry)}`).join(" ")}}`;
  if (value?.constructor?.name === "HtaKeyword") return `:${value.name}`;
  if (value?.constructor?.name === "HtaSymbol") return value.name;
  return String(value);
}

async function evaluateAndInsert() {
  const form = formAtSelection();
  if (!form?.source) return;
  elements.editorStatus.textContent = "EVALUATING FOR INSERT";
  try {
    const result = state.activeDocument?.path === state.activeFile
      ? await state.broker.evalForm(state.activeKernel, state.activeDocument.id, form.source)
      : await state.broker.eval(state.activeKernel, form.source);
    const replacement = haraLiteral(result);
    elements.editor.setRangeText(replacement, form.start, form.end, "end");
    elements.editor.dispatchEvent(new Event("input", { bubbles: true }));
    elements.editorStatus.textContent = "RESULT INSERTED";
    showInlineEval({ end: form.start + replacement.length }, resultLabel(result));
  } catch (error) {
    const message = errorText(error);
    elements.editorStatus.textContent = `ERROR // ${message}`;
    showInlineEval(form, message, true);
  }
}

async function evaluateForm(form = null, target = "FORM") {
  if (!state.activeFile) return;
  elements.run.disabled = true;
  if (!form) {
    form = formAtSelection();
  }
  if (!form?.source) {
    elements.editorStatus.textContent = "NO FORM AT CURSOR";
    elements.run.disabled = false;
    return;
  }
  elements.editorStatus.textContent = `EVALUATING ${target}`;
  state.evalRange = { start: form.start, end: form.end };
  syncHighlight();
  elements.inlineEval.hidden = true;
  const started = performance.now();
  try {
    let result;
    if (target === "FILE" && isAnonymousDocument(form.source)) {
      const documentId = `document${state.activeFile}`;
      const nodeId = `node${state.activeFile}`;
      state.nodeRuntime.registerNode({ id: nodeId, type: "hal/transform" });
      const prepared = await state.broker.prepareDocument(state.activeKernel, documentId, form.source, { nodeId });
      try {
        await state.nodeRuntime.activateDocument(nodeId, {
          documentId,
          generation: prepared.generation,
          moduleId: prepared.moduleId,
          kernelContext: prepared.context
        });
        state.broker.commitDocument(prepared);
        state.activeDocument = { path: state.activeFile, id: documentId, nodeId };
        result = prepared.value;
      } catch (error) {
        state.broker.discardDocument(prepared);
        throw error;
      }
    } else if (state.activeDocument?.path === state.activeFile) {
      result = await state.broker.evalForm(state.activeKernel, state.activeDocument.id, form.source);
    } else {
      result = await state.broker.eval(state.activeKernel, form.source);
    }
    try {
      const scene = validateScene(result);
      state.evalRange = null;
      syncHighlight();
      showScene(scene, started, target);
    } catch {
      try {
        const creative = normalizeCreative(result);
        state.evalRange = null;
        syncHighlight();
        showCreative(creative, started, target);
      } catch {
        const label = resultLabel(result);
        elements.editorStatus.textContent = `EVAL // ${label}`;
        state.evalRange = null;
        syncHighlight();
        showInlineEval(form, label);
      }
    }
  } catch (error) {
    const message = errorText(error);
    elements.editorStatus.textContent = `ERROR // ${message}`;
    state.evalRange = null;
    syncHighlight();
    showInlineEval(form, message, true);
    elements.canvasStatus.textContent = "FRAME // LAST GOOD";
    toast(message, true);
  } finally {
    elements.run.disabled = false;
  }
}

function evaluateFile() {
  return evaluateForm({
    source: elements.editor.value,
    start: 0,
    end: elements.editor.value.length
  }, "FILE");
}

function isAnonymousDocument(source) {
  return /^\s*(?:(?:;[^\n]*(?:\n|$))|(?:#_\s*\([^)]*\)\s*))*\(ns\+(?=[\s()])/s.test(source);
}

function clearEditorPrefix() {
  clearTimeout(state.editorPrefixTimer);
  state.editorPrefixTimer = null;
  state.editorPrefix = null;
}

function startEditorPrefix(prefix, fallback = null) {
  clearEditorPrefix();
  state.editorPrefix = prefix;
  if (!fallback) return;
  state.editorPrefixTimer = setTimeout(() => {
    if (state.editorPrefix === prefix) {
      clearEditorPrefix();
      fallback?.();
    }
  }, 700);
}

function normalizePath(value) {
  if (typeof value !== "string") return null;
  let path = value.trim().replace(/\/+/g, "/");
  if (!path.startsWith("/")) path = `/${path}`;
  if (path === "/" || path.includes("..") || !/\.(hal|edn)$/i.test(path)) return null;
  return path;
}

async function seedFiles(force = false) {
  if (force) {
    for (const path of await listFiles()) {
      await evalStudio(`(fs/delete! ${JSON.stringify(state.activeSpace)} ${JSON.stringify(path)})`);
    }
  }
  for (const [path, content] of DEFAULT_FILES) {
    await evalStudio(
      `(fs/write! ${JSON.stringify(state.activeSpace)} ${JSON.stringify(path)} ${JSON.stringify(content)})`
    );
  }
  await listFiles();
}

function promptDialog({ title, label, value = "", message = "" }) {
  elements.dialogTitle.textContent = title;
  elements.dialogLabel.textContent = label;
  elements.dialogInput.value = value;
  elements.dialogMessage.textContent = message;
  elements.dialogInput.hidden = false;
  elements.dialog.showModal();
  requestAnimationFrame(() => elements.dialogInput.select());
  return new Promise((resolve) => {
    elements.dialogForm.addEventListener("submit", (event) => {
      resolve(event.submitter?.value === "confirm" ? elements.dialogInput.value : null);
    }, { once: true });
  });
}

function confirmDialog(title, message) {
  elements.dialogTitle.textContent = title;
  elements.dialogMessage.textContent = message;
  elements.dialogInput.hidden = true;
  elements.dialog.showModal();
  return new Promise((resolve) => {
    elements.dialogForm.addEventListener("submit", (event) => {
      elements.dialogInput.hidden = false;
      resolve(event.submitter?.value === "confirm");
    }, { once: true });
  });
}

function installFileActions() {
  query("[data-file-new]").addEventListener("click", async () => {
    const raw = await promptDialog({
      title: "NEW HARA FILE",
      label: "PATH",
      value: "/sketches/untitled.hal",
      message: ".hal is added automatically"
    });
    if (raw == null) return;
    const normalized = normalizePath(raw);
    const path = normalized && (normalized.toLowerCase().endsWith(".hal") ? normalized : `${normalized}.hal`);
    if (!path) return toast("INVALID HARA FILE PATH", true);
    if (state.files.includes(path)) return toast("FILE ALREADY EXISTS", true);
    const content = `;; ${path}\n\n{:version 1\n :width 960\n :height 600\n :background "#020408"\n :commands []}\n`;
    await evalStudio(`(fs/write! ${JSON.stringify(state.activeSpace)} ${JSON.stringify(path)} ${JSON.stringify(content)})`);
    if (state.currentProject) {
      await state.workspaceRepository.writeFile(state.currentProject.id, path, content);
    }
    await listFiles();
    await openFile(path, true);
    await evaluateFile();
  });

  query("[data-file-rename]").addEventListener("click", async () => {
    if (!state.activeFile) return;
    const raw = await promptDialog({
      title: "RENAME HARA FILE",
      label: "NEW PATH",
      value: state.activeFile,
      message: "The existing file contents will be preserved"
    });
    if (raw == null) return;
    const nextPath = normalizePath(raw);
    if (!nextPath) return toast("INVALID HARA FILE PATH", true);
    if (state.files.includes(nextPath) && nextPath !== state.activeFile) return toast("FILE ALREADY EXISTS", true);
    await saveFile(false);
    const oldPath = state.activeFile;
    await evalStudio(
      `(fs/write! ${JSON.stringify(state.activeSpace)} ${JSON.stringify(nextPath)} ${JSON.stringify(elements.editor.value)})`
    );
    if (state.currentProject) {
      await state.workspaceRepository.writeFile(state.currentProject.id, nextPath, elements.editor.value);
    }
    if (nextPath !== oldPath) {
      await evalStudio(`(fs/delete! ${JSON.stringify(state.activeSpace)} ${JSON.stringify(oldPath)})`);
      if (state.currentProject) {
        await state.workspaceRepository.deleteFile(state.currentProject.id, oldPath);
      }
    }
    state.activeFile = nextPath;
    localStorage.setItem(ACTIVE_FILE_KEY, nextPath);
    await listFiles();
    updateEditorChrome();
    updateStructuralDiff();
    toast(`RENAMED ${oldPath}`);
  });

  query("[data-file-delete]").addEventListener("click", async () => {
    if (!state.activeFile) return;
    const path = state.activeFile;
    if (!await confirmDialog("DELETE HARA FILE", `Delete ${path}? This cannot be undone.`)) return;
    await evalStudio(`(fs/delete! ${JSON.stringify(state.activeSpace)} ${JSON.stringify(path)})`);
    if (state.currentProject) {
      await state.workspaceRepository.deleteFile(state.currentProject.id, path);
    }
    state.activeFile = null;
    state.dirty = false;
    elements.editor.value = "";
    syncHighlight();
    localStorage.removeItem(ACTIVE_FILE_KEY);
    await listFiles();
    updateEditorChrome();
    if (state.files.length) await openFile(state.files[0], true);
    toast(`DELETED ${path}`);
  });

}

async function bootRuntime() {
  setRuntimeStatus("WASM // BOOTING", "booting");
  elements.editorStatus.textContent = "BOOTING HARA.WASM";
  try {
    const runtimeBase = new URL("./runtime/", import.meta.url);
    const [
      { createBrowserBroker },
      { createHostServices },
      { defaultBootstrap },
      { NodeRuntime },
      { CanvasRuntime },
      { GraphHost },
      { SessionRouter },
      { CapabilityRegistry },
      { createCanvasCapability },
      { createClockCapability }
    ] = await Promise.all([
      import(new URL("studio/broker.js", runtimeBase)),
      import(new URL("studio/host-services.js", runtimeBase)),
      import(new URL("studio/boot.js", runtimeBase)),
      import(new URL("studio/node-runtime.js", runtimeBase)),
      import(new URL("studio/canvas-runtime.js", runtimeBase)),
      import(new URL("studio/graph-host.js", runtimeBase)),
      import(new URL("studio/session-router.js", runtimeBase)),
      import(new URL("studio/capability-registry.js", runtimeBase)),
      import(new URL("studio/capabilities/canvas.js", runtimeBase)),
      import(new URL("studio/capabilities/clock.js", runtimeBase))
    ]);
    const wasmResponse = await fetch(new URL("hara.wasm", runtimeBase));
    if (!wasmResponse.ok) throw new Error(`runtime fetch failed: ${wasmResponse.status}`);
    const moduleBytes = new Uint8Array(await wasmResponse.arrayBuffer());
    const resources = {};
    for (const name of ["store", "fs", "space", "boot", "node", "draw", "audio", "program", "graph", "session"]) {
      const response = await fetch(new URL(`studio/hal/${name}.hal`, runtimeBase));
      if (!response.ok) throw new Error(`resource ${name} fetch failed: ${response.status}`);
      resources[`studio.${name}`] = await response.text();
    }
    for (const name of ["substrate/protocol", "substrate/frame", "substrate"]) {
      const response = await fetch(new URL(`std/${name}.hal`, runtimeBase));
      if (!response.ok) throw new Error(`resource std.${name.replaceAll("/", ".")} fetch failed: ${response.status}`);
      resources[`std.${name.replaceAll("/", ".")}`] = await response.text();
    }
    state.nodeRuntime = new NodeRuntime({ space: `workspace/${SPACE}` });
    state.canvasRuntime = new CanvasRuntime({
      onDiagnostic: (error) => {
        elements.sourceStatus.textContent = `DIAGNOSTIC // ${errorText(error)}`;
      }
    });
    state.canvasRuntime.register("canvas/background", query("[data-tron]"));
    const sessionRouter = new SessionRouter();
    const capabilityRegistry = new CapabilityRegistry({ adapters: {
      "surface/canvas-2d": createCanvasCapability(state.canvasRuntime),
      "clock/frame": createClockCapability()
    } });
    const graphHost = new GraphHost({
      workerUrl: new URL("studio/program-worker.js", runtimeBase),
      sessionRouter, capabilityRegistry
    });
    const hostCalls = createHostServices({
      dbName: "hara-www",
      scopeForContext: (context) => state.contextSpaces.get(context),
      nodeRuntime: state.nodeRuntime,
      canvasRuntime: state.canvasRuntime,
      graphHost,
      audioPipeline,
      graphHostOptions: { sessionRouter },
      renderCanvas: (_canvasId, value) => {
        showScene(validateScene(value), performance.now(), "HAL");
      }
    });
    state.broker = createBrowserBroker({
      workerUrl: new URL("hta-worker.js", runtimeBase),
      sharedWorkerUrl: new URLSearchParams(location.search).has("shared-runtime")
        ? new URL("hta-shared-worker.js", runtimeBase) : undefined,
      moduleBytes,
      hostCalls,
      resources,
      onKernelStarting: async (kernel) => {
        let space = state.kernelSpaces.get(kernel.name);
        if (!space && kernel.name.startsWith("DOC.")) {
          space = [...state.kernelSpaces.entries()]
            .find(([name]) => kernel.name.startsWith(`DOC.${name}.`))?.[1];
        }
        if (!space) throw new Error(`NO_WORKSPACE_SCOPE ${kernel.name}`);
        state.contextSpaces.set(kernel.context, space);
      },
      onKernelCreated: async (kernel) => sessionRouter.register(kernel.name, kernel.context, {
        onRelease: (sessionId) => graphHost.releaseSession(sessionId)
      }),
      onKernelClosed: (kernel) => sessionRouter.unregister(kernel.name)
    });
    state.defaultBootstrap = defaultBootstrap;
    await state.broker.eval(ROOT, defaultBootstrap(SPACE));
    await loadBackgroundWorkspace();
    await loadBackgroundSource(state.backgroundSource);
    const files = await listFiles();
    if (!files.length) await seedFiles();
    setRuntimeStatus("WASM // LIVE", "live");
    elements.editorStatus.textContent = "READY";
    const preferred = localStorage.getItem(ACTIVE_FILE_KEY);
    const path = state.files.includes(preferred) ? preferred :
      state.files.includes("/sketches/neon-orbit.hal") ? "/sketches/neon-orbit.hal" : state.files[0];
    if (path) await openFile(path, true, false);
    syncHighlight();
    await renderSavedWorkspaces();
    setWorkspace(0);
  } catch (error) {
    console.error("[hara www]", error);
    setRuntimeStatus("WASM // ERROR", "error");
    elements.editorStatus.textContent = `BOOT ERROR // ${errorText(error)}`;
    toast(`HARA RUNTIME FAILED: ${errorText(error)}`, true);
  }
}

function installEditor() {
  elements.editor.addEventListener("input", () => {
    state.evalRange = null;
    elements.inlineEval.hidden = true;
    recordEditorChange();
    state.dirty = true;
    updateEditorChrome();
    updateCompletions();
    syncHighlight();
    updateStructuralDiff();
  });
  elements.editor.addEventListener("scroll", () => {
    elements.lineNumbers.scrollTop = elements.editor.scrollTop;
    syncHighlight();
    if (!elements.inlineEval.hidden) positionEditorOverlay(elements.inlineEval, elements.editor.selectionEnd);
    if (!elements.completions.hidden) renderCompletions();
  });
  elements.editor.addEventListener("keydown", (event) => {
    const modifier = event.metaKey || event.ctrlKey;
    if (state.editorPrefix === "insert" && event.key.toLowerCase() === "e") {
      clearEditorPrefix();
      event.preventDefault();
      evaluateAndInsert();
      return;
    }
    if (state.editorPrefix === "run" && event.key.toLowerCase() === "e" && !event.repeat) {
      clearEditorPrefix();
      event.preventDefault();
      evaluateFile();
      return;
    }
    if (state.editorPrefixTimer) {
      clearEditorPrefix();
    }
    if (event.ctrlKey && !event.metaKey && !event.altKey && event.key.toLowerCase() === "x") {
      event.preventDefault();
      startEditorPrefix("insert", () => {});
      return;
    }
    if (modifier && event.key.toLowerCase() === "z") {
      event.preventDefault();
      if (event.shiftKey) redoEditor(); else undoEditor();
      return;
    }
    if (modifier && event.key.toLowerCase() === "y") {
      event.preventDefault();
      redoEditor();
      return;
    }
    if (!elements.completions.hidden) {
      if (event.key === "ArrowDown" || event.key === "ArrowUp") {
        event.preventDefault();
        const direction = event.key === "ArrowDown" ? 1 : -1;
        completionState.index = (completionState.index + direction + completionState.entries.length) % completionState.entries.length;
        renderCompletions();
        return;
      }
      if (event.key === "Enter" || event.key === "Tab") {
        event.preventDefault();
        acceptCompletion();
        return;
      }
      if (event.key === "Escape") hideCompletions();
    }
    if (modifier && event.key.toLowerCase() === "s") {
      event.preventDefault();
      saveFile();
    }
    if (modifier && event.key.toLowerCase() === "e") {
      event.preventDefault();
      startEditorPrefix("run");
      return;
    }
    if (modifier && event.key === "Enter") {
      event.preventDefault();
      evaluateForm();
    }
    if (elements.paredit.getAttribute("aria-pressed") === "true" && event.ctrlKey && !event.metaKey && !event.altKey &&
        event.key.toLowerCase() === "k" && killToFormEnd(elements.editor)) {
      event.preventDefault();
      return;
    }
    if (elements.paredit.getAttribute("aria-pressed") === "true" && event.ctrlKey && !event.metaKey && !event.altKey) {
      const structuralEdit = event.key === "ArrowRight" ? slurpForward : event.key === "ArrowLeft" ? barfForward : null;
      if (structuralEdit?.(elements.editor)) {
        event.preventDefault();
        return;
      }
    }
    if (elements.paredit.getAttribute("aria-pressed") === "true" &&
        !event.metaKey && !event.ctrlKey && !event.altKey &&
        applyParedit(elements.editor, event.key)) {
      event.preventDefault();
      return;
    }
    if (event.key === "Tab") {
      event.preventDefault();
      if (event.shiftKey) insertIndent(elements.editor, true);
      else structuralAlign(elements.editor);
    }
  });
  elements.editor.addEventListener("keyup", (event) => {
    if (event.key === "Control" && state.editorPrefix === "run") {
      clearEditorPrefix();
      evaluateForm();
    }
  });
  elements.paredit.addEventListener("click", () => {
    const enabled = elements.paredit.getAttribute("aria-pressed") !== "true";
    elements.paredit.setAttribute("aria-pressed", String(enabled));
    elements.paredit.textContent = enabled ? "PAREDIT ON" : "PAREDIT OFF";
    toast(enabled ? "PAREDIT ENABLED" : "PAREDIT DISABLED");
  });
  if (elements.diff) elements.diff.addEventListener("click", () => {
    const visible = elements.structuralDiff.hidden;
    elements.structuralDiff.hidden = !visible;
    elements.diff.setAttribute("aria-pressed", String(visible));
    if (visible) updateStructuralDiff();
  });
  elements.editor.addEventListener("blur", () => setTimeout(hideCompletions, 120));
  elements.save.addEventListener("click", () => saveFile());
  elements.run.addEventListener("click", evaluateFile);
}

installWorkspaceNavigation();
installLauncher();
installWorkspaceTabs();
installWorkspaceCreation();
installPublishing();
installHelp();
installWindowManager();
installEditor();
installBackgroundEditor();
installFileActions();
restoreWindows();
setWorkspace(0);
renderSavedWorkspaces().catch((error) => toast(`WORKSPACE INDEX FAILED: ${errorText(error)}`, true));
bootRuntime();
