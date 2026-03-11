import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useGame } from "@/context/GameContext";
import { getVisibleTopStackObject } from "@/lib/stack-targets";
import { useHover } from "@/context/HoverContext";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Input } from "@/components/ui/input";
import { ChevronDown, ChevronUp } from "lucide-react";
import { cn } from "@/lib/utils";
import { SymbolText } from "@/lib/mana-symbols";
import {
  buildTriggerOrderingKey,
  defaultTriggerOrderingOrder,
  isTriggerOrderingDecision,
  normalizeTriggerOrderingOrder,
} from "@/lib/trigger-ordering";
import { normalizeDecisionText } from "./decisionText";
import DecisionSummary from "./DecisionSummary";

const STRIP_ITEM_BASE_CLASS = "h-auto min-h-8 max-w-[360px] min-w-[120px] justify-start self-stretch rounded-none border-0 border-l-2 border-l-[rgba(116,139,164,0.42)] bg-[rgba(12,22,34,0.58)] px-2.5 text-left text-[12px] font-semibold text-[rgba(206,223,242,0.52)] whitespace-nowrap transition-all hover:border-l-[rgba(236,245,255,0.92)] hover:bg-[rgba(220,236,255,0.16)] hover:text-[#f4f9ff] hover:shadow-[0_0_12px_rgba(236,245,255,0.3)]";
const STRIP_ITEM_ACTIVE_CLASS = "border-l-[rgba(236,245,255,0.9)] bg-[rgba(220,236,255,0.16)] text-[#f4f9ff] shadow-[0_0_12px_rgba(236,245,255,0.3)]";
const STRIP_ITEM_DISABLED_CLASS = "border-l-[rgba(63,79,98,0.6)] bg-[rgba(8,15,23,0.76)] text-[#5f7590] hover:border-l-[rgba(63,79,98,0.6)] hover:bg-[rgba(8,15,23,0.76)] hover:text-[#5f7590] hover:shadow-none";
function isPaymentOptionDescription(text) {
  return /^\s*pay\b/i.test(String(text || ""));
}

function isCastOptionDescription(text) {
  return /^\s*cast\b/i.test(String(text || ""));
}

function isPlayOptionDescription(text) {
  return /^\s*play\b/i.test(String(text || ""));
}

function isPaymentDecision(decision) {
  if (!decision || decision.kind !== "select_options") return false;
  const reason = String(decision.reason || "").toLowerCase();
  if (reason.includes("next cost")) return false;
  if (isPaymentOptionDescription(decision.description || "")) return true;
  return (decision.options || []).some((opt) => isPaymentOptionDescription(opt.description));
}

function isSpellCastFlowDecision(decision) {
  if (!decision || decision.kind !== "select_options") return false;
  if (isCastOptionDescription(decision.description || "")) return true;
  return (decision.options || []).some((opt) => isCastOptionDescription(opt.description));
}

function buildContextualOptions(options, hoveredObjectId) {
  const hasObjectBoundOptions = options.some((opt) => opt.object_id != null);
  if (!hasObjectBoundOptions) {
    return {
      options,
      waitingForHover: false,
    };
  }

  const hasHoveredObject = hoveredObjectId != null;
  const hasMatchedHover = hasHoveredObject && options.some(
    (opt) => opt.object_id != null && String(opt.object_id) === String(hoveredObjectId)
  );

  const contextualOptions = options.filter((opt) => {
    if (opt.object_id == null) return true;
    return hasMatchedHover && String(opt.object_id) === String(hoveredObjectId);
  });

  return {
    options: contextualOptions,
    waitingForHover: !hasMatchedHover,
  };
}

function optionsSignature(options) {
  return (options || [])
    .map((option) => `${Number(option?.index)}:${String(option?.description || "")}`)
    .join("|");
}

function useAnimatedRows(rows, showRows, hideDelayMs = 180) {
  const [visibleRows, setVisibleRows] = useState(rows);
  const hideTimerRef = useRef(null);

  useEffect(() => {
    if (hideTimerRef.current) {
      clearTimeout(hideTimerRef.current);
      hideTimerRef.current = null;
    }

    hideTimerRef.current = setTimeout(() => {
      setVisibleRows(showRows ? rows : []);
      hideTimerRef.current = null;
    }, showRows ? 0 : hideDelayMs);
  }, [rows, showRows, hideDelayMs]);

  useEffect(
    () => () => {
      if (hideTimerRef.current) {
        clearTimeout(hideTimerRef.current);
        hideTimerRef.current = null;
      }
    },
    []
  );

  return visibleRows;
}

function HoverHint({ text }) {
  return (
    <div className="text-[12px] italic text-[#89a7c7] px-1 pb-0.5 leading-snug">
      {text}
    </div>
  );
}

function OptionButton({
  opt,
  canAct,
  onClick,
  isHighlighted,
  isSelected = false,
  onMouseEnter,
  onMouseLeave,
  horizontal = false,
}) {
  const disabled = !canAct || opt.legal === false;

  return (
    <Button
      variant="ghost"
      size="sm"
      className={cn(
        horizontal
          ? STRIP_ITEM_BASE_CLASS
          : "h-auto min-h-8 w-full justify-start rounded-none border-0 bg-[rgba(15,27,40,0.9)] px-2.5 py-1.5 text-left text-[13px] text-[#c7dbf2] whitespace-normal transition-all hover:bg-[rgba(25,44,66,0.95)] hover:text-[#eaf3ff]",
        horizontal && isSelected && STRIP_ITEM_ACTIVE_CLASS,
        !horizontal && isSelected && "bg-[rgba(36,58,84,0.72)] text-[#eaf4ff]",
        horizontal && !isSelected && isHighlighted && STRIP_ITEM_ACTIVE_CLASS,
        !horizontal && !isSelected && isHighlighted && "bg-[rgba(25,47,71,0.94)] text-[#d9ecff]",
        disabled
          && (horizontal
            ? STRIP_ITEM_DISABLED_CLASS
            : "bg-[rgba(12,20,30,0.72)] text-[#647f99] hover:bg-[rgba(12,20,30,0.72)] hover:text-[#647f99]")
      )}
      disabled={disabled}
      onPointerDown={(e) => {
        if (disabled || e.button !== 0) return;
        // Trigger as early as possible so option picks are not lost to
        // document-level pointerup handlers used by hand-drag interactions.
        e.preventDefault();
        onClick?.();
      }}
      onClick={(e) => {
        // Keep keyboard activation working while avoiding double-dispatch
        // after pointerdown-triggered selection.
        if (disabled || e.detail !== 0) return;
        onClick?.();
      }}
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
    >
      <SymbolText text={normalizeDecisionText(opt.description)} />
    </Button>
  );
}

function SubmitButton({ canAct, disabled, onClick, children }) {
  return (
    <Button
      variant="ghost"
      size="sm"
      className="decision-neon-button decision-submit-button group h-auto min-h-6 shrink-0 justify-start px-2 py-1 text-left text-[14px] font-bold uppercase whitespace-normal"
      disabled={!canAct || disabled}
      onClick={onClick}
    >
      <span className="inline-block transition-transform duration-200 group-hover:translate-x-0.5">
        {children}
      </span>
    </Button>
  );
}

function SectionHeader({ text }) {
  return (
    <h4 className="text-[12px] uppercase tracking-wider text-[#8ec4ff] font-bold px-1 py-0.5 m-0">
      {text}
    </h4>
  );
}

function Description({ decision, hideDescription = false, layout = "panel" }) {
  return (
    <DecisionSummary
      decision={decision}
      hideDescription={hideDescription}
      layout={layout}
    />
  );
}

function useExternalSubmitAction(onSubmitActionChange, action) {
  useEffect(() => {
    if (!onSubmitActionChange) return undefined;
    onSubmitActionChange(action || null);
    return () => onSubmitActionChange(null);
  }, [onSubmitActionChange, action]);
}

export default function SelectOptionsDecision({
  decision,
  canAct,
  inspectorOracleTextHeight = 0,
  inlineSubmit = true,
  onSubmitActionChange = null,
  hideDescription = false,
  layout = "panel",
}) {
  const reason = (decision.reason || "").toLowerCase();

  // Dispatch to sub-type based on decision metadata
  if (reason === "ordering") {
    return (
      <OrderingDecision
        decision={decision}
        canAct={canAct}
        inlineSubmit={inlineSubmit}
        onSubmitActionChange={onSubmitActionChange}
        hideDescription={hideDescription}
        layout={layout}
      />
    );
  }
  if (decision.distribute || reason.includes("distribut")) {
    return (
      <DistributeDecision
        decision={decision}
        canAct={canAct}
        inlineSubmit={inlineSubmit}
        onSubmitActionChange={onSubmitActionChange}
        hideDescription={hideDescription}
        layout={layout}
      />
    );
  }
  if (decision.counter_type || reason.includes("counter")) {
    return (
      <CountersDecision
        decision={decision}
        canAct={canAct}
        inlineSubmit={inlineSubmit}
        onSubmitActionChange={onSubmitActionChange}
        hideDescription={hideDescription}
        layout={layout}
      />
    );
  }
  const hasRepeatableOption = (decision.options || []).some((opt) => opt.repeatable);
  if (decision.repeatable || hasRepeatableOption) {
    return (
      <RepeatableDecision
        decision={decision}
        canAct={canAct}
        inlineSubmit={inlineSubmit}
        onSubmitActionChange={onSubmitActionChange}
        hideDescription={hideDescription}
        layout={layout}
      />
    );
  }

  const min = decision.min ?? 1;
  const max = decision.max ?? 1;

  if (min === 1 && max === 1) {
    return (
      <SingleSelectDecision
        decision={decision}
        canAct={canAct}
        onSubmitActionChange={onSubmitActionChange}
        hideDescription={hideDescription}
        layout={layout}
      />
    );
  }

  return (
    <MultiSelectDecision
      decision={decision}
      canAct={canAct}
      inspectorOracleTextHeight={inspectorOracleTextHeight}
      inlineSubmit={inlineSubmit}
      onSubmitActionChange={onSubmitActionChange}
      hideDescription={hideDescription}
      layout={layout}
    />
  );
}

function SingleSelectDecision({
  decision,
  canAct,
  onSubmitActionChange = null,
  hideDescription = false,
  layout = "panel",
}) {
  const { dispatch, state } = useGame();
  const { hoveredObjectId, hoverCard, clearHover } = useHover();
  const stripLayout = layout === "strip";
  const options = useMemo(() => decision.options || [], [decision.options]);
  const paymentDecision = useMemo(() => isPaymentDecision(decision), [decision]);
  const castFlowDecision = useMemo(() => isSpellCastFlowDecision(decision), [decision]);
  const payOption = useMemo(
    () => options.find((opt) => isPaymentOptionDescription(opt.description)) || null,
    [options]
  );
  const spellCastPaymentDecision = useMemo(() => {
    if (!paymentDecision) return false;
    const topStackObject = getVisibleTopStackObject(state);
    if (!topStackObject || topStackObject.ability_kind) return false;
    if (decision?.source_name && topStackObject.name && decision.source_name !== topStackObject.name) {
      return false;
    }
    return true;
  }, [paymentDecision, state, decision?.source_name]);
  const canSubmitPayment = canAct && !!payOption && payOption.legal !== false;
  const paymentProgressLabel = canSubmitPayment ? "Submit (1/1)" : "Submit (0/1)";
  const submitPayment = useCallback(() => {
    if (!payOption || payOption.legal === false) return;
    dispatch(
      { type: "select_options", option_indices: [payOption.index] },
      payOption.description || "Submit"
    );
  }, [dispatch, payOption]);
  const displayOptions = useMemo(
    () => (paymentDecision ? options.filter((opt) => !isPaymentOptionDescription(opt.description)) : options),
    [options, paymentDecision]
  );
  const legalDisplayOptions = useMemo(
    () => displayOptions.filter((opt) => opt.legal !== false),
    [displayOptions]
  );
  const singleLegalOption = useMemo(
    () => (legalDisplayOptions.length === 1 ? legalDisplayOptions[0] : null),
    [legalDisplayOptions]
  );
  const canSubmitSingle = canAct && !!singleLegalOption;
  const submitSingle = useCallback(() => {
    if (!singleLegalOption || singleLegalOption.legal === false) return;
    dispatch(
      { type: "select_options", option_indices: [singleLegalOption.index] },
      singleLegalOption.description || "Submit"
    );
  }, [dispatch, singleLegalOption]);
  const singleSubmitLabel = useMemo(() => {
    if (!singleLegalOption) return "Submit";
    const description = String(singleLegalOption.description || "").trim();
    if (isCastOptionDescription(description)) return "Cast";
    if (isPlayOptionDescription(description)) return "Play";
    return "Submit (1/1)";
  }, [singleLegalOption]);
  const contextual = useMemo(
    () => buildContextualOptions(displayOptions, hoveredObjectId),
    [displayOptions, hoveredObjectId]
  );
  const visibleOptions = useAnimatedRows(contextual.options, contextual.options.length > 0);
  const showHoverHint = contextual.waitingForHover && options.some((opt) => opt.object_id != null);
  const submitAction = useMemo(() => {
    if (paymentDecision) {
      return {
        label: (spellCastPaymentDecision || castFlowDecision) ? "Cast" : paymentProgressLabel,
        disabled: !canSubmitPayment,
        onSubmit: submitPayment,
      };
    }
    if (singleLegalOption) {
      return {
        label: singleSubmitLabel,
        disabled: !canSubmitSingle,
        onSubmit: submitSingle,
      };
    }
    return null;
  }, [
    paymentDecision,
    spellCastPaymentDecision,
    castFlowDecision,
    paymentProgressLabel,
    canSubmitPayment,
    submitPayment,
    singleLegalOption,
    singleSubmitLabel,
    canSubmitSingle,
    submitSingle,
  ]);
  useExternalSubmitAction(onSubmitActionChange, submitAction);

  return (
    <div className="flex w-full min-w-0 flex-col gap-1">
      <div className="transition-all duration-200">
        <div
          className={cn(
            stripLayout
              ? "px-1 py-0"
              : "sticky top-0 z-10 border-y border-[#2f4b67] bg-[rgba(13,24,36,0.96)] px-1.5 py-1"
          )}
        >
          {!paymentDecision && !hideDescription && (
            <Description decision={decision} hideDescription={hideDescription} layout={layout} />
          )}
          {showHoverHint && (
            <HoverHint text="Hover a related card to show its available choices." />
          )}
        </div>
        <div className={cn(
          "w-full",
          stripLayout ? "" : "border-b border-[#2f4b67] bg-[rgba(10,20,30,0.45)]"
        )}>
          <div className={cn(
            stripLayout
              ? "flex w-max min-w-full items-center gap-1.5 overflow-x-auto overflow-y-hidden py-0.5"
              : "w-full divide-y divide-[#2f4b67] max-h-[220px] overflow-y-auto"
          )}>
            {visibleOptions.map((opt) => {
              const objId = opt.object_id != null ? String(opt.object_id) : null;
              return (
                <OptionButton
                  key={opt.index}
                  opt={opt}
                  canAct={canAct}
                  isHighlighted={objId != null && hoveredObjectId === objId}
                  horizontal={stripLayout}
                  onClick={() =>
                    dispatch(
                      { type: "select_options", option_indices: [opt.index] },
                      opt.description
                    )
                  }
                  onMouseEnter={() => objId && hoverCard(objId)}
                  onMouseLeave={clearHover}
                />
              );
            })}
            {!showHoverHint && visibleOptions.length === 0 && (
              <div className={cn(
                "text-[12px] italic text-[#89a7c7]",
                stripLayout ? "px-2 py-1 whitespace-nowrap" : "px-2.5 py-2"
              )}>
                {paymentDecision ? "No additional payment actions." : "No legal choices."}
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

function MultiSelectDecision({
  decision,
  canAct,
  inspectorOracleTextHeight = 0,
  inlineSubmit = true,
  onSubmitActionChange = null,
  hideDescription = false,
  layout = "panel",
}) {
  const { dispatch } = useGame();
  const { hoveredObjectId, hoverCard, clearHover } = useHover();
  const stripLayout = layout === "strip";
  const rawOptions = useMemo(() => decision.options || [], [decision.options]);
  const paymentDecision = useMemo(() => isPaymentDecision(decision), [decision]);
  const options = useMemo(
    () => (paymentDecision ? rawOptions.filter((opt) => !isPaymentOptionDescription(opt.description)) : rawOptions),
    [rawOptions, paymentDecision]
  );
  const [selected, setSelected] = useState(new Set());
  const min = decision.min ?? 0;
  const max = decision.max ?? options.length;
  const optionsMaxHeight = useMemo(() => {
    const oracleHeight = Number(inspectorOracleTextHeight);
    if (!Number.isFinite(oracleHeight) || oracleHeight <= 0) return 360;
    const dynamicMax = Math.round(420 - (oracleHeight * 0.55));
    return Math.max(180, Math.min(360, dynamicMax));
  }, [inspectorOracleTextHeight]);
  const contextual = useMemo(
    () => buildContextualOptions(options, hoveredObjectId),
    [options, hoveredObjectId]
  );
  const visibleOptions = useAnimatedRows(contextual.options, contextual.options.length > 0);
  const visibleOptionIndexSet = useMemo(
    () => new Set(visibleOptions.map((opt) => opt.index)),
    [visibleOptions]
  );
  const hiddenSelectedCount = useMemo(
    () => Array.from(selected).filter((idx) => !visibleOptionIndexSet.has(idx)).length,
    [selected, visibleOptionIndexSet]
  );
  const showHoverHint = contextual.waitingForHover && options.some((opt) => opt.object_id != null);

  const toggle = (index) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(index)) next.delete(index);
      else if (next.size < max) next.add(index);
      return next;
    });
  };
  const canSubmit = canAct && selected.size >= min && selected.size <= max;
  const selectedIndices = useMemo(() => Array.from(selected), [selected]);
  const submitLabel = `Submit (${selected.size})`;
  const handleSubmit = useCallback(() => {
    dispatch(
      { type: "select_options", option_indices: selectedIndices },
      `Selected ${selectedIndices.length} option(s)`
    );
  }, [dispatch, selectedIndices]);
  const submitAction = useMemo(
    () => ({
      label: submitLabel,
      disabled: !canSubmit,
      onSubmit: handleSubmit,
    }),
    [submitLabel, canSubmit, handleSubmit]
  );
  useExternalSubmitAction(onSubmitActionChange, submitAction);

  return (
    <div className="flex w-full min-w-0 flex-col gap-1.5">
        <div className={cn(stripLayout ? "transition-all duration-200" : "-mx-1.5 transition-all duration-200")}>
          <div className={cn(
            stripLayout
              ? "px-1 py-0"
              : "sticky top-0 z-10 border-y border-[#2f4b67] bg-[rgba(13,24,36,0.96)] px-1.5 py-1"
          )}>
          {!paymentDecision && !hideDescription && (
            <Description decision={decision} hideDescription={hideDescription} layout={layout} />
          )}
          <SectionHeader text={`Select ${min === max ? min : `${min}–${max}`}`} />
          {showHoverHint && (
            <HoverHint text="Hover a related card to show its choices. You can keep previous selections." />
          )}
        </div>
        <div
          className={cn(
            "w-full transition-[max-height] duration-300 ease-out",
            stripLayout ? "overflow-x-auto overflow-y-hidden" : "overflow-y-auto overflow-x-hidden"
          )}
          style={stripLayout ? undefined : { maxHeight: `${optionsMaxHeight}px` }}
        >
          <div className={cn(
            stripLayout
              ? "flex w-max min-w-full items-center gap-1.5 py-0.5"
              : "w-full divide-y divide-[#2f4b67]"
          )}>
            {visibleOptions.map((opt) => {
              const objId = opt.object_id != null ? String(opt.object_id) : null;
              const isHighlighted = objId != null && hoveredObjectId === objId;
              const isSelected = selected.has(opt.index);
              return (
                <OptionButton
                  key={opt.index}
                  opt={opt}
                  canAct={canAct}
                  isHighlighted={isHighlighted}
                  isSelected={isSelected}
                  horizontal={stripLayout}
                  onClick={() => opt.legal !== false && toggle(opt.index)}
                  onMouseEnter={() => objId && hoverCard(objId)}
                  onMouseLeave={clearHover}
                />
              );
            })}
            {hiddenSelectedCount > 0 && (
              <div className={cn(
                "text-[12px] text-[#89a7c7]",
                stripLayout ? "px-2 py-1 whitespace-nowrap" : "px-2.5 py-1"
              )}>
                {hiddenSelectedCount} selected option(s) from other cards.
              </div>
            )}
            {!showHoverHint && visibleOptions.length === 0 && (
              <div className={cn(
                "text-[12px] italic text-[#89a7c7]",
                stripLayout ? "px-2 py-1 whitespace-nowrap" : "px-2.5 py-2"
              )}>
                No legal choices.
              </div>
            )}
          </div>
        </div>
      </div>
      {inlineSubmit && (
        <div className={cn("w-full shrink-0", stripLayout ? "pt-0" : "pt-1")}>
          <Button
            variant="ghost"
            size="sm"
            className={cn(
              "decision-neon-button decision-submit-button h-6 rounded-sm px-2 text-[13px] font-semibold uppercase",
              stripLayout ? "w-auto ml-1" : "w-full"
            )}
            disabled={!canSubmit}
            onClick={handleSubmit}
          >
            {submitLabel}
          </Button>
        </div>
      )}
    </div>
  );
}

function OrderingDecision({
  decision,
  canAct,
  inlineSubmit = true,
  onSubmitActionChange = null,
  hideDescription = false,
  layout = "panel",
}) {
  const { dispatch, triggerOrderingState, moveTriggerOrderingItem } = useGame();
  const stripLayout = layout === "strip";
  const options = decision.options || [];
  const trivialOrdering = options.length <= 1;
  const triggerOrdering = isTriggerOrderingDecision(decision);
  const triggerOrderingKey = buildTriggerOrderingKey(decision);
  const localOrderingKey = useMemo(
    () => `${decision.description || ""}|${optionsSignature(decision.options || [])}`,
    [decision.description, decision.options]
  );
  const [localOrderState, setLocalOrderState] = useState(
    () => ({
      key: localOrderingKey,
      order: defaultTriggerOrderingOrder(decision),
    })
  );
  const order = useMemo(() => {
    if (!triggerOrdering) {
      if (localOrderState.key === localOrderingKey) {
        return normalizeTriggerOrderingOrder(localOrderState.order, decision);
      }
      return defaultTriggerOrderingOrder(decision);
    }
    if (triggerOrderingState?.key === triggerOrderingKey) {
      return normalizeTriggerOrderingOrder(triggerOrderingState.order, decision);
    }
    return defaultTriggerOrderingOrder(decision);
  }, [
    decision,
    localOrderState,
    localOrderingKey,
    triggerOrdering,
    triggerOrderingKey,
    triggerOrderingState,
  ]);

  const move = (position, direction) => {
    const newPos = position + direction;
    if (newPos < 0 || newPos >= order.length) return;
    if (triggerOrdering) {
      moveTriggerOrderingItem(position, direction);
      return;
    }
    setLocalOrderState((current) => {
      const next = current.key === localOrderingKey
        ? normalizeTriggerOrderingOrder(current.order, decision)
        : defaultTriggerOrderingOrder(decision);
      [next[position], next[newPos]] = [next[newPos], next[position]];
      return {
        key: localOrderingKey,
        order: next,
      };
    });
  };
  const handleSubmit = useCallback(() => {
    dispatch({ type: "select_options", option_indices: order.slice() }, "Order submitted");
  }, [dispatch, order]);
  const submitAction = useMemo(
    () => (trivialOrdering
      ? null
      : {
          label: "Submit Order",
          disabled: !canAct,
          onSubmit: handleSubmit,
        }),
    [canAct, handleSubmit, trivialOrdering]
  );
  useExternalSubmitAction(onSubmitActionChange, submitAction);

  const standardRows = (
    <div className={cn(
      stripLayout ? "flex items-stretch gap-1.5 px-1 py-1" : "flex flex-col gap-0.5"
    )}>
      {order.map((optIndex, pos) => {
        const opt = options.find((o) => o.index === optIndex);
        if (!opt) return null;
        return (
          <div key={optIndex} className={cn(
            "flex items-center gap-1.5 text-[13px] py-1 px-2 rounded-sm text-muted-foreground transition-all hover:text-foreground hover:bg-[rgba(100,169,255,0.06)]",
            stripLayout && "min-w-[220px] max-w-[360px] self-stretch rounded-none border-0 border-l-2 border-l-[rgba(116,139,164,0.42)] bg-[rgba(12,22,34,0.58)] text-[rgba(206,223,242,0.78)] hover:border-l-[rgba(236,245,255,0.92)] hover:bg-[rgba(220,236,255,0.16)] hover:text-[#f4f9ff]"
          )}>
            <span className="text-[11px] text-[#8ec4ff] font-bold w-4 text-center shrink-0">{pos + 1}</span>
            <span className="min-w-0 flex-1">
              <SymbolText text={normalizeDecisionText(opt.description)} />
            </span>
            <Button
              variant="ghost"
              size="sm"
              className="h-5 w-5 p-0 text-[13px]"
              disabled={!canAct || pos === 0}
              onClick={() => move(pos, -1)}
            >
              <ChevronUp className="size-3.5" />
            </Button>
            <Button
              variant="ghost"
              size="sm"
              className="h-5 w-5 p-0 text-[13px]"
              disabled={!canAct || pos === order.length - 1}
              onClick={() => move(pos, 1)}
            >
              <ChevronDown className="size-3.5" />
            </Button>
          </div>
        );
      })}
    </div>
  );

  const triggerOrderingHint = (
    <div className={cn(
      "rounded-sm border border-[#31506e]/80 bg-[rgba(10,20,31,0.72)] text-[#c9dff7]",
      stripLayout ? "min-w-[280px] px-3 py-2" : "px-3 py-2.5"
    )}>
      <div className="text-[12px] font-bold uppercase tracking-[0.14em] text-[#8ec4ff]">
        Order In Stack
      </div>
      <div className="mt-1 text-[13px] leading-snug text-[#dbeaff]">
        Use the arrows on the stack cards to arrange these triggers. The leftmost arrow moves a trigger closer to the top of the stack.
      </div>
    </div>
  );

  if (triggerOrdering && stripLayout) {
    return null;
  }

  if (trivialOrdering && stripLayout) {
    return null;
  }

  return (
    <div className={cn("flex h-full min-h-0 flex-col gap-1", stripLayout && "min-w-0")}>
      {stripLayout ? (
        <div className="min-w-0 overflow-x-auto overflow-y-hidden">
          <div className="flex w-max min-w-full items-center gap-1.5">
            {!hideDescription && (
              <div className="shrink-0 px-1">
                <Description decision={decision} hideDescription={hideDescription} layout={layout} />
              </div>
            )}
            <SectionHeader text={triggerOrdering ? "Stack Order" : "Order"} />
            {triggerOrdering ? triggerOrderingHint : standardRows}
          </div>
        </div>
      ) : (
        <ScrollArea className="flex-1 min-h-0">
          <div className="flex flex-col gap-1 pr-1">
            {!hideDescription && (
              <Description decision={decision} hideDescription={hideDescription} layout={layout} />
            )}
            <SectionHeader text={triggerOrdering ? "Stack Order" : "Order"} />
            {triggerOrdering ? triggerOrderingHint : standardRows}
          </div>
        </ScrollArea>
      )}
      {inlineSubmit && !trivialOrdering && (
        <div className={cn(
          "shrink-0",
          stripLayout ? "pt-0" : "border-t border-game-line-2/70 pt-1"
        )}>
          <SubmitButton
            canAct={canAct}
            onClick={handleSubmit}
          >
            Submit Order
          </SubmitButton>
        </div>
      )}
    </div>
  );
}

function DistributeDecision({
  decision,
  canAct,
  inlineSubmit = true,
  onSubmitActionChange = null,
  hideDescription = false,
  layout = "panel",
}) {
  const { dispatch, setStatus } = useGame();
  const stripLayout = layout === "strip";
  const options = decision.options || [];
  const total = Number(decision.max || 0);
  const [counts, setCounts] = useState(() =>
    Object.fromEntries(options.map((opt) => [opt.index, 0]))
  );

  const assigned = Object.values(counts).reduce((a, b) => a + b, 0);

  const expandOptionCounts = (countsByIndex) => {
    const expanded = [];
    Object.entries(countsByIndex).forEach(([idx, count]) => {
      for (let i = 0; i < Math.max(0, Math.floor(Number(count) || 0)); i++) {
        expanded.push(Number(idx));
      }
    });
    return expanded;
  };
  const canSubmit = canAct && assigned === total;
  const submitLabel = `Submit (${assigned}/${total})`;
  const handleSubmit = useCallback(() => {
    if (assigned !== total) {
      setStatus(`Must assign exactly ${total} (currently ${assigned})`, true);
      return;
    }
    dispatch(
      { type: "select_options", option_indices: expandOptionCounts(counts) },
      "Distribution submitted"
    );
  }, [assigned, total, setStatus, dispatch, counts]);
  const submitAction = useMemo(
    () => ({
      label: submitLabel,
      disabled: !canSubmit,
      onSubmit: handleSubmit,
    }),
    [submitLabel, canSubmit, handleSubmit]
  );
  useExternalSubmitAction(onSubmitActionChange, submitAction);

  const rows = (
    <div className={cn(
      stripLayout ? "flex items-stretch gap-1.5 px-1 py-1" : "flex flex-col gap-0.5"
    )}>
      {options.map((opt) => (
        <label key={opt.index} className={cn(
          "flex items-center gap-2 text-[13px] py-1 px-2 rounded-sm text-muted-foreground transition-all hover:text-foreground hover:bg-[rgba(100,169,255,0.06)]",
          stripLayout && "min-w-[220px] max-w-[360px] self-stretch rounded-none border-0 border-l-2 border-l-[rgba(116,139,164,0.42)] bg-[rgba(12,22,34,0.58)] text-[rgba(206,223,242,0.78)] hover:border-l-[rgba(236,245,255,0.92)] hover:bg-[rgba(220,236,255,0.16)] hover:text-[#f4f9ff]"
        )}>
          <span className="flex-1 min-w-0"><SymbolText text={normalizeDecisionText(opt.description)} /></span>
          <Input
            type="number"
            className="h-6 w-16 text-[13px] bg-transparent text-center"
            min={0}
            max={Number(opt.max_count ?? total)}
            value={counts[opt.index] || 0}
            onChange={(e) =>
              setCounts((prev) => ({ ...prev, [opt.index]: Number(e.target.value) || 0 }))
            }
            disabled={!canAct || opt.legal === false}
          />
        </label>
      ))}
    </div>
  );

  return (
    <div className={cn("flex h-full min-h-0 flex-col gap-1", stripLayout && "min-w-0")}>
      {stripLayout ? (
        <div className="min-w-0 overflow-x-auto overflow-y-hidden">
          <div className="flex w-max min-w-full items-center gap-1.5">
            {!hideDescription && (
              <div className="shrink-0 px-1">
                <Description decision={decision} hideDescription={hideDescription} layout={layout} />
              </div>
            )}
            <SectionHeader text={`Distribute ${total} total`} />
            {rows}
          </div>
        </div>
      ) : (
        <ScrollArea className="flex-1 min-h-0">
          <div className="flex flex-col gap-1 pr-1">
            {!hideDescription && (
              <Description decision={decision} hideDescription={hideDescription} layout={layout} />
            )}
            <SectionHeader text={`Distribute ${total} total`} />
            {rows}
          </div>
        </ScrollArea>
      )}
      {inlineSubmit && (
        <div className={cn(
          "shrink-0",
          stripLayout ? "pt-0" : "border-t border-game-line-2/70 pt-1"
        )}>
          <SubmitButton
            canAct={canAct}
            disabled={assigned !== total}
            onClick={handleSubmit}
          >
            {submitLabel}
          </SubmitButton>
        </div>
      )}
    </div>
  );
}

function CountersDecision({
  decision,
  canAct,
  inlineSubmit = true,
  onSubmitActionChange = null,
  hideDescription = false,
  layout = "panel",
}) {
  const { dispatch } = useGame();
  const stripLayout = layout === "strip";
  const options = decision.options || [];
  const maxTotal = Number(decision.max || 0);
  const [counts, setCounts] = useState(() =>
    Object.fromEntries(options.map((opt) => [opt.index, 0]))
  );

  const total = Object.values(counts).reduce((a, b) => a + b, 0);

  const expandOptionCounts = (countsByIndex) => {
    const expanded = [];
    Object.entries(countsByIndex).forEach(([idx, count]) => {
      for (let i = 0; i < Math.max(0, Math.floor(Number(count) || 0)); i++) {
        expanded.push(Number(idx));
      }
    });
    return expanded;
  };
  const canSubmit = canAct && total <= maxTotal;
  const submitLabel = `Submit Counters (${total}/${maxTotal})`;
  const handleSubmit = useCallback(() => {
    dispatch(
      { type: "select_options", option_indices: expandOptionCounts(counts) },
      "Counter choice submitted"
    );
  }, [dispatch, counts]);
  const submitAction = useMemo(
    () => ({
      label: submitLabel,
      disabled: !canSubmit,
      onSubmit: handleSubmit,
    }),
    [submitLabel, canSubmit, handleSubmit]
  );
  useExternalSubmitAction(onSubmitActionChange, submitAction);

  const rows = (
    <div className={cn(
      stripLayout ? "flex items-stretch gap-1.5 px-1 py-1" : "flex flex-col gap-0.5"
    )}>
      {options.map((opt) => (
        <label key={opt.index} className={cn(
          "flex items-center gap-2 text-[13px] py-1 px-2 rounded-sm text-muted-foreground transition-all hover:text-foreground hover:bg-[rgba(100,169,255,0.06)]",
          stripLayout && "min-w-[220px] max-w-[360px] self-stretch rounded-none border-0 border-l-2 border-l-[rgba(116,139,164,0.42)] bg-[rgba(12,22,34,0.58)] text-[rgba(206,223,242,0.78)] hover:border-l-[rgba(236,245,255,0.92)] hover:bg-[rgba(220,236,255,0.16)] hover:text-[#f4f9ff]"
        )}>
          <span className="flex-1 min-w-0"><SymbolText text={normalizeDecisionText(opt.description)} /></span>
          <Input
            type="number"
            className="h-6 w-16 text-[13px] bg-transparent text-center"
            min={0}
            max={Number(opt.max_count ?? maxTotal)}
            value={counts[opt.index] || 0}
            onChange={(e) =>
              setCounts((prev) => ({ ...prev, [opt.index]: Number(e.target.value) || 0 }))
            }
            disabled={!canAct || opt.legal === false}
          />
        </label>
      ))}
    </div>
  );

  return (
    <div className={cn("flex h-full min-h-0 flex-col gap-1", stripLayout && "min-w-0")}>
      {stripLayout ? (
        <div className="min-w-0 overflow-x-auto overflow-y-hidden">
          <div className="flex w-max min-w-full items-center gap-1.5">
            {!hideDescription && (
              <div className="shrink-0 px-1">
                <Description decision={decision} hideDescription={hideDescription} layout={layout} />
              </div>
            )}
            <SectionHeader text="Counters" />
            {rows}
          </div>
        </div>
      ) : (
        <ScrollArea className="flex-1 min-h-0">
          <div className="flex flex-col gap-1 pr-1">
            {!hideDescription && (
              <Description decision={decision} hideDescription={hideDescription} layout={layout} />
            )}
            <SectionHeader text="Counters" />
            {rows}
          </div>
        </ScrollArea>
      )}
      {inlineSubmit && (
        <div className={cn(
          "shrink-0",
          stripLayout ? "pt-0" : "border-t border-game-line-2/70 pt-1"
        )}>
          <SubmitButton
            canAct={canAct}
            disabled={total > maxTotal}
            onClick={handleSubmit}
          >
            {submitLabel}
          </SubmitButton>
        </div>
      )}
    </div>
  );
}

function RepeatableDecision({
  decision,
  canAct,
  inlineSubmit = true,
  onSubmitActionChange = null,
  hideDescription = false,
  layout = "panel",
}) {
  const { dispatch } = useGame();
  const stripLayout = layout === "strip";
  const options = decision.options || [];
  const maxTotal = Number(decision.max || 0);
  const [counts, setCounts] = useState(() =>
    Object.fromEntries(options.map((opt) => [opt.index, 0]))
  );

  const total = Object.values(counts).reduce((a, b) => a + b, 0);

  const expandOptionCounts = (countsByIndex) => {
    const expanded = [];
    Object.entries(countsByIndex).forEach(([idx, count]) => {
      for (let i = 0; i < Math.max(0, Math.floor(Number(count) || 0)); i++) {
        expanded.push(Number(idx));
      }
    });
    return expanded;
  };
  const min = decision.min || 0;
  const canSubmit = canAct && total >= min && total <= maxTotal;
  const submitLabel = `Submit (${total})`;
  const handleSubmit = useCallback(() => {
    dispatch(
      { type: "select_options", option_indices: expandOptionCounts(counts) },
      `Selected ${total} option(s)`
    );
  }, [dispatch, counts, total]);
  const submitAction = useMemo(
    () => ({
      label: submitLabel,
      disabled: !canSubmit,
      onSubmit: handleSubmit,
    }),
    [submitLabel, canSubmit, handleSubmit]
  );
  useExternalSubmitAction(onSubmitActionChange, submitAction);

  const rows = (
    <div className={cn(
      stripLayout ? "flex items-stretch gap-1.5 px-1 py-1" : "flex flex-col gap-0.5"
    )}>
      {options.map((opt) => (
        <label key={opt.index} className={cn(
          "flex items-center gap-2 text-[13px] py-1 px-2 rounded-sm text-muted-foreground transition-all hover:text-foreground hover:bg-[rgba(100,169,255,0.06)]",
          stripLayout && "min-w-[220px] max-w-[360px] self-stretch rounded-none border-0 border-l-2 border-l-[rgba(116,139,164,0.42)] bg-[rgba(12,22,34,0.58)] text-[rgba(206,223,242,0.78)] hover:border-l-[rgba(236,245,255,0.92)] hover:bg-[rgba(220,236,255,0.16)] hover:text-[#f4f9ff]"
        )}>
          <span className="flex-1 min-w-0"><SymbolText text={normalizeDecisionText(opt.description)} /></span>
          <Input
            type="number"
            className="h-6 w-16 text-[13px] bg-transparent text-center"
            min={0}
            max={Number(opt.max_count ?? maxTotal)}
            value={counts[opt.index] || 0}
            onChange={(e) =>
              setCounts((prev) => ({ ...prev, [opt.index]: Number(e.target.value) || 0 }))
            }
            disabled={!canAct || opt.legal === false}
          />
        </label>
      ))}
    </div>
  );

  return (
    <div className={cn("flex h-full min-h-0 flex-col gap-1", stripLayout && "min-w-0")}>
      {stripLayout ? (
        <div className="min-w-0 overflow-x-auto overflow-y-hidden">
          <div className="flex w-max min-w-full items-center gap-1.5">
            {!hideDescription && (
              <div className="shrink-0 px-1">
                <Description decision={decision} hideDescription={hideDescription} layout={layout} />
              </div>
            )}
            <SectionHeader text="Repeat" />
            {rows}
          </div>
        </div>
      ) : (
        <ScrollArea className="flex-1 min-h-0">
          <div className="flex flex-col gap-1 pr-1">
            {!hideDescription && (
              <Description decision={decision} hideDescription={hideDescription} layout={layout} />
            )}
            <SectionHeader text="Repeat" />
            {rows}
          </div>
        </ScrollArea>
      )}
      {inlineSubmit && (
        <div className={cn(
          "shrink-0",
          stripLayout ? "pt-0" : "border-t border-game-line-2/70 pt-1"
        )}>
          <SubmitButton
            canAct={canAct}
            disabled={total < min || total > maxTotal}
            onClick={handleSubmit}
          >
            {submitLabel}
          </SubmitButton>
        </div>
      )}
    </div>
  );
}
