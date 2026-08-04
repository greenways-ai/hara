// Renders assets/motifs/source/*.svg to assets/motifs/master/*.png at
// 4096x4096 with headless Chromium (full SVG filter support).
// Dev-only tool. Usage:
//   node scripts/render-masters.mjs [--size 4096] [--executable /path/to/chrome-headless-shell]
// Playwright must be resolvable (e.g. run from a checkout that has it, or set
// PLAYWRIGHT_MODULE to an absolute playwright index.mjs path).
import { readdir, mkdir } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const args = process.argv.slice(2);
const opt = (name, fallback) => {
  const i = args.indexOf(`--${name}`);
  return i >= 0 ? args[i + 1] : fallback;
};
const size = Number(opt("size", "4096"));
const executablePath = opt("executable", process.env.CHROME_HEADLESS_SHELL);
const playwrightModule = process.env.PLAYWRIGHT_MODULE ?? "playwright";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const sourceDir = join(root, "assets", "motifs", "source");
const masterDir = join(root, "assets", "motifs", "master");

const { chromium } = await import(playwrightModule);
const browser = await chromium.launch(executablePath ? { executablePath } : {});
const page = await browser.newPage({ viewport: { width: size, height: size }, deviceScaleFactor: 1 });
await mkdir(masterDir, { recursive: true });
const files = (await readdir(sourceDir)).filter((file) => file.endsWith(".svg")).sort();
for (const file of files) {
  await page.goto(pathToFileURL(join(sourceDir, file)).href);
  await page.evaluate((s) => {
    const svg = document.querySelector("svg");
    svg.setAttribute("width", String(s));
    svg.setAttribute("height", String(s));
  }, size);
  await page.waitForTimeout(120);
  const out = join(masterDir, file.replace(".svg", ".png"));
  await page.screenshot({ path: out, omitBackground: false });
  console.log(`${file} -> ${out}`);
}
await browser.close();
