import { useState, useMemo } from "react";
import { useGame } from "@/context/GameContext";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";
import { X, ArrowRight } from "lucide-react";

export default function TargetsDecision({ decision, canAct }) {
  const { dispatch } = useGame();
  const requirements = decision.requirements || [];
  const [currentReqIdx, setCurrentReqIdx] = useState(0);
  // Per-requirement selections: array of arrays
  const [selectionsByReq, setSelectionsByReq] = useState(() =>
    requirements.map(() => [])
  );

  const currentReq = requirements[currentReqIdx];
  const allDone = currentReqIdx >= requirements.length;

  // Flat list of all selections for dispatch
  const allSelections = useMemo(
    () => selectionsByReq.flat(),
    [selectionsByReq]
  );

  // Check if current requirement has met its minimum
  const currentReqSelections = selectionsByReq[currentReqIdx] || [];
  const currentMin = currentReq?.min_targets ?? 1;
  const currentMax = currentReq?.max_targets ?? currentReq?.legal_targets?.length ?? 1;
  const currentMet = currentReqSelections.length >= currentMin;

  // Overall: all requirements met their minimums
  const allMinsMet = requirements.every(
    (req, idx) => (selectionsByReq[idx] || []).length >= (req.min_targets ?? 1)
  );

  // Can submit: either all done cycling through, or all mins are met
  const canSubmit = allDone || allMinsMet;

  const handleSelectTarget = (target) => {
    const targetInput =
      target.kind === "player"
        ? { kind: "player", player: Number(target.player) }
        : { kind: "object", object: Number(target.object) };

    setSelectionsByReq((prev) => {
      const next = prev.map((arr) => [...arr]);
      next[currentReqIdx] = [...(next[currentReqIdx] || []), targetInput];

      // Auto-advance if we've hit max for this requirement
      const reqMax =
        currentReq?.max_targets ?? currentReq?.legal_targets?.length ?? 1;
      if (next[currentReqIdx].length >= reqMax) {
        // Find next unfilled requirement
        let nextIdx = currentReqIdx + 1;
        while (nextIdx < requirements.length) {
          const reqMin = requirements[nextIdx].min_targets ?? 1;
          if ((next[nextIdx] || []).length < reqMin) break;
          nextIdx++;
        }
        // Use setTimeout to batch with the state update
        setTimeout(() => setCurrentReqIdx(nextIdx), 0);
      }
      return next;
    });
  };

  const handleRemoveTarget = (reqIdx, selIdx) => {
    setSelectionsByReq((prev) => {
      const next = prev.map((arr) => [...arr]);
      next[reqIdx] = next[reqIdx].filter((_, i) => i !== selIdx);
      return next;
    });
    // Jump back to this requirement if needed
    if (reqIdx < currentReqIdx) {
      setCurrentReqIdx(reqIdx);
    }
  };

  const handleSkipRequirement = () => {
    if (currentReqIdx + 1 <= requirements.length) {
      setCurrentReqIdx(currentReqIdx + 1);
    }
  };

  const handleSubmit = () => {
    dispatch({ type: "select_targets", targets: allSelections }, "Targets selected");
  };

  if (requirements.length === 0) return null;

  return (
    <div className="flex h-full min-h-0 flex-col gap-2">
      <ScrollArea className="flex-1 min-h-0">
        <div className="flex flex-col gap-2 pr-1">
          {decision.context && (
            <div className="text-[16px] text-muted-foreground">{decision.context}</div>
          )}

          <div className={requirements.length > 1 ? "grid grid-cols-2 gap-2" : ""}>
          {requirements.map((req, reqIdx) => {
            const isActive = reqIdx === currentReqIdx && !allDone;
            const reqSelections = selectionsByReq[reqIdx] || [];
            const reqMin = req.min_targets ?? 1;
            const reqMax = req.max_targets ?? req.legal_targets?.length ?? "?";
            const isOptional = reqMin === 0;

            return (
              <div
                key={reqIdx}
                className={cn(
                  "p-1.5 rounded-sm",
                  isActive && "bg-[rgba(100,169,255,0.05)]"
                )}
              >
                <div className="text-[14px] text-muted-foreground mb-1">
                  Target {reqIdx + 1}: {req.description || "Choose a target"}
                  <span className="ml-1 text-[13px]">
                    ({reqMin}-{reqMax})
                    {isOptional && " [optional]"}
                  </span>
                </div>

                {/* Show current selections for this requirement */}
                {reqSelections.length > 0 && (
                  <div className="flex flex-wrap gap-0.5 mb-1">
                    {reqSelections.map((sel, selIdx) => {
                      const label =
                        sel.kind === "player"
                          ? `Player ${sel.player}`
                          : `Object ${sel.object}`;
                      return (
                        <Button
                          key={selIdx}
                          variant="ghost"
                          size="sm"
                          className="h-5 text-[13px] px-1.5 text-primary bg-primary/10 shadow-[0_0_6px_rgba(100,169,255,0.2)]"
                          disabled={!canAct}
                          onClick={() => handleRemoveTarget(reqIdx, selIdx)}
                        >
                          {label} <X className="size-3 inline" />
                        </Button>
                      );
                    })}
                  </div>
                )}

                {isActive && (
                  <div className="flex flex-wrap gap-1">
                    {(req.legal_targets || []).map((target, tIdx) => {
                      const label =
                        target.kind === "player"
                          ? target.name || `Player ${target.player}`
                          : target.name || `Object ${target.object}`;
                      return (
                        <Button
                          key={tIdx}
                          variant="ghost"
                          size="sm"
                          className="h-6 text-[14px] justify-start px-2 text-muted-foreground transition-all hover:text-foreground hover:bg-[rgba(100,169,255,0.1)]"
                          disabled={!canAct}
                          onClick={() => handleSelectTarget(target)}
                        >
                          {label}
                        </Button>
                      );
                    })}
                    {/* Skip button for optional requirements or when min is met */}
                    {(isOptional || currentMet) && !allDone && (
                      <Button
                        variant="ghost"
                        size="sm"
                        className="h-5 text-[13px] text-muted-foreground"
                        disabled={!canAct}
                        onClick={handleSkipRequirement}
                      >
                        {isOptional ? "Skip (optional)" : <>Next requirement <ArrowRight className="size-3 inline" /></>}
                      </Button>
                    )}
                  </div>
                )}
              </div>
            );
          })}
          </div>
        </div>
      </ScrollArea>

      <div className="shrink-0 border-t border-game-line-2/70 pt-1">
        <Button
          variant="ghost"
          size="sm"
          className="h-7 text-[14px] text-muted-foreground transition-all hover:text-foreground hover:bg-[rgba(100,169,255,0.1)] hover:shadow-[0_0_8px_rgba(100,169,255,0.15)]"
          disabled={!canAct || !canSubmit}
          onClick={handleSubmit}
        >
          Submit Targets ({allSelections.length})
        </Button>
      </div>
    </div>
  );
}
