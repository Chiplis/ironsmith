const key = name => String(name || '').trim().toLowerCase();
const aliasesByCatalog = new WeakMap();
export function indexFormatCatalog(cards) {
  let aliases = aliasesByCatalog.get(cards);
  if (!aliases) {
    aliases = new Map();
    for (const card of Object.values(cards)) {
      const front = key(card.name.split(' // ')[0]);
      // Match the original first-match fallback when faces collide.
      if (!aliases.has(front)) aliases.set(front, card);
    }
    aliasesByCatalog.set(cards, aliases);
  }
  return aliases;
}
export const installFormatAliases = (cards, aliases) => aliasesByCatalog.set(cards, aliases);
