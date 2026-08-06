#!/usr/bin/env node
import { readFile, writeFile, mkdir } from "node:fs/promises";
import { existsSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { decodeHta, encodeHta, HtaKeyword } from "../../../rust/web/packages/hta/index.js";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const wasmPath = resolve(root, "rust/raw/target/wasm32-unknown-unknown/release/hara_wasm_raw.wasm");
const profile = process.argv.includes("--standard") ? "standard" : "smoke";
const iterations = profile === "standard" ? 20_000 : 500;
const outputArg = process.argv.indexOf("--output");
const output = resolve(root, outputArg >= 0 ? process.argv[outputArg + 1] : "target/boundary-benchmark.json");

const payloads = {
  scalar: 42,
  request1k: new Map([
    [new HtaKeyword("method"), "GET"],
    [new HtaKeyword("path"), "/bench"],
    [new HtaKeyword("headers"), new Map([["x-benchmark", "x".repeat(900)]])]
  ]),
  bytes64k: new Uint8Array(64 * 1024).fill(0x5a)
};

function sample(name, operation, count = iterations) {
  for (let i = 0; i < Math.min(count, 100); i += 1) operation();
  const started = process.hrtime.bigint();
  for (let i = 0; i < count; i += 1) operation();
  return { boundary: name, iterations: count,
    steady_ns: Number((process.hrtime.bigint() - started) / BigInt(count)) };
}

const measurements = [];
for (const [name, value] of Object.entries(payloads)) {
  const encoded = encodeHta(value);
  measurements.push({ ...sample(`js-hta-encode/${name}`, () => encodeHta(value)), bytes: encoded.length });
  measurements.push({ ...sample(`js-hta-decode/${name}`, () => decodeHta(encoded)), bytes: encoded.length });
}

if (existsSync(wasmPath)) {
  const bytes = await readFile(wasmPath);
  const instantiateStarted = process.hrtime.bigint();
  const { instance } = await WebAssembly.instantiate(bytes, {});
  const instantiateNs = Number(process.hrtime.bigint() - instantiateStarted);
  const callFrame = (fn, frame) => {
    const pointer = Number(instance.exports.hta_alloc(frame.length));
    new Uint8Array(instance.exports.memory.buffer, pointer, frame.length).set(frame);
    try { return fn(pointer, frame.length); }
    finally { instance.exports.hta_dealloc(pointer, frame.length); }
  };
  const next = () => {
    const packed = instance.exports.hta_next_event();
    if (packed === 0n) throw new Error("raw wasm produced no event");
    const pointer = Number(packed >> 32n), size = Number(packed & 0xffff_ffffn);
    const frame = new Uint8Array(instance.exports.memory.buffer, pointer, size).slice();
    instance.exports.hta_dealloc(pointer, size);
    return decodeHta(frame);
  };
  const invoke = target => {
    const frame = encodeHta([target, ["42"]]);
    const task = Number(callFrame(instance.exports.hta_start, frame));
    const event = next();
    instance.exports.hta_drop_task(BigInt(task));
    if (Number(event[0]) !== 0 || String(event[2]) !== "42") {
      throw new Error(`${target} failed: ${JSON.stringify(event)}`);
    }
  };
  measurements.push({ boundary: "js-to-raw-wasm/instantiate", first_ns: instantiateNs, iterations: 1 });
  measurements.push(sample("js-to-raw-wasm/eval", () => invoke("eval"), profile === "standard" ? 2_000 : 50));
  measurements.push(sample("js-to-raw-wasm/eval-vm", () => invoke("eval-vm"), profile === "standard" ? 2_000 : 50));
} else {
  measurements.push({ boundary: "js-to-raw-wasm", status: "unsupported", reason: `missing ${wasmPath}` });
}

const report = { schema_version: 1, profile, node: process.version, measurements };
await mkdir(dirname(output), { recursive: true });
await writeFile(output, `${JSON.stringify(report, null, 2)}\n`);
const lines = ["# Hara boundary benchmark", "", "| Boundary | ns/call | Bytes | Status |", "|---|---:|---:|---|"];
for (const row of measurements) lines.push(`| ${row.boundary} | ${row.steady_ns ?? row.first_ns ?? "—"} | ${row.bytes ?? "—"} | ${row.status ?? "ok"} |`);
await writeFile(output.replace(/\.json$/, ".md"), `${lines.join("\n")}\n`);
console.log(`wrote ${output}`);
