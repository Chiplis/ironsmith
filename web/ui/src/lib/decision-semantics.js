// Prefer the engine's discriminant. Legacy snapshots use narrowly scoped
// prompt/reason contracts, never card names or quoted option text.
const reasonOf = decision => String(decision?.reason || '').trim().toLowerCase();
const promptOf = decision => String(decision?.description || '').trim().toLowerCase();

export function isOrderingDecision(decision) {
  if (decision?.choice_kind) return ['order', 'trigger_order'].includes(decision.choice_kind);
  return ['ordering', 'order triggers', 'order blockers', 'order attackers'].includes(reasonOf(decision));
}

export function isDistributionDecision(decision) {
  if (decision?.choice_kind) return decision.choice_kind === 'distribution';
  return !!decision?.distribute || reasonOf(decision) === 'distribute';
}

export function isCounterOptionsDecision(decision) {
  if (decision?.choice_kind) return false; // Current counters use select_counters.
  return !!decision?.counter_type || reasonOf(decision) === 'remove counters';
}

export function isPaymentDecision(decision) {
  if (decision?.kind !== 'select_options') return false;
  if (decision.choice_kind && !['options', 'hybrid_payment'].includes(decision.choice_kind)) return false;
  if (['modal choice', 'next cost'].includes(reasonOf(decision))) return false;
  const prompt = promptOf(decision);
  return /^pay\b/.test(prompt) || /^choose (?:how to pay|payment)\b/.test(prompt)
    || decision.choice_kind === 'hybrid_payment' || reasonOf(decision) === 'mana payment';
}

export function isSpellCastFlowDecision(decision) {
  if (decision?.kind !== 'select_options') return false;
  if (decision.choice_kind && decision.choice_kind !== 'options') return false;
  if (reasonOf(decision) === 'modal choice') return false;
  const prompt = promptOf(decision);
  return /^(?:cast|play)\b/.test(prompt)
    || ['choose which spell to cast', 'choose a casting price', 'choose the alternative casting cost'].includes(prompt);
}
