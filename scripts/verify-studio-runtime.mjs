#!/usr/bin/env node
import { access, readFile } from "node:fs/promises";
import { join, relative, resolve } from "node:path";

const root = resolve(process.argv[2] ?? "");
if (!process.argv[2]) throw new Error("usage: verify-studio-runtime.mjs <runtime-root>");

const required = [
  "rust/hara.wasm",
  "rust/hta.js",
  "rust/hta-worker.js",
  "rust/studio/broker.js",
  "rust/studio/capability-registry.js",
  "rust/studio/capabilities/canvas.js",
  "rust/studio/capabilities/clock.js",
  "rust/studio/graph-host.js",
  "rust/studio/host-services.js",
  "rust/studio/program-host.js",
  "rust/studio/program-worker.js",
  "rust/studio/session-router.js",
  "rust/studio/ui.js",
  "rust/studio/studio.css",
  "rust/ui/tokens.css",
  "rust/ui/components.css",
  "rust/ui/studio.css",
  "rust/studio/hal/store.hal",
  "rust/studio/hal/fs.hal",
  "rust/studio/hal/space.hal",
  "rust/studio/hal/boot.hal",
  "rust/studio/hal/graph.hal",
  "rust/studio/hal/program.hal",
  "rust/studio/hal/session.hal",
  "examples/index.json",
  "assets/wasm/demo-synth.wasm",
  "assets/wasm/demo-fft.wasm",
  "assets/artwork/hara-amp-artwork-original.png",
  "assets/artwork/hara-amp-emblem.png",
  "examples/music/hara-amp.html",
  "examples/music/hara-amp.css",
  "examples/music/hara-amp.js",
  "examples/music/runtime/hara.wasm"
];

for (const path of required) await access(join(root, path));
const index = JSON.parse(await readFile(join(root, "examples/index.json"), "utf8"));
if (index.version !== "1.0.0" || !Array.isArray(index.projects) || index.projects.length !== 3) {
  throw new Error("examples/index.json must describe exactly three v1 projects");
}

for (const project of index.projects) {
  for (const key of ["id", "title", "description", "category", "project", "workspace", "capabilities"]) {
    if (project[key] === undefined) throw new Error(`${project.id ?? "project"} missing ${key}`);
  }
  for (const path of [project.project, project.workspace, ...project.files]) {
    const target = resolve(root, path);
    if (relative(root, target).startsWith("..")) throw new Error(`path escapes runtime: ${path}`);
    await access(target);
  }
  const projectEdn = await readFile(join(root, project.project), "utf8");
  const workspaceEdn = await readFile(join(root, project.workspace), "utf8");
  for (const token of [":hara/type :project", ":project/main", ":project/source-paths"]) {
    if (!projectEdn.includes(token)) throw new Error(`${project.project} missing ${token}`);
  }
  for (const token of [":hara/type :workspace", ":workspace/layout", ":workspace/documents",
    ":workspace/areas", ":workspace/nodes", ":workspace/connections", ":workspace/links",
    ":workspace/customizations"]) {
    if (!workspaceEdn.includes(token)) throw new Error(`${project.workspace} missing ${token}`);
  }
}

console.log(`verified studio runtime: ${root}`);
