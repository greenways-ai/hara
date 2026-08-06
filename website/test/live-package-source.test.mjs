import assert from "node:assert/strict";
import { lstat, readFile, readlink } from "node:fs/promises";
import test from "node:test";

const alias = new URL("../packages/live", import.meta.url);

test("@hara-lang/live is sourced from the pinned hara-ui package", async () => {
  const metadata = await lstat(alias);
  assert.equal(metadata.isSymbolicLink(), true,
    "website/packages/live must remain an alias, not a copied implementation");
  assert.equal(await readlink(alias), "../vendor/hara-ui/packages/live");

  const packageJson = JSON.parse(await readFile(
    new URL("../packages/live/package.json", import.meta.url),
    "utf8"
  ));
  assert.equal(packageJson.name, "@hara-lang/live");
  assert.equal(packageJson.repository?.url, "https://github.com/hara-lang/hara-ui.git");
  assert.equal(packageJson.repository?.directory, "packages/live");
});
