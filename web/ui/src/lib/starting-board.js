import { buildCatalogRandomGame } from "./catalog-random-game.js";
import { PUBLIC_FORMATS, usesCommanders } from "./relay/formats.js";
import { baseAssetUrl } from "./asset-base.js";
import { CARD_ASSETS_REUSABLE, CARD_ASSET_FETCH_OPTIONS, versionedCardAssetUrl } from "./card-asset-cache.js";
import { cardRouteKey } from "./scryfall.js";
import { readEmbeddedCardCatalogIndex } from "./embedded-card-catalog.js";

const FIXED_BOARD_STORAGE_KEY = "ironsmith.fixedStartingBoard";

const LANDS = ["Forest", "Plains", "Island", "Mountain", "Swamp", "Tropical Island", "Volcanic Island"];
const CREATURES = [
  "Yawgmoth, Thran Physician", "Ornithopter", "Myr Moonvessel",
];
const FIXED_BATTLEFIELD = ["Omniscience", ...LANDS, ...CREATURES];

export function readFixedStartingBoard() {
  try {
    return window.localStorage.getItem(FIXED_BOARD_STORAGE_KEY) === "true";
  } catch {
    return false;
  }
}

export function storeFixedStartingBoard(enabled) {
  try {
    window.localStorage.setItem(FIXED_BOARD_STORAGE_KEY, String(enabled));
  } catch {
    // Keep the setting usable even when browser storage is unavailable.
  }
}

// Two seats play a random constructed format from a seeded mid-game board.
// Three or more sit down for Commander (or Pauper EDH), which starts as a real
// match so each player's commander is in the command zone.
export const TWO_PLAYER_BOARD_FORMATS = Object.freeze(["modern", "legacy", "pioneer", "standard", "vintage", "pauper"]);
export const COMMANDER_BOARD_FORMATS = Object.freeze(["commander", "paupercommander"]);

export function startingBoardFormats(seatCount) {
  return seatCount >= 3 ? COMMANDER_BOARD_FORMATS : TWO_PLAYER_BOARD_FORMATS;
}

function shuffledFormats(formats, rng) {
  const result = [...formats];
  for (let i = result.length - 1; i > 0; i--) {
    const j = Math.floor(rng() * (i + 1));
    [result[i], result[j]] = [result[j], result[i]];
  }
  return result;
}

export async function buildRandomStartingBoard(playerNames, startingLife, semanticThreshold, {
  rng = Math.random,
  fetchImpl = globalThis.fetch,
  excludedCards = new Set(),
} = {}) {
  const names = Array.isArray(playerNames) && playerNames.length ? playerNames : ["Alice", "Bob"];
  const seatCount = Math.max(2, names.length);
  const minScore = semanticThreshold / 100;
  let lastError = null;
  // A format can run out of decks that clear the fidelity threshold; try the
  // others before giving up.
  for (const format of shuffledFormats(startingBoardFormats(seatCount), rng)) {
    const commander = usesCommanders(format);
    try {
      const payload = await buildCatalogRandomGame({
        playerNames: names,
        startingLife: commander ? PUBLIC_FORMATS[format].startingLife : startingLife,
        minScore, format, seatCount, asMatch: commander, rng, fetchImpl, excludedCards,
      });
      if (payload.match) payload.match.format = PUBLIC_FORMATS[format].engineFormat;
      return { ...payload, format };
    } catch (err) {
      lastError = err;
    }
  }
  throw lastError || new Error("No random deck meets the selected card threshold");
}

// Catalog support can be ahead of the running engine. Reject cards the engine
// cannot load and choose fresh decks, including when the first board was prefetched.
export async function loadRandomStartingBoard(playerNames, startingLife, semanticThreshold, {
  initialPayload = null,
  apply,
  build = buildRandomStartingBoard,
} = {}) {
  const excludedCards = new Set();
  let payload = initialPayload;
  for (let attempt = 0; attempt < 12; attempt++) {
    payload ||= await build(playerNames, startingLife, semanticThreshold, { excludedCards });
    try {
      await apply(payload);
      return payload;
    } catch (error) {
      const message = String(error?.message || error).toLocaleLowerCase("en-US");
      if (!payload.match || !/unsupported|unknown card|compil|commander/.test(message) || attempt === 11) {
        throw error;
      }
      const commanders = (payload.match.commanders || []).flat();
      const cards = [...commanders, ...(payload.match.decks || []).flat()];
      const namedCards = cards.filter(name => message.includes(name.toLocaleLowerCase("en-US")));
      const rejected = namedCards.length ? namedCards : commanders;
      const previousSize = excludedCards.size;
      for (const name of rejected) excludedCards.add(name.trim().toLocaleLowerCase("en-US"));
      if (excludedCards.size === previousSize) throw error;
      payload = null;
    }
  }
}

/**
 * Start generating the startup board from supported lobby decks while the
 * engine loads. Legacy engines without an embedded catalogue warm HTTP assets.
 */
export function prefetchRandomStartingBoard(playerNames, startingLife, semanticThreshold) {
  const key = startingBoardKey(playerNames, startingLife, semanticThreshold);
  const payload = buildRandomStartingBoard(playerNames, startingLife, semanticThreshold);
  payload.then((built) => warmCardAssets(built)).catch(() => {});
  // A failed prefetch is rebuilt at init, where its error is reported.
  payload.catch(() => {});
  return { key, payload };
}

export function startingBoardKey(playerNames, startingLife, semanticThreshold) {
  return JSON.stringify([playerNames, startingLife, semanticThreshold]);
}

// Low-priority and bounded, so the engine download is never queued behind the
// warm-up, yet wide enough to finish before an engine served from cache is up.
const WARM_CONCURRENCY = 16;

async function warmCardAssets(payload, { fetchImpl = globalThis.fetch } = {}) {
  if (await readEmbeddedCardCatalogIndex() !== undefined) return;
  // Without cache reuse the worker revalidates each asset anyway.
  if (!CARD_ASSETS_REUSABLE || typeof fetchImpl !== "function") return Promise.resolve();
  const names = new Set([
    ...(payload?.players || []).flatMap((player) => Object.values(player?.zones || {}).flat()),
    ...[...(payload?.match?.decks || []), ...(payload?.match?.commanders || [])].flat(),
  ]);
  const urls = [...names].map(cardRouteKey).filter(Boolean)
    .map((route) => versionedCardAssetUrl(new URL(`cards/${route}.json`, baseAssetUrl()).href));
  let cursor = 0;
  return Promise.all(Array.from({ length: WARM_CONCURRENCY }, async () => {
    while (cursor < urls.length) {
      const url = urls[cursor++];
      // Reading the body is what completes the cache entry.
      await fetchImpl(url, { ...CARD_ASSET_FETCH_OPTIONS, priority: "low" })
        .then((response) => response.arrayBuffer())
        .catch(() => null);
    }
  }));
}

export async function addFixedStartingBoardPreset(game, playerCount) {
  const players = Array.from({ length: playerCount }, (_, playerIndex) => ({
    battlefield: playerIndex < 2 ? [...FIXED_BATTLEFIELD] : [],
    graveyard: Array(5).fill("Plains"),
    exile: Array(2).fill("Swamp"),
  }));
  for (const [playerIndex, zones] of players.entries()) {
    for (const [zone, cards] of Object.entries(zones)) {
      for (const cardName of cards) {
        try {
          await game.addCardToZone(playerIndex, cardName, zone, true);
        } catch (err) {
          console.warn(`Skipping startup ${zone} card "${cardName}":`, err);
        }
      }
    }
  }
}
