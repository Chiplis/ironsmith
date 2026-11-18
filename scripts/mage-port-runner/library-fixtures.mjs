import { ensureCardSourcesForNames } from "../wasm-test-harness.mjs";
import { registerMageFixtureSource } from "./custom-fixtures.mjs";
import { registerPlanarFixtureSources } from "./planar-fixtures.mjs";

function requireFixture(condition, message) {
  if (!condition) throw new Error(`unsupported library fixture: ${message}`);
}

// Collect only declarative setup. Later additions still run at their authored
// time; a library operation after execution must never be moved into setup.
export function planInitialLibraryFixtures(fileSpec, testSpec, { playerIndex, cardName, numericValue, zoneName }) {
  const records = [];
  const operations = [...(fileSpec.setupOperations || []), ...(testSpec.setupOperations || []),
    ...(testSpec.operations || [])];
  let canCheckEmptyHand = true;
  for (const operation of operations) {
    if (operation.op === "assertHandCount" && operation.turn === undefined && canCheckEmptyHand) {
      records.push({ kind: "hand-count", operation, player: playerIndex(operation.player) });
      continue;
    }
    // This Java helper creates a custom spell in hand. It is installed by the
    // runner, after staging the catalog cards and before any scripted action.
    if (operation.op === "unsupported" && /^addCustomEffect_TargetDestroy\(player[A-D](?:,\s*\d+)?\)$/.test(operation.source)) {
      canCheckEmptyHand = false;
      continue;
    }
    if (operation.op === "addEmblem") {
      records.push({ kind: "emblem", operation, player: playerIndex(operation.player) });
      continue;
    }
    if (operation.op === "addXmageFixture") {
      records.push({ kind: "xmage", operation, player: playerIndex(operation.player) });
      continue;
    }
    if (!["addCard", "clearZone", "setStrictChooseMode", "skipInitShuffling", "setChoice",
      "setModeChoice", "addTarget", "castSpell", "activateAbility", "attack", "block", "playLand",
      "setStopAt", "setLife"].includes(operation.op) || (operation.custom && !operation.fixtureSource)) break;
    if (operation.op !== "addCard" && operation.op !== "clearZone") continue;
    canCheckEmptyHand = false;
    const zone = zoneName(operation.zone);
    if (operation.op === "clearZone") {
      records.push({ kind: "clear", operation, zone, player: playerIndex(operation.player) });
      continue;
    }
    const count = numericValue(operation.count ?? 1);
    requireFixture(Number.isSafeInteger(count) && count >= 0, "card count must be a nonnegative integer");
    const name = operation.fixtureSource ? operation.name : cardName(operation.name);
    records.push({ kind: "add", operation, zone, player: playerIndex(operation.player), name, count });
  }
  const configuredPuzzle = (testSpec.turnOrder ?? fileSpec.turnOrder) ||
    (testSpec.planarDecks ?? fileSpec.planarDecks) ||
    (testSpec.rangeOfInfluence ?? fileSpec.rangeOfInfluence) !== undefined;
  return configuredPuzzle || records.some(record =>
    record.kind === "emblem" || record.kind === "xmage" || record.operation.tapped ||
    ["library", "command", "outside_game"].includes(record.zone))
    ? records : [];
}

// Stage native objects before pregame, so library additions cannot run upkeep
// before the scenario's permanents have been installed.
export function initializeLibraryFixtures(game, playerNames, records, {
  defaultCard, defaultSize, seed, turnOrder, planarDecks, rangeOfInfluence, startingLife = 20, assertInitialHand,
}) {
  game.resetEmpty(playerNames, startingLife, seed);
  if (turnOrder) {
    requireFixture(typeof game.setPuzzleTurnOrder === "function", "updated WASM puzzle turn order support required");
    game.setPuzzleTurnOrder(turnOrder);
  }
  if (rangeOfInfluence !== undefined) {
    requireFixture(typeof game.setLimitedRangeOfInfluence === "function", "native range of influence required");
    game.setLimitedRangeOfInfluence(playerNames.map(() => rangeOfInfluence));
  }
  const cards = playerNames.flatMap((_, playerIndex) => Array.from({ length: defaultSize }, () => ({
    playerIndex, cardName: defaultCard, zoneName: "library", skipTriggers: true,
  })));
  const libraries = playerNames.map((_, player) =>
    Array.from({ length: defaultSize }, (_, i) => player * defaultSize + i));
  const zones = playerNames.map((_, player) => new Map([["library", libraries[player]]]));
  const indicesByOperation = new Map();
  const customSources = new Map();
  const emblems = [];
  const xmageFixtures = [];
  for (const record of records) {
    requireFixture(record.player >= 0 && record.player < playerNames.length, "unknown owner");
    if (record.kind === "hand-count") {
      assertInitialHand(record.operation);
      indicesByOperation.set(record.operation, []);
      continue;
    }
    if (record.kind === "emblem") {
      const { operation } = record;
      let abilitySource = operation.abilitySource;
      if (operation.oracleText) {
        abilitySource = `XMage fixture abilities for ${operation.name}`;
        registerMageFixtureSource(game, { name: abilitySource, typeLine: "Artifact", oracleText: operation.oracleText });
      }
      emblems.push({ playerIndex: record.player, name: operation.name, abilitySource });
      indicesByOperation.set(operation, []);
      continue;
    }
    if (record.kind === "xmage") {
      xmageFixtures.push(record);
      indicesByOperation.set(record.operation, []);
      continue;
    }
    const playerZones = zones[record.player];
    if (!playerZones.has(record.zone)) playerZones.set(record.zone, []);
    const indicesInZone = playerZones.get(record.zone);
    if (record.kind === "clear") {
      indicesInZone.length = 0;
      indicesByOperation.set(record.operation, []);
    } else {
      if (record.operation.fixtureSource) {
        const previous = customSources.get(record.name);
        requireFixture(!previous || fixtureSourceKey(previous) === fixtureSourceKey(record.operation),
          `multiple initial definitions for ${record.name}`);
        customSources.set(record.name, record.operation);
      }
      const indices = [];
      for (let i = 0; i < record.count; i++) {
        indices.push(cards.length);
        cards.push({ playerIndex: record.player, cardName: record.name, zoneName: record.zone,
          skipTriggers: true, tapped: record.operation.tapped === true });
      }
      indicesInZone.push(...indices);
      indicesByOperation.set(record.operation, indices);
    }
  }
  const kept = new Set(zones.flatMap(playerZones => [...playerZones.values()].flat()));
  const liveCards = cards.filter((_, index) => kept.has(index));
  for (const source of customSources.values()) registerMageFixtureSource(game, source);
  ensureCardSourcesForNames(game, [...liveCards.map(card => card.cardName), ...emblems.map(emblem => emblem.abilitySource)]);
  const stagedIds = Array.from(game.stagePuzzleCardsToZones(liveCards), Number);
  requireFixture(stagedIds.length === liveCards.length, "library staging did not return every card id");
  if (emblems.length > 0) {
    requireFixture(typeof game.stagePuzzleEmblems === "function", "updated WASM emblem staging required");
    game.stagePuzzleEmblems(emblems);
  }
  if (xmageFixtures.length > 0) {
    requireFixture(typeof game.stageXmageFixture === "function", "XMage native fixtures require the xmage-fixtures WASM feature");
    for (const { player, operation } of xmageFixtures) {
      game.stageXmageFixture(player, operation.name, operation.ability);
    }
  }
  const ids = new Map();
  let stagedIndex = 0;
  cards.forEach((_, index) => {
    if (kept.has(index)) ids.set(index, stagedIds[stagedIndex++]);
  });
  const prepared = game.getHiddenCardState();
  for (const [index, card] of liveCards.entries()) {
    if (card.tapped) requireFixture(game.objectDetails(BigInt(stagedIds[index])).tapped,
      `fixture ${card.cardName} must begin tapped (updated WASM puzzle staging required)`);
  }
  for (let player = 0; player < playerNames.length; player++) {
    const expected = libraries[player].map(index => ids.get(index));
    const actual = Array.from(prepared.players[player].library, Number);
    requireFixture(JSON.stringify(actual) === JSON.stringify(expected), "preparation changed library order");
  }
  if (planarDecks) {
    requireFixture(typeof game.stagePuzzlePlanarDecks === "function", "updated WASM planar deck staging required");
    const names = planarDecks.flat();
    ensureCardSourcesForNames(game, names);
    registerPlanarFixtureSources(game, names);
    game.stagePuzzlePlanarDecks(planarDecks.map(deck => deck.map(name => ({ name }))));
  }
  game.finishPuzzleSetup();
  return new Map([...indicesByOperation].map(([operation, indices]) =>
    [operation, indices.filter(index => ids.has(index)).map(index => ids.get(index))]));
}

function fixtureSourceKey({ typeLine, manaCost, power, toughness, oracleText }) {
  return JSON.stringify([typeLine, manaCost, power, toughness, oracleText]);
}
