import { fetchScryfallApiJson } from "./scryfall.js";
// Resolve complete printing membership, rather than the one representative
// printing per oracle card in the engine's local catalog.
async function json(url) {
  const response = await fetchScryfallApiJson(url);
  const body = await response.json();
  if (!response.ok) throw new Error(body.details || 'Could not load official cards');
  return body;
}
export async function resolveCommunitySet(code) {
  code = String(code).trim().toLowerCase();
  if (!/^[a-z0-9]+$/.test(code)) throw new Error('Enter an official set code, such as dom');
  const set = await json(`https://api.scryfall.com/sets/${encodeURIComponent(code)}`);
  const cards = new Set();
  let url = `https://api.scryfall.com/cards/search?q=${encodeURIComponent(`set:${code}`)}&unique=cards`;
  while (url) {
    const page = await json(url);
    for (const card of page.data) cards.add(card.name);
    url = page.has_more ? page.next_page : null;
    if (url && !url.startsWith('https://api.scryfall.com/')) throw new Error('Invalid card catalog page');
  }
  return { code: set.code, name: set.name, cards: [...cards] };
}
export async function resolveCommunityCard(name) {
  const card = await json(`https://api.scryfall.com/cards/named?exact=${encodeURIComponent(name.trim())}`);
  return card.name;
}
