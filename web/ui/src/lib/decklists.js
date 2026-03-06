export const LOBBY_DECK_SIZE = 60;

const MAIN_DECK_HEADER = /^Deck$/i;
const EXTRA_DECK_HEADER = /^(Sideboard|Commander|Companion|Maybeboard)$/i;
const CARD_LINE = /^(\d+)x?\s+(.+)$/;

export function parseDeckList(text) {
  const cards = [];
  let parsingMainDeck = true;

  for (const line of String(text || "").split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("//") || trimmed.startsWith("#")) continue;

    if (MAIN_DECK_HEADER.test(trimmed)) {
      parsingMainDeck = true;
      continue;
    }

    if (EXTRA_DECK_HEADER.test(trimmed)) {
      parsingMainDeck = false;
      continue;
    }

    if (!parsingMainDeck) continue;

    const match = trimmed.match(CARD_LINE);
    if (!match) continue;

    const count = parseInt(match[1], 10);
    const name = match[2].replace(/\s*\([A-Z0-9]+\)\s*\d*\*?\s*$/, "").trim();
    for (let i = 0; i < count; i += 1) {
      cards.push(name);
    }
  }

  return cards;
}

export function isLobbyDeckReady(deck) {
  return Array.isArray(deck) && deck.length === LOBBY_DECK_SIZE;
}
