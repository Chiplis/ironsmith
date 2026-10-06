import { useCallback, useEffect, useRef, useState } from "react";
import { clearSourcePreferences, excludePaymentSource, paymentPreferences, paymentTransactionKey, preferenceKey, removePaymentStep, selectPaymentSource } from "@/lib/payment-draft";
import { manaActivationCommand } from "@/lib/mana-payment-actions";
import { selectPaymentPipSource } from "@/lib/payment-pip-rows";

export default function usePaymentDraft({ payment, dispatch, cancelBackgroundDispatch, enabled = true }) {
  const key = enabled ? paymentTransactionKey(payment) : null;
  const callbacks = useRef({ dispatch, cancelBackgroundDispatch });
  callbacks.current = { dispatch, cancelBackgroundDispatch };
  const session = useRef({ key, version: 0, sent: 0, running: false, draft: paymentPreferences(payment), edited: false });
  if (session.current.key !== key) session.current = { key, version: 0, sent: 0, running: false, draft: paymentPreferences(payment), edited: false };
  const [, render] = useState(0);
  const [error, setError] = useState(null);
  const [confirming, setConfirming] = useState(false);
  const current = session.current;
  if (!current.edited && payment && preferenceKey(current.draft) !== preferenceKey(payment)) current.draft = paymentPreferences(payment);
  const dirty = current.sent < current.version || current.running
    || (current.edited && preferenceKey(payment) !== preferenceKey(current.draft));
  const edit = useCallback(transform => {
    const active = session.current;
    if (!active.key) return;
    const next = paymentPreferences(transform(active.draft));
    if (preferenceKey(next) === preferenceKey(active.draft)) return;
    callbacks.current.cancelBackgroundDispatch?.();
    active.draft = next;
    active.edited = true;
    active.version += 1;
    setError(null);
    render(value => value + 1);
  }, []);
  useEffect(() => { setError(null); setConfirming(false); }, [key]);
  useEffect(() => {
    const active = session.current;
    if (!key || active.running || active.sent === active.version) return undefined;
    const timer = setTimeout(async () => {
      if (session.current !== active) return;
      const version = active.version;
      const preferences = active.draft;
      active.running = true;
      render(value => value + 1);
      try {
        await callbacks.current.dispatch({ type: "mana_payment", response: { action: "replan", ...preferences } }, "Mana payment plan adjusted", { waitForPaymentReady: true, paymentTransactionId: key });
        active.sent = version;
      } catch (failure) {
        if (session.current === active) setError(String(failure?.message || failure));
        active.sent = version;
      } finally {
        active.running = false;
        if (session.current === active) render(value => value + 1);
      }
    }, 80);
    return () => clearTimeout(timer);
  }, [key, current.version, current.sent, current.running]);
  const confirm = useCallback(async () => {
    const active = session.current;
    if (!payment || payment.can_confirm === false || active.running || active.sent < active.version
      || (active.edited && preferenceKey(payment) !== preferenceKey(active.draft))) return;
    callbacks.current.cancelBackgroundDispatch?.();
    setConfirming(true);
    try {
      await callbacks.current.dispatch({ type: "mana_payment", response: { action: "confirm", plan_id: String(payment.plan_id), request_hash: String(payment.request_hash) } }, `Paid mana for ${payment.source_name || "spell"}`, { waitForPaymentReady: true });
    } finally { if (session.current === active) setConfirming(false); }
  }, [payment]);
  const activate = useCallback(async action => {
    const active = session.current;
    if (dirty || confirming || !active.key) return;
    callbacks.current.cancelBackgroundDispatch?.();
    // A real activation changes the pool and consumes sources. Adopt the
    // engine's resulting preferences instead of retaining satisfied pins.
    active.edited = false;
    setConfirming(true);
    try {
      // Stopping background planning publishes a snapshot before React renders
      // it. Bind this continuation to the payment, just like a draft edit,
      // so that render delay cannot silently drop the source activation.
      await callbacks.current.dispatch(manaActivationCommand(action), `Activated ${action.source_name}'s mana ability`, { waitForPaymentReady: true, paymentTransactionId: active.key });
    } finally { if (session.current === active) setConfirming(false); }
  }, [dirty, confirming]);
  return {
    draft: current.draft, dirty, confirming, error, edited: current.edited,
    select: (source, options) => edit(draft => selectPaymentSource(draft, source, options)),
    selectPip: (source, row, rows) => edit(draft => selectPaymentPipSource(draft, source, row, rows)),
    remove: source => edit(draft => removePaymentStep(draft, source, source.occurrence)),
    exclude: sourceId => edit(draft => excludePaymentSource(draft, sourceId)),
    restore: sourceId => edit(draft => ({ ...draft, excluded_source_ids: draft.excluded_source_ids.filter(id => id !== String(sourceId)) })),
    unpin: sourceId => edit(draft => clearSourcePreferences(draft, sourceId)),
    preserve: sourceId => edit(draft => ({ ...draft, preserved_source_ids: draft.preserved_source_ids.includes(String(sourceId)) ? draft.preserved_source_ids.filter(id => id !== String(sourceId)) : [...draft.preserved_source_ids, String(sourceId)] })),
    allocateX: allocation => edit(draft => ({ ...draft, x_allocation: allocation })),
    toggleLife: pipId => edit(draft => ({ ...draft, required_life_pips: draft.required_life_pips.includes(pipId) ? draft.required_life_pips.filter(id => id !== pipId) : [...draft.required_life_pips, pipId] })),
    reset: () => edit(() => paymentPreferences({ excluded_source_ids: payment?.fixed_excluded_source_ids || [] })), confirm, activate,
  };
}
