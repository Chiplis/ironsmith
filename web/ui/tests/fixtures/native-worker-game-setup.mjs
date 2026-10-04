// Establish test positions through executable commands on an async engine.
// The caller can mirror mutations to another client while keeping private
// openings separate. No game-state importer is involved.
export async function castAndResolveFixtureSpell(call, { objectId, target = null }) {
  const initial = await call('getHiddenCardState');
  const source = initial.objects.find(object => object.id === Number(objectId));
  if (!source) throw new Error('Fixture spell is missing');
  let cast = false;
  for (let step = 0; step < 40; step++) {
    const state = await call('uiState');
    const current = (await call('getHiddenCardState')).objects.find(object => object.stableId === source.stableId);
    if (cast && state.decision?.kind === 'priority' && current && !['hand', 'stack'].includes(current.zone)) return current;
    const decision = state.decision;
    let command;
    if (decision?.kind === 'priority') {
      const action = !cast && decision.actions.find(entry => entry.action_ref?.kind === 'cast_spell'
        && Number(entry.object_id ?? entry.action_ref.spell_id) === Number(objectId))
        || decision.actions.find(entry => entry.action_ref?.kind === 'pass_priority');
      if (!action) throw new Error(`Fixture cannot advance priority: ${JSON.stringify(decision)}`);
      if (action.action_ref.kind === 'cast_spell') cast = true;
      command = { type: 'priority_action', action_ref: action.action_ref };
    } else if (decision?.kind === 'mana_payment') {
      command = { type: 'mana_payment', response: { action: 'confirm', plan_id: decision.plan_id, request_hash: decision.request_hash } };
    } else if (decision?.kind === 'targets' && target) {
      command = { type: 'select_targets', targets: [target] };
    } else if (decision?.kind === 'select_options' && decision.reason === 'Ordering') {
      command = { type: 'select_options', option_indices: decision.options.map(option => option.index) };
    } else throw new Error(`Unsupported fixture resolution: ${JSON.stringify(decision)}`);
    await call('dispatch', command);
  }
  throw new Error('Fixture spell did not finish resolving');
}

export async function advanceFixtureToMain(call, seat) {
  for (let step = 0; step < 100; step++) {
    const state = await call('uiState');
    if (state.active_player === seat && state.phase === 'first main phase' && state.decision?.player === seat) return state;
    const decision = state.decision;
    let command;
    if (decision?.kind === 'attackers') command = { type: 'declare_attackers', declarations: [], bands: [] };
    else if (decision?.kind === 'blockers') command = { type: 'declare_blockers', declarations: [] };
    else {
      const pass = decision?.actions?.find(action => action.action_ref?.kind === 'pass_priority');
      if (!pass) throw new Error(`Fixture cannot advance turn: ${JSON.stringify(decision)}`);
      command = { type: 'priority_action', action_ref: pass.action_ref };
    }
    await call('dispatch', command);
  }
  throw new Error('Fixture did not reach requested main phase');
}
