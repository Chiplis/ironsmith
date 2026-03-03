import { useState } from "react";
import { useGame } from "@/context/GameContext";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";

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
    <div className="flex flex-col gap-2">
      {decision.description && (
        <div className="text-[16px] text-muted-foreground">{decision.description}</div>
      )}
      <div className="text-[14px] text-muted-foreground">
        Select {min === max ? min : `${min}-${max}`} object(s)
      </div>
      <div className="flex flex-col gap-1">
        {candidates.map((c) => (
          <label
            key={c.id}
            className={`flex items-center gap-2 text-[14px] p-1 border rounded-sm cursor-pointer transition-colors ${
              c.legal
                ? selected.has(c.id)
                  ? "border-primary bg-primary/10"
                  : "border-game-line-2 hover:border-game-line"
                : "border-game-line-2 opacity-50 cursor-not-allowed"
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
      <Button
        variant="outline"
        size="sm"
        className="h-7 text-[14px]"
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
  );
}
