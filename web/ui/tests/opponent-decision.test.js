import test from 'node:test';
import assert from 'node:assert/strict';
import { buildOpponentDecisionCommand, defaultOptionSelection } from '../src/lib/opponent-decision.js';
const choose = (decision, game = {}) => buildOpponentDecisionCommand({ perspective: 0, decision: { player: 1, ...decision } }, game);

test('keeps the opening hand and advances required pregame priority choices', async () => {
  for (const kind of ['keep_opening_hand', 'continue_pregame', 'begin_game', 'pass_priority']) {
    const action_ref = { kind };
    assert.deepEqual(await choose({ kind: 'priority', actions: [
      { index: 0, action_ref: { kind: 'mulligan' } }, { index: 1, action_ref },
    ] }), { type: 'priority_action', action_index: 1, action_ref });
  }
});

test('responds to mandatory, optional, repeated and weighted modes', async () => {
  assert.deepEqual(await choose({ kind: 'select_options', min: 1, max: 1, options: [
    { index: 7, legal: false }, { index: 9, legal: true },
  ] }), { type: 'select_options', option_indices: [9] });
  assert.deepEqual(defaultOptionSelection({ min: 2, max: 2, options: [
    { index: 0, point_cost: 1 }, { index: 1, point_cost: 2 },
  ] }), [1]);
  assert.deepEqual(defaultOptionSelection({ min: 1, max: 1, options: [
    { index: 0, point_cost: 0 }, { index: 1, max_count: 0 }, { index: 2, point_cost: 1 },
  ] }), [2]);
  assert.deepEqual(defaultOptionSelection({ min: 3, max: 3, options: [
    { index: 6, repeatable: true, max_count: 3 },
  ] }), [6, 6, 6]);
  assert.deepEqual(defaultOptionSelection({ min: 0, max: 1, options: [] }), []);
  assert.equal(defaultOptionSelection({ min: 2, max: 2, options: [{ index: 0 }] }), null);
});
test('orders every item and allocates an exact distribution', async () => {
  assert.deepEqual(await choose({ kind: 'select_options', min: 2, max: 2,
    reason: 'ordering', options: [{ index: 8 }, { index: 4 }] }),
  { type: 'select_options', option_indices: [8, 4] });
  assert.deepEqual(await choose({ kind: 'select_options', min: 0, max: 4,
    description: 'Damage (assign exactly 4 total)', options: [{ index: 2, repeatable: true, max_count: 4 }] }),
  { type: 'select_options', option_indices: [2, 2, 2, 2] });
});
test('selects available cards, permits partial searches, and handles empty optional targets', async () => {
  assert.deepEqual(await choose({ kind: 'select_objects', min: 2, candidates: [
    { id: 9, legal: false }, { id: 5 }, { id: 6 },
  ] }), { type: 'select_objects', object_ids: [5, 6] });
  assert.deepEqual(await choose({ kind: 'select_objects', min: 2, allow_partial_completion: true, candidates: [] }),
    { type: 'select_objects', object_ids: [] });
  assert.deepEqual(await choose({ kind: 'targets', requirements: [
    { min_targets: 1, legal_targets: [{ kind: 'object', object: 8 }] },
    { min_targets: 0, legal_targets: [] },
    { min_targets: 1, legal_targets: [{ kind: 'player', player: 0 }] },
  ] }), { type: 'select_targets', targets: [{ kind: 'object', object: 8 }, { kind: 'player', player: 0 }] });
});
test('allocates required counters and chooses bounded numbers', async () => {
  assert.deepEqual(await choose({ kind: 'select_counters', min_total: '4', max_total: '5', options: [
    { index: 0, max_count: 3 }, { index: 1, max_count: 2 },
  ] }), { type: 'select_counters', allocations: [{ index: 0, count: 3 }, { index: 1, count: 1 }] });
  assert.deepEqual(await choose({ kind: 'number', min: 3, max: 6 }), { type: 'number_choice', value: 3 });
});
test('validates a card name and runs the payment planner', async () => {
  assert.deepEqual(await choose({ kind: 'text_input', require_known_value: true, value: 'bogus' },
    { isKnownCardName: async name => name === 'Plains' }), { type: 'text_choice', value: 'Plains' });
  const command = { type: 'mana_payment', response: { action: 'confirm', plan_id: 'p', request_hash: 'r' } };
  assert.deepEqual(await choose({ kind: 'mana_payment', request_hash: 'r' }, { analyzePayment: async () => command }), command);
});
test('never answers the human decision', async () => {
  assert.equal(await buildOpponentDecisionCommand({ perspective: 0, decision: { player: 0, kind: 'number', min: 1 } }), null);
});

test('confirms an already payable plan and cancels an impossible payment', async () => {
  const command = await buildOpponentDecisionCommand({ perspective: 0,
    mana_payment: { can_confirm: true },
    decision: { player: 1, kind: 'mana_payment', plan_id: 'p', request_hash: 'r' },
  }, { analyzePayment: () => assert.fail('payable payment does not need ranking') });
  assert.deepEqual(command, { type: 'mana_payment', response: { action: 'confirm', plan_id: 'p', request_hash: 'r' } });
  assert.deepEqual(await choose({ kind: 'mana_payment', request_hash: 'r' }, { analyzePayment: async () => false }), { type: 'cancel_decision' });
});

test('uses the native contract for related targets and aggregate object choices', async () => {
  const command = { type: 'select_objects', object_ids: [9, 11] };
  assert.deepEqual(await choose({ kind: 'select_objects', min: 0, candidates: [] }, {
    getDefaultSelectionCommand: async () => command,
  }), command);
  const targets = { type: 'select_targets', targets: [{ kind: 'player', player: 1 }, { kind: 'player', player: 0 }] };
  assert.deepEqual(await choose({ kind: 'targets', requirements: [] }, {
    getDefaultSelectionCommand: async () => targets,
  }), targets);
});

test('accepts a distribution with no recipients', () => {
  assert.deepEqual(defaultOptionSelection({ min: 0, max: 4, description: 'Damage (assign exactly 4 total)', options: [] }), []);
});
