import test from 'node:test';
import assert from 'node:assert/strict';
import { reconcilePseudoHand } from '../src/lib/pseudo-hand.js';
const card = { id: 12, name: 'Exiled spell' };
const cached = new Map([[12, { card, name: card.name, fromZone: 'exile', actions: [{ index: 2 }] }]]);
const state = (complete, cards = [card]) => ({ decision: { kind: 'priority', analysis_complete: complete }, players: [{ exile_cards: cards }] });
test('pending analysis preserves display without stale actions', () => {
  const next = reconcilePseudoHand(cached, new Map(), state(false));
  assert.equal(next.get(12).card, card);
  assert.deepEqual(next.get(12).actions, []);
  assert.equal(cached.get(12).actions.length, 1);
});
test('completed analysis removes unavailable cards but preserves current permissions', () => {
  assert.equal(reconcilePseudoHand(cached, new Map(), state(true)).size, 0);
  assert.equal(reconcilePseudoHand(cached, cached, state(true)).size, 1);
});
test('zone changes and removed objects invalidate the cached display immediately', () => {
  assert.equal(reconcilePseudoHand(cached, new Map(), state(false, [])).size, 0);
  const moved = state(false, []);
  moved.players[0].graveyard_cards = [card];
  assert.equal(reconcilePseudoHand(cached, new Map(), moved).size, 0);
});
