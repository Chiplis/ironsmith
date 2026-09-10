import assert from "node:assert/strict";
import test from "node:test";
import { readFile, stat } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { collectRandomGameCards, randomGameCardBudget, resolveNamedCards } from "../src/lib/random-game-catalog.js";
import {
  cardMatchesFilters,
  createSeededRng,
  generateRandomGamePayload,
  randomGameDefaults,
  RANDOM_GAME_ZONES,
  zoneAcceptsCard,
} from "../src/lib/random-game.js";
import { normalizePuzzlePayload, PUZZLE_ZONE_ORDER } from "../src/lib/puzzles.js";

const CARDS_DIR = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../public/cards");
const built = await stat(path.join(CARDS_DIR, "index.json")).then(() => true, () => false);

/** Read the generated card assets straight off disk, as the browser would over HTTP. */
const fileFetch = async (url) => {
  const file = path.join(CARDS_DIR, String(url).split("/cards/")[1]);
  try {
    const body = await readFile(file, "utf8");
    return { ok: true, json: async () => JSON.parse(body) };
  } catch {
    return { ok: false, status: 404, json: async () => ({}) };
  }
};

test("a table generated from the real card assets is one the engine can be handed", { skip: !built, timeout: 120000 }, async () => {
  const config = {
    ...randomGameDefaults(),
    playerCount: 2,
    zones: {
      battlefield: { count: 6, basics: 2 },
      hand: { count: 5, basics: 1 },
      library: { count: 12, basics: 6 },
      graveyard: { count: 3, basics: 0 },
      exile: { count: 1, basics: 0 },
      command: { count: 1, basics: 0 },
    },
  };
  const { cards, indexSize } = await collectRandomGameCards({
    config,
    rng: createSeededRng("real-assets"),
    want: randomGameCardBudget(config),
    accept: (card) => cardMatchesFilters(card, config)
      && RANDOM_GAME_ZONES.some((zone) => zoneAcceptsCard(zone, card)),
    fetchImpl: fileFetch,
  });
  assert.ok(indexSize > 1000, `the manifest should list the whole catalogue, saw ${indexSize}`);
  assert.ok(cards.length > 20, `the pool should fill from real cards, saw ${cards.length}`);
  assert.ok(cards.some((card) => card.permanent), "including permanents");
  assert.ok(cards.some((card) => !card.permanent), "and spells");

  const guaranteedCards = await resolveNamedCards(config.alwaysOnMyBattlefield, { fetchImpl: fileFetch });
  assert.deepEqual(
    guaranteedCards.map((card) => card.name),
    ["Omniscience"],
    "the promised card resolves out of the real catalogue",
  );
  const { payload, shortfalls, unavailableGuaranteed } = generateRandomGamePayload({
    config,
    cards,
    guaranteedCards,
    rng: createSeededRng("real-table"),
  });
  assert.deepEqual(shortfalls, [], "the pool should cover every zone");
  assert.deepEqual(unavailableGuaranteed, []);
  assert.ok(payload.players[0].zones.battlefield.includes("Omniscience"), "on our own battlefield");
  assert.ok(!payload.players[1].zones.battlefield.includes("Omniscience"), "and only ours");

  // The payload has to survive the same normalisation the puzzle loader applies.
  const normalized = normalizePuzzlePayload(payload);
  assert.ok(normalized, "a puzzle payload the loader accepts");
  assert.equal(normalized.players.length, 2);

  const byName = new Map([...cards, ...guaranteedCards].map((card) => [card.name, card]));
  const basics = new Set(["Plains", "Island", "Swamp", "Mountain", "Forest", "Wastes"]);
  for (const player of normalized.players) {
    // Zones the engine has no name for would reject the whole batch of cards.
    // Normalisation fills in every zone the loader knows, ante included, so
    // what matters is that no card was generated into it.
    for (const zone of Object.keys(player.zones)) {
      assert.ok(PUZZLE_ZONE_ORDER.includes(zone), `${zone} is a zone the loader knows`);
    }
    assert.deepEqual(player.zones.ante, [], "the engine has no ante zone to place a card in");
    assert.equal(player.zones.battlefield.length, 6);
    for (const name of player.zones.battlefield) {
      if (basics.has(name)) continue;
      const card = byName.get(name);
      assert.ok(card, `${name} came from the pool`);
      assert.equal(card.permanent, true, `${name} (${card.types.join("/")}) may sit on the battlefield`);
    }
    for (const name of player.zones.command) {
      const card = byName.get(name);
      assert.equal(card.legendary, true, `${name} can command`);
      assert.ok(card.types.some((type) => type === "Creature" || type === "Planeswalker"), name);
    }
    assert.equal(player.zones.library.filter((name) => basics.has(name)).length, 6);
  }
});
