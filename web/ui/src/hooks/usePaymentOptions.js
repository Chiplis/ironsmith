import { useEffect, useRef } from 'react';
import { paymentOptionsKey, mergePaymentOptions } from '../lib/payment-options-analysis.js';

// A cancelled worker job resolves null. Retry it rather than caching the
// null forever; a request that stays unavailable reports an error instead.
const NULL_RESULT_RETRIES = 1;

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
      if (!options.activation_options_error) cached.current = { game, key, options };
      stateRef.current = next;
      setState(next);
    };
    // Keep Pay usable and make an options failure visible instead of spinning.
    const fail = () => apply({ activation_options: [], activation_options_error: true });
    const request = () => {
      if (pending.current?.game !== game || pending.current?.key !== key) {
        pending.current = { game, key, nullResults: 0, promise: game.getPaymentActivationOptions(
          state.mana_payment.request_hash, state.mana_payment.plan_id) };
      }
      const current = pending.current;
      current.promise.then(options => {
        if (disposed) return;
        if (options != null) { apply(options); return; }
        if (pending.current !== current) return;
        if (current.nullResults++ >= NULL_RESULT_RETRIES) { fail(); return; }
        current.promise = game.getPaymentActivationOptions(
          state.mana_payment.request_hash, state.mana_payment.plan_id);
        request();
      }).catch(fail);
    };
    if (cached.current?.game === game && cached.current?.key === key) apply(cached.current.options);
    else request();
    return () => { disposed = true; };
  }, [enabled, game, key, state?.mana_payment, stateRef, setState]);
}
