import { normalizeCardList, text } from "../catalog-utils.mjs";

// Archidekt's numeric deck-format ids for the formats MTGTop8 does not cover.
export const ARCHIDEKT_FORMATS = Object.freeze({
  paupercommander: 17,
});

export function archidektFormatId(format) {
  const id = ARCHIDEKT_FORMATS[text(format).toLocaleLowerCase("en-US")];
  if (!id) throw new Error(`Unsupported Archidekt deck format: ${format}`);
  return id;
}

export function deckSearchUrl({ format, page = 1, pageSize = 50, orderBy = "-viewCount" } = {}) {
  const params = new URLSearchParams({
    deckFormat: String(archidektFormatId(format)),
    orderBy,
    pageSize: String(pageSize),
    page: String(Math.max(1, Number(page) || 1)),
  });
  return `https://archidekt.com/api/decks/v3/?${params.toString()}`;
}

export function deckDetailUrl(id) {
  return `https://archidekt.com/api/decks/${encodeURIComponent(id)}/`;
}

// One page of the public deck search, reduced to what the catalog keeps.
export function parseDeckSearch(json) {
  const payload = typeof json === "string" ? JSON.parse(json) : json;
  return {
    hasNext: Boolean(payload?.next),
    decks: (Array.isArray(payload?.results) ? payload.results : [])
      .filter((deck) => deck && !deck.private && !deck.unlisted && !deck.theorycrafted)
      .map((deck) => ({
        id: String(deck.id),
        name: text(deck.name),
        size: Number(deck.size) || 0,
        viewCount: Number(deck.viewCount) || 0,
        updatedAt: text(deck.updatedAt),
      })),
  };
}

function cardName(entry) {
  const oracle = entry?.card?.oracleCard;
  // Double-faced cards are catalogued by their front face, like tournament lists.
  return text(oracle?.faces?.[0]?.name || oracle?.name || entry?.card?.displayName);
}

// Archidekt puts each card in user-defined categories; the first one is the
// card's primary category, and a category flagged `includedInDeck: false`
// (the Maybeboard, by default) is not part of the deck at all.
export function parseDeckDetail(json, { format, collections = [] } = {}) {
  const deck = typeof json === "string" ? JSON.parse(json) : json;
  const excluded = new Set((deck?.categories || [])
    .filter((category) => category && category.includedInDeck === false)
    .map((category) => text(category.name)));
  const premier = new Set((deck?.categories || [])
    .filter((category) => category?.isPremier)
    .map((category) => text(category.name)));
  const mainboard = [];
  const commander = [];
  const sideboard = [];
  for (const entry of Array.isArray(deck?.cards) ? deck.cards : []) {
    if (entry?.deletedAt) continue;
    const category = text(entry?.categories?.[0]);
    if (excluded.has(category)) continue;
    const name = cardName(entry);
    const count = Number(entry?.quantity);
    if (!name || !Number.isInteger(count) || count < 1) continue;
    if (premier.has(category) || category === "Commander") commander.push({ name, count });
    else if (category === "Sideboard") sideboard.push({ name, count });
    else mainboard.push({ name, count });
  }
  const name = text(deck?.name);
  const date = text(deck?.updatedAt || deck?.createdAt).slice(0, 10);
  return {
    id: `archidekt-${deck?.id}`,
    format,
    name,
    archetype: name,
    event: "",
    date,
    placement: null,
    source: "archidekt",
    sourceUrl: `https://archidekt.com/decks/${deck?.id}`,
    collections,
    mainboard: normalizeCardList(mainboard),
    sideboard: normalizeCardList(sideboard),
    commander: normalizeCardList(commander),
    tags: [...new Set([
      text(deck?.owner?.username) && `by ${text(deck.owner.username)}`,
      ...(Array.isArray(deck?.deckTags) ? deck.deckTags.map((tag) => text(tag?.name ?? tag)) : []),
    ].filter(Boolean))],
  };
}

let previousRequestAt = 0;

// Archidekt rate-limits bursts, so requests are spaced out and a 429 waits
// for as long as the response's Retry-After asks.
export async function fetchJson(url, {
  fetchImpl = globalThis.fetch,
  minDelayMs = 1000,
  attempts = 5,
  sleep = (duration) => new Promise((resolve) => setTimeout(resolve, duration)),
} = {}) {
  for (let attempt = 1; ; attempt += 1) {
    const remaining = Math.max(0, minDelayMs - (Date.now() - previousRequestAt));
    if (remaining) await sleep(remaining);
    previousRequestAt = Date.now();
    const response = await fetchImpl(url, {
      signal: AbortSignal.timeout(60_000),
      headers: { "user-agent": "IronSmith deck catalog sync", accept: "application/json" },
    });
    if (response.ok) return response.json();
    if ((response.status !== 429 && response.status < 500) || attempt >= attempts) {
      throw new Error(`Archidekt request failed (${response.status})`);
    }
    await sleep(1000 * (Number(response.headers?.get?.("retry-after")) || 30 * attempt));
  }
}
