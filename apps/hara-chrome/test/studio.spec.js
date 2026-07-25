import { test, expect } from "@playwright/test";
import { launchWithExtension } from "./extension.js";

test("panel mounts the studio UI", async () => {
  const { context, extensionId } = await launchWithExtension();
  const page = await context.newPage();
  await page.goto(`chrome-extension://${extensionId}/src/panel.html?tabId=0`);
  await page.waitForFunction(() => globalThis.hara !== undefined);
  await expect(page.locator('[data-hara-studio="shell"]')).toBeVisible();
  await expect(page.locator('[data-hara-studio="file-tree"]')).toBeVisible();
  await expect(page.locator('[data-hara-studio="editor"]')).toBeVisible();
  await expect(page.locator('[data-hara-studio="repl-log"]')).toBeVisible();
  await expect(page.locator('[data-hara-studio="repl-input"]')).toBeVisible();
  await expect(page.locator('[data-hara-studio="status"]')).toBeVisible();
  // switchers boot with the ROOT kernel and the default "home" space
  await expect(page.locator('[data-hara-studio="kernel-select"]')).toHaveValue("ROOT");
  await expect(page.locator('[data-hara-studio="space-select"]')).toHaveValue("home");
  await expect(page.locator('[data-hara-studio="status"]')).toContainText("LIVE");
  await context.close();
});

test("studio REPL evals in the active kernel", async () => {
  const { context, extensionId } = await launchWithExtension();
  const page = await context.newPage();
  await page.goto(`chrome-extension://${extensionId}/src/panel.html?tabId=0`);
  await page.waitForFunction(() => globalThis.hara !== undefined);
  const input = page.locator('[data-hara-studio="repl-input"]');
  await input.fill("(+ 1 2)");
  await input.press("Enter");
  await expect(page.locator('[data-hara-studio="repl-log"]')).toContainText("=> 3");
  await context.close();
});
