import assert from "node:assert/strict";
import test from "node:test";
import { normalizeCreative, solveTwoBone } from "./creative.js";

const keyword = (name) => ({ name });
const scene = () => {
  const bone = new Map([[keyword("id"), "bone/root"], [keyword("length"), 1]]);
  const rig = new Map([[keyword("bones"), [bone]]]);
  const entity = new Map([
    [keyword("id"), "mesh/hero"],
    [keyword("mesh"), new Map([[keyword("primitive"), keyword("box")]])],
    [keyword("material"), new Map([[keyword("color"), "#41f5e4"]])],
    [keyword("rig"), rig]
  ]);
  return new Map([
    [keyword("creative/version"), 1],
    [keyword("background"), "#020408"],
    [keyword("entities"), [entity]]
  ]);
};

test("creative scene normalizes entity, rig, audio, and video data", () => {
  const value = normalizeCreative(scene());
  assert.equal(value.entities[0].id, "mesh/hero");
  assert.equal(value.entities[0].rig.bones[0].id, "bone/root");
  assert.equal(value.background, "#020408");
});

test("two-bone IK returns finite joint angles", () => {
  const pose = solveTwoBone([0, 0], [1, 1], 1, 1);
  assert.ok(Number.isFinite(pose.shoulder));
  assert.ok(Number.isFinite(pose.elbow));
});
