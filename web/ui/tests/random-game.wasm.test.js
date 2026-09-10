import test from "node:test";
import assert from "node:assert/strict";
import { readFile, stat } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import init, { WasmGame } from "../../wasm_demo/pkg/ironsmith.js";

import { collectRandomGameCards, loadRandomGameIndex, randomGameCardBudget, resolveNamedCards } from "../src/lib/random-game-catalog.js";
import {
  cardMatchesFilters,
  createSeededRng,
  generateRandomGamePayload,
  randomGameDefaults,
  RANDOM_GAME_ZONES,
  zoneAcceptsCard,
} from "../src/lib/random-game.js";
import { PAPER_BACK_LANES, PAPER_FRONT_LANES } from "../src/lib/battlefield-layout.js";
import { normalizePuzzlePayload, PUZZLE_ZONE_ORDER } from "../src/lib/puzzles.js";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const CARDS_DIR = path.resolve(HERE, "../public/cards");
const PKG_DIR = path.resolve(HERE, "../../wasm_demo/pkg");
const ready = await Promise.all([
  stat(path.join(CARDS_DIR, "index.json")).then(() => true, () => false),
  stat(path.join(PKG_DIR, "engine_bg.wasm")).then(() => true, () => false),
]).then((checks) => checks.every(Boolean));

const fileFetch = async (url) => {
  try {
    const body = await readFile(path.join(CARDS_DIR, String(url).split("/cards/")[1]), "utf8");
    return { ok: true, json: async () => JSON.parse(body) };
  } catch {
    return { ok: false, status: 404, json: async () => ({}) };
  }
};

test("the real engine accepts a generated table and keeps every card where it was put", { skip: !ready, timeout: 240000 }, async () => {
  const config = {
    ...randomGameDefaults(),
    playerCount: 2,
    zones: {
      battlefield: { count: 5, basics: 2 },
      hand: { count: 4, basics: 1 },
      library: { count: 8, basics: 4 },
      graveyard: { count: 2, basics: 0 },
      exile: { count: 1, basics: 0 },
      command: { count: 0, basics: 0 },
    },
  };
  const { cards } = await collectRandomGameCards({
    config,
    rng: createSeededRng("engine-pool"),
    want: randomGameCardBudget(config),
    accept: (card) => cardMatchesFilters(card, config)
      && RANDOM_GAME_ZONES.some((zone) => zoneAcceptsCard(zone, card)),
    fetchImpl: fileFetch,
  });
  const guaranteedCards = await resolveNamedCards(config.alwaysOnMyBattlefield, { fetchImpl: fileFetch });
  const { payload, unavailableGuaranteed } = generateRandomGamePayload({
    config,
    cards,
    guaranteedCards,
    rng: createSeededRng("engine-table"),
  });
  assert.deepEqual(unavailableGuaranteed, [], "the promised card was placed");
  const normalized = normalizePuzzlePayload(payload);

  // The browser registers a card's compiled asset before naming it to the
  // engine; the worker does that automatically, so do it by hand here.
  const routeByName = new Map((await loadRandomGameIndex({ fetchImpl: fileFetch }))
    .map((card) => [String(card.name), card.route]));
  const placements = [];
  for (const [playerIndex, player] of normalized.players.entries()) {
    for (const zone of PUZZLE_ZONE_ORDER) {
      for (const cardName of player.zones?.[zone] || []) {
        placements.push({ playerIndex, cardName, zoneName: zone, skipTriggers: true });
      }
    }
  }
  assert.ok(placements.length > 30, `a full table was generated, saw ${placements.length}`);

  const modules = await Promise.all(["engine", "compiler", "verifier"].map(async (name) => [
    name, await readFile(path.join(PKG_DIR, `${name}_bg.wasm`)),
  ]));
  await init(Object.fromEntries(modules));
  const game = new WasmGame();
  try {
    game.resetEmpty(["Alice", "Bob"], config.startingLife);
    for (const name of new Set(placements.map((entry) => entry.cardName))) {
      const route = routeByName.get(name);
      assert.ok(route, `${name} is listed in the card manifest`);
      const source = JSON.parse(await readFile(path.join(CARDS_DIR, `${route}.json`), "utf8"));
      const summary = JSON.parse(game.registerExternalCardSourcesJson(JSON.stringify(source)));
      assert.deepEqual(summary.failed, [], `${name} registers cleanly`);
    }
    for (const [playerIndex, player] of normalized.players.entries()) {
      game.setLife(playerIndex, player.life);
    }
    // One bad placement rejects the whole batch, so this call is the assertion.
    const ids = game.addCardsToZones(placements);
    assert.equal(Array.from(ids).length, placements.length, "every card was placed");
    game.finishPuzzleSetup();

    const state = game.uiState();
    assert.equal(state.players.length, 2);
    // The engine sorts each battlefield card into a lane from its own card
    // types, and a card that is not a permanent lands in "other" — so the lane
    // is its verdict on whether the placement was legal.
    const permanentLanes = new Set([...PAPER_FRONT_LANES, ...PAPER_BACK_LANES].filter((lane) => lane !== "other"));
    for (const [index, player] of state.players.entries()) {
      const expected = normalized.players[index];
      assert.equal(player.life, expected.life);
      assert.equal(player.battlefield.length, expected.zones.battlefield.length, "battlefield size");
      assert.equal(player.hand_size ?? player.hand_cards?.length, expected.zones.hand.length, "hand size");
      for (const card of player.battlefield) {
        assert.ok(
          permanentLanes.has(String(card.lane)),
          `${card.name} took the ${card.lane} lane, so the engine reads it as a permanent`,
        );
      }
      const names = player.battlefield.map((card) => card.name);
      assert.equal(
        names.includes("Omniscience"),
        index === 0,
        "the engine put the promised card on our battlefield and nowhere else",
      );
    }
  } finally {
    game.free();
  }
});
