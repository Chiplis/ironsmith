import { useEffect, useMemo, useRef, useState } from "react";
import { useGame } from "@/context/GameContext";
import { useHoveredObjectId } from "@/context/HoverContext";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { normalizeDecisionText } from "./decisionText";

export default function SelectObjectsDecision({
  decision,
  canAct,
  inspectorOracleTextHeight = 0,
}) {
  const { dispatch } = useGame();
  const hoveredObjectId = useHoveredObjectId();
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
    <div className="flex w-full flex-col gap-1.5">
      <div
        className={cn(
          "-mx-1.5 transition-all duration-200",
          showRows ? "opacity-100 translate-y-0" : "opacity-0 -translate-y-1 pointer-events-none"
        )}
      >
        <div className="sticky top-0 z-10 border-y border-[#2f4b67] bg-[rgba(13,24,36,0.96)] px-1.5 py-1">
          {decision.description && (
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
          className="w-full overflow-y-auto overflow-x-hidden border-b border-[#2f4b67] bg-[rgba(10,20,30,0.45)] transition-[max-height] duration-300 ease-out"
          style={{ maxHeight: `${optionsMaxHeight}px` }}
        >
          <div className="w-full divide-y divide-[#2f4b67]">
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
                    "h-8 w-full justify-start rounded-none border-0 bg-[rgba(15,27,40,0.9)] px-2.5 text-[13px] text-[#c7dbf2] transition-all hover:bg-[rgba(25,44,66,0.95)] hover:text-[#eaf3ff]",
                    isSelected && "bg-[rgba(36,58,84,0.72)] text-[#eaf4ff]",
                    isDisabled
                      && "bg-[rgba(12,20,30,0.72)] text-[#647f99] hover:bg-[rgba(12,20,30,0.72)] hover:text-[#647f99]"
                  )}
                  disabled={isDisabled}
                  onClick={() => toggleObject(c.id)}
                >
                  {c.name}
                </Button>
              );
            })}
            {visibleCandidates.length === 0 && (
              <div className="px-2.5 py-2 text-[12px] italic text-[#89a7c7]">No legal choices.</div>
            )}
          </div>
        </div>
      </div>
      <div className="w-full shrink-0 pt-1">
        <Button
          variant="ghost"
          size="sm"
          className="w-full h-7 rounded-sm border border-[#315274] bg-[rgba(15,27,40,0.88)] px-3 text-[13px] font-semibold text-[#8ec4ff] transition-all hover:border-[#4f7cad] hover:bg-[rgba(24,43,64,0.95)] hover:text-[#d7ebff]"
          disabled={!canAct || !canSubmit}
          onClick={() =>
            dispatch(
              { type: "select_objects", object_ids: Array.from(selected) },
              `Selected ${selected.size} object(s)`
            )
          }
        >
          Submit ({selected.size}/{min === max ? min : `${min}-${max}`})
        </Button>
      </div>
    </div>
  );
}
