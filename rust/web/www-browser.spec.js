import { existsSync } from "node:fs";
import { expect, test } from "@playwright/test";

test("www shell navigates workspaces and opens desktop apps", async ({ page }) => {
  await page.goto("/website/");
  await page.locator("[data-start]").click();
  await expect(page.locator("body")).toHaveAttribute("data-workspace", "1");
  await expect(page.locator('[data-workspace-dot="1"]')).toHaveAttribute("aria-current", "true");

  await page.locator("[data-home]").click();
  await expect(page.locator("body")).toHaveAttribute("data-workspace", "0");

  await page.locator("[data-launcher-toggle]").click();
  await expect(page.locator("[data-launcher]")).toHaveAttribute("aria-hidden", "false");
  await page.locator('[data-open-window="files"]').click();
  await expect(page.locator("body")).toHaveAttribute("data-workspace", "1");
  await expect(page.locator('[data-window="files"]')).toHaveClass(/is-focused/);
});

const builtRuntime = new URL("../../target/www/runtime/hara.wasm", import.meta.url);
const runtimeTest = existsSync(builtRuntime) ? test : test.skip;

runtimeTest("www evaluates the default Hara sketch into the canvas", async ({ page }) => {
  await page.goto("/target/www/");
  await expect(page.locator("[data-runtime-label]")).toHaveText("WASM // LIVE", { timeout: 60000 });
  await expect(page.locator("[data-editor-title]")).toContainText("NEON-ORBIT.HAL");
  await page.locator("[data-run]").click();
  await expect(page.locator("[data-canvas-empty]")).toHaveClass(/is-hidden/);
  await expect(page.locator("[data-canvas-status]")).toContainText("FRAME //");
  await expect(page.locator("[data-editor-status]")).toHaveText("FILE RENDERED");
});

runtimeTest("www evaluates scalars through the SharedWorker runtime", async ({ page }) => {
  await page.goto("/target/www/?shared-runtime=1");
  await expect(page.locator("[data-runtime-label]")).toHaveText("WASM // LIVE", { timeout: 60000 });

  await page.locator("[data-editor]").fill("(+ 19 23)");
  await page.locator("[data-run]").click();

  await expect(page.locator("[data-inline-eval]")).toHaveText("=> 42");
  await expect(page.locator("[data-editor-status]")).toHaveText("EVAL // 42");
  await expect(page.locator("[data-run]")).toBeEnabled();
});
