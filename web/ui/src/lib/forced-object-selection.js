// The actor's frontend submits this through the normal command path. Peers
// still receive an answer (and any card openings), even when their local
// hidden-card candidates differ from the actor's.
export function forcedObjectSelectionCommand(decision) {
  if (decision?.kind !== "select_objects"
      || decision.min !== 1
      || decision.max !== 1
      || decision.allow_partial_completion === true) return null;

  const legal = (decision.candidates || []).filter((candidate) => candidate.legal === true);
  if (legal.length !== 1) return null;
  return { type: "select_objects", object_ids: [legal[0].id] };
}

export function localForcedObjectSelectionCommand(state) {
  if (state?.game_over || state?.perspective == null || state?.decision?.player == null
      || Number(state.perspective) !== Number(state.decision.player)) return null;
  return forcedObjectSelectionCommand(state.decision);
}
