import { ensureCardSourcesForNames } from "../wasm-test-harness.mjs";

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
    requireFixture(!operation.custom, "custom card registration has no checkpoint restore contract");
    const count = numericValue(operation.count ?? 1);
    requireFixture(Number.isSafeInteger(count) && count >= 0, "card count must be a nonnegative integer");
    records.push({ kind: "add", operation, player: playerIndex(operation.player), name: cardName(operation.name), count });
  }
  return records;
}

// Use the puzzle import API only to prepare libraries, before any scenario
// permanent or hand card exists. Its add API computes an initial decision; the
// checkpoint import creates a fresh GameState and discards that preparatory
// turn history, while retaining the requested library objects and their order.
// finishPuzzleSetup then establishes the ordinary zero-card pregame procedure.
export function initializeLibraryFixtures(game, playerNames, records, { defaultCard, defaultSize, seed }) {
  requireFixture(seed === undefined, "an explicit random seed cannot be restored by the puzzle checkpoint API");
  game.resetEmpty(playerNames, 20);
  const pristine = game.exportSyncCheckpoint();
  const cards = playerNames.flatMap((_, playerIndex) => Array.from({ length: defaultSize }, () => ({
    playerIndex, cardName: defaultCard, zoneName: "library", skipTriggers: true,
  })));
  for (const record of records) {
    requireFixture(record.player >= 0 && record.player < playerNames.length, "unknown owner");
    if (record.kind === "clear") continue;
    for (let i = 0; i < record.count; i++) cards.push({
      playerIndex: record.player, cardName: record.name, zoneName: "library", skipTriggers: true,
    });
  }
  ensureCardSourcesForNames(game, cards.map(card => card.cardName));
  const ids = Array.from(game.addCardsToZones(cards), Number);
  requireFixture(ids.length === cards.length, "library import did not return every card id");
  const checkpoint = game.exportSyncCheckpoint();
  requireFixture((checkpoint.battlefield || []).length === 0 && (checkpoint.stack || []).length === 0,
    "library staging unexpectedly produced a battlefield or stack object");
  requireFixture(checkpoint.players.every(player => (player.hand || []).length === 0 && player.life === 20),
    "library staging changed a hand or life total");
  const libraries = playerNames.map((_, player) => ids.slice(player * defaultSize, (player + 1) * defaultSize));
  const idsByOperation = new Map();
  let offset = playerNames.length * defaultSize;
  for (const record of records) {
    if (record.kind === "clear") {
      libraries[record.player] = [];
      idsByOperation.set(record.operation, []);
    } else {
      const added = ids.slice(offset, offset + record.count);
      libraries[record.player].push(...added);
      idsByOperation.set(record.operation, added);
      offset += record.count;
    }
  }
  const kept = new Set(libraries.flat());
  for (const object of checkpoint.objects) {
    if (!kept.has(Number(object.id))) object.zone = "outside_game";
  }
  checkpoint.players.forEach((player, index) => { player.library = libraries[index]; });
  checkpoint.turn = structuredClone(pristine.turn);
  checkpoint.priorityRuntime = structuredClone(pristine.priorityRuntime);
  game.importSyncCheckpoint(checkpoint, 0);
  game.finishPuzzleSetup();
  const prepared = game.exportSyncCheckpoint();
  requireFixture(JSON.stringify(prepared.turn) === JSON.stringify(pristine.turn),
    "preparation advanced the initial turn or phase");
  for (let player = 0; player < playerNames.length; player++) {
    const expected = libraries[player];
    const actual = Array.from(prepared.players[player].library, Number);
    requireFixture(JSON.stringify(actual) === JSON.stringify(expected), "preparation changed library order");
  }
  return idsByOperation;
}
