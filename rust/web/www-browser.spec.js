import { existsSync } from "node:fs";
import { expect, test } from "@playwright/test";

test("www opens on Home with external site navigation", async ({ page }) => {
  test.setTimeout(90_000);
  await page.addInitScript(() => localStorage.setItem("hara-www.workspace.v1", "1"));
  await page.goto("/target/www/");
  await expect(page.locator("body")).toHaveAttribute("data-workspace", "0");
  await expect(page.locator(".system-bar")).toHaveCount(0);
  await expect(page.getByRole("link", { name: "GITHUB ↗" })).toHaveCount(0);
  await expect(page.getByRole("link", { name: "YOUTUBE ↗" })).toHaveCount(0);
  await expect(page.locator(".system-bottom-bar [data-launcher-toggle]")).toBeVisible();
  await expect(page.locator(".system-bottom-bar [data-workspace-prev]")).toBeVisible();
  await expect(page.locator(".system-bottom-bar [data-workspace-next]")).toBeVisible();
  await expect(page.locator(".system-bottom-bar [data-runtime-toggle]")).toBeVisible();
  await expect(page.getByRole("button", { name: "START" })).toBeVisible({
    timeout: 60_000
  });
  await expect(page.locator("[data-workspace-prev]")).toBeDisabled();
  await expect(page.locator("[data-workspace-next]")).toBeEnabled();
  await expect(page.locator("[data-background-picker]")).toBeVisible();
  await expect(page.locator(".desktop-workspace")).toBeHidden();

  await page.locator("[data-launcher-toggle]").click();
  await expect(page.locator("[data-launcher]")).toHaveAttribute("aria-hidden", "false");
  const siteLinks = page.locator(".site-launcher-links a");
  await expect(siteLinks).toHaveCount(5);
  await expect.poll(() => siteLinks.evaluateAll((links) =>
    links.every((link) =>
      link.getAttribute("target") === "_blank"
      && link.getAttribute("rel") === "noopener noreferrer"
    )
  )).toBe(true);
  const launcherBox = await page.locator("[data-launcher]").boundingBox();
  expect(launcherBox.width).toBeLessThanOrEqual(560);
  await expect(page.getByRole("link", { name: "PLAYGROUND" })).toBeVisible();
  await expect(page.getByRole("link", { name: "DOCS" })).toBeVisible();
  await expect(page.getByRole("link", { name: "SPECS" })).toBeVisible();
  await expect(page.getByRole("link", { name: "GITHUB" })).toBeVisible();
  await expect(page.getByRole("link", { name: "YOUTUBE" })).toBeVisible();
  await expect(page.locator("[data-new-workspace]")).not.toBeVisible();
  await expect(page.locator("[data-github-account]")).not.toBeVisible();
  await expect(page.locator("[data-ai-adapters]")).not.toBeVisible();
  await expect(page.locator("[data-settings]")).toHaveCount(0);
  await expect(page.locator("[data-help]")).toHaveCount(0);
});

test("zoomed desktop opens on the Home screen", async ({ page }) => {
  test.setTimeout(90_000);
  await page.setViewportSize({ width: 892, height: 900 });
  await page.goto("/target/www/");
  await expect(page.locator(".welcome-workspace")).toBeVisible();
  await expect(page.locator(".desktop-workspace")).toBeHidden();
  await expect(page.locator("[data-workspace-prev]")).toBeDisabled();
  await expect(page.locator("[data-workspace-next]")).toBeEnabled();
});

test("phone shell keeps the single-screen controls unobscured", async ({ page }) => {
  test.setTimeout(90_000);
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/target/www/");
  await expect(page.locator("[data-start]")).toBeVisible({ timeout: 60_000 });
  await expect(page.locator(".system-bar")).toHaveCount(0);
  await expect(page.locator(".system-bottom-bar [data-launcher-toggle]")).toBeVisible();
  await expect(page.locator(".system-bottom-bar [data-workspace-prev]")).toBeVisible();
  await expect(page.locator(".system-bottom-bar [data-workspace-next]")).toBeVisible();
  await expect(page.locator(".system-bottom-bar [data-runtime-toggle]")).toBeVisible();
  await expect(page.locator("[data-background-picker]")).toBeVisible();
  await expect(page.locator(".desktop-workspace")).toBeHidden();
  await expect(page.locator("[data-workspace-prev]")).toBeDisabled();
  await expect(page.locator("[data-workspace-next]")).toBeEnabled();
});

test("canonical Hara assets and compact Start button survive responsive widths", async ({ page }) => {
  test.setTimeout(90_000);
  await page.setViewportSize({ width: 600, height: 900 });
  await page.goto("/target/www/");
  await expect(page.locator('link[rel="icon"]')).toHaveAttribute("href", "./assets/hara-favicon.svg?v=3");
  await expect(page.locator(".system-mark")).toHaveCount(0);
  await expect(page.locator(".launcher-mark")).toHaveCount(0);
  await expect(page.locator(".app-launcher-glyph i")).toHaveCount(9);
  await expect(page.getByRole("button", { name: "START" })).toBeVisible({
    timeout: 60_000
  });
  const tablet = await page.locator(".start-button").boundingBox();
  expect(tablet.width).toBeLessThan(400);

  await page.setViewportSize({ width: 390, height: 844 });
  const extraSmall = await page.locator(".start-button").boundingBox();
  expect(extraSmall.width).toBeLessThan(220);
});


test("kernel loader does not move the hero callout", async ({ page }) => {
  test.setTimeout(90_000);
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/target/www/");
  const loader = page.locator("[data-kernel-loading]");
  const callout = page.locator(".hero-callout");
  await loader.evaluate((node) => { node.hidden = true; });
  const before = await callout.boundingBox();
  await loader.evaluate((node) => { node.hidden = false; });
  const during = await callout.boundingBox();
  await loader.evaluate((node) => { node.hidden = true; });
  const after = await callout.boundingBox();
  expect(Math.abs(during.y - before.y)).toBeLessThan(0.5);
  expect(Math.abs(after.y - before.y)).toBeLessThan(0.5);
});

test("mobile editor toolbar uses icons instead of visible text", async ({ page }) => {
  test.setTimeout(90_000);
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/target/www/");
  const styles = await page.locator(".editor-toolbar").evaluate((toolbar) => {
    const paredit = toolbar.querySelector("[data-paredit]");
    const save = toolbar.querySelector("[data-save]");
    const runIcon = toolbar.querySelector("[data-run] > span");
    return {
      pareditFontSize: getComputedStyle(paredit).fontSize,
      pareditIcon: getComputedStyle(paredit, "::before").content,
      saveFontSize: getComputedStyle(save).fontSize,
      saveIcon: getComputedStyle(save, "::before").content,
      runIconSize: parseFloat(getComputedStyle(runIcon).fontSize)
    };
  });
  expect(styles.pareditFontSize).toBe("0px");
  expect(styles.pareditIcon).toContain("()");
  expect(styles.saveFontSize).toBe("0px");
  expect(styles.saveIcon).toContain("⇩");
  expect(styles.runIconSize).toBeGreaterThan(0);
});

const builtRuntime = new URL("../../target/www/runtime/hara.wasm", import.meta.url);
const runtimeTest = process.env.HARA_TEST_WWW_RUNTIME === "1" && existsSync(builtRuntime)
  ? test
  : test.skip;

runtimeTest("www package includes the Hara UI image assets", async ({ page }) => {
  await page.goto("/target/www/");
  await expect(page.getByRole("heading", { name: "HARA" })).toBeVisible();
  await expect(page.locator("img.welcome-mark")).toHaveCount(0);
  await expect(page.locator("img.system-mark")).toHaveCount(0);
  await expect(page.locator('link[rel="icon"]')).toHaveAttribute("href", "./assets/hara-favicon.svg?v=3");
  const marks = page.locator('img.start-mark[src*="logo-white.svg"]');
  await expect(marks).toHaveCount(1);
  await expect
    .poll(() => marks.evaluateAll((images) => images.every((image) => image.complete && image.naturalWidth > 0)))
    .toBe(true);
});

runtimeTest("kernel indicator opens comprehensive live telemetry on desktop and mobile", async ({ page }) => {
  await page.goto("/target/www/");
  await expect(page.locator("[data-runtime-label]")).toHaveText("WASM // LIVE", { timeout: 60000 });
  const toggle = page.locator("[data-runtime-toggle]");
  await expect(toggle).toHaveAttribute("aria-expanded", "false");
  await toggle.click();
  const panel = page.locator("[data-kernel-statistics]");
  await expect(panel).toBeVisible();
  await expect(toggle).toHaveAttribute("aria-expanded", "true");
  for (const section of ["RUNTIME", "KERNELS", "SESSIONS", "TRAFFIC", "CAPABILITIES", "STORAGE"]) {
    await expect(panel.getByRole("heading", { name: section })).toBeVisible();
  }
  await expect(panel).toContainText("Uptime");
  await expect(panel).toContainText("Eval requests");
  await expect(panel).toContainText("Frames rendered");
  await expect(panel).toContainText("Render rate");
  await expect(panel).toContainText("Session messages");
  await expect(panel).toContainText("DEDICATED KERNEL");
  const statistic = (label) => panel.locator("dt", { hasText: label }).locator("xpath=following-sibling::dd[1]");
  await expect.poll(async () => Number(await statistic("Frames rendered").textContent())).toBeGreaterThan(10);
  await expect.poll(async () => Number((await statistic("Render rate").textContent()).replace(" FPS", "")))
    .toBeGreaterThan(0);

  await page.setViewportSize({ width: 390, height: 844 });
  const box = await panel.boundingBox();
  expect(box.x).toBe(0);
  expect(Math.round(box.width)).toBe(390);
  await page.keyboard.press("Escape");
  await expect(panel).toBeHidden();
});

runtimeTest("www runs workspace-discovered HAL background programs", async ({ page }) => {
  await page.addInitScript(() => localStorage.removeItem("hara-www.workspace.v1"));
  await page.goto("/target/www/");
  await expect(page.locator("[data-runtime-label]")).toHaveText("WASM // LIVE", { timeout: 60000 });
  await page.locator(".project-tab[data-home]").click();
  const canvas = page.locator("[data-tron]");
  const source = page.locator("select[data-background-source]");

  await expect(source.locator("option")).toHaveCount(22);
  await expect(source.locator("optgroup")).toHaveCount(5);
  await expect(source.locator("optgroup").evaluateAll((groups) => groups.map((group) => group.label)))
    .resolves.toEqual(["BASIC", "NATURE", "SHAPES", "SIMULATION", "CONTROLS"]);
  await expect(source.locator("option").evaluateAll((options) => options.every((option) => !option.textContent.includes(".HAL"))))
    .resolves.toBe(true);
  await expect.poll(() => source.locator("option:checked").evaluate(
    (option) => option.parentElement.label
  )).toMatch(/^(BASIC|NATURE|SHAPES)$/);
  await expect(page.locator("[data-background-status]")).toContainText("GENERATION");
  await expect.poll(() => canvas.evaluate((node) => node.width * node.height)).toBeGreaterThan(0);

  await expect(source.locator('option[value="document/background/grid"]')).toHaveCount(0);
  await source.selectOption("document/background/aurora");
  await expect(canvas).toHaveAttribute("data-background-name", "aurora");
  await expect(page.locator("[data-background-status]")).toContainText("GENERATION");

  await source.selectOption("document/background/pulse");
  await expect(canvas).toHaveAttribute("data-background-name", "pulse");
  await expect(page.locator("body")).toHaveAttribute("data-background-name", "pulse");
  await expect(page.locator("[data-background-status]")).toContainText("GENERATION");

  for (const effect of [
    "stars",
    "crt-patterns",
    "subterranean",
    "octagrams",
    "mirror-fog",
    "ocean",
    "plasma-storm",
    "universe-within",
    "audio-tunnel",
    "sunlit-landscape",
    "desert-sand",
    "rain-ripples",
    "boids",
    "double-pendulum",
    "ants",
    "space-invaders",
    "pong"
  ]) {
    await source.selectOption(`document/background/${effect}`);
    await expect(canvas).toHaveAttribute("data-background-name", effect.replaceAll("-", " "));
    await expect(page.locator("[data-background-status]")).toContainText("GENERATION");
  }

  await source.selectOption("document/background/fire");
  await expect(canvas).toHaveAttribute("data-background-name", "fire");
  await expect(page.locator("[data-background-status]")).toContainText(/GENERATION|FALLBACK/);

  await source.selectOption("document/background/off");
  await expect(canvas).toHaveAttribute("data-background-name", "off");
  await expect(canvas).toBeVisible();
});

runtimeTest("desktop background picker opens upward and selects an effect", async ({ page }) => {
  await page.goto("/target/www/");
  await expect(page.locator("[data-runtime-label]")).toHaveText("WASM // LIVE", { timeout: 60000 });
  const toggle = page.locator("[data-background-menu-toggle]");
  const menu = page.locator("[data-background-menu]");
  await toggle.click();
  await expect(menu).toBeVisible();
  await expect(toggle).toHaveAttribute("aria-expanded", "true");
  const toggleBox = await toggle.boundingBox();
  const menuBox = await menu.boundingBox();
  expect(menuBox.y + menuBox.height).toBeLessThanOrEqual(toggleBox.y + 1);
  await menu.locator('[data-background-menu-item="document/background/aurora"]').click();
  await expect(menu).toBeHidden();
  await expect(page.locator("[data-background-source]")).toHaveValue("document/background/aurora");
  await expect(toggle).toContainText("AURORA");
});

runtimeTest("Retina WebGL backgrounds cover the full canvas backing store", async ({ page }) => {
  await page.goto("/target/www/");
  await expect(page.locator("[data-runtime-label]")).toHaveText("WASM // LIVE", { timeout: 60000 });
  const source = page.locator("[data-background-source]");
  const canvas = page.locator("[data-tron]");
  await source.selectOption("document/background/desert-sand");
  await expect(canvas).toHaveAttribute("data-background-name", "desert sand");
  const geometry = await canvas.evaluate((node) => ({
    cssWidth: node.clientWidth,
    cssHeight: node.clientHeight,
    pixelWidth: node.width,
    pixelHeight: node.height,
    ratio: devicePixelRatio
  }));
  expect(geometry.pixelWidth).toBe(Math.round(geometry.cssWidth * Math.min(2, geometry.ratio)));
  expect(geometry.pixelHeight).toBe(Math.round(geometry.cssHeight * Math.min(2, geometry.ratio)));
  const rightPixel = await canvas.evaluate((node) => {
    const context = node.getContext("2d");
    return [...context.getImageData(node.width - 4, Math.floor(node.height * 0.25), 1, 1).data];
  });
  expect(rightPixel[3]).toBe(255);
  expect(rightPixel[0] + rightPixel[1] + rightPixel[2]).toBeGreaterThan(20);
});

runtimeTest("mobile games fill the viewport and grouped source labels retain their hierarchy", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/target/www/");
  await expect(page.locator("[data-runtime-label]")).toHaveText("WASM // LIVE", { timeout: 60000 });
  const source = page.locator("[data-background-source]");
  const canvas = page.locator("[data-tron]");
  await expect(source).toBeVisible();

  const groupStyle = await source.locator("optgroup").first().evaluate((group) => {
    const groupStyle = getComputedStyle(group);
    const optionStyle = getComputedStyle(group.querySelector("option"));
    return { groupColor: groupStyle.color, groupSize: parseFloat(groupStyle.fontSize), optionSize: parseFloat(optionStyle.fontSize) };
  });
  expect(groupStyle.groupColor).toBe("rgb(156, 123, 255)");
  expect(groupStyle.groupSize).toBeLessThan(groupStyle.optionSize);

  for (const game of ["space-invaders", "pong"]) {
    await source.selectOption(`document/background/${game}`);
    await expect(page.locator("body")).toHaveAttribute("data-background-name", game);
    const bounds = await canvas.boundingBox();
    expect(Math.round(bounds.width)).toBe(390);
    expect(Math.round(bounds.height)).toBe(844);
    const titleOpacity = Number(await page.locator(".welcome-copy h1").evaluate((node) => getComputedStyle(node).opacity));
    expect(titleOpacity).toBeLessThan(0.5);
  }

  const sources = await page.evaluate(async () => ({
    pulse: await (await fetch("./sources/pulse.hal")).text(),
    tron: await (await fetch("./sources/tron.hal")).text(),
    invaders: await (await fetch("./sources/space-invaders.hal")).text()
  }));
  expect(sources.pulse).toContain("distanceFromCorner");
  expect(sources.tron).toContain("probe-spacing");
  expect(sources.tron).toContain("choose-direction");
  expect(sources.invaders).toContain(":width width :height height :left 0 :top 0");
});

runtimeTest("live source errors roll back and explicit save uses the local overlay", async ({ page }) => {
  await page.addInitScript(() => localStorage.removeItem("hara-www.workspace.v1"));
  await page.goto("/target/www/");
  await expect(page.locator("[data-runtime-label]")).toHaveText("WASM // LIVE", { timeout: 60000 });
  await page.locator(".project-tab[data-home]").click();
  await page.locator("[data-source-toggle]").click();
  const editor = page.locator("[data-background-editor]");
  await expect(editor).toBeVisible();
  await expect.poll(() => page.locator("[data-tron]").evaluate((canvas) => {
    const bounds = canvas.getBoundingClientRect();
    return {
      left: Math.round(bounds.left),
      top: Math.round(bounds.top),
      width: Math.round(bounds.width),
      height: Math.round(bounds.height)
    };
  })).toEqual({ left: 0, top: 0, width: 1280, height: 720 });
  await expect(page.locator("[data-background-line-numbers]")).toBeVisible();
  await expect(page.locator("[data-background-paredit]")).toHaveText("PAREDIT ON");
  await expect(page.locator("[data-background-apply]")).toBeVisible();
  await expect(page.locator("[data-background-highlight]")).toHaveCSS("overflow", "hidden");
  const editorFontSize = () => editor.evaluate((input) => getComputedStyle(input).fontSize);
  const lineNumberFontSize = () => page.locator("[data-background-line-numbers]")
    .evaluate((gutter) => getComputedStyle(gutter).fontSize);
  const initialEditorFontSize = await editorFontSize();
  await expect.poll(lineNumberFontSize).toBe(initialEditorFontSize);
  await page.locator("[data-background-font-increase]").click();
  await expect.poll(editorFontSize).not.toBe(initialEditorFontSize);
  await expect.poll(lineNumberFontSize).toBe(await editorFontSize());
  const goodSource = await editor.inputValue();
  await editor.evaluate((input) => {
    input.scrollTop = input.scrollHeight;
    input.scrollLeft = input.scrollWidth;
    input.dispatchEvent(new Event("scroll"));
  });
  await expect.poll(() => page.locator("[data-background-highlight]").evaluate((highlight) => ({
    top: Math.round(highlight.scrollTop),
    left: Math.round(highlight.scrollLeft),
    text: highlight.textContent
  }))).toEqual(expect.objectContaining({
    top: expect.any(Number),
    left: expect.any(Number),
    text: expect.stringContaining("(node/start")
  }));
  await expect.poll(() => page.locator("[data-background-highlight]").evaluate((highlight) => {
    const editor = document.querySelector("[data-background-editor]");
    const content = highlight.querySelector(".code-highlight-content");
    const transform = new DOMMatrix(getComputedStyle(content).transform);
    return {
      hasVerticalScroll: editor.scrollTop > 0,
      topMatches: Math.abs(transform.m42 + editor.scrollTop) < 1,
      leftMatches: Math.abs(transform.m41 + editor.scrollLeft) < 1
    };
  })).toEqual({
    hasVerticalScroll: true,
    topMatches: true,
    leftMatches: true
  });
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
  await expect(page.locator("[data-kernel-loading]")).toBeVisible();
  await expect(page.locator("[data-kernel-loading]")).toContainText("KERNEL LOADING");
  await expect(page.locator("body")).toHaveAttribute("data-workspace", "1");
  await expect(page.locator("[data-project-id]")).toHaveCount(1);
  await expect(page.locator('[data-file="/project.edn"]')).toBeVisible();
  await expect(page.locator('[data-file="/workspace.edn"]')).toBeVisible();
  await expect(page.locator("body")).toHaveAttribute("data-kernel", "live");
  await page.locator(".project-tab[data-home]").click();
  await expect(page.locator("body")).toHaveAttribute("data-workspace", "0");
  await expect(page.locator("body")).toHaveAttribute("data-kernel", "stopped");
  await expect(page.locator("[data-project-id]")).toHaveCount(1);
  await page.locator("[data-project-id]").click();
  await expect(page.locator("body")).toHaveAttribute("data-workspace", "1");
  await expect(page.locator("body")).toHaveAttribute("data-kernel", "live");
  await expect(page.locator('[data-file="/src/main.hal"]')).toBeVisible();
  await page.locator("[data-launcher-toggle]").click();
  await expect(page.locator("[data-close-active-workspace]")).toBeVisible();
  await page.locator("[data-close-active-workspace]").click();
  await expect(page.locator("body")).toHaveAttribute("data-workspace", "0");
  await expect(page.locator("[data-project-id]")).toHaveCount(0);
  await page.locator("[data-launcher-toggle]").click();
  await expect(page.locator("[data-saved-workspace-id]")).toHaveCount(1);
  page.once("dialog", (dialog) => dialog.accept());
  await page.locator("[data-saved-workspace-id] .saved-workspace-delete").click();
  await expect(page.locator("[data-saved-workspace-id]")).toHaveCount(0);
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
