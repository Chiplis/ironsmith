import test from 'node:test';
import assert from 'node:assert/strict';
import { handPriorityAction } from '../src/lib/hand-priority-action.js';
const state = {
  perspective: 0, players: [{ id: 0, hand_cards: [{ id: 7, name: 'Cycling spell' }] }],
  decision: { kind: 'priority', player: 0, analysis_complete: false, actions: [
    { kind: 'cast_spell', object_id: 7, payment_proven: false },
    { kind: 'activate_ability', object_id: 7, payment_proven: false },
    { kind: 'activate_ability', object_id: 99, payment_proven: true },
  ] },
};
test('pending payment keeps both options; completed analysis keeps only payable actions', () => {
  assert.equal(handPriorityAction(state, true, 7).label, 'Play/Activate');
  for (const [play, activate, expected] of [[true, false, 'Play'], [false, true, 'Activate'], [true, true, 'Play/Activate'], [false, false, 'No available action']]) {
    const next = structuredClone(state);
    next.decision.analysis_complete = true;
    next.decision.actions[0].payment_proven = play;
    next.decision.actions[1].payment_proven = activate;
    const result = handPriorityAction(next, true, 7);
    assert.equal(result.label, expected);
    assert.equal(result.actions.length, Number(play) + Number(activate));
  }
});
test('the hand action requires local priority and a selected card actually in hand', () => {
  assert.equal(handPriorityAction(state, false, 7).active, false);
  assert.equal(handPriorityAction(state, true, 99).label, 'Select a hand card');
  assert.equal(handPriorityAction({ ...state, decision: { ...state.decision, player: 1 } }, true, 7).active, false);
  const pregame = { ...state, decision: { ...state.decision, actions: [{ kind: 'pass_priority', label: 'Keep hand' }] } };
  assert.equal(handPriorityAction(pregame, true, 7).active, false);
});
