import test from "node:test";
import assert from "node:assert/strict";
import { canShowPhasePass } from "../src/lib/opening-hand-actions.js";

const state = (actions = [], stack_size = 0) => ({ decision: { kind: "priority", actions }, stack_size });

test("phase pass is only available at priority with an empty stack", () => {
  assert.equal(canShowPhasePass(state()), true);
  assert.equal(canShowPhasePass(state([], 1)), false);
  assert.equal(canShowPhasePass({ decision: { kind: "mana_payment" } }), false);
  assert.equal(canShowPhasePass({ ...state(), game_over: true }), false);
});

test("phase pass stays hidden through mulligans, pregame abilities, and beginning the game", () => {
  for (const kind of ["keep_opening_hand", "take_mulligan", "use_pregame_action", "continue_pregame", "begin_game"]) {
    assert.equal(canShowPhasePass(state([{ kind }])), false, kind);
    assert.equal(canShowPhasePass(state([{ kind: "pass_priority", action_ref: { kind } }])), false, kind);
  }
  assert.equal(canShowPhasePass(state([{ kind: "pass_priority", label: "Keep hand" }])), false);
});
