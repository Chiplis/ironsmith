import useUiText from "@/i18n/useUiText";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { AlertTriangle, ChevronDown, LoaderCircle, RotateCcw, SlidersHorizontal } from "lucide-react";
import usePaymentDraft from "@/hooks/usePaymentDraft";
import useDecisionRollout from "@/hooks/useDecisionRollout";
import { useManaPaymentEditor } from "@/context/ManaPaymentEditorContext.shared";
import ActionPopover from "@/components/overlays/ActionPopover";
import { paymentDraftRows, paymentSourceOptions, sourceChoiceKey } from "@/lib/payment-draft";
import { useGame } from "@/context/GameContext";
import { useHover } from "@/context/HoverContext";
import { Button } from "@/components/ui/button";
import { Popover, PopoverTrigger, PopoverContent } from "@/components/ui/popover";
import { paymentPipRows, paymentOptionsForPip } from "@/lib/payment-pip-rows";
import { ManaSymbol } from "@/lib/mana-symbols";
import { cn } from "@/lib/utils";

const POOL_SYMBOLS = [
  ["white", "W"],
  ["blue", "U"],
  ["black", "B"],
  ["red", "R"],
  ["green", "G"],
  ["colorless", "C"],
];

function poolEntries(pool) {
  return POOL_SYMBOLS
    .map(([key, symbol]) => ({ symbol, amount: Number(pool?.[key] || 0) }))
    .filter((entry) => entry.amount > 0);
}

function PoolSummary({ label, pool }) {
  const ui = useUiText();
  const entries = poolEntries(pool);
  return (
    <div className="mana-plan-pool">
      <span className="mana-plan-label">{ui(label)}</span>
      <span className="flex min-h-6 items-center gap-1">
        {entries.length ? entries.map(({ symbol, amount }) => (
          <span key={symbol} className="inline-flex items-center gap-0.5 text-xs font-semibold">
            <ManaSymbol sym={symbol} size={17} />
            {amount > 1 ? <span>×{amount}</span> : null}
          </span>
        )) : <span className="text-xs opacity-60">{ui("Empty")}</span>}
      </span>
    </div>
  );
}

function warningText(value) {
  const text = String(value || "");
  if (text.startsWith("UsesPreservedSource")) return "This plan uses a source marked Preserve.";
  if (text.startsWith("ProducesExcessMana")) return "This plan leaves mana floating after payment.";
  if (text.startsWith("PaysLife")) return "This plan pays life.";
  return text.replace(/([a-z])([A-Z])/g, "$1 $2");
}

function sourceActionLabel(source) {
  if (source.payment_kind === "convoke") return "Tap for convoke";
  if (source.payment_kind === "improvise") return "Tap for improvise";
  if (source.payment_kind === "waterbend") return "Tap for waterbend";
  if (source.payment_kind === "delve") return "Exile for delve";
  return "";
}

function PaymentCardName({ objectId, onInspect, children, className = "" }) {
  const ui = useUiText();
  const label = <span>{children}</span>;
  if (objectId == null || typeof onInspect !== "function") {
    return <span className={cn("block min-w-0 max-w-full whitespace-normal break-words", className)}>{ui(label)}</span>;
  }
  return (
    <button
      type="button"
      className={cn("decision-card-name-trigger block min-w-0 max-w-full whitespace-normal break-words text-left", className)}
      data-inspector-object-id={String(objectId)}
      aria-label={ui("Inspect {0}", { 0: String(children || "card") })}
      onPointerDown={(event) => {
        event.stopPropagation();
      }}
      onPointerUp={(event) => {
        if (event.button !== 0) return;
        event.stopPropagation();
        onInspect(objectId, event.currentTarget);
      }}
      onClick={(event) => {
        event.stopPropagation();
        if (event.detail !== 0) return;
        onInspect(objectId, event.currentTarget);
      }}
    >
      {ui(label)}
    </button>
  );
}

function outputText(source) {
  const mana = poolEntries(source.expected_mana).map(({ symbol, amount }) => `{${symbol}}`.repeat(amount)).join("");
  return mana || sourceActionLabel(source);
}

export default function ManaPaymentDecision({ decision, canAct, inlineSubmit = true, onSubmitActionChange = null, quickControls = null }) {
  const ui = useUiText();
  const { state, dispatch, dispatchInBackground, cancelBackgroundDispatch, cancelDecision } = useGame();
  const { setPreviewLinkedObjects, clearPreviewLinkedObjects, showAnchoredCardPreview } = useHover();
  const payment = state?.mana_payment || null;
  const rolloutRef = useDecisionRollout(`${decision?.source_id ?? ''}|${Boolean(payment)}`, true);
  const sharedEditor = useManaPaymentEditor();
  const localEditor = usePaymentDraft({ payment, dispatch, cancelBackgroundDispatch, enabled: !sharedEditor });
  const editor = sharedEditor || localEditor;
  const { draft, dirty, confirming } = editor;
  const [menuState, setMenu] = useState(null);
  const menu = menuState?.planId === payment?.plan_id ? menuState : null;
  const closeMenu = useCallback(() => setMenu(null), []);
  const optimizationKeyRef = useRef('');
  const sources = useMemo(() => paymentDraftRows(payment, draft), [payment, draft]);
  const rows = useMemo(() => paymentPipRows(payment, draft), [payment, draft]);
  useEffect(() => {
    setPreviewLinkedObjects(sources.map(source => source.source_id));
    return () => clearPreviewLinkedObjects();
  }, [sources, clearPreviewLinkedObjects, setPreviewLinkedObjects]);
  useEffect(() => {
    if (editor.edited || confirming || !canAct || !payment || payment.planning_complete || !dispatchInBackground) return;
    const key = `${payment.request_hash}:${payment.plan_id}`;
    if (optimizationKeyRef.current === key) return;
    optimizationKeyRef.current = key;
    dispatchInBackground();
  }, [editor.edited, confirming, canAct, dispatchInBackground, payment]);
  // Confirmation always waits for the authoritative acknowledgement of edits.
  const payDisabled = !canAct || !payment || payment.can_confirm !== true || dirty || confirming;
  const submitAction = useMemo(() => ({ label: 'Pay', disabled: payDisabled, onSubmit: editor.confirm }), [payDisabled, editor.confirm]);
  useEffect(() => { onSubmitActionChange?.(submitAction); return () => onSubmitActionChange?.(null); }, [onSubmitActionChange, submitAction]);
  const cancel = () => { closeMenu(); cancelBackgroundDispatch?.(); cancelDecision({ waitForPaymentReady: true }); };
  const pickAction = action => {
    closeMenu();
    if (action.operation === 'activate') editor.activate(action);
    else if (action.operation === 'preserve') editor.preserve(action.source_id);
    else if (action.operation === 'unpin') editor.unpin(action.source_id);
    else if (action.operation === 'current') return;
    else if (action.operation === 'life') editor.selectPip({ payment_kind: 'life', pip_id: menu.row.pip_id }, menu.row, rows);
    else editor.selectPip(action, menu.row, rows);
  };
  const openSourceMenu = (event, row) => {
    const choices = paymentOptionsForPip(payment, draft, row, rows);
    const source = row.source;
    const currentKey = source && sourceChoiceKey(source);
    const actions = choices.map(option => {
      const selected = currentKey === sourceChoiceKey(option);
      return { ...option, object_id: option.source_id, selected, operation: selected ? 'current' : undefined, label: `${option.source_name}: ${ui(outputText(option))}` };
    });
    // The reviewed current choice remains visible even while options refresh.
    if (source && !actions.some(action => action.selected)) actions.unshift({ ...source, object_id: source.source_id, selected: true, operation: 'current', label: `${source.source_name}: ${ui(outputText(source))}` });
    if (row.kind === 'pool') actions.unshift({ operation: 'current', selected: true, label: ui('Floating mana') });
    const life = payment.life_options?.find(option => option.pip_id === row.pip_id);
    if (life) actions.push({ operation: row.kind === 'life' ? 'current' : 'life', selected: row.kind === 'life', label: ui('Pay {0} life', { 0: life.life }) });
    // An ability represented by a planner option needs no separate activation.
    // Only abilities requiring unresolved choices use the explicit flow.
    if (payment.activation_options_complete !== false && !payment.activation_options_error) {
      for (const ability of payment.mana_abilities || []) {
        if (!(payment.activation_options || []).some(option => String(option.source_id) === String(ability.source_id) && option.ability_index === ability.ability_index)) {
          actions.push({ ...ability, object_id: ability.source_id, operation: 'activate', disabled: dirty || confirming, label: `${ui('Activate now')} ${ability.source_name}: ${ability.label}` });
        }
      }
    }
    setMenu({ anchor: event.currentTarget, row, actions, mode: 'sources', planId: payment.plan_id });
  };
  const openAdvancedMenu = event => {
    const preferenceSources = [...new Map([...paymentSourceOptions(payment), ...sources].map(source => [String(source.source_id), source])).values()];
    const actions = preferenceSources.map(source => ({ source_id: source.source_id, object_id: source.source_id, operation: 'preserve', selected: draft.preserved_source_ids.includes(String(source.source_id)), label: ui('Prefer to save {0}', { 0: source.source_name || source.source_id }) }));
    for (const source of preferenceSources.filter(source => sources.some(step => String(step.source_id) === String(source.source_id) && step.pinned))) {
      actions.push({ source_id: source.source_id, object_id: source.source_id, operation: 'unpin', label: ui('Let the planner choose for {0}', { 0: source.source_name || source.source_id }) });
    }
    setMenu({ anchor: event.currentTarget, actions, mode: 'advanced', planId: payment.plan_id });
  };
  if (!payment) return <div className="p-3 text-sm italic opacity-70">{ui('Preparing a mana payment plan…')}</div>;
  const warnings = [...new Set([
    payment.life_to_pay > 0 ? ui('Pay {0} life.', { 0: payment.life_to_pay }) : '',
    ...sources.filter(source => source.payment_kind === 'delve').map(source => ui('Exile {0} for delve.', { 0: source.source_name })),
    ...(payment.warnings || []).filter(warning => !String(warning).startsWith('PaysLife') && !String(warning).startsWith('UsesNonUndoSafeSource')).map(warningText),
  ].filter(Boolean))];
  const additionalSources = sources.filter(source => !rows.some(row => row.source?.choice_key === source.choice_key && row.source?.occurrence === source.occurrence));
  const hasChoices = editor.edited || draft.required_source_ids.length || draft.required_activations.length || draft.required_alternatives.length || draft.required_life_pips.length || draft.preserved_source_ids.length || draft.excluded_source_ids.some(id => !payment.fixed_excluded_source_ids?.includes(id));
  const busyLabel = dirty ? ui('Updating payment…') : !payment.planning_complete && !editor.edited ? ui('Improving') : null;
  const costPips = payment.pips || payment.payment_pips || [];
  const canConfigureSources = sources.length > 0 || paymentSourceOptions(payment).length > 0;
  return <div ref={rolloutRef} className="mana-payment-editor">
    <div className="mana-payment-editor-header">
      <div className="mana-payment-editor-heading"><div className="mana-plan-eyebrow">{ui('Mana payment')}</div>{canConfigureSources && <button type="button" className="mana-payment-source-picker" disabled={!canAct || confirming} aria-label={ui('Advanced payment controls')} aria-expanded={menu?.mode === 'advanced'} onClick={openAdvancedMenu}><SlidersHorizontal size={15} /></button>}</div>
      <h3 className="mana-payment-editor-title"><PaymentCardName objectId={decision?.source_id} onInspect={showAnchoredCardPreview}>{payment.source_name || decision?.subject}</PaymentCardName></h3>
      <div className="mana-payment-editor-cost"><span>{ui('Cost to pay')}</span><span className="inline-flex flex-wrap justify-end gap-1">{costPips.map((pip, index) => <ManaSymbol key={index} sym={pip.join('/')} size={23} />)}</span></div>
      {payment.cost_context?.length > 0 && <div className="mana-plan-cost-context">{payment.cost_context.map(context => ui(context)).join(' · ')}</div>}
    </div>
    <div className="mana-payment-editor-scroll">
      <div className="mana-payment-pip-list" aria-label={ui('Payment sources')}>
        {rows.map(row => <div key={row.pip_id} className={cn('mana-payment-pip-row', row.source?.pending && 'is-pending')} data-payment-pip-id={row.pip_id}>
          <ManaSymbol sym={row.pip.join('/')} size={24} />
          <div className="min-w-0 flex-1">
            {row.source ? <PaymentCardName objectId={row.source.source_id} onInspect={showAnchoredCardPreview} className="mana-payment-source-name">{row.source.source_name || row.source.source_id}</PaymentCardName> : <span className="mana-payment-source-name">{ui(row.kind === 'pool' ? 'Floating mana' : row.kind === 'life' ? '{0} life' : 'Choose a source', { 0: row.allocation?.life || payment.life_options?.find(option => option.pip_id === row.pip_id)?.life })}</span>}
          </div>
          <button type="button" className="mana-payment-source-picker" disabled={!canAct || confirming} aria-expanded={menu?.row?.pip_id === row.pip_id} aria-label={row.source ? ui('Choose payment for {0}', { 0: row.source.source_name || row.source.source_id }) : ui('Choose source for mana pip {0}', { 0: row.pip_id + 1 })} onClick={event => openSourceMenu(event, row)}><ChevronDown size={17} /></button>
        </div>)}
      </div>
      {additionalSources.length > 0 && <div className="mana-payment-additional-sources"><span>{ui('Additional mana sources')}</span>{additionalSources.map(source => <PaymentCardName key={`${source.choice_key}:${source.occurrence}`} objectId={source.source_id} onInspect={showAnchoredCardPreview}>{source.source_name}</PaymentCardName>)}</div>}
      {poolEntries(payment.pool_before).length > 0 && <PoolSummary label="Floating mana" pool={payment.pool_before} />}
      {poolEntries(payment.pool_after_payment).length > 0 && <PoolSummary label="After payment" pool={payment.pool_after_payment} />}
      {payment.activation_options_error && <div role="status">{ui('Payment options could not be loaded.')}</div>}
      {payment.activation_options_complete === false && <div role="status">{ui('Loading payment options…')}</div>}
      {editor.error && <div role="alert">{editor.error}</div>}
    </div>
    <div className="mana-plan-actions mana-payment-editor-footer">
      {busyLabel && <span className="mana-payment-editor-busy" role="status"><LoaderCircle size={14} className="animate-spin" />{busyLabel}</span>}
      {hasChoices && <Button type="button" variant="ghost" size="sm" className="mana-payment-reset-button" disabled={!canAct || confirming} onClick={editor.reset}><RotateCcw size={13} />{ui('Reset')}</Button>}
      <div className="mana-payment-submit-row">
      <Button type="button" variant="ghost" size="sm" className="decision-neon-button decision-neon-button--danger decision-cancel-button font-bold uppercase" disabled={!canAct || confirming} onClick={cancel}>{ui('Cancel')}</Button>
      {inlineSubmit && <div className="mana-payment-pay-region">
        <Button type="button" variant="ghost" size="sm" className="mana-payment-pay-button decision-neon-button decision-main-button decision-submit-button action-strip-submit-button font-bold uppercase" disabled={payDisabled} onClick={editor.confirm}>{ui('Pay')}</Button>
        {warnings.length > 0 && <Popover><PopoverTrigger asChild><button type="button" className="mana-payment-warning-trigger" aria-label={ui('Payment warnings')}><AlertTriangle size={17} /></button></PopoverTrigger><PopoverContent side="top" aria-label={ui('Payment warnings')} className="mana-payment-warning-details"><ul>{warnings.map((warning, index) => <li key={index}>{ui(warning)}</li>)}</ul></PopoverContent></Popover>}
      </div>}
      {inlineSubmit ? quickControls : null}
      </div>
    </div>
    {menu?.anchor.isConnected && <ActionPopover anchorElement={menu.anchor} anchorRect={menu.anchor.getBoundingClientRect()} actions={menu.actions} onAction={pickAction} onClose={closeMenu} variant="game" collapseEquivalentActions={false} previewCards={false} highlightObjects fitViewport focusOnOpen disabled={!canAct || confirming} ariaLabel={ui(menu.mode === 'advanced' ? 'Advanced payment controls' : 'Choose payment source')} />}
  </div>;
}
