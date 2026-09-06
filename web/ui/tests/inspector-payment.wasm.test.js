import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import initEngine, { WasmGame } from "../../wasm_demo/pkg/engine.js";

test("real WASM exposes activation payment availability from the planner", async () => {
  await initEngine({ module_or_path: await readFile(new URL("../../wasm_demo/pkg/engine_bg.wasm", import.meta.url)) });
  const sources = await Promise.all([
    "yawgmoth-thran-physician", "grizzly-bears", "ornithopter", "swamp", "mind-stone",
  ].map(async route => JSON.parse(await readFile(new URL(`../public/cards/${route}.json`, import.meta.url)))));

  function setup(swamps, name = "Yawgmoth, Thran Physician") {
    const game = new WasmGame();
    game.registerExternalCardSourcesJson(JSON.stringify(sources));
    game.resetEmpty(["Alice", "Bob"], 20);
    const source = game.addCardToZone(0, name, "battlefield", true);
    game.addCardToZone(0, "Grizzly Bears", "battlefield", true);
    game.addCardToZone(0, "Ornithopter", "battlefield", true);
    game.addCardToZone(0, "Grizzly Bears", "hand", true);
    for (let i = 0; i < swamps; i += 1) game.addCardToZone(0, "Swamp", "battlefield", true);
    game.finishPuzzleSetup();
    let state = game.uiState();
    for (let i = 0; i < 20; i += 1) {
      if ((state.decision?.actions || []).some(action => action.kind === "activate_ability" && Number(action.object_id) === Number(source))) break;
      const next = (state.decision?.actions || []).find(action => [
        "keep_opening_hand", "continue_pregame", "begin_game", "pass_priority",
      ].includes(action.kind));
      assert.ok(next, "setup must reach priority");
      state = game.dispatch({ type: "priority_action", action_ref: next.action_ref });
    }
    return { game, state, source };
  }

  for (const swamps of [0, 1, 2]) {
    const { game, state, source } = setup(swamps);
    try {
      assert.ok(state.decision.actions.every(action => !("mana_payment_available" in action)), "ordinary snapshots do not plan payments");
      const actions = game.inspectorActions(source);
      assert.ok(actions.every(action => Number(action.object_id) === Number(source)), "only the inspected source is evaluated");
      const proliferate = actions.find(action => action.label.includes("Proliferate"));
      assert.ok(proliferate, "unaffordable activations remain in the action list");
      assert.equal(proliferate.mana_payment_available, swamps === 2, `${swamps} Swamps`);
      const sacrifice = actions.find(action => action.label.includes("Pay 1 life"));
      assert.equal(sacrifice?.mana_payment_available, true, "non-mana costs do not need mana");
      const firstOnly = game.inspectorActions(source, sacrifice.ability_index);
      assert.equal(firstOnly.length, 1, "an individual preview does not wait for other abilities");
      assert.equal(firstOnly[0].mana_payment_available, true);
      assert.ok(game.uiState().decision.actions.every(action => !("mana_payment_available" in action)), "inspection does not add planning to later snapshots");
    } finally { game.free(); }
  }

  for (const swamps of [0, 1]) {
    const { game, state, source } = setup(swamps, "Mind Stone");
    try {
      assert.ok(state.decision.actions.every(action => !("mana_payment_available" in action)));
      const draw = game.inspectorActions(source).find(action => action.kind === "activate_ability");
      assert.ok(draw);
      assert.equal(draw.mana_payment_available, swamps === 1, "Mind Stone cannot tap for mana and pay its own tap cost");
      if (swamps === 1) {
        let paid = game.dispatch({ type: "priority_action", action_ref: draw.action_ref });
        assert.equal(paid.decision?.kind, "mana_payment");
        paid = game.dispatch({ type: "mana_payment", response: {
          action: "confirm", plan_id: paid.decision.plan_id, request_hash: paid.decision.request_hash,
        } });
        for (let step = 0; step < 8 && paid.decision?.kind !== "priority"; step += 1) {
          assert.equal(paid.decision?.kind, "select_options", JSON.stringify(paid.decision));
          assert.equal(paid.decision.reason, "Next cost");
          const cost = paid.decision.options.find(option => option.legal);
          assert.ok(cost);
          paid = game.dispatch({ type: "select_options", option_indices: [cost.index] });
        }
        assert.equal(paid.decision?.kind, "priority", JSON.stringify(paid.decision));
        assert.ok((paid.stack_objects || []).some(entry => entry.name === "Mind Stone" && entry.ability_kind === "Activated"), "payable activation reaches the stack");
      }
    } finally { game.free(); }
  }
});
