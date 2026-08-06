import { existsSync } from "node:fs";
import { expect, test } from "@playwright/test";

const builtDemo = new URL("../../target/www/hara-animation.html", import.meta.url);
test.skip(!existsSync(builtDemo), "built animation demo is unavailable");

test("animation demo synchronizes cast and actions with HAL source", async ({ page }) => {
  await page.goto("/target/www/hara-animation.html");
  const source = page.locator("[data-source]");
  await expect(source).toHaveValue(/"selected" "robot"/);
  await expect(page.locator("body")).toHaveAttribute("data-view", "stage");

  await page.getByRole("button", { name: /OPEN BUILD VIEW/ }).click();
  await expect(page.locator("body")).toHaveAttribute("data-view", "build");

  await page.getByRole("button", { name: /FOX/ }).click();
  await expect(source).toHaveValue(/"selected" "fox"/);

  await page.getByRole("button", { name: "+ JUMP" }).click();
  await expect(source).toHaveValue(/"actions" \["walk" "wave" "jump" "spin" "bow" "jump"\]/);

  await page.getByRole("button", { name: /STAGE VIEW/, exact: true }).last().click();
  await page.getByRole("button", { name: "PLAY PIPELINE" }).click();
  await expect(page.locator("[data-frame]")).not.toHaveText("FRAME 0000");
  await expect(page.locator("[data-current-action]")).not.toHaveText("WAITING");

  const frame = Number((await page.locator("[data-frame]").textContent()).match(/\d+/)[0]);
  await page.getByRole("button", { name: /OPEN BUILD VIEW/ }).click();
  await expect(page.locator("body")).toHaveAttribute("data-view", "build");
  await expect(source).toBeVisible();
  await expect.poll(async () =>
    Number((await page.locator("[data-frame]").textContent()).match(/\d+/)[0])).toBeGreaterThan(frame);

  await page.getByRole("button", { name: /STAGE VIEW/, exact: true }).last().click();
  await expect(page.locator("body")).toHaveAttribute("data-view", "stage");
  await expect(page.locator("[data-stage]")).toBeVisible();
});
