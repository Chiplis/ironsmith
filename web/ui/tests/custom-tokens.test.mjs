import test from 'node:test';
import assert from 'node:assert/strict';
import { applyCustomTokens, draftTokens, validateToken } from '../src/lib/custom-tokens.js';

const face = { name: 'Bobble', manaCost: null, cardTypes: ['Creature'], subtypes: ['Goblin'], colorIndicator: ['R'], power: '1', toughness: '1', oracleText: 'Haste' };
const bobble = { face, input: face };
const card = text => ({ faces: [{ name: 'Bobble Maker', oracleText: text }] });
const compiled = (text, tokens = [bobble]) => {
  const draft = card(text);
  return applyCustomTokens(draft, draftTokens(draft, tokens)).faces[0];
};

test('creation references become the named token shape; display keeps the authored text', () => {
  const result = compiled('When this creature enters, create two Bobble tokens.');
  assert.equal(result.oracleText, 'When this creature enters, create two 1/1 colorless creature tokens named Bobble.');
  assert.equal(result.displayOracleText, 'When this creature enters, create two Bobble tokens.');
  assert.equal(compiled('{T}: Create a tapped Bobble token.').oracleText, '{T}: Create a tapped 1/1 colorless creature token named Bobble.');
  assert.equal(compiled('Create X Bobble tokens, where X is your life total.').oracleText,
    'Create X 1/1 colorless creature tokens named Bobble, where X is your life total.');
});

test('other references name the token; quoted text and bare names are untouched', () => {
  assert.equal(compiled('Sacrifice a Bobble: Draw a card.').oracleText, 'Sacrifice a token named Bobble: Draw a card.');
  assert.equal(compiled('This creature gets +1/+1 for each Bobble you control.').oracleText,
    'This creature gets +1/+1 for each token named Bobble you control.');
  const quoted = 'Creatures you control have "Bobble tokens are great."';
  assert.equal(compiled(quoted).oracleText, quoted);
  const draft = card('Bobble is a name.');
  assert.deepEqual(draftTokens(draft, [bobble]), []);
});

test('referenced tokens travel with the input in dependency order', () => {
  const gem = { name: 'Gem', manaCost: null, cardTypes: ['Artifact'], oracleText: 'When this token is put into a graveyard, create a Bobble token.' };
  const draft = card('Create a Gem token.');
  const tokens = draftTokens(draft, [bobble, { face: gem, input: gem }]);
  assert.deepEqual(tokens.map(token => token.name), ['Bobble', 'Gem']);
  const input = applyCustomTokens(draft, tokens);
  assert.equal(input.faces[0].oracleText, 'Create a colorless artifact token named Gem.');
  assert.equal(input.customTokenFaces[1].oracleText, 'When this token is put into a graveyard, create a 1/1 colorless creature token named Bobble.');
  assert.equal(applyCustomTokens(card('Draw a card.'), []).customTokenFaces, undefined);
});

test('predefined, malformed and costed tokens are rejected', () => {
  assert.throws(() => validateToken({ face: { ...face, name: 'Treasure' }, input: { ...face, name: 'Treasure' } }), /predefined/);
  assert.throws(() => validateToken({ face: { ...face, name: '2 Bobble' }, input: { ...face, name: '2 Bobble' } }), /cannot name a token/);
  assert.throws(() => validateToken({ face, input: { ...face, manaCost: '{1}' } }), /mana cost/);
});
