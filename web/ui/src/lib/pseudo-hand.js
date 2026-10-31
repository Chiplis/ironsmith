/** Keep confirmed cards visible while cumulative action analysis is pending.
 * Zone snapshots remain authoritative; cached actions are never executable.
 */
export function reconcilePseudoHand(previous, current, state) {
  const next = new Map(current);
  if (state?.decision?.kind !== "priority"
      || state.decision.analysis_complete !== false) return next;
  const zones = new Map();
  for (const player of state.players || []) {
    for (const zone of ["graveyard", "exile", "command", "ante", "library"]) {
      for (const card of player[`${zone}_cards`] || []) {
        if (zone === "library" && !card.show_in_pseudo_hand) continue;
        zones.set(Number(card.id), { card, zone, ownerId: player.id });
      }
    }
  }
  for (const [id, cached] of previous) {
    const visible = zones.get(id);
    if (!next.has(id) && visible && visible.zone === cached.fromZone) {
      next.set(id, { ...cached, card: visible.card, name: visible.card.name,
        ownerId: visible.ownerId ?? cached.ownerId, actions: [] });
    }
  }
  return next;
}
