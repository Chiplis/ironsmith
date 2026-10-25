// Prices come from the engine's normal-cast preview, independently of printed
// characteristics. Never use a front-face price for a different hovered face.
export function handManaCostPresentation(card, state) {
  if (!card) return card;
  const price = (state?.players || []).flatMap(player => player.hand_mana_costs || [])
    .find(price => String(price.id) === String(card.id) && price.name === card.name);
  if (!price || typeof price.mana_cost !== 'string' || price.mana_cost === card.mana_cost) return card;
  if (card.card_faces?.length) {
    if (!card.card_faces.some(face => face.name === price.name)) return card;
    return { ...card, card_faces: card.card_faces.map(face => face.name === price.name
      ? { ...face, display_mana_cost: price.mana_cost } : face) };
  }
  return { ...card, display_mana_cost: price.mana_cost };
}
