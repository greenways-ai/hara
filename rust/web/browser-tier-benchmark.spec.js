import { test } from "@playwright/test";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";

const standard = process.env.HARA_BENCH_PROFILE === "standard";
test.setTimeout(standard ? 30 * 60_000 : 30_000);

test("benchmark browser bytecode against browser whole-Wasm", async ({ page }) => {
  await page.goto("/rust/web/index.html");
  const rows = await page.evaluate(async ({ standard }) => {
    const corpus = await fetch("/lib/bench/lisp-hara/general-workloads.json").then((response) => response.json());
    const [{ start: startVm }, { start: startWhole }] = await Promise.all([
      import("/rust/web/packages/browser/dist/hara-wasm-vm/hara.mjs"),
      import("/rust/web/packages/browser/dist/hara-wasm-full/hara.mjs")
    ]);
    const vm = await startVm();
    const whole = await startWhole();
    const median = (values) => values.sort((a, b) => a - b)[Math.floor(values.length / 2)];
    const measure = (call) => {
      for (let index = 0; index < 5; index += 1) call();
      const targetMilliseconds = standard ? 250 : 25;
      const windowCount = standard ? 30 : 7;
      let calls = 1;
      while (calls < 16_777_216) {
        const started = performance.now();
        for (let index = 0; index < calls; index += 1) call();
        const elapsed = performance.now() - started;
        if (elapsed >= targetMilliseconds) break;
        calls *= Math.max(2, Math.ceil(targetMilliseconds / Math.max(elapsed, 0.01)));
      }
      const samples = [];
      for (let window = 0; window < windowCount; window += 1) {
        const started = performance.now();
        for (let callIndex = 0; callIndex < calls; callIndex += 1) call();
        samples.push((performance.now() - started) * 1e6 / calls);
      }
      const steady_ns = Math.round(median(samples));
      return { steady_ns, samples_ns: samples.map(Math.round),
        throughput_per_sec: 1e9 / steady_ns, calls_per_window: calls };
    };
    const results = [];
    for (const workload of corpus.workloads) {
      let prepareStarted = performance.now();
      const artifact = vm.compileBytecode(workload.hara_source);
      const prepareNs = Math.round((performance.now() - prepareStarted) * 1e6);
      const vmCall = () => {
        const value = vm.evalBytecode(artifact);
        if (value !== workload.expected) throw new Error(`VM checksum: ${value}`);
      };
      let firstStarted = performance.now();
      vmCall();
      const firstNs = Math.round((performance.now() - firstStarted) * 1e6);
      results.push({ runtime: "hara-wasm-vm", workload: workload.id,
        prepare_ns: prepareNs, first_ns: firstNs, checksum: workload.expected,
        ...measure(vmCall), status: "ok" });
      try {
        prepareStarted = performance.now();
        const compiled = await whole.compileWholeWasm(workload.hara_source);
        const wholePrepareNs = Math.round((performance.now() - prepareStarted) * 1e6);
        const wholeCall = () => {
          const value = String(compiled.call());
          if (value !== workload.expected) throw new Error(`whole-Wasm checksum: ${value}`);
        };
        firstStarted = performance.now();
        wholeCall();
        const wholeFirstNs = Math.round((performance.now() - firstStarted) * 1e6);
        results.push({ runtime: "hara-wasm-full", workload: workload.id,
          prepare_ns: wholePrepareNs, first_ns: wholeFirstNs, checksum: workload.expected,
          ...measure(wholeCall), status: "ok" });
      } catch (error) {
        results.push({ runtime: "hara-wasm-full", workload: workload.id,
          status: "error", reason: error.message });
      }
    }
    return results;
  }, { standard });
  console.log(`BROWSER_TIER_BENCHMARK=${JSON.stringify(rows)}`);
  const output = resolve(import.meta.dirname, "../../target/browser-tier-benchmark.json");
  mkdirSync(dirname(output), { recursive: true });
  writeFileSync(output, `${JSON.stringify({
    schema_version: 1,
    profile: standard ? "standard" : "smoke",
    generated_at: new Date().toISOString(),
    measurements: rows
  }, null, 2)}\n`);
});
