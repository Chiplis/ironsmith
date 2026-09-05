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

test("decision button identifies the decision owner while keeping a fixed yellow color", () => {
  const state = {
    players,
    perspective: 0,
    active_player: 1,
    priority_player: 1,
  };
  const decision = { kind: "priority", player: 0 };

  assert.equal(decisionButtonPlayerId(state, decision), 0);
  assert.equal(decisionButtonAccentVars(state, decision)["--decision-main-accent"], "#ffe083");
});

test("decision button keeps its yellow color during phase transitions", () => {
  const state = {
    players,
    perspective: 0,
    active_player: 1,
    priority_player: null,
  };

  assert.equal(decisionButtonPlayerId(state, null), 0);
  assert.equal(decisionButtonAccentVars(state, null)["--decision-main-accent"], "#ffe083");
});


test("decision button color ignores player identity and custom seat colors", () => {
  const state = { players, perspective: 0, priority_player: 0 };
  assert.deepEqual(
    decisionButtonAccentVars(state, { player: 0 }, { "0": "#ff0000" }),
    decisionButtonAccentVars({ ...state, perspective: 1 }, { player: 1 }, { "1": "#00ff00" })
  );
});
