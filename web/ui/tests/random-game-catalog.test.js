import assert from "node:assert/strict";
import test from "node:test";

import {
  collectRandomGameCards,
  loadRandomCardPool,
  loadRandomGameIndex,
  randomGameCardBudget,
  resolveNamedCards,
} from "../src/lib/random-game-catalog.js";
import { decodeRandomCardPool, RANDOM_CARD_POOL_FORMAT } from "../src/lib/random-card-pool.js";
import { classifyCard, createSeededRng, randomGameDefaults } from "../src/lib/random-game.js";

const cardAsset = (name, types, score = 1) => ({
  canonicalName: name,
  group: { name, score },
  artifacts: [{
    payload: {
      definition: {
        card: { card_types: types, supertypes: [], subtypes: [], mana_cost: null, is_token: false, linked_face_layout: "None" },
      },
    },
  }],
});

/** A transport over an in-memory set of card assets, counting what it served. */
function stubFetch(entries, { missing = new Set(), pool = null } = {}) {
  const requested = [];
  const index = { cards: entries.map(({ name, route, score }) => ({ name, route, score })) };
  const fetchImpl = async (url) => {
    if (String(url).includes("random-card-pool.json")) {
      return pool ? { ok: true, json: async () => pool } : { ok: false, status: 404, json: async () => ({}) };
    }
    const route = String(url).split("/cards/")[1]?.replace(/\.json$/, "");
    requested.push(route);
    if (route === "index") return { ok: true, json: async () => index };
    if (missing.has(route)) return { ok: false, status: 404, json: async () => ({}) };
    const entry = entries.find((candidate) => candidate.route === route);
    return entry
      ? { ok: true, json: async () => entry.asset }
      : { ok: false, status: 404, json: async () => ({}) };
  };
  return { fetchImpl, requested };
}

const entry = (route, name, types, score = 1) => ({
  route,
  name,
  score,
  asset: cardAsset(name, types, score),
});

test("the budget covers every card the zones will ask the catalogue for", () => {
  const config = {
    ...randomGameDefaults(),
    playerCount: 2,
    zones: {
      battlefield: { count: 6, basics: 3 },
      hand: { count: 7, basics: 1 },
      library: { count: 30, basics: 12 },
      graveyard: { count: 3, basics: 0 },
      exile: { count: 0, basics: 0 },
      command: { count: 0, basics: 0 },
    },
  };
  // 3 + 6 + 18 + 3 = 30 non-basic cards per player, doubled, plus headroom.
  assert.equal(randomGameCardBudget(config), Math.ceil(60 * 1.8));
  // Duplicates draw from one shared pool, so one player's worth is enough.
  assert.equal(randomGameCardBudget({ ...config, allowDuplicates: true }), Math.ceil(30 * 1.8));
  // A table made only of basics still needs a floor, never zero.
  assert.equal(randomGameCardBudget({ ...config, zones: { library: { count: 10, basics: 10 } } }), 24);
});

test("the pool is filled from the manifest and stops at the target", async () => {
  const entries = Array.from({ length: 40 }, (_, index) => entry(`card-${index}`, `Card ${index}`, ["Creature"]));
  const { fetchImpl, requested } = stubFetch(entries);
  const progress = [];
  const { cards, indexSize } = await collectRandomGameCards({
    config: randomGameDefaults(),
    rng: createSeededRng("pool"),
    want: 10,
    fetchImpl,
    onProgress: (update) => progress.push(update.collected),
  });
  assert.equal(indexSize, 40);
  assert.equal(cards.length, 10, "no more than asked for");
  assert.equal(new Set(cards.map((card) => card.name)).size, 10, "and no card twice");
  assert.ok(requested.filter((route) => route !== "index").length <= 12, "one batch was enough");
  assert.deepEqual(progress, [10]);
});

test("cards the manifest scores below the floor are never even requested", async () => {
  const entries = [
    ...Array.from({ length: 8 }, (_, i) => entry(`good-${i}`, `Good ${i}`, ["Creature"], 1)),
    ...Array.from({ length: 8 }, (_, i) => entry(`weak-${i}`, `Weak ${i}`, ["Creature"], 0.4)),
  ];
  const { fetchImpl, requested } = stubFetch(entries);
  const { cards } = await collectRandomGameCards({
    config: { ...randomGameDefaults(), minScore: 1 },
    rng: createSeededRng("scores"),
    want: 8,
    fetchImpl,
  });
  assert.equal(cards.length, 8);
  assert.ok(requested.every((route) => !route.startsWith("weak")), "the manifest score spared those requests");
});

test("cards that fail to load or classify are skipped, and the caller can refuse the rest", async () => {
  const entries = [
    entry("bears", "Grizzly Bears", ["Creature"]),
    entry("bolt", "Lightning Bolt", ["Instant"]),
    entry("plane", "Some Plane", ["Plane"]),
    entry("gone", "Missing Card", ["Creature"]),
  ];
  const { fetchImpl } = stubFetch(entries, { missing: new Set(["gone"]) });
  const { cards } = await collectRandomGameCards({
    config: randomGameDefaults(),
    rng: createSeededRng("skips"),
    want: 4,
    fetchImpl,
    accept: (card) => card.permanent,
  });
  assert.deepEqual(cards.map((card) => card.name), ["Grizzly Bears"]);
});

test("a manifest that cannot be read is reported and not remembered", async () => {
  let attempts = 0;
  const broken = async () => {
    attempts += 1;
    return { ok: false, status: 503, json: async () => ({}) };
  };
  await assert.rejects(() => loadRandomGameIndex({ fetchImpl: broken }), /HTTP 503/);
  await assert.rejects(() => loadRandomGameIndex({ fetchImpl: broken }), /HTTP 503/);
  assert.equal(attempts, 2, "a failed manifest is retried rather than cached");
});

test("a repeated read of the manifest is served once per transport", async () => {
  const { fetchImpl, requested } = stubFetch([entry("bears", "Grizzly Bears", ["Creature"])]);
  await loadRandomGameIndex({ fetchImpl });
  await loadRandomGameIndex({ fetchImpl });
  assert.deepEqual(requested, ["index"]);
});

test("classification matches what the catalogue hands back", async () => {
  const { fetchImpl } = stubFetch([entry("saga", "Urza's Saga", ["Enchantment", "Land"])]);
  const { cards } = await collectRandomGameCards({
    config: randomGameDefaults(),
    rng: createSeededRng("one"),
    want: 1,
    fetchImpl,
  });
  assert.deepEqual(cards[0], classifyCard(cardAsset("Urza's Saga", ["Enchantment", "Land"])));
});

/** A pool file the way scripts/build-random-card-pool.mjs writes one. */
function encodePool(assets) {
  const dictionaries = { types: [], supertypes: [], colors: [] };
  const code = (dictionary, value) => {
    let at = dictionaries[dictionary].indexOf(value);
    if (at < 0) at = dictionaries[dictionary].push(value) - 1;
    return at;
  };
  const cards = assets.map(classifyCard).filter(Boolean).map((card) => [
    card.name,
    card.types.map((type) => code("types", type)),
    card.supertypes.map((type) => code("supertypes", type)),
    card.colors.map((color) => code("colors", color)),
    card.manaValue,
    card.singleFaced ? 1 : 0,
    card.score,
  ]);
  return { format: RANDOM_CARD_POOL_FORMAT, ...dictionaries, cards };
}

test("a pool decodes to exactly what classifying each asset gives", () => {
  const assets = [
    cardAsset("Urza's Saga", ["Enchantment", "Land"]),
    cardAsset("Lightning Bolt", ["Instant"], 0.97),
    cardAsset("Some Plane", ["Plane"]),
  ];
  assert.deepEqual(decodeRandomCardPool(encodePool(assets)), assets.map(classifyCard).filter(Boolean));
  assert.equal(decodeRandomCardPool({ format: "something-else", cards: [] }), null);
});

test("with a pool the table is drawn without reading a single card asset", async () => {
  const entries = [
    ...Array.from({ length: 30 }, (_, i) => entry(`good-${i}`, `Good ${i}`, ["Creature"], 1)),
    ...Array.from({ length: 30 }, (_, i) => entry(`weak-${i}`, `Weak ${i}`, ["Creature"], 0.4)),
    entry("bolt", "Lightning Bolt", ["Instant"]),
  ];
  const pool = encodePool(entries.map((candidate) => candidate.asset));
  const { fetchImpl, requested } = stubFetch(entries, { pool });
  const { cards, indexSize } = await collectRandomGameCards({
    config: { ...randomGameDefaults(), minScore: 1 },
    rng: createSeededRng("pool-draw"),
    want: 12,
    fetchImpl,
    accept: (card) => card.permanent,
  });
  assert.deepEqual(requested, [], "no manifest and no card assets");
  assert.equal(indexSize, entries.length);
  assert.equal(cards.length, 12);
  assert.ok(cards.every((card) => card.name.startsWith("Good ")), "score floor and caller filter both apply");
  assert.equal(new Set(cards.map((card) => card.name)).size, 12);
});

test("named cards come from the pool, and the manifest covers the ones it lacks", async () => {
  const entries = [entry("omniscience", "Omniscience", ["Enchantment"]), entry("bears", "Grizzly Bears", ["Creature"])];
  const pool = encodePool([entries[0].asset]);
  const pooled = stubFetch(entries, { pool });
  assert.deepEqual((await resolveNamedCards(["Omniscience"], pooled)).map((card) => card.name), ["Omniscience"]);
  assert.deepEqual(pooled.requested, []);
  const fallback = stubFetch(entries, { pool });
  assert.deepEqual((await resolveNamedCards(["Grizzly Bears"], fallback)).map((card) => card.name), ["Grizzly Bears"]);
  assert.deepEqual(fallback.requested, ["index", "bears"]);
});

test("a pool that failed to load is retried, while an absent one is remembered", async () => {
  let attempts = 0;
  const flaky = async () => {
    attempts += 1;
    throw new Error("offline");
  };
  assert.equal(await loadRandomCardPool({ fetchImpl: flaky }), null);
  assert.equal(await loadRandomCardPool({ fetchImpl: flaky }), null);
  assert.equal(attempts, 2);
  const { fetchImpl } = stubFetch([]);
  let absent = 0;
  const counting = async (url) => { absent += 1; return fetchImpl(url); };
  await loadRandomCardPool({ fetchImpl: counting });
  await loadRandomCardPool({ fetchImpl: counting });
  assert.equal(absent, 1);
});
