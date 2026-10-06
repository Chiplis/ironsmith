import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import initEngine, { WasmGame } from "../../wasm_demo/pkg/engine.js";
import { replayTrustedActions } from "../src/lib/relay/replay-trusted-match.js";

test("real WASM opens unproven payment, promotes plans, and uses manual floating mana", async () => {
  await initEngine({ module_or_path: await readFile(new URL("../../wasm_demo/pkg/engine_bg.wasm", import.meta.url)) });
  const sources = await Promise.all(["grizzly-bears", "forest"].map(async name =>
    JSON.parse(await readFile(new URL(`../public/cards/${name}.json`, import.meta.url)))));
  for (const count of [1, 2]) {
    const game = new WasmGame();
    try {
      game.registerExternalCardSourcesJson(JSON.stringify(sources));
      game.resetEmpty(["Alice", "Bob"], 20);
      const spell = game.addCardToZone(0, "Grizzly Bears", "hand", true);
      for (let index = 0; index < count; index++) game.addCardToZone(0, "Forest", "battlefield", true);
      game.setDeferredPriorityAnalysis(true);
      game.finishPuzzleSetup();
      let state = game.uiState();
      let cast;
      for (let step = 0; step < 20; step++) {
        cast = state.decision?.actions?.find(action => action.kind === "cast_spell" && Number(action.object_id) === Number(spell));
        if (cast) break;
        const next = state.decision?.actions?.find(action => ["keep_opening_hand", "continue_pregame", "begin_game", "pass_priority"].includes(action.kind));
        assert.ok(next, "setup reaches timing-legal priority");
        state = game.dispatch({ type: "priority_action", action_ref: next.action_ref });
      }
      assert.ok(cast);
      assert.equal(cast.payment_proven, false);
      state = game.dispatch({ type: "priority_action", action_ref: cast.action_ref });
      assert.equal(state.decision?.kind, "mana_payment");
      assert.equal(state.mana_payment.can_confirm, false);
      assert.equal(state.mana_payment.allocations.length, 0);
      if (count === 2) {
        assert.equal(game.beginPaymentAnalysis("funded"), true);
        let command;
        for (let step = 0; step < 1000; step++) {
          command = game.stepPaymentAnalysis("funded", 8);
          if (command !== null) break;
        }
        assert.ok(command && command.type === "mana_payment", "a funded plan improves the initially empty proposal");
        state = game.dispatch(command);
        assert.equal(state.mana_payment.can_confirm, true);
      }
      let activation;
      for (let index = 0; index < count; index++) {
        const ability = state.mana_payment.mana_abilities[0];
        assert.ok(ability, "mana abilities remain available while paying");
        activation = { type: "mana_payment", response: { action: "activate", source_id: ability.source_id, ability_index: ability.ability_index } };
        state = game.dispatch(activation);
        assert.equal(state.decision?.kind, "mana_payment");
        assert.equal(state.mana_payment.pool_before.green, index + 1);
      }
      assert.equal(state.mana_payment.can_confirm, count === 2);
      if (count === 2) {
        state = game.dispatch({ type: "mana_payment", response: { action: "confirm", plan_id: state.decision.plan_id, request_hash: state.decision.request_hash } });
        assert.ok(state.stack_objects.some(card => card.name === "Grizzly Bears"));
        assert.equal(state.players[0].mana_pool.green, 0);
      } else {
        game.cancelDecision();
        state = game.uiState();
        assert.ok(state.players[0].hand_cards.some(card => card.name === "Grizzly Bears"));
        assert.equal(state.players[0].mana_pool.green, 0);
        assert.ok(state.players[0].battlefield.every(card => !card.tapped));
        state = await replayTrustedActions(game, [
          { seq: 1, command: { type: "priority_action", action_ref: cast.action_ref } },
          { seq: 2, command: activation },
          { seq: 3, command: { type: "cancel_decision" } },
        ]);
        assert.ok(state.players[0].hand_cards.some(card => card.name === "Grizzly Bears"));
        assert.equal(state.players[0].mana_pool.green, 0);
        assert.ok(state.players[0].battlefield.every(card => !card.tapped));
      }
    } finally { game.setDeferredPriorityAnalysis(false); game.free(); }
  }
});
