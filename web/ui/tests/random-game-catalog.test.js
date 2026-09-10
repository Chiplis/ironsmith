import assert from "node:assert/strict";
import test from "node:test";

import {
  collectRandomGameCards,
  loadRandomGameIndex,
  randomGameCardBudget,
} from "../src/lib/random-game-catalog.js";
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
function stubFetch(entries, { missing = new Set() } = {}) {
  const requested = [];
  const index = { cards: entries.map(({ name, route, score }) => ({ name, route, score })) };
  const fetchImpl = async (url) => {
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
