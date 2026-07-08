export const PUZZLE_VERSION = 1;
const PUZZLE_DRAFT_STORAGE_KEY = "ironsmith.puzzleDraft";
export const PUZZLE_ZONE_ORDER = [
  "battlefield",
  "hand",
  "graveyard",
  "exile",
  "library",
  "command",
];

const CARD_LINE = /^(\d+)x?\s+(.+)$/;
const BASE64_ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

function normalizeCardName(raw) {
  return String(raw || "")
    .replace(/\s*\([A-Z0-9]+\)\s*\d*\*?\s*$/, "")
    .trim();
}

function normalizePlayerName(raw, index) {
  const trimmed = String(raw || "").trim();
  return trimmed || `Player ${index + 1}`;
}

function normalizePlayerLife(raw) {
  const parsed = Number(raw);
  return Number.isFinite(parsed) ? Math.trunc(parsed) : 20;
}

function normalizeZoneCards(rawCards) {
  if (typeof rawCards === "string") return parsePuzzleCardList(rawCards);
  if (!Array.isArray(rawCards)) return [];
  return rawCards
    .map((card) => normalizeCardName(card))
    .filter(Boolean);
}

function canUseLocalStorage() {
  return typeof window !== "undefined" && typeof window.localStorage !== "undefined";
}

export function createEmptyPuzzleZoneTexts(players = []) {
  return players.map(() => ({
    battlefield: "",
    hand: "",
    graveyard: "",
    exile: "",
    library: "",
    command: "",
  }));
}

export function createPuzzlePlayers(count = 2) {
  return Array.from({ length: Math.max(1, Number(count) || 1) }, (_, index) => ({
    id: `puzzle-player-${index + 1}`,
    name: `Player ${index + 1}`,
    life: 20,
  }));
}

function normalizePuzzleDraftPlayers(players) {
  if (!Array.isArray(players) || players.length === 0) return createPuzzlePlayers(2);
  return players.map((player, index) => ({
    id: String(player?.id || `puzzle-player-${index + 1}`),
    name: normalizePlayerName(player?.name, index),
    life: normalizePlayerLife(player?.life),
  }));
}

export function fitPuzzleZoneTextsToPlayers(players = [], existing = []) {
  return players.map((_, index) => {
    const previous = existing[index] || {};
    return Object.fromEntries(
      PUZZLE_ZONE_ORDER.map((zone) => [zone, String(previous[zone] || "")])
    );
  });
}

export function parsePuzzleCardList(text) {
  const cards = [];

  for (const line of String(text || "").split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("//") || trimmed.startsWith("#")) continue;

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

  return cards.filter(Boolean);
}

export function buildPuzzlePayload(players = [], zoneTexts = []) {
  return {
    version: PUZZLE_VERSION,
    players: players.map((player, index) => {
      const playerZones = zoneTexts[index] || {};
      return {
        name: normalizePlayerName(player?.name, index),
        life: normalizePlayerLife(player?.life),
        zones: Object.fromEntries(
          PUZZLE_ZONE_ORDER.map((zone) => [zone, parsePuzzleCardList(playerZones[zone])])
        ),
      };
    }),
  };
}

function zoneCardsToText(cards) {
  return normalizeZoneCards(cards).join("\n");
}

export function buildPuzzleZoneTextsFromPayload(payload) {
  const normalized = normalizePuzzlePayload(payload);
  if (!normalized) return [];

  return normalized.players.map((player) => Object.fromEntries(
    PUZZLE_ZONE_ORDER.map((zone) => [zone, zoneCardsToText(player.zones?.[zone])])
  ));
}

export function buildPuzzlePayloadFromGameState(state) {
  const players = Array.isArray(state?.players) ? state.players : [];
  if (players.length === 0) return null;

  return normalizePuzzlePayload({
    version: PUZZLE_VERSION,
    players: players.map((player, index) => ({
      name: normalizePlayerName(player?.name, index),
      life: normalizePlayerLife(player?.life),
      zones: {
        battlefield: Array.isArray(player?.battlefield)
          ? player.battlefield.map((card) => card?.name)
          : [],
        hand: Array.isArray(player?.hand_cards)
          ? player.hand_cards.map((card) => card?.name)
          : [],
        graveyard: Array.isArray(player?.graveyard_cards)
          ? player.graveyard_cards.map((card) => card?.name)
          : [],
        exile: Array.isArray(player?.exile_cards)
          ? player.exile_cards.map((card) => card?.name)
          : [],
        library: [],
        command: Array.isArray(player?.command_cards)
          ? player.command_cards.map((card) => card?.name)
          : [],
      },
    })),
  });
}

export function buildPuzzleUrlFromGameState(state) {
  const payload = buildPuzzlePayloadFromGameState(state);
  if (!payload) return "";
  return buildPuzzleUrl(payload);
}

export function encodeBase64UrlUtf8(raw) {
  const value = String(raw || "");
  if (!value) return "";
  return encodeBase64Bytes(encodeUtf8Bytes(value))
    .replace(/\+/g, "-")
    .replace(/\//g, "_")
    .replace(/=+$/g, "");
}

export function decodeBase64UrlUtf8(raw) {
  if (!raw) return "";

  try {
    const normalized = String(raw)
      .trim()
      .replace(/-/g, "+")
      .replace(/_/g, "/");
    const padding = normalized.length % 4;
    const padded = padding === 0 ? normalized : `${normalized}${"=".repeat(4 - padding)}`;
    return decodeUtf8Bytes(decodeBase64Bytes(padded));
  } catch {
    return "";
  }
}

function encodeUtf8Bytes(value) {
  if (typeof TextEncoder !== "undefined") {
    return Array.from(new TextEncoder().encode(value));
  }

  const encoded = encodeURIComponent(value);
  const bytes = [];
  for (let i = 0; i < encoded.length; i += 1) {
    if (encoded[i] === "%") {
      bytes.push(parseInt(encoded.slice(i + 1, i + 3), 16));
      i += 2;
    } else {
      bytes.push(encoded.charCodeAt(i));
    }
  }
  return bytes;
}

function decodeUtf8Bytes(bytes) {
  if (typeof TextDecoder !== "undefined" && typeof Uint8Array !== "undefined") {
    return new TextDecoder().decode(Uint8Array.from(bytes));
  }

  return decodeURIComponent(
    bytes.map((byte) => `%${byte.toString(16).padStart(2, "0")}`).join("")
  );
}

function getBase64BrowserFunction(name) {
  if (typeof window !== "undefined" && typeof window[name] === "function") {
    return window[name].bind(window);
  }
  if (typeof globalThis !== "undefined" && typeof globalThis[name] === "function") {
    return globalThis[name].bind(globalThis);
  }
  return null;
}

function encodeBase64Bytes(bytes) {
  const btoaFn = getBase64BrowserFunction("btoa");
  if (btoaFn) {
    let binary = "";
    for (const byte of bytes) {
      binary += String.fromCharCode(byte);
    }
    return btoaFn(binary);
  }

  let output = "";
  for (let i = 0; i < bytes.length; i += 3) {
    const first = bytes[i];
    const hasSecond = i + 1 < bytes.length;
    const hasThird = i + 2 < bytes.length;
    const second = hasSecond ? bytes[i + 1] : 0;
    const third = hasThird ? bytes[i + 2] : 0;
    const triplet = (first << 16) | (second << 8) | third;
    output += BASE64_ALPHABET[(triplet >> 18) & 63];
    output += BASE64_ALPHABET[(triplet >> 12) & 63];
    output += hasSecond ? BASE64_ALPHABET[(triplet >> 6) & 63] : "=";
    output += hasThird ? BASE64_ALPHABET[triplet & 63] : "=";
  }
  return output;
}

function decodeBase64Bytes(value) {
  const atobFn = getBase64BrowserFunction("atob");
  if (atobFn) {
    const binary = atobFn(value);
    return Array.from(binary, (char) => char.charCodeAt(0));
  }

  const bytes = [];
  let buffer = 0;
  let bits = 0;
  for (const char of String(value)) {
    if (char === "=") break;
    const next = BASE64_ALPHABET.indexOf(char);
    if (next < 0) continue;
    buffer = (buffer << 6) | next;
    bits += 6;
    if (bits >= 8) {
      bits -= 8;
      bytes.push((buffer >> bits) & 255);
    }
  }
  return bytes;
}

export function normalizePuzzlePayload(raw) {
  if (!raw || typeof raw !== "object") return null;
  const players = Array.isArray(raw.players) ? raw.players : [];
  if (players.length === 0) return null;

  return {
    version: Number(raw.version) || PUZZLE_VERSION,
    players: players.map((player, index) => {
      const zones = player?.zones || {};
      return {
        name: normalizePlayerName(player?.name, index),
        life: normalizePlayerLife(player?.life),
        zones: Object.fromEntries(
          PUZZLE_ZONE_ORDER.map((zone) => [zone, normalizeZoneCards(zones[zone])])
        ),
      };
    }),
  };
}

export function loadSavedPuzzleDraft() {
  if (!canUseLocalStorage()) return null;

  try {
    const raw = window.localStorage.getItem(PUZZLE_DRAFT_STORAGE_KEY);
    if (!raw) return null;

    const parsed = JSON.parse(raw);
    const players = normalizePuzzleDraftPlayers(parsed?.players);
    const zoneTexts = fitPuzzleZoneTextsToPlayers(players, parsed?.zoneTexts);
    return { players, zoneTexts };
  } catch {
    return null;
  }
}

export function saveSavedPuzzleDraft(players = [], zoneTexts = []) {
  if (!canUseLocalStorage()) return;

  try {
    const normalizedPlayers = normalizePuzzleDraftPlayers(players);
    const normalizedZoneTexts = fitPuzzleZoneTextsToPlayers(normalizedPlayers, zoneTexts);
    window.localStorage.setItem(
      PUZZLE_DRAFT_STORAGE_KEY,
      JSON.stringify({
        players: normalizedPlayers,
        zoneTexts: normalizedZoneTexts,
        updatedAt: Date.now(),
      })
    );
  } catch {
    // Ignore localStorage failures.
  }
}

export function clearSavedPuzzleDraft() {
  if (!canUseLocalStorage()) return;

  try {
    window.localStorage.removeItem(PUZZLE_DRAFT_STORAGE_KEY);
  } catch {
    // Ignore localStorage failures.
  }
}

export function encodePuzzlePayload(payload) {
  return encodeBase64UrlUtf8(JSON.stringify(payload));
}

export function decodePuzzlePayload(raw) {
  const decoded = decodeBase64UrlUtf8(raw);
  if (!decoded) return null;

  try {
    return normalizePuzzlePayload(JSON.parse(decoded));
  } catch {
    return null;
  }
}

export function buildPuzzleUrl(payload) {
  if (typeof window === "undefined") return "";
  const encoded = encodePuzzlePayload(payload);
  if (!encoded) return "";

  const url = new URL(window.location.href);
  url.searchParams.delete("lobby");
  url.searchParams.delete("name");
  url.searchParams.delete("deck");
  url.searchParams.delete("commander");
  url.searchParams.set("puzzle", encoded);
  return url.toString();
}
