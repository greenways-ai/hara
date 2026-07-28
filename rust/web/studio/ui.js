import { HtaKeyword, HtaSymbol } from "../hta.js";
import { defaultBootstrap } from "./boot.js";

/**
 * Shared, framework-free studio UI. `mountStudio(root, { broker })` builds
 * the whole studio DOM inside `root` (no dependency on surrounding page
 * markup), so the same mounting code serves the mkdocs website page now and
 * the hara-chrome panel later. Styling comes from the `.hara-studio-*`
 * classes (rust/web/studio/studio.css); hosts provide the stylesheet.
 *
 * All hara interaction goes through `broker.eval` — the UI never touches
 * IndexedDB or the worker directly. Panes: file tree, writable editor
 * (explicit save, no autosave), REPL log + input, status strip. Switchers:
 * space (list/switch/new/import-from-GitHub) and kernel
 * (list/switch/new/close; kernels stay alive in the broker when switching).
 */

const PROMPT = "hara ›";

/** Render a decoded HTA value as a display string (same approach as
 *  extensions/hara-chrome/src/resp-client.js `renderHta`, plus symbols). */
export function renderValue(value) {
  if (value === null || value === undefined) return "nil";
  if (value instanceof HtaKeyword) return `:${value.name}`;
  if (value instanceof HtaSymbol) return value.name;
  if (value instanceof Map) {
    return `{${[...value].map(([k, v]) => `${renderValue(k)} ${renderValue(v)}`).join(", ")}}`;
  }
  if (value instanceof Set) return `#{${[...value].map(renderValue).join(" ")}}`;
  if (Array.isArray(value)) return `[${value.map(renderValue).join(" ")}]`;
  if (typeof value === "string") return value;
  return String(value);
}

/** Normalize a user-supplied file path to the studio.fs shape ("/a/b.hal").
 *  Returns null for empty, root-only, or parent-escaping paths. */
export function normalizePath(input) {
  if (typeof input !== "string") return null;
  let path = input.trim();
  if (!path) return null;
  if (!path.startsWith("/")) path = `/${path}`;
  path = path.replace(/\/{2,}/g, "/");
  if (path.length > 1 && path.endsWith("/")) path = path.slice(0, -1);
  if (path === "/") return null;
  if (path.split("/").includes("..")) return null;
  return path;
}

/** New source files default to HAL while manifest and explicitly-typed files
 * keep their extension. */
export function normalizeNewFilePath(input) {
  const path = normalizePath(input);
  if (!path) return null;
  const leaf = path.slice(path.lastIndexOf("/") + 1);
  return leaf.includes(".") ? path : `${path}.hal`;
}

/** Build a nested tree (directories first, alphabetical) from a flat list
 *  of file paths. Nodes: { name, path, directory, children? }. */
export function buildTree(paths) {
  const root = { name: "/", path: "/", directory: true, children: [] };
  const directories = new Map([["/", root]]);
  for (const raw of paths ?? []) {
    const path = normalizePath(raw);
    if (!path) continue;
    const segments = path.slice(1).split("/");
    let current = root;
    let currentPath = "";
    for (let index = 0; index < segments.length; index++) {
      currentPath += `/${segments[index]}`;
      if (index === segments.length - 1) {
        current.children.push({ name: segments[index], path: currentPath, directory: false });
      } else {
        let directory = directories.get(currentPath);
        if (!directory) {
          directory = { name: segments[index], path: currentPath, directory: true, children: [] };
          directories.set(currentPath, directory);
          current.children.push(directory);
        }
        current = directory;
      }
    }
  }
  const sort = (node) => {
    node.children.sort((a, b) =>
      a.directory === b.directory ? a.name.localeCompare(b.name) : a.directory ? -1 : 1
    );
    for (const child of node.children) if (child.directory) sort(child);
  };
  sort(root);
  return root.children;
}

/** Parse "owner/repo[@ref]" into { repo, ref, space }; null when malformed.
 *  The imported space takes the repo's bare name. */
export function parseGithubSpec(input) {
  if (typeof input !== "string") return null;
  const match = input.trim().match(/^([A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+)(?:@([A-Za-z0-9_./-]+))?$/);
  if (!match) return null;
  const repo = match[1];
  return { repo, ref: match[2] ?? "main", space: repo.split("/").pop() };
}

/** Source that imports a GitHub repo into a space. Evaluated in the ACTIVE
 *  kernel (spaces live in the shared host store, so any kernel can import);
 *  a dedicated bootstrap kernel would only add lifecycle bookkeeping. */
export function importGithubSource({ space, repo, ref }) {
  return (
    "(do (require [studio.space :as space]) " +
    `(space/import-github! ${JSON.stringify(space)} ${JSON.stringify(repo)} {:ref ${JSON.stringify(ref)}}))`
  );
}

/** Wrap an internal studio form with the requires it needs. Raw wasm
 *  require vectors are UNQUOTED — keep them that way. */
export function studioSource(form) {
  return (
    "(do (require [studio.space :as space]) (require [studio.fs :as fs]) " +
    `(require [studio.boot :as boot]) ${form})`
  );
}

/** Seed content for a newly created file. */
export function defaultFileContent(path) {
  return `;; ${path}\n\n(ns user)\n`;
}

/**
 * Mount the studio into `root`. Options: { broker } — a KernelBroker whose
 * kernels already have the studio.* hal resources registered. Returns a
 * controller: { shell, state, submitRepl, refresh, unmount }.
 */
export function mountStudio(root, {
  broker,
  projects = [],
  runtimeVersion = "development",
  canvasRuntime = null
} = {}) {
  if (!root) throw new Error("mountStudio requires a root element");
  if (!broker) throw new Error("mountStudio requires a broker");
  return new StudioController(root, broker, { projects, runtimeVersion, canvasRuntime });
}

class StudioController {
  constructor(root, broker, { projects, runtimeVersion, canvasRuntime }) {
    this.root = root;
    this.broker = broker;
    this.projects = projects;
    this.runtimeVersion = runtimeVersion;
    this.canvasRuntime = canvasRuntime;
    this.state = {
      kernel: "ROOT",
      space: null,
      files: [],
      open: null,
      dirty: false,
      busy: 0,
      runtime: "BOOTING",
      failed: false
    };
    this.buildDom();
    this.bindEvents();
    this.init();
  }

  // -------------------------------------------------------------- dom build

  buildDom() {
    const shell = el("div", "hara-studio");
    shell.setAttribute("data-hara-studio", "shell");

    const head = el("div", "hara-studio-head");
    head.append(el("span", "hara-kicker", "HARA STUDIO"), el("span", "hara-index", "ENV/01 · LIVE WASM"));

    // Switcher strip: spaces and kernels, reusing the mock's steps styling.
    const steps = el("div", "hara-studio-steps");
    steps.setAttribute("data-hara-studio", "steps");
    this.spaceSelect = el("select", "hara-studio-select");
    this.spaceSelect.setAttribute("data-hara-studio", "space-select");
    this.spaceSelect.setAttribute("aria-label", "Active space");
    this.kernelSelect = el("select", "hara-studio-select");
    this.kernelSelect.setAttribute("data-hara-studio", "kernel-select");
    this.kernelSelect.setAttribute("aria-label", "Active kernel");
    this.newSpaceAction = stepAction("NEW", "Create blank space");
    this.importAction = stepAction("IMPORT", "Import a space from GitHub (owner/repo[@ref])");
    this.newKernelAction = stepAction("NEW", "Create kernel");
    this.closeKernelAction = stepAction("CLOSE", "Close active kernel");
    steps.append(
      label("SPACE"),
      this.spaceSelect,
      this.newSpaceAction,
      this.importAction,
      label("KERNEL"),
      this.kernelSelect,
      this.newKernelAction,
      this.closeKernelAction
    );

    // File tree.
    const tree = el("aside", "hara-frame hara-studio-tree");
    tree.setAttribute("data-hara-studio", "file-tree");
    this.treeSpace = el("span", "hara-index", "—");
    this.newFileAction = action("NEW FILE", "Create a file in the active space");
    const treeHead = el("div", "hara-studio-pane-head");
    const treeHeadRight = el("span");
    treeHeadRight.append(this.treeSpace, text(" · "), this.newFileAction);
    treeHead.append(el("span", null, "FILES"), treeHeadRight);
    this.treeBody = el("div", "hara-studio-tree-body");
    tree.append(treeHead, this.treeBody);

    // Editor (writable; explicit save).
    const editorWrap = el("section", "hara-frame hara-studio-editor-wrap");
    this.editorName = el("span", null, "—");
    this.editorName.setAttribute("data-hara-studio", "editor-name");
    this.dirtyFlag = el("span", "hara-studio-dirty", "");
    this.saveAction = action("SAVE", "Save file to the active space");
    this.runAction = action("RUN", "Evaluate the whole file");
    const editorHead = el("div", "hara-studio-pane-head");
    const editorHeadRight = el("span");
    editorHeadRight.append(this.dirtyFlag, this.runAction, this.saveAction, el("span", "hara-index", "EDITABLE"));
    editorHead.append(this.editorName, editorHeadRight);
    this.editor = el("textarea", "hara-studio-editor");
    this.editor.setAttribute("data-hara-studio", "editor");
    this.editor.setAttribute("spellcheck", "false");
    this.editor.setAttribute("wrap", "off");
    this.editor.setAttribute("aria-label", "File editor");
    this.editor.disabled = true;
    editorWrap.append(editorHead, this.editor);

    // REPL.
    const repl = el("section", "hara-frame hara-studio-repl");
    const replHead = el("div", "hara-studio-pane-head");
    this.replKernel = el("span", "hara-index", "TTY/01 · ROOT");
    replHead.append(el("span", null, "CONSOLE"), this.replKernel);
    this.replLog = el("div", "hara-studio-repl-log");
    this.replLog.setAttribute("data-hara-studio", "repl-log");
    const entry = el("div", "hara-studio-repl-entry");
    this.promptLabel = el("span", "hara-tty-p", PROMPT);
    this.input = el("input", "hara-studio-repl-input");
    this.input.setAttribute("data-hara-studio", "repl-input");
    this.input.setAttribute("type", "text");
    this.input.setAttribute("placeholder", "(your first form)");
    this.input.setAttribute("autocomplete", "off");
    this.input.setAttribute("spellcheck", "false");
    this.input.setAttribute("aria-label", "REPL input");
    entry.append(this.promptLabel, this.input);
    repl.append(replHead, this.replLog, entry);

    const main = el("div", "hara-studio-main");
    main.append(tree, editorWrap, repl);
    this.canvasPanel = el("section", "hara-frame hara-studio-canvas-panel");
    this.canvasPanel.hidden = true;
    this.canvas = el("canvas", "hara-studio-canvas");
    this.canvas.setAttribute("data-hara-studio", "canvas");
    this.canvasPanel.append(
      el("div", "hara-studio-pane-head", "LIVE CANVAS · HAL OWNED"),
      this.canvas
    );
    this.ampFrame = el("iframe", "hara-studio-amp");
    this.ampFrame.title = "Hara Amp live workspace";
    this.ampFrame.hidden = true;
    this.ampFrame.setAttribute("loading", "lazy");
    this.canvasPanel.appendChild(this.ampFrame);
    main.appendChild(this.canvasPanel);
    if (this.canvasRuntime) {
      this.canvasRuntime.register("canvas/background", this.canvas);
      this.canvasRuntime.register("canvas/visualizer", this.canvas);
    }

    // Status strip.
    const status = el("div", "hara-strip hara-studio-status");
    status.setAttribute("data-hara-studio", "status");
    this.statusRuntime = el("b", null, "WASM · BOOTING");
    this.statusKernel = el("b", null, "ROOT");
    this.statusSpace = el("b", null, "—");
    this.statusFiles = el("b", null, "0");
    this.statusState = el("b", null, "BUSY");
    status.append(
      strip("RUNTIME", this.statusRuntime),
      strip("KERNEL", this.statusKernel),
      strip("SPACE", this.statusSpace),
      strip("FILES", this.statusFiles),
      strip("STATE", this.statusState)
    );

    this.projectChooser = el("section", "hara-studio-chooser");
    this.projectChooser.setAttribute("data-hara-studio", "project-chooser");
    this.projectChooser.hidden = true;
    shell.append(head, this.projectChooser, steps, main, status);
    this.shell = shell;
    this.root.appendChild(shell);
    this.buildDialog();
  }

  buildDialog() {
    this.dialog = el("div", "hara-studio-dialog");
    this.dialog.hidden = true;
    this.dialog.setAttribute("role", "dialog");
    this.dialog.setAttribute("aria-modal", "true");
    this.dialogTitle = el("h2", null, "");
    this.dialogLabel = el("label", null, "");
    this.dialogInput = el("input", "hara-studio-dialog-input");
    this.dialogInput.setAttribute("data-hara-studio", "dialog-input");
    this.dialogLabel.appendChild(this.dialogInput);
    this.dialogError = el("p", "hara-studio-dialog-error", "");
    this.dialogCancel = action("CANCEL");
    this.dialogAccept = action("CONTINUE");
    const actions = el("div", "hara-studio-dialog-actions");
    actions.append(this.dialogCancel, this.dialogAccept);
    this.dialog.append(this.dialogTitle, this.dialogLabel, this.dialogError, actions);
    this.shell.appendChild(this.dialog);
  }

  bindEvents() {
    this.spaceSelect.addEventListener("change", () => this.switchSpace(this.spaceSelect.value));
    this.newSpaceAction.addEventListener("click", () => this.newSpace());
    this.importAction.addEventListener("click", () => this.importGithub());
    this.kernelSelect.addEventListener("change", () => this.switchKernel(this.kernelSelect.value));
    this.newKernelAction.addEventListener("click", () => this.newKernel());
    this.closeKernelAction.addEventListener("click", () => this.closeKernel());
    this.newFileAction.addEventListener("click", () => this.newFile());
    this.saveAction.addEventListener("click", () => this.saveFile());
    this.runAction.addEventListener("click", () => this.runFile());
    this.editor.addEventListener("input", () => {
      this.state.dirty = true;
      this.renderEditorHead();
    });
    this.editor.addEventListener("keydown", (event) => {
      if ((event.ctrlKey || event.metaKey) && event.key === "s") {
        event.preventDefault();
        this.saveFile();
      }
    });
    this.input.addEventListener("keydown", (event) => {
      if (event.key === "Enter") {
        event.preventDefault();
        this.submitRepl(this.input.value);
        this.input.value = "";
      }
    });
  }

  // ------------------------------------------------------------------- init

  async init() {
    try {
      this.refreshKernelSelect();
      let spaces = await this.evalStudio("(space/list-spaces)");
      spaces = Array.isArray(spaces) ? spaces.map(String).sort() : [];
      if (spaces.length === 0) {
        await this.broker.eval(this.state.kernel, defaultBootstrap("home"));
        spaces = ["home"];
      }
      if (this.projects.length > 0) {
        this.renderProjectChooser(spaces);
        this.state.runtime = "LIVE";
        this.logNote(";; choose a local browser project to begin");
        this.renderStatus();
        return;
      }
      this.state.space = spaces[0];
      this.renderSpaceSelect(spaces);
      await this.refreshFiles();
      this.state.runtime = "LIVE";
      this.logNote(`;; hara studio — wasm runtime live, kernel ${this.state.kernel}, space ${this.state.space}`);
    } catch (error) {
      this.state.runtime = "ERROR";
      this.state.failed = true;
      this.logError(error);
      this.logNote(";; boot failed — check the console and reload");
    }
    this.renderStatus();
  }

  projectSpace(project) {
    return `project-${project.id}-${this.runtimeVersion}`.replace(/[^A-Za-z0-9_.-]/g, "-");
  }

  renderProjectChooser(spaces = []) {
    this.projectChooser.replaceChildren(
      el("p", "hara-kicker", "CHOOSE A LOCAL PROJECT"),
      el("h1", null, "Make something. Keep it live."),
      el("p", null, "Projects stay in this browser. Pick a complete workspace, edit it, save it, and return later.")
    );
    const cards = el("div", "hara-studio-projects");
    for (const project of this.projects) {
      const space = this.projectSpace(project);
      const recovered = spaces.includes(space);
      const card = el("article", "hara-studio-project");
      card.setAttribute("data-project", project.id);
      card.append(
        el("span", "hara-index", recovered ? "CONTINUE LOCAL PROJECT" : project.category.toUpperCase()),
        el("h2", null, project.title),
        el("p", null, project.description)
      );
      const open = action(recovered ? "CONTINUE" : "OPEN PROJECT");
      open.setAttribute("data-project-open", project.id);
      open.addEventListener("click", () => this.openProject(project, { reset: !recovered }));
      card.appendChild(open);
      if (recovered) {
        const reset = action("RESET");
        reset.setAttribute("data-project-reset", project.id);
        reset.addEventListener("click", async () => {
          if (await this.askConfirm(`Reset ${project.title}?`, "Only this project's local edits will be cleared.")) {
            await this.openProject(project, { reset: true });
          }
        });
        card.appendChild(reset);
      }
      cards.appendChild(card);
    }
    this.projectChooser.appendChild(cards);
    this.projectChooser.hidden = false;
    this.shell.classList.add("is-choosing-project");
  }

  async openProject(project, { reset = false } = {}) {
    const space = this.projectSpace(project);
    await this.task(async () => {
      await this.evalStudio(`(space/create! ${JSON.stringify(space)})`);
      const existing = await this.evalStudio(`(space/files ${JSON.stringify(space)})`);
      if (reset) {
        for (const path of existing ?? []) {
          await this.evalStudio(`(fs/delete! ${JSON.stringify(space)} ${JSON.stringify(String(path))})`);
        }
        for (const [path, content] of Object.entries(project.files)) {
          await this.evalStudio(
            `(fs/write! ${JSON.stringify(space)} ${JSON.stringify(`/${path}`)} ${JSON.stringify(content)})`
          );
        }
      }
    });
    this.state.space = space;
    this.projectChooser.hidden = true;
    this.shell.classList.remove("is-choosing-project");
    this.renderSpaceSelect(await this.listSpaces());
    await this.refreshFiles();
    const preferred = project.main ? `/${project.main}` : this.state.files.find((path) => path.endsWith(".hal"));
    if (preferred && this.state.files.includes(preferred)) await this.openFile(preferred);
    this.activeProject = project;
    const ownsCanvas = project.capabilities.some((value) => value === "canvas/2d" || value === "audio/playback");
    this.canvasPanel.hidden = !ownsCanvas;
    this.canvas.hidden = project.category === "audio";
    this.ampFrame.hidden = project.category !== "audio";
    if (project.category === "audio" && !this.ampFrame.src) {
      this.ampFrame.src = new URL("../../examples/music/hara-amp.html", import.meta.url).href;
    }
    if (ownsCanvas && project.category === "visual") await this.runFile();
    this.logNote(`;; project ${project.title} · manifests loaded · recovery local`);
  }

  askInput(title, label, value = "") {
    return this.showDialog({ title, label, value, confirm: false });
  }

  askConfirm(title, label) {
    return this.showDialog({ title, label, confirm: true });
  }

  showDialog({ title, label, value = "", confirm }) {
    this.dialogTitle.textContent = title;
    this.dialogLabel.firstChild.nodeValue = label;
    this.dialogInput.value = value;
    this.dialogInput.hidden = confirm;
    this.dialogError.textContent = "";
    this.dialog.hidden = false;
    if (!confirm) queueMicrotask(() => this.dialogInput.focus());
    return new Promise((resolve) => {
      const finish = (result) => {
        this.dialog.hidden = true;
        this.dialogCancel.onclick = null;
        this.dialogAccept.onclick = null;
        resolve(result);
      };
      this.dialogCancel.onclick = () => finish(confirm ? false : null);
      this.dialogAccept.onclick = () => finish(confirm ? true : this.dialogInput.value);
    });
  }

  // ------------------------------------------------------------------ evals

  // Eval a studio-lib form in the active kernel (requires wrapped in).
  evalStudio(form) {
    return this.broker.eval(this.state.kernel, studioSource(form));
  }

  // Run an async operation, tracking busy/error state. Errors are logged to
  // the REPL and flip the status strip to ERROR until the next success.
  async task(operation) {
    this.state.busy += 1;
    this.renderStatus();
    try {
      const value = await operation();
      this.state.failed = false;
      return value;
    } catch (error) {
      this.state.failed = true;
      this.logError(error);
      return undefined;
    } finally {
      this.state.busy -= 1;
      this.renderStatus();
    }
  }

  // ------------------------------------------------------------------ repl

  async submitRepl(source) {
    source = (source ?? "").trim();
    if (!source) return;
    const form = el("div");
    form.append(el("span", "hara-tty-p", PROMPT), text(` ${source}`));
    this.replLog.appendChild(form);
    await this.task(async () => {
      const value = await this.broker.eval(this.state.kernel, source);
      this.logValue(value);
    });
    this.replLog.scrollTop = this.replLog.scrollHeight;
  }

  logValue(value) {
    this.replLog.appendChild(el("div", "hara-tty-v", `=> ${renderValue(value)}`));
    this.replLog.scrollTop = this.replLog.scrollHeight;
  }

  logNote(message) {
    this.replLog.appendChild(el("div", "hara-tty-o", message));
    this.replLog.scrollTop = this.replLog.scrollHeight;
  }

  logError(error) {
    this.replLog.appendChild(el("div", "hara-tty-e", `!! ${error?.message ?? error}`));
    this.replLog.scrollTop = this.replLog.scrollHeight;
  }

  // ----------------------------------------------------------------- spaces

  renderSpaceSelect(spaces) {
    this.spaceSelect.replaceChildren();
    for (const name of spaces) {
      const option = el("option", null, name);
      option.value = name;
      if (name === this.state.space) option.selected = true;
      this.spaceSelect.appendChild(option);
    }
    this.treeSpace.textContent = this.state.space ?? "—";
  }

  async listSpaces() {
    const spaces = await this.evalStudio("(space/list-spaces)");
    return Array.isArray(spaces) ? spaces.map(String).sort() : [];
  }

  async switchSpace(name) {
    if (!name || name === this.state.space) return;
    if (!(await this.confirmDiscard())) {
      this.spaceSelect.value = this.state.space ?? "";
      return;
    }
    this.state.space = name;
    this.clearEditor();
    this.renderSpaceSelect(await this.task(() => this.listSpaces()) ?? [name]);
    await this.refreshFiles();
  }

  async newSpace() {
    const name = await this.askInput("New space", "Space name", "");
    if (!name || !name.trim()) return;
    const trimmed = name.trim();
    if (trimmed.includes("/")) {
      this.logError(new Error("space names cannot contain '/'"));
      return;
    }
    const booted = await this.task(() => this.evalStudio(`(boot/boot! ${JSON.stringify(trimmed)})`));
    if (booted === undefined) return;
    this.state.space = trimmed;
    this.clearEditor();
    this.renderSpaceSelect(await this.task(() => this.listSpaces()) ?? [trimmed]);
    await this.refreshFiles();
    this.logNote(`;; space ${trimmed} ready`);
  }

  // Import from GitHub: prompt for owner/repo[@ref], then import-github! in
  // the ACTIVE kernel (see importGithubSource). The imported space takes the
  // repo's bare name.
  async importGithub() {
    const spec = await this.askInput("Import from GitHub", "owner/repo[@ref]", "");
    if (!spec || !spec.trim()) return;
    const parsed = parseGithubSpec(spec);
    if (!parsed) {
      this.logError(new Error(`invalid GitHub spec: ${spec.trim()} (expected owner/repo[@ref])`));
      return;
    }
    this.logNote(`;; importing ${parsed.repo}@${parsed.ref} into space ${parsed.space} …`);
    const summary = await this.task(() => this.evalStudio(importGithubSource(parsed)));
    if (summary === undefined) return;
    this.state.space = parsed.space;
    this.clearEditor();
    this.renderSpaceSelect(await this.task(() => this.listSpaces()) ?? [parsed.space]);
    await this.refreshFiles();
    this.logValue(summary);
  }

  // ---------------------------------------------------------------- kernels

  refreshKernelSelect() {
    const names = this.broker.list();
    this.kernelSelect.replaceChildren();
    for (const name of names) {
      const option = el("option", null, name);
      option.value = name;
      if (name === this.state.kernel) option.selected = true;
      this.kernelSelect.appendChild(option);
    }
    this.replKernel.textContent = `TTY/01 · ${this.state.kernel}`;
  }

  // Switching kernels never closes them — everything stays alive in the
  // broker, so nothing is lost.
  switchKernel(name) {
    if (!name || name === this.state.kernel) return;
    this.state.kernel = name;
    this.refreshKernelSelect();
    this.renderStatus();
    this.logNote(`;; active kernel ${name}`);
  }

  async newKernel() {
    const name = await this.askInput("New kernel", "Kernel name (A-Za-z0-9_.-)", "");
    if (!name || !name.trim()) return;
    const trimmed = name.trim();
    const fallback = defaultBootstrap(this.state.space ?? "home");
    const custom = await this.askInput("Kernel bootstrap", "Source (empty uses the active space)", "");
    const bootstrap = custom && custom.trim() ? custom.trim() : fallback;
    const kernel = await this.task(() => this.broker.create(trimmed, { bootstrap }));
    if (kernel === undefined) {
      this.refreshKernelSelect();
      return;
    }
    this.state.kernel = trimmed;
    this.refreshKernelSelect();
    this.renderStatus();
    this.logNote(`;; kernel ${trimmed} booted`);
  }

  async closeKernel() {
    const name = this.state.kernel;
    if (name === "ROOT") {
      this.logError(new Error("ROOT_CANNOT_CLOSE"));
      return;
    }
    if (!(await this.askConfirm(`Close kernel ${name}?`, "Its in-memory state will be lost."))) return;
    // broker.close resolves undefined on success too, so confirm the close
    // by checking the kernel is actually gone before switching back.
    await this.task(() => this.broker.close(name));
    if (this.broker.list().includes(name)) return;
    this.state.kernel = "ROOT";
    this.refreshKernelSelect();
    this.renderStatus();
    this.logNote(`;; kernel ${name} closed`);
  }

  // ------------------------------------------------------------------ files

  async refreshFiles() {
    const files = await this.task(() =>
      this.evalStudio(`(space/files ${JSON.stringify(this.state.space)})`)
    );
    this.state.files = (Array.isArray(files) ? files.map(String) : []).sort();
    this.renderTree();
    this.renderStatus();
  }

  renderTree() {
    this.treeBody.replaceChildren();
    if (this.state.files.length === 0) {
      this.treeBody.appendChild(el("div", "hara-studio-tree-group", "EMPTY — NEW FILE TO START"));
      return;
    }
    const renderNodes = (nodes) => {
      for (const node of nodes) {
        if (node.directory) {
          this.treeBody.appendChild(el("div", "hara-studio-tree-group", node.path.slice(1).toUpperCase()));
          renderNodes(node.children);
        } else {
          const row = el("div", "hara-studio-file");
          row.setAttribute("data-file", node.path);
          if (node.path === this.state.open) row.classList.add("is-active");
          row.append(el("span", "hara-index", "◇"), text(node.name));
          row.addEventListener("click", () => this.openFile(node.path));
          this.treeBody.appendChild(row);
        }
      }
    };
    renderNodes(buildTree(this.state.files));
  }

  async openFile(path) {
    if (path === this.state.open && !this.state.dirty) return;
    if (!(await this.confirmDiscard())) return;
    const content = await this.task(() =>
      this.evalStudio(`(fs/read ${JSON.stringify(this.state.space)} ${JSON.stringify(path)})`)
    );
    if (content === undefined) return;
    this.state.open = path;
    this.state.dirty = false;
    this.editor.value = content === null ? "" : String(content);
    this.editor.disabled = false;
    this.renderEditorHead();
    this.renderTree();
  }

  async saveFile() {
    if (!this.state.open) return;
    const path = this.state.open;
    const content = this.editor.value;
    const ok = await this.task(() =>
      this.evalStudio(
        `(fs/write! ${JSON.stringify(this.state.space)} ${JSON.stringify(path)} ${JSON.stringify(content)})`
      )
    );
    if (ok === undefined) return;
    this.state.dirty = false;
    this.renderEditorHead();
    this.logNote(`;; saved ${path}`);
  }

  async runFile() {
    if (!this.state.open) return;
    const source = this.editor.value;
    await this.task(async () => {
      if (/^\s*(?:;[^\n]*\n\s*)*\(ns\+/.test(source)) {
        const documentId = `${this.activeProject?.id ?? "document"}:${this.state.open}`;
        const nodeId = `node/${this.activeProject?.id ?? "document"}`;
        this.canvasRuntime?.claim(nodeId, this.activeProject?.category === "audio"
          ? "canvas/visualizer"
          : "canvas/background");
        const result = await this.broker.evalDocument(this.state.kernel, documentId, source, { nodeId });
        if (typeof result.value === "string" && result.value.startsWith("task-")) {
          this.broker.evalForm(
            this.state.kernel,
            documentId,
            `(studio.node/run-task ${JSON.stringify(result.value)})`
          ).catch((error) => this.logError(error));
        }
        this.logNote(`;; activated ${this.state.open} generation ${result.generation}`);
        return result;
      }
      const value = await this.broker.eval(this.state.kernel, source);
      this.logValue(value);
      return value;
    });
  }

  async newFile() {
    if (!this.state.space) return;
    const input = await this.askInput(`New file in ${this.state.space}`, "File path", "/scratch.hal");
    if (!input) return;
    const path = normalizeNewFilePath(input);
    if (!path) {
      this.logError(new Error(`invalid file path: ${input.trim()}`));
      return;
    }
    const ok = await this.task(() =>
      this.evalStudio(
        `(fs/write! ${JSON.stringify(this.state.space)} ${JSON.stringify(path)} ${JSON.stringify(defaultFileContent(path))})`
      )
    );
    if (ok === undefined) return;
    // The file is created either way; only switch to it when any unsaved
    // edits in the currently open file may go (same guard as file/space
    // switching).
    if (!(await this.confirmDiscard())) {
      await this.refreshFiles();
      return;
    }
    this.state.dirty = false;
    await this.refreshFiles();
    await this.openFile(path);
  }

  clearEditor() {
    this.state.open = null;
    this.state.dirty = false;
    this.editor.value = "";
    this.editor.disabled = true;
    this.renderEditorHead();
  }

  async confirmDiscard() {
    if (!this.state.dirty) return true;
    return this.askConfirm("Discard unsaved changes?", "The editor contents have not been saved.");
  }

  // ---------------------------------------------------------------- render

  renderEditorHead() {
    this.editorName.textContent = this.state.open ?? "—";
    this.dirtyFlag.textContent = this.state.dirty ? "● " : "";
  }

  renderStatus() {
    this.statusRuntime.textContent = `WASM · ${this.state.runtime}`;
    this.statusKernel.textContent = this.state.kernel;
    this.statusSpace.textContent = this.state.space ?? "—";
    this.statusFiles.textContent = String(this.state.files.length);
    this.statusState.textContent = this.state.busy > 0 ? "BUSY" : this.state.failed ? "ERROR" : "IDLE";
    this.treeSpace.textContent = this.state.space ?? "—";
  }

  // ----------------------------------------------------------------- handle

  async refresh() {
    await this.refreshFiles();
    this.refreshKernelSelect();
    this.renderStatus();
  }

  unmount() {
    this.shell.remove();
  }
}

// ------------------------------------------------------------------ helpers

function el(tag, className, content) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (content !== undefined) node.textContent = content;
  return node;
}

function text(content) {
  return document.createTextNode(content);
}

function label(content) {
  return el("span", "hara-studio-step hara-studio-label", content);
}

function stepAction(content, title) {
  const node = el("button", "hara-studio-step hara-studio-action", content);
  node.type = "button";
  if (title) node.setAttribute("title", title);
  return node;
}

function action(content, title) {
  const node = el("button", "hara-studio-action", content);
  node.type = "button";
  if (title) node.setAttribute("title", title);
  return node;
}

function strip(name, valueNode) {
  const span = el("span");
  span.append(text(`${name} `), valueNode);
  return span;
}
