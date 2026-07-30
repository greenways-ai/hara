import { existsSync } from "node:fs";
import { expect, test } from "@playwright/test";

const builtDemo = new URL("../../target/www/hara-animation.html", import.meta.url);
test.skip(!existsSync(builtDemo), "built animation demo is unavailable");

test("animation demo synchronizes cast and actions with HAL source", async ({ page }) => {
  await page.goto("/target/www/hara-animation.html");
  const source = page.getByRole("textbox", { name: "Editable animation pipeline source" });
  await expect(source).toHaveValue(/"selected" "robot"/);

  await page.getByRole("button", { name: /FOX/ }).click();
  await expect(source).toHaveValue(/"selected" "fox"/);

  await page.getByRole("button", { name: "+ JUMP" }).click();
  await expect(source).toHaveValue(/"actions" \["walk" "wave" "jump" "spin" "bow" "jump"\]/);

  await page.getByRole("button", { name: "PLAY PIPELINE" }).click();
  await expect(page.locator("[data-frame]")).not.toHaveText("FRAME 0000");
  await expect(page.locator("[data-current-action]")).not.toHaveText("WAITING");
});
