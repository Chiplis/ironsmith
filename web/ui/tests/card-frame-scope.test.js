import test from 'node:test';
import assert from 'node:assert/strict';
import { cardNeedsFrame } from '../src/lib/card-frame-scope.js';

const state = {perspective: 0, players: [
  {id: 0, hand_cards: [{id: 1}], battlefield: [{id: 2, member_ids: [3]}], graveyard_cards: [{id: 4}], exile_cards: [{id: 5}], command_cards: [{id: 6}]},
  {id: 1, hand_cards: [{id: 7}], battlefield: [{id: 8}]},
]};
test('prepares only our hand and either battlefield, including grouped cards', () => {
  for (const id of [1, 2, 3, 8]) assert.equal(cardNeedsFrame(state, String(id)), true);
  for (const id of [null, undefined, 4, 5, 6, 7, 9]) assert.equal(cardNeedsFrame(state, id), false);
});
test('scope follows zone changes and perspective changes', () => {
  assert.equal(cardNeedsFrame({...state, perspective: 1}, 1), false);
  assert.equal(cardNeedsFrame({...state, perspective: 1}, 7), true);
  assert.equal(cardNeedsFrame({players: [{id: 0, hand_cards: [{id: 1}]}]}, 1), false);
  assert.equal(cardNeedsFrame({perspective: 0, players: [{id: 0, graveyard_cards: [{id: 2}]}]}, 2), false);
  assert.equal(cardNeedsFrame(null, 1), false);
});
