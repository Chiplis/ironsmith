import test from "node:test";
import assert from "node:assert/strict";
import { initWasmGame } from "./wasm-test-harness.mjs";

test("Sphinx's opening reveal does not become another reveal at its upkeep trigger", async () => {
  const { game } = await initWasmGame();
  try {
    game.resetEmpty(["Alice", "Bob"], 20);
    game.addCardToZone(0, "Sphinx of Foresight", "hand", true);
    for (const player of [0, 1]) {
      for (const name of ["Island", "Plains", "Forest", "Mountain", "Swamp"]) {
        game.addCardToZone(player, name, "library", true);
      }
    }
    game.finishPuzzleSetup();
    let state = game.uiState();
    let revealed = false;
    for (let step = 0; step < 30; step++) {
      if (revealed && state.stack_size > 0) break;
      assert.equal(state.decision?.kind, "priority");
      const actions = state.decision.actions;
      const reveal = !revealed && actions.find(action => action.label?.startsWith("Reveal Sphinx"));
      const action = reveal || actions.find(action => [
        "keep_opening_hand", "continue_pregame", "begin_game", "pass_priority",
      ].includes(action.action_ref?.kind));
      assert.ok(action, "must be able to advance the starting procedure");
      state = game.dispatch({ type: "priority_action", action_ref: action.action_ref });
      if (reveal) {
        revealed = true;
        assert.equal(state.viewed_cards?.visibility, "public");
        assert.equal(state.viewed_cards.inspector_only, false, "the opening-hand reveal needs acknowledgement");
      }
    }
    assert.ok(revealed && state.stack_size > 0, "must reach the first-upkeep trigger");
    assert.equal(state.viewed_cards?.inspector_only, true, "stack-source visibility is inspection only");
    assert.equal(state.viewed_cards.cards[0].name, "Sphinx of Foresight");
    for (let step = 0; step < 10 && state.decision?.kind === "priority"; step++) {
      const action = state.decision.actions.find(action => action.action_ref?.kind === "pass_priority");
      assert.ok(action);
      state = game.dispatch({ type: "priority_action", action_ref: action.action_ref });
    }
    assert.equal(state.decision?.kind, "select_objects", "the trigger must still scry");
    assert.equal(state.decision.candidates.length, 3);
    assert.equal(state.viewed_cards.inspector_only, false, "scry executes a real look instruction");
    assert.equal(state.viewed_cards.acknowledged, false);
    state = game.dispatch({ type: "select_objects", object_ids: [] });
    while (state.decision?.kind === "select_options") {
      state = game.dispatch({ type: "select_options", option_indices: state.decision.options.map(option => option.index) });
    }
    assert.equal(state.decision?.kind, "priority");
    assert.equal(state.viewed_cards.acknowledged, true, "finishing the scry acknowledges its look");
  } finally {
    game.free();
  }
});
