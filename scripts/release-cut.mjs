#!/usr/bin/env node
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const SCHEMA = "hara-release-cut/v1";
const VERSION = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/;
const COMMIT = /^[0-9a-f]{40}$/;
const WORKFLOW = /^[A-Za-z0-9_.-]+\.ya?ml$/;

export function normalizeReleaseCut(value) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error("release cut must be an object");
  }
  const schema = String(value.schema ?? "").trim();
  const tag = String(value.tag ?? "").trim();
  const version = String(value.version ?? "").trim();
  const commit = String(value.commit ?? "").trim().toLowerCase();
  const workflow = String(value.workflow ?? "").trim();

  if (schema !== SCHEMA) throw new Error(`release cut schema must be ${SCHEMA}`);
  if (!VERSION.test(version)) throw new Error(`release cut version is invalid: ${version || "missing"}`);
  if (tag !== `v${version}`) throw new Error(`release cut tag ${tag || "missing"} does not match version ${version}`);
  if (!COMMIT.test(commit)) throw new Error("release cut commit must be a full 40-character SHA-1");
  if (!WORKFLOW.test(workflow) || workflow.includes("/") || workflow.includes("\\")) {
    throw new Error(`release cut workflow is invalid: ${workflow || "missing"}`);
  }

  const allowed = new Set(["schema", "tag", "version", "commit", "workflow"]);
  const unknown = Object.keys(value).filter((key) => !allowed.has(key));
  if (unknown.length) throw new Error(`release cut contains unknown fields: ${unknown.join(", ")}`);

  return Object.freeze({ schema, tag, version, commit, workflow });
}

export async function readReleaseCut(path) {
  let source;
  try {
    source = await readFile(path, "utf8");
  } catch (error) {
    throw new Error(`unable to read release cut ${path}: ${error.message}`, { cause: error });
  }
  let value;
  try {
    value = JSON.parse(source);
  } catch (error) {
    throw new Error(`release cut ${path} is not valid JSON: ${error.message}`, { cause: error });
  }
  return normalizeReleaseCut(value);
}

export function formatGitHubOutput(cut) {
  const value = normalizeReleaseCut(cut);
  return [
    `schema=${value.schema}`,
    `tag=${value.tag}`,
    `version=${value.version}`,
    `commit=${value.commit}`,
    `workflow=${value.workflow}`
  ].join("\n") + "\n";
}

async function main(argv) {
  const [command, manifestPath] = argv;
  if (!command || !manifestPath) usage();
  const cut = await readReleaseCut(resolve(manifestPath));
  if (command === "validate") {
    process.stdout.write(`${JSON.stringify(cut, null, 2)}\n`);
    return;
  }
  if (command === "github-output") {
    process.stdout.write(formatGitHubOutput(cut));
    return;
  }
  usage();
}

function usage() {
  throw new Error("usage: release-cut.mjs validate MANIFEST | github-output MANIFEST");
}

const invoked = process.argv[1]
  && import.meta.url === pathToFileURL(resolve(process.argv[1])).href;
if (invoked) {
  main(process.argv.slice(2)).catch((error) => {
    console.error(error.message || error);
    process.exit(1);
  });
}
