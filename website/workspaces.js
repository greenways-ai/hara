const DATABASE = "hara-www-projects";
const WORKSPACES = "workspaces";
const FILES = "files";

const now = () => new Date().toISOString();
const safeId = (value) => String(value).toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");

const PROJECT = (id, capabilities = []) => `{:hara/type :project
 :hara/version "1.0.0"
 :project/id ${id.replaceAll("-", ".")}
 :project/version "0.1.0"
 :project/source-paths ["src"]
 :project/test-paths ["test"]
 :project/extension-paths ["extensions"]
 :project/main ${id.replaceAll("-", ".")}.main
 :project/capabilities #{${capabilities.map((item) => `:${item}`).join(" ")}}}
`;

const WORKSPACE = (id, template, areas, layout) => `{:hara/type :workspace
 :hara/version "1.0.0"
 :workspace/id :${id}
 :workspace/layout ${layout}
 :workspace/documents
 [{:document/id "document/main" :document/path "src/main.hal"
   :document/language :hal :document/anonymous true}]
 :workspace/areas
 [${areas.map((area) => `{:area/id "${area.id}" :area/type :${area.type} :area/title "${area.title}"}`).join("\n  ")}]
 :workspace/nodes []
 :workspace/connections []
 :workspace/links []
 :workspace/customizations {:template :${template}}}
`;

const split = (first, second, direction = "horizontal", ratio = 0.35) =>
  `{:layout/type :split :layout/direction :${direction} :layout/ratio ${ratio}
   :layout/first {:layout/type :area :layout/area "${first}"}
   :layout/second {:layout/type :area :layout/area "${second}"}}`;

const nested = (first, second, third, direction = "horizontal") =>
  `{:layout/type :split :layout/direction :${direction} :layout/ratio 0.28
   :layout/first {:layout/type :area :layout/area "${first}"}
   :layout/second ${split(second, third, direction === "horizontal" ? "vertical" : "horizontal", 0.62)}}`;

const SOURCES = {
  blank: ";; A new Hara workspace.\n(+ 19 23)\n",
  canvas: `{:version 1 :width 960 :height 600 :background "#020408"
 :commands [[:circle 480 300 90 "#41f5e4"]
            [:circle 480 300 24 "#9c7bff"]]}\n`,
  music: `;; Music workspace entry point.\n{:tempo 120 :tracks [] :playing false}\n`,
  "3d": `{:creative/version 1 :background "#020408"
 :entities [{:id "mesh/hero" :mesh {:primitive :box}
             :material {:color "#41f5e4"}
             :transform {:rotation [0 0 0]}}]}\n`,
  graphs: `{:version 1 :width 960 :height 600 :background "#020408"
 :commands [[:line 80 500 880 500 "#225f70" 2]
            [:polyline [[100 430] [260 360] [420 390] [580 230] [740 280] [860 120]]
             "#41f5e4" 5]]}\n`
};

const DEFINITIONS = {
  blank: {
    label: "Blank",
    capabilities: [],
    areas: [
      { id: "area/explorer", type: "file-explorer", title: "Explorer" },
      { id: "area/source", type: "source-editor", title: "Source" },
      { id: "area/output", type: "output", title: "Output" }
    ],
    layout: nested("area/explorer", "area/source", "area/output")
  },
  canvas: {
    label: "Canvas",
    capabilities: ["surface/canvas-2d"],
    areas: [
      { id: "area/explorer", type: "file-explorer", title: "Explorer" },
      { id: "area/source", type: "source-editor", title: "Source" },
      { id: "area/canvas", type: "visual-canvas", title: "Canvas" }
    ],
    layout: nested("area/explorer", "area/source", "area/canvas")
  },
  music: {
    label: "Music",
    capabilities: ["audio/playback", "surface/canvas-2d"],
    areas: [
      { id: "area/player", type: "hara-ui/player", title: "Player" },
      { id: "area/mixer", type: "hara-ui/equalizer", title: "Mixer" },
      { id: "area/playlist", type: "hara-ui/playlist", title: "Playlist" },
      { id: "area/spectrum", type: "visual-canvas", title: "Spectrum" }
    ],
    layout: nested("area/playlist", "area/player", "area/spectrum")
  },
  "3d": {
    label: "3D",
    capabilities: ["surface/webgl"],
    areas: [
      { id: "area/hierarchy", type: "scene-hierarchy", title: "Hierarchy" },
      { id: "area/viewport", type: "creative-viewport", title: "3D Viewport" },
      { id: "area/inspector", type: "property-inspector", title: "Inspector" },
      { id: "area/source", type: "source-editor", title: "Source" }
    ],
    layout: nested("area/hierarchy", "area/source", "area/viewport")
  },
  graphs: {
    label: "Graphs",
    capabilities: ["surface/canvas-2d"],
    areas: [
      { id: "area/data", type: "data-source", title: "Source & Data" },
      { id: "area/graph", type: "graph-viewport", title: "Graph" },
      { id: "area/series", type: "series-inspector", title: "Series" }
    ],
    layout: nested("area/data", "area/graph", "area/series")
  }
};

export const workspaceTemplates = Object.freeze(Object.entries(DEFINITIONS).map(([id, value]) => ({
  id,
  label: value.label
})));

export function templateFiles(template, id) {
  const definition = DEFINITIONS[template];
  if (!definition) throw new Error(`UNKNOWN_WORKSPACE_TEMPLATE ${template}`);
  return new Map([
    ["/project.edn", PROJECT(id, definition.capabilities)],
    ["/workspace.edn", WORKSPACE(id, template, definition.areas, definition.layout)],
    ["/src/main.hal", SOURCES[template]]
  ]);
}

export class WorkspaceRepository {
  constructor({ dbName = DATABASE, indexedDB = globalThis.indexedDB } = {}) {
    this.dbName = dbName;
    this.indexedDB = indexedDB;
    this.opening = null;
  }

  open() {
    this.opening ??= new Promise((resolve, reject) => {
      const request = this.indexedDB.open(this.dbName, 1);
      request.onupgradeneeded = () => {
        request.result.createObjectStore(WORKSPACES, { keyPath: "id" });
        request.result.createObjectStore(FILES, { keyPath: ["workspaceId", "path"] });
      };
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
    });
    return this.opening;
  }

  async create({ name, template = "blank" }) {
    const base = safeId(name) || template;
    let id = base;
    let suffix = 2;
    while (await this.get(id)) id = `${base}-${suffix++}`;
    const record = {
      id, name: name.trim() || DEFINITIONS[template].label, template,
      createdAt: now(), updatedAt: now(), providers: {}
    };
    const db = await this.open();
    const tx = db.transaction([WORKSPACES, FILES], "readwrite");
    tx.objectStore(WORKSPACES).put(record);
    for (const [path, content] of templateFiles(template, id)) {
      tx.objectStore(FILES).put({ workspaceId: id, path, content, updatedAt: record.updatedAt });
    }
    await transaction(tx);
    return record;
  }

  async list() {
    const db = await this.open();
    const records = await request(db.transaction(WORKSPACES).objectStore(WORKSPACES), "getAll");
    return records.sort((left, right) => right.updatedAt.localeCompare(left.updatedAt));
  }

  async get(id) {
    const db = await this.open();
    return request(db.transaction(WORKSPACES).objectStore(WORKSPACES), "get", id);
  }

  async files(id) {
    const db = await this.open();
    const records = await request(db.transaction(FILES).objectStore(FILES), "getAll");
    return new Map(records.filter((item) => item.workspaceId === id).map((item) => [item.path, item.content]));
  }

  async writeFile(id, path, content) {
    const db = await this.open();
    const tx = db.transaction([WORKSPACES, FILES], "readwrite");
    const stamp = now();
    tx.objectStore(FILES).put({ workspaceId: id, path, content, updatedAt: stamp });
    const record = await request(tx.objectStore(WORKSPACES), "get", id);
    if (record) tx.objectStore(WORKSPACES).put({ ...record, updatedAt: stamp });
    await transaction(tx);
  }

  async deleteFile(id, path) {
    const db = await this.open();
    const tx = db.transaction(FILES, "readwrite");
    tx.objectStore(FILES).delete([id, path]);
    await transaction(tx);
  }

  async setProvider(id, provider, metadata) {
    const db = await this.open();
    const tx = db.transaction(WORKSPACES, "readwrite");
    const store = tx.objectStore(WORKSPACES);
    const record = await request(store, "get", id);
    if (!record) throw new Error(`UNKNOWN_WORKSPACE ${id}`);
    store.put({ ...record, providers: { ...record.providers, [provider]: metadata }, updatedAt: now() });
    await transaction(tx);
  }

  async setTemplate(id, template) {
    if (!DEFINITIONS[template]) throw new Error(`UNKNOWN_WORKSPACE_TEMPLATE ${template}`);
    const db = await this.open();
    const tx = db.transaction(WORKSPACES, "readwrite");
    const store = tx.objectStore(WORKSPACES);
    const record = await request(store, "get", id);
    if (!record) throw new Error(`UNKNOWN_WORKSPACE ${id}`);
    store.put({ ...record, template, updatedAt: now() });
    await transaction(tx);
  }
}

export function kernelName(id) {
  return `workspace.${safeId(id)}`;
}

function request(store, method, ...args) {
  return new Promise((resolve, reject) => {
    const operation = store[method](...args);
    operation.onsuccess = () => resolve(operation.result ?? null);
    operation.onerror = () => reject(operation.error);
  });
}

function transaction(tx) {
  return new Promise((resolve, reject) => {
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
    tx.onabort = () => reject(tx.error);
  });
}
