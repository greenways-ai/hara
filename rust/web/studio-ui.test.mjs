import assert from "node:assert/strict";
import test from "node:test";

import { HtaKeyword, HtaSymbol } from "./hta.js";
import {
  buildTree,
  defaultFileContent,
  importGithubSource,
  normalizeNewFilePath,
  normalizePath,
  parseGithubSpec,
  renderValue,
  studioSource
} from "./studio/ui.js";

// Logic-level tests for the DOM-free parts of rust/web/studio/ui.js: value
// rendering, path normalization, file-tree building, GitHub import spec
// parsing, and hara source builders. No DOM needed.

test("renderValue renders scalars like the hara-chrome RESP client", () => {
  assert.equal(renderValue(null), "nil");
  assert.equal(renderValue(undefined), "nil");
  assert.equal(renderValue(3), "3");
  assert.equal(renderValue(42n), "42");
  assert.equal(renderValue(true), "true");
  assert.equal(renderValue("plain"), "plain");
});

test("renderValue renders keywords, collections, and nesting", () => {
  assert.equal(renderValue(new HtaKeyword("space")), ":space");
  assert.equal(renderValue([1, 2, 3]), "[1 2 3]");
  assert.equal(renderValue(new Set([1, 2])), "#{1 2}");
  assert.equal(
    renderValue(new Map([[new HtaKeyword("a"), 1], [new HtaKeyword("b"), [1, 2]]])),
    "{:a 1, :b [1 2]}"
  );
  assert.equal(renderValue([new HtaKeyword("ok"), null, ["x"]]), "[:ok nil [x]]");
});

test("normalizePath anchors, collapses, and strips", () => {
  assert.equal(normalizePath("intro.hal"), "/intro.hal");
  assert.equal(normalizePath("/docs/day1.hal"), "/docs/day1.hal");
  assert.equal(normalizePath("  docs//day1.hal "), "/docs/day1.hal");
  assert.equal(normalizePath("/docs/"), "/docs");
});

test("normalizePath rejects empty, root, and parent-escaping paths", () => {
  assert.equal(normalizePath(""), null);
  assert.equal(normalizePath("   "), null);
  assert.equal(normalizePath("/"), null);
  assert.equal(normalizePath("//"), null);
  assert.equal(normalizePath("../secret"), null);
  assert.equal(normalizePath("/a/../b"), null);
  assert.equal(normalizePath(42), null);
});

test("new files receive .hal unless an extension was explicitly supplied", () => {
  assert.equal(normalizeNewFilePath("src/player"), "/src/player.hal");
  assert.equal(normalizeNewFilePath("src/player.hal"), "/src/player.hal");
  assert.equal(normalizeNewFilePath("workspace.edn"), "/workspace.edn");
  assert.equal(normalizeNewFilePath("../escape"), null);
});

test("buildTree returns an empty forest for no paths", () => {
  assert.deepEqual(buildTree([]), []);
  assert.deepEqual(buildTree(null), []);
});

test("buildTree sorts directories before files, alphabetically", () => {
  const tree = buildTree(["/zeta.hal", "/docs/day1.hal", "/alpha.hal", "/docs/advanced.hal"]);
  assert.deepEqual(tree, [
    {
      name: "docs",
      path: "/docs",
      directory: true,
      children: [
        { name: "advanced.hal", path: "/docs/advanced.hal", directory: false },
        { name: "day1.hal", path: "/docs/day1.hal", directory: false }
      ]
    },
    { name: "alpha.hal", path: "/alpha.hal", directory: false },
    { name: "zeta.hal", path: "/zeta.hal", directory: false }
  ]);
});

test("buildTree nests intermediate directories and skips junk paths", () => {
  const tree = buildTree(["/a/b/c.hal", "", "../bad"]);
  assert.deepEqual(tree, [
    {
      name: "a",
      path: "/a",
      directory: true,
      children: [
        {
          name: "b",
          path: "/a/b",
          directory: true,
          children: [{ name: "c.hal", path: "/a/b/c.hal", directory: false }]
        }
      ]
    }
  ]);
});

test("parseGithubSpec parses owner/repo with optional ref", () => {
  assert.deepEqual(parseGithubSpec("octo/lessons"), { repo: "octo/lessons", ref: "main", space: "lessons" });
  assert.deepEqual(parseGithubSpec("octo/lessons@dev"), { repo: "octo/lessons", ref: "dev", space: "lessons" });
  assert.deepEqual(parseGithubSpec("  octo/my.repo@v1.2  "), { repo: "octo/my.repo", ref: "v1.2", space: "my.repo" });
});

test("parseGithubSpec rejects malformed specs", () => {
  for (const bad of ["", "  ", "repo", "owner/", "/repo", "owner/repo/extra", "owner repo", "owner/repo@", 42, null]) {
    assert.equal(parseGithubSpec(bad), null, `expected null for ${JSON.stringify(bad)}`);
  }
});

test("importGithubSource emits an UNQUOTED require vector and escaped args", () => {
  const source = importGithubSource({ space: "lessons", repo: "octo/lessons", ref: "main" });
  assert.equal(
    source,
    '(do (require [studio.space :as space]) (space/import-github! "lessons" "octo/lessons" {:ref "main"}))'
  );
  assert.ok(!source.includes("(require '"), "require vector must stay unquoted");
});

test("studioSource wraps a form with unquoted studio requires", () => {
  assert.equal(
    studioSource('(fs/read "home" "/a.hal")'),
    '(do (require [studio.space :as space]) (require [studio.fs :as fs]) (require [studio.boot :as boot]) (fs/read "home" "/a.hal"))'
  );
  assert.ok(!studioSource("x").includes("(require '"));
});

test("defaultFileContent seeds a fresh file", () => {
  assert.equal(defaultFileContent("/scratch.hal"), ";; /scratch.hal\n\n(ns user)\n");
});

test("HtaSymbol values render as their name", () => {
  assert.equal(renderValue(new HtaSymbol("foo")), "foo");
});
