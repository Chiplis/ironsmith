import test from "node:test";
import assert from "node:assert/strict";
import { initWasmGame } from "../../../scripts/wasm-test-harness.mjs";

test("Kumano chapter II gives only the next cast creature an entry counter", async () => {
  const { game } = await initWasmGame({ pkg: "demo" });
  try {
    game.resetEmpty(["Alice", "Bob"], 20);
    game.addCardToZone(0, "Kumano Faces Kakkazan", "battlefield", true);
    const first = Number(game.addCardToHand(0, "Ornithopter"));
    const second = Number(game.addCardToHand(0, "Ornithopter"));
    for (let player = 0; player < 2; player += 1) {
      game.addCardToZone(player, "Mountain", "library", true);
    }
    game.finishPuzzleSetup();
    let state = game.uiState();

    const dispatch = (action) => {
      assert.ok(action, `Expected a legal action: ${JSON.stringify(state.decision)}`);
      state = game.dispatch({ type: "priority_action", action_ref: action.action_ref });
    };
    const castAction = (id) => (state.decision?.actions || []).find((action) => (
      action.action_ref?.kind === "cast_spell" && Number(action.action_ref.spell_id) === id
    ));
    const pass = () => dispatch((state.decision?.actions || []).find((action) => (
      ["keep_opening_hand", "continue_pregame", "begin_game", "pass_priority"]
        .includes(action.action_ref?.kind)
    )));
    const resolveStack = () => {
      for (let step = 0; state.stack_size > 0 && step < 12; step += 1) pass();
      assert.equal(state.stack_size, 0, "Expected the stack to resolve");
    };
    const creature = (stableId) => state.players[0].battlefield.find((object) => (
      Number(object.stable_id) === stableId
    ));

    // Puzzle placement supplies lore I. Advancing through upkeep and draw
    // adds lore II and resolves its chapter ability before the first cast.
    for (let step = 0; !castAction(first) && step < 20; step += 1) pass();
    const saga = state.players[0].battlefield.find((object) => (
      object.name === "Kumano Faces Kakkazan"
    ));
    assert.ok(saga?.counters.some((counter) => counter.kind === "lore" && counter.amount === 2));
    assert.equal(state.stack_size, 0, "Chapter II must resolve before casting");

    dispatch(castAction(first));
    assert.equal(state.stack_size, 2, "The delayed cast trigger must stack above the creature");
    resolveStack();
    assert.equal(creature(first)?.power_toughness, "1/3");
    assert.deepEqual(creature(first)?.counters, [{ kind: "+1/+1", amount: 1 }]);

    dispatch(castAction(second));
    assert.equal(state.stack_size, 1, "The delayed trigger must be consumed by the first cast");
    resolveStack();
    assert.equal(creature(second)?.power_toughness, "0/2");
    assert.deepEqual(creature(second)?.counters, []);
  } finally {
    game.free();
  }
});
