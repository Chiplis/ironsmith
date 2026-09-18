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
    colors: [...new Set((Array.isArray(raw.colors) ? raw.colors : []).map((color) => text(color).toUpperCase()).filter((color) => /^[WUBRGC]$/.test(color)))].sort(),
    mechanics: [...new Set((Array.isArray(raw.mechanics) ? raw.mechanics : []).map(text).filter(Boolean))].sort(),
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
