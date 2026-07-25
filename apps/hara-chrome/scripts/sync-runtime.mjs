import { copyFileSync, existsSync, mkdirSync, readdirSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const repo = path.resolve(root, "../..");
const vendor = path.join(root, "vendor");
mkdirSync(vendor, { recursive: true });

const files = [
  [path.join(repo, "rust/web/hta.js"), path.join(vendor, "hta.js")],
  [path.join(repo, "rust/web/hta-worker.js"), path.join(vendor, "hta-worker.js")],
  [
    path.join(repo, "rust/raw/target/wasm32-unknown-unknown/release/hara_wasm_raw.wasm"),
    path.join(vendor, "hara.wasm"),
  ],
];
for (const [from, to] of files) {
  if (!existsSync(from)) {
    console.error(`missing ${from} — run: bash scripts/build-hara-wasm-raw`);
    process.exit(1);
  }
  copyFileSync(from, to);
  console.log(`synced ${path.basename(to)}`);
}

// Studio environment (broker, host services, boot template, UI, styles, hal
// libs): copied preserving the layout — vendor/studio/*.js import "../hta.js"
// and the panel fetches vendor/studio/hal/*.hal as kernel resources.
const studio = path.join(repo, "rust/web/studio");
for (const [sub, filter] of [
  ["", (name) => name.endsWith(".js") || name.endsWith(".css")],
  ["hal", (name) => name.endsWith(".hal")],
]) {
  const out = path.join(vendor, "studio", sub);
  mkdirSync(out, { recursive: true });
  for (const name of readdirSync(path.join(studio, sub)).filter(filter)) {
    copyFileSync(path.join(studio, sub, name), path.join(out, name));
    console.log(`synced studio/${sub ? `${sub}/` : ""}${name}`);
  }
}
