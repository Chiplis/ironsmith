import test from 'node:test';
import assert from 'node:assert/strict';
import {compileDraftInput, validateMechanic} from '../src/lib/custom-mechanics.js';
const reminder='You may pay {Cost} and exile this card from your hand as a sorcery. When you next resolve a spell, cast this card from exile without paying its mana cost.';
const definition=validateMechanic({keyword:'HarmonizeLater',template:'{Cost}',reminder});
test('derived preparation grants bind recipient mana cost without adding an activation',()=>{
  const source='Each instant and sorcery card in your hand has HarmonizeLater. Its HarmonizeLater cost is equal to its mana cost reduced by {2}.';
  const face=compileDraftInput({faces:[{name:'Grantor audit',oracleText:source}]},[definition]).faces[0];
  assert.equal(face.oracleText,'Each instant and sorcery card in your hand has "You may pay its mana cost reduced by {2} and exile this card from your hand as a sorcery. When you next resolve a spell, cast this card from exile without paying its mana cost. This is the harmonizelater preparation ability.".');
  assert.equal(face.displayOracleText,source);
  assert.deepEqual(face.customKeywordIdentities ?? [],[], 'the grantor has no recipient keyword identity, whether empty metadata is omitted or retained');
  assert.deepEqual(face.customKeywordPayments ?? [],[], 'the recipient preparation is not a payment or identity of its grantor');
  assert.doesNotMatch(face.oracleText,/\{[^}]+\}\s*:/,'the grant remains a preparation program, not a mana activation');
  assert.equal((face.oracleText.match(/This is the harmonizelater preparation ability\./g)||[]).length,1,'the source-declared name belongs to the single quoted recipient program');
});
test('quoted reference and unrelated price clause do not gain a preparation grant',()=>{
  const source='Name a card named "has HarmonizeLater. Its HarmonizeLater cost is equal to its mana cost reduced by {2}."';
  const face=compileDraftInput({faces:[{name:'Opaque name audit',oracleText:source}]},[definition]).faces[0];
  assert.equal(face.oracleText,source);
});
