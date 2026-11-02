import test from 'node:test';
import assert from 'node:assert/strict';
import { expandRecipientActions } from '../src/lib/reminder-recipient-actions.js';
import { compileDraftInput, validateMechanic } from '../src/lib/custom-mechanics.js';
const body = 'It becomes a legendary Rebel creature with base power and toughness {Number}/{Number} in addition to its other types until end of turn.';
const definition = validateMechanic({keyword: 'Rally {Number}', reminder: body});
test('recipient scalar bodies bind optional, target and source operands independently of keyword', () => {
  for (const object of ['this creature', 'target artifact', 'another target attacking creature you control', 'Volunteer']) {
    const input = `When Volunteer attacks, you may have ${object} rally 4.`;
    assert.equal(expandRecipientActions(input, definition, 'Volunteer'), `When Volunteer attacks, you may have ${object} become a legendary Rebel creature with base power and toughness 4/4 in addition to its other types until end of turn.`);
  }
});
test('group animation preserves the declared group and duration', () => {
  assert.equal(expandRecipientActions('Any number of target creatures you control each rally 5 until end of turn.', definition), 'Any number of target creatures you control each become a legendary Rebel creature with base power and toughness 5/5 in addition to its other types until end of turn.');
});
test('unbound subjects, quoted actions and mismatched literal scalars stay opaque', () => {
  for (const input of ['You may have an opponent rally 3.', 'You may have every permanent rally 3.', 'The name is "you may have this creature rally 3."']) assert.equal(expandRecipientActions(input, definition), input);
  const fixed = {keyword:'Rally', reminder:body.replaceAll('{Number}', '4')};
  const input = 'You may have this creature rally 5.';
  assert.equal(expandRecipientActions(input, fixed), input);
});
test('draft discovery emits keyword identity and preserves authored display', () => {
  const text = 'Whenever this creature attacks, you may have it rally 4.';
  const result = compileDraftInput({faces:[{name:'Volunteer',oracleText:text}]}, [definition]).faces[0];
  assert.match(result.oracleText, /base power and toughness 4\/4/);
  assert.deepEqual(result.customKeywordIdentities, ['rally']);
  assert.equal(result.displayOracleText, text);
});
