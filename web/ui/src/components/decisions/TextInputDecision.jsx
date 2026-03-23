import { useCallback, useEffect, useState } from "react";
import { useGame } from "@/context/GameContext";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import DecisionSummary from "./DecisionSummary";

export default function TextInputDecision({
  decision,
  canAct,
  inlineSubmit = true,
  onSubmitActionChange = null,
  hideDescription = false,
  layout = "panel",
}) {
  const { dispatch, game } = useGame();
  const stripLayout = layout === "strip";
  const initialValue = String(decision.value || "");
  const inputResetKey = `${decision.description || ""}|${decision.source_id || ""}|${initialValue}`;
  const [inputState, setInputState] = useState({ key: "", value: initialValue });
  const value = inputState.key === inputResetKey ? inputState.value : initialValue;
  const trimmedValue = value.trim();
  const requiresKnownValue = !!decision.require_known_value;
  const validationKey = `${requiresKnownValue ? 1 : 0}|${trimmedValue.toLowerCase()}`;
  const [validationState, setValidationState] = useState({ key: "", status: "idle" });
  const validationStatus = !requiresKnownValue
    ? "known"
    : !trimmedValue
      ? "idle"
      : validationState.key === validationKey
        ? validationState.status
        : "checking";
  const canSubmit = canAct
    && trimmedValue.length > 0
    && (!requiresKnownValue || validationStatus === "known");

  useEffect(() => {
    if (!requiresKnownValue || !trimmedValue) return undefined;
    if (!game || typeof game.isKnownCardName !== "function") return undefined;

    let cancelled = false;
    const timeoutId = window.setTimeout(async () => {
      try {
        const known = await game.isKnownCardName(trimmedValue);
        if (cancelled) return;
        setValidationState({
          key: validationKey,
          status: known ? "known" : "unknown",
        });
      } catch {
        if (cancelled) return;
        setValidationState({
          key: validationKey,
          status: "unknown",
        });
      }
    }, 120);

    return () => {
      cancelled = true;
      window.clearTimeout(timeoutId);
    };
  }, [game, requiresKnownValue, trimmedValue, validationKey]);

  const handleSubmit = useCallback(() => {
    if (!trimmedValue) return;
    dispatch({ type: "text_choice", value: trimmedValue }, trimmedValue);
  }, [dispatch, trimmedValue]);

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
          type="text"
          className={cn(
            "decision-inline-input h-8 bg-transparent",
            stripLayout ? "w-[220px] text-[14px]" : "w-full text-[16px]"
          )}
          value={value}
          onChange={(event) =>
            setInputState({ key: inputResetKey, value: event.target.value })
          }
          onKeyDown={(event) => {
            if (event.key === "Enter" && canSubmit) {
              event.preventDefault();
              handleSubmit();
            }
          }}
          placeholder={decision.placeholder || "Enter text"}
          disabled={!canAct}
          autoFocus
        />
      </div>
      {requiresKnownValue && trimmedValue ? (
        <div className="px-1 text-[11px] text-[#bfae8e]">
          {validationStatus === "checking"
            ? "Checking card name..."
            : validationStatus === "unknown"
              ? "Unknown card name"
              : "Known card name"}
        </div>
      ) : null}
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
