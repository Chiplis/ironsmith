export async function setupCombatPriorityFixture(call, seat) {
  await call('resetEmpty', ['Host', 'Guest'], 20);
  for (let i = 0; i < 3; i++) await call('addCardToZone', seat, 'Swamp', 'battlefield', true);
  await call('addCardToZone', 1 - seat, 'Grizzly Bears', 'battlefield', true);
  for (const name of ['Shoot the Sheriff', 'Requiting Hex', 'Thoughtseize', 'Swamp']) {
    await call('addCardToZone', seat, name, 'hand', true);
  }
  for (let owner = 0; owner < 2; owner++) {
    for (let i = 0; i < 10; i++) await call('addCardToZone', owner, 'Swamp', 'library', true);
  }
  await call('finishPuzzleSetup');
  let state = await call('uiState');
  for (let step = 0; step < 160; step++) {
    if (/combat/i.test(state.phase) && state.decision?.kind === 'priority'
      && Number(state.active_player) === 1 - seat && Number(state.priority_player) === seat) {
      await call('setPerspective', seat);
      return call('uiState');
    }
    const action = state.decision?.actions?.find(candidate =>
      ['keep_opening_hand', 'continue_pregame', 'begin_game', 'pass_priority'].includes(candidate.action_ref?.kind));
    const command = action ? { type: 'priority_action', action_ref: action.action_ref }
      : ['attackers', 'blockers'].includes(state.decision?.kind)
        ? { type: state.decision.kind === 'attackers' ? 'declare_attackers' : 'declare_blockers', declarations: [] }
        : null;
    if (!command) throw new Error(`Cannot advance combat fixture: ${JSON.stringify(state.decision)}`);
    state = await call('dispatch', command);
  }
  throw new Error('Combat fixture never reached opponent combat priority');
}
