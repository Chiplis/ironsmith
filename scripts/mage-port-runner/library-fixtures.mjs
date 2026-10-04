import { ensureCardSourcesForNames, getInspectionState } from "../wasm-test-harness.mjs";

function requireFixture(condition, message) {
  if (!condition) throw new Error(`unsupported library fixture: ${message}`);
}

// Collect only declarative setup. Later additions still run at their authored
// time; a library operation after execution must never be moved into setup.
export function planInitialLibraryFixtures(fileSpec, testSpec, { playerIndex, cardName, numericValue }) {
  const records = [];
  const operations = [...(fileSpec.setupOperations || []), ...(testSpec.setupOperations || []),
    ...(testSpec.operations || [])];
  for (const operation of operations) {
    if (operation.op === "execute" || operation.op?.startsWith("assert")
        || /\b(?:execute|assert\w*)\s*\(/.test(operation.source || "")) break;
    if (String(operation.zone).toLowerCase() !== "library") continue;
    if (operation.op === "clearZone") {
      records.push({ kind: "clear", operation, player: playerIndex(operation.player) });
      continue;
    }
    if (operation.op !== "addCard") continue;
    requireFixture(!operation.custom, "custom card registration must run at its authored setup operation");
    const count = numericValue(operation.count ?? 1);
    requireFixture(Number.isSafeInteger(count) && count >= 0, "card count must be a nonnegative integer");
    records.push({ kind: "add", operation, player: playerIndex(operation.player), name: cardName(operation.name), count });
  }
  return records;
}

// Stage native objects before pregame, so library additions cannot run upkeep
// before the scenario's permanents have been installed.
export function initializeLibraryFixtures(game, playerNames, records, { defaultCard, defaultSize, seed }) {
  requireFixture(seed === undefined, "explicit seeds are not supported by resetEmpty");
  game.resetEmpty(playerNames, 20);
  const cards = playerNames.flatMap((_, playerIndex) => Array.from({ length: defaultSize }, () => ({
    playerIndex, cardName: defaultCard, zoneName: "library", skipTriggers: true,
  })));
  const libraries = playerNames.map((_, player) =>
    Array.from({ length: defaultSize }, (_, i) => player * defaultSize + i));
  const indicesByOperation = new Map();
  for (const record of records) {
    requireFixture(record.player >= 0 && record.player < playerNames.length, "unknown owner");
    if (record.kind === "clear") {
      libraries[record.player] = [];
      indicesByOperation.set(record.operation, []);
    } else {
      const indices = [];
      for (let i = 0; i < record.count; i++) {
        indices.push(cards.length);
        cards.push({ playerIndex: record.player, cardName: record.name, zoneName: "library", skipTriggers: true });
      }
      libraries[record.player].push(...indices);
      indicesByOperation.set(record.operation, indices);
    }
  }
  const kept = new Set(libraries.flat());
  cards.forEach((card, index) => { if (!kept.has(index)) card.zoneName = "outside_game"; });
  ensureCardSourcesForNames(game, cards.map(card => card.cardName));
  const ids = Array.from(game.stagePuzzleCardsToZones(cards), Number);
  requireFixture(ids.length === cards.length, "library staging did not return every card id");
  game.finishPuzzleSetup();
  const prepared = getInspectionState(game);
  requireFixture(game.uiState().turn_number === 1, "preparation advanced the initial turn");
  requireFixture(prepared.battlefield.length === 0 && prepared.stack.length === 0,
    "library staging unexpectedly produced a battlefield or stack object");
  requireFixture(prepared.players.every(player => player.hand.length === 0 && player.life === 20),
    "library staging changed a hand or life total");
  for (let player = 0; player < playerNames.length; player++) {
    const expected = libraries[player].map(index => ids[index]);
    const actual = Array.from(prepared.players[player].library, Number);
    requireFixture(JSON.stringify(actual) === JSON.stringify(expected), "preparation changed library order");
  }
  return new Map([...indicesByOperation].map(([operation, indices]) =>
    [operation, indices.map(index => ids[index])]));
}
