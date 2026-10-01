import useUiText from "@/i18n/useUiText";
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { AlertTriangle, LoaderCircle, LockKeyhole, Plus, RotateCcw, X } from "lucide-react";
import usePaymentDraft from "@/hooks/usePaymentDraft";
import { useManaPaymentEditor } from "@/context/ManaPaymentEditorContext.shared";
import ActionPopover from "@/components/overlays/ActionPopover";
import { paymentDraftRows, paymentSourceOptions } from "@/lib/payment-draft";
import { useGame } from "@/context/GameContext";
import { useHover } from "@/context/HoverContext";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
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
  if (text.startsWith("UsesNonUndoSafeSource")) return "This plan uses a source that cannot be safely undone.";
  if (text.startsWith("UsesPreservedSource")) return "This plan uses a source marked Preserve.";
  if (text.startsWith("ProducesExcessMana")) return "This plan leaves mana floating after payment.";
  if (text.startsWith("PaysLife")) return "This plan pays life.";
  return text.replace(/([a-z])([A-Z])/g, "$1 $2");
}

function sourceActionLabel(source) {
  if (source.payment_kind === "convoke") return "Tap for convoke";
  if (source.payment_kind === "improvise") return "Tap for improvise";
  if (source.payment_kind === "delve") return "Exile for delve";
  return "";
}

function PaymentCardName({ objectId, onInspect, children, className = "" }) {
  const ui = useUiText();
  const containerRef = useRef(null);
  const textRef = useRef(null);
  useLayoutEffect(() => {
    const container = containerRef.current;
    const text = textRef.current;
    if (!container || !text) return undefined;
    const fit = () => {
      text.style.fontSize = "inherit";
      const baseSize = parseFloat(getComputedStyle(container).fontSize);
      const available = container.clientWidth;
      const width = text.getBoundingClientRect().width;
      if (available > 0 && width > available) {
        text.style.fontSize = `${baseSize * available / width}px`;
      }
    };
    fit();
    const observer = new ResizeObserver(fit);
    observer.observe(container);
    let disposed = false;
    document.fonts?.ready.then(() => { if (!disposed) fit(); });
    return () => { disposed = true; observer.disconnect(); };
  }, [children]);
  const label = <span ref={textRef} className="inline-block whitespace-nowrap">{children}</span>;
  if (objectId == null || typeof onInspect !== "function") {
    return <span ref={containerRef} className={cn("block min-w-0 max-w-full overflow-hidden whitespace-nowrap", className)}>{ui(label)}</span>;
  }
  return (
    <button
      type="button"
      ref={containerRef}
      className={cn("decision-card-name-trigger block min-w-0 max-w-full overflow-hidden whitespace-nowrap", className)}
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

export default function ManaPaymentDecision({ decision, canAct, inlineSubmit = true, onSubmitActionChange = null, layout = "panel" }) {
  const ui = useUiText();
  const { state, dispatch, dispatchInBackground, cancelBackgroundDispatch } = useGame();
  const { setPreviewLinkedObjects, clearPreviewLinkedObjects, showAnchoredCardPreview } = useHover();
  const payment = state?.mana_payment || null;
  const sharedEditor = useManaPaymentEditor();
  const localEditor = usePaymentDraft({ payment, dispatch, cancelBackgroundDispatch, enabled: !sharedEditor });
  const editor = sharedEditor || localEditor;
  const { draft, dirty, confirming } = editor;
  const [menu, setMenu] = useState(null);
  const closeMenu = useCallback(() => setMenu(null), []);
  const optimizationKeyRef = useRef("");
  const strip = layout === "strip";
  const sources = useMemo(() => paymentDraftRows(payment, draft), [payment, draft]);
  const options = useMemo(() => paymentSourceOptions(payment), [payment]);
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
  const payDisabled = !canAct || !payment || payment.can_confirm === false || dirty || confirming;
  const submitAction = useMemo(() => ({ label: "Pay", disabled: payDisabled, onSubmit: editor.confirm }), [payDisabled, editor.confirm]);
  useEffect(() => {
    onSubmitActionChange?.(submitAction);
    return () => onSubmitActionChange?.(null);
  }, [onSubmitActionChange, submitAction]);
  const cancel = () => { cancelBackgroundDispatch?.(); dispatch({ type: "mana_payment", response: { action: "cancel" } }, "Mana payment cancelled", { waitForPaymentReady: true }); };
  const pickAction = action => {
    closeMenu();
    if (action.operation === "activate") {
      editor.activate(action);
    } else if (action.operation === "exclude") editor.exclude(action.source_id);
    else if (action.operation === "restore") editor.restore(action.source_id);
    else if (action.operation === "preserve") editor.preserve(action.source_id);
    else if (action.operation === "unpin") editor.unpin(action.source_id);
    else editor.select(action, { replace: !action.repeatable, step: menu?.source });
  };
  const openSourceMenu = (event, source = null) => {
    const id = source && String(source.source_id);
    const choices = options.filter(option => !id || String(option.source_id) === id);
    const actions = choices.map(option => ({ ...option, label: `${option.source_name}: ${ui(outputText(option))}${option.payment_kind === "mana_ability" ? ` · ${option.label}` : ""}` }));
    if (source) {
      actions.push({ source_id: id, operation: "exclude", label: ui("Keep this source unused") });
      actions.push({ source_id: id, operation: "preserve", label: ui(draft.preserved_source_ids.includes(id) ? "Remove save preference" : "Prefer to save this source") });
      if (source.pinned) actions.push({ source_id: id, operation: "unpin", label: ui("Let the planner choose") });
      for (const ability of payment.mana_abilities || []) if (String(ability.source_id) === id) {
        actions.push({ ...ability, operation: "activate", disabled: dirty || confirming, label: `${ui("Activate now")}: ${ability.label}` });
      }
    } else {
      // Complex abilities that cannot be represented by an exact planned output
      // remain explicit activations, with their own subsequent choices.
      for (const ability of payment.mana_abilities || []) if (!choices.some(option => String(option.source_id) === String(ability.source_id) && option.ability_index === ability.ability_index)) {
        actions.push({ ...ability, operation: "activate", disabled: dirty || confirming, label: `${ui("Activate now")} ${ability.source_name}: ${ability.label}` });
      }
      for (const sourceId of draft.excluded_source_ids.filter(id => !payment.fixed_excluded_source_ids?.includes(id))) {
        const sourceName = payment.available_sources?.find(value => String(value.source_id) === sourceId)?.source_name || sourceId;
        actions.push({ source_id: sourceId, operation: "restore", label: `${ui("Allow again")}: ${sourceName}` });
      }
    }
    if (actions.length) setMenu({ anchor: event.currentTarget, source, actions });
  };
  if (!payment) return <div className="p-3 text-sm italic opacity-70">{ui("Preparing a mana payment plan…")}</div>;
  const warnings = [
    payment.life_to_pay > 0 ? ui("Pay {0} life.", { 0: payment.life_to_pay }) : "",
    ...sources.filter(source => source.payment_kind === "delve").map(source => ui("Exile {0} for delve.", { 0: source.source_name })),
    ...sources.filter(source => source.undo_safe === false && source.payment_kind === "mana_ability").map(source => {
      const ability = payment.mana_abilities?.find(ability => String(ability.source_id) === String(source.source_id) && ability.ability_index === source.ability_index);
      return `${source.source_name}: ${ability?.label || ui("Cannot safely undo")}`;
    }),
    ...(payment.warnings || []).filter(warning => !String(warning).startsWith("PaysLife")).map(warningText),
  ].filter(Boolean);
  const sourceList = <div className={strip ? "mana-plan-strip-source-scroller" : "mana-plan-source-list"} aria-label={ui("Payment sources")}>
    {sources.map((source, index) => <div key={`${source.choice_key}:${source.occurrence}`} className={cn(strip ? "mana-plan-strip-source" : "mana-plan-source", "is-planned", source.pinned && "is-required", source.pending && "is-pending")}>
      <span className="mana-plan-source-index" title={ui(source.pinned ? "Your choice" : "Automatic choice")}>
        {source.pinned ? <LockKeyhole size={12} /> : index + 1}
      </span>
      <span className={strip ? "mana-plan-strip-source-copy" : "min-w-0 flex-1"}>
        <PaymentCardName objectId={source.source_id} onInspect={showAnchoredCardPreview} className={strip ? "mana-plan-strip-source-name" : "text-sm font-semibold"}>{source.source_name || source.source_id}</PaymentCardName>
        <span className="mana-plan-strip-source-action">{ui(sourceActionLabel(source)) || ui(source.pinned ? "Your choice" : "Automatic")}</span>
      </span>
      <button type="button" className="mana-plan-output" disabled={!canAct || confirming} onClick={event => openSourceMenu(event, source)} aria-label={ui("Choose payment for {0}", { 0: source.source_name || source.source_id })} title={ui("Choose ability or mana output")}>
        {poolEntries(source.expected_mana).map(({ symbol, amount }) => <span key={symbol} className="inline-flex items-center gap-0.5"><ManaSymbol sym={symbol} size={16} />{amount > 1 ? `×${amount}` : ""}</span>)}
        <span aria-hidden="true">⌄</span>
      </button>
      <button type="button" className="mana-plan-constraint" disabled={!canAct || confirming} aria-label={ui("Remove {0} from payment", { 0: source.source_name || source.source_id })} title={ui("Remove this payment source")} onClick={() => editor.remove(source)}><X size={13} /></button>
    </div>)}
    {!sources.length && <span className="mana-plan-empty">{ui(payment.can_confirm === false ? "Choose another source to cover the cost." : "Floating mana covers the cost.")}</span>}
    <Button type="button" variant="outline" size="sm" disabled={!canAct || confirming} onClick={event => openSourceMenu(event)} aria-label={ui("Add payment source")} className="mana-plan-add"><Plus size={14} />{ui("Source")}</Button>
  </div>;
  const lifeChoices = payment.life_options?.length ? <div className="mana-plan-life-choices" aria-label={ui("Life payment choices")}>
    {payment.life_options.map(option => <button type="button" key={option.pip_id} className="mana-plan-life-choice" disabled={!canAct || confirming} aria-pressed={draft.required_life_pips.includes(option.pip_id)} title={ui("Choose life for mana pip {0}", { 0: option.pip_id + 1 })} onClick={() => editor.toggleLife(option.pip_id)}>
      {payment.payment_pips?.[option.pip_id] && <ManaSymbol sym={payment.payment_pips[option.pip_id].join("/")} size={17} />}
      {ui("Pip {0}: {1} life", { 0: option.pip_id + 1, 1: option.life })} · {ui(draft.required_life_pips.includes(option.pip_id) ? "Selected" : "Auto")}
    </button>)}
  </div> : null;
  const busyLabel = dirty ? ui("Updating payment…") : !payment.planning_complete && !editor.edited ? ui("Improving") : null;
  const hasChoices = editor.edited || draft.required_source_ids.length || draft.required_activations.length || draft.required_alternatives.length || draft.required_life_pips.length || draft.preserved_source_ids.length || draft.excluded_source_ids.some(id => !payment.fixed_excluded_source_ids?.includes(id));
  const resetControl = hasChoices ? <Button type="button" variant="ghost" size="sm" disabled={!canAct || confirming} onClick={editor.reset} title={ui("Clear payment choices; actual activations stay paid")}><RotateCcw size={13} />{ui("Reset")}</Button> : null;
  const controls = <>
    {busyLabel && <span className="mana-plan-strip-planning" role="status"><LoaderCircle size={14} className="animate-spin" />{busyLabel}</span>}
    {strip && resetControl}
  </>;
  const popover = menu?.anchor.isConnected ? <ActionPopover anchorElement={menu.anchor} anchorRect={menu.anchor.getBoundingClientRect()} actions={menu.actions} onAction={pickAction} onClose={closeMenu} variant="game" collapseEquivalentActions={false} previewCards={false} fitViewport focusOnOpen disabled={!canAct || confirming} ariaLabel={ui("Choose payment source")} /> : null;
  if (strip) return <div className="mana-plan-strip">
    {payment.cost_context?.length > 0 && <span className="mana-plan-strip-context" title={payment.cost_context.map(context => ui(context)).join(" · ")}>{payment.cost_context.map(context => ui(context)).join(" · ")}</span>}
    <div className="mana-plan-strip-source-region">{sourceList}</div>
    {lifeChoices}
    {warnings.length > 0 && <div className="mana-plan-strip-warning" title={warnings.join(" ")} aria-label={warnings.join(" ")}><AlertTriangle size={15} /><span>{payment.life_to_pay > 0 ? ui("{0} life", { 0: payment.life_to_pay }) : ui("Warning")}</span></div>}
    {controls}{popover}
  </div>;
  return <div className="flex h-full min-h-0 flex-col">
    <ScrollArea className="min-h-0 flex-1"><div className="mana-plan-content">
      <div className="mana-plan-heading"><div><div className="mana-plan-eyebrow">{ui("Mana payment")}</div><h3 className="mana-plan-title"><PaymentCardName objectId={decision?.source_id} onInspect={showAnchoredCardPreview}>{payment.source_name || decision.subject}</PaymentCardName></h3></div>
        <div className="flex flex-wrap gap-1">{(payment.payment_pips || payment.pips || []).map((pip, index) => {
          const allocation = payment.allocations?.find(allocation => allocation.pip_id === index);
          const label = allocation?.payment_kind === "life" ? ui("{0} life", { 0: allocation.life }) : allocation?.payment_kind === "mana" ? allocation.symbol : allocation?.payment_kind;
          return <span className="mana-plan-pip" key={index} title={ui("Mana pip {0}", { 0: index + 1 })}><ManaSymbol sym={pip.join("/")} size={22} />{label && <span className="mana-plan-pip-method">{ui(label)}</span>}</span>;
        })}</div>
      </div>
      {payment.cost_context?.length > 0 && <div className="mana-plan-cost-context">{payment.cost_context.map(context => ui(context)).join(" · ")}</div>}
      <div className="mana-plan-pools"><PoolSummary label={ui("Pool now")} pool={payment.pool_before} /><span className="mana-plan-arrow">→</span><PoolSummary label={ui("After sources")} pool={payment.pool_after_activations} /><span className="mana-plan-arrow">→</span><PoolSummary label={ui("After payment")} pool={payment.pool_after_payment} /></div>
      <div className="mana-plan-section"><div className="mana-plan-section-title">{ui("Payment sources")}</div>{sourceList}</div>
      {lifeChoices}
      {payment.can_confirm === false && !dirty && <div className="mana-plan-warnings" role="status">{ui("These choices do not cover the cost. Add a source or remove a restriction.")}</div>}
      {warnings.length > 0 && <div className="mana-plan-warnings"><AlertTriangle size={15} /><div>{warnings.map((warning, index) => <div key={index}>{ui(warning)}</div>)}</div></div>}
      {payment.reserved_sources?.length > 0 && <div className="mana-plan-reservations">{payment.reserved_sources.map(source => <div key={source.source_id}>{source.source_name}: {ui(source.reason)}</div>)}</div>}
      {editor.error && <div role="alert">{editor.error}</div>}
      {controls}
    </div></ScrollArea>
    <div className="mana-plan-actions"><Button type="button" variant="ghost" size="sm" disabled={!canAct || confirming} onClick={cancel}>{ui("Cancel")}</Button>{resetControl}{inlineSubmit && <Button type="button" size="sm" disabled={payDisabled} onClick={editor.confirm}>{ui("Pay")}</Button>}</div>
    {popover}
  </div>;
}
