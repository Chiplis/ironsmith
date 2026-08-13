export function normalizeDecisionText(text) {
  return text;
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
