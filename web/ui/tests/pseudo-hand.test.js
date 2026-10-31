import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
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

test('library cache tracks only the visible playable top', () => {
  const top = { id: 14, name: 'Top spell', show_in_pseudo_hand: true };
  const previous = new Map([[14, { card: top, fromZone: 'library', actions: [{ index: 3 }] }]]);
  const snapshot = { decision: { kind: 'priority', analysis_complete: false }, players: [{ library_cards: [top] }] };
  assert.deepEqual(reconcilePseudoHand(previous, new Map(), snapshot).get(14).actions, []);
  snapshot.players[0].library_cards = [{ ...top, id: 15 }];
  assert.equal(reconcilePseudoHand(previous, new Map(), snapshot).size, 0);
  snapshot.players[0].library_cards = [{ ...top, show_in_pseudo_hand: false }];
  assert.equal(reconcilePseudoHand(previous, new Map(), snapshot).size, 0);
  snapshot.players[0].library_cards = [];
  assert.equal(reconcilePseudoHand(previous, new Map(), snapshot).size, 0);
});

test('hand builder surfaces library permissions and attaches current cast actions', () => {
  const source = readFileSync(new URL('../src/components/board/HandZone.jsx', import.meta.url), 'utf8');
  const start = source.indexOf('function buildPlayableMaps(');
  const end = source.indexOf('\n// A hovered hand card', start);
  const build = new Function('samePlayerId', `${source.slice(start, end)}; return buildPlayableMaps;`)((a, b) => String(a) === String(b));
  const top = { id: 14, name: 'Top spell', show_in_pseudo_hand: true, pseudo_hand_glow_kind: 'extra' };
  const player = { id: 0, hand_cards: [], library_cards: [top] };
  const state = { perspective: 0, players: [player], decision: { kind: 'priority', player: 0, actions: [] } };
  const extra = build(state, player).extraPlayable.get(14);
  assert.equal(extra.card, top);
  assert.equal(extra.fromZone, 'library');
  assert.equal(extra.ownerId, 0);
  assert.deepEqual(extra.actions, []);
  const action = { kind: 'cast_spell', object_id: 14, from_zone: 'library', index: 2 };
  state.decision.actions = [action];
  assert.deepEqual(build(state, player).extraPlayable.get(14).actions, [action]);
  state.decision.actions = [];
  player.library_cards = [{ ...top, show_in_pseudo_hand: false }];
  assert.equal(build(state, player).extraPlayable.size, 0);
});
