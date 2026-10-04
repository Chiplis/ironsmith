export async function advancePriorityFixture(call, seat = 0) {
  let state = await call('uiState');
  for (let step = 0; step < 160; step++) {
    if (/first.main/i.test(state.phase) && Number(state.active_player) === seat
      && Number(state.priority_player) === seat && state.decision?.kind === 'priority') return state;
    const action = state.decision?.actions?.find(candidate =>
      ['keep_opening_hand', 'continue_pregame', 'begin_game', 'pass_priority'].includes(candidate.action_ref?.kind));
    const command = action ? { type: 'priority_action', action_ref: action.action_ref }
      : ['attackers', 'blockers'].includes(state.decision?.kind)
        ? { type: state.decision.kind === 'attackers' ? 'declare_attackers' : 'declare_blockers', declarations: [] }
        : null;
    if (!command) throw new Error(`Cannot advance priority fixture: ${JSON.stringify(state.decision)}`);
    state = await call('dispatch', command);
  }
  throw new Error(`Priority fixture did not reach seat ${seat}'s main phase`);
}

export async function setupIncrementalPriorityFixture(call) {
  await call('resetEmpty', ['Alice', 'Bob'], 20);
  for (let i = 0; i < 4; i++) await call('addCardToZone', 0, 'Nova Hellkite', 'hand', true);
  for (let i = 0; i < 2; i++) await call('addCardToZone', 0, 'Magmatic Hellkite', 'hand', true);
  const land = Number(await call('addCardToZone', 0, 'Sunbillow Verge', 'hand', true));
  for (let i = 0; i < 3; i++) await call('addCardToZone', 0, 'Mountain', 'battlefield', true);
  await call('addCardToZone', 1, 'Icetill Explorer', 'battlefield', true);
  await call('addCardToZone', 0, 'Icetill Explorer', 'battlefield', true);
  await call('addCardToZone', 0, 'Mountain', 'graveyard', true);
  const secondLand = Number(await call('addCardToZone', 1, 'Mountain', 'hand', true));
  const secondSpell = Number(await call('addCardToZone', 1, 'Ornithopter', 'hand', true));
  for (let seat = 0; seat < 2; seat++) {
    for (let i = 0; i < 20; i++) await call('addCardToZone', seat, 'Mountain', 'library', true);
  }
  await call('finishPuzzleSetup');
  return { land, secondLand, secondSpell, state: await advancePriorityFixture(call) };
}
