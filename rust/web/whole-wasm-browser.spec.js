import { test, expect } from "@playwright/test";

test("Chromium compiles and executes a Hara whole-Wasm function", async ({ page }) => {
  await page.goto("/rust/web/index.html");
  const result = await page.evaluate(async () => {
    const { start } = await import("/rust/web/packages/browser/dist/hara-wasm-full/hara.mjs");
    const hara = await start();
    const compiled = await hara.compileWholeWasm(
      "(loop [i 0 acc 0] (if (< i 5000) (recur (+ i 1) (+ acc i)) acc))"
    );
    return String(compiled.call());
  });
  expect(result).toBe("12497500");
});
