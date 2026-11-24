// Drives headless seats through a match with simple legal choices.
export function chooseCommand(decision) {
  if (!decision) return null;
  if (decision.kind === 'priority') {
    const actions = decision.actions || [];
    const pick = actions.find(a => a.action_ref?.kind === 'keep_opening_hand')
      || actions.find(a => /play_land/.test(a.action_ref?.kind || '') || /^play_land|play land/i.test(a.label || ''))
      || actions.find(a => a.kind === 'pass_priority' || a.action_ref?.kind === 'pass_priority');
    if (!pick) return null;
    return { type: 'priority_action', action_ref: pick.action_ref };
  }
  if (decision.kind === 'attackers') return { type: 'declare_attackers', declarations: [] };
  if (decision.kind === 'blockers') return { type: 'declare_blockers', declarations: [] };
  if (decision.kind === 'select_objects') {
    const min = Number(decision.min ?? decision.min_count ?? 0);
    const candidates = decision.candidates || decision.objects || [];
    const ids = candidates.slice(0, min).map(c => Number(c.id ?? c.object_id ?? c));
    return { type: 'select_objects', object_ids: ids };
  }
  return null;
}

export async function playStep(seat, index, { log } = {}) {
  const snap = await seat.snapshot();
  const decision = snap.state?.decision;
  if (!decision || snap.state.game_over) return false;
  if (Number(decision.player) !== index) return false;
  if (snap.multiplayer.submittingAction) return false;
  const command = chooseCommand(decision);
  if (!command) { log?.(`${seat.name}: no command for ${decision.kind} ${JSON.stringify(decision).slice(0, 300)}`); return false; }
  log?.(`${seat.name} -> ${command.action_ref?.kind || command.type}`);
  await seat.lobby('submitMultiplayerCommand', command, command.action_ref?.kind || command.type);
  return true;
}
