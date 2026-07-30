#!/usr/bin/env node

import { createHash } from "node:crypto";
import { readFile, writeFile } from "node:fs/promises";

const [output, ...specs] = process.argv.slice(2);
if (!output || !specs.length) {
  throw new Error("usage: build-www-harp.mjs OUTPUT.harp NAMESPACE=FILE.hir...");
}

const encoder = new TextEncoder();
const files = [];
for (const spec of specs) {
  const separator = spec.indexOf("=");
  if (separator < 1) throw new Error(`invalid HIR resource: ${spec}`);
  const namespace = spec.slice(0, separator);
  const sourcePath = spec.slice(separator + 1);
  const path = `hir/${namespace.replaceAll(".", "/")}.hir`;
  const bytes = new Uint8Array(await readFile(sourcePath));
  if (new TextDecoder("latin1").decode(bytes.subarray(0, 4)) !== "HIR\0") {
    throw new Error(`${sourcePath} is not a HIR artifact`);
  }
  files.push({ namespace, path, bytes });
}

const tree = [];
const declarations = [];
const resources = [];
for (const { namespace, path, bytes } of files) {
  tree.push(encoder.encode(path), Uint8Array.of(0), bytes);
  declarations.push(`  ${JSON.stringify(path)} {:sha256 "sha256:${sha256(bytes)}" :size ${bytes.length}}`);
  resources.push(`  ${JSON.stringify(namespace)} ${JSON.stringify(path)}`);
}
const manifest = encoder.encode(
  `{:harp/format 1\n`
  + ` :package {:identity "hara/www-kernel" :version "1"}\n`
  + ` :files {\n${declarations.join("\n")}\n}`
  + ` :resources {\n${resources.join("\n")}\n}`
  + ` :extensions []\n`
  + ` :integrity {:tree-sha256 "sha256:${sha256(concat(tree))}"}}\n`
);

await writeFile(output, storedZip([
  { path: "package.edn", bytes: manifest },
  ...files.map(({ path, bytes }) => ({ path, bytes }))
]));

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function storedZip(entries) {
  const local = [];
  const central = [];
  let offset = 0;
  for (const { path, bytes } of entries) {
    const name = encoder.encode(path);
    const checksum = crc32(bytes);
    const localHeader = new Uint8Array(30);
    write32(localHeader, 0, 0x04034b50);
    write16(localHeader, 4, 20);
    write32(localHeader, 14, checksum);
    write32(localHeader, 18, bytes.length);
    write32(localHeader, 22, bytes.length);
    write16(localHeader, 26, name.length);
    local.push(localHeader, name, bytes);

    const centralHeader = new Uint8Array(46);
    write32(centralHeader, 0, 0x02014b50);
    write16(centralHeader, 4, 20);
    write16(centralHeader, 6, 20);
    write32(centralHeader, 16, checksum);
    write32(centralHeader, 20, bytes.length);
    write32(centralHeader, 24, bytes.length);
    write16(centralHeader, 28, name.length);
    write32(centralHeader, 42, offset);
    central.push(centralHeader, name);
    offset += localHeader.length + name.length + bytes.length;
  }
  const localBytes = concat(local);
  const centralBytes = concat(central);
  const end = new Uint8Array(22);
  write32(end, 0, 0x06054b50);
  write16(end, 8, entries.length);
  write16(end, 10, entries.length);
  write32(end, 12, centralBytes.length);
  write32(end, 16, localBytes.length);
  return concat([localBytes, centralBytes, end]);
}

function crc32(bytes) {
  let value = 0xffffffff;
  for (const byte of bytes) {
    value ^= byte;
    for (let bit = 0; bit < 8; bit++) {
      value = (value >>> 1) ^ (0xedb88320 & -(value & 1));
    }
  }
  return (value ^ 0xffffffff) >>> 0;
}

function concat(parts) {
  const output = new Uint8Array(parts.reduce((size, part) => size + part.length, 0));
  let offset = 0;
  for (const part of parts) {
    output.set(part, offset);
    offset += part.length;
  }
  return output;
}

function write16(bytes, offset, value) {
  new DataView(bytes.buffer).setUint16(offset, value, true);
}

function write32(bytes, offset, value) {
  new DataView(bytes.buffer).setUint32(offset, value, true);
}
