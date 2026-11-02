import test from 'node:test';
import assert from 'node:assert/strict';
import { compileDraftInput, mergeMechanics } from '../src/lib/custom-mechanics.js';
const definitions = mergeMechanics([], [{keyword: 'growwise', reminder: 'Whenever this creature attacks, if it has two or fewer +1/+1 counters on it, put a +1/+1 counter on it.'}]);
const expand = oracleText => compileDraftInput({faces: [{name: 'Mentor', oracleText}]}, definitions).faces[0];
test('bare triggered grants retain recipient-relative self and source display', () => {
  for (const text of ['Enchanted creature has growwise.', 'Other nontoken creatures you control have growwise.', 'Target creature gains growwise until end of turn.']) {
    const face = expand(text);
    assert.ok(face.oracleText.includes('custom keyword "growwise", and "Whenever this creature attacks,'));
    assert.equal(face.displayOracleText, text);
    assert.equal(face.customKeywordIdentities, undefined);
    if (text.includes('until')) assert.match(face.oracleText, /" until end of turn\./);
  }
});
test('quoted card names remain opaque even when their text resembles an observer', () => {
  for (const text of [
    'Search your library for a card named "has growwise".',
    'Search your library for a card named "Whenever a creature with growwise attacks, draw a card.".',
  ]) {
    const face = expand(text);
    assert.equal(face.oracleText, text);
    assert.equal(face.customKeywordIdentities, undefined);
  }
});
test('an explicitly granted observer body binds its predicate without granting the observed keyword', () => {
  const text = 'Target creature gains "Whenever a creature with growwise attacks, draw a card." until end of turn.';
  const face = expand(text);
  assert.equal(face.oracleText, 'Target creature gains "Whenever a creature with custom keyword "growwise" attacks, draw a card." until end of turn.');
  assert.equal(face.displayOracleText, text);
  assert.equal(face.customKeywordIdentities, undefined, 'the observer source does not acquire the observed identity');
  assert.ok(!face.oracleText.includes(definitions[0].reminder), 'the recipient gains only the authored observer, not growwise itself');
  assert.equal((face.oracleText.match(/draw a card/g) || []).length, 1, 'the granted observer body is preserved once');
});
test('a declared keyword observer reads identity without granting the keyword or its program', () => {
  const text = 'Whenever a creature with growwise attacks, draw a card.';
  const face = expand(text);
  assert.equal(face.oracleText, 'Whenever a creature with custom keyword "growwise" attacks, draw a card.');
  assert.equal(face.displayOracleText, text);
  assert.equal(face.customKeywordIdentities, undefined);
  assert.ok(!face.oracleText.includes(definitions[0].reminder), 'observing the keyword does not grant its triggered program');
  const unrelated = 'Whenever a creature with unfamiliar craft attacks, draw a card.';
  assert.equal(expand(unrelated).oracleText, unrelated, 'only the supplied finite definition vocabulary binds an observer');
});
test('complete multi-sentence programs and temporary grants preserve every source-relative operation', () => {
  const bodies = [
    'When this creature dies, if it was attacking, return it to the battlefield transformed under your control.',
    "Whenever this creature attacks, create a tapped and attacking token that's a copy of this creature, except it's a 1/1 Illusion instead of its other creature types. Exile that token at the beginning of the next end step.",
    "When this creature dies, return it to the battlefield with a haze counter on it. For as long as it has a haze counter on it, it loses all abilities and can't block.",
  ];
  for (const body of bodies) {
    const defs = mergeMechanics([], [{keyword: 'Second chance', reminder: body}]);
    const original = 'Other attacking creatures you control gain second chance until end of turn.';
    const face = compileDraftInput({faces: [{name: 'Grantor', oracleText: original}]}, defs).faces[0];
    assert.equal(face.oracleText, `Other attacking creatures you control gain custom keyword "second chance", and "${body}" until end of turn.`);
    assert.equal(face.displayOracleText, original);
    assert.equal(face.customKeywordIdentities, undefined);
  }
});
test('spell-cost programs can be granted while casting without changing the grantor identity',()=>{
 const defs=mergeMechanics([],[{keyword:'Battle craft',reminder:'This spell costs {1} less to cast for each creature you attacked with this turn.'}]);
 const original='{T}: The next creature spell you cast this turn gains battle craft as you cast it.';
 const face=compileDraftInput({faces:[{name:'Teacher',oracleText:original}]},defs).faces[0];
 assert.match(face.oracleText,/gains custom keyword "battle craft", and "This spell costs/);assert.match(face.oracleText,/as you cast it\.$/);assert.equal(face.customKeywordIdentities,undefined);
});
test('official ability lists and token programs compose with custom triggered or static programs',()=>{
 const bodies=['Whenever this creature becomes the target of a spell, sacrifice it.','Damage dealt to this creature also causes its controller to lose that much life.'];
 for(const body of bodies){const defs=mergeMechanics([],[{keyword:'Wavering',reminder:body}]);
 for(const original of ['Until end of turn, gain control of target creature and it gains haste and wavering. Untap it.','Create a 1/1 red Lizard creature token with haste and wavering.']){
 const face=compileDraftInput({faces:[{name:'Source',oracleText:original}]},defs).faces[0];assert.ok(face.oracleText.includes(`haste, custom keyword "wavering", and "${body}"`));assert.equal(face.customKeywordIdentities,undefined);assert.equal(face.displayOracleText,original);
 }}
});
test('nested quoted static recipients use independent native inner quote delimiters',()=>{
 const defs=mergeMechanics([],[{keyword:'Wavering',reminder:'Damage dealt to this creature also causes its controller to lose that much life.'}]);
 const original='Create a 4/4 red Elemental creature token with haste and “Creatures blocking this creature have wavering.”';
 const face=compileDraftInput({faces:[{name:'Creator',oracleText:original}]},defs).faces[0];assert.ok(face.oracleText.includes("have custom keyword 'wavering', and 'Damage dealt to this creature also causes its controller to lose that much life.'"));assert.equal(face.customKeywordIdentities,undefined);
});
test('a token custom grant composes before an existing quoted ability',()=>{
 const body='Whenever this creature deals combat damage to a player, you gain 1 life.';const defs=mergeMechanics([],[{keyword:'Rewarding',reminder:body}]);const original='Create a 1/1 creature token with rewarding and “This creature can\'t block.”';const face=compileDraftInput({faces:[{name:'Creator',oracleText:original}]},defs).faces[0];assert.ok(face.oracleText.includes(`custom keyword "rewarding", and "${body}" and “This creature can't block.”`));assert.equal(face.customKeywordIdentities,undefined);
});

test('past custom membership is retained identity, including an omitted repeated pronoun', () => {
  for (const [original, expected] of [
    ['Return target creature to its owner\'s hand. If it was tapped or had growwise, scry 2.', 'If it was tapped or it had custom keyword "growwise", scry 2.'],
    ['If it had growwise, draw a card.', 'If it had custom keyword "growwise", draw a card.'],
  ]) {
    const face=expand(original);assert.ok(face.oracleText.includes(expected));assert.equal(face.displayOracleText,original);
    assert.equal(face.customKeywordIdentities,undefined);assert.ok(!face.oracleText.includes('Whenever this creature attacks'));
  }
  const printed='Search your library for a card named "it had growwise".';
  assert.equal(expand(printed).oracleText,printed);
});
