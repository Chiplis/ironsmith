// A finite group of already-stacked triggers. Source identity is intentionally
// excluded: matching abilities on different permanents may share the group.
const same = (a, b) => a != null && b != null && String(a) === String(b);
const key = entry => entry?.ability_kind?.toLowerCase() === 'triggered' && entry.automation_identity
  ? JSON.stringify([entry.controller, entry.automation_identity]) : null;
export function resolvableTriggerGroup(state) {
  if (state?.decision?.kind !== 'priority' || state.resolving_stack_object || state.loop_shortcut) return null;
  const stack = state?.stack_objects || [];
  const identity = key(stack[0]);
  if (!identity) return null;
  const entries = [];
  for (const entry of stack) {
    if (key(entry) !== identity) break;
    entries.push(String(entry.id));
  }
  return entries.length > 1 && new Set(entries).size === entries.length ? { identity, entries } : null;
}
export function createResolveGroup(state) {
  const group = resolvableTriggerGroup(state);
  return group ? { ...group, turn: state.turn_number, actor: state.perspective,
    current: null, step: 0, choices: {} } : null;
}
const belongs = (group, entry) => entry && key(entry) === group.identity && group.entries.includes(String(entry.id));
export function observeResolveGroup(group, state) {
  if (!group || state.game_over || group.turn !== state.turn_number) return null;
  const resolving = state.resolving_stack_object;
  const stack = state.stack_objects || [];
  if (resolving && !belongs(group, resolving)) return null;
  if (!resolving && stack[0] && !belongs(group, stack[0])) return null;
  if (!belongs(group, resolving) && !stack.some(entry => belongs(group, entry))) return null;
  if (resolving && group.current !== String(resolving.id)) {
    group.current = String(resolving.id);
    group.step = 0;
  }
  return group;
}
function canonical(value) {
  if (Array.isArray(value)) return value.map(canonical);
  if (value && typeof value === 'object') return Object.fromEntries(Object.keys(value).sort()
    .filter(key => value[key] !== undefined).map(key => [key, canonical(value[key])]));
  return value;
}
const commandKinds = {
  select_objects: 'select_objects', select_options: 'select_options',
  number: 'number_choice', text_input: 'text_choice', targets: 'select_targets',
  select_counters: 'select_counters', mana_payment: 'mana_payment',
};
function promptKey(group, state) {
  const decision = state.decision;
  if (!group || !belongs(group, state.resolving_stack_object)
    || !same(decision?.player, group.actor) || !commandKinds[decision.kind]) return null;
  // Unopened hidden cards must never compare equal just because both say
  // "Hidden card". A hidden reference is an exact commitment, not a label.
  if (decision.candidates?.some(card => /^hidden card$/i.test(card.name || '') && !card.hidden_ref)) return null;
  const { source_id: _sourceId, source_name: _sourceName, ...prompt } = decision;
  const viewed = state.viewed_cards;
  const visibleCards = viewed && !viewed.inspector_only && !viewed.inspectorOnly
    ? { viewer: viewed.viewer, subject: viewed.subject, zone: viewed.zone,
      visibility: viewed.visibility, cards: viewed.cards, card_ids: viewed.card_ids } : null;
  const payment = decision.kind === 'mana_payment'
    ? { details: state.mana_payment, players: (state.players || []).map(player => ({
      id: player.id, mana_pool: player.mana_pool, battlefield: player.battlefield,
    })) } : null;
  return JSON.stringify(canonical({ step: group.step, prompt, visibleCards, payment }));
}
export function rememberGroupChoice(group, state, command) {
  const signature = promptKey(group, state);
  if (signature && command.type === commandKinds[state.decision.kind]) {
    group.choices[signature] = structuredClone(command);
  }
  if (belongs(group, state.resolving_stack_object) && state.decision?.kind !== 'priority') group.step++;
}
export function replayGroupChoice(group, state) {
  const signature = promptKey(group, state);
  const command = signature && group.choices[signature];
  return command ? structuredClone(command) : null;
}
