import { normalizeDecisionText } from "@/components/decisions/decisionText";

export function isTriggerOrderingDecision(decision) {
  if (!decision || decision.kind !== "select_options") return false;
  const reason = String(decision.reason || "").toLowerCase();
  const isOrderingReason = reason === "ordering" || reason.startsWith("order ");
  if (!isOrderingReason) return false;
  const optionCount = (decision.options || []).length;
  if (optionCount <= 1) return false;
  const description = String(decision.description || "").toLowerCase();
  const contextText = String(decision.context_text || "").toLowerCase();
  const consequenceText = String(decision.consequence_text || "").toLowerCase();
  const optionTexts = (decision.options || []).map((option) => String(option?.description || "").toLowerCase());
  return (
    description.includes("trigger")
    || description.includes("stack")
    || reason.includes("trigger")
    || contextText.includes("trigger")
    || contextText.includes("stack")
    || consequenceText.includes("trigger")
    || consequenceText.includes("stack")
    || optionTexts.some((text) => text.includes("\n"))
  );
}

export function buildTriggerOrderingKey(decision) {
  if (!isTriggerOrderingDecision(decision)) return "";
  return `${decision.kind}|${decision.player}|${decision.description || ""}|${(decision.options || [])
    .map((option) => `${Number(option?.index)}:${String(option?.description || "")}`)
    .join("|")}`;
}

export function defaultTriggerOrderingOrder(decision) {
  return (decision?.options || []).map((option) => Number(option.index));
}

export function normalizeTriggerOrderingOrder(order, decision) {
  const expected = defaultTriggerOrderingOrder(decision);
  const allowed = new Set(expected);
  const next = [];

  for (const index of order || []) {
    const numericIndex = Number(index);
    if (!allowed.has(numericIndex) || next.includes(numericIndex)) continue;
    next.push(numericIndex);
  }

  for (const index of expected) {
    if (!next.includes(index)) next.push(index);
  }

  return next;
}

export function splitTriggerOrderingOptionText(text) {
  const lines = String(text || "")
    .split("\n")
    .map((line) => normalizeDecisionText(String(line || "").trim()))
    .filter(Boolean);

  return {
    title: lines[0] || "Triggered ability",
    detail: lines.slice(1).join(" "),
  };
}

export function triggerOrderingSourceObjectId(option) {
  const related = Array.isArray(option?.related_object_ids) ? option.related_object_ids : [];
  const candidate = related.find((id) => id != null) ?? option?.object_id ?? null;
  if (candidate == null) return null;
  const numeric = Number(candidate);
  return Number.isFinite(numeric) ? numeric : null;
}

export function buildTriggerOrderingEntries(decision, order) {
  if (!isTriggerOrderingDecision(decision)) return [];

  const optionsByIndex = new Map(
    (decision.options || []).map((option) => [Number(option.index), option])
  );

  return normalizeTriggerOrderingOrder(order, decision)
    .map((optionIndex) => {
      const option = optionsByIndex.get(Number(optionIndex));
      if (!option) return null;

      const { title, detail } = splitTriggerOrderingOptionText(option.description);
      // The option id is a placeholder; the engine names the permanent (or
      // graveyard card) the trigger came from as a related object, which is
      // what a hover or click on the pending tile previews.
      const sourceObjectId = triggerOrderingSourceObjectId(option);

      return {
        id: `trigger-order-${optionIndex}`,
        inspect_object_id: sourceObjectId,
        stable_id: null,
        source_stable_id: null,
        controller: Number(decision.player),
        name: title,
        mana_cost: null,
        effect_text: null,
        ability_kind: "Triggered",
        ability_text: detail || title,
        targets: [],
        __trigger_ordering: true,
        __trigger_ordering_option_index: Number(optionIndex),
        __subtitle: detail,
      };
    })
    .filter(Boolean);
}
