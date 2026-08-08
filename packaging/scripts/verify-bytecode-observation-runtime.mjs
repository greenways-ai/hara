#!/usr/bin/env node
import { access, readFile } from "node:fs/promises";
import { join, resolve } from "node:path";

const root = resolve(process.argv[2] ?? "");
if (!process.argv[2]) {
  throw new Error("usage: verify-bytecode-observation-runtime.mjs <runtime-root>");
}

const payloads = [
  ["rust/bytecode-observation.wasm", "rust/host/bytecode-observation.js"],
  [
    "examples/music/runtime/bytecode-observation.wasm",
    "examples/music/runtime/host/bytecode-observation.js",
  ],
];

for (const [wasmPath, hostPath] of payloads) {
  await access(join(root, wasmPath));
  await access(join(root, hostPath));

  const source = await readFile(join(root, hostPath), "utf8");
  for (const forbidden of ["innerHTML", "insertAdjacentHTML", "new Function", "eval("]) {
    if (source.includes(forbidden)) {
      throw new Error(`${hostPath} contains forbidden browser construction: ${forbidden}`);
    }
  }
  for (const required of [
    "BytecodeObservationRuntime",
    "BytecodeObservationSession",
    "observation_invoke",
    "dispose-all",
  ]) {
    if (!source.includes(required)) {
      throw new Error(`${hostPath} missing ${required}`);
    }
  }

  const bytes = await readFile(join(root, wasmPath));
  const { instance } = await WebAssembly.instantiate(bytes, {});
  const exports = instance.exports;
  for (const name of [
    "memory",
    "observation_abi_version",
    "observation_alloc",
    "observation_dealloc",
    "observation_invoke",
  ]) {
    if (!(name in exports)) throw new Error(`${wasmPath} missing ${name}`);
  }
  if (exports.observation_abi_version() !== 1) {
    throw new Error(`${wasmPath} has an unsupported observation ABI`);
  }
}

console.log(`verified bytecode observation runtime: ${root}`);
