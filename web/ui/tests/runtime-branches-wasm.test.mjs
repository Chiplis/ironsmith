import test from 'node:test';
import assert from 'node:assert/strict';
import { initWasmGame, startEmptyMatch } from '../../../scripts/wasm-test-harness.mjs';

test('real WASM runtime branches retain independent game state and gameplay ID cursors', async () => {
  const { game } = await initWasmGame({ pkg: 'demo' });
  startEmptyMatch(game, { startingPlayer: 0, decks: [Array(60).fill('Forest'), Array(60).fill('Island')] });
  game.addCardToZone(0, 'Llanowar Elves', 'Battlefield', true);
  const handle = game.createRuntimeSavepoint();
  game.addCardToZone(0, 'Forest', 'Battlefield', true);
  const first = game.uiState();
  const forest = first.players[0].battlefield.find(card => card.name === 'Forest');
  game.addLifeDelta(0, -4);
  game.exchangeRuntimeSavepoint(handle);
  assert.equal(game.uiState().players[0].life, 20);
  assert.equal(game.uiState().players[0].battlefield.some(card => card.name === 'Forest'), false);
  game.addCardToZone(0, 'Forest', 'Battlefield', true);
  const other = game.uiState();
  const verifiedForest = other.players[0].battlefield.find(card => card.name === 'Forest');
  assert.equal(verifiedForest.id, forest.id, 'replaying a command preserves gameplay object IDs');
  assert.equal(verifiedForest.stable_id, forest.stable_id);
  game.addLifeDelta(0, -2);
  game.exchangeRuntimeSavepoint(handle);
  assert.equal(game.uiState().players[0].life, 16);
  game.copyRuntimeSavepoint(handle);
  assert.equal(game.uiState().players[0].life, 18);
  game.addLifeDelta(0, -1);
  game.exchangeRuntimeSavepoint(handle);
  assert.equal(game.uiState().players[0].life, 18, 'copy did not consume or modify the retained verified branch');
  game.exchangeRuntimeSavepoint(handle);
  assert.equal(game.uiState().players[0].life, 17);
  assert.equal(game.releaseRuntimeSavepoint(handle), true);
  game.free();
});

test('real runtime branches preserve a live casting continuation while the visible cast is cancelled', async () => {
  const { game } = await initWasmGame({ pkg: 'demo' });
  try {
    game.resetEmpty(['Alice', 'Bob'], 20);
    game.addCardToZone(0, 'Mountain', 'Battlefield', true);
    game.addCardToHand(0, 'Lightning Bolt');
    game.finishPuzzleSetup();
    for (let i = 0; i < 4; i++) {
      const action = game.uiState().decision.actions.find(entry =>
        ['keep_opening_hand', 'continue_pregame', 'begin_game'].includes(entry.action_ref?.kind));
      assert.ok(action);
      game.dispatch({ type: 'priority_action', action_ref: action.action_ref });
    }
    const cast = game.uiState().decision.actions.find(entry => entry.action_ref?.kind === 'cast_spell');
    assert.ok(cast);
    game.dispatch({ type: 'priority_action', action_ref: cast.action_ref });
    assert.equal(game.uiState().decision.kind, 'targets');
    const retained = game.createRuntimeSavepoint();
    game.cancelDecision();
    assert.equal(game.uiState().decision.kind, 'priority');
    game.exchangeRuntimeSavepoint(retained);
    assert.equal(game.uiState().decision.kind, 'targets');
    // This resumes the retained engine continuation rather than reconstructing
    // it from the visible board or a wire checkpoint.
    game.dispatch({ type: 'select_targets', targets: [{ kind: 'player', player: 1 }] });
    const payment = game.uiState();
    assert.notEqual(payment.decision.kind, 'targets');
    game.exchangeRuntimeSavepoint(retained);
    assert.equal(game.uiState().decision.kind, 'priority');
    game.copyRuntimeSavepoint(retained);
    assert.equal(game.uiState().decision.kind, payment.decision.kind);
    game.cancelDecision();
    assert.equal(game.uiState().decision.kind, 'priority');
    assert.equal(game.releaseRuntimeSavepoint(retained), true);
  } finally { game.free(); }
});
