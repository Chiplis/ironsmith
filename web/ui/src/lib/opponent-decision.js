import { initialCounterDraft, parseCounterAllocationChoice } from './counter-choice.js';
import { improvePayment } from './payment-analysis.js';
import { priorityCommandForAction } from './sync-commands.js';

// Solve the option point budget rather than assuming every mode costs one.
// A greedy selection can strand a required choice even when a solution exists.
export function defaultOptionSelection(decision) {
  const options = (decision.options || []).filter(option => option.legal !== false);
  const distribution = /assign exactly \d+ total/.test(decision.description || '');
  if (distribution && options.length === 0) return [];
  const min = distribution ? decision.max : decision.min;
  const max = decision.max;
  if (min === 0) return [];
  const reachable = new Map([[0, []]]);
  for (const option of options) {
    const cost = option.point_cost ?? 1;
    if (cost <= 0) continue;
    const limit = option.repeatable ? Math.min(option.max_count ?? max, Math.floor(max / cost)) : Math.min(option.max_count ?? 1, 1);
    for (const [total, selected] of [...reachable]) {
      for (let count = 1; count <= limit && total + count * cost <= max; count++) {
        const next = total + count * cost;
        if (!reachable.has(next)) reachable.set(next, [...selected, ...Array(count).fill(option.index)]);
        if (next >= min) return reachable.get(next);
      }
    }
  }
  return null;
}

export async function buildOpponentDecisionCommand(state, game) {
  const decision = state?.decision;
  if (!decision || Number(decision.player) === Number(state.perspective)) return null;
  if (['select_objects', 'targets'].includes(decision.kind) && game?.getDefaultSelectionCommand) {
    return game.getDefaultSelectionCommand();
  }
  switch (decision.kind) {
    case 'priority': {
      const actions = (decision.actions || []).filter(action => action.legal !== false);
      const action = actions.find(action => ['keep_opening_hand', 'continue_pregame', 'begin_game'].includes(action.action_ref?.kind))
        || actions.find(action => action.kind === 'pass_priority' || action.action_ref?.kind === 'pass_priority')
        || actions[0];
      return action ? priorityCommandForAction(action) : null;
    }
    case 'select_options': {
      const indices = defaultOptionSelection(decision);
      return indices && { type: 'select_options', option_indices: indices };
    }
    case 'select_objects': {
      const legal = (decision.candidates || []).filter(card => card.legal !== false);
      const count = decision.allow_partial_completion ? Math.min(decision.min, legal.length) : decision.min;
      if (legal.length < count) return null;
      return { type: 'select_objects', object_ids: legal.slice(0, count).map(card => card.id) };
    }
    case 'select_counters':
      return parseCounterAllocationChoice(decision, initialCounterDraft(decision))?.command || null;
    case 'targets': {
      const targets = [];
      for (const requirement of decision.requirements || []) {
        if (requirement.legal_targets.length < requirement.min_targets) return null;
        targets.push(...requirement.legal_targets.slice(0, requirement.min_targets).map(target =>
          target.kind === 'player' ? { kind: 'player', player: target.player } : { kind: 'object', object: target.object }));
      }
      return { type: 'select_targets', targets };
    }
    case 'number': return { type: 'number_choice', value: decision.min };
    case 'text_input': {
      let value = String(decision.value || '').trim();
      if (decision.require_known_value) {
        if (!value || !await game.isKnownCardName(value)) {
          if (await game.isKnownCardName('Plains')) value = 'Plains';
          else {
            const names = await game.autocompleteCardNames('a', 1);
            value = names[0];
          }
        }
      } else value ||= 'Creature';
      return value ? { type: 'text_choice', value } : null;
    }
    case 'mana_payment': {
      if (state.mana_payment?.can_confirm) return {
        type: 'mana_payment', response: { action: 'confirm', plan_id: decision.plan_id, request_hash: decision.request_hash },
      };
      const command = await improvePayment({ game, token: `opponent:${decision.request_hash}`, isCurrent: () => true });
      return command || { type: 'cancel_decision' };
    }
    default: return null;
  }
}
