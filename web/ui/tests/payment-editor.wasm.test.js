import { finishPuzzlePregame, passToFirstMain } from "./fixtures/native-game-setup.mjs";
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
    finishPuzzlePregame(game);
    passToFirstMain(game);
    const cast = game.uiState().decision.actions.find(action => action.kind === "cast_spell" && Number(action.object_id) === spell);
    assert.ok(cast);
    let state = game.dispatch({ type: "priority_action", action_ref: cast.action_ref });
    assert.equal(state.decision.kind, "mana_payment");
    const before = state.mana_payment;
    assert.equal(before instanceof Map, false, "the worker and React consume plain object fields");
    const black = before.activation_options.find(option => option.source_id === String(land) && option.color_restriction?.join() === "black");
    assert.equal(black?.expected_mana.black, 1);
    assert.equal(black?.max_activations, 1, "a tapped mana source supplies only one activation");
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
    const proposed = game.exportPublicAuditCheckpoint();
    assert.equal(proposed.objects.find(object => object.id === land).tapped, false);
    assert.equal(proposed.players[0].life, 20, "planning does not pay an activation cost");
    game.exchangeRuntimeSavepoint(originalPayment);
    assert.deepEqual(game.uiState().mana_payment, before, "the other branch retains its own payment proposal and options");
    state = game.copyRuntimeSavepoint(originalPayment);
    assert.deepEqual(state.mana_payment, edited, "copying the edited branch restores its payment view");
    state = game.dispatch({ type: "mana_payment", response: { action: "confirm", plan_id: edited.plan_id, request_hash: edited.request_hash } });
    assert.equal(state.decision.kind, "priority", "the selected color must not prompt again");
    assert.ok(state.stack_objects.some(object => object.name === "Sol Ring"));
    const paid = game.exportPublicAuditCheckpoint();
    assert.equal(paid.objects.find(object => object.id === land).tapped, true);
    assert.equal(paid.objects.find(object => object.id === mountain).tapped, false);
    assert.equal(paid.players[0].life, 19);
    assert.equal(paid.players[0].manaPool.black, 0);
    game.exchangeRuntimeSavepoint(originalPayment);
    assert.deepEqual(game.uiState().mana_payment, edited, "paying in the copy does not mutate the retained branch");
    assert.equal(game.exportPublicAuditCheckpoint().players[0].life, 20);
    assert.equal(game.releaseRuntimeSavepoint(originalPayment), true);
    state = game.cancelDecision();
    assert.equal(state.decision.kind, "priority", "Cancel returns to the pre-cast decision");
    assert.equal(state.mana_payment, undefined);
    assert.equal(state.stack_objects.some(object => object.name === "Sol Ring"), false);
    const cancelled = game.exportPublicAuditCheckpoint();
    assert.equal(cancelled.players[0].life, 20);
    assert.equal(cancelled.objects.find(object => object.id === land).tapped, false);
  } finally { game.free(); }
});
