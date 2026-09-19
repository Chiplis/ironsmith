import {
  deckTokenValues,
  normalizeCardList,
  shortHash,
  slugify,
  text,
} from "./catalog-utils.mjs";

export class DeckValidationError extends Error {
  constructor(errors) {
    super(`Invalid competitive deck: ${errors.join("; ")}`);
    this.name = "DeckValidationError";
    this.errors = errors;
  }
}

function positivePlacement(value) {
  if (value === null || value === undefined || value === "") return null;
  const placement = Number(value);
  return Number.isInteger(placement) && placement > 0 ? placement : null;
}

function normalizeManaProfile(raw) {
  if (!raw || typeof raw !== "object") return null;
  const counts = (value) => Object.fromEntries(
    Object.entries(value && typeof value === "object" ? value : {})
      .filter(([color, count]) => /^[WUBRGC]$/.test(color) && Number(count) > 0)
      .map(([color, count]) => [color, Number(count)]),
  );
  const coverage = raw.metadataCoverage && typeof raw.metadataCoverage === "object"
    ? raw.metadataCoverage
    : {};
  return {
    colors: [...new Set((Array.isArray(raw.colors) ? raw.colors : []).map((color) => text(color).toUpperCase()).filter((color) => /^[WUBRGC]$/.test(color)))].sort(),
    pipCounts: counts(raw.pipCounts),
    sourceCounts: counts(raw.sourceCounts),
    landCount: Number.isInteger(Number(raw.landCount)) && Number(raw.landCount) >= 0 ? Number(raw.landCount) : 0,
    predominantColors: [...new Set((Array.isArray(raw.predominantColors) ? raw.predominantColors : []).map((color) => text(color).toUpperCase()).filter((color) => /^[WUBRGC]$/.test(color)))].sort(),
    predominantLands: (Array.isArray(raw.predominantLands) ? raw.predominantLands : [])
      .map((land) => ({ name: text(land?.name), count: Number(land?.count) }))
      .filter((land) => land.name && Number.isInteger(land.count) && land.count > 0),
    metadataCoverage: {
      source: text(coverage.source),
      resolvedCardCount: Number(coverage.resolvedCardCount) || 0,
      totalCardCount: Number(coverage.totalCardCount) || 0,
      unresolvedCardNames: [...new Set((Array.isArray(coverage.unresolvedCardNames) ? coverage.unresolvedCardNames : []).map(text).filter(Boolean))].sort(),
      complete: coverage.complete === true,
    },
  };
}

function canonicalDeck(raw) {
  const format = text(raw.format).toLocaleLowerCase("en-US");
  const source = text(raw.source).toLocaleLowerCase("en-US");
  const event = text(raw.event);
  const date = text(raw.date);
  const placement = positivePlacement(raw.placement);
  const mainboard = normalizeCardList(raw.mainboard || raw.mainBoard);
  const sideboard = normalizeCardList(raw.sideboard || raw.sideBoard);
  const commander = normalizeCardList(raw.commander);
  const tags = [...new Set((Array.isArray(raw.tags) ? raw.tags : []).map(text).filter(Boolean))].sort();
  const collections = [...new Set((Array.isArray(raw.collections) ? raw.collections : [])
    .map(text)
    .map((value) => value.toLocaleLowerCase("en-US"))
    .filter((value) => /^[a-z0-9-]+$/.test(value)))].sort();
  const manaProfile = normalizeManaProfile(raw.manaProfile);
  const identity = {
    format,
    source,
    event,
    date,
    placement,
    mainboard,
    sideboard,
    commander,
  };
  const id = text(raw.id) || [format, source, slugify(event), date, placement || "deck", shortHash(identity)].filter(Boolean).join("-");
  return {
    id,
    format,
    name: text(raw.name || raw.archetype),
    archetype: text(raw.archetype),
    colors: [...new Set((Array.isArray(raw.colors) && raw.colors.length ? raw.colors : (manaProfile?.colors || [])).map((color) => text(color).toUpperCase()).filter((color) => /^[WUBRGC]$/.test(color)))].sort(),
    mechanics: [...new Set((Array.isArray(raw.mechanics) ? raw.mechanics : []).map(text).filter(Boolean))].sort(),
    collections,
    ...(manaProfile ? { manaProfile } : {}),
    cardNames: [...new Set([
      ...mainboard.map((card) => card.name),
      ...sideboard.map((card) => card.name),
      ...commander.map((card) => card.name),
    ])].sort(),
    event,
    date,
    placement,
    source,
    sourceUrl: text(raw.sourceUrl || raw.source_url),
    mainboard,
    sideboard,
    commander,
    tags,
    hash: shortHash(identity),
    _searchValues: deckTokenValues({ id, format, name: text(raw.name || raw.archetype), archetype: text(raw.archetype), event, source, tags, mainboard, sideboard }),
  };
}

export function validateCompetitiveDeck(deck) {
  const errors = [];
  if (!deck.id) errors.push("id is required");
  if (!deck.format) errors.push("format is required");
  if (!deck.source) errors.push("source is required");
  if (!deck.mainboard.length) errors.push("mainboard must contain at least one card");
  if (!/^[a-f0-9]{16}$/.test(deck.hash)) errors.push("hash must be a 16-character lowercase hex string");
  for (const section of ["mainboard", "sideboard", "commander"]) {
    if (deck[section].some((card) => !card.name || !Number.isInteger(card.count) || card.count < 1)) {
      errors.push(`${section} contains an invalid card`);
    }
  }
  return errors;
}

export function normalizeCompetitiveDeck(raw) {
  if (!raw || typeof raw !== "object") throw new DeckValidationError(["record must be an object"]);
  const deck = canonicalDeck(raw);
  const errors = validateCompetitiveDeck(deck);
  if (errors.length) throw new DeckValidationError(errors);
  const { _searchValues, ...publicDeck } = deck;
  return publicDeck;
}

export function normalizeCompetitiveDecks(records) {
  if (!Array.isArray(records)) throw new TypeError("deck records must be an array");
  const unique = new Map();
  for (const record of records) {
    const deck = normalizeCompetitiveDeck(record);
    unique.set(`${deck.source}:${deck.hash}`, deck);
  }
  return [...unique.values()].sort((left, right) => {
    const dateOrder = right.date.localeCompare(left.date);
    return dateOrder || (left.placement || Number.MAX_SAFE_INTEGER) - (right.placement || Number.MAX_SAFE_INTEGER);
  });
}
