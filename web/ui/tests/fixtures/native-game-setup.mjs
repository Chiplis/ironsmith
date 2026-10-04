import assert from 'node:assert/strict';

// Advance through normal game commands so fixtures retain real continuations,
// priorities and turn history. Never edit or import an engine-state DTO.
export function finishPuzzlePregame(game, { filler = 'Mountain', players = 2 } = {}) {
  for (let seat = 0; seat < players; seat++) {
    for (let card = 0; card < 10; card++) game.addCardToZone(seat, filler, 'library', true);
  }
  game.finishPuzzleSetup();
  for (let step = 0; step < players * 3 + 2; step++) {
    const state = game.uiState();
    const action = state.decision?.actions?.find(entry =>
      ['keep_opening_hand', 'continue_pregame', 'begin_game'].includes(entry.action_ref?.kind));
    if (!action) return state;
    game.dispatch({ type: 'priority_action', action_ref: action.action_ref });
  }
  throw new Error('Fixture did not finish pregame');
}

export function passToFirstMain(game, { seat = 0, turn = 1 } = {}) {
  for (let step = 0; step < 100; step++) {
    const state = game.uiState();
    if (state.active_player === seat && state.turn_number >= turn && /first.main/i.test(state.phase)) return state;
    const pass = state.decision?.actions?.find(action => action.action_ref?.kind === 'pass_priority');
    assert.ok(pass, `Expected priority while advancing fixture: ${JSON.stringify(state.decision)}`);
    game.dispatch({ type: 'priority_action', action_ref: pass.action_ref });
  }
  throw new Error('Fixture did not reach first main phase');
}
