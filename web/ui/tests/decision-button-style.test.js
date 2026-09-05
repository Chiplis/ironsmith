import test from "node:test";
import assert from "node:assert/strict";
import {
  decisionButtonAccentVars,
  decisionButtonPlayerId,
} from "../src/lib/decision-button-style.js";

const players = [
  { id: 0, name: "Alice" },
  { id: 1, name: "Bob" },
];

test("decision button color is driven by the decision player before priority state", () => {
  const state = {
    players,
    perspective: 0,
    active_player: 1,
    priority_player: 1,
  };
  const decision = { kind: "priority", player: 0 };

  assert.equal(decisionButtonPlayerId(state, decision), 0);
  assert.equal(decisionButtonAccentVars(state, decision)["--decision-main-accent"], "#b79cff");
});

test("decision button color does not fall back to active player during phase transitions", () => {
  const state = {
    players,
    perspective: 0,
    active_player: 1,
    priority_player: null,
  };

  assert.equal(decisionButtonPlayerId(state, null), 0);
  assert.equal(decisionButtonAccentVars(state, null)["--decision-main-accent"], "#b79cff");
});
