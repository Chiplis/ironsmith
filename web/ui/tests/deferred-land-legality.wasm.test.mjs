import test from 'node:test';
import assert from 'node:assert/strict';
import { initWasmGame } from '../../../scripts/wasm-test-harness.mjs';

test('deferred menus still validate ownership, phase, and the land play limit in the real engine', async () => {
  const { game } = await initWasmGame({ pkg: 'demo' });
  try {
    game.resetEmpty(['Alice', 'Bob'], 20);
    const first = Number(game.addCardToHand(0, 'Mountain'));
    const second = Number(game.addCardToHand(0, 'Mountain'));
    const opponent = Number(game.addCardToHand(1, 'Island'));
    game.finishPuzzleSetup();
    for (let i = 0; i < 4; i++) {
      const action = game.uiState().decision.actions.find(entry =>
        ['keep_opening_hand', 'continue_pregame', 'begin_game'].includes(entry.action_ref?.kind));
      assert.ok(action);
      game.dispatch({ type: 'priority_action', action_ref: action.action_ref });
    }
    const checkpoint = game.exportSyncCheckpoint();
    checkpoint.turn.phase = 'first_main';
    delete checkpoint.turn.step;
    checkpoint.turn.activePlayer = 0;
    checkpoint.turn.priorityPlayer = 0;
    checkpoint.priorityRuntime.turnRunnerState = 'first_main_priority';
    game.setDeferredPriorityAnalysis(true);
    const reset = () => {
      game.importSyncCheckpoint(checkpoint);
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
    checkpoint.turn.phase = 'beginning';
    checkpoint.turn.step = 'upkeep';
    checkpoint.priorityRuntime.turnRunnerState = 'upkeep_priority';
    reset();
    assert.throws(() => play(first), /legal|available|action/i);
  } finally {
    game.setDeferredPriorityAnalysis(false);
    game.free();
  }
});
