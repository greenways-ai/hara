#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
} from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const source = path.join(root, "website", "vendor", "hara-ui");
const check = process.argv.slice(2).includes("--check");
const unexpected = process.argv.slice(2).filter((argument) => argument !== "--check");

if (unexpected.length) {
  console.error(`usage: node scripts/sync-hara-ui.mjs [--check]`);
  process.exit(2);
}

const revision = execFileSync("git", ["-C", source, "rev-parse", "HEAD"], {
  encoding: "utf8",
}).trim();
const repository = "https://github.com/hara-lang/hara-ui";
const targets = [
  {
    directory: path.join(root, "docs", "vendor", "hara-ui"),
    files: ["tokens.css", "components.css", "LICENSE"],
  },
  {
    directory: path.join(root, "specs", "vendor", "hara-ui"),
    files: [
      "tokens.css",
      "components.css",
      "spec-explorer.css",
      "spec-explorer.js",
      "logo-white.svg",
      "LICENSE",
    ],
  },
];

let failed = false;

for (const target of targets) {
  const provenance = `${JSON.stringify(
    { repository, commit: revision, files: target.files },
    null,
    2,
  )}\n`;
  const provenancePath = path.join(target.directory, "hara-ui.source.json");

  if (check) {
    for (const name of target.files) {
      const sourcePath = path.join(source, name);
      const targetPath = path.join(target.directory, name);
      if (
        !existsSync(targetPath) ||
        !readFileSync(sourcePath).equals(readFileSync(targetPath))
      ) {
        console.error(
          `stale Hara UI asset: ${path.relative(root, targetPath)}`,
        );
        failed = true;
      }
    }
    if (
      !existsSync(provenancePath) ||
      readFileSync(provenancePath, "utf8") !== provenance
    ) {
      console.error(
        `stale Hara UI provenance: ${path.relative(root, provenancePath)}`,
      );
      failed = true;
    }
    continue;
  }

  mkdirSync(target.directory, { recursive: true });
  for (const name of target.files) {
    copyFileSync(path.join(source, name), path.join(target.directory, name));
  }
  await import("node:fs/promises").then(({ writeFile }) =>
    writeFile(provenancePath, provenance),
  );
  console.log(
    `synced ${target.files.length} Hara UI files to ${path.relative(root, target.directory)}`,
  );
}

if (failed) {
  console.error("run: node scripts/sync-hara-ui.mjs");
  process.exit(1);
}

if (check) {
  console.log(`Hara UI snapshots match ${revision}`);
}
