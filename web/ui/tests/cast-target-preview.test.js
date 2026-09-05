import { castingMethodChoiceForAction } from "../src/lib/casting-method-choice.js";
import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { previewCastTargetDecision } from "../src/lib/cast-target-preview.js";

test("provisional Lightning Bolt highlights only legal targets without changing the live game", async () => {
  const wasm = await import("../../wasm_demo/pkg/ironsmith.js");
  const modules = Object.fromEntries(await Promise.all(["engine", "compiler", "verifier"].map(async name =>
    [name, await readFile(new URL(`../../wasm_demo/pkg/${name}_bg.wasm`, import.meta.url))]
  )));
  await wasm.default(modules);
  const previewModule = await import("../../wasm_demo/pkg/engine.js?target-preview");
  await previewModule.default({ module_or_path: modules.engine });
  const game = new wasm.WasmGame();
  try {
    game.resetEmpty(["Alice", "Bob"], 20);
    const sources = await Promise.all(["lightning-bolt", "mountain", "grizzly-bears", "omniscience"].map(async name =>
      JSON.parse(await readFile(new URL(`../public/cards/${name}.json`, import.meta.url), "utf8"))
    ));
    game.registerExternalCardSources(sources);
    game.addCardToHand(0, "Lightning Bolt");
    const mountain = Number(game.addCardToZone(0, "Mountain", "battlefield", true));
    const creature = Number(game.addCardToZone(1, "Grizzly Bears", "battlefield", true));
    game.finishPuzzleSetup();
    for (let step = 0; step < 8; step++) {
      const action = game.uiState().decision?.actions?.find(action =>
        ["keep_opening_hand", "begin_game", "continue_pregame"].includes(action.action_ref?.kind)
      );
      if (!action) break;
      game.dispatch({ type: "priority_action", action_index: action.index, action_ref: action.action_ref });
    }
    const before = game.exportSyncCheckpoint();
    const actions = game.uiState().decision.actions.filter(action => action.kind === "cast_spell");
    assert.ok(actions.length);
    const decision = previewCastTargetDecision(previewModule.WasmGame, before, 0, actions,
      preview => wasm.compileAndRegisterCardSources(preview, sources));
    const targets = decision.requirements.flatMap(requirement => requirement.legal_targets);
    assert.ok(targets.some(target => target.kind === "object" && Number(target.object) === creature));
    assert.ok(!targets.some(target => target.kind === "object" && Number(target.object) === mountain));
    assert.deepEqual(targets.filter(target => target.kind === "player").map(target => target.player).sort(), [0, 1]);
    assert.deepEqual(game.exportSyncCheckpoint(), before);

    // The dead-zone release path rolls an already-declared cast back to hand.
    const cast = actions[0];
    const targeting = game.dispatch({ type: "priority_action", action_index: cast.index, action_ref: cast.action_ref });
    assert.equal(targeting.decision.kind, "targets");
    const cancelled = game.cancelDecision();
    assert.equal(cancelled.decision.kind, "priority");
    assert.ok(cancelled.decision.actions.some(action => action.kind === "cast_spell"));
    const restored = game.exportSyncCheckpoint();
    // Cast and cancellation advance the UI serial while restoring gameplay state.
    assert.ok(restored.snapshotSerial > before.snapshotSerial);
    assert.deepEqual({ ...restored, snapshotSerial: before.snapshotSerial }, before);

    game.addCardToZone(0, "Omniscience", "battlefield", true);
    const methodActions = game.uiState().decision.actions.filter(action => action.kind === "cast_spell");
    const paid = methodActions.find(action => action.action_ref.casting_method.kind === "normal");
    const free = methodActions.find(action => action.action_ref.casting_method.kind !== "normal");
    assert.ok(paid && free, "both paid and free routes must be available");
    const paidPrompt = game.dispatch({ type: "priority_action", action_index: paid.index, action_ref: paid.action_ref });
    const chosenMethod = castingMethodChoiceForAction(paidPrompt.decision, paid);
    assert.ok(chosenMethod, JSON.stringify(paidPrompt.decision));
    assert.equal(game.dispatch(chosenMethod).decision.kind, "targets");
    game.cancelDecision();
    const freePrompt = game.dispatch({ type: "priority_action", action_index: free.index, action_ref: free.action_ref });
    assert.equal(freePrompt.decision.kind, "targets");
    assert.equal(castingMethodChoiceForAction(freePrompt.decision, free), null);

  } finally {
    game.free();
  }
});
