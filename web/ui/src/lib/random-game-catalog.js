import { classifyCard, RANDOM_GAME_ZONES } from "./random-game.js";
import { baseAssetUrl } from "./asset-base.js";
import { CARD_ASSET_FETCH_OPTIONS, versionedCardAssetUrl } from "./card-asset-cache.js";
import { decodeRandomCardPool } from "./random-card-pool.js";
import { resolveAssetUrl } from "./card-art-url.js";

const CARD_FETCH_CONCURRENCY = 12;
// A random table only needs a pool a little larger than the cards it places.
const POOL_HEADROOM = 1.8;
const POOL_FLOOR = 24;
// Cards outside the filters still cost a request, so the walk has to end.
const ATTEMPT_MULTIPLIER = 5;

export function cardAssetUrl(path) {
  return versionedCardAssetUrl(resolveAssetUrl(`cards/${String(path || "").replace(/^\/+/, "")}`));
}

// Keyed by the fetch that read it, so the app shares one manifest while a
// caller supplying its own transport gets its own.
const indexPromises = new WeakMap();

/**
 * The frontend card manifest: every compiled card's name, route and semantic
 * score. It carries no card types, so a card's types are only known once its
 * own asset is read — which is why the pool is sampled rather than filtered.
 */
export function loadRandomGameIndex({ fetchImpl = globalThis.fetch } = {}) {
  if (!indexPromises.has(fetchImpl)) {
    const request = fetchImpl(cardAssetUrl("index.json"), CARD_ASSET_FETCH_OPTIONS)
      .then((response) => {
        if (!response.ok) throw new Error(`Card index fetch failed: HTTP ${response.status}`);
        return response.json();
      })
      .then((index) => (Array.isArray(index?.cards) ? index.cards : []).filter((card) => card?.route))
      .catch((error) => {
        indexPromises.delete(fetchImpl);
        throw error;
      });
    indexPromises.set(fetchImpl, request);
  }
  return indexPromises.get(fetchImpl);
}

const poolPromises = new WeakMap();

/**
 * Every compiled card already classified at build time, or null when this
 * deployment has no pool (a dev server, a test transport). The pool makes
 * sampling local; without it the catalogue falls back to reading card assets.
 */
export function loadRandomCardPool({ fetchImpl = globalThis.fetch } = {}) {
  if (!poolPromises.has(fetchImpl)) {
    const url = versionedCardAssetUrl(new URL("random-card-pool.json", baseAssetUrl()).href);
    const request = Promise.resolve()
      .then(() => fetchImpl(url, CARD_ASSET_FETCH_OPTIONS))
      .then((response) => (response?.ok ? response.json() : null))
      .then((pool) => decodeRandomCardPool(pool))
      .catch(() => {
        // A deployment without a pool resolves to null and is remembered; a
        // request that failed outright is tried again next time.
        poolPromises.delete(fetchImpl);
        return null;
      });
    poolPromises.set(fetchImpl, request);
  }
  return poolPromises.get(fetchImpl);
}

/** How many distinct cards the configuration will ask the catalogue for. */
export function randomGameCardBudget(config) {
  const players = Math.max(1, Math.trunc(Number(config?.playerCount)) || 1);
  let distinct = 0;
  for (const zone of RANDOM_GAME_ZONES) {
    const count = Math.max(0, Math.trunc(Number(config?.zones?.[zone]?.count)) || 0);
    const basics = zone === "command" ? 0 : Math.max(0, Math.trunc(Number(config?.zones?.[zone]?.basics)) || 0);
    distinct += Math.max(0, count - Math.min(count, basics));
  }
  // Duplicates are drawn from the same pool, so only a shared pool is needed.
  const needed = config?.allowDuplicates ? distinct : distinct * players;
  return Math.max(POOL_FLOOR, Math.ceil(needed * POOL_HEADROOM));
}

function sampleRoutes(index, config, rng) {
  const minScore = Number(config?.minScore);
  // The manifest already knows each card's score, so cards that could never
  // pass the fidelity filter are skipped before they cost a request.
  const eligible = Number.isFinite(minScore) && minScore > 0
    ? index.filter((card) => Number(card.score) >= minScore)
    : index;
  const pool = eligible.length > 0 ? eligible : index;
  const order = [...pool];
  for (let i = order.length - 1; i > 0; i -= 1) {
    const j = Math.floor(rng() * (i + 1));
    [order[i], order[j]] = [order[j], order[i]];
  }
  return order;
}

async function fetchClassifiedCard(route, fetchImpl) {
  try {
    const response = await fetchImpl(cardAssetUrl(`${route}.json`), CARD_ASSET_FETCH_OPTIONS);
    if (!response.ok) return null;
    return classifyCard(await response.json());
  } catch {
    return null;
  }
}

const nameKey = (name) => String(name || "").trim().toLocaleLowerCase("en-US");

/**
 * Read specific cards by name, for the ones a table is promised rather than
 * sampled. Names the manifest does not list are simply absent from the result,
 * which is how the generator learns it cannot honour them.
 */
export async function resolveNamedCards(names, { fetchImpl = globalThis.fetch } = {}) {
  const wanted = [...new Set((Array.isArray(names) ? names : [])
    .map((name) => String(name || "").trim())
    .filter(Boolean))];
  if (wanted.length === 0) return [];
  const pool = await loadRandomCardPool({ fetchImpl });
  if (pool) {
    const byName = new Map(pool.map((card) => [nameKey(card.name), card]));
    const found = wanted.map((name) => byName.get(nameKey(name)));
    if (found.every(Boolean)) return found;
  }
  const index = await loadRandomGameIndex({ fetchImpl });
  const routeByName = new Map(index.map((card) => [nameKey(card.name), card.route]));
  const routes = wanted.map((name) => routeByName.get(nameKey(name))).filter(Boolean);
  const cards = await Promise.all(routes.map((route) => fetchClassifiedCard(route, fetchImpl)));
  return cards.filter(Boolean);
}

/**
 * Gather a pool of classified cards to generate a table from.
 *
 * Candidates are drawn at random from the manifest and read in small batches
 * until enough of them classify, so a narrow filter costs more requests rather
 * than a wrong table. `accept` lets the caller keep only cards it can use.
 */
export async function collectRandomGameCards({
  config,
  rng = Math.random,
  want = null,
  accept = () => true,
  onProgress = null,
  fetchImpl = globalThis.fetch,
  signal = null,
} = {}) {
  const pool = await loadRandomCardPool({ fetchImpl });
  if (pool) {
    return collectFromPool({ pool, config, rng, want, accept, onProgress });
  }
  const index = await loadRandomGameIndex({ fetchImpl });
  const target = Math.max(1, Math.trunc(Number(want)) || randomGameCardBudget(config));
  const order = sampleRoutes(index, config, rng);
  const budget = Math.min(order.length, target * ATTEMPT_MULTIPLIER);
  const cards = [];
  const seen = new Set();

  for (let cursor = 0; cursor < budget && cards.length < target;) {
    if (signal?.aborted) break;
    const batch = order.slice(cursor, cursor + CARD_FETCH_CONCURRENCY);
    cursor += batch.length;
    if (batch.length === 0) break;
    const classified = await Promise.all(batch.map((card) => fetchClassifiedCard(card.route, fetchImpl)));
    for (const card of classified) {
      // A batch can finish the pool part way through; the rest of it is read
      // but not kept, so `want` is an exact ceiling.
      if (cards.length >= target) break;
      if (!card || seen.has(card.name) || !accept(card)) continue;
      seen.add(card.name);
      cards.push(card);
    }
    onProgress?.({ collected: cards.length, target, inspected: cursor });
  }

  return { cards, inspected: Math.min(budget, order.length), indexSize: index.length };
}

// The same draw as the asset walk, over classifications known up front: a
// uniform shuffle of the score-eligible cards, kept in order until the pool is
// full. Nothing is fetched, so no attempt budget is needed.
function collectFromPool({ pool, config, rng, want, accept, onProgress }) {
  const target = Math.max(1, Math.trunc(Number(want)) || randomGameCardBudget(config));
  const order = sampleRoutes(pool, config, rng);
  const cards = [];
  const seen = new Set();
  let inspected = 0;
  for (const card of order) {
    if (cards.length >= target) break;
    inspected += 1;
    if (seen.has(card.name) || !accept(card)) continue;
    seen.add(card.name);
    cards.push(card);
  }
  onProgress?.({ collected: cards.length, target, inspected });
  return { cards, inspected, indexSize: pool.length };
}
