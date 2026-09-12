import useUiText from "@/i18n/useUiText";
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { AlertTriangle, Check, LoaderCircle, RotateCcw, Shield, X } from "lucide-react";
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

function idSet(values) {
  return new Set((values || []).map((value) => String(value)));
}

function sortedIds(values) {
  return Array.from(values, String).sort((a, b) => {
    if (a.length !== b.length) return a.length - b.length;
    return a.localeCompare(b);
  });
}

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
  return source.planned ? "" : "Available permanent";
}

function pipPaymentLabel(allocation) {
  if (!allocation) return "planned";
  if (allocation.payment_kind === "life") return `${allocation.life || 0} life`;
  if (allocation.payment_kind === "convoke") return "convoke";
  if (allocation.payment_kind === "improvise") return "improvise";
  if (allocation.payment_kind === "assist") return "assist";
  return allocation.symbol ? `pay ${allocation.symbol}` : "mana";
}

function SourceConstraintButtons({ sourceId, required, excluded, preserved, onChange, disabled }) {
  const ui = useUiText();
  return (
    <div className="flex shrink-0 items-center gap-1" aria-label={ui("Payment source preference")}>
      <button
        type="button"
        disabled={disabled}
        className={cn("mana-plan-constraint", required && "is-required")}
        onClick={() => onChange(sourceId, "required")}
        aria-pressed={required}
        title={ui("Require this source")}
      >
        <Check size={13} />
      </button>
      <button
        type="button"
        disabled={disabled}
        className={cn("mana-plan-constraint", preserved && "is-preserved")}
        onClick={() => onChange(sourceId, "preserved")}
        aria-pressed={preserved}
        title={ui("Prefer to preserve this source")}
      >
        <Shield size={13} />
      </button>
      <button
        type="button"
        disabled={disabled}
        className={cn("mana-plan-constraint", excluded && "is-excluded")}
        onClick={() => onChange(sourceId, "excluded")}
        aria-pressed={excluded}
        title={ui("Exclude this source")}
      >
        <X size={13} />
      </button>
    </div>
  );
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

export default function ManaPaymentDecision({
  decision,
  canAct,
  inlineSubmit = true,
  onSubmitActionChange = null,
  layout = "panel",
}) {
  const ui = useUiText();
  const { state, dispatch, dispatchInBackground } = useGame();
  const {
    setPreviewLinkedObjects,
    clearPreviewLinkedObjects,
    showAnchoredCardPreview,
  } = useHover();
  const payment = state?.mana_payment || null;
  const stripLayout = layout === "strip";
  const [adjusting, setAdjusting] = useState(false);
  const [required, setRequired] = useState(() => idSet(payment?.required_source_ids));
  const [excluded, setExcluded] = useState(() => idSet(payment?.excluded_source_ids));
  const [preserved, setPreserved] = useState(() => idSet(payment?.preserved_source_ids));
  const [preferLife, setPreferLife] = useState(() => Boolean(payment?.prefer_life));
  const payWhenReadyRef = useRef(false);
  const optimizationKeyRef = useRef("");

  const plannedIds = useMemo(
    () => (payment?.planned_sources || []).map((source) => source.source_id),
    [payment?.planned_sources]
  );
  const allocationsByIndex = useMemo(() => new Map(
    (payment?.allocations || []).map((allocation) => [Number(allocation.printed_index), allocation])
  ), [payment?.allocations]);
  useEffect(() => {
    setPreviewLinkedObjects(plannedIds);
    return () => clearPreviewLinkedObjects();
  }, [clearPreviewLinkedObjects, plannedIds, setPreviewLinkedObjects]);

  const sourceRows = useMemo(() => {
    const rows = new Map();
    for (const source of payment?.planned_sources || []) {
      rows.set(String(source.source_id), { ...source, planned: true });
    }
    if (adjusting) {
      for (const source of payment?.available_sources || []) {
        const id = String(source.source_id);
        if (!rows.has(id)) {
          rows.set(id, {
            ...source,
            planned: false,
            undo_safe: true,
            expected_mana: null,
          });
        }
      }
    }
    return Array.from(rows.values());
  }, [adjusting, payment?.available_sources, payment?.planned_sources]);

  const sendConfirmation = useCallback((currentPayment) => {
    if (!currentPayment || currentPayment.can_confirm === false) return;
    dispatch({
      type: "mana_payment",
      response: {
        action: "confirm",
        plan_id: String(currentPayment.plan_id),
        request_hash: String(currentPayment.request_hash),
      },
    }, `Paid mana for ${currentPayment.source_name || decision.subject}`, { waitForPaymentReady: true });
  }, [decision.subject, dispatch]);

  const confirm = useCallback(() => {
    if (!payment || payment.can_confirm === false) return;
    if (!payment.planning_complete) {
      payWhenReadyRef.current = true;
      return;
    }
    sendConfirmation(payment);
  }, [payment, sendConfirmation]);

  useEffect(() => {
    if (!payWhenReadyRef.current || !payment?.planning_complete) return;
    payWhenReadyRef.current = false;
    sendConfirmation(payment);
  }, [payment, sendConfirmation]);

  useEffect(() => {
    if (!canAct || !payment || payment.planning_complete || !dispatchInBackground) return;
    const optimizationKey = `${payment.request_hash}:${payment.plan_id}`;
    if (optimizationKeyRef.current === optimizationKey) return;
    optimizationKeyRef.current = optimizationKey;
    dispatchInBackground({
      type: "mana_payment",
      response: {
        action: "replan",
        required_source_ids: (payment.required_source_ids || []).map(String),
        excluded_source_ids: (payment.excluded_source_ids || []).map(String),
        preserved_source_ids: (payment.preserved_source_ids || []).map(String),
        prefer_life: Boolean(payment.prefer_life),
      },
    });
  }, [canAct, dispatchInBackground, payment]);

  const cancel = useCallback(() => {
    dispatch({ type: "mana_payment", response: { action: "cancel" } }, "Mana payment cancelled");
  }, [dispatch]);

  const replan = useCallback(() => {
    dispatch({
      type: "mana_payment",
      response: {
        action: "replan",
        required_source_ids: sortedIds(required),
        excluded_source_ids: sortedIds(excluded),
        preserved_source_ids: sortedIds(preserved),
        prefer_life: preferLife,
      },
    }, "Mana payment plan adjusted");
    setAdjusting(false);
  }, [dispatch, excluded, preferLife, preserved, required]);

  const changeConstraint = useCallback((sourceId, kind) => {
    const id = String(sourceId);
    const setters = { required: setRequired, excluded: setExcluded, preserved: setPreserved };
    const setter = setters[kind];
    setter((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
    if (kind === "required") setExcluded((current) => new Set([...current].filter((value) => value !== id)));
    if (kind === "excluded") setRequired((current) => new Set([...current].filter((value) => value !== id)));
  }, []);

  const resetAdjustments = useCallback(() => {
    setRequired(idSet(payment?.required_source_ids));
    setExcluded(idSet(payment?.excluded_source_ids));
    setPreserved(idSet(payment?.preserved_source_ids));
    setPreferLife(Boolean(payment?.prefer_life));
    setAdjusting(false);
  }, [payment]);

  const submitAction = useMemo(() => ({
    label: "Pay",
    disabled: !canAct || !payment || payment.can_confirm === false,
    onSubmit: confirm,
    secondaryAction: {
      label: adjusting ? "Use these sources" : "Change sources",
      disabled: !canAct || !payment,
      active: adjusting,
      onSubmit: adjusting ? replan : () => setAdjusting(true),
    },
  }), [adjusting, canAct, confirm, payment, replan]);
  useEffect(() => {
    if (!onSubmitActionChange) return undefined;
    onSubmitActionChange(submitAction);
    return () => onSubmitActionChange(null);
  }, [onSubmitActionChange, submitAction]);

  if (!payment) {
    return <div className="p-3 text-sm italic opacity-70">{ui("Preparing a mana payment plan…")}</div>;
  }

  if (stripLayout) {
    const warningSummary = [
      payment.life_to_pay > 0 ? `Pay ${payment.life_to_pay} life.` : "",
      ...(payment.warnings || [])
        .filter((warning) => !String(warning).startsWith("PaysLife"))
        .map(warningText),
    ].filter(Boolean).join(" ");

    return (
      <div className={cn("mana-plan-strip", adjusting && "is-adjusting")}>
        <div
          className="mana-plan-strip-source-region"
          aria-label={ui(adjusting ? "Available sources" : "Planned sources")}
        >
          <div className="mana-plan-strip-source-scroller">
            {sourceRows.length ? sourceRows.map((source, index) => {
              const id = String(source.source_id);
              const produced = poolEntries(source.expected_mana);
              return (
                <div
                  key={`${id}-${index}`}
                  className={cn(
                    "mana-plan-strip-source",
                    source.planned && "is-planned",
                    excluded.has(id) && "is-excluded",
                    required.has(id) && "is-required",
                  )}
                >
                  <span className="mana-plan-source-index">{source.planned ? index + 1 : "·"}</span>
                  <span className="mana-plan-strip-source-copy">
                    <span className="mana-plan-strip-source-name-row">
                      <PaymentCardName
                        objectId={id}
                        onInspect={showAnchoredCardPreview}
                        className="mana-plan-strip-source-name"
                      >
                        {source.source_name}
                      </PaymentCardName>
                      {produced.length ? (
                        <span className="mana-plan-strip-produced">
                          {produced.map(({ symbol, amount }) => (
                            <span key={symbol} className="mana-plan-strip-pool-symbol">
                              <ManaSymbol sym={symbol} size={15} />{amount > 1 ? `×${amount}` : ""}
                            </span>
                          ))}
                        </span>
                      ) : null}
                    </span>
                    <span className="mana-plan-strip-source-action">
                      {[sourceActionLabel(source), !source.undo_safe && "no undo"].filter(Boolean).map(value => ui(value)).join(" · ")}
                    </span>
                  </span>
                  {adjusting ? (
                    <SourceConstraintButtons
                      sourceId={source.source_id}
                      required={required.has(id)}
                      excluded={excluded.has(id)}
                      preserved={preserved.has(id)}
                      onChange={changeConstraint}
                      disabled={!canAct}
                    />
                  ) : null}
                </div>
              );
            }) : (
              <span className="mana-plan-strip-no-sources">{payment.can_confirm === false ? ui("The cost needs more mana. Activate sources or change the plan.") : ui("Floating mana covers the cost.")}</span>
            )}
          </div>
        </div>

        {warningSummary ? (
          <div className="mana-plan-strip-warning" title={ui(warningSummary)} aria-label={ui(warningSummary)}>
            <AlertTriangle size={15} />
            <span>{payment.life_to_pay > 0 ? ui("{0} life", { 0: payment.life_to_pay }) : ui("Warning")}</span>
          </div>
        ) : null}

        {!payment.planning_complete ? (
          <div className="mana-plan-strip-planning" title={ui("Checking for a better payment plan")}>
            <LoaderCircle size={15} className="animate-spin" />
            <span>{ui("Improving")}</span>
          </div>
        ) : null}

        {adjusting ? (
          <label className="mana-plan-strip-life-preference" title={ui("Prefer legal life payments over spending mana")}>
            <input
              type="checkbox"
              checked={preferLife}
              disabled={!canAct}
              onChange={(event) => setPreferLife(event.target.checked)}
            />
            <span>{ui("Life first")}</span>
          </label>
        ) : null}

        <div className="mana-plan-strip-actions">
          {adjusting ? (
            <Button type="button" variant="ghost" size="sm" onClick={resetAdjustments}>
              <RotateCcw size={13} />{" " + ui("Reset")}</Button>
          ) : null}
        </div>
      </div>
    );
  }

  const content = (
    <div className="mana-plan-content">
      <div className="mana-plan-heading">
        <div>
          <div className="mana-plan-eyebrow">{ui("Mana payment")}</div>
          <h3 className="mana-plan-title">
            <PaymentCardName objectId={decision?.source_id} onInspect={showAnchoredCardPreview}>
              {payment.source_name || decision.subject}
            </PaymentCardName>
          </h3>
        </div>
        <div className="flex flex-wrap items-center justify-end gap-1">
          {(payment.pips || []).map((pip, index) => (
            <span
              key={`${pip.join("-")}-${index}`}
              className={cn("mana-plan-pip", `is-${allocationsByIndex.get(index)?.payment_kind || "planned"}`)}
              title={ui(pipPaymentLabel(allocationsByIndex.get(index)))}
            >
              <ManaSymbol sym={pip.join("/")} size={22} />
              <span className="mana-plan-pip-method">
                {ui(pipPaymentLabel(allocationsByIndex.get(index)))}
              </span>
            </span>
          ))}
        </div>
      </div>

      <div className="mana-plan-pools">
        <PoolSummary label={ui("Pool now")} pool={payment.pool_before} />
        <span className="mana-plan-arrow">→</span>
        <PoolSummary label={ui("After sources")} pool={payment.pool_after_activations} />
        <span className="mana-plan-arrow">→</span>
        <PoolSummary label={ui("After payment")} pool={payment.pool_after_payment} />
      </div>

      <div className="mana-plan-section">
        <div className="mana-plan-section-title">
          <span>{ui("Planned sources")}</span>
          <span className="font-normal opacity-65">{plannedIds.length}{" " + ui("source")}{plannedIds.length === 1 ? "" : ui("s")}</span>
        </div>
        <div className="mana-plan-source-list">
          {sourceRows.length ? sourceRows.map((source, index) => {
            const id = String(source.source_id);
            const produced = poolEntries(source.expected_mana);
            return (
              <div
                key={`${id}-${index}`}
                className={cn(
                  "mana-plan-source",
                  source.planned && "is-planned",
                  excluded.has(id) && "is-excluded",
                  required.has(id) && "is-required",
                )}
              >
                <span className="mana-plan-source-index">{source.planned ? index + 1 : "·"}</span>
                <span className="min-w-0 flex-1">
                  <PaymentCardName
                    objectId={id}
                    onInspect={showAnchoredCardPreview}
                    className="block w-full max-w-full text-sm font-semibold"
                  >
                    {source.source_name}
                  </PaymentCardName>
                  <span className="flex items-center gap-1 text-[11px] opacity-70">
                    {[sourceActionLabel(source), !source.undo_safe && "cannot safely undo"].filter(Boolean).map(value => ui(value)).join(" · ")}
                  </span>
                </span>
                {produced.length ? (
                  <span className="flex items-center gap-0.5">
                    {produced.map(({ symbol, amount }) => (
                      <span key={symbol} className="inline-flex items-center gap-0.5 text-xs">
                        <ManaSymbol sym={symbol} size={16} />{amount > 1 ? `×${amount}` : ""}
                      </span>
                    ))}
                  </span>
                ) : null}
                {adjusting ? (
                  <SourceConstraintButtons
                    sourceId={source.source_id}
                    required={required.has(id)}
                    excluded={excluded.has(id)}
                    preserved={preserved.has(id)}
                    onChange={changeConstraint}
                    disabled={!canAct}
                  />
                ) : null}
              </div>
            );
          }) : (
            <div className="mana-plan-empty">{payment.can_confirm === false ? ui("The cost needs more mana. Activate sources or change the plan.") : ui("The floating mana pool already covers this cost.")}</div>
          )}
        </div>
      </div>

      {(payment.warnings || []).length || payment.life_to_pay > 0 ? (
        <div className="mana-plan-warnings">
          <AlertTriangle size={15} />
          <div>
            {payment.life_to_pay > 0 ? <div>{ui("Pay") + " "}{payment.life_to_pay}{" " + ui("life.")}</div> : null}
            {(payment.warnings || []).filter((warning) => !String(warning).startsWith("PaysLife")).map((warning, index) => (
              <div key={`${warning}-${index}`}>{ui(warningText(warning))}</div>
            ))}
          </div>
        </div>
      ) : null}

      {adjusting ? (
        <label className="mana-plan-life-preference">
          <input
            type="checkbox"
            checked={preferLife}
            disabled={!canAct}
            onChange={(event) => setPreferLife(event.target.checked)}
          />{ui("Prefer legal life payments over spending mana")}</label>
      ) : null}

    </div>
  );

  return (
    <div className="flex h-full min-h-0 flex-col">
      <ScrollArea className="min-h-0 flex-1">{content}</ScrollArea>
      <div className="mana-plan-actions">
        <Button type="button" variant="ghost" size="sm" disabled={!canAct} onClick={cancel}>{ui("Cancel")}</Button>
        {adjusting ? (
          <>
            <Button type="button" variant="outline" size="sm" onClick={resetAdjustments}>
              <RotateCcw size={14} />{" " + ui("Reset")}</Button>
            <Button type="button" size="sm" disabled={!canAct} onClick={replan}>{ui("Replan")}</Button>
          </>
        ) : (
          <>
            <Button type="button" variant="outline" size="sm" disabled={!canAct} onClick={() => setAdjusting(true)}>{ui("Plan")}</Button>
            {inlineSubmit ? (
              <Button type="button" size="sm" disabled={!canAct || payment.can_confirm === false} onClick={confirm}>{ui("Pay")}</Button>
            ) : null}
          </>
        )}
      </div>
    </div>
  );
}
