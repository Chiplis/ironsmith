import { cardRouteKey } from "./scryfall.js";
import { baseAssetUrl } from "./asset-base.js";
import { versionedCardAssetUrl } from "./card-asset-cache.js";

// A card the engine cannot load still has to occupy its slot in the deck, so
// it becomes a basic land of the colour it leaned on most rather than blocking
// the match. Ties go in WUBRG order, the order Magic prints its colours in.
const COLOR_ORDER = ["W", "U", "B", "R", "G"];
const BASIC_LANDS = { W: "Plains", U: "Island", B: "Swamp", R: "Mountain", G: "Forest", C: "Wastes" };

export function dominantManaColor(manaCost) {
  const counts = {};
  // Split and modal cards print both halves; the front face is the one cast.
  const front = String(manaCost || "").split("//")[0];
  for (const match of front.matchAll(/\{([^}]+)\}/g)) {
    // A hybrid symbol can be paid either way, so it counts for both colours.
    for (const symbol of match[1].toUpperCase().split("/")) {
      if (COLOR_ORDER.includes(symbol) || symbol === "C") counts[symbol] = (counts[symbol] || 0) + 1;
    }
  }
  const colored = COLOR_ORDER.filter((color) => counts[color]);
  if (!colored.length) return counts.C ? "C" : "";
  const most = Math.max(...colored.map((color) => counts[color]));
  return colored.find((color) => counts[color] === most);
}

export function basicLandForManaCost(manaCost) {
  return BASIC_LANDS[dominantManaColor(manaCost)] || BASIC_LANDS.C;
}

// Keyed by route and holding the request, so a deck that repeats an unknown
// card fetches its printing once.
const manaCostRequests = new Map();

export async function loadCardManaCost(cardName, { fetchImpl = globalThis.fetch } = {}) {
  const route = cardRouteKey(cardName);
  if (!route || typeof fetchImpl !== "function") return "";
  if (manaCostRequests.has(route)) return manaCostRequests.get(route);
  const request = Promise.resolve()
    .then(() => fetchImpl(versionedCardAssetUrl(new URL(`cards/${route}.json`, baseAssetUrl()).href), { cache: "force-cache" }))
    .then(async (response) => (response?.ok ? String((await response.json())?.scryfall?.mana_cost || "") : ""))
    // A card with no printing on hand tells us nothing about its colours; the
    // colourless basic is the honest stand-in.
    .catch(() => "");
  manaCostRequests.set(route, request);
  return request;
}

// Every decision made this session, so the reconnect, seat-assignment and
// host-takeover paths that re-derive a deck from its original text apply the
// same swap without having to ask the engine again.
const decidedSubstitutions = new Map();

export function applyKnownSubstitutions(cards) {
  if (!decidedSubstitutions.size || !Array.isArray(cards)) return cards;
  let changed = false;
  const swapped = cards.map((name) => {
    const replacement = decidedSubstitutions.get(String(name || "").trim());
    if (!replacement) return name;
    changed = true;
    return replacement;
  });
  return changed ? swapped : cards;
}

/**
 * Replace every card the engine cannot load with a basic land, keeping each
 * list the same length so deck sizes still validate.
 */
export async function substituteUnsupportedCards(submission, { game, fetchImpl } = {}) {
  const deck = Array.isArray(submission?.deck) ? submission.deck : [];
  const sideboard = Array.isArray(submission?.sideboard) ? submission.sideboard : [];
  const empty = { deck, sideboard, substitutions: [] };
  if (!game || typeof game.filterKnownCardNames !== "function") return empty;

  const names = [...new Set([...deck, ...sideboard].map((name) => String(name || "").trim()).filter(Boolean))];
  if (!names.length) return empty;
  const known = new Set(await game.filterKnownCardNames(names));
  const unsupported = names.filter((name) => !known.has(name));
  if (!unsupported.length) return empty;

  const replacements = new Map();
  for (const name of unsupported) {
    const replacement = basicLandForManaCost(await loadCardManaCost(name, { fetchImpl }));
    replacements.set(name, replacement);
    decidedSubstitutions.set(name, replacement);
  }
  return {
    deck: applyKnownSubstitutions(deck),
    sideboard: applyKnownSubstitutions(sideboard),
    substitutions: [...replacements.entries()].map(([from, to]) => ({ from, to })),
  };
}

export function describeSubstitutions(substitutions) {
  const entries = Array.isArray(substitutions) ? substitutions : [];
  if (!entries.length) return "";
  const shown = entries.slice(0, 3).map(({ from, to }) => `${from} → ${to}`).join(", ");
  const extra = entries.length - 3;
  return extra > 0 ? `${shown} +${extra} more` : shown;
}

/**
 * A parsed submission with every unloadable card already swapped, ready to be
 * committed. Every path that turns deck text into a player's deck goes through
 * this, so an audit manifest is always built over what will actually be played.
 */
export async function withSupportedCards(parsed, { game, onSubstitute } = {}) {
  const substituted = await substituteUnsupportedCards(parsed, { game });
  if (!substituted.substitutions.length) return parsed;
  onSubstitute?.(substituted.substitutions);
  return { ...parsed, deck: substituted.deck, sideboard: substituted.sideboard };
}

// Tests only: the decisions are per engine build and otherwise live for the
// session.
export function resetKnownSubstitutions() {
  decidedSubstitutions.clear();
}
