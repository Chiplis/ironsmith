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
  // Production branches are choices within one ability, not separate abilities.
  // A source click activates now; the engine asks for its color when needed.
  const mana = [...planned.filter(action => action.payment_kind === "mana_ability"), ...manual];
  const seen = new Set();
  const activations = mana.filter(action => {
    const key = `${action.source_id}:${action.ability_index}`;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  }).map(action => ({ ...action, kind: "activate_mana_ability" }));
  const manaSources = new Set(activations.map(action => String(action.source_id)));
  const alternatives = planned.filter(action => action.payment_kind !== "mana_ability" && !manaSources.has(String(action.source_id)));
  for (const action of [...activations, ...alternatives]) {
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

// Card-frame mana buttons activate a source now, rather than selecting a
// speculative source for the eventual plan. Keep both reviewed and manual
// abilities, including those whose production needs a player choice.
export function manaPaymentFrameActions(state, objectIds) {
  return [...manaPaymentActionMap(state).values()].flat()
    .filter(action => objectIds.has(String(action.object_id))
      && action.ability_index != null
      && (action.kind === "activate_mana_ability" || action.payment_kind === "mana_ability"))
    .map(action => ({ ...action, kind: "activate_mana_ability" }));
}
