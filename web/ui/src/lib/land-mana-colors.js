const ORDER = ['W', 'U', 'B', 'R', 'G', 'C'];
export const LAND_MANA_COLORS = {
  W: { name: 'White', background: '#e5dcb3' },
  U: { name: 'Blue', background: '#448aba' },
  B: { name: 'Black', background: '#777080' },
  R: { name: 'Red', background: '#bd6252' },
  G: { name: 'Green', background: '#568868' },
  C: { name: 'Colorless', background: '#a0a7aa' },
};
export function landManaColors(card) {
  const types = card?.card_types?.length ? card.card_types.join(' ') : card?.type_line || card?.lane || '';
  if (!/land/i.test(types)) return [];
  // Production metadata includes conditional mana and stays visible when tapped.
  // Empty metadata may be a missing Scryfall field; live ability text still applies.
  if (Array.isArray(card.produced_mana) && card.produced_mana.length > 0) {
    const produced = new Set(card.produced_mana.map(value => String(value).toUpperCase()));
    return ORDER.filter(color => produced.has(color));
  }
  const text = [card.oracle_text, card.rules_text, card.effect_text, card.ability_text, ...(card.abilities || [])].filter(Boolean).join('\n');
  const produced = new Set();
  for (const match of text.matchAll(/\badd\s+([^.;\n]+)/gi)) {
    const output = match[1];
    if (/any (?:one )?colou?r|any combination of colou?rs/i.test(output)) ORDER.slice(0,5).forEach(color => produced.add(color));
    for (const symbol of output.matchAll(/\{([WUBRGC])\}/gi)) produced.add(symbol[1].toUpperCase());
  }
  if (Array.isArray(card.produced_mana) && card.produced_mana.length === 0) return ORDER.filter(color => produced.has(color));
  // Basic land types grant intrinsic mana abilities, even without reminder text.
  for (const [type,color] of [['Plains','W'],['Island','U'],['Swamp','B'],['Mountain','R'],['Forest','G']]) {
    if (new RegExp(`\\b${type}\\b`, 'i').test(types)) produced.add(color);
  }
  return ORDER.filter(color => produced.has(color));
}
