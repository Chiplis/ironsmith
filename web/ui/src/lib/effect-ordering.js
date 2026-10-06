import {
  buildTriggerOrderingEntries,
  defaultTriggerOrderingOrder,
  isTriggerOrderingDecision,
  normalizeTriggerOrderingOrder,
  splitTriggerOrderingOptionText,
  triggerOrderingSourceObjectId,
} from "./trigger-ordering";
import { buildObjectControllerById, buildObjectNameById } from "./decision-object-meta";
import { playerDisplayName } from "./player-display";

function isReplacementChoice(decision) {
  if (decision?.kind !== "select_options" || (decision.options || []).length <= 1) return false;
  if (Number(decision.min ?? 1) !== 1 || Number(decision.max ?? 1) !== 1) return false;
  const reason = String(decision.reason || "").trim().toLowerCase();
  const description = String(decision.description || "").trim().toLowerCase();
  return reason === "replacement effect"
    || description.startsWith("choose which replacement effect to apply");
}

// A replacement and its explicit decline are one optional action, not an order.
export function optionalReplacementChoice(decision) {
  if (!isReplacementChoice(decision) || decision.options.length !== 2) return null;
  const decline = decision.options.find(option => /^Do not apply /i.test(String(option.description || "").trim()));
  if (!decline) return null;
  const apply = decision.options.find(option => option !== decline);
  const name = String(decline.description).trim().replace(/^Do not apply /i, "");
  const applyName = String(apply.description || "").trim().replace(/^Apply /i, "");
  if (!name || applyName !== name) return null;
  if (apply.object_id != null && decline.object_id != null
      && String(apply.object_id) !== String(decline.object_id)) return null;
  return { apply, decline, name };
}

export function presentOptionalReplacementDecision(decision) {
  const choice = optionalReplacementChoice(decision);
  if (!choice) return decision;
  return {
    ...decision,
    reason: "May ability",
    description: `You may apply the replacement effect from ${choice.name}.`,
    source_id: choice.apply.object_id ?? decision.source_id,
    source_name: choice.name,
    options: [
      { ...choice.apply, object_id: null, related_object_ids: undefined, description: "Yes" },
      { ...choice.decline, object_id: null, related_object_ids: undefined, description: "No" },
    ],
  };
}

export function isReplacementOrderingDecision(decision) {
  return isReplacementChoice(decision) && !optionalReplacementChoice(decision);
}

export function isEffectOrderingDecision(decision) {
  return isTriggerOrderingDecision(decision) || isReplacementOrderingDecision(decision);
}

export function buildEffectOrderingKey(decision) {
  if (!isEffectOrderingDecision(decision)) return "";
  return JSON.stringify([
    decision.kind, decision.player, decision.reason, decision.description,
    decision.source_id, decision.context_text,
    (decision.options || []).map((option) => [
      option.index, option.description, option.object_id, option.related_object_ids,
    ]),
  ]);
}

export const defaultEffectOrderingOrder = defaultTriggerOrderingOrder;
export const normalizeEffectOrderingOrder = normalizeTriggerOrderingOrder;

export function effectOrderingSubmitLabel(decision) {
  return isReplacementOrderingDecision(decision) ? "Apply First" : "Submit Order";
}

export function effectOrderingOptionIndices(decision, order) {
  const normalized = normalizeEffectOrderingOrder(order, decision);
  // A replacement changes the event. The engine must re-evaluate what still
  // applies before asking for another choice; only the top card is submitted.
  return isReplacementOrderingDecision(decision) ? normalized.slice(0, 1) : normalized;
}

export function buildEffectOrderingEntries(decision, order, state) {
  if (!isReplacementOrderingDecision(decision)) {
    return buildTriggerOrderingEntries(decision, order).map((entry) => ({
      ...entry,
      __effect_ordering: true,
      __effect_ordering_option_index: entry.__trigger_ordering_option_index,
    }));
  }
  const optionsByIndex = new Map((decision.options || []).map((option) => [Number(option.index), option]));
  const names = buildObjectNameById(state);
  const controllers = buildObjectControllerById(state);
  return normalizeEffectOrderingOrder(order, decision).map((optionIndex) => {
    const option = optionsByIndex.get(optionIndex);
    const sourceObjectId = triggerOrderingSourceObjectId(option);
    const { title, detail } = splitTriggerOrderingOptionText(option.description);
    const sourceController = controllers.get(String(sourceObjectId)) ?? option.controller;
    const controller = sourceController ?? decision.player;
    const controllerName = sourceController == null ? null : playerDisplayName(state?.players, sourceController);
    return {
      id: `replacement-order-${optionIndex}`,
      inspect_object_id: sourceObjectId,
      controller: Number(controller),
      name: names.get(String(sourceObjectId)) || title,
      ability_kind: "Replacement",
      ability_text: detail || title,
      targets: [],
      __effect_ordering: true,
      __replacement_ordering: true,
      __effect_ordering_option_index: optionIndex,
      // Names lead the detail so identical sources stay distinguishable even
      // when a long affected card name makes the remaining text ellipsize.
      __subtitle: controllerName ? `${controllerName} · ${detail || title}` : detail || title,
    };
  });
}
