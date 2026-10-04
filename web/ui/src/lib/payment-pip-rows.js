import { paymentDraftRows, paymentSourceOptions, paymentPreferences, sourceChoiceKey, selectPaymentSource, removePaymentStep } from './payment-draft.js';

const COLORS = { W: 'white', U: 'blue', B: 'black', R: 'red', G: 'green', C: 'colorless' };
const stepKey = source => `${sourceChoiceKey(source)}:${source.occurrence || 0}`;
const isMana = source => !source.payment_kind || ['mana_ability', 'manaability'].includes(source.payment_kind);
const outputCount = source => Object.values(COLORS).reduce((sum, color) => sum + Number(source.expected_mana?.[color] || 0), 0);
const generic = pip => pip.some(symbol => /^\d+$/.test(symbol));
function produces(source, pip) {
  if (!isMana(source)) return source.payment_kind === 'convoke' || generic(pip);
  return generic(pip) ? outputCount(source) > 0 : pip.some(symbol => Number(source.expected_mana?.[COLORS[symbol]] || 0) > 0);
}

/** Cost pips, including floating mana and gaps, rather than one row per activation.
 * An activation producing multiple mana units may cover multiple rows, but is
 * still a single planner step. Match constrained pips before generic pips.
 */
export function paymentPipRows(payment, draft) {
  const sources = paymentDraftRows(payment, draft);
  const resources = [{ pool: { ...payment?.pool_before }, source: null }, ...sources.filter(isMana).map(source => ({ pool: { ...source.expected_mana }, source }))];
  const pips = payment?.payment_pips || (payment?.pips || []).flatMap(pip => pip.length === 1 && /^\d+$/.test(pip[0]) ? Array.from({ length: Number(pip[0]) }, () => ['1']) : [pip]);
  const rows = pips.map((pip, pipId) => ({ pip, pip_id: pipId, source: null, allocation: payment?.allocations?.find(value => value.pip_id === pipId) }));
  const usedAlternatives = new Set();
  for (const row of [...rows].sort((a, b) => Number(generic(a.pip)) - Number(generic(b.pip)))) {
    const allocation = row.allocation;
    if (draft.required_life_pips.includes(row.pip_id) || allocation?.payment_kind === 'life') { row.kind = 'life'; continue; }
    if (allocation?.source_id != null && allocation.payment_kind !== 'mana') {
      const source = sources.find(value => String(value.source_id) === String(allocation.source_id) && value.payment_kind === allocation.payment_kind);
      if (source && !usedAlternatives.has(stepKey(source))) { row.source = source; row.kind = source.payment_kind; usedAlternatives.add(stepKey(source)); continue; }
    }
    const preferred = allocation?.symbol && COLORS[allocation.symbol] ? [allocation.symbol] : [];
    const symbols = preferred.length ? preferred : generic(row.pip) ? Object.keys(COLORS) : row.pip.filter(symbol => COLORS[symbol]);
    const resource = resources.find(value => symbols.some(symbol => Number(value.pool[COLORS[symbol]] || 0) > 0));
    if (resource) {
      const symbol = symbols.find(symbol => Number(resource.pool[COLORS[symbol]] || 0) > 0);
      resource.pool[COLORS[symbol]] -= 1;
      row.source = resource.source; row.kind = resource.source ? 'source' : 'pool'; row.symbol = symbol;
    } else {
      const source = sources.find(value => !isMana(value) && !usedAlternatives.has(stepKey(value)) && produces(value, row.pip));
      if (source) { row.source = source; row.kind = source.payment_kind; usedAlternatives.add(stepKey(source)); }
      else row.kind = 'unassigned';
    }
  }
  return rows;
}

/** Capacity counts activations, not displayed pips. All output branches of an
 * ability share the same budget; a bounded repeatable ability is never infinite.
 * Full cross-source affordability/restrictions remain the planner's decision.
 */
export function paymentOptionsForPip(payment, draft, row, rows) {
  const sources = paymentDraftRows(payment, draft);
  const otherPips = rows.filter(value => value !== row && value.source);
  const releasingStep = row.source && !otherPips.some(value => stepKey(value.source) === stepKey(row.source));
  const retained = sources.filter(source => !(releasingStep && stepKey(source) === stepKey(row.source)));
  return paymentSourceOptions(payment).filter(option => {
    if (!produces(option, row.pip)) return false;
    const sameSource = retained.filter(source => String(source.source_id) === String(option.source_id));
    if (!isMana(option)) return sameSource.length === 0;
    if (sameSource.some(source => !isMana(source))) return false;
    const capacity = Number.isFinite(option.max_activations) ? option.max_activations : option.repeatable ? 2 : 1;
    const uses = sameSource.filter(source => source.ability_index === option.ability_index).length;
    // Remaining output from a chosen multi-mana activation needs no extra use.
    const spareOutput = sameSource.some(source => sourceChoiceKey(source) === sourceChoiceKey(option)
      && otherPips.filter(value => stepKey(value.source) === stepKey(source)).length < outputCount(source));
    return spareOutput || (uses < capacity && (option.repeatable || sameSource.length === 0));
  });
}

export function selectPaymentPipSource(draft, option, row, rows) {
  const source = row.source;
  if (source && sourceChoiceKey(source) === sourceChoiceKey(option)) return selectPaymentSource(draft, option, { step: source, replace: !option.repeatable });
  const others = rows.filter(value => value !== row && value.source);
  let next = draft;
  if (source && !others.some(value => stepKey(value.source) === stepKey(source))) {
    // Removing a repeated automatic step must not exclude its other occurrences.
    if (others.some(value => String(value.source.source_id) === String(source.source_id))) {
      const key = sourceChoiceKey(source);
      let occurrence = 0;
      const index = next.required_activations.findIndex(value => sourceChoiceKey(value) === key && occurrence++ === source.occurrence);
      if (index >= 0) next = { ...next, required_activations: next.required_activations.filter((_, i) => i !== index) };
    } else next = removePaymentStep(next, source, source.occurrence);
  }
  if (option.payment_kind === 'life') return paymentPreferences({ ...next, required_life_pips: [...next.required_life_pips.filter(id => id !== row.pip_id), row.pip_id] });
  const retainedSteps = [...new Map(others.filter(value => String(value.source.source_id) === String(option.source_id)).map(value => [stepKey(value.source), value.source])).values()];
  // Preserve the already displayed uses before asking for an additional use.
  // Otherwise one new required activation could merely pin the old suggestion.
  const counts = new Map();
  for (const step of retainedSteps) {
    const key = sourceChoiceKey(step);
    const count = (counts.get(key) || 0) + 1; counts.set(key, count);
    if (next.required_activations.filter(value => sourceChoiceKey(value) === key).length < count) next = selectPaymentSource(next, { ...step, repeatable: count > 1 });
  }
  const existing = others.find(value => sourceChoiceKey(value.source) === sourceChoiceKey(option)
    && others.filter(other => stepKey(other.source) === stepKey(value.source)).length < outputCount(value.source));
  // Pin an existing multi-output activation once; selecting its spare output
  // must not demand a second tap/activation.
  return selectPaymentSource({ ...next, required_life_pips: next.required_life_pips.filter(id => id !== row.pip_id) }, option, existing ? { step: existing.source, replace: !option.repeatable } : { replace: !option.repeatable });
}
