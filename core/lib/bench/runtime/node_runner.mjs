import { createRequire } from "node:module";
const require = createRequire(import.meta.url);
const glue = process.env.HARA_WASM_GLUE;
if (!glue) throw new Error("HARA_WASM_GLUE is required");
const { Runtime } = require(glue);
const [, , runtimeName, id, encodedSource, expected, windowsText, callsText] = process.argv;
const source = Buffer.from(encodedSource, "base64url").toString("utf8");
const windows = Number(windowsText);
const calls = Number(callsText);
const runtime = new Runtime();
const firstStart = process.hrtime.bigint();
const firstValue = runtime.eval(source);
const firstNs = process.hrtime.bigint() - firstStart;
if (firstValue !== expected) throw new Error(`${id}: expected ${expected}, got ${firstValue}`);
const samples = [];
for (let window = 0; window < windows; window += 1) {
  const started = process.hrtime.bigint();
  for (let call = 0; call < calls; call += 1) {
    const value = runtime.eval(source);
    if (value !== expected) throw new Error(`${id}: checksum changed to ${value}`);
  }
  samples.push(Number((process.hrtime.bigint() - started) / BigInt(calls)));
}
console.log(JSON.stringify({runtime: runtimeName, workload: id,
  first_ns: Number(firstNs), samples_ns: samples}));
