import assert from "node:assert/strict";
import test from "node:test";

import {
  BASIC_LAND_BY_COLOR,
  RANDOM_GAME_ZONES,
  basicLandCycle,
  cardMatchesFilters,
  classifyCard,
  createSeededRng,
  generateRandomGamePayload,
  randomGameDefaults,
  zoneAcceptsCard,
} from "../src/lib/random-game.js";

const asset = ({ name, types, supertypes = [], pips = [], score = 1, token = false, linked = "None" }) => ({
  canonicalName: name,
  group: { name, score },
  artifacts: [{
    payload: {
      definition: {
        card: {
          card_types: types,
          supertypes,
          subtypes: [],
          mana_cost: pips.length ? { pips } : null,
          is_token: token,
          linked_face_layout: linked,
        },
      },
    },
  }],
});

const bears = classifyCard(asset({ name: "Grizzly Bears", types: ["Creature"], pips: [[{ Generic: 1 }], ["Green"]] }));
const bolt = classifyCard(asset({ name: "Lightning Bolt", types: ["Instant"], pips: [["Red"]] }));
const ritual = classifyCard(asset({ name: "Dark Ritual", types: ["Sorcery"], pips: [["Black"]] }));
const solRing = classifyCard(asset({ name: "Sol Ring", types: ["Artifact"], pips: [[{ Generic: 1 }]] }));
const jace = classifyCard(asset({
  name: "Jace, the Mind Sculptor",
  types: ["Planeswalker"],
  supertypes: ["Legendary"],
  pips: [[{ Generic: 2 }], ["Blue"], ["Blue"]],
}));
const saga = classifyCard(asset({ name: "Urza's Saga", types: ["Enchantment", "Land"] }));

test("a card is described by the engine's own types, cost and supertypes", () => {
  assert.deepEqual(bears.types, ["Creature"]);
  assert.deepEqual(bears.colors, ["Green"]);
  assert.equal(bears.manaValue, 2);
  assert.equal(bears.permanent, true);
  assert.equal(bears.legendary, false);
  assert.equal(jace.legendary, true);
  assert.equal(jace.manaValue, 4);
  assert.equal(bolt.permanent, false, "an instant is never a permanent");
  assert.equal(saga.permanent, true, "an enchantment land is");
  assert.deepEqual(saga.colors, [], "and costs no coloured mana");
  // A hybrid pip counts once and offers either colour.
  const hybrid = classifyCard(asset({ name: "Boros Charm", types: ["Instant"], pips: [["Red", "White"]] }));
  assert.deepEqual(hybrid.colors, ["Red", "White"]);
  assert.equal(hybrid.manaValue, 1);
});

test("cards with no place in a normal game are not described at all", () => {
  assert.equal(classifyCard(asset({ name: "Goblin", types: ["Creature"], token: true })), null, "token");
  assert.equal(classifyCard(asset({ name: "Academy at Tolaria West", types: ["Plane"] })), null, "plane");
  assert.equal(classifyCard(asset({ name: "Nothing", types: [] })), null, "typeless");
  assert.equal(classifyCard({ canonicalName: "Broken", artifacts: [] }), null, "uncompiled");
  assert.equal(classifyCard(null), null);
});

test("only permanents may be generated onto the battlefield", () => {
  assert.equal(zoneAcceptsCard("battlefield", bears), true);
  assert.equal(zoneAcceptsCard("battlefield", saga), true);
  assert.equal(zoneAcceptsCard("battlefield", bolt), false);
  assert.equal(zoneAcceptsCard("battlefield", ritual), false);
  // Every other zone holds cards of any type.
  for (const zone of ["hand", "library", "graveyard", "exile"]) {
    assert.equal(zoneAcceptsCard(zone, bolt), true, zone);
    assert.equal(zoneAcceptsCard(zone, bears), true, zone);
  }
  // A commander has to be something that can command.
  assert.equal(zoneAcceptsCard("command", jace), true);
  assert.equal(zoneAcceptsCard("command", bears), false, "not legendary");
  assert.equal(zoneAcceptsCard("command", solRing), false, "not a creature or planeswalker");
  assert.equal(zoneAcceptsCard("ante", bears), false, "the engine has no ante zone");
});

test("the filters keep only the cards that were asked for", () => {
  const config = { ...randomGameDefaults(), types: { Creature: true }, colors: { Green: true } };
  assert.equal(cardMatchesFilters(bears, config), true);
  assert.equal(cardMatchesFilters(bolt, config), false, "wrong type");
  assert.equal(cardMatchesFilters({ ...bears, colors: ["Blue"] }, config), false, "wrong colour");
  // A colourless card needs the colourless box, not a colour.
  assert.equal(cardMatchesFilters(solRing, { ...config, types: { Artifact: true } }), false);
  assert.equal(cardMatchesFilters(solRing, { ...config, types: { Artifact: true }, colors: { Colorless: true } }), true);
  assert.equal(cardMatchesFilters(bears, { ...config, manaValue: { min: 3, max: 9 } }), false, "too cheap");
  assert.equal(cardMatchesFilters(bears, { ...config, manaValue: { min: 0, max: 1 } }), false, "too expensive");
  assert.equal(cardMatchesFilters({ ...bears, score: 0.8 }, config), false, "below the fidelity floor");
  assert.equal(cardMatchesFilters({ ...bears, score: 0.8 }, { ...config, minScore: 0.5 }), true);
  assert.equal(cardMatchesFilters({ ...bears, score: null }, { ...config, minScore: 0 }), true, "unscored cards need the floor off");
  assert.equal(cardMatchesFilters({ ...bears, singleFaced: false }, config), false);
  assert.equal(cardMatchesFilters({ ...bears, singleFaced: false }, { ...config, singleFacedOnly: false }), true);
});

test("basics follow the chosen colours so a table can still make mana", () => {
  assert.deepEqual(basicLandCycle({ colors: { Red: true, Blue: true } }), ["Island", "Mountain"]);
  assert.deepEqual(basicLandCycle({ colors: { Colorless: true } }), ["Wastes"]);
  assert.deepEqual(basicLandCycle({ colors: {} }), []);
});

test("a generated table places every card where it may legally sit", () => {
  const cards = [bears, bolt, ritual, solRing, jace, saga];
  const config = {
    ...randomGameDefaults(),
    playerCount: 2,
    zones: {
      battlefield: { count: 4, basics: 2 },
      hand: { count: 3, basics: 0 },
      library: { count: 6, basics: 4 },
      graveyard: { count: 2, basics: 0 },
      exile: { count: 1, basics: 0 },
      command: { count: 1, basics: 0 },
    },
    allowDuplicates: true,
    colors: { White: true, Blue: true, Black: true, Red: true, Green: true, Colorless: true },
  };
  const { payload } = generateRandomGamePayload({ config, cards, rng: createSeededRng("table") });
  assert.equal(payload.version, 1);
  assert.equal(payload.players.length, 2);
  const basics = new Set(Object.values(BASIC_LAND_BY_COLOR));
  const byName = new Map(cards.map((card) => [card.name, card]));
  for (const player of payload.players) {
    assert.equal(player.life, 20);
    assert.deepEqual(Object.keys(player.zones).sort(), [...RANDOM_GAME_ZONES].sort());
    assert.equal(player.zones.ante, undefined, "the payload never mentions ante");
    assert.equal(player.zones.battlefield.length, 4);
    for (const name of player.zones.battlefield) {
      if (basics.has(name)) continue;
      assert.equal(byName.get(name).permanent, true, `${name} on the battlefield`);
    }
    assert.equal(player.zones.command.length, 1);
    assert.equal(player.zones.command[0], "Jace, the Mind Sculptor", "the only legal commander here");
    assert.equal(player.zones.library.filter((name) => basics.has(name)).length, 4, "requested basics");
    assert.equal(player.zones.library.length, 6);
    assert.equal(player.zones.exile.length, 1);
  }
});

test("a shortfall is reported rather than filled with something illegal", () => {
  const config = {
    ...randomGameDefaults(),
    playerCount: 1,
    zones: {
      battlefield: { count: 5, basics: 0 },
      hand: { count: 0, basics: 0 },
      library: { count: 0, basics: 0 },
      graveyard: { count: 0, basics: 0 },
      exile: { count: 0, basics: 0 },
      command: { count: 0, basics: 0 },
    },
  };
  // One permanent, no duplicates allowed: four of the five asked-for cards
  // cannot be drawn, and none of the spells may stand in for them.
  const { payload, shortfalls } = generateRandomGamePayload({
    config,
    cards: [bears, bolt, ritual],
    rng: createSeededRng("short"),
  });
  assert.deepEqual(payload.players[0].zones.battlefield, ["Grizzly Bears"]);
  assert.deepEqual(shortfalls, ["battlefield"]);
});

test("one legend never starts twice on the same battlefield", () => {
  const config = {
    ...randomGameDefaults(),
    playerCount: 1,
    allowDuplicates: true,
    zones: {
      battlefield: { count: 4, basics: 0 },
      hand: { count: 4, basics: 0 },
      library: { count: 0, basics: 0 },
      graveyard: { count: 0, basics: 0 },
      exile: { count: 0, basics: 0 },
      command: { count: 0, basics: 0 },
    },
  };
  const { payload } = generateRandomGamePayload({ config, cards: [jace], rng: createSeededRng("legend") });
  assert.deepEqual(payload.players[0].zones.battlefield, ["Jace, the Mind Sculptor"], "the legend rule would kill the rest");
  // A hand is free to hold as many copies as were asked for.
  assert.equal(payload.players[0].zones.hand.length, 4);
  const allowed = generateRandomGamePayload({
    config: { ...config, allowDuplicateLegends: true },
    cards: [jace],
    rng: createSeededRng("legend"),
  });
  assert.equal(allowed.payload.players[0].zones.battlefield.length, 4);
});

test("duplicates are refused unless they were allowed", () => {
  const config = {
    ...randomGameDefaults(),
    playerCount: 1,
    zones: {
      battlefield: { count: 3, basics: 0 },
      hand: { count: 0, basics: 0 },
      library: { count: 0, basics: 0 },
      graveyard: { count: 0, basics: 0 },
      exile: { count: 0, basics: 0 },
      command: { count: 0, basics: 0 },
    },
  };
  const strict = generateRandomGamePayload({ config, cards: [bears, solRing], rng: createSeededRng("dup") });
  assert.deepEqual([...strict.payload.players[0].zones.battlefield].sort(), ["Grizzly Bears", "Sol Ring"]);
  const loose = generateRandomGamePayload({
    config: { ...config, allowDuplicates: true },
    cards: [bears],
    rng: createSeededRng("dup"),
  });
  assert.deepEqual(loose.payload.players[0].zones.battlefield, ["Grizzly Bears", "Grizzly Bears", "Grizzly Bears"]);
});

test("the same seed rebuilds the same table", () => {
  const config = { ...randomGameDefaults(), playerCount: 2 };
  const cards = [bears, bolt, ritual, solRing, jace, saga];
  const first = generateRandomGamePayload({ config, cards, rng: createSeededRng("seed-a") });
  const again = generateRandomGamePayload({ config, cards, rng: createSeededRng("seed-a") });
  const other = generateRandomGamePayload({ config, cards, rng: createSeededRng("seed-b") });
  assert.deepEqual(first.payload, again.payload);
  assert.notDeepEqual(first.payload, other.payload);
});

test("a table with nothing to draw from reports it instead of throwing", () => {
  const { payload, eligibleCount } = generateRandomGamePayload({
    config: { ...randomGameDefaults(), playerCount: 1 },
    cards: [],
    rng: createSeededRng("empty"),
  });
  assert.equal(eligibleCount, 0);
  assert.equal(payload.players.length, 1);
  // Basics need no catalogue, so the zones that asked for them still get them.
  assert.equal(payload.players[0].zones.battlefield.length, 3);
  assert.equal(payload.players[0].zones.library.length, 12);
});

const omniscience = classifyCard(asset({
  name: "Omniscience",
  types: ["Enchantment"],
  pips: [[{ Generic: 10 }]],
}));

const soloConfig = (overrides = {}) => ({
  ...randomGameDefaults(),
  playerCount: 2,
  zones: {
    battlefield: { count: 4, basics: 1 },
    hand: { count: 2, basics: 0 },
    library: { count: 0, basics: 0 },
    graveyard: { count: 0, basics: 0 },
    exile: { count: 0, basics: 0 },
    command: { count: 0, basics: 0 },
  },
  ...overrides,
});

test("the promised cards always start on the local player's battlefield", () => {
  const config = soloConfig();
  const { payload, unavailableGuaranteed } = generateRandomGamePayload({
    config,
    cards: [bears, solRing, bolt],
    guaranteedCards: [omniscience],
    rng: createSeededRng("promised"),
  });
  assert.deepEqual(unavailableGuaranteed, []);
  const [me, opponent] = payload.players;
  assert.ok(me.zones.battlefield.includes("Omniscience"), "on my battlefield");
  assert.equal(me.zones.battlefield.length, 4, "and it took one of the requested slots");
  assert.equal(me.zones.battlefield.filter((name) => name === "Omniscience").length, 1);
  assert.ok(!opponent.zones.battlefield.includes("Omniscience"), "never on theirs");
  // It is placed even though the filters would never have sampled it: the pool
  // here has no enchantments at all.
  assert.ok(![bears, solRing, bolt].some((card) => card.name === "Omniscience"));
  // And it is not drawn a second time into another of my zones.
  assert.ok(!me.zones.hand.includes("Omniscience"));
});

test("a promised card is placed even when the battlefield asked for nothing", () => {
  const config = soloConfig({
    zones: {
      battlefield: { count: 0, basics: 0 },
      hand: { count: 0, basics: 0 },
      library: { count: 0, basics: 0 },
      graveyard: { count: 0, basics: 0 },
      exile: { count: 0, basics: 0 },
      command: { count: 0, basics: 0 },
    },
  });
  const { payload } = generateRandomGamePayload({
    config,
    cards: [bears],
    guaranteedCards: [omniscience],
    rng: createSeededRng("promise-only"),
  });
  assert.deepEqual(payload.players[0].zones.battlefield, ["Omniscience"]);
  assert.deepEqual(payload.players[1].zones.battlefield, []);
});

test("promised basics still leave room for the promised cards", () => {
  const config = soloConfig({ zones: { ...soloConfig().zones, battlefield: { count: 2, basics: 2 } } });
  const { payload } = generateRandomGamePayload({
    config,
    cards: [bears],
    guaranteedCards: [omniscience],
    rng: createSeededRng("basics-room"),
  });
  const battlefield = payload.players[0].zones.battlefield;
  assert.equal(battlefield.length, 2);
  assert.ok(battlefield.includes("Omniscience"));
  assert.equal(battlefield.filter((name) => name === "Plains").length, 1, "one basic gave up its slot");
});

test("a card that cannot start on a battlefield is reported instead of placed", () => {
  const config = soloConfig({ alwaysOnMyBattlefield: ["Lightning Bolt", "Not A Real Card", "Omniscience"] });
  const { payload, unavailableGuaranteed } = generateRandomGamePayload({
    config,
    cards: [bears],
    guaranteedCards: [omniscience, bolt],
    rng: createSeededRng("refused"),
  });
  assert.deepEqual(unavailableGuaranteed, ["Lightning Bolt", "Not A Real Card"]);
  assert.ok(payload.players[0].zones.battlefield.includes("Omniscience"));
  assert.ok(!payload.players[0].zones.battlefield.includes("Lightning Bolt"));
});

test("the promise can be turned off", () => {
  const { payload, unavailableGuaranteed } = generateRandomGamePayload({
    config: soloConfig({ alwaysOnMyBattlefield: [] }),
    cards: [bears, solRing],
    guaranteedCards: [omniscience],
    rng: createSeededRng("no-promise"),
  });
  assert.deepEqual(unavailableGuaranteed, []);
  assert.ok(!payload.players[0].zones.battlefield.includes("Omniscience"));
});

test("Omniscience is what a table promises unless told otherwise", () => {
  assert.deepEqual(randomGameDefaults().alwaysOnMyBattlefield, ["Omniscience"]);
});
