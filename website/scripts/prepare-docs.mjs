import { cp, mkdir, readFile, readdir, rm, writeFile } from "node:fs/promises";
import { basename, dirname, extname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const site = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const source = resolve(site, "../docs/docs");
const destination = resolve(site, "src/content/docs/docs");
const runtimeDestination = resolve(site, "public/docs-assets");

function titleFor(body, file) {
  const heading = body.match(/^#\s+(.+)$/m)?.[1]?.replace(/[`*_]/g, "").trim();
  if (heading) return heading;
  return basename(file, extname(file)).split(/[-_]/).map((word) =>
    word ? word[0].toUpperCase() + word.slice(1) : word).join(" ");
}

function hasTitle(body) {
  return body.startsWith("---\n") && /^title\s*:/m.test(body.slice(0, body.indexOf("\n---", 4)));
}

function normalizeMkDocsFrontmatter(body) {
  if (!body.startsWith("---\n")) return body;
  const end = body.indexOf("\n---", 4);
  if (end < 0) return body;
  const lines = body.slice(4, end).split("\n");
  const kept = [];
  let skippingList = false;
  for (const line of lines) {
    if (/^(template|hara_kernel_loading):/.test(line)) continue;
    if (/^hide:/.test(line)) { skippingList = true; continue; }
    if (skippingList && /^\s+-\s/.test(line)) continue;
    skippingList = false;
    kept.push(line);
  }
  return `---\n${kept.join("\n")}\n---${body.slice(end + 4)}`;
}

async function walk(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  for (const entry of entries) {
    const input = join(directory, entry.name);
    if (entry.isDirectory()) { await walk(input); continue; }
    if (!/\.mdx?$/.test(entry.name)) continue;
    const rel = relative(source, input);
    const output = join(destination, rel);
    let body = await readFile(input, "utf8");
    body = normalizeMkDocsFrontmatter(body);
    body = body.replaceAll("https://docs.hara-lang.org/", "/docs/");
    // Shiki does not yet ship a Hara grammar. Clojure is the closest reader
    // grammar and preserves useful highlighting until the dedicated grammar lands.
    body = body.replace(/^```(?:hara|hal)(\s+eval)?\s*$/gm, "```clojure$1");
    if (!hasTitle(body)) body = `---\ntitle: ${JSON.stringify(titleFor(body, entry.name))}\n---\n\n${body}`;
    const frontmatterEnd = body.indexOf("\n---", 4) + 4;
    const afterFrontmatter = body.slice(frontmatterEnd).replace(/^\s*#\s+[^\n]+\n+/, "\n");
    body = body.slice(0, frontmatterEnd) + afterFrontmatter;
    await mkdir(dirname(output), { recursive: true });
    await writeFile(output, body);
  }
}

await rm(destination, { recursive: true, force: true });
await mkdir(destination, { recursive: true });
await walk(source);

// Keep the documentation repository authoritative for its browser kernel while
// publishing it at the canonical /docs route through the Astro shell.
await rm(runtimeDestination, { recursive: true, force: true });
await mkdir(join(runtimeDestination, "javascripts"), { recursive: true });
await cp(resolve(source, "rust"), join(runtimeDestination, "rust"), { recursive: true });
await cp(resolve(source, "javascripts/kernel.js"), join(runtimeDestination, "javascripts/kernel.js"));
