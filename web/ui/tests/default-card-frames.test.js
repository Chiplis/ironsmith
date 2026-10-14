import test from 'node:test';
import assert from 'node:assert/strict';
import { existsSync } from 'node:fs';
import { defaultCardFrame, defaultCardFrameKey } from '../src/lib/default-card-frames.js';
test('all 32 color sets have a bundled, text-free default frame', () => {
  for (let mask = 0; mask < 32; mask++) {
    const colors = [...'WUBRG'].filter((_, i) => mask & (1 << i));
    const frame = defaultCardFrame({ cards: [{ colors }], typeLine: 'Artifact Creature' });
    assert.ok(frame, colors.join(''));
    assert.equal(frame.defaultFrame, colors.join('') || 'C');
    assert.equal(frame.flavorText, '');
    assert.equal(frame.printing, null);
    assert.ok(existsSync(new URL(`../public/card-frames/default/${frame.defaultFrame}.webp`, import.meta.url)));
  }
});
test('identity derivation and distinct colorless materials select the frame', () => {
  assert.equal(defaultCardFrameKey({ cards: [{ colors: [] }], manaCost: '{G}', typeLine: 'Creature' }), 'G');
  assert.equal(defaultCardFrameKey({ cards: [{ characteristic_signature: 'colors:blue red' }], typeLine: 'Creature' }), 'UR');
  assert.equal(defaultCardFrameKey({ typeLine: 'Basic Land — Forest' }), 'G');
});
test('special layouts retain their existing renderer before metadata arrives', () => {
  for (const layout of ['class', 'split', 'adventure', 'transform', 'modal_dfc', 'leveler', 'prototype']) {
    assert.equal(defaultCardFrameKey({ cards: [{ layout }], typeLine: 'Creature' }), null);
  }
  for (const typeLine of ['Enchantment — Class', 'Planeswalker — Jace', 'Battle — Siege', 'Enchantment — Room']) {
    assert.equal(defaultCardFrameKey({ typeLine }), null);
  }
  assert.equal(defaultCardFrameKey({ cards: [{ name: 'Bonecrusher Giant' }], typeLine: 'Creature — Giant' }), null);
  assert.equal(defaultCardFrameKey({ cards: [{ name: 'Delver of Secrets' }], typeLine: 'Creature — Human Wizard' }), null);
});

test('fallback identity overrides live colors and includes rules mana without reminder text', () => {
  const key = (card, printing = null) => defaultCardFrameKey({ cards: [card], printing, typeLine: card.type_line || 'Artifact' });
  assert.equal(key({ colors: [], color_identity: ['R', 'U'] }), 'UR');
  assert.equal(key({ colors: ['G'], color_identity: [], mana_cost: '{G}' }), 'C');
  assert.equal(key({ colors: [] }, { color_identity: ['G'] }), 'G');
  assert.equal(key({ colors: [], mana_cost: '{2}', oracle_text: '{T}: Add {W} or {U}.' }), 'WU');
  assert.equal(key({ colors: [], mana_cost: '{R}', oracle_text: 'Devoid' }), 'R');
  assert.equal(key({ colors: ['B'], oracle_text: 'Extort (You may pay {W/B}.)' }), 'B');
  assert.equal(key({ type_line: 'Land — Island Mountain', colors: [] }), 'UR');
});

test('Sagas select identity frames despite their nonstandard layout and name', () => {
  assert.equal(defaultCardFrameKey({ cards: [{ name: "Urza's Saga", color_identity: [] }], typeLine: 'Enchantment Land — Urza’s Saga' }), 'land');
  assert.equal(defaultCardFrameKey({ cards: [{ name: 'Binding the Old Gods', layout: 'saga', color_identity: ['B', 'G'] }], typeLine: 'Enchantment — Saga' }), 'BG');
  assert.equal(defaultCardFrameKey({ cards: [{ layout: 'transform', color_identity: ['G'] }], typeLine: 'Enchantment — Saga' }), 'G');
  assert.equal(defaultCardFrameKey({ cards: [{ layout: 'saga', colors: ['R'], color_identity: ['U'] }], typeLine: 'Enchantment' }), 'U');
});

test('a land adds the colors of its current intrinsic mana to its identity frame', () => {
  assert.equal(defaultCardFrameKey({ cards: [{ color_identity: [], intrinsic_mana: ['G'] }], typeLine: 'Legendary Land — Forest' }), 'G');
  assert.equal(defaultCardFrameKey({ cards: [{ color_identity: [] }], typeLine: 'Legendary Land' }), 'land');
});
