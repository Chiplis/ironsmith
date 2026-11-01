import test from 'node:test';
import assert from 'node:assert/strict';
import {validateMechanic, compileDraftInput, draftMechanics} from '../src/lib/custom-mechanics.js';

const definition = () => validateMechanic({
  keyword: 'Veilpath', template: '— Coral debt',
  reminder: 'You may cast this spell by paying {U} and paying 2 life rather than paying its mana cost if an opponent lost life this turn.',
});
const compile = (oracleText, def = definition()) => compileDraftInput({faces:[{name:'Journeyman', oracleText}]}, [def]).faces[0];

test('a complete declared literal layout expands its configured behavior and preserves source display', () => {
  const original = 'Veilpath—Coral debt.\nDraw a card.';
  const face = compile(original);
  assert.equal(face.oracleText, definition().expansion + '\nDraw a card.');
  assert.equal(face.displayOracleText, original);
  assert.deepEqual(face.customKeywordIdentities, ['veilpath']);
  assert.equal(draftMechanics({faces:[{name:'Journeyman', oracleText:original}]}, [definition()]).length, 1);
});

test('literal layouts cannot swallow a different operand or trailing instruction', () => {
  for (const original of ['Veilpath—Crimson debt.', 'Veilpath—Coraldebt.', 'Veilpath—Coral debt extra.', 'Veilpath—Coral debt. Draw two cards.', 'Veilpath.']) {
    const face = compile(original);
    assert.equal(face.oracleText, original);
    assert.equal(face.customKeywordIdentities, undefined);
  }
});

test('literal declarations are whole lines outside quotes and never quoted granted rules', () => {
  for (const original of ['Other creatures have "Veilpath—Coral debt."', 'Enchanted creature has "\nVeilpath—Coral debt.\n"']) {
    const face = compile(original);
    assert.equal(face.oracleText, original);
    assert.equal(face.customKeywordIdentities, undefined);
  }
  assert.throws(() => validateMechanic({keyword:'Veilpath',template:'— Coral\ndebt',reminder:'Draw a card.'}), /one line/);
});

test('literal layouts do not reinterpret their suffix as an activation price or replace parameter layouts', () => {
  const activation = validateMechanic({keyword:'Veilpath',template:'— Still mode',reminder:'{T}: Add {U}.'});
  assert.equal(compile('Veilpath—Still mode.', activation).oracleText, '{T}: Add {U}.');
  const alternative = validateMechanic({keyword:'Veilpath',template:'— Still mode',reminder:'You may cast this spell for {2}{U} rather than paying its mana cost.'});
  assert.deepEqual(compile('Veilpath—Still mode.', alternative).customKeywordPayments, ['veilpath']);
  const parameter = validateMechanic({keyword:'Veilpath',template:'{Cost}',reminder:'As an additional cost to cast this spell, you may pay {Cost}. When you do, copy this spell.'});
  assert.match(compile('Veilpath {2}', parameter).oracleText, /you may pay \{2\}/);
});

test('fixed source declaration material is fully consumed without mechanic-specific branching', () => {
  const bleed = validateMechanic({keyword:'Bleed',template:'— Reappropriate blue',reminder:'You may cast this spell by exiling two blue cards from your hand and/or graveyard rather than paying its mana cost if an opponent lost life this turn.'});
  const original = 'Bleed—Reappropriate blue.';
  const face = compile(original, bleed);
  assert.equal(face.oracleText, bleed.expansion);
  assert.equal(face.displayOracleText, original);
  assert.deepEqual(face.customKeywordIdentities, ['bleed']);
  assert.ok(!face.oracleText.includes('Reappropriate'));
});
