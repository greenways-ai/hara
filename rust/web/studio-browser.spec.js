import { existsSync } from "node:fs";
import { expect, test } from "@playwright/test";

// Studio smoke: boots the shared UI against the real raw wasm and evals in
// the ROOT kernel. Skipped when the raw wasm artifact has not been built.
const wasmPath = new URL("../raw/target/wasm32-unknown-unknown/release/hara_wasm_raw.wasm", import.meta.url);
test.skip(!existsSync(wasmPath), "raw wasm artifact not built");

test("studio boots live and evals (+ 1 2) in the ROOT kernel", async ({ page }) => {
  await page.goto("/rust/web/studio-browser.html");
  await expect(page.locator('[data-hara-studio="status"]')).toContainText("WASM · LIVE", { timeout: 60000 });
  await expect(page.locator('[data-hara-studio="status"]')).toContainText("ROOT");
  await page.fill('[data-hara-studio="repl-input"]', "(+ 1 2)");
  await page.press('[data-hara-studio="repl-input"]', "Enter");
  await expect(page.locator('[data-hara-studio="repl-log"]')).toContainText("=> 3");
});
