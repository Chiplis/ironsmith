import test from 'node:test';
import assert from 'node:assert/strict';
import { setCatalogRows, filterCatalogRows } from '../src/lib/build-catalog.js';
import { validateMechanic } from '../src/lib/custom-mechanics.js';

test('catalog preserves mechanic variants and links only the cards using each definition', () => {
  const a = validateMechanic({ keyword: 'Kindle', reminder: 'Draw a card.' });
  const b = validateMechanic({ keyword: 'Kindle', reminder: 'Draw two cards.' });
  const subtype = { singular: 'emberkin', plural: 'emberkin', family: 'Creature' };
  const card = (name, mechanic) => ({ draft: { faces: [{ name, subtypes: ['Emberkin'] }], customMechanics: [mechanic], customSubtypes: [subtype] } });
  const value = { name: 'Embers', customCards: [card('First', a), card('Second', b)], customMechanics: [a] };
  const before = structuredClone(value);
  const rows = setCatalogRows(value);
  assert.equal(rows.mechanics.length, 2);
  assert.deepEqual(rows.mechanics.map(row => row.cards), [['First'], ['Second']]);
  assert.deepEqual(rows.subtypes[0].cards, ['First', 'Second']);
  assert.equal(filterCatalogRows(rows.mechanics, 'two cards')[0].definition, rows.mechanics[1].definition);
  assert.equal(filterCatalogRows(rows.cards, 'embers').length, 2);
  assert.deepEqual(value, before);
});
