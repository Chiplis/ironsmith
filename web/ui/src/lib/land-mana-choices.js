// Intrinsic mana abilities come from basic land types (CR 305.6): a Forest
// has "{T}: Add {G}." without printing it. The engine words them like any
// ability, but the card's text box never shows them; the land's mana strip
// is where they are used instead.
const ORDER = ['W', 'U', 'B', 'R', 'G', 'C'];
const BASIC_TYPES = [['Plains', 'W'], ['Island', 'U'], ['Swamp', 'B'], ['Mountain', 'R'], ['Forest', 'G']];

// Live permanents carry their types in card_types and the characteristic
// signature (which lists subtypes such as Forest) rather than a type line.
// Only its type lines count: the signature also holds the name and rules
// text, where "Forest" can appear without the object being one (Yavimaya).
export function cardTypeText(card) {
  const signature = String(card?.characteristic_signature || '').split('\n')
    .filter(line => /^(?:supertypes|types|subtypes):/.test(line)).map(line => line.replace(/^[a-z]+:/, ''));
  return [card?.type_line, card?.type_line_display, ...(Array.isArray(card?.card_types) ? card.card_types : []), ...signature]
    .filter(Boolean).join(' ');
}

export function intrinsicManaColors(typeLine) {
  const text = String(typeLine || '');
  if (!/\bLand\b/i.test(text)) return [];
  return BASIC_TYPES.filter(([type]) => new RegExp(`\\b${type}\\b`, 'i').test(text)).map(([, color]) => color);
}

// Live permanents report their intrinsic mana (engine snapshot); other
// snapshots, such as inspector details, carry a type line to read it from.
export function cardIntrinsicManaColors(card) {
  if (Array.isArray(card?.intrinsic_mana)) return ORDER.filter(color => card.intrinsic_mana.includes(color));
  return intrinsicManaColors(cardTypeText(card));
}

// Remove one "{T}: Add {X}." line per intrinsic mana color. An equal printed
// or granted ability is a separate line and stays.
export function withoutIntrinsicManaLines(lines, colors) {
  const pending = new Set(colors);
  if (!pending.size) return lines;
  return lines.filter(line => {
    const color = String(line).trim().match(/^\{T\}:\s*Add\s+\{([WUBRG])\}\.$/i)?.[1]?.toUpperCase();
    if (!color || !pending.has(color)) return true;
    pending.delete(color);
    return false;
  });
}

// Colors a mana action produces, read from its "…: Add {G} or {C}." label.
export function manaActionColors(action) {
  const text = String(action?.label || action?.description || '');
  const output = text.match(/\bAdd\b([^.\n]*)/i)?.[1] || '';
  if (/any (?:one )?colou?r|any combination of colou?rs/i.test(output)) return ORDER.slice(0, 5);
  const found = new Set([...output.matchAll(/\{([WUBRGC])\}/gi)].map(match => match[1].toUpperCase()));
  return ORDER.filter(color => found.has(color));
}

// One choice per color the land can make now: the action that adds exactly
// that color wins over one that offers it among others (whose color the
// engine then asks for).
export function landManaChoices(actions) {
  const choices = new Map();
  for (const action of actions || []) {
    if (action?.kind !== 'activate_mana_ability') continue;
    const colors = manaActionColors(action);
    for (const color of colors) {
      const exact = colors.length === 1;
      const current = choices.get(color);
      if (!current || (exact && !current.exact)) choices.set(color, { color, action, exact });
    }
  }
  return ORDER.filter(color => choices.has(color)).map(color => choices.get(color));
}
