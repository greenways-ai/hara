import { existsSync } from "node:fs";
import { expect, test } from "@playwright/test";

const builtRuntime = new URL("../../target/www/runtime/hara.wasm", import.meta.url);
test.skip(!existsSync(builtRuntime), "built website runtime is unavailable");

test("Hara Amp routes WASM FFT frames through ns+ before canvas rendering", async ({ page }) => {
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
  await page.goto("/target/www/?amp=editor");
  await expect(page.getByRole("heading", { name: /A player is/ })).toBeVisible({
    timeout: 60000
  });
  await expect(page.locator("[data-amp-node-graph] [data-node-id]")).toHaveCount(11);
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
