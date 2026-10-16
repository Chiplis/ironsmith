const frames = {
  initiative: { label: 'The Initiative', symbol: '◆', color: '#73b6a1' },
  monarch: { label: 'The Monarch', symbol: '♛', color: '#e6bd65' },
  ring: { label: 'The Ring', symbol: '◉', color: '#cf9c55' },
  dungeon_room: { label: 'Dungeon Room', symbol: '▥', color: '#94a6cf' },
};

export function rulesSourceFrame(card) {
  return frames[card?.source_kind] || null;
}

export function rulesSourceCard(card) {
  const frame = rulesSourceFrame(card);
  if (!frame) return card;
  return { ...card, mana_cost: null, type_line: frame.label,
    type_line_display: frame.label,
    oracle_text: card.source_ability_text || card.ability_text || card.oracle_text || card.effect_text || '',
    power: null, toughness: null, power_toughness: null, loyalty: null, defense: null };
}
