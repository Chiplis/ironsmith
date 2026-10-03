import { useEffect, useRef } from 'react';
import { paymentOptionsKey, mergePaymentOptions } from '../lib/payment-options-analysis.js';

export function usePaymentOptions({ game, state, stateRef, setState, enabled = true }) {
  const cached = useRef(null);
  const pending = useRef(null);
  const key = paymentOptionsKey(state);
  useEffect(() => {
    if (!enabled || !key || state?.mana_payment?.activation_options_complete !== false
        || !game?.getPaymentActivationOptions) return;
    let disposed = false;
    const apply = options => {
      if (disposed) return;
      const next = mergePaymentOptions(stateRef.current, key, options);
      if (next === stateRef.current) return;
      cached.current = { game, key, options };
      stateRef.current = next;
      setState(next);
    };
    if (cached.current?.game === game && cached.current?.key === key) apply(cached.current.options);
    else {
      if (pending.current?.game !== game || pending.current?.key !== key) {
        pending.current = { game, key, promise: game.getPaymentActivationOptions(
          state.mana_payment.request_hash, state.mana_payment.plan_id) };
      }
      pending.current.promise.then(apply).catch(() => {
        // Keep Pay usable and make an options failure visible instead of spinning.
        apply({ activation_options: [], mana_abilities: [], activation_options_error: true });
      });
    }
    return () => { disposed = true; };
  }, [enabled, game, key, state?.mana_payment, stateRef, setState]);
}
