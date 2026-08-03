import assert from "node:assert/strict";
import test from "node:test";
import { zipSync } from "fflate";
import {
  installLockedPackages,
  loadLockedPackageResources
} from "./src/packages.js";

const encoder = new TextEncoder();

function hex(bytes) {
  return [...bytes].map((byte) => byte.toString(16).padStart(2, "0")).join("");
}

async function digest(bytes) {
  return `sha256:${hex(new Uint8Array(await crypto.subtle.digest("SHA-256", bytes)))}`;
}

async function fixture() {
  const source = encoder.encode("(ns demo.world) (def world {:title \"Demo\"})");
  const sourceDigest = await digest(source);
  const manifest = encoder.encode(
    `{:files {"src/demo/world.hal" {:size ${source.byteLength} :sha256 "${sourceDigest}"}} `
      + `:resources {"demo.world" "src/demo/world.hal"}}`
  );
  const archive = zipSync({
    "package.edn": manifest,
    "src/demo/world.hal": source
  });
  const archiveDigest = await digest(archive);
  const lock = `{:lock/format 2 :packages {"demo:world" `
    + `{:packages/url "https://packages.example/demo.harp" `
    + `:release-url "https://github.example/demo.harp" `
    + `:size ${archive.byteLength} :harp-sha256 "${archiveDigest}"}}}`;
  return { archive, lock };
}

test("format-2 locks prefer packages.* and verify HARP resources", async () => {
  const { archive, lock } = await fixture();
  const requested = [];
  const resources = await loadLockedPackageResources(lock, async (url) => {
    requested.push(url);
    return new Response(archive);
  });

  assert.deepEqual(requested, ["https://packages.example/demo.harp"]);
  assert.equal(resources["demo.world"], "(ns demo.world) (def world {:title \"Demo\"})");
});

test("installation is atomic when a locked archive fails verification", async () => {
  const { archive, lock } = await fixture();
  const registered = [];
  const runtime = {
    registerResource(namespace, source) {
      registered.push([namespace, source]);
    }
  };
  const corrupt = archive.slice();
  corrupt[corrupt.length - 1] ^= 1;

  await assert.rejects(
    installLockedPackages(runtime, lock, { fetch: async () => new Response(corrupt) }),
    /digest mismatch/
  );
  assert.deepEqual(registered, []);
});
