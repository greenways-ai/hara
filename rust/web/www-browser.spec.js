import { existsSync } from "node:fs";
import { expect, test } from "@playwright/test";

test("www always opens Home and offers manifest-backed workspace templates", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("hara-www.workspace.v1", "1"));
  await page.goto("/website/");
  await expect(page.locator("body")).toHaveAttribute("data-workspace", "0");
  await expect(page.getByRole("link", { name: "GITHUB ↗" })).toHaveAttribute(
    "href",
    "https://github.com/hara-lang/hara",
  );
  await expect(page.getByRole("link", { name: "YOUTUBE ↗" })).toHaveAttribute(
    "href",
    /youtube\.com\/results/,
  );
  await page.locator("[data-start]").click();
  await expect(page.locator("[data-template-dialog]")).toBeVisible();
  await expect(page.locator("[data-template]")).toHaveCount(5);
  await page.locator("[data-template-close]").click();

  await page.locator("[data-launcher-toggle]").click();
  await expect(page.locator("[data-launcher]")).toHaveAttribute("aria-hidden", "false");
  await expect(page.locator("[data-new-workspace]")).toBeVisible();
  await expect(page.locator("[data-deploy-template]")).toHaveCount(0);
});

test("zoomed desktop keeps explorer, source, and output visible", async ({ page }) => {
  await page.setViewportSize({ width: 892, height: 900 });
  await page.goto("/website/");
  await page.evaluate(() => { document.body.dataset.workspace = "1"; });
  for (const name of ["files", "editor", "canvas"]) {
    await expect(page.locator(`[data-window="${name}"]`)).toBeVisible();
  }
});

test("phone shell uses touch controls and one explicit workspace panel", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/website/");
  await expect(page.locator("[data-start]")).toBeVisible();
  await expect(page.getByRole("link", { name: "GITHUB ↗" })).toBeVisible();
  await page.evaluate(() => { document.body.dataset.workspace = "1"; });
  await expect(page.locator("[data-mobile-panels]")).toBeVisible();
  await expect(page.locator('[data-window="editor"]')).toBeVisible();
  await expect(page.locator('[data-window="files"]')).toBeHidden();
  await page.locator('[data-mobile-panels] [data-focus-window="files"]').click();
  await expect(page.locator('[data-window="files"]')).toBeVisible();
  await expect(page.locator('[data-window="editor"]')).toBeHidden();
});

const builtRuntime = new URL("../../target/www/runtime/hara.wasm", import.meta.url);
const runtimeTest = existsSync(builtRuntime) ? test : test.skip;

runtimeTest("www package includes the Hara UI image assets", async ({ page }) => {
  await page.goto("/target/www/");
  await expect(page.getByRole("heading", { name: "HARA" })).toBeVisible();
  await expect(page.locator("img.welcome-mark")).toHaveCount(0);
  const marks = page.locator('img.system-mark[src*="hara-mark.svg"], img.start-mark[src*="hara-mark.svg"]');
  await expect(marks).toHaveCount(2);
  await expect
    .poll(() => marks.evaluateAll((images) => images.every((image) => image.complete && image.naturalWidth > 0)))
    .toBe(true);
});

runtimeTest("www runs workspace-discovered HAL background programs", async ({ page }) => {
  await page.addInitScript(() => localStorage.removeItem("hara-www.workspace.v1"));
  await page.goto("/target/www/");
  await expect(page.locator("[data-runtime-label]")).toHaveText("WASM // LIVE", { timeout: 60000 });
  await page.locator(".project-tab[data-home]").click();
  const canvas = page.locator("[data-tron]");
  const source = page.locator("select[data-background-source]");

  await expect(source.locator("option")).toHaveCount(4);
  await expect(source).toHaveValue("document/background/tron");
  await expect(canvas).toHaveAttribute("data-background-name", "tron");
  await expect(page.locator("[data-background-status]")).toContainText("GENERATION");
  await expect.poll(() => canvas.evaluate((node) => node.width * node.height)).toBeGreaterThan(0);

  await source.selectOption("document/background/grid");
  await expect(canvas).toHaveAttribute("data-background-name", "grid");
  await expect(page.locator("[data-background-status]")).toContainText("GENERATION");

  await source.selectOption("document/background/fire");
  await expect(canvas).toHaveAttribute("data-background-name", "fire");
  await expect(page.locator("[data-background-status]")).toContainText(/GENERATION|FALLBACK/);

  await source.selectOption("document/background/off");
  await expect(canvas).toHaveAttribute("data-background-name", "off");
  await expect(canvas).toBeVisible();
});

runtimeTest("live source errors roll back and explicit save uses the local overlay", async ({ page }) => {
  await page.addInitScript(() => localStorage.removeItem("hara-www.workspace.v1"));
  await page.goto("/target/www/");
  await expect(page.locator("[data-runtime-label]")).toHaveText("WASM // LIVE", { timeout: 60000 });
  await page.locator(".project-tab[data-home]").click();
  await page.locator("[data-source-toggle]").click();
  const editor = page.locator("[data-background-editor]");
  await expect(editor).toBeVisible();
  const goodSource = await editor.inputValue();
  await editor.fill("(ns+");
  await expect(page.locator("[data-background-status]")).toContainText("ERROR", { timeout: 10000 });
  await expect(page.locator("[data-tron]")).toBeVisible();
  await editor.fill(goodSource);
  await expect(page.locator("[data-background-status]")).toContainText("GENERATION", { timeout: 10000 });
  await page.locator("[data-background-save]").click();
  await expect(page.locator("[data-background-status]")).toContainText("SAVED");
});

runtimeTest("www evaluates the default Hara sketch into the canvas", async ({ page }) => {
  await page.goto("/target/www/");
  await expect(page.locator("[data-runtime-label]")).toHaveText("WASM // LIVE", { timeout: 60000 });
  await expect(page.locator("[data-editor-title]")).toContainText("NEON-ORBIT.HAL");
  await page.locator("[data-run]").click();
  await expect(page.locator("[data-canvas-empty]")).toHaveClass(/is-hidden/);
  await expect(page.locator("[data-canvas-status]")).toContainText("FRAME //");
  await expect(page.locator("[data-editor-status]")).toHaveText("FILE RENDERED");
});

runtimeTest("workspace template opens a dedicated project tab and survives Home navigation", async ({ page }) => {
  await page.goto("/target/www/");
  await expect(page.locator("[data-runtime-label]")).toHaveText("WASM // LIVE", { timeout: 60000 });
  await page.locator("[data-start]").click();
  await page.locator("[data-workspace-name]").fill(`Canvas ${Date.now()}`);
  await page.locator('[data-template="canvas"]').click();
  await expect(page.locator("body")).toHaveAttribute("data-workspace", "1");
  await expect(page.locator("[data-project-id]")).toHaveCount(1);
  await expect(page.locator('[data-file="/project.edn"]')).toBeVisible();
  await expect(page.locator('[data-file="/workspace.edn"]')).toBeVisible();
  await page.locator(".project-tab[data-home]").click();
  await expect(page.locator("body")).toHaveAttribute("data-workspace", "0");
  await expect(page.locator("[data-project-id]")).toHaveCount(1);
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

runtimeTest("www activates ns+ documents and reuses their private generation", async ({ page }) => {
  await page.goto("/target/www/");
  await expect(page.locator("[data-runtime-label]")).toHaveText("WASM // LIVE", { timeout: 60000 });

  await page.locator("[data-editor]").fill("(ns+)\n(def answer 41)\nanswer");
  await page.locator("[data-run]").click();
  await expect(page.locator("[data-inline-eval]")).toHaveText("=> 41");

  await page.locator("[data-editor]").fill("(+ answer 1)");
  await page.locator("[data-run]").click();
  await expect(page.locator("[data-inline-eval]")).toHaveText("=> 42");
});
