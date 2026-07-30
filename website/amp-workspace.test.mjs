import assert from "node:assert/strict";
import test from "node:test";
import { seedAmpWorkspace } from "./amp-workspace.js";

const project = "{:hara/type :project}";
const workspace = "{:workspace/customizations {:recovery/journal true}}";
const visualizer = "(ns+ hara.amp)";

test("the Amp workspace seed carries the selected controls", () => {
  const files = seedAmpWorkspace({
    project,
    workspace,
    visualizer,
    preset: "bass",
    mode: "scope"
  });

  assert.equal(files.get("/project.edn"), project);
  assert.equal(files.get("/src/amp.hal"), visualizer);
  assert.match(files.get("/workspace.edn"), /:amp\/eq-preset :bass/);
  assert.match(files.get("/workspace.edn"), /:amp\/visual-mode :scope/);
});

test("the Amp workspace seed falls back to safe options", () => {
  const files = seedAmpWorkspace({
    project,
    workspace,
    visualizer,
    preset: "unknown",
    mode: "unknown"
  });

  assert.match(files.get("/workspace.edn"), /:amp\/eq-preset :hara/);
  assert.match(files.get("/workspace.edn"), /:amp\/visual-mode :spectrum/);
});
