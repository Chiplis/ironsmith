// A price belongs to exactly one published view, including its perspective.
export function mergeHandManaCosts(state, preview) {
  if (!preview || state?.snapshot_id !== preview.snapshot_id
      || state?.perspective !== preview.perspective || !Array.isArray(preview.prices)) return state;
  return {...state, players:state.players.map(player => {
    const visible = new Set((player.hand_cards || []).map(card => String(card.id)));
    return {...player, hand_mana_costs:preview.prices.filter(price => visible.has(String(price.id)))};
  })};
}
