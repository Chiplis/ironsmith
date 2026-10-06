import { finishPuzzlePregame, passToFirstMain } from "./fixtures/native-game-setup.mjs";
import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import initEngine, { WasmGame } from "../../wasm_demo/pkg/engine.js";

test("real WASM inspector keeps dormant rules and complete uncounterable spell text", async () => {
  await initEngine({ module_or_path: await readFile(new URL("../../wasm_demo/pkg/engine_bg.wasm", import.meta.url)) });
  const sources = await Promise.all(["rhox-pummeler", "nimble-mongoose", "forest", "island", "plains", "supreme-verdict"].map(async route =>
    JSON.parse(await readFile(new URL(`../public/cards/${route}.json`, import.meta.url)))));
  const game = new WasmGame();
  try {
    game.registerExternalCardSourcesJson(JSON.stringify(sources));
    game.resetEmpty(["Alice", "Bob"], 20);
    for (const zone of ["hand", "graveyard", "battlefield"]) {
      const id = game.addCardToZone(0, "Rhox Pummeler", zone, true);
      const details = game.objectDetails(id);
      assert.ok(details.compiled_text.some(line => line.includes("shield counter") && line.includes("trample")),
        `Rhox's conditional rule remains in ${zone}: ${JSON.stringify(details.compiled_text)}`);
    }
    const mongoose = game.addCardToZone(0, "Nimble Mongoose", "battlefield", true);
    const before = game.objectDetails(mongoose);
    assert.equal(before.power, 1);
    assert.ok(before.compiled_text.some(line => line.includes("+2/+2")));
    for (let i = 0; i < 7; i++) game.addCardToZone(0, "Forest", "graveyard", true);
    const active = game.objectDetails(mongoose);
    assert.equal(active.power, 3);
    assert.ok(active.compiled_text.some(line => line.includes("+2/+2")));
    game.clearPlayerZoneForSetup(0, "graveyard");
    assert.equal(game.objectDetails(mongoose).power, 1);
    assert.deepEqual(game.objectDetails(mongoose).compiled_text, before.compiled_text);
    const verdict = game.addCardToZone(0, "Supreme Verdict", "hand", true);
    const checkVerdict = (id, zone) => {
      const details = game.objectDetails(id);
      assert.ok(details.compiled_text.some(line => line.includes("can't be countered")));
      assert.ok(details.compiled_text.some(line => line.includes("Destroy all creatures")),
        `spell effect survives its static ability in ${zone}: ${JSON.stringify(details.compiled_text)}`);
      assert.equal(details.oracle_text, details.compiled_text.join("\n"));
    };
    checkVerdict(verdict, "hand");
    for (const name of ["Island", "Plains", "Plains", "Forest"]) game.addCardToZone(0, name, "battlefield", true);
    finishPuzzlePregame(game, { filler: "Forest" });
    passToFirstMain(game);
    const cast = game.uiState().decision.actions.find(action => action.kind === "cast_spell" && action.label.includes("Supreme Verdict"));
    assert.ok(cast, "Supreme Verdict can be cast");
    let state = game.dispatch({ type: "priority_action", action_ref: cast.action_ref });
    assert.equal(state.decision.kind, "mana_payment");
    state = game.dispatch({ type: "mana_payment", response: {
      action: "confirm", plan_id: state.decision.plan_id, request_hash: state.decision.request_hash,
    } });
    const spell = state.stack_objects.find(entry => entry.name === "Supreme Verdict");
    assert.ok(spell, "the paid spell reaches the stack");
    checkVerdict(BigInt(spell.inspect_object_id), "stack");
  } finally { game.free(); }
});
