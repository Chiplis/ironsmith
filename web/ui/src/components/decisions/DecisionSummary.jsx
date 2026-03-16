import { useMemo } from "react";
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

function stackObjectMatchesDecisionSource(stackObject, decision) {
  if (!stackObject || !decision) return false;

  const decisionSourceId = decision?.source_id != null ? String(decision.source_id) : "";
  const stackIds = [
    stackObject?.inspect_object_id,
    stackObject?.id,
  ]
    .filter((value) => value != null)
    .map((value) => String(value));

  if (decisionSourceId && stackIds.includes(decisionSourceId)) {
    return true;
  }

  const decisionSourceName = normalizeLine(decision?.source_name).toLowerCase();
  const stackName = normalizeLine(stackObject?.name).toLowerCase();
  return Boolean(decisionSourceName && stackName && decisionSourceName === stackName);
}

function StripSummaryLines({ lines, expanded = false, contentRef = null }) {
  return (
    <div ref={contentRef} className="decision-strip-summary-lines flex min-w-0 flex-col gap-y-0.5 px-1">
      {lines.map((line) => (
        <div key={line.key} className="min-w-0">
          <div
            data-strip-line="true"
            className={cn(
              "decision-strip-summary-line block min-w-0 leading-tight",
              expanded
                ? "whitespace-normal break-words"
                : "overflow-hidden text-ellipsis whitespace-nowrap",
              line.className
            )}
          >
            <SymbolText text={line.text} noWrap={!expanded} />
          </div>
        </div>
      ))}
    </div>
  );
}

export default function DecisionSummary({
  decision,
  hideDescription = false,
  layout = "panel",
  className = "",
}) {
  const { state } = useGame();
  const stripLayout = layout === "strip";
  const tabLayout = layout === "tab";
  const shouldHideSummary = !decision || (stripLayout && hideDescription);
  const description = hideDescription ? "" : normalizeLine(decision?.description);
  const topStackObject = getVisibleTopStackObject(state);
  const resolvingStackContextText = (() => {
    if (!decision || !stackObjectMatchesDecisionSource(topStackObject, decision)) return "";

    const rawStackText = normalizeLine(
      topStackObject?.ability_text || topStackObject?.effect_text || ""
    );
    if (!rawStackText) return "";

    const stackPrefix = topStackObject?.ability_kind
      ? `${normalizeLine(topStackObject.ability_kind)} effects`
      : "Spell effects";
    const normalizedPrefix = `${stackPrefix.toLowerCase()}:`;
    if (rawStackText.toLowerCase().startsWith(normalizedPrefix)) {
      return rawStackText;
    }
    return `${stackPrefix}: ${rawStackText}`;
  })();
  const contextText = resolvingStackContextText || normalizeLine(decision?.context_text);
  const consequenceText = normalizeLine(decision?.consequence_text);

  const lines = useMemo(() => {
    const nextLines = [];

    if (stripLayout) {
      if (description) {
        nextLines.push({
          key: "description",
          text: description,
          className: "text-[#eadfc4]",
        });
      }
      const secondarySegments = [];
      if (contextText && !sameLine(contextText, description)) {
        secondarySegments.push(contextText);
      }
      if (consequenceText && !sameLine(consequenceText, description) && !sameLine(consequenceText, contextText)) {
        secondarySegments.push(`Follow-up: ${consequenceText}`);
      }
      if (secondarySegments.length > 0) {
        nextLines.push({
          key: "secondary",
          text: secondarySegments.join(" | "),
          className: "text-[#bfae8e]",
        });
      }
    } else if (tabLayout) {
      if (description) {
        nextLines.push({
          key: "description",
          text: description,
          className: "decision-summary-tab-primary text-[#eadfc4]",
        });
      }
      const secondarySegments = [];
      if (contextText && !sameLine(contextText, description)) {
        secondarySegments.push(contextText);
      }
      if (consequenceText && !sameLine(consequenceText, description) && !sameLine(consequenceText, contextText)) {
        secondarySegments.push(`Follow-up: ${consequenceText}`);
      }
      if (secondarySegments.length > 0) {
        nextLines.push({
          key: "secondary",
          text: secondarySegments.join(" | "),
          className: "decision-summary-tab-secondary text-[#bfae8e]",
        });
      }
    } else {
      if (description) {
        nextLines.push({
          key: "description",
          text: description,
          className: "text-[14px] text-[#eadfc4]",
        });
      }
      if (contextText && !sameLine(contextText, description)) {
        nextLines.push({
          key: "context",
          text: contextText,
          className: "text-[13px] text-[#bfae8e]",
        });
      }
      if (consequenceText && !sameLine(consequenceText, description) && !sameLine(consequenceText, contextText)) {
        nextLines.push({
          key: "consequence",
          text: `Follow-up: ${consequenceText}`,
          className: "text-[13px] text-[#f0cf8a]",
        });
      }
    }

    return nextLines;
  }, [consequenceText, contextText, description, stripLayout, tabLayout]);

  if (shouldHideSummary || lines.length === 0) return null;

  const stripDense = stripLayout && lines.some((line) => normalizeLine(line.text).length > 108);

  return (
    <div
      className={cn(
        stripLayout
          ? cn(
            "decision-strip-summary-shell relative min-w-0",
            stripDense && "is-dense"
          )
          : tabLayout
            ? "decision-summary-tab-shell flex min-w-0 flex-col gap-1"
          : "flex flex-col gap-0.5 px-1 leading-snug",
        className
      )}
    >
      {stripLayout ? (
        <div className="decision-strip-summary-surface relative z-[1] min-w-0 overflow-hidden rounded-none">
          <div className="decision-strip-summary-lines flex min-w-0 flex-col gap-y-0.5 px-1.5 py-1">
            {lines.map((line) => (
              <div key={line.key} className="min-w-0">
                <div
                  data-strip-line="true"
                  className={cn(
                    "decision-strip-summary-line block min-w-0 overflow-hidden text-ellipsis whitespace-nowrap",
                    line.className
                  )}
                  title={line.text}
                >
                  <SymbolText text={line.text} noWrap />
                </div>
              </div>
            ))}
          </div>
        </div>
      ) : tabLayout ? (
        lines.map((line) => (
          <div key={line.key} className={cn("min-w-0", line.className)}>
            <SymbolText text={line.text} />
          </div>
        ))
      ) : (
        lines.map((line) => (
          <div key={line.key} className={line.className}>
            <SymbolText text={line.text} />
          </div>
        ))
      )}
    </div>
  );
}
