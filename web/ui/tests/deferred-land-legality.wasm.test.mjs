import { finishPuzzlePregame, passToFirstMain } from "./fixtures/native-game-setup.mjs";
import test from 'node:test';
import assert from 'node:assert/strict';
import { initWasmGame } from '../../../scripts/wasm-test-harness.mjs';

test('deferred menus still validate ownership, phase, and the land play limit in the real engine', async () => {
  const { game } = await initWasmGame({ pkg: 'demo' });
  try {
    game.setDeferredPriorityAnalysis(true);
    game.resetEmpty(['Alice', 'Bob'], 20);
    const first = Number(game.addCardToHand(0, 'Mountain'));
    const second = Number(game.addCardToHand(0, 'Mountain'));
    const opponent = Number(game.addCardToHand(1, 'Island'));
    finishPuzzlePregame(game);
    const upkeep = game.createRuntimeSavepoint();
    passToFirstMain(game);
    const main = game.createRuntimeSavepoint();
    const reset = () => {
      game.copyRuntimeSavepoint(main);
      assert.equal(game.uiState().decision.analysis_complete, false);
    };
    const play = id => game.dispatch({ type: 'priority_action', action_ref: { kind: 'play_land', land_id: id } });
    reset();
    assert.throws(() => play(opponent), /legal|available|action/i);
    reset();
    play(first);
    assert.equal(game.uiState().players[0].battlefield.length, 1);
    assert.throws(() => play(second), /legal|available|action/i);
    assert.equal(game.uiState().players[0].battlefield.length, 1);
    game.copyRuntimeSavepoint(upkeep);
    assert.throws(() => play(first), /legal|available|action/i);
    game.releaseRuntimeSavepoint(main);
    game.releaseRuntimeSavepoint(upkeep);
  } finally {
    game.setDeferredPriorityAnalysis(false);
    game.free();
  }
});
