import test from 'node:test';
import assert from 'node:assert/strict';
import { arenaPermanentKind, partitionArenaBattlefield } from '../src/lib/mobile-arena.js';
import { solveMobileBattleLayout } from '../src/lib/mobile-battle-layout.js';

test('live creature characteristics take precedence over resource and enchantment lanes', () => {
  for (const type_line of ['Artifact Creature — Golem', 'Enchantment Creature — God', 'Land Creature — Forest Dryad']) {
    assert.equal(arenaPermanentKind({type_line}), 'creature');
  }
  assert.equal(arenaPermanentKind({lane:'lands', power_toughness:'3/3'}), 'creature');
});

test('all permanent types stay in their appropriate mobile area', () => {
  const cards = ['Creature', 'Land', 'Planeswalker', 'Battle', 'Artifact', 'Enchantment', 'Enchantment — Saga'].map((type_line,id)=>({id,type_line}));
  const rows = partitionArenaBattlefield(cards);
  assert.deepEqual(rows.frontCards.map(c=>c.id), [0]);
  assert.deepEqual(rows.backCards.map(c=>c.id), [1]);
  assert.deepEqual(rows.specialCards.map(c=>c.id), [2,3]);
  assert.deepEqual(rows.supportCards.map(c=>c.id), [4,5,6]);
  assert.equal(arenaPermanentKind(cards[6]), 'saga');
});

test('lands leave more vertical room for combat on short and tall mobile screens', () => {
  for (const viewportHeight of [255,305,369,422]) {
    const layout = solveMobileBattleLayout({viewportHeight, viewportWidth:734});
    assert.ok(layout.landHeight < layout.cardHeight * .7);
    assert.ok(layout.cardHeight >= 60);
    assert.ok(layout.totalHeight <= viewportHeight);
  }
});
