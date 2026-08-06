import assert from "node:assert/strict";
import test from "node:test";

import { downloadWorkspace, GitHubDeviceAuth, GistPublisher, workspaceBundle, zipWorkspace } from "../../../website/hara-www/publishing.js";

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

test("gist publisher updates an existing Gist", async () => {
  const gistCalls = [];
  await new GistPublisher({ request: async (...args) => gistCalls.push(args) })
    .publish(await workspaceBundle(repository, "demo"), { previous: { id: "gist-1" } });
  assert.equal(gistCalls[0][0], "/gists/gist-1");
});

test("workspace download is a zip containing the source tree", async () => {
  const bundle = await workspaceBundle(repository, "demo");
  const archive = zipWorkspace(bundle);
  assert.equal(new DataView(archive.buffer).getUint32(0, true), 0x04034b50);
  assert.deepEqual(zipNames(archive), ["src/main.hal", "workspace.edn"]);
  let saved = null;
  assert.equal(downloadWorkspace(bundle, { save: (blob, filename) => { saved = { blob, filename }; } }), "demo.zip");
  assert.equal(saved.filename, "demo.zip");
  assert.equal(saved.blob.type, "application/zip");
});

test("GitHub device flow polls until authorization completes", async () => {
  const calls = [];
  const answers = [
    { device_code: "device", user_code: "ABCD-EFGH", verification_uri: "https://github.com/login/device", interval: 1 },
    { error: "authorization_pending" },
    { access_token: "token" }
  ];
  const auth = new GitHubDeviceAuth({
    clientId: "client",
    request: async (...args) => { calls.push(args); return answers.shift(); },
    sleep: async () => {}
  });
  const device = await auth.begin();
  const token = await auth.authorize(device);
  assert.equal(token.access_token, "token");
  assert.deepEqual(calls[0], ["https://github.com/login/device/code", { client_id: "client" }]);
  assert.equal(calls.filter(([url]) => url.includes("access_token")).length, 2);
});

function zipNames(bytes) {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const names = [];
  for (let offset = 0; offset <= bytes.length - 4; offset += 1) {
    if (view.getUint32(offset, true) !== 0x02014b50) continue;
    const nameLength = view.getUint16(offset + 28, true);
    names.push(new TextDecoder().decode(bytes.slice(offset + 46, offset + 46 + nameLength)));
  }
  return names;
}
