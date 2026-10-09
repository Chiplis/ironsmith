import { getVisibleStackObjects } from './stack-targets.js';

const zones = ['battlefield', 'hand_cards', 'graveyard_cards', 'exile_cards', 'command_cards', 'ante_cards'];
export function visibleFrameCards(state) {
  return (state?.players || []).flatMap(player => zones.flatMap(zone => player[zone] || []));
}

// Presentation ids are not object ids. Only inspect ids or stable source ids
// may identify a card; never accidentally inspect a different permanent.
export function stackFramePreview(state, remembered = []) {
  const entry = state?.decision?.kind !== 'priority' && state?.resolving_stack_object
    || getVisibleStackObjects(state)[0];
  if (!entry) return null;
  const stableId = entry.source_stable_id ?? entry.stable_id;
  const matchesStable = card => stableId != null && (String(card.stable_id) === String(stableId)
    || (card.member_stable_ids || []).some(id => String(id) === String(stableId)));
  const cards = visibleFrameCards(state);
  const live = cards.find(matchesStable) || cards.find(card => entry.inspect_object_id != null
    && String(card.id) === String(entry.inspect_object_id)
    && (stableId == null || card.stable_id == null || matchesStable(card)));
  const retained = remembered.find(matchesStable) || remembered.find(card => entry.inspect_object_id != null
    && String(card.id) === String(entry.inspect_object_id)
    && (stableId == null || card.stable_id == null || matchesStable(card)));
  return { entry, card: live || retained || entry };
}
