import { deckListText, parseCommanderList, parseDeckList, parseSideboardList } from './decklists.js';
import { validateFormatDeck } from './relay/format-legality.js';

// Main-deck and commander errors still exclude a deck. Sideboard cards are
// optional: keep each copy only if adding it leaves the complete deck legal.
export function prepareFormatDeck(format, texts, data, options = {}) {
  const deck = parseDeckList(texts.deckText);
  const commanders = parseCommanderList(texts.commanderText);
  const sideboard = parseSideboardList(texts.deckText);
  const validate = side => validateFormatDeck(format, deck, commanders, side, data, options);
  if (validate(sideboard).ready) return { ...texts, sideboardCount: sideboard.length, removedSideboard: [] };
  if (!validate([]).ready) return null;
  const kept = [];
  const removed = new Map();
  for (const name of sideboard) {
    const result = validate([...kept, name]);
    if (result.ready) kept.push(name);
    else {
      const key = JSON.stringify([name, result.errors]);
      const removal = removed.get(key) || { name, count: 0, reasons: result.errors };
      removal.count += 1;
      removed.set(key, removal);
    }
  }
  return { ...texts, deckText: deckListText(deck, kept), sideboardCount: kept.length, removedSideboard: [...removed.values()] };
}
