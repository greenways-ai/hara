#!/usr/bin/env node
import { access, readFile } from "node:fs/promises";
import { dirname, extname, join, relative, resolve } from "node:path";

const root = resolve(process.argv[2] ?? "");
if (!process.argv[2]) throw new Error("usage: verify-studio-runtime.mjs <runtime-root>");

const required = [
  "rust/hara.wasm",
  "rust/hta.js",
  "rust/hta-worker.js",
  "rust/hta-shared-worker.js",
  "rust/packages/hta/index.js",
  "rust/packages/hta/worker.js",
  "rust/packages/hta/shared-worker.js",
  "rust/host/broker.js",
  "rust/host/services.js",
  "rust/studio/broker.js",
  "rust/studio/capability-registry.js",
  "rust/studio/capabilities/canvas.js",
  "rust/studio/capabilities/clock.js",
  "rust/studio/graph-host.js",
  "rust/studio/host-services.js",
  "rust/studio/program-host.js",
  "rust/studio/program-worker.js",
  "rust/studio/session-router.js",
  "rust/studio/supersonic.js",
  "rust/studio/ui.js",
  "rust/studio/studio.css",
  "rust/ui/tokens.css",
  "rust/ui/components.css",
  "rust/ui/studio.css",
  "rust/ui/studio-shell.js",
  "rust/studio/hal/store.hal",
  "rust/studio/hal/boot.hal",
  "rust/studio/hal/graph.hal",
  "rust/studio/hal/program.hal",
  "rust/studio/hal/session.hal",
  "rust/studio/hal/supersonic.hal",
  "rust/std/lib/substrate.hal",
  "rust/std/lib/substrate/frame.hal",
  "rust/std/lib/substrate/protocol.hal",
  "examples/index.json",
  "assets/wasm/demo-synth.wasm",
  "assets/wasm/demo-fft.wasm",
  "assets/artwork/hara-amp-artwork-original.png",
  "assets/artwork/hara-amp-emblem.png",
  "examples/music/hara-amp.html",
  "examples/music/hara-amp.css",
  "examples/music/hara-amp.js",
  "examples/music/runtime/hara.wasm",
  "examples/music/runtime/hta.js",
  "examples/music/runtime/hta-worker.js",
  "examples/music/runtime/hta-shared-worker.js",
  "examples/music/runtime/packages/hta/index.js",
  "examples/music/runtime/packages/hta/worker.js",
  "examples/music/runtime/packages/hta/shared-worker.js",
  "examples/music/runtime/host/broker.js",
  "examples/music/runtime/host/services.js",
  "examples/music/runtime/studio/supersonic.js",
  "examples/music/runtime/studio/hal/supersonic.hal"
];

for (const path of required) await access(join(root, path));

// The public HTA entry points are intentionally tiny wrappers. Verify their
// relative module graph recursively so an archive cannot pass while omitting a
// browser dependency, as happened when packages/hta was left behind.
const moduleEntries = [
  "rust/hta.js",
  "rust/hta-worker.js",
  "rust/hta-shared-worker.js",
  "examples/music/runtime/hta.js",
  "examples/music/runtime/hta-worker.js",
  "examples/music/runtime/hta-shared-worker.js"
];
const verifiedModules = new Set();
for (const path of moduleEntries) await verifyRelativeImports(path);

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

async function verifyRelativeImports(path) {
  if (verifiedModules.has(path)) return;
  verifiedModules.add(path);
  const absolute = join(root, path);
  const source = await readFile(absolute, "utf8");
  const specifications = new Set([
    ...[...source.matchAll(/\bfrom\s*["'](\.[^"']+)["']/g)].map((match) => match[1]),
    ...[...source.matchAll(/\bimport\s*["'](\.[^"']+)["']/g)].map((match) => match[1])
  ]);

  for (const specification of specifications) {
    const target = resolve(dirname(absolute), specification);
    const targetPath = relative(root, target);
    if (targetPath.startsWith("..") || targetPath === "") {
      throw new Error(`${path} import escapes runtime: ${specification}`);
    }
    await access(target);
    if ([".js", ".mjs"].includes(extname(target))) await verifyRelativeImports(targetPath);
  }
}
