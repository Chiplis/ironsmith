import { PUBLIC_FORMATS, usesCommanders } from './relay/formats.js';
import { deckCardBaseName } from './deck-color-notes.js';
export const MATCH_FORMAT_NORMAL = "normal";
export const MATCH_FORMAT_COMMANDER = "commander";
export const MATCH_FORMAT_PLANECHASE = "planechase";
export const LOBBY_DECK_SIZE = 60;
export const COMMANDER_DECK_SIZE = 99;
export const PARTNER_DECK_SIZE = 98;
export { listSavedDeckPresets, findSavedDeckPreset, saveSavedDeckPreset, removeSavedDeckPreset } from "./saved-decks.js";
const DEFAULT_LOBBY_DECK_STORAGE_KEY = "ironsmith.defaultLobbyDeck.v1";

const MAIN_DECK_HEADER = /^Deck$/i;
const COMMANDER_HEADER = /^(Commander|Commanders)$/i;
const EXTRA_DECK_HEADER = /^(Sideboard|Companion|Maybeboard)\s*:?$/i;
// Unfinity sticker sheets (CR 123.2) a player brings. Sheets are not cards.
const STICKERS_HEADER = /^Stickers$/i;
const CARD_LINE = /^(\d+)x?\s+(.+)$/;
const SET_COLLECTOR_SUFFIX =
  /\s*(?:\((?<parenSet>[a-z0-9]{2,8})\)|\[(?<bracketSet>[a-z0-9]{2,8})\])\s*(?<collector>\d+[a-z]?\*?)?\s*$/i;
const SET_COLLECTOR_COMPACT_SUFFIX =
  /\s*\[(?<set>[a-z0-9]{2,8}):(?<collector>\d+[a-z]?\*?)\]\s*$/i;

// Finish markers after a card name. The words need a space before them so the
// cards "Foil" and "Lavinia, Foil to Conspiracy" keep their names; "$foil"
// is the table's own foil marker and may sit anywhere on the line.
const FINISH_SUFFIX = /(?:\s*(?:\*F\*|★)|\s+(?:foil|nonfoil|etched))\s*$/i;
const FOIL_TAG = /\s*\$foil\b\s*/gi;

function stripFinishMarkers(raw) {
  return String(raw || "").replace(FOIL_TAG, " ").trim().replace(FINISH_SUFFIX, "");
}

function normalizeCardName(raw) {
  return stripFinishMarkers(raw)
    .replace(/\s*(?:\([a-z0-9]{2,8}\)|\[[a-z0-9]{2,8}\])\s*\d*[a-z]?\*?\s*$/i, "")
    .replace(/\s*\[[a-z0-9]{2,8}:\d+[a-z]?\]\s*$/i, "")
    // MTGO exports use a single slash; other exports use two. The registry
    // identifies these combined cards by their first face or half.
    .split(/\s*\/+\s*/, 1)[0]
    .trim();
}

function parseCardPrintPreference(raw) {
  const source = stripFinishMarkers(raw).trim();
  if (!source) return null;

  const compactMatch = source.match(SET_COLLECTOR_COMPACT_SUFFIX);
  const suffixMatch = compactMatch || source.match(SET_COLLECTOR_SUFFIX);
  if (!suffixMatch?.groups) return null;

  const setCode = String(
    suffixMatch.groups.set
      || suffixMatch.groups.parenSet
      || suffixMatch.groups.bracketSet
      || ""
  ).trim();
  if (!setCode) return null;

  // Printings belong to the card, not to its circled-color note.
  const name = deckCardBaseName(normalizeCardName(raw));
  if (!name) return null;

  const collectorNumber = String(suffixMatch.groups.collector || "")
    .replace(/\*$/, "")
    .trim();
  return {
    name,
    setCode,
    collectorNumber,
  };
}

function canUseLocalStorage() {
  return typeof window !== "undefined" && typeof window.localStorage !== "undefined";
}

function normalizeLobbyDeckDefault(entry) {
  if (!entry || typeof entry !== "object") {
    return {
      deckText: "",
      commanderText: "",
      updatedAt: 0,
    };
  }

  return {
    deckText: String(entry.deckText || ""),
    commanderText: String(entry.commanderText || ""),
    updatedAt: Number(entry.updatedAt) || 0,
  };
}

export function readDefaultLobbyDeck() {
  if (!canUseLocalStorage()) return normalizeLobbyDeckDefault(null);

  try {
    const raw = window.localStorage.getItem(DEFAULT_LOBBY_DECK_STORAGE_KEY);
    if (!raw) return normalizeLobbyDeckDefault(null);
    return normalizeLobbyDeckDefault(JSON.parse(raw));
  } catch (error) {
    console.warn("Failed to read default lobby deck:", error);
    return normalizeLobbyDeckDefault(null);
  }
}

export function saveDefaultLobbyDeck({ deckText = "", commanderText = "" } = {}) {
  if (!canUseLocalStorage()) return readDefaultLobbyDeck();

  const nextEntry = normalizeLobbyDeckDefault({
    deckText,
    commanderText,
    updatedAt: Date.now(),
  });
  if (!nextEntry.deckText.trim() && !nextEntry.commanderText.trim()) {
    return readDefaultLobbyDeck();
  }

  try {
    window.localStorage.setItem(
      DEFAULT_LOBBY_DECK_STORAGE_KEY,
      JSON.stringify(nextEntry)
    );
  } catch (error) {
    console.warn("Failed to write default lobby deck:", error);
  }

  return nextEntry;
}

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

    if (COMMANDER_HEADER.test(trimmed) || EXTRA_DECK_HEADER.test(trimmed) || STICKERS_HEADER.test(trimmed)) {
      parsingMainDeck = false;
      continue;
    }

    if (!parsingMainDeck) continue;

    const match = trimmed.match(CARD_LINE);
    if (!match) {
      cards.push(normalizeCardName(trimmed));
      continue;
    }

    const count = parseInt(match[1], 10);
    const name = normalizeCardName(match[2]);
    for (let i = 0; i < count; i += 1) {
      cards.push(name);
    }
  }

  return cards;
}

export function parseDeckPrintPreferences(text) {
  const preferences = [];

  for (const line of String(text || "").split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("//") || trimmed.startsWith("#")) continue;
    if (
      MAIN_DECK_HEADER.test(trimmed)
      || COMMANDER_HEADER.test(trimmed)
      || EXTRA_DECK_HEADER.test(trimmed)
      || STICKERS_HEADER.test(trimmed)
    ) {
      continue;
    }

    const match = trimmed.match(CARD_LINE);
    const rawName = match ? match[2] : trimmed;
    const preference = parseCardPrintPreference(rawName);
    if (preference) preferences.push(preference);
  }

  return preferences;
}

// Card lines marked foil: "$foil" anywhere on the line, or the "*F*" / "★"
// suffixes Moxfield and Arena exports write. A bare "foil" word is not a
// marker, since cards like "Foil" carry it in their names. normalizeCardName
// strips the markers, so the deck itself is unchanged.
const FOIL_MARKER = /\$foil\b|\*F\*\s*$|★\s*$/i;

export function parseDeckFoilNames(text) {
  const names = [];

  for (const line of String(text || "").split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("//") || trimmed.startsWith("#")) continue;
    if (
      MAIN_DECK_HEADER.test(trimmed)
      || COMMANDER_HEADER.test(trimmed)
      || EXTRA_DECK_HEADER.test(trimmed)
      || STICKERS_HEADER.test(trimmed)
    ) {
      continue;
    }

    const match = trimmed.match(CARD_LINE);
    const rawName = match ? match[2] : trimmed;
    if (!FOIL_MARKER.test(rawName)) continue;
    const name = deckCardBaseName(normalizeCardName(rawName));
    if (name) names.push(name);
  }

  return names;
}

export function parseSideboardList(text) {
  const cards = [];
  let parsingSideboard = false;

  for (const line of String(text || "").split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("//") || trimmed.startsWith("#")) continue;

    if (MAIN_DECK_HEADER.test(trimmed) || COMMANDER_HEADER.test(trimmed) || STICKERS_HEADER.test(trimmed)) {
      parsingSideboard = false;
      continue;
    }

    if (EXTRA_DECK_HEADER.test(trimmed)) {
      parsingSideboard = true;
      continue;
    }

    if (!parsingSideboard) continue;

    const match = trimmed.match(CARD_LINE);
    if (!match) {
      cards.push(normalizeCardName(trimmed));
      continue;
    }

    const count = parseInt(match[1], 10);
    const name = normalizeCardName(match[2]);
    for (let i = 0; i < count; i += 1) {
      cards.push(name);
    }
  }

  return cards;
}

// Sticker sheet names listed under a "Stickers" header, one per line in first
// seen order. A count prefix is accepted and ignored: each sheet is unique.
export function parseStickerList(text) {
  const sheets = [];
  let parsingStickers = false;

  for (const line of String(text || "").split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("//") || trimmed.startsWith("#")) continue;

    if (STICKERS_HEADER.test(trimmed)) {
      parsingStickers = true;
      continue;
    }
    if (MAIN_DECK_HEADER.test(trimmed) || COMMANDER_HEADER.test(trimmed) || EXTRA_DECK_HEADER.test(trimmed)) {
      parsingStickers = false;
      continue;
    }
    if (!parsingStickers) continue;

    const match = trimmed.match(CARD_LINE);
    const name = normalizeCardName(match ? match[2] : trimmed);
    if (name) sheets.push(name);
  }

  return sheets;
}

export function parseCommanderList(text) {
  const cards = [];

  for (const line of String(text || "").split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("//") || trimmed.startsWith("#")) continue;
    if (
      MAIN_DECK_HEADER.test(trimmed)
      || COMMANDER_HEADER.test(trimmed)
      || EXTRA_DECK_HEADER.test(trimmed)
      || STICKERS_HEADER.test(trimmed)
    ) {
      continue;
    }

    const match = trimmed.match(CARD_LINE);
    if (match) {
      const count = parseInt(match[1], 10);
      const name = normalizeCardName(match[2]);
      for (let i = 0; i < count; i += 1) {
        cards.push(name);
      }
      continue;
    }

    cards.push(normalizeCardName(trimmed));
  }

  return cards;
}

// The inverse of parseDeckList / parseSideboardList / parseStickerList: "N Name"
// lines, first seen order, with the sideboard and sticker sheets under their headers.
export function deckListText(deck = [], sideboard = [], stickers = []) {
  const lines = (cards) => {
    const counts = new Map();
    for (const card of cards || []) {
      const name = String(card || "").trim();
      if (name) counts.set(name, (counts.get(name) || 0) + 1);
    }
    return [...counts].map(([name, count]) => `${count} ${name}`);
  };
  const main = lines(deck);
  const side = lines(sideboard);
  const sheets = lines(stickers);
  const out = [...main];
  if (side.length) out.push("", "Sideboard", ...side);
  if (sheets.length) out.push("", "Stickers", ...sheets);
  return out.join("\n");
}

export function normalizeMatchFormat(raw) {
  if (typeof raw === 'string' && Object.hasOwn(PUBLIC_FORMATS, raw)) return raw;
  if (raw === MATCH_FORMAT_COMMANDER) return MATCH_FORMAT_COMMANDER;
  if (raw === MATCH_FORMAT_PLANECHASE) return MATCH_FORMAT_PLANECHASE;
  return MATCH_FORMAT_NORMAL;
}

export function evaluateLobbyDeckSubmission(format, deck, commanders = []) {
  const normalizedFormat = normalizeMatchFormat(format);
  const deckCount = Array.isArray(deck) ? deck.length : 0;
  const commanderCount = Array.isArray(commanders) ? commanders.length : 0;

  if (usesCommanders(normalizedFormat)) {
    const requiredDeckCount = commanderCount === 2 ? PARTNER_DECK_SIZE : COMMANDER_DECK_SIZE;
    const ready =
      (commanderCount === 1 || commanderCount === 2)
      && deckCount === requiredDeckCount;
    return {
      ready,
      deckCount,
      commanderCount,
      requiredDeckCount,
    };
  }

  if (normalizedFormat === MATCH_FORMAT_PLANECHASE) {
    const uniquePlanarCards = new Set(
      (commanders || []).map((name) => String(name || "").trim().toLowerCase())
    );
    const ready =
      deckCount >= LOBBY_DECK_SIZE
      && commanderCount >= 10
      && uniquePlanarCards.size === commanderCount;
    return {
      ready,
      deckCount,
      commanderCount,
      requiredDeckCount: LOBBY_DECK_SIZE,
    };
  }

  return {
    ready: PUBLIC_FORMATS[normalizedFormat] ? deckCount >= LOBBY_DECK_SIZE && commanderCount === 0 : deckCount >= LOBBY_DECK_SIZE,
    deckCount,
    commanderCount,
    requiredDeckCount: LOBBY_DECK_SIZE,
  };
}

export function isLobbyDeckReady(deck) {
  return Array.isArray(deck) && deck.length >= LOBBY_DECK_SIZE;
}

// Preserve section text (including print choices and sideboards) when saving a lobby deck.
export function serializeLobbyDeck(deckText, commanderText) {
  return commanderText.trim()
    ? `Commander\n${commanderText.trim()}\n\nDeck\n${deckText}`
    : deckText;
}

export function splitSavedLobbyDeck(text) {
  const main = [];
  const commanders = [];
  let inCommanders = false;
  for (const line of String(text || "").split("\n")) {
    if (COMMANDER_HEADER.test(line.trim())) { inCommanders = true; continue; }
    if (MAIN_DECK_HEADER.test(line.trim()) || EXTRA_DECK_HEADER.test(line.trim()) || STICKERS_HEADER.test(line.trim())) inCommanders = false;
    (inCommanders ? commanders : main).push(line);
  }
  return { deckText: main.join("\n").trim(), commanderText: commanders.join("\n").trim() };
}
