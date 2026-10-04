// Shared by the benchmark's injected React effect and its offline attribution.
// Keep this function self-contained: the production-build plugin embeds it.
export function publicationIdentity(state) {
  if (!state) return null;
  const projected = Object.hasOwn(state, 'decisionDetail');
  return JSON.stringify({
    snapshot: state.snapshot_id, perspective: state.perspective,
    revision: projected ? state.priority_revision : state.__priority_revision,
    turn: state.turn_number, active: state.active_player, phase: state.phase, step: state.step,
    decision: state.decisionDetail ?? state.decision,
    combat: state.combat,
    stack: (state.stack_preview || []).map(card => [card.id, card.name]),
    players: (state.players || []).map(player => ({
      id: player.id, life: player.life, mana: player.mana_pool,
      hand: (player.hand_cards || []).map(card => [card.id, card.name]),
      handSize: Number.isFinite(Number(player.hand_size)) ? Number(player.hand_size) : (player.hand_cards || []).length,
      librarySize: projected && player.library_size === null ? null
        : Number.isFinite(Number(player.library_size)) ? Number(player.library_size) : null,
      graveyardSize: Number.isFinite(Number(player.graveyard_size)) ? Number(player.graveyard_size) : (player.graveyard_cards || []).length,
      exile: (player.exile_cards || []).map(card => [card.id, card.name, card.count]),
      battlefield: (player.battlefield || []).map(card => [card.id, card.name, card.tapped, card.count]),
    })),
  });
}

export function attributePublication(interaction, eventsBySeat) {
  const targets = interaction.publicationTargets;
  if (!targets || targets.length !== eventsBySeat.length) return null;
  const seats = targets.map((target, seat) => eventsBySeat[seat].find(event =>
    event.at >= interaction.submittedAt
    && event.sequence === target.sequence && !event.pendingVerification
    && event.identity === target.identity));
  if (seats.some(event => !event)) return null;
  const publishedAt = Math.max(...seats.map(event => event.at));
  return { publishedAt, callerThroughRenderEffectMs: publishedAt - interaction.submittedAt,
    identitySchemaVersion: 2,
    perSeatPublishedAt: seats.map(event => event.at),
    boundary: 'React passive effect exposing the exact observed rendered state and accepted sequence on both peers; excludes later test polling. Not a paint timestamp.' };
}
