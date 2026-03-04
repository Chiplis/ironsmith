import { useState } from "react";
import { useGame } from "@/context/GameContext";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Checkbox } from "@/components/ui/checkbox";
import { normalizeDecisionText } from "./decisionText";

export default function SelectObjectsDecision({ decision, canAct }) {
  const { dispatch } = useGame();
  const candidates = decision.candidates || [];
  const [selected, setSelected] = useState(new Set());
  const min = decision.min ?? 0;
  const max = decision.max ?? candidates.length;

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
          <div className="grid grid-cols-2 gap-1">
            {candidates.map((c) => (
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
