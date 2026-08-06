import { existsSync } from "node:fs";
import { expect, test } from "@playwright/test";

const builtRuntime = new URL("../../target/www/runtime/hara.wasm", import.meta.url);
test.skip(!existsSync(builtRuntime), "built website runtime is unavailable");

test("Hara Amp routes WASM FFT frames through ns+ before canvas rendering", async ({ page }) => {
  test.setTimeout(90_000);
  await page.goto("/target/www/");
  await expect(page.getByRole("button", { name: "START" })).toBeEnabled({
    timeout: 60000
  });
  await page.getByRole("button", { name: "START" }).click();
  await expect(page.locator("[data-amp-runtime-state]")).toContainText("SILENT PROBE COMPLETED", {
    timeout: 60000
  });
  await page.getByRole("button", { name: /PLAY HARA AMP/ }).click();

  await expect(page.locator("[data-story-audio]")).toHaveText("PLAYING / WASM");
  await expect(page.locator("[data-story-frame-status]")).toContainText("HAL · FRAME", {
    timeout: 15000
  });
  await expect.poll(async () => Number(await page.locator("[data-story-rendered]").textContent()))
    .toBeGreaterThan(1);
  await expect.poll(async () => Number((await page.locator("[data-story-queue]").textContent()).split(" ")[0]))
    .toBeLessThanOrEqual(1);

  await expect(page.locator(".story-next-copy strong")).toHaveText("Build View ↔ Stage View");
  await page.locator("[data-workspace-next]").click();
  await expect(page.getByRole("heading", { name: /A player is/ })).toBeVisible();
  await expect(page.locator("[data-story-audio]")).toHaveText("PLAYING / WASM");
  await page.locator("[data-workspace-prev]").click();
  await expect(page.getByRole("button", { name: /PAUSE HARA AMP/ }))
    .toHaveAttribute("aria-pressed", "true");
});

test("Hara Amp exposes synchronized Node/Text views and selectable completion", async ({ page }) => {
  test.setTimeout(90_000);
  await page.goto("/target/www/?amp=editor");
  await expect(page.getByRole("heading", { name: /A player is/ })).toBeVisible({
    timeout: 60000
  });
  await expect(page.locator("[data-amp-node-graph] [data-node-id]")).toHaveCount(11, {
    timeout: 60000
  });
  await page.getByRole("tab", { name: "TEXT VIEW" }).click();
  await expect(page.getByRole("textbox", { name: "Editable Hara Amp source" }))
    .toHaveValue(/"id" "playlist"/);
  const repl = page.getByRole("textbox", { name: "HAL" });
  await repl.fill("sonic/st");
  await expect(page.getByRole("listbox")).toBeVisible();
  await page.getByRole("option", { name: /sonic\/status symbol/ }).click();
  await expect(repl).toHaveValue("sonic/status");

  await repl.fill("(+ 19 23)");
  await repl.press("Enter");
  await expect(page.locator("[data-story-repl-history] > div").last().locator("output"))
    .toHaveText("42");

  await repl.fill("(str \"first\"\n \"second\")");
  await repl.press("Shift+Enter");
  await expect(repl).toHaveValue("(str \"first\"\n \"second\")\n");
});

test("story navigation continues through the animation stage and build screens", async ({ page }) => {
  test.setTimeout(90_000);
  await page.goto("/target/www/");
  const next = page.locator("[data-workspace-next]");
  await expect(page.getByRole("button", { name: "START" })).toBeEnabled({
    timeout: 60000
  });
  await page.getByRole("button", { name: "START" }).click();
  await next.click();

  const stage = page.frameLocator('iframe[title="Hara animation stage view"]');
  await next.click();
  await expect(stage.locator("body")).toHaveAttribute("data-view", "stage");
  await expect(stage.getByRole("heading", { name: "DIRECT A CHARACTER" })).toBeVisible();

  const build = page.frameLocator('iframe[title="Hara animation build view"]');
  await next.click();
  await expect(build.locator("body")).toHaveAttribute("data-view", "build");
  await expect(build.getByRole("heading", { name: "HAL SOURCE" })).toBeVisible();
  await expect(next).toBeDisabled();
});

test("startup does not route filesystem probes into the background task", async ({ page }) => {
  test.setTimeout(90_000);
  await page.goto("/target/www/");
  await expect(page.getByRole("button", { name: "START" })).toBeEnabled({
    timeout: 60000
  });
  await page.reload();
  await expect(page.getByRole("button", { name: "START" })).toBeEnabled({
    timeout: 60000
  });
  await page.waitForTimeout(5000);
  await expect(page.locator("[data-toasts]")).not.toContainText(/file\/(not-found|not-directory)/);
  await expect(page.locator("[data-background-status]")).not.toContainText("ERROR");
});
