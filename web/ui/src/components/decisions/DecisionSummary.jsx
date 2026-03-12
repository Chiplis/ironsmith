import { useGame } from "@/context/GameContext";
import { SymbolText } from "@/lib/mana-symbols";
import { getVisibleTopStackObject } from "@/lib/stack-targets";
import { cn } from "@/lib/utils";
import { normalizeDecisionText } from "./decisionText";

function normalizeLine(text) {
  if (typeof text !== "string") return "";
  return normalizeDecisionText(text).trim();
}

function sameLine(left, right) {
  return normalizeLine(left).toLowerCase() === normalizeLine(right).toLowerCase();
}

export default function DecisionSummary({
  decision,
  hideDescription = false,
  layout = "panel",
  className = "",
}) {
  const { state } = useGame();
  if (!decision) return null;
  if (layout === "strip" && hideDescription) return null;

  const stripLayout = layout === "strip";
  const description = hideDescription ? "" : normalizeLine(decision.description);
  const topStackObject = getVisibleTopStackObject(state);
  const resolvingStackContextText = (() => {
    if (!topStackObject?.ability_kind) return "";
    if (decision.source_id == null) return "";
    const sourceId = String(decision.source_id);
    const stackSourceId = topStackObject.inspect_object_id != null
      ? String(topStackObject.inspect_object_id)
      : (topStackObject.id != null ? String(topStackObject.id) : "");
    if (!stackSourceId || stackSourceId !== sourceId) return "";
    return normalizeLine(topStackObject.ability_text || topStackObject.effect_text || "");
  })();
  const contextText = resolvingStackContextText || normalizeLine(decision.context_text);
  const consequenceText = normalizeLine(decision.consequence_text);

  const lines = [];
  if (description) {
    lines.push({
      key: "description",
      text: description,
      className: stripLayout ? "text-[13px] text-[#c7dcf3]" : "text-[14px] text-[#c7dcf3]",
    });
  }
  if (contextText && !sameLine(contextText, description)) {
    lines.push({
      key: "context",
      text: contextText,
      className: stripLayout ? "text-[12px] text-[#8fb5d8]" : "text-[13px] text-[#8fb5d8]",
    });
  }
  if (consequenceText && !sameLine(consequenceText, description) && !sameLine(consequenceText, contextText)) {
    lines.push({
      key: "consequence",
      text: `Follow-up: ${consequenceText}`,
      className: stripLayout ? "text-[12px] text-[#f0cf8a]" : "text-[13px] text-[#f0cf8a]",
    });
  }

  if (lines.length === 0) return null;

  return (
    <div className={cn("flex flex-col gap-0.5 px-1 leading-snug", className)}>
      {lines.map((line) => (
        <div key={line.key} className={line.className}>
          <SymbolText text={line.text} />
        </div>
      ))}
    </div>
  );
}
