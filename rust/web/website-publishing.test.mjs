import assert from "node:assert/strict";
import test from "node:test";

import { GistPublisher, GreenwaysPublisher, workspaceBundle } from "../../website/publishing.js";

const repository = {
  get: async () => ({ id: "demo", name: "Demo", template: "canvas" }),
  files: async () => new Map([
    ["/workspace.edn", "{:hara/type :workspace}"],
    ["/src/main.hal", "(+ 19 23)"]
  ])
};

test("publish bundle includes manifests and project files", async () => {
  const bundle = await workspaceBundle(repository, "demo");
  assert.equal(bundle.workspace.id, "demo");
  assert.equal(bundle.files["/src/main.hal"], "(+ 19 23)");
  assert.ok(bundle.files["/workspace.edn"]);
});

test("gist publisher creates public multi-file gists by default", async () => {
  const calls = [];
  const publisher = new GistPublisher({ request: async (...args) => { calls.push(args); return { id: "1" }; } });
  await publisher.publish(await workspaceBundle(repository, "demo"));
  assert.equal(calls[0][0], "/gists");
  assert.equal(calls[0][1].body.public, true);
  assert.deepEqual(Object.keys(calls[0][1].body.files).sort(), ["src__main.hal", "workspace.edn"]);
});

test("publishers update an existing remote artifact", async () => {
  const gistCalls = [];
  const greenwaysCalls = [];
  await new GistPublisher({ request: async (...args) => gistCalls.push(args) })
    .publish(await workspaceBundle(repository, "demo"), { previous: { id: "gist-1" } });
  await new GreenwaysPublisher({ request: async (...args) => greenwaysCalls.push(args) })
    .publish(await workspaceBundle(repository, "demo"), { previous: { id: "work-1" } });
  assert.equal(gistCalls[0][0], "/gists/gist-1");
  assert.equal(greenwaysCalls[0][0], "/works/work-1");
});
