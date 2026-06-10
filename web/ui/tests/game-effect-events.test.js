import test from "node:test";
import assert from "node:assert/strict";

import {
  buildPlayerVitals,
  collectGameEffectEvents,
  viewedCardsSignature,
} from "../src/lib/game-effect-events.js";

function player(id, overrides = {}) {
  return {
    id,
    life: 20,
    hand_size: 7,
    library_size: 53,
    graveyard_size: 0,
    ...overrides,
  };
}

function collect({ state, previousVitals, processedTransitionIds = new Set(), previousViewedSignature = "" }) {
  return collectGameEffectEvents({
    state,
    previousVitals,
    vitals: buildPlayerVitals(state.players || []),
    processedTransitionIds,
    previousViewedSignature,
    frameToken: "t1",
  });
}

test("detects an opponent draw burst from hand/library deltas", () => {
  const previousVitals = buildPlayerVitals([player(0), player(1)]);
  const state = {
    players: [player(0), player(1, { hand_size: 9, library_size: 51 })],
  };
  const { events } = collect({ state, previousVitals });
  assert.deepEqual(events, [
    { type: "draw-burst", id: "draw:t1:1", playerKey: "1", count: 2 },
  ]);
});

test("visible library->hand transitions replace the diff-based draw burst", () => {
  const previousVitals = buildPlayerVitals([player(0)]);
  const state = {
    players: [player(0, { hand_size: 8, library_size: 52 })],
    zone_transitions: [{
      id: 7,
      owner: 0,
      controller: 0,
      from_zone: "library",
      to_zone: "hand",
      card: { name: "Opt" },
    }],
  };
  const { events } = collect({ state, previousVitals });
  assert.equal(events.length, 1);
  assert.equal(events[0].type, "zone-flight");
  assert.equal(events[0].kind, "draw");
  assert.equal(events[0].cardName, "Opt");
  assert.equal(events[0].revealsFace, true);
});

test("mill and discard transitions become flights and are processed once", () => {
  const processedTransitionIds = new Set();
  const state = {
    players: [player(0)],
    zone_transitions: [
      { id: 1, owner: 0, from_zone: "library", to_zone: "graveyard", card: { name: "Llanowar Elves" } },
      { id: 2, owner: 0, from_zone: "hand", to_zone: "graveyard", card: { name: "Opt" } },
    ],
  };
  const first = collect({ state, previousVitals: buildPlayerVitals(state.players), processedTransitionIds });
  assert.deepEqual(first.events.map((event) => event.kind), ["mill", "discard"]);

  const second = collect({ state, previousVitals: buildPlayerVitals(state.players), processedTransitionIds });
  assert.equal(second.events.length, 0);
});

test("library growth reads as a shuffle, single put-backs do not", () => {
  const previousVitals = buildPlayerVitals([player(0), player(1)]);
  const state = {
    players: [
      player(0, { library_size: 56, hand_size: 4 }),
      player(1, { library_size: 54, hand_size: 6 }),
    ],
  };
  const { events } = collect({ state, previousVitals });
  assert.deepEqual(events.map((event) => `${event.type}:${event.playerKey}`), ["shuffle:0"]);
});

test("life deltas pulse, board resets stay silent", () => {
  const previousVitals = buildPlayerVitals([player(0), player(1)]);
  const damaged = {
    players: [player(0, { life: 17 }), player(1, { life: 24 })],
  };
  const { events } = collect({ state: damaged, previousVitals });
  assert.deepEqual(
    events.map((event) => `${event.type}:${event.playerKey}:${event.delta}`),
    ["life:0:-3", "life:1:4"]
  );

  const reset = {
    players: [player(0, { library_size: 0 }), player(1)],
  };
  assert.equal(collect({ state: reset, previousVitals }).events.length, 0);
});

test("a new library view emits one peek per signature", () => {
  const viewedCards = {
    subject: 0,
    viewer: 0,
    zone: "library",
    visibility: "private",
    description: "Scry 2",
    cards: [{ id: 1 }, { id: 2 }],
    card_ids: [1, 2],
  };
  const state = { players: [player(0)], viewed_cards: viewedCards };
  const previousVitals = buildPlayerVitals(state.players);

  const first = collect({ state, previousVitals });
  assert.equal(first.events.length, 1);
  assert.equal(first.events[0].type, "library-peek");
  assert.equal(first.events[0].count, 2);
  assert.equal(first.events[0].label, "Scry 2");
  assert.equal(first.viewedSignature, viewedCardsSignature(viewedCards));

  const repeat = collect({
    state,
    previousVitals,
    previousViewedSignature: first.viewedSignature,
  });
  assert.equal(repeat.events.length, 0);
});
