import test from "node:test";
import assert from "node:assert/strict";
import { manaPaymentActionMap, manaPaymentFrameActions, manaActivationCommand } from "../src/lib/mana-payment-actions.js";

test("mana frame actions activate inside payment, including manual sources", () => {
  const state = {
    perspective: 0,
    decision: { kind: "mana_payment", player: 0 },
    mana_payment: {
      activation_options: [{ source_id: "10", ability_index: 0, payment_kind: "mana_ability", expected_mana: { red: 1 } }],
      mana_abilities: [{ source_id: "11", ability_index: 1, label: "Choose a color" }],
    },
  };
  const actions = manaPaymentFrameActions(state, new Set(["10", "11"]));
  assert.equal(actions.length, 2);
  assert.ok(actions.every(action => action.kind === "activate_mana_ability"));
  assert.deepEqual(manaActivationCommand(actions[0]), {
    type: "mana_payment", response: { action: "activate", source_id: "10", ability_index: 0 },
  });
  assert.equal(manaPaymentFrameActions({ ...state, perspective: 1 }, new Set(["10", "11"])).length, 0);
});

test("source clicks count mana abilities, not their possible color outputs", () => {
  const state = { perspective: 0, decision: { kind: "mana_payment", player: 0 }, mana_payment: {
    activation_options: [
      { source_id: "10", ability_index: 0, expected_mana: { red: 1 } },
      { source_id: "10", ability_index: 0, expected_mana: { blue: 1 } },
      { source_id: "11", ability_index: 0, expected_mana: { colorless: 1 } },
      { source_id: "11", ability_index: 1, expected_mana: { green: 1 } },
    ],
  } };
  const map = manaPaymentActionMap(state);
  assert.equal(map.get(10).length, 1, "one ability activates immediately and lets the engine ask its color");
  assert.equal(map.get(11).length, 2, "two abilities need a source chooser");
  assert.ok([...map.values()].flat().every(action => action.kind === "activate_mana_ability"));
});
