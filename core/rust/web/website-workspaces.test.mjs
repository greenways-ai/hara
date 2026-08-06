import assert from "node:assert/strict";
import test from "node:test";
import "fake-indexeddb/auto";

import { WorkspaceRepository, kernelName, templateFiles, workspaceTemplates } from "../../../website/hara-www/workspaces.js";

test("all workspace templates seed project, workspace, and source files", () => {
  assert.deepEqual(workspaceTemplates.map(({ id }) => id), ["blank", "canvas", "music", "3d", "graphs"]);
  for (const { id } of workspaceTemplates) {
    const files = templateFiles(id, `test-${id}`);
    assert.match(files.get("/project.edn"), /:hara\/type :project/);
    assert.match(files.get("/workspace.edn"), /:hara\/type :workspace/);
    assert.match(files.get("/workspace.edn"), new RegExp(`:template :${id}`));
    assert.ok(files.has("/src/main.hal"));
  }
});

test("repository persists independent workspaces and files", async () => {
  const repository = new WorkspaceRepository({ dbName: "workspace-repository-test" });
  const canvas = await repository.create({ name: "Canvas One", template: "canvas" });
  const music = await repository.create({ name: "Music One", template: "music" });

  await repository.writeFile(canvas.id, "/src/main.hal", "canvas-value");
  assert.equal((await repository.files(canvas.id)).get("/src/main.hal"), "canvas-value");
  assert.notEqual((await repository.files(music.id)).get("/src/main.hal"), "canvas-value");
  assert.deepEqual((await repository.list()).map(({ id }) => id).sort(), [canvas.id, music.id].sort());

  await repository.delete(canvas.id);
  assert.equal(await repository.get(canvas.id), null);
  assert.equal((await repository.files(canvas.id)).size, 0);
  assert.deepEqual((await repository.list()).map(({ id }) => id), [music.id]);

  await repository.clear();
  assert.deepEqual(await repository.list(), []);
  assert.equal((await repository.files(music.id)).size, 0);
});

test("workspace names produce safe dedicated kernel names", () => {
  assert.equal(kernelName("My Project 01"), "workspace.my-project-01");
});
