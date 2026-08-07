#!/usr/bin/env node
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(fileURLToPath(new URL("../..", import.meta.url)));
const expected = process.argv[2];
if (!/^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/.test(expected || "")) {
  throw new Error("usage: check-release-version.mjs VERSION");
}

const files = Object.fromEntries(await Promise.all([
  "core/rust/Cargo.toml",
  "core/rust/raw/Cargo.toml",
  "rust/compiler/Cargo.toml",
  "rust/vm-runtime/Cargo.toml",
  "rust/Cargo.lock",
  ".github/studio-runtime-release.json",
  ".github/workflows/publish-studio-runtime.yml",
  ".github/workflows/publish-rust-crates.yml",
  "scripts/build-www"
].map(async (path) => [path, await readFile(resolve(root, path), "utf8")])));

assertEqual(packageVersion(files["core/rust/Cargo.toml"]), expected, "core/rust/Cargo.toml package");
assertEqual(packageVersion(files["core/rust/raw/Cargo.toml"]), expected, "core/rust/raw/Cargo.toml package");
assertEqual(dependencyVersion(files["rust/compiler/Cargo.toml"], "hara-wasm"), expected,
  "hara-compiler hara-wasm dependency");
assertEqual(dependencyVersion(files["rust/vm-runtime/Cargo.toml"], "hara-wasm"), expected,
  "hara-vm hara-wasm dependency");
assertEqual(lockVersion(files["rust/Cargo.lock"], "hara-wasm"), expected,
  "Cargo.lock hara-wasm package");
assertEqual(lockVersion(files["rust/Cargo.lock"], "hara-wasm-raw"), expected,
  "Cargo.lock hara-wasm-raw package");

const studioRelease = JSON.parse(files[".github/studio-runtime-release.json"]);
assertEqual(studioRelease.tag, `v${expected}`, "Studio runtime release tag");
requireText(files[".github/workflows/publish-studio-runtime.yml"], `default: v${expected}`,
  "Studio publication workflow default");
requireText(files[".github/workflows/publish-rust-crates.yml"],
  `wait_for_crate hara-wasm ${expected}`, "crate publication visibility check");
requireText(files[".github/workflows/publish-rust-crates.yml"],
  `hara-wasm-${expected}.crate`, "crate publication archive name");
requireText(files["scripts/build-www"], "RUNTIME_VERSION=", "website runtime version derivation");
requireText(files["scripts/build-www"], '"$RUNTIME_VERSION"', "website manifest version argument");

const hardcodedRuntimeAsset = /hara-wasm-(?:core|vm|trace)-\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?\.wasm/;
if (hardcodedRuntimeAsset.test(files["scripts/build-www"])) {
  throw new Error("scripts/build-www contains a hard-coded versioned runtime filename");
}

console.log(`release version surfaces agree on ${expected}`);

function packageVersion(source) {
  return source.match(/^version\s*=\s*"([^"]+)"/m)?.[1] || null;
}

function dependencyVersion(source, name) {
  const escaped = name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  return source.match(new RegExp(`^${escaped}\\s*=\\s*\\{[^\\n]*version\\s*=\\s*"([^"]+)"`, "m"))?.[1] || null;
}

function lockVersion(source, name) {
  const escaped = name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  return source.match(new RegExp(`\\[\\[package\\]\\]\\nname = "${escaped}"\\nversion = "([^"]+)"`))?.[1] || null;
}

function assertEqual(actual, expectedValue, label) {
  if (actual !== expectedValue) {
    throw new Error(`${label} is ${JSON.stringify(actual)}; expected ${JSON.stringify(expectedValue)}`);
  }
}

function requireText(source, text, label) {
  if (!source.includes(text)) throw new Error(`${label} is missing ${JSON.stringify(text)}`);
}
