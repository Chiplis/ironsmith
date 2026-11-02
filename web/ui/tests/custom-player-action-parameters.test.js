import test from 'node:test';
import assert from 'node:assert/strict';
import {validateMechanic, compileDraftInput} from '../src/lib/custom-mechanics.js';
import {expandActionParameterDeclarations} from '../src/lib/reminder-action-parameters.js';

const definition = () => validateMechanic({keyword:'Pressure {Player}', reminder:'{Player} loses 3 life unless they discard a card or sacrifice a nonland permanent.'});
test('player action repetition announces one target before the complete repeated body', () => {
  const text = expandActionParameterDeclarations('Pressure target player three times.', definition());
  assert.equal(text, 'For each target player, do the following: [that player loses 3 life unless they discard a card or sacrifice a nonland permanent; that player loses 3 life unless they discard a card or sacrifice a nonland permanent; that player loses 3 life unless they discard a card or sacrifice a nonland permanent].');
  assert.equal((text.match(/target player/g) || []).length, 1);
  assert.equal((text.match(/unless/g) || []).length, 3);
});
test('quantified repetition keeps every iteration within one player scope', () => {
  assert.equal(expandActionParameterDeclarations('Pressure each player twice.', definition()),
    'For each player, that player loses 3 life unless they discard a card or sacrifice a nonland permanent, then that player loses 3 life unless they discard a card or sacrifice a nonland permanent.');
});
test('player layouts preserve source display and custom identity without official identity', () => {
  const original='Pressure target player twice.';
  const [face]=compileDraftInput({faces:[{name:'Inquiry',oracleText:original}]},[definition()]).faces;
  assert.equal(face.displayOracleText,original);
  assert.deepEqual(face.customKeywordIdentities,['pressure']);
  assert.equal((face.oracleText.match(/target player/g)||[]).length,1);
});
test('explicit self player operands bind the same complete non-targeted action', () => {
  for (const operand of ['yourself','you']) {
    assert.equal(expandActionParameterDeclarations(`Pressure ${operand}.`,definition()),
      'you lose 3 life unless you discard a card or sacrifice a nonland permanent.');
  }
});
test('unsupported actor, repetition, quotation and cost contexts stay authored', () => {
  for (const text of ['Pressure target creature twice.','Pressure target player 33 times.','Pressure target player 0 times.','Pressure target player X times.','Pressure an opponent.','This creature has "Pressure target player twice."','As an additional cost to cast this spell, pressure target player twice.']) {
    assert.equal(expandActionParameterDeclarations(text,definition()),text);
  }
  const multi=validateMechanic({keyword:'Ritual {Player}',reminder:'{Player} draws a card. They gain 1 life.'});
  assert.equal(expandActionParameterDeclarations('Ritual each player twice.',multi),'Ritual each player twice.');
});
test('player operand validation does not broaden permanent operands or mix independent selectors', () => {
  assert.throws(()=>validateMechanic({keyword:'Pressure {Player} {Object}',reminder:'{Player} sacrifices {Object}.'}),/one player operand/);
  const object=validateMechanic({keyword:'Mark {Object}',reminder:'Put a +1/+1 counter on {Object}.'});
  assert.equal(expandActionParameterDeclarations('Mark target player.',object),'Mark target player.');
  assert.equal(expandActionParameterDeclarations('Mark target creature twice.',object),'Mark target creature twice.');
});
test('a prior independent casting cost does not suppress a following effect action', () => {
  const text='As an additional cost to cast this spell, sacrifice a creature.\nPressure target player twice.';
  const expanded=expandActionParameterDeclarations(text,definition());
  assert.match(expanded,/sacrifice a creature\.\nFor each target player, do the following:/);
  assert.equal((expanded.match(/target player/g)||[]).length,1);
});
