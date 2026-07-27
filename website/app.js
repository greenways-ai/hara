import { renderScene, validateScene } from "./scene.js";
import { startTron } from "./tron.js";
import { applyParedit, insertIndent, localFormAt } from "./editor.js";

const SPACE = "home";
const ROOT = "ROOT";
const ACTIVE_FILE_KEY = "hara-www.active-file.v1";
const WINDOWS_KEY = "hara-www.windows.v1";
const WORKSPACE_KEY = "hara-www.workspace.v1";
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
  files: [],
  activeFile: null,
  dirty: false,
  lastScene: null,
  zIndex: 10,
  workspace: Number(localStorage.getItem(WORKSPACE_KEY)) === 1 ? 1 : 0
};

const elements = {
  launcher: query("[data-launcher]"),
  launcherToggle: query("[data-launcher-toggle]"),
  launcherScrim: query("[data-launcher-scrim]"),
  systemContext: query("[data-system-context]"),
  runtimeLed: query("[data-runtime-led]"),
  runtimeLabel: query("[data-runtime-label]"),
  fileTree: query("[data-file-tree]"),
  fileCount: query("[data-file-count]"),
  editor: query("[data-editor]"),
  editorTitle: query("[data-editor-title]"),
  editorStatus: query("[data-editor-status]"),
  lineNumbers: query("[data-line-numbers]"),
  dirty: query("[data-dirty]"),
  save: query("[data-save]"),
  run: query("[data-run]"),
  paredit: query("[data-paredit]"),
  inlineEval: query("[data-inline-eval]"),
  completions: query("[data-hal-completions]"),
  outputCanvas: query("[data-output-canvas]"),
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
  toasts: query("[data-toasts]")
};

const completionState = { entries: [], index: 0, start: 0 };

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

function setWorkspace(index) {
  state.workspace = index === 1 ? 1 : 0;
  document.body.dataset.workspace = String(state.workspace);
  localStorage.setItem(WORKSPACE_KEY, String(state.workspace));
  elements.systemContext.textContent = state.workspace === 0 ? "WELCOME // 01" : "VISUAL LAB // 02";
  for (const dot of queryAll("[data-workspace-dot]")) {
    dot.setAttribute("aria-current", String(Number(dot.dataset.workspaceDot) === state.workspace));
  }
  closeLauncher();
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
  saveWindows();
}

function openWindow(name) {
  setWorkspace(1);
  const windowNode = query(`[data-window="${name}"]`);
  focusWindow(windowNode);
  closeLauncher();
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
  query("[data-start]").addEventListener("click", () => setWorkspace(1));
  query("[data-home]").addEventListener("click", () => setWorkspace(0));
  for (const dot of queryAll("[data-workspace-dot]")) {
    dot.addEventListener("click", () => setWorkspace(Number(dot.dataset.workspaceDot)));
  }

  document.addEventListener("keydown", (event) => {
    if (elements.dialog.open || elements.launcher.classList.contains("is-open")) return;
    if (event.target.matches("input, textarea, select")) return;
    if (event.key === "ArrowRight") setWorkspace(1);
    if (event.key === "ArrowLeft") setWorkspace(0);
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
      setWorkspace(dx < 0 ? 1 : 0);
    }
  });
}

function installLauncher() {
  elements.launcherToggle.addEventListener("click", () => {
    setLauncher(!elements.launcher.classList.contains("is-open"));
  });
  elements.launcherScrim.addEventListener("click", closeLauncher);
  for (const tile of queryAll("[data-open-window]")) {
    tile.addEventListener("click", () => openWindow(tile.dataset.openWindow));
  }
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && elements.launcher.classList.contains("is-open")) {
      closeLauncher();
      elements.launcherToggle.focus();
    }
  });
}

function studioSource(form) {
  return `(do (require [studio.space :as space]) (require [studio.fs :as fs]) ${form})`;
}

function evalStudio(form) {
  return state.broker.eval(ROOT, studioSource(form));
}

async function listFiles() {
  const result = await evalStudio(`(space/files ${JSON.stringify(SPACE)})`);
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
  const count = elements.editor.value.split("\n").length;
  elements.lineNumbers.textContent = Array.from({ length: count }, (_, index) => index + 1).join("\n");
  renderFiles();
}

async function openFile(path, force = false) {
  if (state.dirty && !force) {
    const discard = await confirmDialog("UNSAVED CHANGES", "Discard the current editor changes?");
    if (!discard) return;
  }
  const content = await evalStudio(`(fs/read ${JSON.stringify(SPACE)} ${JSON.stringify(path)})`);
  state.activeFile = path;
  state.dirty = false;
  elements.editor.value = content == null ? "" : String(content);
  elements.editorStatus.textContent = "READY";
  localStorage.setItem(ACTIVE_FILE_KEY, path);
  updateEditorChrome();
  openWindow("editor");
}

async function saveFile(showToast = true) {
  if (!state.activeFile) return false;
  elements.editorStatus.textContent = "SAVING";
  await evalStudio(
    `(fs/write! ${JSON.stringify(SPACE)} ${JSON.stringify(state.activeFile)} ${JSON.stringify(elements.editor.value)})`
  );
  state.dirty = false;
  elements.editorStatus.textContent = "SAVED";
  updateEditorChrome();
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

function positionEditorOverlay(node, offset) {
  const source = elements.editor.value.slice(0, offset);
  const line = source.split("\n").length - 1;
  const column = source.length - source.lastIndexOf("\n") - 1;
  node.style.top = `${14 + line * 18 - elements.editor.scrollTop}px`;
  node.style.left = `${62 + Math.min(column * 7.1, Math.max(30, elements.editor.clientWidth - 190))}px`;
}

function showInlineEval(form, label, error = false) {
  elements.inlineEval.textContent = `⇒ ${label}`;
  elements.inlineEval.classList.toggle("is-error", error);
  elements.inlineEval.hidden = false;
  positionEditorOverlay(elements.inlineEval, form.end ?? elements.editor.selectionEnd);
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

async function evaluateForm() {
  if (!state.activeFile) return;
  elements.run.disabled = true;
  const selection = elements.editor.value.slice(elements.editor.selectionStart, elements.editor.selectionEnd).trim();
  const form = selection ? { source: selection } : localFormAt(elements.editor.value, elements.editor.selectionStart);
  if (!form?.source) {
    elements.editorStatus.textContent = "NO FORM AT CURSOR";
    elements.run.disabled = false;
    return;
  }
  elements.editorStatus.textContent = "EVALUATING FORM";
  const started = performance.now();
  try {
    const result = await state.broker.eval(ROOT, form.source);
    try {
      const scene = validateScene(result);
      state.lastScene = scene;
      query('[data-window="canvas"]').classList.remove("is-hidden");
      drawLastScene();
      elements.canvasEmpty.classList.add("is-hidden");
      elements.canvasStatus.textContent = `FRAME // ${Math.round(performance.now() - started)} MS`;
      elements.canvasSize.textContent = `${scene.width} × ${scene.height}`;
      elements.editorStatus.textContent = "FORM RENDERED";
      if (innerWidth <= 900) focusWindow(query('[data-window="canvas"]'));
      toast("FORM RENDERED");
    } catch {
      const label = resultLabel(result);
      elements.editorStatus.textContent = `EVAL // ${label}`;
      showInlineEval(form, label);
    }
  } catch (error) {
    const message = errorText(error);
    elements.editorStatus.textContent = `ERROR // ${message}`;
    showInlineEval(form, message, true);
    elements.canvasStatus.textContent = "FRAME // LAST GOOD";
    toast(message, true);
  } finally {
    elements.run.disabled = false;
  }
}

function normalizePath(value) {
  if (typeof value !== "string") return null;
  let path = value.trim().replace(/\/+/g, "/");
  if (!path.startsWith("/")) path = `/${path}`;
  if (path === "/" || path.includes("..") || !/\.hal$/i.test(path)) return null;
  return path;
}

async function seedFiles(force = false) {
  if (force) {
    for (const path of await listFiles()) {
      await evalStudio(`(fs/delete! ${JSON.stringify(SPACE)} ${JSON.stringify(path)})`);
    }
  }
  for (const [path, content] of DEFAULT_FILES) {
    await evalStudio(
      `(fs/write! ${JSON.stringify(SPACE)} ${JSON.stringify(path)} ${JSON.stringify(content)})`
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
      message: "Paths must end in .hal"
    });
    if (raw == null) return;
    const path = normalizePath(raw);
    if (!path) return toast("INVALID HARA FILE PATH", true);
    if (state.files.includes(path)) return toast("FILE ALREADY EXISTS", true);
    const content = `;; ${path}\n\n{:version 1\n :width 960\n :height 600\n :background "#020408"\n :commands []}\n`;
    await evalStudio(`(fs/write! ${JSON.stringify(SPACE)} ${JSON.stringify(path)} ${JSON.stringify(content)})`);
    await listFiles();
    await openFile(path, true);
    await evaluateForm();
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
      `(fs/write! ${JSON.stringify(SPACE)} ${JSON.stringify(nextPath)} ${JSON.stringify(elements.editor.value)})`
    );
    if (nextPath !== oldPath) {
      await evalStudio(`(fs/delete! ${JSON.stringify(SPACE)} ${JSON.stringify(oldPath)})`);
    }
    state.activeFile = nextPath;
    localStorage.setItem(ACTIVE_FILE_KEY, nextPath);
    await listFiles();
    updateEditorChrome();
    toast(`RENAMED ${oldPath}`);
  });

  query("[data-file-delete]").addEventListener("click", async () => {
    if (!state.activeFile) return;
    const path = state.activeFile;
    if (!await confirmDialog("DELETE HARA FILE", `Delete ${path}? This cannot be undone.`)) return;
    await evalStudio(`(fs/delete! ${JSON.stringify(SPACE)} ${JSON.stringify(path)})`);
    state.activeFile = null;
    state.dirty = false;
    elements.editor.value = "";
    localStorage.removeItem(ACTIVE_FILE_KEY);
    await listFiles();
    updateEditorChrome();
    if (state.files.length) await openFile(state.files[0], true);
    toast(`DELETED ${path}`);
  });

  query("[data-reset-demo]").addEventListener("click", async () => {
    closeLauncher();
    if (!await confirmDialog("RESET HARA DEMO", "Restore the example files and default window layout?")) return;
    localStorage.removeItem(WINDOWS_KEY);
    await seedFiles(true);
    state.activeFile = null;
    state.dirty = false;
    location.reload();
  });
}

async function bootRuntime() {
  setRuntimeStatus("WASM // BOOTING", "booting");
  elements.editorStatus.textContent = "BOOTING HARA.WASM";
  try {
    const runtimeBase = new URL("./runtime/", import.meta.url);
    const [{ createBrowserBroker }, { createHostServices }, { defaultBootstrap }] = await Promise.all([
      import(new URL("studio/broker.js", runtimeBase)),
      import(new URL("studio/host-services.js", runtimeBase)),
      import(new URL("studio/boot.js", runtimeBase))
    ]);
    const wasmResponse = await fetch(new URL("hara.wasm", runtimeBase));
    if (!wasmResponse.ok) throw new Error(`runtime fetch failed: ${wasmResponse.status}`);
    const moduleBytes = new Uint8Array(await wasmResponse.arrayBuffer());
    const resources = {};
    for (const name of ["store", "fs", "space", "boot"]) {
      const response = await fetch(new URL(`studio/hal/${name}.hal`, runtimeBase));
      if (!response.ok) throw new Error(`resource ${name} fetch failed: ${response.status}`);
      resources[`studio.${name}`] = await response.text();
    }
    state.broker = createBrowserBroker({
      workerUrl: new URL("hta-worker.js", runtimeBase),
      sharedWorkerUrl: new URL("hta-shared-worker.js", runtimeBase),
      moduleBytes,
      hostCalls: createHostServices({ dbName: "hara-www" }),
      resources
    });
    await state.broker.eval(ROOT, defaultBootstrap(SPACE));
    const files = await listFiles();
    if (!files.length) await seedFiles();
    setRuntimeStatus("WASM // LIVE", "live");
    elements.editorStatus.textContent = "READY";
    const preferred = localStorage.getItem(ACTIVE_FILE_KEY);
    const path = state.files.includes(preferred) ? preferred :
      state.files.includes("/sketches/neon-orbit.hal") ? "/sketches/neon-orbit.hal" : state.files[0];
    if (path) await openFile(path, true);
  } catch (error) {
    console.error("[hara www]", error);
    setRuntimeStatus("WASM // ERROR", "error");
    elements.editorStatus.textContent = `BOOT ERROR // ${errorText(error)}`;
    toast(`HARA RUNTIME FAILED: ${errorText(error)}`, true);
  }
}

function installEditor() {
  elements.editor.addEventListener("input", () => {
    state.dirty = true;
    updateEditorChrome();
    updateCompletions();
  });
  elements.editor.addEventListener("scroll", () => {
    elements.lineNumbers.scrollTop = elements.editor.scrollTop;
    if (!elements.inlineEval.hidden) positionEditorOverlay(elements.inlineEval, elements.editor.selectionEnd);
    if (!elements.completions.hidden) renderCompletions();
  });
  elements.editor.addEventListener("keydown", (event) => {
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
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "s") {
      event.preventDefault();
      saveFile();
    }
    if ((event.metaKey || event.ctrlKey) && (event.key === "Enter" || event.key.toLowerCase() === "e")) {
      event.preventDefault();
      evaluateForm();
    }
    if (elements.paredit.getAttribute("aria-pressed") === "true" &&
        !event.metaKey && !event.ctrlKey && !event.altKey &&
        applyParedit(elements.editor, event.key)) {
      event.preventDefault();
      return;
    }
    if (event.key === "Tab") {
      event.preventDefault();
      insertIndent(elements.editor, event.shiftKey);
    }
  });
  elements.paredit.addEventListener("click", () => {
    const enabled = elements.paredit.getAttribute("aria-pressed") !== "true";
    elements.paredit.setAttribute("aria-pressed", String(enabled));
    elements.paredit.textContent = enabled ? "PAREDIT // ON" : "PAREDIT // OFF";
    toast(enabled ? "PAREDIT ENABLED" : "PAREDIT DISABLED");
  });
  elements.editor.addEventListener("blur", () => setTimeout(hideCompletions, 120));
  elements.save.addEventListener("click", () => saveFile());
  elements.run.addEventListener("click", evaluateForm);
}

startTron(query("[data-tron]"));
installWorkspaceNavigation();
installLauncher();
installWindowManager();
installEditor();
installFileActions();
restoreWindows();
setWorkspace(state.workspace);
bootRuntime();
