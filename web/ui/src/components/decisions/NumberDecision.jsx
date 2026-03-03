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
        <div className="text-[16px] text-muted-foreground">{decision.description}</div>
      )}
      {decision.source_name && (
        <div className="text-[14px] italic text-[#c0a060]">{decision.source_name}</div>
      )}
      <div className="flex items-center gap-2">
        <Input
          type="number"
          className="h-7 w-28 text-[16px] bg-transparent"
          min={decision.min ?? 0}
          max={decision.max ?? 999}
          value={value}
          onChange={(e) => setValue(Number(e.target.value))}
          disabled={!canAct}
        />
        <span className="text-[14px] text-muted-foreground">
          ({decision.min} - {decision.max})
        </span>
      </div>
      <Button
        variant="ghost"
        size="sm"
        className="h-7 text-[14px] text-muted-foreground transition-all hover:text-foreground hover:bg-[rgba(100,169,255,0.1)] hover:shadow-[0_0_8px_rgba(100,169,255,0.15)]"
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
