import { existsSync } from "node:fs";
import { expect, test } from "@playwright/test";

const builtRuntime = new URL("../../target/www/runtime/hara.wasm", import.meta.url);
test.skip(!existsSync(builtRuntime), "built website runtime is unavailable");

test("Hara Amp routes WASM FFT frames through ns+ before canvas rendering", async ({ page }) => {
  await page.goto("/target/www/hara-amp.html");
  await expect(page.locator("[data-runtime-status]")).toContainText("WASM · LIVE", {
    timeout: 60000
  });

  await page.getByRole("button", { name: "Play", exact: true }).click();

  await expect(page.locator("[data-audio-status]")).toHaveText("PLAYING / WASM");
  await expect(page.locator("[data-frame-status]")).toContainText("HAL · FRAME", {
    timeout: 15000
  });
  await expect.poll(async () => Number(await page.locator("html").getAttribute("data-rendered-frames")))
    .toBeGreaterThan(1);
  await expect.poll(async () => Number(await page.locator("html").getAttribute("data-node-queued")))
    .toBeLessThanOrEqual(1);
});

test("Hara Amp exposes spectrum, scope, and artwork visualizer modes", async ({ page }) => {
  await page.goto("/target/www/hara-amp.html");
  await expect(page.locator("[data-runtime-status]")).toContainText("WASM · LIVE", {
    timeout: 60000
  });

  await page.getByRole("button", { name: "SCOPE", exact: true }).click();
  await expect(page.getByRole("button", { name: "SCOPE", exact: true })).toHaveAttribute(
    "aria-pressed",
    "true"
  );
  await page.getByRole("button", { name: "ARTWORK", exact: true }).click();
  await expect(page.getByRole("button", { name: "ARTWORK", exact: true })).toHaveAttribute(
    "aria-pressed",
    "true"
  );
});
