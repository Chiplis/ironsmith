const ORDER = ['W', 'U', 'B', 'R', 'G'];
const NAMES = { white: 'W', blue: 'U', black: 'B', red: 'R', green: 'G' };
export function normalizeArtColors(colors) {
  const values = Array.isArray(colors) ? colors : String(colors || '').split(/[,\s]+/);
  const found = new Set(values.map(value => NAMES[String(value).toLowerCase()] || String(value).toUpperCase()));
  return ORDER.filter(color => found.has(color));
}
export function cardArtColors(card) {
  // Live colors include effects such as devoid. Color identity/produced mana
  // are deliberately excluded: a Forest is still colorless.
  const signature = String(card?.characteristic_signature || '').match(/^colors:([^\n]*)/m);
  if (signature) return normalizeArtColors(signature[1]);
  if (card?.colors != null) return normalizeArtColors(card.colors);
  const symbols = [...String(card?.mana_cost || '').matchAll(/\{([^}]+)\}/g)];
  return normalizeArtColors(symbols.flatMap(match => match[1].split('/')));
}
export function cardArtSymbolLayout(colors) {
  const normalized = normalizeArtColors(colors);
  const symbols = normalized.length ? normalized : ['C'];
  const count = symbols.length;
  const size = count === 1 ? 56 : count === 2 ? 38 : count === 3 ? 34 : 30;
  const radius = count === 1 ? 0 : count === 2 ? 23 : 28;
  return symbols.map((color, index) => {
    const angle = (count === 2 ? Math.PI : -Math.PI / 2) + index * 2 * Math.PI / count;
    return { color, size, x: 50 + radius * Math.cos(angle) - size / 2, y: 50 + radius * Math.sin(angle) - size / 2 };
  });
}
