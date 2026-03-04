import { useEffect, useMemo, useRef, useState } from "react";
import { useGame } from "@/context/GameContext";
import { useHoveredObjectId } from "@/context/HoverContext";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Checkbox } from "@/components/ui/checkbox";
import { normalizeDecisionText } from "./decisionText";

export default function SelectObjectsDecision({ decision, canAct }) {
  const { dispatch } = useGame();
  const hoveredObjectId = useHoveredObjectId();
  const candidates = useMemo(() => decision.candidates || [], [decision.candidates]);
  const [selected, setSelected] = useState(new Set());
  const min = decision.min ?? 0;
  const max = decision.max ?? candidates.length;
  const hideTimerRef = useRef(null);

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
    <div className="flex h-full min-h-0 flex-col gap-2">
      <ScrollArea className="flex-1 min-h-0">
        <div className="flex flex-col gap-2 pr-1">
          {decision.description && (
            <div className="text-[16px] text-muted-foreground">{normalizeDecisionText(decision.description)}</div>
          )}
          <div className="text-[14px] text-muted-foreground">
            Select {min === max ? min : `${min}-${max}`} object(s)
          </div>
          {focusedToHover && (
            <div className="text-[12px] italic text-[#89a7c7] px-0.5 -mt-1">
              Showing options for the hovered card.
            </div>
          )}
          <div
            className={`grid grid-cols-2 gap-1 transition-all duration-200 ${
              showRows ? "opacity-100 translate-y-0" : "opacity-0 -translate-y-1 pointer-events-none"
            }`}
          >
            {visibleCandidates.map((c) => (
              <label
                key={c.id}
                className={`flex items-center gap-2 text-[14px] p-1 rounded-sm cursor-pointer transition-all ${
                  c.legal
                    ? selected.has(c.id)
                      ? "text-foreground bg-primary/10 shadow-[0_0_6px_rgba(100,169,255,0.2)]"
                      : "text-muted-foreground hover:text-foreground hover:bg-[rgba(100,169,255,0.06)]"
                    : "opacity-50 cursor-not-allowed"
                }`}
              >
                <Checkbox
                  checked={selected.has(c.id)}
                  onCheckedChange={() => c.legal && toggleObject(c.id)}
                  disabled={!canAct || !c.legal}
                  className="h-3.5 w-3.5"
                />
                <span>{c.name}</span>
              </label>
            ))}
          </div>
        </div>
      </ScrollArea>
      <div className="shrink-0 border-t border-game-line-2/70 pt-1">
        <Button
          variant="ghost"
          size="sm"
          className="h-auto min-h-7 py-1.5 text-[14px] px-3 text-[#69c769]/60 hover:text-[#69c769] hover:bg-[#69c769]/10 hover:shadow-[0_0_8px_rgba(105,199,105,0.15)] transition-all"
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
