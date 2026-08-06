import { cp, mkdir, readFile, readdir, rm, writeFile } from "node:fs/promises";
import { relative, resolve, sep } from "node:path";

async function listFiles(root, directory = root) {
  const entries = await readdir(directory, { withFileTypes: true });
  const output = [];
  for (const entry of entries) {
    const path = resolve(directory, entry.name);
    if (entry.isDirectory()) output.push(...await listFiles(root, path));
    else if (entry.isFile()) output.push(relative(root, path).split(sep).join("/"));
  }
  return output;
}

export async function packageDbProvider({ source, output, nodeBuild, browserBuild }) {
  await rm(output, { recursive: true, force: true });
  await mkdir(output, { recursive: true });
  await cp(resolve(source, "package.json"), resolve(output, "package.json"));
  await cp(nodeBuild, resolve(output, "node"), { recursive: true });
  await cp(browserBuild, resolve(output, "browser"), { recursive: true });

  const assets = (await listFiles(output))
    .filter(path => path !== "node/worker.mjs" && path !== "browser/worker.mjs")
    .sort();
  const template = await readFile(resolve(source, "hara.extension.edn"), "utf8");
  if (!template.includes(":assets []")) {
    throw new Error("database provider manifest must contain :assets [] placeholder");
  }
  const assetBlock = assets.length === 0
    ? ":assets []"
    : `:assets\n [${assets.map(path => JSON.stringify(path)).join("\n  ")}]`;
  await writeFile(
    resolve(output, "hara.extension.edn"),
    template.replace(":assets []", assetBlock),
    "utf8"
  );
  return { output, assets };
}
