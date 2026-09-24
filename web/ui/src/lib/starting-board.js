import { collectRandomGameCards, resolveNamedCards } from "./random-game-catalog.js";
import { cardMatchesFilters, generateRandomGamePayload, randomGameDefaults } from "./random-game.js";
import { baseAssetUrl } from "./asset-base.js";
import { CARD_ASSETS_REUSABLE, CARD_ASSET_FETCH_OPTIONS, versionedCardAssetUrl } from "./card-asset-cache.js";
import { cardRouteKey } from "./scryfall.js";

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

export async function buildRandomStartingBoard(playerNames, startingLife, semanticThreshold, {
  rng = Math.random,
  fetchImpl = globalThis.fetch,
} = {}) {
  const defaults = randomGameDefaults();
  const config = {
    ...defaults,
    playerCount: playerNames.length,
    startingLife,
    minScore: semanticThreshold / 100,
    types: { ...defaults.types, Battle: true },
    singleFacedOnly: false,
    manaValue: { min: 0, max: Number.MAX_SAFE_INTEGER },
    zones: { ...defaults.zones, exile: { count: 2, basics: 0 } },
  };
  const [{ cards }, guaranteedCards] = await Promise.all([
    collectRandomGameCards({
      config,
      rng,
      accept: (card) => cardMatchesFilters(card, config),
      fetchImpl,
    }),
    resolveNamedCards(config.alwaysOnMyBattlefield, { fetchImpl }),
  ]);
  const { payload, eligibleCount, shortfalls } = generateRandomGamePayload({
    config, cards, guaranteedCards, rng,
  });
  if (!eligibleCount || shortfalls.length > 0) {
    throw new Error("Could not find enough supported cards to generate a starting board");
  }
  payload.players.forEach((player, index) => {
    player.name = playerNames[index];
  });
  return payload;
}

/**
 * Start the startup board before the engine exists: generating it needs only
 * the card pool, and the engine's own card-asset reads then hit the HTTP cache
 * this warms instead of waiting on the network after the WASM is ready.
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

function warmCardAssets(payload, { fetchImpl = globalThis.fetch } = {}) {
  // Without cache reuse the worker revalidates each asset anyway.
  if (!CARD_ASSETS_REUSABLE || typeof fetchImpl !== "function") return Promise.resolve();
  const names = new Set((payload?.players || []).flatMap((player) => Object.values(player?.zones || {}).flat()));
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
