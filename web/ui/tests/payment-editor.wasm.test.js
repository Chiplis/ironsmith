import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import initEngine, { WasmGame } from "../../wasm_demo/pkg/engine.js";

test("real WASM exposes plain payment views and commits an exact-color proposal only on Pay", async () => {
  await initEngine({ module_or_path: await readFile(new URL("../../wasm_demo/pkg/engine_bg.wasm", import.meta.url)) });
  const sources = await Promise.all(["mana-confluence", "mountain", "sol-ring"].map(async route =>
    JSON.parse(await readFile(new URL(`../public/cards/${route}.json`, import.meta.url)))));
  const game = new WasmGame();
  try {
    game.registerExternalCardSourcesJson(JSON.stringify(sources));
    game.resetEmpty(["Alice", "Bob"], 20);
    const land = Number(game.addCardToZone(0, "Mana Confluence", "battlefield", true));
    const mountain = Number(game.addCardToZone(0, "Mountain", "battlefield", true));
    const spell = Number(game.addCardToZone(0, "Sol Ring", "hand", true));
    game.finishPuzzleSetup();
    for (let i = 0; i < 4; i++) {
      const action = game.uiState().decision.actions.find(action => ["keep_opening_hand", "continue_pregame", "begin_game"].includes(action.action_ref?.kind));
      assert.ok(action);
      game.dispatch({ type: "priority_action", action_ref: action.action_ref });
    }
    const checkpoint = game.exportSyncCheckpoint();
    checkpoint.turn = { ...checkpoint.turn, activePlayer: 0, priorityPlayer: 0, turnNumber: 2, phase: "first_main", step: null };
    checkpoint.priorityRuntime.turnRunnerState = "first_main_priority";
    for (const object of checkpoint.objects) object.summoningSick = false;
    game.importSyncCheckpoint(checkpoint, 0);
    const cast = game.uiState().decision.actions.find(action => action.kind === "cast_spell" && Number(action.object_id) === spell);
    assert.ok(cast);
    let state = game.dispatch({ type: "priority_action", action_ref: cast.action_ref });
    assert.equal(state.decision.kind, "mana_payment");
    const before = state.mana_payment;
    assert.equal(before instanceof Map, false, "the worker and React consume plain object fields");
    const black = before.activation_options.find(option => option.source_id === String(land) && option.color_restriction?.join() === "black");
    assert.equal(black?.expected_mana.black, 1);
    const originalPayment = game.createRuntimeSavepoint();
    state = game.dispatch({ type: "mana_payment", response: {
      action: "replan", required_source_ids: [],
      required_activations: [{ source_id: String(land), ability_index: black.ability_index, color_restriction: ["black"] }],
      required_alternatives: [], excluded_source_ids: [String(mountain)], preserved_source_ids: [], prefer_life: false, required_life_pips: [],
    } });
    const edited = state.mana_payment;
    assert.equal(edited.transaction_id, before.transaction_id);
    assert.notEqual(edited.request_hash, before.request_hash);
    assert.equal(edited.can_confirm, true);
    assert.equal(edited.planned_sources.length, 1);
    assert.deepEqual(edited.planned_sources[0].color_restriction, ["black"]);
    const proposed = game.exportSyncCheckpoint();
    assert.equal(proposed.objects.find(object => object.id === land).tapped, false);
    assert.equal(proposed.players[0].life, 20, "planning does not pay an activation cost");
    game.exchangeRuntimeSavepoint(originalPayment);
    assert.deepEqual(game.uiState().mana_payment, before, "the other branch retains its own payment proposal and options");
    state = game.copyRuntimeSavepoint(originalPayment);
    assert.deepEqual(state.mana_payment, edited, "copying the edited branch restores its payment view");
    state = game.dispatch({ type: "mana_payment", response: { action: "confirm", plan_id: edited.plan_id, request_hash: edited.request_hash } });
    assert.equal(state.decision.kind, "priority", "the selected color must not prompt again");
    assert.ok(state.stack_objects.some(object => object.name === "Sol Ring"));
    const paid = game.exportSyncCheckpoint();
    assert.equal(paid.objects.find(object => object.id === land).tapped, true);
    assert.equal(paid.objects.find(object => object.id === mountain).tapped, false);
    assert.equal(paid.players[0].life, 19);
    assert.equal(paid.players[0].manaPool.black, 0);
    game.exchangeRuntimeSavepoint(originalPayment);
    assert.deepEqual(game.uiState().mana_payment, edited, "paying in the copy does not mutate the retained branch");
    assert.equal(game.exportSyncCheckpoint().players[0].life, 20);
    assert.equal(game.releaseRuntimeSavepoint(originalPayment), true);
  } finally { game.free(); }
});
