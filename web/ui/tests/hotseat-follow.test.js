import test from 'node:test';
import assert from 'node:assert/strict';
import { createPregameFollower, emptyPregameAction, hotseatFollowTarget } from '../src/lib/hotseat-follow.js';

const state = (perspective, player, action, active = 2) => ({
  perspective, active_player: active,
  decision: { kind: 'priority', player, actions: [{ action_ref: { kind: action } }] },
});

test('pregame visits every deciding seat then restores the starting player once', () => {
  const follow = createPregameFollower();
  assert.equal(follow(state(2, 2, 'keep_opening_hand')), null);
  assert.equal(follow(state(2, 0, 'take_mulligan')), 0);
  assert.equal(follow({ ...state(0, 1), decision: { kind: 'select_objects', player: 1 } }), 1);
  assert.equal(follow(state(1, 0, 'use_pregame_action')), 0);
  assert.equal(follow(state(0, 1, 'begin_game')), null);
  assert.equal(follow(state(1, 2, 'pass_priority')), 2);
  assert.equal(follow(state(2, 0, 'pass_priority', 0)), null);
});

test('first priority already viewed consumes the handoff without a later unwanted switch', () => {
  const follow = createPregameFollower();
  follow(state(2, 2, 'begin_game'));
  assert.equal(follow(state(2, 2, 'pass_priority')), null);
  assert.equal(follow(state(2, 0, 'pass_priority', 0)), null);
});

test('ordinary play still follows turns without following off-turn priority', () => {
  assert.equal(hotseatFollowTarget(state(0, 2, 'pass_priority')), 2);
  assert.equal(hotseatFollowTarget(state(2, 0, 'pass_priority')), null);
});

test('goldfished seats and blocker steps with no legal blocker are never handed over', () => {
  assert.equal(hotseatFollowTarget(state(0, 2, 'pass_priority'), new Set([2])), null);
  assert.equal(hotseatFollowTarget(state(0, 2, 'pass_priority'), new Set([1])), 2);
  const blockers = valid => ({ perspective: 0, active_player: 0,
    decision: { kind: 'blockers', player: 1, blocker_options: [{ attacker: 7, valid_blockers: valid }] } });
  assert.equal(hotseatFollowTarget(blockers([])), null);
  assert.equal(hotseatFollowTarget(blockers([{ id: 9 }])), 1);
  assert.equal(hotseatFollowTarget(blockers([{ id: 9 }]), new Set([1])), null);
});


test('empty opening steps skip every seat, including the viewed seat and final Begin Game', () => {
  const follow = createPregameFollower();
  for (const player of [2, 0, 1]) {
    const snapshot = state(2, player, player === 1 ? 'begin_game' : 'continue_pregame');
    assert.equal(emptyPregameAction(snapshot), snapshot.decision.actions[0]);
    assert.equal(follow(snapshot), null);
  }
  assert.equal(follow(state(1, 2, 'pass_priority')), 2);
});

test('mulligans, available card actions and nested choices are never empty steps', () => {
  for (const kind of ['keep_opening_hand', 'take_mulligan', 'use_pregame_action', 'pass_priority']) {
    assert.equal(emptyPregameAction(state(2, 0, kind)), null);
  }
  const snapshot = state(2, 0, 'continue_pregame');
  snapshot.decision.actions.push({ action_ref: { kind: 'use_pregame_action' } });
  assert.equal(emptyPregameAction(snapshot), null);
  const follow = createPregameFollower();
  assert.equal(follow(snapshot), 0);
  const choice = { ...snapshot, decision: { kind: 'select_objects', player: 0 } };
  assert.equal(emptyPregameAction(choice), null);
  assert.equal(follow(choice), 0);
});

test('legacy action kinds support skipping empty steps', () => {
  const snapshot = state(2, 0, 'continue_pregame');
  snapshot.decision.actions = [{ kind: 'begin_game' }];
  assert.equal(emptyPregameAction(snapshot), snapshot.decision.actions[0]);
});
