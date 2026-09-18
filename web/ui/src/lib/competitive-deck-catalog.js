/**
 * Source-neutral contract for competitive deck snapshots.
 *
 * The browser consumes a cached catalog, never scrapes a tournament site
 * directly. Source adapters can normalize their records into this shape.
 */
export const COMPETITIVE_DECK_CATALOG_VERSION = 1;
export const DEFAULT_COMPETITIVE_DECK_CATALOG_PATH = "decks/modern.json";

// The site is served from a subdirectory, so a root-absolute path would miss.
function assetBaseUrl() {
  const configured = typeof import.meta !== "undefined" ? import.meta.env?.BASE_URL : null;
  return new URL(configured || "/", globalThis?.location?.href || "http://localhost/").href;
}

export const DEFAULT_COMPETITIVE_DECK_CATALOG_URL = new URL(
  DEFAULT_COMPETITIVE_DECK_CATALOG_PATH,
  assetBaseUrl(),
).href;

function text(value) {
  return String(value ?? "").trim();
}

function positiveInteger(value) {
  const parsed = Number(value);
  return Number.isInteger(parsed) && parsed > 0 ? parsed : 0;
}

function normalizeCards(cards) {
  if (!Array.isArray(cards)) return [];

  return cards
    .map((card) => ({
      name: text(card?.name),
      count: positiveInteger(card?.count),
      scryfallId: text(card?.scryfallId || card?.scryfall_id),
      imageUrl: text(card?.imageUrl || card?.image_url),
    }))
    .filter((card) => card.name && card.count);
}

export function normalizeCompetitiveDeck(raw) {
  if (!raw || typeof raw !== "object") return null;

  const mainboard = normalizeCards(raw.mainboard || raw.mainBoard);
  if (!text(raw.id) || !mainboard.length) return null;

  return {
    id: text(raw.id),
    format: text(raw.format).toLowerCase(),
    name: text(raw.name || raw.archetype || "Unnamed deck"),
    archetype: text(raw.archetype),
    source: text(raw.source),
    sourceUrl: text(raw.sourceUrl || raw.source_url),
    event: text(raw.event),
    date: text(raw.date),
    placement: positiveInteger(raw.placement),
    record: text(raw.record),
    tags: Array.isArray(raw.tags) ? raw.tags.map(text).filter(Boolean) : [],
    mainboard,
    sideboard: normalizeCards(raw.sideboard || raw.sideBoard),
    commander: normalizeCards(raw.commander),
    updatedAt: text(raw.updatedAt || raw.updated_at),
  };
}

export function normalizeCompetitiveDeckCatalog(raw) {
  const source = raw && typeof raw === "object" ? raw : {};
  const decks = Array.isArray(source.decks)
    ? source.decks.map(normalizeCompetitiveDeck).filter(Boolean)
    : [];

  return {
    version: Number(source.version) || COMPETITIVE_DECK_CATALOG_VERSION,
    format: text(source.format).toLowerCase(),
    generatedAt: text(source.generatedAt || source.generated_at),
    sources: Array.isArray(source.sources)
      ? source.sources.map(text).filter(Boolean)
      : [],
    decks,
  };
}

export async function loadCompetitiveDeckCatalog({
  url = DEFAULT_COMPETITIVE_DECK_CATALOG_URL,
  fetchImpl = globalThis.fetch,
  signal,
} = {}) {
  if (typeof fetchImpl !== "function") {
    throw new Error("A fetch implementation is required to load the deck catalog");
  }

  const response = await fetchImpl(url, { signal });
  if (!response?.ok) {
    throw new Error(`Deck catalog request failed (${response?.status || "unknown"})`);
  }

  return normalizeCompetitiveDeckCatalog(await response.json());
}

function searchableText(deck) {
  return [
    deck.name,
    deck.archetype,
    deck.source,
    deck.event,
    deck.record,
    ...deck.tags,
    ...deck.mainboard.map((card) => card.name),
    ...deck.sideboard.map((card) => card.name),
  ].join(" ").toLocaleLowerCase();
}

export function searchCompetitiveDecks(
  catalog,
  { query = "", format = "", source = "", limit = 50 } = {},
) {
  const normalizedCatalog = normalizeCompetitiveDeckCatalog(catalog);
  const normalizedQuery = text(query).toLocaleLowerCase();
  const normalizedFormat = text(format).toLocaleLowerCase();
  const normalizedSource = text(source).toLocaleLowerCase();
  const maxResults = Math.max(0, Number(limit) || 50);

  return normalizedCatalog.decks
    .filter((deck) => !normalizedFormat || deck.format === normalizedFormat)
    .filter((deck) => !normalizedSource || deck.source.toLocaleLowerCase() === normalizedSource)
    .filter((deck) => !normalizedQuery || searchableText(deck).includes(normalizedQuery))
    .sort((left, right) => {
      if (left.placement && right.placement && left.placement !== right.placement) {
        return left.placement - right.placement;
      }
      return right.date.localeCompare(left.date);
    })
    .slice(0, maxResults);
}

function formatCards(section, cards) {
  if (!cards.length) return [];
  return [section, ...cards.map((card) => `${card.count} ${card.name}`)];
}

export function competitiveDeckToLobbyText(deck) {
  const normalizedDeck = normalizeCompetitiveDeck(deck);
  if (!normalizedDeck) return { deckText: "", commanderText: "" };

  const deckLines = [
    ...formatCards("Deck", normalizedDeck.mainboard),
    ...formatCards("Sideboard", normalizedDeck.sideboard),
  ];
  const commanderText = formatCards("Commander", normalizedDeck.commander).join("\n");

  return {
    deckText: deckLines.join("\n"),
    commanderText,
  };
}
