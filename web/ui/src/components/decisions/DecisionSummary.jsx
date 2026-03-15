import { useLayoutEffect, useMemo, useRef, useState } from "react";
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
    <div ref={contentRef} className="flex min-w-0 flex-col gap-y-0.5 px-1">
      {lines.map((line) => (
        <div key={line.key} className="min-w-0">
          <div
            data-strip-line="true"
            className={cn(
              "block min-w-0 leading-tight",
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
  const [stripExpanded, setStripExpanded] = useState(false);
  const [stripHeights, setStripHeights] = useState({ collapsed: 0, expanded: 0, overflowing: false });
  const stripContentRef = useRef(null);
  const stripMeasureRef = useRef(null);
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
          className: "text-[#c7dcf3]",
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
          className: "text-[#8fb5d8]",
        });
      }
    } else if (tabLayout) {
      if (description) {
        nextLines.push({
          key: "description",
          text: description,
          className: "decision-summary-tab-primary text-[#d6e6f7]",
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
          className: "decision-summary-tab-secondary text-[#91b7d9]",
        });
      }
    } else {
      if (description) {
        nextLines.push({
          key: "description",
          text: description,
          className: "text-[14px] text-[#c7dcf3]",
        });
      }
      if (contextText && !sameLine(contextText, description)) {
        nextLines.push({
          key: "context",
          text: contextText,
          className: "text-[13px] text-[#8fb5d8]",
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

  useLayoutEffect(() => {
    if (shouldHideSummary || !stripLayout || lines.length === 0) return undefined;

    const visibleContent = stripContentRef.current;
    const measureContent = stripMeasureRef.current;
    if (!visibleContent || !measureContent) return undefined;

    const recomputeHeights = () => {
      const nextCollapsed = Math.ceil(visibleContent.offsetHeight);
      const nextExpanded = Math.ceil(measureContent.offsetHeight);
      const overflowing = Array.from(visibleContent.querySelectorAll("[data-strip-line='true']")).some(
        (node) => node.scrollWidth > node.clientWidth + 1
      ) || nextExpanded > nextCollapsed + 1;

      setStripHeights((prev) => {
        if (
          prev.collapsed === nextCollapsed
          && prev.expanded === nextExpanded
          && prev.overflowing === overflowing
        ) {
          return prev;
        }
        return {
          collapsed: nextCollapsed,
          expanded: Math.max(nextExpanded, nextCollapsed),
          overflowing,
        };
      });
    };

    recomputeHeights();

    if (typeof ResizeObserver === "undefined") return undefined;

    const resizeObserver = new ResizeObserver(recomputeHeights);
    resizeObserver.observe(visibleContent);
    resizeObserver.observe(measureContent);

    return () => {
      resizeObserver.disconnect();
    };
  }, [shouldHideSummary, stripLayout, lines]);

  if (shouldHideSummary || lines.length === 0) return null;

  return (
    <div
      className={cn(
        stripLayout
          ? cn(
            "relative min-w-0 overflow-visible",
            stripHeights.overflowing && "cursor-help"
          )
          : tabLayout
            ? "decision-summary-tab-shell flex min-w-0 flex-col gap-1"
          : "flex flex-col gap-0.5 px-1 leading-snug",
        className
      )}
      onMouseEnter={stripLayout && stripHeights.overflowing ? () => setStripExpanded(true) : undefined}
      onMouseLeave={stripLayout ? () => setStripExpanded(false) : undefined}
      onFocusCapture={stripLayout && stripHeights.overflowing ? () => setStripExpanded(true) : undefined}
      onBlurCapture={stripLayout ? () => setStripExpanded(false) : undefined}
    >
      {stripLayout ? (
        <>
          <div
            className="relative z-[1] min-w-0 overflow-hidden rounded-none border border-[#2f4662]/55 bg-[rgba(10,18,28,0.72)] py-1 shadow-[inset_0_0_0_1px_rgba(84,118,150,0.08)] transition-[max-height,box-shadow,border-color] duration-150"
            style={{
              maxHeight: `${Math.max(
                0,
                stripExpanded && stripHeights.overflowing
                  ? stripHeights.expanded
                  : stripHeights.collapsed
              )}px`,
              borderColor: stripExpanded && stripHeights.overflowing
                ? "rgba(120, 170, 214, 0.72)"
                : undefined,
              boxShadow: stripExpanded && stripHeights.overflowing
                ? "0 0 16px rgba(71, 132, 189, 0.18), inset 0 0 0 1px rgba(97, 157, 212, 0.15)"
                : undefined,
            }}
          >
            <StripSummaryLines
              lines={lines}
              expanded={stripExpanded && stripHeights.overflowing}
              contentRef={stripContentRef}
            />
          </div>
          <div
            aria-hidden="true"
            className="pointer-events-none invisible absolute inset-x-0 top-0 -z-10 min-w-0 py-1"
          >
            <div
              ref={stripMeasureRef}
              className="rounded-none border border-transparent py-1"
            >
              <StripSummaryLines lines={lines} expanded />
            </div>
          </div>
        </>
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
