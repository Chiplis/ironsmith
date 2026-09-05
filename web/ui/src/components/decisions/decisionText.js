export function normalizeDecisionText(text) {
  if (typeof text !== "string") return text;

  const normalized = text
    .replace(/\ba another\b/gi, "another")
    .replace(/\s+/g, " ")
    .trim();

  if (/^Cast without paying mana cost:\s*Free$/i.test(normalized)) {
    return "Cast for free";
  }
  if (/^Normal:\s*/i.test(normalized)) {
    return normalized.replace(/^Normal:\s*/i, "Pay mana cost · ");
  }
  return normalized;
}

export function translateKnownDecisionText(text, t) {
  const normalized = normalizeDecisionText(text);
  if (typeof normalized !== "string" || typeof t !== "function") return normalized;
  switch (normalized.trim()) {
    case "Keep hand":
      return t("action.keepHand");
    case "Mulligan":
      return t("action.mulligan");
    case "Pass priority":
      return t("action.passPriority");
    case "Resolve":
      return t("action.resolve");
    default:
      return normalized;
  }
}
