import { buildObjectLocationById, decisionObjectLocation } from './decision-object-meta.js';
import { isObjectChosen } from './object-selection.js';

// Compare every visible property except the physical copy's identifiers and
// library position. Unknown properties (including future modifications) keep
// copies separate rather than silently treating them as interchangeable.
const copyFields = new Set(['id', 'stable_id', 'position', 'selection_identity', 'hidden_ref']);
function signature(value, top = true) {
  if (Array.isArray(value)) return value.map(item => signature(item, false));
  if (!value || typeof value !== 'object') return value;
  return Object.fromEntries(Object.keys(value).sort().filter(key => !top || !copyFields.has(key))
    .map(key => [key, signature(value[key], false)]));
}
export function groupDecisionCards(candidates, state, decision = {}) {
  const locations = buildObjectLocationById(state);
  const cards = new Map();
  for (const card of state?.viewed_cards?.cards || []) cards.set(String(card.id), card);
  for (const player of state?.players || []) {
    for (const zone of ['hand', 'library']) {
      for (const card of player[`${zone}_cards`] || []) cards.set(String(card.id), card);
    }
  }
  const groups = [], byKey = new Map();
  for (const candidate of candidates) {
    const location = decisionObjectLocation(locations, candidate.id, candidate);
    const card = cards.get(String(candidate.id));
    const index = (decision.candidates || candidates).findIndex(item => String(item.id) === String(candidate.id));
    const alternatives = (decision.selection_alternatives || []).map(option => option.candidate_indices?.includes(index) ?? false);
    // A name alone is insufficient evidence, especially for hidden cards.
    const canGroup = ['hand', 'library'].includes(location?.zone) && location.owner != null
      && card && typeof card.oracle_text === 'string' && candidate.name && candidate.name.toLowerCase() !== 'hidden card'
      && !card.face_down && !candidate.face_down;
    const key = canGroup ? JSON.stringify([location, signature(card), signature(candidate), alternatives]) : null;
    let group = key == null ? null : byKey.get(key);
    if (!group) {
      group = { candidate, members: [] };
      groups.push(group);
      if (key != null) byKey.set(key, group);
    }
    group.members.push(candidate);
  }
  return groups;
}
export function nextGroupedCardChoice(members, selected, max) {
  const chosen = members.filter(card => isObjectChosen(selected, card.id));
  const next = members.find(card => !isObjectChosen(selected, card.id));
  if (next && selected.length < max) return next.id;
  return chosen.at(-1)?.id ?? null;
}
