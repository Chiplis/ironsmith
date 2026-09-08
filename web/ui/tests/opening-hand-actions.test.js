import test from "node:test";
import assert from "node:assert/strict";
import {
  findOpeningHandMulliganAction,
  isOpeningHandDecision,
} from "../src/lib/opening-hand-actions.js";

const keepHand = { index: 0, kind: "pass_priority", label: "Keep hand", action_ref: { kind: "keep_opening_hand" } };
const mulligan = { index: 1, kind: "take_mulligan", label: "Mulligan", action_ref: { kind: "take_mulligan" } };
const passPriority = { index: 0, kind: "pass_priority", label: "Pass priority", action_ref: { kind: "pass_priority" } };

test("finds the mulligan action beside the keep-hand advance action", () => {
  const actions = [keepHand, mulligan];
  assert.equal(isOpeningHandDecision(actions, keepHand), true);
  assert.equal(findOpeningHandMulliganAction(actions, keepHand), mulligan);
});

test("recognizes keep hand from the pass action label alone", () => {
  const keepByLabel = { index: 0, kind: "pass_priority", label: "Keep hand" };
  assert.equal(findOpeningHandMulliganAction([keepByLabel, mulligan], keepByLabel), mulligan);
});

test("returns null once the player can no longer mulligan", () => {
  assert.equal(findOpeningHandMulliganAction([keepHand], keepHand), null);
});

test("ignores non-opening-hand priority decisions", () => {
  const cast = { index: 1, kind: "cast_spell", label: "Cast Mulligan", object_id: 7 };
  const stray = { index: 2, kind: "take_mulligan", label: "Mulligan" };
  assert.equal(isOpeningHandDecision([passPriority, cast, stray], passPriority), false);
  assert.equal(findOpeningHandMulliganAction([passPriority, cast, stray], passPriority), null);
});
