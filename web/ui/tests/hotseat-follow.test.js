import test from 'node:test';
import assert from 'node:assert/strict';
import { createPregameFollower, hotseatFollowTarget } from '../src/lib/hotseat-follow.js';

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
  assert.equal(follow(state(0, 1, 'begin_game')), 1);
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
