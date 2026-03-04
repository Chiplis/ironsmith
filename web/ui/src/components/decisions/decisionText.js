export function normalizeDecisionText(text) {
  if (typeof text !== "string") return text;
  return text.replace(/\bPay mana pips?\b/gi, "Pay");
}

