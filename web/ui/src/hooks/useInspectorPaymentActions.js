import { useEffect, useReducer } from "react";
import { cachedInspectorPayment, inspectorPaymentKey, requestInspectorPayment } from "@/lib/inspector-payment-cache";

function isActivation(action) {
  return ["activate_ability", "activate_mana_ability"].includes(action.kind)
    && action.object_id != null && action.ability_index != null;
}

export default function useInspectorPaymentActions(game, state, actions) {
  const [, refresh] = useReducer(value => value + 1, 0);
  const requestKey = [...new Set(actions.filter(isActivation).map(inspectorPaymentKey))].sort().join(",");
  const canQuery = typeof game?.inspectorActions === "function" && state != null && requestKey !== "";

  useEffect(() => {
    if (!canQuery) return undefined;
    let active = true;
    for (const key of requestKey.split(",")) {
      const entry = requestInspectorPayment(game, state, key);
      // Publish each ability separately: a no-mana ability must not wait for
      // an unrelated ability's potentially expensive mana search.
      if (!entry.ready) entry.promise.then(() => { if (active) refresh(); });
    }
    return () => { active = false; };
  }, [canQuery, game, requestKey, state]);

  if (!canQuery) return actions;
  return actions.map(action => {
    if (!isActivation(action)) return action;
    const entry = cachedInspectorPayment(game, state, inspectorPaymentKey(action));
    return {
      ...action,
      payment_pending: !entry?.ready,
      mana_payment_available: entry?.available,
    };
  });
}
