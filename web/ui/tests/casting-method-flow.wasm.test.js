import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import * as wasm from "../../wasm_demo/pkg/ironsmith.js";
import { finishExplicitCastingMethod } from "../src/lib/casting-method-choice.js";
import { createWasmInteractionGate } from "../src/lib/wasmInteractionGate.js";

test("Unsummon popover routes retain the dropped creature and reach paid or free casting", async () => {
  await wasm.default(Object.fromEntries(await Promise.all(
    ["engine", "compiler", "verifier"].map(async (name) => [
      name, await readFile(new URL(`../../wasm_demo/pkg/${name}_bg.wasm`, import.meta.url)),
    ])
  )));
  const sources = await Promise.all(["unsummon", "island", "omniscience", "ornithopter"].map(
    async (name) => JSON.parse(await readFile(new URL(`../public/cards/${name}.json`, import.meta.url), "utf8"))
  ));
  for (const paid of [true, false]) {
    const game = new wasm.WasmGame();
    try {
      game.registerExternalCardSourcesJson(JSON.stringify(sources));
      game.resetEmpty(["Alice", "Bob"], 20);
      const spell = game.addCardToZone(0, "Unsummon", "hand", true);
      game.addCardToZone(0, "Island", "battlefield", true);
      game.addCardToZone(0, "Omniscience", "battlefield", true);
      const creature = game.addCardToZone(1, "Ornithopter", "battlefield", true);
      game.finishPuzzleSetup();
      let state = game.uiState();
      for (let step = 0; step < 20; step++) {
        const setup = state.decision?.actions?.find((action) =>
          ["keep_opening_hand", "continue_pregame", "begin_game"].includes(action.action_ref?.kind)
        );
        if (!setup) break;
        state = game.dispatch({ type: "priority_action", action_ref: setup.action_ref });
      }
      const action = state.decision.actions.find((entry) =>
        entry.action_ref?.kind === "cast_spell"
        && Number(entry.action_ref.spell_id) === Number(spell)
        && (entry.action_ref.casting_method.kind === "normal") === paid
      );
      assert.ok(action);
      const gate = createWasmInteractionGate();
      state = await gate.run(async () => finishExplicitCastingMethod(
        game.dispatch({ type: "priority_action", action_ref: action.action_ref }),
        action, (command) => game.dispatch(command)
      ));
      assert.equal(state.decision.kind, "targets");
      state = game.dispatch({ type: "select_targets", targets: [{ kind: "object", object: Number(creature) }] });
      if (paid) {
        assert.equal(state.decision.kind, "mana_payment");
      } else {
        assert.equal(state.decision.kind, "priority");
        assert.equal(state.stack_size, 1);
      }
    } finally {
      game.free();
    }
  }
});
