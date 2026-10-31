import { createResolveGroup, observeResolveGroup, rememberGroupChoice, replayGroupChoice } from './resolve-group.js';
import { priorityCommandForAction } from './sync-commands.js';
import { normalizePhaseStep } from './constants.js';
import { priorityStepKey } from './priority-stops.js';

export const YIELD_LABELS = {
  responses: 'Pass until a response is needed this turn',
  turn: 'Yield through this turn',
  no_play: 'Yield when no play is possible',
  until: 'Yield until selected stop',
  stack: 'Yield through this stack',
};
const id = value => value == null ? null : String(value);
const same = (a, b) => a != null && b != null && id(a) === id(b);
const stackIds = state => (state.stack_objects || []).map(entry => id(entry.id));
const windowKey = state => JSON.stringify([state.turn_number, state.active_player, state.phase,
  state.step, state.combat_damage_step, state.decision?.player, stackIds(state)]);

// Native structural identity, never a rendered or translated label. The name
// scope also pins the controller and ability, so other abilities on that card
// cannot inherit an unrelated answer.
export function abilityKey(entry, scope = 'source') {
  if (!entry?.ability_kind || !entry.automation_identity) return null;
  const source = scope === 'name' ? entry.name : entry.source_stable_id;
  if (source == null) return null;
  return JSON.stringify([entry.controller, scope, source, entry.automation_identity]);
}
export function booleanChoice(decision, yes) {
  if (decision?.kind !== 'select_options' || decision.choice_kind !== 'boolean') return null;
  const index = yes ? 1 : 0; // Native Boolean decision contract.
  const option = decision.options?.find(option => option.index === index && option.legal !== false);
  return option ? { type: 'select_options', option_indices: [index] } : null;
}

export function createPriorityPolicy({ stops } = {}) {
  const listeners = new Set();
  const seats = new Map();
  let revision = 0;
  let lastTurn = null;
  const publish = () => { revision++; listeners.forEach(listener => listener()); };
  const seat = player => {
    const key = id(player);
    if (!seats.has(key)) seats.set(key, { yield: null, fullControl: false, held: null,
      maintain: false, holdNextAction: false, rules: [], groupTriggers: false, resolveGroup: null });
    return seats.get(key);
  };
  const observe = state => {
    if (!state) return;
    if (state.game_over || (lastTurn != null && state.turn_number < lastTurn)) seats.clear();
    lastTurn = state.turn_number ?? lastTurn;
    for (const current of seats.values()) {
      if (current.resolveGroup) {
        current.resolveGroup = observeResolveGroup(current.resolveGroup, state);
        if (!current.resolveGroup) current.held = windowKey(state);
      }
      if (state.game_over) { current.yield = null; current.maintain = false; }
      current.rules = current.rules.filter(rule => rule.turn == null || rule.turn === state.turn_number);
      const intent = current.yield;
      if (!intent) continue;
      if (['responses', 'turn', 'stack'].includes(intent.mode) && intent.turn !== state.turn_number) current.yield = null;
      if (intent.mode === 'until') {
        const here = intent.key.startsWith('step:') ? `step:${priorityStepKey(state)}` : `phase:${normalizePhaseStep(state.phase, state.step)}`;
        const atTarget = (intent.anyPlayer || same(intent.player, state.active_player)) && intent.key === here;
        if (!atTarget) intent.left = true;
        if (atTarget && intent.left && same(state.decision?.player, intent.actor)) {
          current.yield = null;
          // A future stop wins even if its persistent preference is off.
          current.held = windowKey(state);
        }
      }
      if (intent.mode === 'stack' && (!(state.stack_objects || []).length
        || stackIds(state).some(key => !intent.stack.includes(key)))) {
        current.yield = null;
        current.held = windowKey(state);
      }
    }
  };
  const ruleFor = (state, entry, field) => seat(state.perspective).rules.find(rule =>
    rule.key === abilityKey(entry, rule.scope) && rule[field] != null);
  const evaluate = (state, canAct = state?.decision?.can_act ?? null) => {
    observe(state);
    const decision = state?.decision;
    if (!decision || state.game_over || !same(decision.player, state.perspective) || state.loop_shortcut) return null;
    const current = seat(state.perspective);
    if (current.fullControl && !current.resolveGroup) return null;
    if (current.resolveGroup && decision.kind !== 'priority') return replayGroupChoice(current.resolveGroup, state);
    if (!current.resolveGroup && state.viewed_cards && !state.viewed_cards.inspector_only && !state.viewed_cards.inspectorOnly) return null;
    if (decision.kind !== 'priority') {
      // Only an explicitly saved Boolean preference can answer a resolution.
      const resolving = state.resolving_stack_object;
      if (resolving && same(decision.source_id, resolving.inspect_object_id)) {
        const rule = ruleFor(state, resolving, 'answer');
        if (rule) return booleanChoice(decision, rule.answer === 'yes');
      }
      if (current.groupTriggers && decision.choice_kind === 'trigger_order') {
        const options = decision.options || [];
        if (options.length > 1 && options.every(option => option.automation_identity
          && option.automation_identity === options[0].automation_identity && option.legal !== false)) {
          return { type: 'select_options', option_indices: options.map(option => option.index) };
        }
      }
      return null;
    }
    if (current.maintain) { current.held = windowKey(state); current.maintain = false; }
    if (current.held === windowKey(state)) return null;
    const pass = decision.actions?.find(action => action.kind === 'pass_priority'
      && action.legal !== false && (!action.label || action.label === 'Pass priority'));
    if (!pass) return null;
    const command = priorityCommandForAction(pass);
    if (current.resolveGroup) return command;
    const intent = current.yield;
    if (intent?.mode === 'no_play') {
      // Native analysis excludes mana abilities themselves, but includes
      // spells and non-mana abilities payable using available mana sources.
      return canAct === false && decision.analysis_complete !== false ? command : null;
    }
    if (intent && ['turn', 'until', 'stack'].includes(intent.mode)) return command;
    const top = state.stack_objects?.[0];
    if (Number(state.stack_size || 0) > 0) {
      if (!top) return null;
      if (ruleFor(state, top, 'yield')) return command;
      return same(top.controller, state.perspective) ? command : null;
    }
    if (intent?.mode === 'responses') return command;
    return stops?.stopReason(state, canAct) ? null : command;
  };
  return {
    subscribe(listener) { listeners.add(listener); return () => listeners.delete(listener); },
    getSnapshot: () => revision,
    status(state) { observe(state); return seat(state?.perspective); },
    observe,
    evaluate,
    needsAnalysis(state) { return seat(state?.perspective).yield?.mode === 'no_play' || stops?.awaitsCanAct(state); },
    startGroup(state) {
      const group = createResolveGroup(state);
      if (!group || state.game_over || !same(state.decision?.player, state.perspective)) return false;
      const current = seat(state.perspective);
      current.resolveGroup = group; current.yield = null; current.held = null; current.maintain = false;
      stops?.resume(state); publish(); return true;
    },
    beforeAutomaticCommand(command, state) {
      observe(state);
      const current = seat(state.perspective);
      const checkpoint = { ...current, rules: [...current.rules], resolveGroup: structuredClone(current.resolveGroup) };
      if (current.resolveGroup) rememberGroupChoice(current.resolveGroup, state, command);
      return checkpoint;
    },
    start(mode, state, target = {}) {
      if (!state || state.game_over) return;
      const current = seat(state.perspective);
      current.held = null; current.maintain = false; current.resolveGroup = null;
      const here = target.key?.startsWith('step:') ? `step:${priorityStepKey(state)}` : `phase:${normalizePhaseStep(state.phase, state.step)}`;
      current.yield = { mode, turn: state.turn_number, actor: state.perspective,
        left: (!target.anyPlayer && !same(target.player, state.active_player)) || target.key !== here,
        start: windowKey(state), stack: stackIds(state), ...target };
      stops?.resume(state);
      publish();
    },
    beforeCommand(command, state, { maintain = false } = {}) {
      observe(state);
      const current = seat(state.perspective);
      const checkpoint = { ...current, rules: [...current.rules], resolveGroup: structuredClone(current.resolveGroup) };
      if (current.resolveGroup) rememberGroupChoice(current.resolveGroup, state, command);
      current.held = null;
      const actionKind = command.action_ref?.kind
        || state.decision?.actions?.find(action => action.index === command.action_index)?.kind;
      if (command.type === 'cancel_decision' || (command.type === 'priority_action' && actionKind !== 'pass_priority')) current.resolveGroup = null;
      if (command.type === 'priority_action' && (actionKind === 'activate_ability'
        || actionKind === 'cast_spell' || actionKind?.startsWith('cast_'))) {
        current.maintain ||= maintain || current.holdNextAction;
        current.holdNextAction = false;
      }
      if (current.yield?.mode === 'responses' && !(command.type === 'priority_action'
        && (command.action_ref?.kind === 'pass_priority' || decisionPassIndex(state) === command.action_index))) current.yield = null;
      publish();
      return checkpoint;
    },
    rollback(checkpoint, state) { if (checkpoint) seats.set(id(state.perspective), checkpoint); publish(); },
    clear(state) {
      const current = seat(state?.perspective);
      current.resolveGroup = null; current.yield = null; current.rules = []; current.groupTriggers = false; current.maintain = false; current.holdNextAction = false;
      current.held = state ? windowKey(state) : null;
      stops?.holdCurrentPriority(state);
      publish();
    },
    holdNextAction(enabled, state) { seat(state?.perspective).holdNextAction = enabled; publish(); },
    setFullControl(enabled, state) { seat(state?.perspective).fullControl = enabled; publish(); },
    setGroupTriggers(enabled, state) { seat(state?.perspective).groupTriggers = enabled; publish(); },
    setRule(entry, scope, values, state, untilEndOfTurn = false) {
      const key = abilityKey(entry, scope);
      if (!key) return;
      const current = seat(state.perspective);
      current.held = null;
      stops?.resume(state);
      for (const [field, value] of Object.entries(values)) {
        if (!['yield', 'answer'].includes(field)) continue;
        current.rules = current.rules.filter(rule => rule.key !== key || rule[field] == null);
        current.rules.push({ key, scope, name: entry.name, [field]: value,
          turn: untilEndOfTurn ? state.turn_number : null });
      }
      publish();
    },
    reset() { seats.clear(); lastTurn = null; publish(); },
  };
}
const decisionPassIndex = state => state.decision?.actions?.find(action => action.kind === 'pass_priority')?.index;

// Keep groups adjacent without deciding the relative order of different
// triggers. Unclassified/targeted entries remain independent singleton groups.
export function groupedTriggerOrder(decision, order) {
  if (decision?.choice_kind !== 'trigger_order') return order;
  const options = new Map((decision.options || []).map(option => [option.index, option]));
  const groups = new Map();
  for (const index of order) {
    const key = options.get(index)?.automation_identity || `single:${index}`;
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key).push(index);
  }
  return [...groups.values()].flat();
}
export function moveTriggerGroup(decision, order, position, direction) {
  const grouped = groupedTriggerOrder(decision, order);
  const identities = new Map((decision.options || []).map(option => [option.index, option.automation_identity || `single:${option.index}`]));
  const groups = [];
  for (const index of grouped) {
    if (!groups.length || identities.get(groups.at(-1)[0]) !== identities.get(index)) groups.push([]);
    groups.at(-1).push(index);
  }
  const from = groups.findIndex(group => group.includes(order[position]));
  const to = from + direction;
  if (from < 0 || to < 0 || to >= groups.length) return grouped;
  [groups[from], groups[to]] = [groups[to], groups[from]];
  return groups.flat();
}
export function yieldDescription(intent, state) {
  if (!intent) return '';
  if (intent.mode !== 'until') return YIELD_LABELS[intent.mode];
  if (intent.anyPlayer) return 'Yielding until the next end step';
  const player = state?.players?.find(player => same(player.id, intent.player));
  const phase = String(intent.key || '').split(':')[1]?.replace(/([a-z])([A-Z])/g, '$1 $2');
  return `Yielding until ${player?.name || 'the selected player'}’s ${phase || 'selected stop'}`;
}
