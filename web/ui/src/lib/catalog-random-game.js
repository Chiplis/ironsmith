import { loadCatalogIndex, loadCatalogDeckDetail } from './catalog-client.js';
import { loadRandomGameIndex, resolveNamedCards } from './random-game-catalog.js';
import { PUZZLE_VERSION } from './puzzles.js';
import { validateDeckCatalogEntry } from './deck-catalog-import.js';

function shuffled(values, rng) {
  const result = [...values];
  for (let i = result.length - 1; i > 0; i--) {
    const j = Math.floor(rng() * (i + 1));
    [result[i], result[j]] = [result[j], result[i]];
  }
  return result;
}
const key = name => String(name).trim().toLocaleLowerCase('en-US');

// Each physical copy is moved once; the rest of the selected deck stays in the
// library. Sideboard cards and synthetic mana sources never enter the position.
export function catalogDeckPosition(deck, cards, { rng = Math.random, name, life = 20 } = {}) {
  const byName = new Map(cards.map(card => [key(card.name), card]));
  const remaining = shuffled(deck.mainboard.flatMap(card =>
    Array.from({ length: card.count }, () => byName.get(key(card.name)))), rng);
  if (remaining.some(card => !card)) throw new Error('Deck contains unavailable cards');
  const zones = { battlefield: [], hand: [], library: [], graveyard: [], exile: [], command: [] };
  const take = (zone, count, accepts) => {
    for (let i = 0; i < remaining.length && zones[zone].length < count;) {
      const card = remaining[i];
      if (accepts(card)) {
        zones[zone].push(card.name);
        remaining.splice(i, 1);
      } else i++;
    }
  };
  const legendary = new Set();
  const keepLegend = card => {
    if (card.legendary && legendary.has(card.name)) return false;
    if (card.legendary) legendary.add(card.name);
    return true;
  };
  take('battlefield', 4, card => card.types.includes('Land') && keepLegend(card));
  const lands = zones.battlefield.length;
  take('battlefield', lands + 3, card => {
    // Auras, Battles and planeswalkers need attachment/counter setup. Keep the
    // seeded battlefield to permanents that can stand alone without that setup.
    if (!card.permanent || card.types.includes('Land') || card.manaValue > lands
        || !card.types.some(type => ['Creature', 'Artifact'].includes(type))) return false;
    return keepLegend(card);
  });
  take('hand', 7, () => true);
  take('graveyard', 3, card => !card.types.includes('Land'));
  zones.library = remaining.map(card => card.name);
  zones.command = (deck.commander || []).flatMap(card => Array(card.count).fill(byName.get(key(card.name))?.name || card.name));
  return { name, life, zones };
}

export async function buildCatalogRandomGame({
  playerNames = ['Alice', 'Bob'], startingLife = 20, minScore = 0.96,
  format = 'modern', rng = Math.random, fetchImpl = globalThis.fetch,
  signal, onProgress,
} = {}) {
  const [catalog, index] = await Promise.all([
    loadCatalogIndex({ format, fetchImpl }), loadRandomGameIndex({ fetchImpl }),
  ]);
  const supported = new Map(index.map(card => [key(card.name), card]));
  const selected = [];
  const entries = shuffled(catalog.decks || [], rng);
  for (let cursor = 0; cursor < entries.length && selected.length < 2; cursor += 12) {
    signal?.throwIfAborted();
    const details = await Promise.allSettled(entries.slice(cursor, cursor + 12).map(entry => loadCatalogDeckDetail(entry, { format, fetchImpl })));
    for (const result of details) {
      if (result.status !== 'fulfilled') continue;
      const deck = result.value;
      if (!validateDeckCatalogEntry(deck, { format }).valid) continue;
      const inventory = [...(deck.mainboard || []), ...(deck.commander || [])];
      if (!deck.mainboard?.length || inventory.some(card => !Number.isInteger(card.count) || card.count <= 0)) continue;
      if (!inventory.every(card => {
        const record = supported.get(key(card.name));
        return record && (minScore <= 0 || record.score >= minScore);
      })) continue;
      const cards = await resolveNamedCards(inventory.map(card => card.name), { fetchImpl });
      if (!inventory.every(card => cards.some(candidate => key(candidate.name) === key(card.name)))) continue;
      selected.push({ deck, cards });
      if (selected.length === 2) break;
    }
    onProgress?.({ collected: selected.length, target: 2 });
  }
  signal?.throwIfAborted();
  if (!selected.length) throw new Error(`No complete supported ${format} deck meets the selected card threshold`);
  // A small catalog may have only one supported deck; a mirror match is valid.
  if (selected.length === 1) selected.push(selected[0]);
  return {
    version: PUZZLE_VERSION,
    players: selected.map(({ deck, cards }, i) => catalogDeckPosition(deck, cards, {
      rng, name: playerNames[i] || `Player ${i + 1}`, life: startingLife,
    })),
  };
}
