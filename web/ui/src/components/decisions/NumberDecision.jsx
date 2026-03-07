import { useCallback, useEffect, useState } from "react";
import { useGame } from "@/context/GameContext";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { normalizeDecisionText } from "./decisionText";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";

export default function NumberDecision({
  decision,
  canAct,
  inlineSubmit = true,
  onSubmitActionChange = null,
  hideDescription = false,
  layout = "panel",
}) {
  const { dispatch } = useGame();
  const stripLayout = layout === "strip";
  const [value, setValue] = useState(decision.min ?? 0);
  const handleSubmit = useCallback(() => {
    dispatch({ type: "number_choice", value }, `Chose ${value}`);
  }, [dispatch, value]);

  useEffect(() => {
    if (!onSubmitActionChange) return undefined;
    onSubmitActionChange({
      label: "Submit",
      disabled: !canAct,
      onSubmit: handleSubmit,
    });
    return () => onSubmitActionChange(null);
  }, [onSubmitActionChange, canAct, handleSubmit]);

  const content = (
    <div className={cn(
      stripLayout ? "flex min-w-max items-center gap-2 px-1" : "flex flex-col gap-2 pr-1"
    )}>
      {!hideDescription && decision.description && (
        <div className={cn(
          "text-muted-foreground",
          stripLayout ? "text-[13px] whitespace-nowrap" : "text-[16px]"
        )}>
          {normalizeDecisionText(decision.description)}
        </div>
      )}
      {decision.source_name && (
        <div className={cn(
          "italic text-[#c0a060]",
          stripLayout ? "text-[12px] whitespace-nowrap" : "text-[14px]"
        )}>
          {decision.source_name}
        </div>
      )}
      <div className="flex items-center gap-2">
        <Input
          type="number"
          className={cn(
            "h-7 bg-transparent",
            stripLayout ? "w-[88px] text-[14px]" : "w-28 text-[16px]"
          )}
          min={decision.min ?? 0}
          max={decision.max ?? 999}
          value={value}
          onChange={(e) => setValue(Number(e.target.value))}
          disabled={!canAct}
        />
        <span className={cn(
          "text-muted-foreground",
          stripLayout ? "text-[12px] whitespace-nowrap" : "text-[14px]"
        )}>
          ({decision.min} - {decision.max})
        </span>
      </div>
    </div>
  );

  return (
    <div className={cn(
      "flex h-full min-h-0 flex-col gap-2",
      stripLayout && "min-w-0 gap-1.5"
    )}>
      {stripLayout ? (
        <div className="min-w-0 overflow-x-auto overflow-y-hidden">
          {content}
        </div>
      ) : (
        <ScrollArea className="flex-1 min-h-0">
          {content}
        </ScrollArea>
      )}
      {inlineSubmit && (
        <div className={cn(
          "shrink-0",
          stripLayout ? "pt-0" : "border-t border-game-line-2/70 pt-1"
        )}>
          <Button
            variant="ghost"
            size="sm"
            className={cn(
              "decision-neon-button decision-submit-button h-6 rounded-sm px-2 text-[13px] font-semibold uppercase",
              stripLayout ? "w-auto" : "w-full"
            )}
            disabled={!canAct}
            onClick={handleSubmit}
          >
            Submit
          </Button>
        </div>
      )}
    </div>
  );
}
