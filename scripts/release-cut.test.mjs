import test from "node:test";
import assert from "node:assert/strict";
import { formatGitHubOutput, normalizeReleaseCut } from "./release-cut.mjs";

const valid = Object.freeze({
  schema: "hara-release-cut/v1",
  tag: "v0.1.2",
  version: "0.1.2",
  commit: "08322990fd1a26e3a004e7bf3459f45a25158311",
  workflow: "release.yml"
});

test("a reviewed release cut is normalized without changing its target", () => {
  assert.deepEqual(normalizeReleaseCut(valid), valid);
});

test("tag and version must identify the same immutable release", () => {
  assert.throws(
    () => normalizeReleaseCut({ ...valid, tag: "v0.1.3" }),
    /does not match version 0\.1\.2/
  );
  assert.throws(
    () => normalizeReleaseCut({ ...valid, version: "latest" }),
    /version is invalid/
  );
});

test("release cuts require a full commit rather than a movable branch", () => {
  for (const commit of ["main", "08322990", "g".repeat(40), ""]) {
    assert.throws(
      () => normalizeReleaseCut({ ...valid, commit }),
      /full 40-character SHA-1/
    );
  }
});

test("the workflow must be a top-level YAML file", () => {
  for (const workflow of ["../release.yml", ".github/workflows/release.yml", "release", "release.json"]) {
    assert.throws(
      () => normalizeReleaseCut({ ...valid, workflow }),
      /workflow is invalid/
    );
  }
});

test("unknown manifest fields are rejected", () => {
  assert.throws(
    () => normalizeReleaseCut({ ...valid, force: true }),
    /unknown fields: force/
  );
});

test("GitHub output contains only validated scalar fields", () => {
  assert.equal(formatGitHubOutput(valid), [
    "schema=hara-release-cut/v1",
    "tag=v0.1.2",
    "version=0.1.2",
    "commit=08322990fd1a26e3a004e7bf3459f45a25158311",
    "workflow=release.yml",
    ""
  ].join("\n"));
});
