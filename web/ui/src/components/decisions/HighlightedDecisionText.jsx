import { useMemo } from "react";
import { SymbolText } from "@/lib/mana-symbols";

function splitHighlightedText(text, highlightText) {
  const source = String(text || "");
  const needle = String(highlightText || "").trim();
  if (!source || !needle) {
    return { before: source, match: "", after: "" };
  }

  const sourceLower = source.toLowerCase();
  const needleLower = needle.toLowerCase();
  const matchIndex = sourceLower.indexOf(needleLower);
  if (matchIndex < 0) {
    return { before: source, match: "", after: "" };
  }

  return {
    before: source.slice(0, matchIndex),
    match: source.slice(matchIndex, matchIndex + needle.length),
    after: source.slice(matchIndex + needle.length),
  };
}

export default function HighlightedDecisionText({
  text,
  highlightText = "",
  highlightColor = null,
  className = "",
  style = undefined,
  onHighlightClick = null,
  highlightAriaLabel = null,
}) {
  const normalizedText = String(text || "");
  const segments = useMemo(
    () => splitHighlightedText(normalizedText, highlightText),
    [highlightText, normalizedText]
  );

  if (!segments.match) {
    return (
      <span className={className} style={style}>
        <SymbolText text={normalizedText} style={{ whiteSpace: "inherit" }} />
      </span>
    );
  }

  return (
    <span className={className} style={style}>
      {segments.before && (
        <SymbolText text={segments.before} style={{ whiteSpace: "inherit" }} />
      )}
      <span
        className={onHighlightClick ? "decision-card-name-trigger" : undefined}
        style={highlightColor ? { color: highlightColor } : undefined}
        role={onHighlightClick ? "button" : undefined}
        tabIndex={onHighlightClick ? 0 : undefined}
        aria-label={onHighlightClick ? (highlightAriaLabel || `Inspect ${segments.match}`) : undefined}
        onPointerDown={onHighlightClick ? (event) => {
          event.stopPropagation();
        } : undefined}
        onPointerUp={onHighlightClick ? (event) => {
          if (event.button !== 0) return;
          event.stopPropagation();
          onHighlightClick(event);
        } : undefined}
        onClick={onHighlightClick ? (event) => {
          event.stopPropagation();
          if (event.detail === 0) onHighlightClick(event);
        } : undefined}
        onKeyDown={onHighlightClick ? (event) => {
          if (event.key !== "Enter" && event.key !== " ") return;
          event.preventDefault();
          event.stopPropagation();
          onHighlightClick(event);
        } : undefined}
      >
        <SymbolText text={segments.match} style={{ whiteSpace: "inherit" }} />
      </span>
      {segments.after && (
        <SymbolText text={segments.after} style={{ whiteSpace: "inherit" }} />
      )}
    </span>
  );
}
