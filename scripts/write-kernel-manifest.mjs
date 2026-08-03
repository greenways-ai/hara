import { brotliCompressSync, gzipSync } from "node:zlib";
import { createHash } from "node:crypto";
import { readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";

const [directoryArgument, version = "0.1.1"] = process.argv.slice(2);
if (!directoryArgument) throw new Error("usage: write-kernel-manifest.mjs DIRECTORY VERSION");
const directory = resolve(directoryArgument);
const variants = {
  core: { file: `hara-wasm-core-${version}.wasm`, features: ["interpreter", "hta", "sessions", "host-bridges"] },
  vm: { file: `hara-wasm-vm-${version}.wasm`, features: ["bytecode-vm", "hta", "sessions", "host-bridges"] },
  trace: { file: `hara-wasm-trace-${version}.wasm`, features: ["dev-trace", "hta", "sessions", "host-bridges"] }
};
for (const value of Object.values(variants)) {
  const bytes = await readFile(resolve(directory, value.file));
  value.url = `/runtime/${value.file}`;
  value.sha256 = createHash("sha256").update(bytes).digest("hex");
  value.bytes = { raw: bytes.length, gzip: gzipSync(bytes, { level: 9 }).length, brotli: brotliCompressSync(bytes).length };
}
if (variants.core.bytes.gzip > 335_000 || variants.core.bytes.brotli > 270_000) {
  throw new Error(`hara-wasm-core exceeds its transfer budget: ${JSON.stringify(variants.core.bytes)}`);
}
const manifest = { schema: "hara-kernel-manifest/v1", version, htaAbi: 2, variants };
await writeFile(resolve(directory, "kernel-manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`);
