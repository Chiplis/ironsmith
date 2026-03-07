import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useGame } from "@/context/GameContext";
import { useHoveredObjectId } from "@/context/HoverContext";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { normalizeDecisionText } from "./decisionText";

const STRIP_ITEM_BASE_CLASS = "h-8 max-w-[360px] min-w-[120px] justify-start self-stretch rounded-none border-0 border-l-2 border-l-[rgba(116,139,164,0.42)] bg-[rgba(12,22,34,0.58)] px-2.5 text-[12px] font-semibold text-[rgba(206,223,242,0.52)] transition-all hover:border-l-[rgba(236,245,255,0.92)] hover:bg-[rgba(220,236,255,0.16)] hover:text-[#f4f9ff] hover:shadow-[0_0_12px_rgba(236,245,255,0.3)]";
const STRIP_ITEM_ACTIVE_CLASS = "border-l-[rgba(236,245,255,0.9)] bg-[rgba(220,236,255,0.16)] text-[#f4f9ff] shadow-[0_0_12px_rgba(236,245,255,0.3)]";
const STRIP_ITEM_DISABLED_CLASS = "border-l-[rgba(63,79,98,0.6)] bg-[rgba(8,15,23,0.76)] text-[#5f7590] hover:border-l-[rgba(63,79,98,0.6)] hover:bg-[rgba(8,15,23,0.76)] hover:text-[#5f7590] hover:shadow-none";

export default function SelectObjectsDecision({
  decision,
  canAct,
  inspectorOracleTextHeight = 0,
  inlineSubmit = true,
  onSubmitActionChange = null,
  hideDescription = false,
  layout = "panel",
}) {
  const { dispatch } = useGame();
  const hoveredObjectId = useHoveredObjectId();
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

  const scopedCandidates = useMemo(() => {
    if (hoveredObjectId == null) return candidates;
    const hoveredStr = String(hoveredObjectId);
    const hasHoveredCandidate = candidates.some((c) => String(c.id) === hoveredStr);
    if (!hasHoveredCandidate) return candidates;
    return candidates.filter(
      (c) => String(c.id) === hoveredStr || selected.has(c.id)
    );
  }, [candidates, hoveredObjectId, selected]);
  const showRows = scopedCandidates.length > 0;
  const [visibleCandidates, setVisibleCandidates] = useState(scopedCandidates);
  const focusedToHover = hoveredObjectId != null
    && candidates.some((c) => String(c.id) === String(hoveredObjectId));

  const toggleObject = (id) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else if (next.size < max) {
        next.add(id);
      }
      return next;
    });
  };

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
        <div
          className={cn(
            stripLayout
              ? "px-1 py-0"
              : "sticky top-0 z-10 border-y border-[#2f4b67] bg-[rgba(13,24,36,0.96)] px-1.5 py-1"
          )}
        >
          {!hideDescription && decision.description && (
            <div className="text-[14px] text-[#b6cae1] leading-snug">
              {normalizeDecisionText(decision.description)}
            </div>
          )}
          <div className="text-[13px] text-[#8ba4c1] leading-snug">
            Select {min === max ? min : `${min}-${max}`} object(s)
          </div>
          {focusedToHover && (
            <div className="text-[12px] italic text-[#89a7c7] leading-snug">
              Showing options for the hovered card.
            </div>
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
            {visibleCandidates.map((c) => {
              const isSelected = selected.has(c.id);
              const isUnavailable = !isSelected && selected.size >= max;
              const isDisabled = !canAct || !c.legal || isUnavailable;
              return (
                <Button
                  key={c.id}
                  variant="ghost"
                  size="sm"
                  className={cn(
                    stripLayout
                      ? STRIP_ITEM_BASE_CLASS
                      : "h-8 w-full justify-start rounded-none border-0 bg-[rgba(15,27,40,0.9)] px-2.5 text-[13px] text-[#c7dbf2] transition-all hover:bg-[rgba(25,44,66,0.95)] hover:text-[#eaf3ff]",
                    stripLayout && isSelected && STRIP_ITEM_ACTIVE_CLASS,
                    !stripLayout && isSelected && "bg-[rgba(36,58,84,0.72)] text-[#eaf4ff]",
                    isDisabled
                      && (stripLayout
                        ? STRIP_ITEM_DISABLED_CLASS
                        : "bg-[rgba(12,20,30,0.72)] text-[#647f99] hover:bg-[rgba(12,20,30,0.72)] hover:text-[#647f99]")
                  )}
                  disabled={isDisabled}
                  onClick={() => toggleObject(c.id)}
                >
                  {c.name}
                </Button>
              );
            })}
            {visibleCandidates.length === 0 && (
              <div className={cn(
                "text-[12px] italic text-[#89a7c7]",
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
              "decision-neon-button decision-submit-button h-6 rounded-sm px-2 text-[13px] font-semibold uppercase",
              stripLayout ? "w-auto ml-1" : "w-full"
            )}
            disabled={!canAct || !canSubmit}
            onClick={handleSubmit}
          >
            {submitLabel}
          </Button>
        </div>
      )}
    </div>
  );
}
