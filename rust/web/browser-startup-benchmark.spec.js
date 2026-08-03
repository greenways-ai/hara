import { test } from "@playwright/test";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";

test("measure browser package startup", async ({ page }) => {
  await page.goto("/rust/web/index.html");
  const startup = await page.evaluate(async () => {
    const modules = {
      "hara-wasm-vm": "/rust/web/packages/browser/dist/hara-wasm-vm/hara.mjs",
      "hara-wasm-full": "/rust/web/packages/browser/dist/hara-wasm-full/hara.mjs",
    };
    const result = {};
    for (const [runtime, moduleUrl] of Object.entries(modules)) {
      const samples = [];
      for (let index = 0; index < 30; index += 1) {
        const started = performance.now();
        const module = await import(`${moduleUrl}?startup-sample=${index}`);
        await module.start();
        samples.push(Math.round((performance.now() - started) * 1e6));
      }
      const ordered = [...samples].sort((a, b) => a - b);
      result[runtime] = { samples_ns: samples, p50_ns: ordered[Math.floor(ordered.length / 2)] };
    }
    return result;
  });
  const output = resolve(import.meta.dirname, "../../target/browser-startup-benchmark.json");
  mkdirSync(dirname(output), { recursive: true });
  writeFileSync(output, `${JSON.stringify({ schema_version: 1, profile: "standard",
    generated_at: new Date().toISOString(), startup }, null, 2)}\n`);
});
