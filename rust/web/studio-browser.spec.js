import { existsSync } from "node:fs";
import { expect, test } from "@playwright/test";

// Studio smoke: boots the shared UI against the real raw wasm and evals in
// the ROOT kernel. Skipped when the raw wasm artifact has not been built.
const wasmPath = new URL("../raw/target/wasm32-unknown-unknown/release/hara_wasm_raw.wasm", import.meta.url);
test.skip(!existsSync(wasmPath), "raw wasm artifact not built");

test("studio boots live and evals (+ 1 2) in the ROOT kernel", async ({ page }) => {
  await page.goto("/rust/web/studio-browser.html");
  await expect(page.locator('[data-hara-studio="runtime-status"]')).toHaveAttribute("data-state", "live", { timeout: 60000 });
  await expect(page.locator('[data-hara-studio="project-bar"]')).toBeVisible();
  await expect(page.getByText("HARA STUDIO", { exact: true })).toHaveCount(0);
  await expect(page.getByText("ENV/01 · LIVE WASM", { exact: true })).toHaveCount(0);
  await page.locator('[data-hara-studio="runtime-status"]').click();
  await expect(page.locator('[data-hara-studio="runtime"]')).toHaveText("LIVE");
  await expect(page.locator('[data-hara-studio="kernel"]')).toHaveText("ROOT");
  await page.getByRole("button", { name: "Show console" }).click();
  await page.fill('[data-hara-studio="repl-input"]', "(+ 1 2)");
  await page.press('[data-hara-studio="repl-input"]', "Enter");
  await expect(page.locator('[data-hara-studio="repl-log"]')).toContainText("=> 3");
});

test("compact studio chrome has no horizontal overflow on phone", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/rust/web/studio-browser.html");
  await expect(page.locator('[data-hara-studio="runtime-status"]')).toHaveAttribute("data-state", "live", { timeout: 60000 });
  await expect(page.locator(".hara-studio-mobile-tabs")).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
});
