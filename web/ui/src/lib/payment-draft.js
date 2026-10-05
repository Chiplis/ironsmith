const COLOR_ORDER = ["white", "blue", "black", "red", "green"];
const sortedIds = values => [...new Set((values || []).map(String))].sort((a, b) => a.localeCompare(b, "en", { numeric: true }));
export function paymentTransactionKey(payment) {
  return payment ? String(payment.transaction_id ?? `${payment.source_name}:${JSON.stringify(payment.pips || [])}`) : null;
}
export function activationChoice(source) {
  const colors = source.color_restriction?.length ? [...source.color_restriction].sort((a, b) => COLOR_ORDER.indexOf(a) - COLOR_ORDER.indexOf(b)) : null;
  return { source_id: String(source.source_id), ability_index: Number(source.ability_index || 0), color_restriction: colors };
}
export function sourceChoiceKey(source) {
  const kind = source.payment_kind || "mana_ability";
  return kind === "mana_ability" || kind === "manaability"
    ? JSON.stringify(activationChoice(source)) : `${String(source.source_id)}:${kind}`;
}
export function paymentPreferences(payment = {}) {
  payment ||= {};
  return {
    required_source_ids: sortedIds(payment.required_source_ids),
    required_activations: (payment.required_activations || []).map(activationChoice).sort((a, b) => sourceChoiceKey(a).localeCompare(sourceChoiceKey(b))),
    required_alternatives: (payment.required_alternatives || []).map(value => ({ source_id: String(value.source_id), payment_kind: value.payment_kind })).sort((a, b) => sourceChoiceKey(a).localeCompare(sourceChoiceKey(b))),
    excluded_source_ids: sortedIds(payment.excluded_source_ids),
    preserved_source_ids: sortedIds(payment.preserved_source_ids),
    prefer_life: Boolean(payment.prefer_life),
    required_life_pips: [...new Set(payment.required_life_pips || [])].map(Number).sort((a, b) => a - b),
    ...(payment.x_allocation == null ? {} : { x_allocation: [...payment.x_allocation] }),
  };
}
export function preferenceKey(preferences) { return JSON.stringify(paymentPreferences(preferences)); }
export function clearSourcePreferences(draft, sourceId) {
  const id = String(sourceId);
  return { ...draft, required_source_ids: draft.required_source_ids.filter(value => value !== id),
    required_activations: draft.required_activations.filter(value => value.source_id !== id),
    required_alternatives: draft.required_alternatives.filter(value => value.source_id !== id) };
}
export function selectPaymentSource(draft, source, { replace = false, step = null } = {}) {
  const id = String(source.source_id);
  let next = replace ? clearSourcePreferences(draft, id) : { ...draft };
  if (step && source.repeatable && !replace) {
    const key = sourceChoiceKey(step);
    let occurrence = 0;
    const index = next.required_activations.findIndex(value => sourceChoiceKey(value) === key && occurrence++ === (step.occurrence || 0));
    // An output menu edits this activation; Add Source creates a new one.
    if (index >= 0) next.required_activations = next.required_activations.filter((_, i) => i !== index);
  }
  next.excluded_source_ids = next.excluded_source_ids.filter(value => value !== id);
  if (["convoke", "improvise", "delve"].includes(source.payment_kind)) {
    // A permanent cannot also be tapped for mana in this payment.
    next = clearSourcePreferences(next, id);
    next.required_alternatives = [...next.required_alternatives, { source_id: id, payment_kind: source.payment_kind }];
  } else {
    next.required_alternatives = next.required_alternatives.filter(value => value.source_id !== id);
    const activation = activationChoice(source);
    const alreadySelected = next.required_activations.some(value => sourceChoiceKey(value) === sourceChoiceKey(activation));
    if (source.repeatable || !alreadySelected) next.required_activations = [...next.required_activations, activation];
  }
  return paymentPreferences(next);
}
export function excludePaymentSource(draft, sourceId) {
  const id = String(sourceId);
  return paymentPreferences({ ...clearSourcePreferences(draft, id), excluded_source_ids: [...draft.excluded_source_ids, id] });
}
export function removePaymentStep(draft, source, occurrence = 0) {
  const key = sourceChoiceKey(source);
  let found = 0;
  const index = draft.required_activations.findIndex(value => sourceChoiceKey(value) === key && found++ === occurrence);
  const count = draft.required_activations.filter(value => sourceChoiceKey(value) === key).length;
  // Remove one repeated activation; an ordinary source removal excludes the
  // permanent so automatic filling cannot immediately put it back.
  if (count > 1 && index >= 0) return { ...draft, required_activations: draft.required_activations.filter((_, i) => i !== index) };
  return excludePaymentSource(draft, source.source_id);
}
export function paymentSourceOptions(payment) {
  const options = (payment?.activation_options || []).map(option => ({ ...option, payment_kind: "mana_ability" }));
  for (const source of payment?.available_sources || []) for (const kind of source.payment_kinds || []) {
    if (["convoke", "improvise", "delve"].includes(kind)) options.push({ ...source, payment_kind: kind });
  }
  return options.filter(option => !(payment?.fixed_excluded_source_ids || []).includes(String(option.source_id)));
}
export function paymentDraftRows(payment, draft) {
  const rows = [];
  const pins = new Map();
  for (const selected of [...draft.required_activations, ...draft.required_alternatives]) {
    const key = sourceChoiceKey(selected); pins.set(key, (pins.get(key) || 0) + 1);
  }
  for (const source of payment?.planned_sources || []) {
    if (draft.excluded_source_ids.includes(String(source.source_id))) continue;
    const key = sourceChoiceKey(source), count = pins.get(key) || 0;
    if (!count && [...draft.required_activations, ...draft.required_alternatives].some(selected => selected.source_id === String(source.source_id) && sourceChoiceKey(selected) !== key)) continue;
    rows.push({ ...source, pinned: count > 0 || draft.required_source_ids.includes(String(source.source_id)), suggested: count === 0 });
    if (count) pins.set(key, count - 1);
  }
  const options = paymentSourceOptions(payment);
  for (const selected of [...draft.required_activations, ...draft.required_alternatives]) {
    const key = sourceChoiceKey(selected), count = pins.get(key) || 0;
    if (!count) continue;
    rows.push({ ...options.find(option => sourceChoiceKey(option) === key), ...selected, pinned: true, suggested: false, pending: true });
    pins.set(key, count - 1);
  }
  const counts = new Map();
  return rows.map(row => { const key = sourceChoiceKey(row), occurrence = counts.get(key) || 0; counts.set(key, occurrence + 1); return { ...row, choice_key: key, occurrence }; });
}
