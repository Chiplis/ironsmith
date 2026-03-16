import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useGame } from "@/context/GameContext";
import { useHover } from "@/context/HoverContext";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import DecisionSummary from "./DecisionSummary";
import HighlightedDecisionText from "./HighlightedDecisionText";
import { getPlayerAccent } from "@/lib/player-colors";
import { buildObjectControllerById } from "@/lib/decision-object-meta";

const STRIP_ITEM_BASE_CLASS = "decision-option-row decision-option-row--strip h-8 max-w-[360px] min-w-[120px] shrink-0 justify-start self-stretch px-2.5 text-[12px] font-semibold";
const STRIP_ITEM_ACTIVE_CLASS = "is-selected";
const STRIP_ITEM_DISABLED_CLASS = "is-disabled";

export default function SelectObjectsDecision({
  decision,
  canAct,
  inspectorOracleTextHeight = 0,
  inlineSubmit = true,
  onSubmitActionChange = null,
  hideDescription = false,
  layout = "panel",
}) {
  const { dispatch, state } = useGame();
  const { hoveredObjectId, hoverCard, clearHover } = useHover();
  const stripLayout = layout === "strip";
  const candidates = useMemo(() => decision.candidates || [], [decision.candidates]);
  const [selected, setSelected] = useState(new Set());
  const min = decision.min ?? 0;
  const max = decision.max ?? candidates.length;
  const hideTimerRef = useRef(null);
  const optionsMaxHeight = useMemo(() => {
    const oracleHeight = Number(inspectorOracleTextHeight);
    if (!Number.isFinite(oracleHeight) || oracleHeight <= 0) return 360;
    const dynamicMax = Math.round(420 - (oracleHeight * 0.55));
    return Math.max(180, Math.min(360, dynamicMax));
  }, [inspectorOracleTextHeight]);
  const objectControllerById = useMemo(
    () => buildObjectControllerById(state),
    [state]
  );

  const scopedCandidates = useMemo(() => {
    if (stripLayout) return candidates;
    if (hoveredObjectId == null) return candidates;
    const hoveredStr = String(hoveredObjectId);
    const hasHoveredCandidate = candidates.some((c) => String(c.id) === hoveredStr);
    if (!hasHoveredCandidate) return candidates;
    return candidates.filter(
      (c) => String(c.id) === hoveredStr || selected.has(c.id)
    );
  }, [candidates, hoveredObjectId, selected, stripLayout]);
  const showRows = scopedCandidates.length > 0;
  const [visibleCandidates, setVisibleCandidates] = useState(scopedCandidates);
  const focusedToHover = hoveredObjectId != null
    && candidates.some((c) => String(c.id) === String(hoveredObjectId));
  const showHeader = !stripLayout;

  const toggleObject = useCallback((id) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else if (next.size < max) {
        next.add(id);
      }
      return next;
    });
  }, [max]);

  useEffect(() => {
    const onExternalObjectChoice = (event) => {
      if (!canAct) return;
      const externalObjectId = event?.detail?.objectId;
      if (externalObjectId == null) return;
      const matchedCandidate = candidates.find(
        (candidate) => String(candidate?.id) === String(externalObjectId)
      );
      if (!matchedCandidate || matchedCandidate.legal === false) return;
      toggleObject(matchedCandidate.id);
    };

    window.addEventListener("ironsmith:select-object-choice", onExternalObjectChoice);
    return () => {
      window.removeEventListener("ironsmith:select-object-choice", onExternalObjectChoice);
    };
  }, [canAct, candidates, toggleObject]);

  const canSubmit = selected.size >= min && selected.size <= max;
  const selectedIds = useMemo(() => Array.from(selected), [selected]);
  const submitLabel = `Submit (${selected.size}/${min === max ? min : `${min}-${max}`})`;
  const handleSubmit = useCallback(() => {
    dispatch(
      { type: "select_objects", object_ids: selectedIds },
      `Selected ${selectedIds.length} object(s)`
    );
  }, [dispatch, selectedIds]);

  useEffect(() => {
    if (!onSubmitActionChange) return undefined;
    onSubmitActionChange({
      label: submitLabel,
      disabled: !canAct || !canSubmit,
      onSubmit: handleSubmit,
    });
    return () => onSubmitActionChange(null);
  }, [onSubmitActionChange, submitLabel, canAct, canSubmit, handleSubmit]);

  useEffect(() => {
    if (hideTimerRef.current) {
      clearTimeout(hideTimerRef.current);
      hideTimerRef.current = null;
    }
    hideTimerRef.current = setTimeout(() => {
      setVisibleCandidates(showRows ? scopedCandidates : []);
      hideTimerRef.current = null;
    }, showRows ? 0 : 180);
  }, [scopedCandidates, showRows]);

  useEffect(
    () => () => {
      if (hideTimerRef.current) {
        clearTimeout(hideTimerRef.current);
        hideTimerRef.current = null;
      }
    },
    []
  );

  return (
    <div className="flex w-full min-w-0 flex-col gap-1.5">
      <div
        className={cn(
          stripLayout ? "transition-all duration-200" : "-mx-1.5 transition-all duration-200",
          showRows ? "opacity-100 translate-y-0" : "opacity-0 -translate-y-1 pointer-events-none"
        )}
      >
        {showHeader && (
          <div
            className={cn(
              stripLayout
                ? "decision-strip-header px-1.5 py-1"
                : "decision-panel-header sticky top-0 z-10 px-1.5 py-1"
            )}
          >
            <DecisionSummary
              decision={decision}
              hideDescription={hideDescription}
              layout={layout}
            />
            {!stripLayout && (
              <div className="decision-helper-text text-[13px] leading-snug">
                Select {min === max ? min : `${min}-${max}`} object(s)
              </div>
            )}
            {!stripLayout && focusedToHover && (
              <div className="decision-helper-text decision-helper-text--muted text-[12px] italic leading-snug">
                Showing options for the hovered card.
              </div>
            )}
          </div>
        )}
        <div
          className={cn(
            "w-full transition-[max-height] duration-300 ease-out",
            stripLayout ? "overflow-x-auto overflow-y-hidden pb-1" : "overflow-y-auto overflow-x-hidden"
          )}
          style={stripLayout ? undefined : { maxHeight: `${optionsMaxHeight}px` }}
        >
          <div className={cn(
            stripLayout
              ? "flex w-max min-w-full flex-nowrap items-center gap-1.5 py-0.5 pr-1"
              : "w-full divide-y divide-[rgba(128,107,78,0.28)]"
          )}>
            {visibleCandidates.map((c) => {
              const isSelected = selected.has(c.id);
              const isUnavailable = !isSelected && selected.size >= max;
              const isDisabled = !canAct || !c.legal || isUnavailable;
              const controllerId = objectControllerById.get(String(c.id));
              const accent = controllerId == null || Number(controllerId) === Number(state?.perspective)
                ? null
                : getPlayerAccent(state?.players || [], controllerId);
              return (
                <Button
                  key={c.id}
                  variant="ghost"
                  size="sm"
                  className={cn(
                    stripLayout
                      ? STRIP_ITEM_BASE_CLASS
                      : "decision-option-row decision-option-row--panel h-8 w-full min-w-0 justify-start px-2.5 text-[13px]",
                    stripLayout && isSelected && STRIP_ITEM_ACTIVE_CLASS,
                    !stripLayout && isSelected && "is-selected",
                    isDisabled
                      && (stripLayout
                        ? STRIP_ITEM_DISABLED_CLASS
                        : "is-disabled")
                  )}
                  disabled={isDisabled}
                  onPointerDown={(event) => {
                    if (isDisabled || event.button !== 0) return;
                    event.preventDefault();
                    toggleObject(c.id);
                  }}
                  onClick={(event) => {
                    if (isDisabled || event.detail !== 0) return;
                    toggleObject(c.id);
                  }}
                  onMouseEnter={() => hoverCard(c.id)}
                  onMouseLeave={clearHover}
                >
                  <HighlightedDecisionText
                    className="decision-option-label"
                    text={c.name}
                    highlightText={c.name}
                    highlightColor={accent?.hex || null}
                  />
                </Button>
              );
            })}
            {visibleCandidates.length === 0 && (
              <div className={cn(
                "decision-empty-note text-[12px] italic",
                stripLayout ? "px-2 py-1" : "px-2.5 py-2"
              )}>
                No legal choices.
              </div>
            )}
          </div>
        </div>
      </div>
      {inlineSubmit && (
        <div className="w-full shrink-0 pt-1">
          <Button
            variant="ghost"
            size="sm"
            className={cn(
              "decision-neon-button decision-submit-button h-6 rounded-none px-2 text-[13px] font-semibold uppercase",
              stripLayout ? "w-auto ml-1" : "w-full"
            )}
            disabled={!canAct || !canSubmit}
            onPointerDown={(event) => {
              if (!canAct || !canSubmit || event.button !== 0) return;
              event.preventDefault();
              handleSubmit();
            }}
            onClick={(event) => {
              if (!canAct || !canSubmit || event.detail !== 0) return;
              handleSubmit();
            }}
          >
            {submitLabel}
          </Button>
        </div>
      )}
    </div>
  );
}
