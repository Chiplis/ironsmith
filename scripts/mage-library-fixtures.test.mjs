import test from "node:test";
import assert from "node:assert/strict";
import { initWasmGame, getLibrary, getInspectionState } from "./wasm-test-harness.mjs";
import { normalizePhase, zoneName } from "./mage-port-runner/names.mjs";
import { initializeLibraryFixtures, planInitialLibraryFixtures } from "./mage-port-runner/library-fixtures.mjs";

test("library fixture planning preserves order and never hoists later gameplay additions", () => {
  const before = { op: "addCard", zone: "LIBRARY", player: 0, name: "Island", count: 2 };
  const later = { ...before, name: "Forest" };
  const records = planInitialLibraryFixtures({}, { operations: [before, { op: "execute" }, later] }, {
    playerIndex: Number, cardName: String, numericValue: Number, zoneName,
  });
  assert.equal(records.length, 1);
  assert.equal(records[0].operation, before);
  assert.equal(records[0].count, 2);
});

test("WASM library fixtures retain top order and run the first upkeep exactly once", async () => {
  const { game } = await initWasmGame({ pkg: "demo" });
  try {
    const island = { op: "addCard", zone: "LIBRARY", player: 0, name: "Lightning Bolt", count: 10 };
    const forest = { op: "addCard", zone: "LIBRARY", player: 0, name: "Forest", count: 1 };
    const mantra = { op: "addCard", zone: "BATTLEFIELD", player: 0, name: "Ajani's Mantra", count: 1 };
    const records = planInitialLibraryFixtures({}, { operations: [island, forest, mantra] }, {
      playerIndex: Number, cardName: String, numericValue: Number, zoneName,
    });
    const mapped = initializeLibraryFixtures(game, ["Alice", "Bob"], records, {
      defaultCard: "Mountain", defaultSize: 71,
    });
    assert.equal(mapped.get(island).length, 10);
    assert.equal(mapped.get(forest).length, 1);
    let checkpoint = getInspectionState(game);
    assert.equal(game.uiState().active_player, 0);
    assert.equal(game.uiState().turn_number, 1);
    const actual = getLibrary(checkpoint, 0, { topFirst: true }).map(card => card.name);
    assert.deepEqual(actual, ["Forest", ...Array(10).fill("Lightning Bolt"), ...Array(71).fill("Mountain")]);
    assert.ok(checkpoint.battlefield.includes(mapped.get(mantra)[0]));
    let reachedMain = false;
    for (let step = 0; step < 40; step++) {
      const state = game.uiState();
      assert.equal(state.turn_number, 1, "fixture must remain on the first turn");
      if (normalizePhase(state.phase, state.step) === "PRECOMBAT_MAIN") {
        reachedMain = true;
        break;
      }
      const decision = state.decision;
      if (decision?.kind === "boolean" || (decision?.kind === "select_options"
          && decision.options.some(option => option.index === 1 && option.description === "Yes"))) {
        game.dispatch({ type: "select_options", option_indices: [1] });
        continue;
      }
      assert.equal(decision?.kind, "priority", `unexpected decision ${JSON.stringify(decision)}`);
      const action = decision.actions.find(action =>
        ["keep_opening_hand", "continue_pregame", "begin_game", "pass_priority"].includes(action.action_ref?.kind));
      assert.ok(action, "expected setup continuation or pass priority");
      game.dispatch({ type: "priority_action", action_ref: action.action_ref });
    }
    assert.ok(reachedMain, "fixture must reach the first main phase");
    checkpoint = getInspectionState(game);
    assert.equal(checkpoint.players[0].life, 21, "the source added after staging must see exactly one upkeep");
    assert.equal(checkpoint.players[0].hand.length, 0, "fixture setup must not draw cards");
    game.drawCard(0);
    checkpoint = getInspectionState(game);
    const drawn = checkpoint.objects.find(card => Number(card.id) === Number(checkpoint.players[0].hand[0]));
    assert.equal(drawn.name, "Forest", "actual draw must respect the authored library top");
  } finally {
    game.free();
  }
});

test("WASM library clears preserve authored add/clear order independently for each player", async () => {
  const { game } = await initWasmGame({ pkg: "demo" });
  try {
    const removed = { op: "addCard", zone: "LIBRARY", player: 0, name: "Island", count: 2 };
    const clear = { op: "clearZone", zone: "library", player: 0 };
    const added = { op: "addCard", zone: "LIBRARY", player: 0, name: "Forest", count: 1 };
    const operations = [removed, clear, added];
    const records = planInitialLibraryFixtures({}, { operations }, {
      playerIndex: Number, cardName: String, numericValue: Number, zoneName,
    });
    const mapped = initializeLibraryFixtures(game, ["Alice", "Bob"], records, {
      defaultCard: "Mountain", defaultSize: 71,
    });
    const checkpoint = getInspectionState(game);
    assert.deepEqual(getLibrary(checkpoint, 0).map(card => card.name), ["Forest"]);
    assert.deepEqual(getLibrary(checkpoint, 1).map(card => card.name), Array(71).fill("Mountain"));
    assert.deepEqual(mapped.get(removed), []);
    assert.ok(!checkpoint.objects.some(card => card.owner === 0 && card.zone === "outside_game"),
      "library clears must not create wish targets outside the game");
    assert.deepEqual(mapped.get(clear), []);
    assert.equal(mapped.get(added).length, 1);
    assert.equal(game.uiState().turn_number, 1);
  } finally {
    game.free();
  }
});
