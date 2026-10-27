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

// Only pure tap-for-mana abilities qualify for the shortcut. Anchoring the
// whole effect keeps damage, life payments, spending restrictions, and other
// conditions on the explicit ability controls.
export function isSimpleTapManaAction(action) {
  if (action?.kind !== 'activate_mana_ability') return false;
  const text = String(action.ability_text || action.action_ref?.ability_text || action.label || '')
    .replace(/^Activate\s+.*?:\s*/i, '').trim();
  return /^\{T\}:\s*Add\s+(?:(?:\{[WUBRGC]\}\s*)+(?:(?:,\s*(?:or\s+)?|\s*or\s+)(?:\{[WUBRGC]\}\s*)+)*|one mana of any colou?r)\.?(?:\s*·\s*\d+\s+[WUBRGC])*$/i.test(text);
}

// One choice per color the permanent can make now. Prefer a single-color
// action over an ability for which the engine must ask for a color.
export function landManaChoices(actions) {
  const choices = new Map();
  for (const action of actions || []) {
    if (!isSimpleTapManaAction(action)) continue;
    const colors = manaActionColors(action);
    for (const color of colors) {
      const exact = colors.length === 1;
      const current = choices.get(color);
      if (!current || (exact && !current.exact)) choices.set(color, { color, action, exact });
    }
  }
  return ORDER.filter(color => choices.has(color)).map(color => choices.get(color));
}

// Passive strips remain visible while tapped or summoning sick. Live ability
// text is authoritative, including an empty list after abilities are removed.
export function cardSimpleTapManaColors(card) {
  const lines = Array.isArray(card?.abilities) ? card.abilities : [card?.oracle_text || ''];
  return landManaChoices(lines.flatMap(line => String(line).split('\n')).map(label => ({
    kind: 'activate_mana_ability', label: label.replace(/^Mana ability:\s*/i, ''),
  }))).map(choice => choice.color);
}
