// Only the engine's forced whole-hand disclosure may bypass manual selection.
// Sending the ordinary command preserves multiplayer openings and verification.
export function automaticHandRevealCommand(state) {
  const decision = state?.decision;
  if (state?.game_over || decision?.kind !== 'select_objects'
      || decision.automatic_public_reveal !== true
      || decision.reveal_policy !== 'public'
      || state?.perspective == null || decision.player == null
      || String(decision.player) !== String(state.perspective)
      || decision.allow_partial_completion) return null;
  const candidates = decision.candidates || [];
  if (!candidates.length || decision.min !== candidates.length
      || decision.max !== candidates.length
      || candidates.some(card => card.legal === false
        || (card.reveal_policy != null && card.reveal_policy !== 'public'))) return null;
  const ids = candidates.map(card => Number(card.id));
  if (ids.some(id => !Number.isSafeInteger(id) || id <= 0)
      || new Set(ids).size !== ids.length) return null;
  return { type: 'select_objects', object_ids: ids };
}
