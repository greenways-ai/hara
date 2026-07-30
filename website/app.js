import { keywordName, mapValue, renderScene, validateScene } from "./scene.js";
import { CreativeRuntime, normalizeCreative } from "./creative.js";
import { applyParedit, barfForward, insertIndent, killToFormEnd, localFormAt, slurpForward, structuralAlign } from "./editor.js";
import { WorkspaceRepository, kernelName, workspaceTemplates } from "./workspaces.js";
import { downloadWorkspace, GistPublisher, GreenwaysPublisher, workspaceBundle } from "./publishing.js";
import { GitHubAuthClient, authBaseFromDocument } from "./github-auth.js";
import { AiAdapterRepository, createAiCapability } from "./ai-adapters.js";
import { seedAmpWorkspace } from "./amp-workspace.js";

const SPACE = "home";
const ROOT = "ROOT";
const ACTIVE_FILE_KEY = "hara-www.active-file.v1";
const WINDOWS_KEY = "hara-www.windows.v1";
const BACKGROUND_WORKSPACE = "./examples/studio-backgrounds/";
const AMP_WORKSPACE = "./examples/hara-amp/";
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
  backgroundSource: null,
  backgroundDocuments: new Map(),
  activeBackground: null,
  canvasRuntime: null,
  sourceTimer: null,
  zIndex: 10,
  workspace: 0,
  workspaceRepository: new WorkspaceRepository(),
  githubAuth: new GitHubAuthClient({ baseUrl: authBaseFromDocument() }),
  githubSession: { authenticated: false, configured: false, profile: null },
  aiAdapters: new AiAdapterRepository(),
  capabilityRegistry: null,
  aiSessionWorkspaces: new Map(),
  workspaceRecords: new Map(),
  openWorkspaces: [],
  currentProject: null,
  activeKernel: ROOT,
  activeSpace: SPACE,
  kernelSpaces: new Map([[ROOT, SPACE]]),
  contextSpaces: new WeakMap(),
  defaultBootstrap: null,
  sessionRouter: null,
  runtimeStatus: "standby",
  runtimeStartedAt: null,
  runtimeModuleBytes: 0,
  telemetry: {
    kernelsCreated: 0,
    kernelsClosed: 0,
    evalRequests: 0,
    documentRuns: 0,
    deliveredMessages: 0,
    frameRequests: 0,
    renderCalls: 0,
    framesPerSecond: 0,
    frameRateWindowStartedAt: 0,
    frameRateWindowCount: 0,
    kernelBusyMs: 0,
    kernelBusyPercent: 0,
    kernelBusyWindowStartedAt: 0,
    errors: 0
  }
};

const elements = {
  launcher: query("[data-launcher]"),
  launcherToggle: query("[data-launcher-toggle]"),
  launcherScrim: query("[data-launcher-scrim]"),
  runtimeLed: query("[data-runtime-led]"),
  runtimeLabel: query("[data-runtime-label]"),
  runtimeToggle: query("[data-runtime-toggle]"),
  kernelStatistics: query("[data-kernel-statistics]"),
  kernelStatisticsGrid: query("[data-kernel-statistics-grid]"),
  kernelStatisticsState: query("[data-kernel-state]"),
  kernelStatisticsUpdated: query("[data-kernel-statistics-updated]"),
  backgroundPicker: query("[data-background-picker]"),
  backgroundSource: query("[data-background-source]"),
  backgroundMenu: query("[data-background-menu]"),
  backgroundMenuToggle: query("[data-background-menu-toggle]"),
  backgroundMenuLabel: query("[data-background-menu-label]"),
  sourceToggle: query("[data-source-toggle]"),
  sourcePanel: query("[data-background-panel]"),
  sourceEditor: query("[data-background-editor]"),
  sourceHighlight: query("[data-background-highlight]"),
  sourceLineNumbers: query("[data-background-line-numbers]"),
  sourceStatus: query("[data-background-status]"),
  sourceParedit: query("[data-background-paredit]"),
  sourceApply: query("[data-background-apply]"),
  sourceEval: query("[data-background-eval]"),
  sourceSave: query("[data-background-save]"),
  sourceTrace: query("[data-background-trace]"),
  sourceFontDecrease: query("[data-background-font-decrease]"),
  sourceFontIncrease: query("[data-background-font-increase]"),
  sourceHelp: query("[data-background-help]"),
  sourceHelpPanel: query("[data-background-help-panel]"),
  sourceClose: query("[data-background-close]"),
  sourceResizer: query("[data-background-resizer]"),
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
  closeActiveWorkspace: query("[data-close-active-workspace]"),
  clearWorkspaces: query("[data-clear-workspaces]"),
  publish: query("[data-publish]"),
  publishDialog: query("[data-publish-dialog]"),
  publishNote: query("[data-publish-note]"),
  kernelLoading: query("[data-kernel-loading]"),
  kernelProgress: query("[data-kernel-progress]"),
  kernelDetail: query("[data-kernel-detail]"),
  kernelMeter: query("[data-kernel-meter]"),
  accountDialog: query("[data-account-dialog]"),
  accountName: query("[data-account-name]"),
  accountStatus: query("[data-account-status]"),
  accountAvatar: query("[data-account-avatar]"),
  settingsDialog: query("[data-settings-dialog]"),
  aiDialog: query("[data-ai-dialog]"),
  adapterList: query("[data-adapter-list]"),
  adapterForm: query("[data-adapter-form]"),
  aiNote: query("[data-ai-note]"),
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
  state.runtimeStatus = status;
  elements.runtimeLabel.textContent = label;
  elements.runtimeLed.classList.toggle("is-live", status === "live");
  elements.runtimeLed.classList.toggle("is-error", status === "error");
  if (!elements.kernelStatistics.hidden) renderKernelStatistics();
}

function formatDuration(milliseconds) {
  if (!Number.isFinite(milliseconds) || milliseconds < 0) return "—";
  const seconds = Math.floor(milliseconds / 1000);
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor(seconds % 86400 / 3600);
  const minutes = Math.floor(seconds % 3600 / 60);
  const remainder = seconds % 60;
  return days ? `${days}d ${hours}h ${minutes}m` :
    hours ? `${hours}h ${minutes}m ${remainder}s` :
      `${minutes}m ${String(remainder).padStart(2, "0")}s`;
}

function formatBytes(bytes) {
  if (!Number.isFinite(bytes) || bytes <= 0) return "—";
  const units = ["B", "KB", "MB", "GB"];
  const exponent = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  return `${(bytes / 1024 ** exponent).toFixed(exponent ? 1 : 0)} ${units[exponent]}`;
}

function telemetryGroup(title, entries) {
  const section = document.createElement("section");
  section.className = "kernel-statistics-group";
  const heading = document.createElement("h3");
  heading.textContent = title;
  const list = document.createElement("dl");
  for (const [label, value] of entries) {
    const term = document.createElement("dt");
    const detail = document.createElement("dd");
    term.textContent = label;
    detail.textContent = String(value);
    list.append(term, detail);
  }
  section.append(heading, list);
  return section;
}

function renderKernelStatistics() {
  if (!state.broker) {
    elements.kernelStatisticsState.textContent = state.runtimeStatus.toUpperCase();
    elements.kernelStatisticsGrid.replaceChildren(telemetryGroup("RUNTIME", [
      ["State", state.runtimeStatus.toUpperCase()],
      ["Kernel", "NOT AVAILABLE"]
    ]));
    return;
  }
  const kernels = state.broker.list();
  const sessions = state.sessionRouter?.list() ?? [];
  const subscriptions = sessions.reduce((total, session) => total + session.subscriptions, 0);
  const capabilities = state.capabilityRegistry?.available() ?? [];
  const grants = sessions.reduce((total, session) =>
    total + (state.capabilityRegistry?.forSession(session.sessionId).length ?? 0), 0);
  const memory = performance.memory?.usedJSHeapSize;
  const activeWorkspace = state.currentProject?.name ?? (state.workspace ? "UNNAMED" : "HOME");
  elements.kernelStatisticsState.textContent = state.runtimeStatus.toUpperCase();
  elements.kernelStatisticsGrid.replaceChildren(
    telemetryGroup("RUNTIME", [
      ["State", state.runtimeStatus.toUpperCase()],
      ["Uptime", formatDuration(performance.now() - (state.runtimeStartedAt ?? performance.now()))],
      ["WASM module", formatBytes(state.runtimeModuleBytes)],
      ["JS heap", formatBytes(memory)],
      ["Page", document.visibilityState.toUpperCase()]
    ]),
    telemetryGroup("KERNELS", [
      ["Running", kernels.length],
      ["Pending", state.broker.pending?.size ?? 0],
      ["Documents", state.broker.documents?.size ?? 0],
      ["Created", state.telemetry.kernelsCreated],
      ["Stopped", state.telemetry.kernelsClosed],
      ["Active", state.activeKernel]
    ]),
    telemetryGroup("SESSIONS", [
      ["Registered", sessions.length],
      ["Subscriptions", subscriptions],
      ["Workspace", activeWorkspace],
      ["Open workspaces", state.openWorkspaces.length],
      ["Filesystem", state.activeSpace],
      ["Isolation", "DEDICATED KERNEL"]
    ]),
    telemetryGroup("TRAFFIC", [
      ["Eval requests", state.telemetry.evalRequests],
      ["Document runs", state.telemetry.documentRuns],
      ["Host calls", state.telemetry.frameRequests + state.telemetry.renderCalls],
      ["Frame requests", state.telemetry.frameRequests],
      ["Frames rendered", state.telemetry.renderCalls],
      ["Render rate", `${state.telemetry.framesPerSecond} FPS`],
      ["Kernel busy estimate", `${state.telemetry.kernelBusyPercent}%`],
      ["Session messages", state.telemetry.deliveredMessages],
      ["Errors", state.telemetry.errors],
      ["Queue", state.broker.pending?.size ?? 0]
    ]),
    telemetryGroup("CAPABILITIES", [
      ["Available", capabilities.length],
      ["Granted", grants],
      ["Adapters", capabilities.join(", ") || "NONE"],
      ["AI profiles", state.currentProject ? state.aiAdapters.list(state.currentProject.id).length : 0]
    ]),
    telemetryGroup("STORAGE", [
      ["Provider", "INDEXEDDB"],
      ["Scope", state.activeSpace],
      ["Workspace files", state.files.length],
      ["Persistence", "LOCAL"],
      ["Worker model", "ONE PER KERNEL"]
    ])
  );
  elements.kernelStatisticsUpdated.textContent =
    new Intl.DateTimeFormat(undefined, { hour: "2-digit", minute: "2-digit", second: "2-digit" }).format(new Date());
}

function instrumentBrokerTelemetry(broker) {
  for (const method of ["eval", "evalForm"]) {
    const original = broker[method].bind(broker);
    broker[method] = async (...args) => {
      state.telemetry.evalRequests += 1;
      const started = performance.now();
      try {
        return await original(...args);
      } catch (error) {
        state.telemetry.errors += 1;
        throw error;
      } finally {
        recordKernelBusy(started);
      }
    };
  }
  for (const method of ["prepareDocument"]) {
    const original = broker[method].bind(broker);
    broker[method] = async (...args) => {
      state.telemetry.documentRuns += 1;
      const started = performance.now();
      try {
        return await original(...args);
      } catch (error) {
        state.telemetry.errors += 1;
        throw error;
      } finally {
        recordKernelBusy(started);
      }
    };
  }
  return broker;
}

function recordKernelBusy(started) {
  const now = performance.now();
  if (!state.telemetry.kernelBusyWindowStartedAt) state.telemetry.kernelBusyWindowStartedAt = now;
  state.telemetry.kernelBusyMs += Math.max(0, now - started);
  const elapsed = now - state.telemetry.kernelBusyWindowStartedAt;
  if (elapsed < 1000) return;
  state.telemetry.kernelBusyPercent = Math.min(100, Math.round(state.telemetry.kernelBusyMs * 100 / elapsed));
  state.telemetry.kernelBusyMs = 0;
  state.telemetry.kernelBusyWindowStartedAt = now;
}

function instrumentCanvasTelemetry(canvasRuntime) {
  const nextFrame = canvasRuntime.nextFrame.bind(canvasRuntime);
  canvasRuntime.nextFrame = async (...args) => {
    state.telemetry.frameRequests += 1;
    try {
      return await nextFrame(...args);
    } catch (error) {
      state.telemetry.errors += 1;
      throw error;
    }
  };
  const render = canvasRuntime.render.bind(canvasRuntime);
  canvasRuntime.render = (...args) => {
    state.telemetry.renderCalls += 1;
    const now = performance.now();
    if (!state.telemetry.frameRateWindowStartedAt) state.telemetry.frameRateWindowStartedAt = now;
    state.telemetry.frameRateWindowCount += 1;
    const elapsed = now - state.telemetry.frameRateWindowStartedAt;
    if (elapsed >= 1000) {
      state.telemetry.framesPerSecond = Math.round(state.telemetry.frameRateWindowCount * 1000 / elapsed);
      state.telemetry.frameRateWindowStartedAt = now;
      state.telemetry.frameRateWindowCount = 0;
    }
    try {
      return render(...args);
    } catch (error) {
      state.telemetry.errors += 1;
      throw error;
    }
  };
  return canvasRuntime;
}

function installKernelStatistics() {
  let refreshTimer = null;
  const close = () => {
    elements.kernelStatistics.hidden = true;
    elements.runtimeToggle.setAttribute("aria-expanded", "false");
    clearInterval(refreshTimer);
    refreshTimer = null;
  };
  const open = () => {
    renderKernelStatistics();
    elements.kernelStatistics.hidden = false;
    elements.runtimeToggle.setAttribute("aria-expanded", "true");
    refreshTimer ??= setInterval(renderKernelStatistics, 1000);
  };
  elements.runtimeToggle.addEventListener("click", () => {
    if (elements.kernelStatistics.hidden) open();
    else close();
  });
  query("[data-kernel-statistics-close]").addEventListener("click", close);
  document.addEventListener("pointerdown", (event) => {
    if (elements.kernelStatistics.hidden ||
        elements.kernelStatistics.contains(event.target) ||
        elements.runtimeToggle.contains(event.target)) return;
    close();
  });
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && !elements.kernelStatistics.hidden) close();
  });
}

function setKernelProgress(percent, label = "KERNEL LOADING", detail = "") {
  const value = Math.max(0, Math.min(100, Math.round(percent)));
  elements.kernelLoading.hidden = false;
  elements.kernelLoading.firstElementChild.textContent = label;
  elements.kernelProgress.textContent = `${value}%`;
  elements.kernelDetail.textContent = detail;
  elements.kernelMeter.style.width = `${value}%`;
  document.body.dataset.kernel = value === 100 ? "live" : "loading";
}

function hideKernelProgress() {
  elements.kernelLoading.hidden = true;
}

function formatDownloadBytes(bytes) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

async function fetchRuntimeBytes(url, { start = 10, end = 45 } = {}) {
  const response = await fetch(url, { cache: "no-store" });
  if (!response.ok) throw new Error(`runtime fetch failed: ${response.status}`);
  const total = Number(response.headers.get("content-length")) || 0;
  if (!response.body) {
    const bytes = new Uint8Array(await response.arrayBuffer());
    setKernelProgress(end, "RUNTIME DOWNLOADED", formatDownloadBytes(bytes.byteLength));
    return bytes;
  }
  const reader = response.body.getReader();
  const chunks = [];
  let received = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    chunks.push(value);
    received += value.byteLength;
    const ratio = total ? Math.min(received / total, 1) : 0;
    const percent = total
      ? start + (end - start) * ratio
      : Math.min(end - 1, start + Math.log2(1 + received / 16384) * 3);
    const detail = total
      ? `${formatDownloadBytes(received)} / ${formatDownloadBytes(total)}`
      : `${formatDownloadBytes(received)} RECEIVED`;
    setKernelProgress(percent, "DOWNLOADING HARA.WASM", detail);
  }
  const bytes = new Uint8Array(received);
  let offset = 0;
  for (const chunk of chunks) {
    bytes.set(chunk, offset);
    offset += chunk.byteLength;
  }
  setKernelProgress(end, "RUNTIME DOWNLOADED", formatDownloadBytes(received));
  return bytes;
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
  elements.backgroundMenu.replaceChildren();
  const categories = new Map();
  const menuCategories = new Map();
  const categoryOrder = ["Basic", "Nature", "Shapes", "Simulation", "Controls"];
  const categoryRank = (category) => {
    const index = categoryOrder.indexOf(category);
    return index === -1 ? categoryOrder.length : index;
  };
  const documents = [...(mapValue(value, "workspace/documents") ?? [])]
    .sort((left, right) => categoryRank(String(mapValue(left, "document/category")))
      - categoryRank(String(mapValue(right, "document/category"))));
  for (const documentValue of documents) {
    if (keywordName(mapValue(documentValue, "document/role")) !== "studio/background") continue;
    const descriptor = {
      id: String(mapValue(documentValue, "document/id")),
      title: String(mapValue(documentValue, "document/title")),
      category: String(mapValue(documentValue, "document/category") ?? "Other"),
      path: String(mapValue(documentValue, "document/path")).replace("../../sources/", "./sources/"),
      node: String(mapValue(documentValue, "document/node")),
      canvas: String(mapValue(documentValue, "document/canvas"))
    };
    state.backgroundDocuments.set(descriptor.id, descriptor);
    const option = document.createElement("option");
    option.value = descriptor.id;
    option.textContent = descriptor.title.toUpperCase();
    let category = categories.get(descriptor.category);
    if (!category) {
      category = document.createElement("optgroup");
      category.label = descriptor.category.toUpperCase();
      categories.set(descriptor.category, category);
      elements.backgroundSource.append(category);
    }
    category.append(option);
    let menuCategory = menuCategories.get(descriptor.category);
    if (!menuCategory) {
      menuCategory = document.createElement("section");
      menuCategory.className = "background-menu-group";
      const heading = document.createElement("h3");
      heading.textContent = descriptor.category.toUpperCase();
      menuCategory.append(heading);
      menuCategories.set(descriptor.category, menuCategory);
      elements.backgroundMenu.append(menuCategory);
    }
    const menuItem = document.createElement("button");
    menuItem.type = "button";
    menuItem.role = "menuitemradio";
    menuItem.dataset.backgroundMenuItem = descriptor.id;
    menuItem.textContent = descriptor.title.toUpperCase();
    menuItem.setAttribute("aria-checked", "false");
    menuCategory.append(menuItem);
  }
  const startupPool = [...state.backgroundDocuments.values()]
    .filter(({ category }) => ["Basic", "Nature", "Shapes"].includes(category));
  const selected = startupPool[Math.floor(Math.random() * startupPool.length)]
    ?? state.backgroundDocuments.values().next().value;
  state.backgroundSource = selected.id;
  syncBackgroundPicker();
}

function syncBackgroundPicker() {
  const descriptor = state.backgroundDocuments.get(state.backgroundSource);
  if (!descriptor) return;
  elements.backgroundSource.value = descriptor.id;
  elements.backgroundMenuLabel.textContent = descriptor.title.toUpperCase();
  queryAll("[data-background-menu-item]", elements.backgroundMenu).forEach((item) => {
    const active = item.dataset.backgroundMenuItem === descriptor.id;
    item.classList.toggle("is-active", active);
    item.setAttribute("aria-checked", String(active));
  });
}

function closeBackgroundMenu() {
  elements.backgroundMenu.hidden = true;
  elements.backgroundMenuToggle.setAttribute("aria-expanded", "false");
}

function sourceStorageKey(documentId, kind) {
  return `hara-www.background.${kind}.v2:${documentId}`;
}

async function fetchBackgroundSource(descriptor) {
  const response = await fetch(new URL(descriptor.path, import.meta.url), { cache: "no-store" });
  if (!response.ok) throw new Error(`background source fetch failed: ${response.status}`);
  const bundled = await response.text();
  const saved = localStorage.getItem(sourceStorageKey(descriptor.id, "saved"));
  const recovery = localStorage.getItem(sourceStorageKey(descriptor.id, "recovery"));
  const base = localStorage.getItem(sourceStorageKey(descriptor.id, "base"));
  const overlayMatchesBundle = base === bundled;
  const overlay = overlayMatchesBundle ? recovery ?? saved : null;
  return {
    bundled,
    source: overlay ?? bundled,
    recovered: overlayMatchesBundle && recovery !== null
  };
}

async function activateBackground(descriptor, source, generation) {
  const nodeId = `${descriptor.node}@${generation}`;
  const prepared = await state.broker.prepareDocument(ROOT, descriptor.id, source, { nodeId });
  if (elements.sourceTrace.getAttribute("aria-pressed") === "true") {
    try {
      await prepared.context.call("trace-namespace-enable", [prepared.moduleId, "transitive"]);
    } catch (error) {
      if (!errorText(error).includes("hta/target-unknown")) throw error;
      elements.sourceTrace.setAttribute("aria-pressed", "false");
      elements.sourceTrace.textContent = "TRACE UNAVAILABLE";
      elements.sourceTrace.disabled = true;
      localStorage.removeItem("hara-www.trace-enabled.v1");
      toast("TRACE RUNTIME IS NOT INCLUDED IN THIS BUILD", true);
    }
  }
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
    taskSettled.catch((error) => {
      if (state.activeBackground?.nodeId !== nodeId) return;
      const message = errorText(error);
      state.telemetry.errors += 1;
      state.telemetry.framesPerSecond = 0;
      elements.sourceStatus.textContent = `ERROR // ${message}`;
      toast(`BACKGROUND SOURCE FAILED: ${message}`, true);
    });
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
  syncBackgroundPicker();
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
    document.body.dataset.backgroundName = descriptor.title.toLowerCase().replaceAll(" ", "-");
    elements.sourceEditor.value = source;
    elements.sourceEditor.scrollTop = 0;
    elements.sourceEditor.scrollLeft = 0;
    elements.sourceEditor.dataset.baseSource = loaded.bundled;
    elements.sourceEditor.dataset.documentId = descriptor.id;
    elements.sourceStatus.textContent =
      `${loaded.recovered ? "RECOVERED" : "LIVE"} // GENERATION ${generation}`;
    syncBackgroundHighlight();
  } catch (error) {
    if (generation !== backgroundLoadGeneration) return;
    elements.sourceStatus.textContent = `ERROR // ${errorText(error)}`;
    toast(`BACKGROUND SOURCE FAILED: ${errorText(error)}`, true);
    throw error;
  }
}

function setWorkspace(index, { reloadBackground = true } = {}) {
  if (index === 1 && !document.body.classList.contains("is-start-ready")) return;
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
    showWorkspaceWindows();
  } else {
    state.canvasRuntime?.setVisible(true);
    query("[data-tron]").hidden = false;
    if (state.broker && reloadBackground) loadBackgroundSource(state.backgroundSource).catch(() => {});
  }
  closeLauncher();
}

const workspacePresentation = {
  blank: { files: "EXPLORER", editor: "SOURCE", canvas: "OUTPUT", tabs: ["explorer", "source", "output"] },
  canvas: { files: "EXPLORER", editor: "SOURCE", canvas: "CANVAS", tabs: ["explorer", "source", "canvas"] },
  music: { files: "PLAYLIST", editor: "PLAYER / SOURCE", canvas: "SPECTRUM", tabs: ["playlist", "source", "spectrum"] },
  "hara-amp": { files: "SIGNAL GRAPH", editor: "HAL / SOURCE", canvas: "SPECTRUM", tabs: ["graph", "source", "spectrum"] },
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
  elements.closeActiveWorkspace.hidden = !state.currentProject;
}

async function renderSavedWorkspaces() {
  const records = await state.workspaceRepository.list();
  state.workspaceRecords = new Map(records.map((record) => [record.id, record]));
  elements.clearWorkspaces.hidden = records.length === 0;
  elements.savedWorkspaces.replaceChildren();
  if (!records.length) {
    const empty = document.createElement("span");
    empty.className = "launcher-empty";
    empty.textContent = "NO SAVED WORKSPACES";
    elements.savedWorkspaces.append(empty);
  }
  for (const record of records) {
    const row = document.createElement("div");
    row.className = "saved-workspace-row";
    row.dataset.savedWorkspaceId = record.id;
    const open = document.createElement("button");
    open.type = "button";
    open.className = "saved-workspace";
    open.textContent = `${record.name.toUpperCase()}  //  ${record.template.toUpperCase()}`;
    open.addEventListener("click", () => openWorkspace(record).catch((error) => toast(errorText(error), true)));
    const remove = document.createElement("button");
    remove.type = "button";
    remove.className = "saved-workspace-delete";
    remove.setAttribute("aria-label", `Delete ${record.name}`);
    remove.textContent = "DELETE";
    remove.addEventListener("click", () => {
      deleteSavedWorkspace(record).catch((error) => toast(errorText(error), true));
    });
    row.append(open, remove);
    elements.savedWorkspaces.append(row);
  }
  renderProjectTabs();
}

async function syncProjectIntoKernel(record) {
  const files = await state.workspaceRepository.files(record.id);
  for (const [path, content] of files) {
    await writeStudioText(path, content, kernelName(record.id));
  }
}

async function stopWorkspaceKernel(record = state.currentProject) {
  if (!record || !state.broker) return;
  const name = kernelName(record.id);
  if (state.activeDocument && state.activeKernel === name) {
    state.broker.releaseDocument(name, state.activeDocument.id);
    state.activeDocument = null;
  }
  if (state.broker.list().includes(name)) await state.broker.close(name);
  document.body.dataset.kernel = "stopped";
}

async function transitionHome() {
  if (state.workspace === 1) {
    setKernelProgress(10, "STOPPING KERNEL");
    try {
      await stopWorkspaceKernel();
    } finally {
      hideKernelProgress();
    }
  }
  state.activeKernel = ROOT;
  state.activeSpace = SPACE;
  state.activeFile = null;
  state.dirty = false;
  elements.publish.disabled = true;
  setWorkspace(0);
}

async function openWorkspace(record) {
  if (!state.broker || !state.defaultBootstrap) {
    toast("RUNTIME STILL BOOTING", true);
    return;
  }
  const name = kernelName(record.id);
  const space = `workspace-${record.id}`;
  if (state.workspace === 1 && state.currentProject?.id === record.id && state.broker.list().includes(name)) return;
  setKernelProgress(5);
  if (state.workspace === 1 && state.currentProject?.id !== record.id) {
    setKernelProgress(12, "STOPPING PREVIOUS KERNEL");
    await stopWorkspaceKernel(state.currentProject);
  }
  if (!state.openWorkspaces.includes(record.id)) state.openWorkspaces.push(record.id);
  state.workspaceRecords.set(record.id, record);
  state.kernelSpaces.set(name, space);
  try {
    setKernelProgress(24);
    if (state.broker.list().includes(name)) await state.broker.close(name);
    await state.broker.create(name, { bootstrap: state.defaultBootstrap(space) });
    setKernelProgress(52);
    await syncProjectIntoKernel(record);
    setKernelProgress(72);
    state.currentProject = record;
    state.activeKernel = name;
    state.activeSpace = space;
    state.activeFile = null;
    state.dirty = false;
    applyWorkspacePresentation(record);
    setWorkspace(1);
    renderProjectTabs();
    const files = await listFiles();
    setKernelProgress(88);
    const path = files.includes("/src/main.hal") ? "/src/main.hal" :
      files.includes("/workspace.edn") ? "/workspace.edn" : files[0];
    if (path) await openFile(path, true);
    elements.publish.disabled = false;
    setKernelProgress(100);
    setTimeout(hideKernelProgress, 280);
  } catch (error) {
    document.body.dataset.kernel = "error";
    hideKernelProgress();
    throw error;
  }
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
      await transitionHome();
    }
  }
  renderProjectTabs();
}

async function deleteSavedWorkspace(record) {
  if (!confirm(`Delete "${record.name}" and all of its files?`)) return;
  await closeWorkspace(record.id);
  await state.workspaceRepository.delete(record.id);
  state.workspaceRecords.delete(record.id);
  await renderSavedWorkspaces();
  toast(`${record.name.toUpperCase()} DELETED`);
}

async function clearSavedWorkspaces() {
  const records = await state.workspaceRepository.list();
  if (!records.length) return;
  if (!confirm(`Delete all ${records.length} saved workspaces and their files?`)) return;
  if (state.currentProject) await stopWorkspaceKernel(state.currentProject);
  state.openWorkspaces = [];
  state.workspaceRecords.clear();
  state.currentProject = null;
  state.activeKernel = ROOT;
  state.activeSpace = SPACE;
  state.activeDocument = null;
  await state.workspaceRepository.clear();
  elements.publish.disabled = true;
  setWorkspace(0);
  await renderSavedWorkspaces();
  toast("ALL SAVED WORKSPACES CLEARED");
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
    elements.backgroundSource.addEventListener("change", () => {
      loadBackgroundSource(elements.backgroundSource.value).catch(() => {});
    });
    elements.backgroundMenuToggle.addEventListener("click", () => {
      const open = elements.backgroundMenu.hidden;
      elements.backgroundMenu.hidden = !open;
      elements.backgroundMenuToggle.setAttribute("aria-expanded", String(open));
    });
    elements.backgroundMenu.addEventListener("click", (event) => {
      const item = event.target.closest("[data-background-menu-item]");
      if (!item) return;
      closeBackgroundMenu();
      loadBackgroundSource(item.dataset.backgroundMenuItem).catch(() => {});
    });
    document.addEventListener("pointerdown", (event) => {
      if (elements.backgroundMenu.hidden || event.target.closest(".background-picker")) return;
      closeBackgroundMenu();
    });
    document.addEventListener("keydown", (event) => {
      if (event.key === "Escape" && !elements.backgroundMenu.hidden) closeBackgroundMenu();
    });
  }
  const startButton = query("[data-start]");
  const previousButton = query("[data-workspace-prev]");
  const nextButton = query("[data-workspace-next]");
  previousButton.disabled = true;
  nextButton.disabled = true;
  startButton.disabled = nextButton.disabled;
  startButton.addEventListener("click", () => {
    if (startButton.disabled) return;
    setWorkspace(1);
  });
  queryAll("[data-home]").forEach((button) => button.addEventListener("click", () => {
    transitionHome().catch((error) => toast(errorText(error), true));
  }));

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
      if (dx > 0) transitionHome().catch((error) => toast(errorText(error), true));
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
  query("[data-new-workspace]").addEventListener("click", () => openTemplateDialog());
  for (const button of queryAll("[data-quick-template]")) {
    button.addEventListener("click", () => openTemplateDialog(button.dataset.quickTemplate));
  }
  elements.closeActiveWorkspace.addEventListener("click", async () => {
    if (!state.currentProject) return;
    try {
      await closeWorkspace(state.currentProject.id);
      closeLauncher();
    } catch (error) {
      toast(errorText(error), true);
    }
  });
  elements.clearWorkspaces.addEventListener("click", () => {
    clearSavedWorkspaces().catch((error) => toast(errorText(error), true));
  });
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && elements.launcher.classList.contains("is-open")) {
      closeLauncher();
      elements.launcherToggle.focus();
    }
  });
}

function openTemplateDialog(preferredTemplate = "blank") {
  closeLauncher();
  const definition = workspaceTemplates.find((template) => template.id === preferredTemplate);
  elements.workspaceName.value =
    preferredTemplate === "blank" ? "Untitled Workspace" : `Untitled ${definition?.label ?? "Workspace"}`;
  elements.templateDialog.showModal();
  requestAnimationFrame(() => {
    elements.workspaceName.select();
    query(`[data-template="${preferredTemplate}"]`, elements.templateGrid)?.focus();
  });
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
  document.addEventListener("hara:create-amp-workspace", (event) => {
    createAmpWorkspace(event.detail).catch((error) => {
      document.dispatchEvent(new CustomEvent("hara:amp-workspace-error", {
        detail: { message: errorText(error) }
      }));
    });
  });
}

async function createAmpWorkspace({ preset = "hara", mode = "spectrum", source = "" } = {}) {
  const load = async (path) => {
    const response = await fetch(new URL(`${AMP_WORKSPACE}${path}`, import.meta.url));
    if (!response.ok) throw new Error(`Hara Amp ${path}: ${response.status}`);
    return response.text();
  };
  const [project, workspace, visualizer] = await Promise.all([
    load("project.edn"),
    load("workspace.edn"),
    load("src/visualizer.hal")
  ]);
  const files = seedAmpWorkspace({
    project,
    workspace,
    visualizer: typeof source === "string" && source.trim() ? source : visualizer,
    preset,
    mode
  });
  const record = await state.workspaceRepository.createFromFiles({
    name: "Hara Amp",
    template: "hara-amp",
    files
  });
  await renderSavedWorkspaces();
  await openWorkspace(record);
  document.dispatchEvent(new CustomEvent("hara:amp-workspace-created", {
    detail: { id: record.id }
  }));
}

function installPublishing() {
  elements.publish.addEventListener("click", () => elements.publishDialog.showModal());
  query("[data-publish-close]").addEventListener("click", () => elements.publishDialog.close());
  for (const button of queryAll("[data-publish-provider]")) {
    button.addEventListener("click", async () => {
      if (!state.currentProject) return;
      const provider = button.dataset.publishProvider;
      const publicVisibility = query("[data-publish-public]").checked;
      if (provider === "download") {
        downloadWorkspace(await workspaceBundle(state.workspaceRepository, state.currentProject.id));
        elements.publishNote.textContent = "WORKSPACE EXPORTED FOR SELF-HOSTING.";
        return;
      }
      if (!state.githubSession.authenticated) {
        elements.publishNote.textContent = "CONNECT GITHUB BEFORE PUBLISHING.";
        renderGitHubAccount();
        elements.accountDialog.showModal();
        return;
      }
      button.disabled = true;
      try {
        const bundle = await workspaceBundle(state.workspaceRepository, state.currentProject.id);
        const previous = state.currentProject.providers?.[provider] ?? null;
        const publisher = provider === "gist"
          ? new GistPublisher({ request: (path, options) => state.githubAuth.request(`/github${path}`, options) })
          : new GreenwaysPublisher({ request: (path, options) => state.githubAuth.request(`/greenways${path}`, options) });
        const result = await publisher.publish(bundle, { public: publicVisibility, previous });
        const metadata = { id: result.id, url: result.html_url ?? result.url };
        await state.workspaceRepository.setProvider(state.currentProject.id, provider, metadata);
        state.currentProject = { ...state.currentProject, providers: { ...state.currentProject.providers, [provider]: metadata } };
        elements.publishNote.textContent = `PUBLISHED // ${metadata.url}`;
      } catch (error) {
        elements.publishNote.textContent = `PUBLISH FAILED // ${errorText(error)}`;
      } finally {
        button.disabled = false;
      }
    });
  }
}

function renderGitHubAccount() {
  const { authenticated, configured, profile } = state.githubSession;
  query("[data-github-label]").textContent = authenticated ? `@${profile.login}` : "Connect GitHub";
  elements.accountName.textContent = authenticated ? `@${profile.login}` : "NOT CONNECTED";
  elements.accountStatus.textContent = authenticated
    ? "GitHub is connected for Gist publishing and your Greenways profile."
    : configured
      ? "Connect GitHub to publish work and create your Greenways profile."
      : "The secure GitHub auth service needs a GitHub App client ID and secret before sign-in can go live.";
  elements.accountAvatar.hidden = !authenticated || !profile.avatarUrl;
  if (profile?.avatarUrl) elements.accountAvatar.src = profile.avatarUrl;
  query("[data-account-signin]").hidden = authenticated;
  query("[data-account-signin]").disabled = !configured;
  query("[data-account-signout]").hidden = !authenticated;
}

async function refreshGitHubSession() {
  try {
    state.githubSession = await state.githubAuth.session();
  } catch (error) {
    state.githubSession = { authenticated: false, configured: false, profile: null };
    elements.accountStatus.textContent = `AUTH SERVICE ERROR // ${errorText(error)}`;
  }
  renderGitHubAccount();
}

function installGitHubAccount() {
  query("[data-github-account]").addEventListener("click", () => {
    elements.settingsDialog.close();
    renderGitHubAccount();
    elements.accountDialog.showModal();
  });
  query("[data-account-close]").addEventListener("click", () => elements.accountDialog.close());
  query("[data-account-signin]").addEventListener("click", () => {
    try { state.githubAuth.signIn(); } catch (error) { elements.accountStatus.textContent = errorText(error); }
  });
  query("[data-account-signout]").addEventListener("click", async () => {
    await state.githubAuth.signOut();
    await refreshGitHubSession();
  });
  refreshGitHubSession();
}

function workspaceIdForSession(sessionId) {
  const name = String(sessionId).replace(/^DOC\./, "");
  return [...state.workspaceRecords.keys()].find((id) => name.startsWith(kernelName(id))) ?? null;
}

function renderAiAdapters() {
  elements.adapterList.replaceChildren();
  const workspaceId = state.currentProject?.id;
  const adapters = workspaceId ? state.aiAdapters.list(workspaceId) : [];
  if (!adapters.length) {
    const empty = document.createElement("div");
    empty.className = "adapter-empty";
    empty.textContent = workspaceId ? "NO ADAPTERS CONNECTED" : "OPEN A WORKSPACE FIRST";
    elements.adapterList.append(empty);
  }
  for (const adapter of adapters) {
    const row = document.createElement("div");
    row.className = "adapter-row";
    const summary = document.createElement("span");
    const name = document.createElement("strong");
    name.textContent = adapter.name;
    summary.append(name, document.createElement("br"), `${adapter.kind} // ${adapter.model}`);
    row.append(summary);
    const remove = document.createElement("button");
    remove.type = "button";
    remove.textContent = "REMOVE";
    remove.addEventListener("click", () => {
      state.aiAdapters.remove(adapter.id);
      renderAiAdapters();
    });
    row.append(remove);
    elements.adapterList.append(row);
  }
  elements.aiNote.textContent = workspaceId
    ? "API KEYS LAST FOR THIS TAB ONLY. THE ADAPTER DEFINITION STAYS WITH THIS BROWSER."
    : "SELECT A WORKSPACE BEFORE ADDING AN ADAPTER.";
}

function adapterInput() {
  const data = new FormData(elements.adapterForm);
  return Object.fromEntries(data.entries());
}

function installAiAdapters() {
  query("[data-ai-adapters]").addEventListener("click", () => {
    elements.settingsDialog.close();
    renderAiAdapters();
    elements.aiDialog.showModal();
  });
  query("[data-ai-close]").addEventListener("click", () => elements.aiDialog.close());
  elements.adapterForm.addEventListener("submit", (event) => {
    event.preventDefault();
    if (!state.currentProject) return;
    try {
      state.aiAdapters.save({ ...adapterInput(), workspaceId: state.currentProject.id });
      state.capabilityRegistry?.grant(state.activeKernel, ["ai/chat"]);
      elements.adapterForm.reset();
      renderAiAdapters();
      toast("AI ADAPTER CONNECTED");
    } catch (error) {
      elements.aiNote.textContent = `ADAPTER ERROR // ${errorText(error)}`;
    }
  });
  query("[data-ai-test]").addEventListener("click", async () => {
    if (!state.currentProject) return;
    try {
      const adapter = state.aiAdapters.save({ ...adapterInput(), workspaceId: state.currentProject.id });
      const result = await state.aiAdapters.chat(state.currentProject.id, adapter.id, [
        { role: "user", content: "Reply with OK." }
      ]);
      elements.aiNote.textContent = `ADAPTER LIVE // ${result.text || "OK"}`;
      renderAiAdapters();
    } catch (error) {
      elements.aiNote.textContent = `TEST FAILED // ${errorText(error)}`;
    }
  });
}

function installSettings() {
  query("[data-settings]").addEventListener("click", () => elements.settingsDialog.showModal());
  query("[data-settings-close]").addEventListener("click", () => elements.settingsDialog.close());
}

function installWorkspaceTabs() {
  for (const tab of queryAll("[data-focus-window]")) {
    tab.addEventListener("click", () => openWindow(tab.dataset.focusWindow));
  }
}

function studioSource(form) {
  return `(do (require [std.foundation.file :as file]) ${form})`;
}

function evalStudio(form, kernel = state.activeKernel) {
  return state.broker.eval(kernel, studioSource(form));
}

function readStudioText(path, kernel = state.activeKernel) {
  return evalStudio(`(str/decode-utf8 (deref (file/read ${JSON.stringify(path)})))`, kernel);
}

async function writeStudioText(path, content, kernel = state.activeKernel) {
  const parent = path.slice(0, path.lastIndexOf("/")) || "/";
  if (parent !== "/") {
    await evalStudio(`(deref (file/mkdir ${JSON.stringify(parent)}))`, kernel);
  }
  return evalStudio(
    `(deref (file/write ${JSON.stringify(path)} (str/encode-utf8 ${JSON.stringify(content)})))`,
    kernel
  );
}

function deleteStudioPath(path, kernel = state.activeKernel) {
  return evalStudio(`(deref (file/delete ${JSON.stringify(path)}))`, kernel);
}

async function listStudioFiles(path = "/", kernel = state.activeKernel) {
  const children = await evalStudio(`(deref (file/list ${JSON.stringify(path)}))`, kernel);
  const files = [];
  for (const child of children ?? []) {
    try {
      files.push(...await listStudioFiles(String(child), kernel));
    } catch {
      files.push(String(child));
    }
  }
  return files;
}

async function listFiles() {
  const result = await listStudioFiles();
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
  const content = await readStudioText(path);
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
  await writeStudioText(state.activeFile, elements.editor.value);
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
  elements.sourceHighlight.innerHTML =
    `<code class="code-highlight-content">${highlightHara(elements.sourceEditor.value)}</code>`;
  const content = elements.sourceHighlight.firstElementChild;
  if (content) {
    content.style.transform =
      `translate(${-elements.sourceEditor.scrollLeft}px, ${-elements.sourceEditor.scrollTop}px)`;
  }
  const base = elements.sourceEditor.dataset.baseSource ?? "";
  const changed = changedLineNumbers(base, elements.sourceEditor.value);
  elements.sourceLineNumbers.innerHTML = elements.sourceEditor.value.split("\n").map((_, index) =>
    `<span class="${changed.has(index) ? "is-changed" : ""}">${index + 1}</span>`
  ).join("\n");
  elements.sourceLineNumbers.scrollTop = elements.sourceEditor.scrollTop;
}

function setSourcePanel(open) {
  elements.sourcePanel.classList.toggle("is-open", open);
  elements.sourcePanel.setAttribute("aria-hidden", String(!open));
  elements.sourceToggle.setAttribute("aria-expanded", String(open));
  document.body.classList.toggle("source-open", open);
}

const BACKGROUND_EDITOR_WIDTH_KEY = "hara-www.editor-width.v1";
const BACKGROUND_EDITOR_FONT_KEY = "hara-www.editor-font-size.v1";

function setBackgroundEditorWidth(width) {
  const minimum = 420;
  const maximum = Math.max(minimum, window.innerWidth - 80);
  const next = Math.round(Math.min(maximum, Math.max(minimum, width)));
  elements.sourcePanel.style.setProperty("--background-editor-width", `${next}px`);
  localStorage.setItem(BACKGROUND_EDITOR_WIDTH_KEY, String(next));
}

function setBackgroundEditorFontSize(size) {
  const next = Math.round(Math.min(22, Math.max(10, size)));
  elements.sourcePanel.style.setProperty("--background-editor-font-size", `${next}px`);
  localStorage.setItem(BACKGROUND_EDITOR_FONT_KEY, String(next));
}

function installBackgroundEditorSizing() {
  const savedWidth = Number(localStorage.getItem(BACKGROUND_EDITOR_WIDTH_KEY));
  const savedFont = Number(localStorage.getItem(BACKGROUND_EDITOR_FONT_KEY));
  if (Number.isFinite(savedWidth) && savedWidth > 0) setBackgroundEditorWidth(savedWidth);
  if (Number.isFinite(savedFont) && savedFont > 0) setBackgroundEditorFontSize(savedFont);

  elements.sourceFontDecrease.addEventListener("click", () => {
    setBackgroundEditorFontSize(parseFloat(getComputedStyle(elements.sourceEditor).fontSize) - 1);
  });
  elements.sourceFontIncrease.addEventListener("click", () => {
    setBackgroundEditorFontSize(parseFloat(getComputedStyle(elements.sourceEditor).fontSize) + 1);
  });

  elements.sourceResizer.addEventListener("pointerdown", (event) => {
    if (matchMedia("(max-width: 700px)").matches) return;
    event.preventDefault();
    elements.sourceResizer.setPointerCapture(event.pointerId);
    elements.sourceResizer.classList.add("is-dragging");
  });
  elements.sourceResizer.addEventListener("pointermove", (event) => {
    if (!elements.sourceResizer.hasPointerCapture(event.pointerId)) return;
    setBackgroundEditorWidth(window.innerWidth - event.clientX);
  });
  const finishResize = (event) => {
    if (elements.sourceResizer.hasPointerCapture(event.pointerId)) {
      elements.sourceResizer.releasePointerCapture(event.pointerId);
    }
    elements.sourceResizer.classList.remove("is-dragging");
  };
  elements.sourceResizer.addEventListener("pointerup", finishResize);
  elements.sourceResizer.addEventListener("pointercancel", finishResize);
}

function scheduleBackgroundPreview() {
  clearTimeout(state.sourceTimer);
  const documentId = elements.sourceEditor.dataset.documentId;
  if (!documentId) return;
  localStorage.setItem(
    sourceStorageKey(documentId, "base"),
    elements.sourceEditor.dataset.baseSource ?? ""
  );
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
  localStorage.setItem(sourceStorageKey(descriptor.id, "base"), currentSource);
  localStorage.setItem(sourceStorageKey(descriptor.id, "saved"), elements.sourceEditor.value);
  localStorage.removeItem(sourceStorageKey(descriptor.id, "recovery"));
  elements.sourceStatus.textContent = "SAVED // INDEXEDDB OVERLAY";
  toast(`SAVED ${descriptor.title.toUpperCase()}.HAL LOCALLY`);
}

async function evaluateBackgroundForm() {
  const documentId = elements.sourceEditor.dataset.documentId;
  if (!documentId || !state.activeBackground) return;
  const start = elements.sourceEditor.selectionStart;
  const end = elements.sourceEditor.selectionEnd;
  const selected = elements.sourceEditor.value.slice(start, end).trim();
  const form = selected ? { source: selected, start, end } :
    localFormAt(elements.sourceEditor.value, start);
  if (!form?.source) return;
  elements.sourceStatus.textContent = "EVALUATING IN BACKGROUND ENVIRONMENT";
  try {
    const result = await state.broker.evalForm(ROOT, documentId, form.source);
    elements.sourceStatus.textContent = `RESULT // ${resultLabel(result)}`;
    toast(`EVAL => ${resultLabel(result)}`);
  } catch (error) {
    const message = errorText(error);
    elements.sourceStatus.textContent = `EVAL ERROR // ${message}`;
    toast(`EVAL ERROR: ${message}`, true);
  }
}

function installBackgroundEditor() {
  installBackgroundEditorSizing();
  elements.sourceParedit.setAttribute("aria-pressed", "true");
  elements.sourceToggle.addEventListener("click", () =>
    setSourcePanel(!elements.sourcePanel.classList.contains("is-open")));
  elements.sourceEditor.addEventListener("input", () => {
    syncBackgroundHighlight();
    scheduleBackgroundPreview();
  });
  elements.sourceEditor.addEventListener("scroll", syncBackgroundHighlight);
  elements.sourceEditor.addEventListener("keydown", (event) => {
    const modifier = event.ctrlKey || event.metaKey;
    if (modifier && event.key.toLowerCase() === "s") {
      event.preventDefault();
      void saveBackgroundSource();
      return;
    }
    if (modifier && event.key === "Enter") {
      event.preventDefault();
      clearTimeout(state.sourceTimer);
      void loadBackgroundSource(
        elements.sourceEditor.dataset.documentId,
        elements.sourceEditor.value
      );
      return;
    }
    if (modifier && event.key.toLowerCase() === "e") {
      event.preventDefault();
      void evaluateBackgroundForm();
      return;
    }
    if (event.key === "Tab") {
      event.preventDefault();
      if (event.shiftKey) insertIndent(elements.sourceEditor, true);
      else structuralAlign(elements.sourceEditor);
      elements.sourceEditor.dispatchEvent(new Event("input", { bubbles: true }));
      return;
    }
    if (elements.sourceParedit.getAttribute("aria-pressed") === "true" &&
        !event.metaKey && !event.ctrlKey && !event.altKey &&
        applyParedit(elements.sourceEditor, event.key)) {
      event.preventDefault();
      elements.sourceEditor.dispatchEvent(new Event("input", { bubbles: true }));
    }
  });
  elements.sourceApply.addEventListener("click", () => {
    clearTimeout(state.sourceTimer);
    void loadBackgroundSource(
      elements.sourceEditor.dataset.documentId,
      elements.sourceEditor.value
    );
  });
  elements.sourceEval.addEventListener("click", () => void evaluateBackgroundForm());
  elements.sourceSave.addEventListener("click", () => void saveBackgroundSource());
  elements.sourceTrace.addEventListener("click", () => {
    const enabled = elements.sourceTrace.getAttribute("aria-pressed") !== "true";
    elements.sourceTrace.setAttribute("aria-pressed", String(enabled));
    elements.sourceTrace.textContent = enabled ? "TRACE ON" : "TRACE OFF";
    localStorage.setItem("hara-www.trace-enabled.v1", String(enabled));
    toast(enabled ? "TRACING ENABLED FOR NEXT APPLY" : "TRACING DISABLED");
  });
  if (localStorage.getItem("hara-www.trace-enabled.v1") === "true") {
    elements.sourceTrace.setAttribute("aria-pressed", "true");
    elements.sourceTrace.textContent = "TRACE ON";
  }
  elements.sourceHelp.addEventListener("click", () => {
    const open = elements.sourceHelp.getAttribute("aria-expanded") !== "true";
    elements.sourceHelp.setAttribute("aria-expanded", String(open));
    elements.sourceHelpPanel.hidden = !open;
  });
  elements.sourceClose.addEventListener("click", () => {
    elements.sourceHelp.setAttribute("aria-expanded", "false");
    elements.sourceHelpPanel.hidden = true;
    setSourcePanel(false);
    elements.sourceToggle.focus();
  });
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
      await deleteStudioPath(path);
    }
  }
  for (const [path, content] of DEFAULT_FILES) {
    await writeStudioText(path, content);
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
    await writeStudioText(path, content);
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
    await writeStudioText(nextPath, elements.editor.value);
    if (state.currentProject) {
      await state.workspaceRepository.writeFile(state.currentProject.id, nextPath, elements.editor.value);
    }
    if (nextPath !== oldPath) {
      await deleteStudioPath(oldPath);
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
    await deleteStudioPath(path);
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
  state.runtimeStartedAt = performance.now();
  setRuntimeStatus("WASM // BOOTING", "booting");
  elements.editorStatus.textContent = "BOOTING HARA.WASM";
  setKernelProgress(2, "PREPARING RUNTIME", "LOADING BROWSER MODULES");
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
    setKernelProgress(10, "MODULES READY", "STARTING RUNTIME DOWNLOAD");
    const moduleBytes = await fetchRuntimeBytes(new URL("hara.wasm", runtimeBase));
    state.runtimeModuleBytes = moduleBytes.byteLength;
    const resources = {};
    const studioResources = ["store", "boot", "node", "draw", "program", "graph", "session"];
    const standardResources = [
      "lib/substrate/protocol",
      "lib/substrate/frame",
      "lib/substrate"
    ];
    const resourceCount = studioResources.length + standardResources.length;
    let resourceIndex = 0;
    for (const name of studioResources) {
      const response = await fetch(new URL(`studio/hal/${name}.hal`, runtimeBase));
      if (!response.ok) throw new Error(`resource ${name} fetch failed: ${response.status}`);
      resources[`studio.${name}`] = await response.text();
      resourceIndex += 1;
      setKernelProgress(
        45 + (resourceIndex / resourceCount) * 17,
        "LOADING KERNEL LIBRARIES",
        `${resourceIndex} / ${resourceCount} · STUDIO.${name.toUpperCase()}`
      );
    }
    for (const name of standardResources) {
      const response = await fetch(new URL(`std/${name}.hal`, runtimeBase));
      if (!response.ok) throw new Error(`resource std.${name.replaceAll("/", ".")} fetch failed: ${response.status}`);
      resources[`std.${name.replaceAll("/", ".")}`] = await response.text();
      resourceIndex += 1;
      setKernelProgress(
        45 + (resourceIndex / resourceCount) * 17,
        "LOADING KERNEL LIBRARIES",
        `${resourceIndex} / ${resourceCount} · STD.${name.replaceAll("/", ".").toUpperCase()}`
      );
    }
    setKernelProgress(64, "CREATING KERNEL", "CONFIGURING ISOLATED WORKER");
    state.nodeRuntime = new NodeRuntime({ space: `workspace/${SPACE}` });
    state.canvasRuntime = instrumentCanvasTelemetry(new CanvasRuntime({
      onDiagnostic: (error) => {
        elements.sourceStatus.textContent = `DIAGNOSTIC // ${errorText(error)}`;
      }
    }));
    state.canvasRuntime.register("canvas/background", query("[data-tron]"));
    const sessionRouter = new SessionRouter();
    const deliver = sessionRouter.deliver.bind(sessionRouter);
    sessionRouter.deliver = async (...args) => {
      try {
        const result = await deliver(...args);
        state.telemetry.deliveredMessages += result?.delivered ?? 0;
        return result;
      } catch (error) {
        state.telemetry.errors += 1;
        throw error;
      }
    };
    state.sessionRouter = sessionRouter;
    const capabilityRegistry = new CapabilityRegistry({ adapters: {
      "surface/canvas-2d": createCanvasCapability(state.canvasRuntime),
      "clock/frame": createClockCapability(),
      "ai/chat": createAiCapability(state.aiAdapters, {
        workspaceForSession: (sessionId) => state.aiSessionWorkspaces.get(String(sessionId)) ?? null
      })
    } });
    state.capabilityRegistry = capabilityRegistry;
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
      graphHostOptions: { sessionRouter },
      renderCanvas: (_canvasId, value) => {
        showScene(validateScene(value), performance.now(), "HAL");
      }
    });
    state.broker = instrumentBrokerTelemetry(createBrowserBroker({
      workerUrl: new URL("hta-worker.js", runtimeBase),
      sharedWorkerUrl: new URLSearchParams(location.search).has("shared-runtime")
        ? new URL("hta-shared-worker.js", runtimeBase) : undefined,
      moduleBytes,
      hostCalls,
      resources,
      onKernelStarting: async (kernel) => {
        state.telemetry.kernelsCreated += 1;
        let space = state.kernelSpaces.get(kernel.name);
        if (!space && kernel.name.startsWith("DOC.")) {
          space = [...state.kernelSpaces.entries()]
            .find(([name]) => kernel.name.startsWith(`DOC.${name}.`))?.[1];
        }
        if (!space) throw new Error(`NO_WORKSPACE_SCOPE ${kernel.name}`);
        state.contextSpaces.set(kernel.context, space);
        const mount = await kernel.context.createFilesystem({ provider: "indexeddb", key: space });
        await kernel.context.session().attachFilesystem(mount);
      },
      onKernelCreated: async (kernel) => {
        const session = sessionRouter.register(kernel.name, kernel.context, {
          onRelease: (sessionId) => graphHost.releaseSession(sessionId)
        });
        const workspaceId = workspaceIdForSession(kernel.name);
        if (workspaceId && state.aiAdapters.list(workspaceId).length) {
          state.aiSessionWorkspaces.set(kernel.name, workspaceId);
          capabilityRegistry.grant(kernel.name, ["ai/chat"]);
        } else if (workspaceId) {
          state.aiSessionWorkspaces.set(kernel.name, workspaceId);
        }
        return session;
      },
      onKernelClosed: (kernel) => {
        state.telemetry.kernelsClosed += 1;
        state.aiSessionWorkspaces.delete(kernel.name);
        capabilityRegistry.revokeSession(kernel.name);
        return sessionRouter.unregister(kernel.name);
      }
    }));
    state.defaultBootstrap = defaultBootstrap;
    setKernelProgress(74, "STARTING KERNEL", "BOOTSTRAPPING HOME SPACE");
    await state.broker.eval(ROOT, defaultBootstrap(SPACE));
    setKernelProgress(84, "LOADING WORKSPACE", "READING PROJECT MANIFESTS");
    await loadBackgroundWorkspace();
    setKernelProgress(92, "LOADING SOURCE", "EVALUATING BACKGROUND PROGRAM");
    await loadBackgroundSource(state.backgroundSource);
    const files = await listFiles();
    if (!files.length) await seedFiles();
    setRuntimeStatus("WASM // LIVE", "live");
    elements.backgroundPicker.hidden = false;
    elements.editorStatus.textContent = "READY";
    const preferred = localStorage.getItem(ACTIVE_FILE_KEY);
    const path = state.files.includes(preferred) ? preferred :
      state.files.includes("/sketches/neon-orbit.hal") ? "/sketches/neon-orbit.hal" : state.files[0];
    if (path) await openFile(path, true, false);
    syncHighlight();
    await renderSavedWorkspaces();
    setWorkspace(0, { reloadBackground: false });
    setKernelProgress(100, "KERNEL READY", "HARA.WASM LIVE");
    setTimeout(hideKernelProgress, 700);
    setTimeout(() => {
      document.body.classList.add("is-start-ready");
      query("[data-start]").disabled = false;
    }, 950);
  } catch (error) {
    state.telemetry.errors += 1;
    console.error("[hara www]", error);
    setRuntimeStatus("WASM // ERROR", "error");
    elements.editorStatus.textContent = `BOOT ERROR // ${errorText(error)}`;
    hideKernelProgress();
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
installKernelStatistics();
installWorkspaceTabs();
installWorkspaceCreation();
installPublishing();
installGitHubAccount();
installAiAdapters();
installWindowManager();
installEditor();
installBackgroundEditor();
installFileActions();
restoreWindows();
setWorkspace(0);
renderSavedWorkspaces().catch((error) => toast(`WORKSPACE INDEX FAILED: ${errorText(error)}`, true));
bootRuntime();
