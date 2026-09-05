import test from "node:test";
import assert from "node:assert/strict";
import { castingMethodChoiceForAction, finishExplicitCastingMethod } from "../src/lib/casting-method-choice.js";
import { createWasmInteractionGate } from "../src/lib/wasmInteractionGate.js";

const action = { action_ref: { kind: "cast_spell", spell_id: 42, casting_method: { kind: "normal" } } };
const decision = {
  kind: "select_options", source_id: 42, description: "Choose casting method for Lightning Bolt",
  options: [{ index: 0, description: "Normal: {R}", legal: true }, { index: 1, description: "Free" }],
};
test("the paid-method continuation completes inside the gate before its cooldown", async () => {
  const gate = createWasmInteractionGate({ now: () => 0, debounceMs: 100 });
  const targetState = { decision: { kind: "targets" } };
  const commands = [];
  const result = await gate.run(() => finishExplicitCastingMethod(
    { decision }, action, async (command) => {
      commands.push(command);
      return targetState;
    }
  ));
  assert.equal(result, targetState);
  assert.deepEqual(commands, [{ type: "select_options", option_indices: [0] }]);
  assert.equal(await gate.run(() => assert.fail("second user input should remain blocked")), undefined);
});
test("an explicit paid cast answers the redundant method prompt with the normal route", () => {
  assert.deepEqual(castingMethodChoiceForAction(decision, action), { type: "select_options", option_indices: [0] });
});
test("free casts and unrelated choices are never changed to normal casting", () => {
  assert.equal(castingMethodChoiceForAction(decision, { action_ref: { ...action.action_ref, casting_method: { kind: "alternative", index: 0 } } }), null);
  assert.equal(castingMethodChoiceForAction({ ...decision, source_id: 99 }, action), null);
  assert.equal(castingMethodChoiceForAction({ ...decision, description: "Choose the next cost to pay" }, action), null);
  assert.equal(castingMethodChoiceForAction({ ...decision, kind: "targets" }, action), null);
});
test("split-card normal casting keeps the normal method even when it is labeled with the face name", () => {
  assert.deepEqual(castingMethodChoiceForAction({ ...decision, options: [{ index: 0, description: "Fire: {1}{R}" }, { index: 1, description: "Ice: {1}{U}" }] }, action), { type: "select_options", option_indices: [0] });
});
