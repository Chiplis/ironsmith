import { useState, useMemo } from "react";
import { useGame } from "@/context/GameContext";
import { Button } from "@/components/ui/button";

export default function TargetsDecision({ decision, canAct }) {
  const { dispatch } = useGame();
  const requirements = decision.requirements || [];
  const [currentReqIdx, setCurrentReqIdx] = useState(0);
  const [selections, setSelections] = useState([]);

  const currentReq = requirements[currentReqIdx];
  const allDone = currentReqIdx >= requirements.length;

  const handleSelectTarget = (target) => {
    const targetInput =
      target.kind === "player"
        ? { kind: "player", player: Number(target.player) }
        : { kind: "object", object: Number(target.object) };

    const newSelections = [...selections, targetInput];
    setSelections(newSelections);

    // Check if we've met the minimum for this requirement
    const selectionsForReq = newSelections.length;
    if (selectionsForReq >= (currentReq?.min_targets || 1)) {
      if (currentReqIdx + 1 < requirements.length) {
        setCurrentReqIdx(currentReqIdx + 1);
      }
    }
  };

  const handleSubmit = () => {
    dispatch({ type: "select_targets", targets: selections }, "Targets selected");
  };

  // Flatten for simple form-based approach
  if (requirements.length === 0) return null;

  return (
    <div className="flex flex-col gap-2">
      {decision.context && (
        <div className="text-[12px] text-muted-foreground">{decision.context}</div>
      )}

      {requirements.map((req, reqIdx) => {
        const isActive = reqIdx === currentReqIdx && !allDone;
        return (
          <div key={reqIdx} className={`border p-1.5 rounded-sm ${isActive ? "border-primary" : "border-game-line-2"}`}>
            <div className="text-[11px] text-muted-foreground mb-1">
              Target {reqIdx + 1}: {req.description || "Choose a target"}
              <span className="ml-1 text-[10px]">
                ({req.min_targets}-{req.max_targets ?? req.legal_targets?.length ?? "?"})
              </span>
            </div>
            {isActive && (
              <div className="flex flex-col gap-0.5">
                {(req.legal_targets || []).map((target, tIdx) => {
                  const label =
                    target.kind === "player"
                      ? `Player ${target.player}`
                      : target.name || `Object ${target.object}`;
                  return (
                    <Button
                      key={tIdx}
                      variant="outline"
                      size="sm"
                      className="h-6 text-[11px] justify-start px-2"
                      disabled={!canAct}
                      onClick={() => handleSelectTarget(target)}
                    >
                      {label}
                    </Button>
                  );
                })}
              </div>
            )}
          </div>
        );
      })}

      <Button
        variant="outline"
        size="sm"
        className="h-7 text-[11px]"
        disabled={!canAct || selections.length === 0}
        onClick={handleSubmit}
      >
        Submit Targets ({selections.length})
      </Button>
    </div>
  );
}
