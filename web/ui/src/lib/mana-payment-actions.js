import { samePlayerId } from "./player-display.js";

export function manaPaymentActionMap(state) {
  const actions = new Map();
  if (state?.decision?.kind !== "mana_payment"
    || !samePlayerId(state.decision.player, state.perspective)) return actions;
  for (const ability of state.mana_payment?.mana_abilities || []) {
    const id = Number(ability.source_id);
    if (!Number.isFinite(id)) continue;
    const action = { ...ability, object_id: id, kind: "activate_mana_ability" };
    if (!actions.has(id)) actions.set(id, []);
    actions.get(id).push(action);
  }
  return actions;
}

export function manaActivationCommand(action) {
  return {
    type: "mana_payment",
    response: {
      action: "activate",
      source_id: String(action.source_id),
      ability_index: action.ability_index,
    },
  };
}
