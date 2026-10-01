import { paymentSourceOptions } from "./payment-draft.js";
import { samePlayerId } from "./player-display.js";

export function manaPaymentActionMap(state) {
  const actions = new Map();
  if (state?.decision?.kind !== "mana_payment"
    || !samePlayerId(state.decision.player, state.perspective)) return actions;
  const payment = state.mana_payment;
  const options = paymentSourceOptions(payment);
  const planned = options.map(option => ({ ...option, object_id: Number(option.source_id), kind: "plan_payment_source",
    label: `${option.label || option.payment_kind}${Object.entries(option.expected_mana || {}).filter(([, count]) => count > 0).map(([color, count]) => ` · ${count} ${color}`).join("")}` }));
  const manual = (payment?.mana_abilities || [])
    .filter(ability => !options.some(option => String(option.source_id) === String(ability.source_id) && option.payment_kind === "mana_ability" && option.ability_index === ability.ability_index))
    .map(ability => ({ ...ability, object_id: Number(ability.source_id), kind: "activate_mana_ability", label: `Activate now: ${ability.label}` }));
  for (const action of [...planned, ...manual]) {
    const id = Number(action.source_id);
    if (!Number.isFinite(id)) continue;
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
