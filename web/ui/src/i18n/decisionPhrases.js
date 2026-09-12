// Localize the prose the engine composes itself.
//
// Prompts that quote a card are handled positionally by
// `@/lib/decision-text-translation`. What is left is the engine's own frame
// vocabulary — "Choose the next cost to pay for X's ability", the stage chips,
// the reason badges. That vocabulary is small and closed, so it lives here as
// message keys rather than as strings to be machine-translated.

// `decision_reason` in the wasm bridge produces exactly these, and nothing else.
export const DECISION_REASON_KEYS = {
  "Additional costs": "decision.reason.additionalCosts",
  "Choose color": "decision.reason.chooseColor",
  "Choose number": "decision.reason.chooseNumber",
  "Choose targets": "decision.reason.chooseTargets",
  Destroy: "decision.reason.destroy",
  Discard: "decision.reason.discard",
  Distribute: "decision.reason.distribute",
  Exile: "decision.reason.exile",
  "Legend rule": "decision.reason.legendRule",
  Madness: "decision.reason.madness",
  "Mana payment": "decision.reason.manaPayment",
  "May ability": "decision.reason.mayAbility",
  Miracle: "decision.reason.miracle",
  "Modal choice": "decision.reason.modalChoice",
  "Next cost": "decision.reason.nextCost",
  "Order attackers": "decision.reason.orderAttackers",
  "Order blockers": "decision.reason.orderBlockers",
  "Order triggers": "decision.reason.orderTriggers",
  Ordering: "decision.reason.ordering",
  Proliferate: "decision.reason.proliferate",
  "Remove counters": "decision.reason.removeCounters",
  "Replacement effect": "decision.reason.replacementEffect",
  Retarget: "decision.reason.retarget",
  Return: "decision.reason.return",
  Sacrifice: "decision.reason.sacrifice",
  Scry: "decision.reason.scry",
  "Search library": "decision.reason.searchLibrary",
  Surveil: "decision.reason.surveil",
  "Text entry": "decision.reason.textEntry",
  Ward: "decision.reason.ward",
  "X value": "decision.reason.xValue",
};

// Ordered: the first pattern that matches wins, so the possessive form of a
// phrase has to come before its generic form.
const ENGINE_PHRASES = [
  {
    pattern: /^Choose the next cost to pay for (.+)'s ability$/u,
    key: "decision.chooseNextCostAbility",
    params: (match, card) => ({ card: card(match[1]) }),
  },
  {
    pattern: /^Choose the next cost to pay for (.+)$/u,
    key: "decision.chooseNextCost",
    params: (match, card) => ({ source: card(match[1]) }),
  },
  {
    pattern: /^Tapping resolves immediately\. Other costs may open a follow-up payment prompt\.$/u,
    key: "decision.nextCostHint",
    params: () => ({}),
  },
  {
    // An engine frame wrapped around a cost quoted from the card, so the tail
    // goes back through the card-text resolver.
    pattern: /^Choose a creature to sacrifice: (.+)$/u,
    key: "decision.chooseCreatureToSacrifice",
    params: (match, card, quote) => ({ cost: quote(match[1]) }),
  },
  {
    pattern: /^Choose (.+) to sacrifice$/u,
    key: "decision.chooseToSacrifice",
    params: (match) => ({ subject: match[1] }),
  },
  // The wasm bridge renders a yes/no decision as these two exact options.
  { pattern: /^Yes$/u, key: "decision.yes", params: () => ({}) },
  { pattern: /^No$/u, key: "decision.no", params: () => ({}) },
  { pattern: /^Mana: (.+)$/u, key: "decision.manaOption", params: (match) => ({ cost: match[1] }) },
  { pattern: /^Tap this permanent$/u, key: "decision.tapThisPermanent", params: () => ({}) },
  { pattern: /^Untap this permanent$/u, key: "decision.untapThisPermanent", params: () => ({}) },
  { pattern: /^(.+)'s ability$/u, key: "decision.sourceAbility", params: (match, card) => ({ card: card(match[1]) }) },
];

/**
 * Localize one engine-composed phrase, or null when none applies.
 *
 * `cardName`/`englishCardName` carry the decision source's official localized
 * printing so a name inside a phrase is swapped too; an unrecognized name is
 * left as the engine wrote it. `quote` re-localizes a captured fragment that
 * came from the card rather than from the engine.
 */
export function localizeEnginePhrase(
  text,
  { t, cardName = "", englishCardName = "", quote = (value) => value } = {}
) {
  const value = String(text || "").trim();
  if (!value || typeof t !== "function") return null;

  const reasonKey = DECISION_REASON_KEYS[value];
  if (reasonKey) return t(reasonKey);

  const localizeName = (name) => (
    cardName && englishCardName && name.trim() === englishCardName.trim() ? cardName : name
  );
  for (const { pattern, key, params } of ENGINE_PHRASES) {
    const match = pattern.exec(value);
    if (match) return t(key, params(match, localizeName, quote));
  }
  return null;
}
