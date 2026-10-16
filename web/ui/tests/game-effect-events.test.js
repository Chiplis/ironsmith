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

test("all zone pairs route once, with death delegated to the existing collapse", async () => {
  const { ANIMATION_ZONES, collectZoneFlightEvents, ZONE_COLORS } = await import('../src/lib/game-effect-events.js');
  let id = 0;
  for (const from of ANIMATION_ZONES) for (const to of ANIMATION_ZONES) {
    const seen = new Set();
    const state = { zone_transitions: [{ id: ++id, owner: 0, controller: 1, stable_id: 20,
      from_zone: from, to_zone: to, card: { name: 'Opt', controller: 1 } }] };
    const flights = collectZoneFlightEvents(state, seen);
    assert.equal(flights.length, from === 'battlefield' && to === 'graveyard' ? 0 : 1, `${from}->${to}`);
    if (flights.length) {
      assert.equal(flights[0].color, ZONE_COLORS[to]);
      assert.equal(flights[0].destinationPlayerKey, ['battlefield', 'stack'].includes(to) ? '1' : '0');
    }
    assert.equal(collectZoneFlightEvents(state, seen).length, 0);
  }
});

test("marquee exile and counter effects suppress duplicate card flights", async () => {
  const { collectZoneFlightEvents } = await import('../src/lib/game-effect-events.js');
  for (const [from, to, kind] of [['battlefield', 'exile', 'exiled'], ['stack', 'graveyard', 'spell_countered']]) {
    const state = { zone_transitions: [{ id: 1, stable_id: 10, owner: 0, from_zone: from, to_zone: to, card: { name: 'Opt' } }],
      battlefield_transitions: [{ kind, stable_id: 10 }], effect_events: [{ id: 2, kind, stable_ids: [10] }] };
    assert.equal(collectZoneFlightEvents(state, new Set()).length, 0);
  }
});

test("face-down cards never request face artwork", async () => {
  const { collectZoneFlightEvents } = await import('../src/lib/game-effect-events.js');
  for (const card of [{ name: 'Secret', face_down: true }, { name: 'Hidden card' }]) {
    const events = collectZoneFlightEvents({ zone_transitions: [{ id: 1, owner: 0, from_zone: 'hand', to_zone: 'exile', card }] }, new Set());
    assert.equal(events[0].cardName, '');
  }
});


test("anonymous draws replace the count fallback without revealing or duplicating cards", () => {
  const previousVitals = buildPlayerVitals([player(0), player(1)]);
  const processedTransitionIds = new Set();
  const state = { players: [player(0), player(1, { hand_size: 8, library_size: 52 })],
    zone_transitions: [{ id: 17, owner: 1, controller: 1, identity_hidden: true,
      from_zone: 'library', to_zone: 'hand' }] };
  const { events } = collect({ state, previousVitals, processedTransitionIds });
  assert.equal(events.length, 1);
  assert.equal(events[0].kind, 'draw');
  assert.equal(events[0].cardName, '');
  assert.equal(events[0].objectId, null);
  assert.equal(events[0].stableId, '');
  assert.equal(events[0].revealsFace, false);
  assert.equal(collect({ state, previousVitals: buildPlayerVitals(state.players), processedTransitionIds }).events.length, 0);
});

test("every anonymous route uses a face-down flight, including vanished deaths", async () => {
  const { ANIMATION_ZONES, collectZoneFlightEvents } = await import('../src/lib/game-effect-events.js');
  let id = 0;
  for (const from of ANIMATION_ZONES) for (const to of ANIMATION_ZONES) {
    const events = collectZoneFlightEvents({ zone_transitions: [{ id: ++id, owner: 1, controller: 1,
      identity_hidden: true, from_zone: from, to_zone: to }] }, new Set());
    assert.equal(events.length, 1, `${from}->${to}`);
    assert.equal(events[0].cardName, '');
    assert.equal(events[0].objectId, null);
    assert.equal(events[0].revealsFace, false);
  }
});

test("redaction wins over stray card data and normalizes outside-game moves", async () => {
  const { collectZoneFlightEvents } = await import('../src/lib/game-effect-events.js');
  const [event] = collectZoneFlightEvents({ zone_transitions: [{ id: 10, owner: 1, controller: 1,
    identity_hidden: true, from_zone: 'outside the game', to_zone: 'hand',
    stable_id: 8, new_object_id: 9, card: { id: 9, stable_id: 8, name: 'Secret' } }] }, new Set());
  assert.equal(event.fromZone, 'outside_game');
  assert.equal(event.cardName, '');
  assert.equal(event.stableId, '');
  assert.equal(event.objectId, null);
});
