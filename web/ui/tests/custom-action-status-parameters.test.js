import test from 'node:test';
import assert from 'node:assert/strict';
import {validateMechanic, compileDraftInput} from '../src/lib/custom-mechanics.js';
import {expandActionParameterDeclarations} from '../src/lib/reminder-action-parameters.js';
const definition = () => validateMechanic({keyword:'Pressure {Player}', reminder:'{Player} loses 3 life unless they discard a card or sacrifice a nonland permanent.'});
test('a self player operand preserves actor and subject agreement', () => {
  assert.equal(expandActionParameterDeclarations('Pressure yourself.', definition()), 'you lose 3 life unless you discard a card or sacrifice a nonland permanent.');
});
test('a controller operand remains tied to the referred permanent', () => {
  assert.equal(expandActionParameterDeclarations("Pressure that permanent's controller.", definition()), "that permanent's controller loses 3 life unless they discard a card or sacrifice a nonland permanent.");
});
test('typed actions expand in explicitly granted triggered abilities', () => {
  const original = 'Until end of turn, this land becomes a 2/2 creature with “Whenever this creature attacks or blocks, pressure yourself.” It\'s still a land.';
  const face = compileDraftInput({faces:[{name:'Animated Field',oracleText:original}]}, [definition()]).faces[0];
  assert.match(face.oracleText, /Whenever this creature attacks or blocks, you lose 3 life unless you discard a card or sacrifice a nonland permanent/);
  assert.equal(face.displayOracleText,original);
});
test('self operands in costs and quoted names stay untouched', () => {
  for (const source of ['As an additional cost to cast this spell, pressure yourself.', 'Search your library for a card named "Pressure yourself.".']) {
    assert.equal(expandActionParameterDeclarations(source,definition()),source);
  }
});
test('a reused target operand binds once across counter and persistent grant', () => {
  const definition = validateMechanic({keyword:'Designate {Object}', reminder:'Put a bounty counter on {Object}. For as long as {Object} has a bounty counter on it, {Object} has "When this permanent is put into a graveyard from the battlefield, each opponent draws a card."'});
  const text = expandActionParameterDeclarations('Designate target permanent.',definition);
  assert.equal((text.match(/target permanent/g)||[]).length,1);
  assert.match(text,/Choose target permanent\. Put a bounty counter on that permanent\./);
  assert.match(text,/For as long as that permanent has a bounty counter on it, that permanent has/);
});
