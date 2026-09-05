import { useCallback, useEffect, useId, useState } from "react";
import { useGame } from "@/context/GameContext";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import { parseNumberChoice } from "@/lib/number-choice";
import DecisionSummary from "./DecisionSummary";

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
  const inputId = useId();
  const min = decision.min ?? 0;
  const max = decision.max ?? 999;
  const [value, setValue] = useState(String(min));
  const number = parseNumberChoice(value, min, max);
  const valid = number !== null;
  const canSubmit = canAct && valid;
  const handleSubmit = useCallback(() => {
    if (!canSubmit) return;
    dispatch({ type: "number_choice", value: number }, `Chose ${number}`);
  }, [dispatch, number, canSubmit]);

  useEffect(() => {
    if (!onSubmitActionChange) return undefined;
    onSubmitActionChange({
      label: "Submit",
      disabled: !canSubmit,
      onSubmit: handleSubmit,
    });
    return () => onSubmitActionChange(null);
  }, [onSubmitActionChange, canSubmit, handleSubmit]);

  const content = (
    <div className={cn(
      stripLayout ? "flex min-w-max items-center gap-2 px-1" : "flex flex-col gap-2 pr-1"
    )}>
      <DecisionSummary
        decision={decision}
        hideDescription={hideDescription}
        layout={layout}
        className={stripLayout ? "min-w-[220px]" : ""}
      />
      <div className="flex items-center gap-2">
        <Input
          type="number"
          className={cn(
            "decision-inline-input h-7 bg-transparent",
            stripLayout ? "w-[88px] text-[14px]" : "w-28 text-[16px]"
          )}
          id={inputId}
          aria-label={decision.description || "Choose a number"}
          aria-describedby={`${inputId}-range`}
          aria-invalid={!valid}
          min={min}
          max={max}
          step={1}
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && canSubmit) {
              event.preventDefault();
              handleSubmit();
            }
          }}
          disabled={!canAct}
        />
        <span id={`${inputId}-range`} className={cn(
          "decision-helper-text",
          stripLayout ? "text-[12px] whitespace-nowrap" : "text-[14px]"
        )}>
          ({min} – {max})
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
              "decision-neon-button decision-submit-button h-6 rounded-none px-2 text-[13px] font-semibold uppercase",
              stripLayout ? "w-auto" : "w-full"
            )}
            disabled={!canSubmit}
            onClick={handleSubmit}
          >
            Submit
          </Button>
        </div>
      )}
    </div>
  );
}
