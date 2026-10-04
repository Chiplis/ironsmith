import { useCallback, useEffect, useId, useState } from "react";
import useUiText from "@/i18n/useUiText";
import { useGame } from "@/context/GameContext";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { initialCounterDraft, parseCounterAllocationChoice, updateCounterDraft } from "@/lib/counter-choice";
import DecisionSummary from "./DecisionSummary";

export default function SelectCountersDecision({ decision, canAct, inlineSubmit = true, onSubmitActionChange = null, hideDescription = false, layout = "panel" }) {
  const ui = useUiText();
  const { dispatch } = useGame();
  const inputId = useId();
  const [entries, setEntries] = useState(() => initialCounterDraft(decision));
  const choice = parseCounterAllocationChoice(decision, entries);
  const canSubmit = Boolean(canAct && choice);
  const submit = useCallback(() => {
    if (canSubmit) dispatch(choice.command, `Removed ${choice.total} counters`);
  }, [canSubmit, choice, dispatch]);
  useEffect(() => {
    if (!onSubmitActionChange) return undefined;
    onSubmitActionChange({ label: "Submit", disabled: !canSubmit, onSubmit: submit });
    return () => onSubmitActionChange(null);
  }, [onSubmitActionChange, canSubmit, submit]);
  return <div className="flex h-full min-h-0 flex-col gap-2">
    <DecisionSummary decision={decision} hideDescription={hideDescription} layout={layout} />
    <div id={`${inputId}-range`} className="decision-helper-text">{decision.min_total} – {decision.max_total}</div>
    <ScrollArea className="flex-1 min-h-0">
      <div className={layout === "strip" ? "flex min-w-max gap-3" : "flex flex-col gap-2"}>
        {(decision.options || []).map((option) => <label key={option.index} className="flex items-center gap-2">
          <span>{option.description}</span>
          <Input type="text" inputMode="numeric" pattern="[0-9]*" className="decision-inline-input w-32"
            aria-label={option.description} aria-describedby={`${inputId}-range`} aria-invalid={!choice}
            value={entries.find((entry) => entry.index === option.index)?.value ?? "0"}
            disabled={!canAct || option.legal === false}
            onChange={(event) => setEntries((old) => updateCounterDraft(old, option.index, event.target.value))}
            onKeyDown={(event) => { if (event.key === "Enter" && canSubmit) { event.preventDefault(); submit(); } }} />
        </label>)}
      </div>
    </ScrollArea>
    {inlineSubmit && <Button variant="ghost" size="sm" className="decision-submit-button" disabled={!canSubmit} onClick={submit}>{ui("Submit")}</Button>}
  </div>;
}
