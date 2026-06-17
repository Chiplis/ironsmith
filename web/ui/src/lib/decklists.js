export const MATCH_FORMAT_NORMAL = "normal";
export const MATCH_FORMAT_COMMANDER = "commander";
export const LOBBY_DECK_SIZE = 60;
export const COMMANDER_DECK_SIZE = 99;
export const PARTNER_DECK_SIZE = 98;
const SAVED_DECK_PRESETS_STORAGE_KEY = "ironsmith.savedDeckPresets";

const MAIN_DECK_HEADER = /^Deck$/i;
const COMMANDER_HEADER = /^(Commander|Commanders)$/i;
const EXTRA_DECK_HEADER = /^(Sideboard|Companion|Maybeboard)$/i;
const CARD_LINE = /^(\d+)x?\s+(.+)$/;
const SET_COLLECTOR_SUFFIX =
  /\s*(?:\((?<parenSet>[a-z0-9]{2,8})\)|\[(?<bracketSet>[a-z0-9]{2,8})\])\s*(?<collector>\d+[a-z]?\*?)?\s*$/i;
const SET_COLLECTOR_COMPACT_SUFFIX =
  /\s*\[(?<set>[a-z0-9]{2,8}):(?<collector>\d+[a-z]?\*?)\]\s*$/i;

function normalizeCardName(raw) {
  return String(raw || "")
    .replace(/\s*(?:\*F\*|foil|nonfoil|etched|★)\s*$/i, "")
    .replace(/\s*(?:\([a-z0-9]{2,8}\)|\[[a-z0-9]{2,8}\])\s*\d*[a-z]?\*?\s*$/i, "")
    .replace(/\s*\[[a-z0-9]{2,8}:\d+[a-z]?\]\s*$/i, "")
    .trim();
}

function parseCardPrintPreference(raw) {
  const source = String(raw || "")
    .replace(/\s*(?:\*F\*|foil|nonfoil|etched|★)\s*$/i, "")
    .trim();
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

  const name = normalizeCardName(raw);
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

function normalizeDeckPresetName(raw) {
  return String(raw || "").trim();
}

function sanitizeDeckPresetTexts(texts) {
  if (!Array.isArray(texts)) return [];
  return texts.map((text) => String(text || ""));
}

function canUseLocalStorage() {
  return typeof window !== "undefined" && typeof window.localStorage !== "undefined";
}

function readSavedDeckPresets() {
  if (!canUseLocalStorage()) return [];

  try {
    const raw = window.localStorage.getItem(SAVED_DECK_PRESETS_STORAGE_KEY);
    if (!raw) return [];

    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];

    return parsed
      .map((entry) => {
        const name = normalizeDeckPresetName(entry?.name);
        if (!name) return null;
        return {
          name,
          texts: sanitizeDeckPresetTexts(entry?.texts),
          updatedAt: Number(entry?.updatedAt) || 0,
        };
      })
      .filter(Boolean)
      .sort((left, right) => {
        if (right.updatedAt !== left.updatedAt) return right.updatedAt - left.updatedAt;
        return left.name.localeCompare(right.name);
      });
  } catch (error) {
    console.warn("Failed to read saved deck presets:", error);
    return [];
  }
}

function writeSavedDeckPresets(entries) {
  if (!canUseLocalStorage()) return;

  try {
    window.localStorage.setItem(
      SAVED_DECK_PRESETS_STORAGE_KEY,
      JSON.stringify(entries)
    );
  } catch (error) {
    console.warn("Failed to write saved deck presets:", error);
  }
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

    if (COMMANDER_HEADER.test(trimmed) || EXTRA_DECK_HEADER.test(trimmed)) {
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

export function parseSideboardList(text) {
  const cards = [];
  let parsingSideboard = false;

  for (const line of String(text || "").split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("//") || trimmed.startsWith("#")) continue;

    if (MAIN_DECK_HEADER.test(trimmed) || COMMANDER_HEADER.test(trimmed)) {
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

export function parseCommanderList(text) {
  const cards = [];

  for (const line of String(text || "").split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("//") || trimmed.startsWith("#")) continue;
    if (
      MAIN_DECK_HEADER.test(trimmed)
      || COMMANDER_HEADER.test(trimmed)
      || EXTRA_DECK_HEADER.test(trimmed)
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

export function listSavedDeckPresets() {
  return readSavedDeckPresets();
}

export function findSavedDeckPreset(name) {
  const normalizedName = normalizeDeckPresetName(name);
  if (!normalizedName) return null;

  const expectedKey = normalizedName.toLowerCase();
  return (
    readSavedDeckPresets().find(
      (entry) => entry.name.toLowerCase() === expectedKey
    ) || null
  );
}

export function saveSavedDeckPreset(name, texts) {
  const normalizedName = normalizeDeckPresetName(name);
  if (!normalizedName) {
    return {
      saved: false,
      replaced: false,
      entry: null,
      entries: readSavedDeckPresets(),
    };
  }

  const now = Date.now();
  const nextEntry = {
    name: normalizedName,
    texts: sanitizeDeckPresetTexts(texts),
    updatedAt: now,
  };
  const normalizedKey = normalizedName.toLowerCase();
  const entries = readSavedDeckPresets();
  const existingIndex = entries.findIndex(
    (entry) => entry.name.toLowerCase() === normalizedKey
  );
  const replaced = existingIndex >= 0;

  if (replaced) {
    entries.splice(existingIndex, 1);
  }
  entries.unshift(nextEntry);
  writeSavedDeckPresets(entries);

  return {
    saved: true,
    replaced,
    entry: nextEntry,
    entries,
  };
}

export function normalizeMatchFormat(raw) {
  return raw === MATCH_FORMAT_COMMANDER
    ? MATCH_FORMAT_COMMANDER
    : MATCH_FORMAT_NORMAL;
}

export function evaluateLobbyDeckSubmission(format, deck, commanders = []) {
  const normalizedFormat = normalizeMatchFormat(format);
  const deckCount = Array.isArray(deck) ? deck.length : 0;
  const commanderCount = Array.isArray(commanders) ? commanders.length : 0;

  if (normalizedFormat === MATCH_FORMAT_COMMANDER) {
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

  return {
    ready: deckCount === LOBBY_DECK_SIZE,
    deckCount,
    commanderCount,
    requiredDeckCount: LOBBY_DECK_SIZE,
  };
}

export function isLobbyDeckReady(deck) {
  return Array.isArray(deck) && deck.length === LOBBY_DECK_SIZE;
}
