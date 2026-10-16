import { parseDeckList, parseSideboardList, parseCommanderList } from './decklists.js';
function parseCommanderSection(text) {
  const section = String(text || '').match(/(?:^|\n)Commanders?\s*\n([\s\S]*?)(?=\n(?:Deck|Sideboard)\s*(?:\n|$)|$)/i);
  return section ? parseCommanderList(section[1]) : [];
}
const PLAYED_KEY = 'ironsmith.playedDecks.v1';
const CHOICES_KEY = 'ironsmith.deckChoices.v1';
function read(key) {
  try { const value = JSON.parse(localStorage.getItem(key) || '[]'); return Array.isArray(value) ? value.filter(x => x && typeof x.deckText === 'string') : []; } catch { return []; }
}
function write(key, entries) { try { localStorage.setItem(key, JSON.stringify(entries)); } catch { /* Storage can be unavailable or full. */ } }
export function deckIdentity({ deckText = '', commanderText = '' }) {
  return JSON.stringify([parseDeckList(deckText).sort(), parseSideboardList(deckText).sort(), [...parseCommanderSection(deckText), ...parseDeckList(commanderText)].sort()]);
}
export function rememberDeckChoice(deck, entry) {
  const next = { ...entry, deckText: deck.deckText, commanderText: deck.commanderText || '' };
  const identity = deckIdentity(next);
  write(CHOICES_KEY, [next, ...read(CHOICES_KEY).filter(x => deckIdentity(x) !== identity)].slice(0, 30));
}
export function recordPlayedDeck(deck) {
  if (!parseDeckList(deck?.deckText).length) return;
  const identity = deckIdentity(deck);
  const metadata = read(CHOICES_KEY).find(x => deckIdentity(x) === identity);
  const cards = parseDeckList(deck.deckText);
  const artCard = cards.find(name => !/^(Plains|Island|Swamp|Mountain|Forest|Wastes|Snow-Covered .+)$/i.test(name)) || cards[0];
  const next = { name: deck.name || artCard, artCard, ...metadata, ...deck, id: `played-${identity}`, event: 'Recently played', playedAt: Date.now() };
  write(PLAYED_KEY, [next, ...read(PLAYED_KEY).filter(x => deckIdentity(x) !== identity)].slice(0, 3));
}
export function readRecentlyPlayedDecks() { return read(PLAYED_KEY).slice(0, 3); }
export function pickFeaturedDecks(recent, catalog, random = Math.random) {
  const choices = [...catalog];
  for (let i = choices.length - 1; i > 0; i--) { const j = Math.floor(random() * (i + 1)); [choices[i], choices[j]] = [choices[j], choices[i]]; }
  const candidates = choices.filter(x => !recent.some(r => r.catalogId === x.id || r.id === x.id));
  const result = [...recent];
  const nameKey = x => String(x.archetype || x.name || x.id).toLowerCase();
  for (const entry of candidates) {
    if (result.length >= 3) break;
    if (!result.some(x => nameKey(x) === nameKey(entry))) result.push(entry);
  }
  return [...result, ...candidates.filter(x => !result.includes(x))].slice(0, 3);
}
