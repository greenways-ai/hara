import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { patchAnimationActions, patchAnimationCharacter, projectAnimationSource } from "./animation-source-model.js";

const source = await readFile(new URL("./examples/hara-animation/src/animation.hal", import.meta.url), "utf8");

test("projects character and ordered actions from authoritative HAL", () => {
  assert.deepEqual(projectAnimationSource(source), {
    selected: "robot",
    actions: ["walk", "wave", "jump", "spin", "bow"]
  });
});

test("patches character and actions without replacing the document", () => {
  const changed = patchAnimationActions(patchAnimationCharacter(source, "fox"), ["bow", "jump"]);
  assert.deepEqual(projectAnimationSource(changed), { selected: "fox", actions: ["bow", "jump"] });
  assert.match(changed, /\(defn animation-plan/);
});
