import { useState } from "react";
import { useGame } from "@/context/GameContext";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

export default function NumberDecision({ decision, canAct }) {
  const { dispatch } = useGame();
  const [value, setValue] = useState(decision.min ?? 0);

  return (
    <div className="flex flex-col gap-2">
      {decision.description && (
        <div className="text-[12px] text-muted-foreground">{decision.description}</div>
      )}
      {decision.source_name && (
        <div className="text-[11px] italic text-[#c0a060]">{decision.source_name}</div>
      )}
      <div className="flex items-center gap-2">
        <Input
          type="number"
          className="h-7 w-20 text-[12px] bg-transparent"
          min={decision.min ?? 0}
          max={decision.max ?? 999}
          value={value}
          onChange={(e) => setValue(Number(e.target.value))}
          disabled={!canAct}
        />
        <span className="text-[11px] text-muted-foreground">
          ({decision.min} - {decision.max})
        </span>
      </div>
      <Button
        variant="outline"
        size="sm"
        className="h-7 text-[11px]"
        disabled={!canAct}
        onClick={() =>
          dispatch({ type: "number_choice", value }, `Chose ${value}`)
        }
      >
        Submit
      </Button>
    </div>
  );
}
