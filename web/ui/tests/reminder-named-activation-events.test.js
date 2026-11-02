import test from 'node:test';
import assert from 'node:assert/strict';
import {compileDraftInput, validateMechanic} from '../src/lib/custom-mechanics.js';
const rule = validateMechanic({keyword:'Revealform', reminder:'{Cost}: This is the disclosure activated ability. Transform this permanent. Activate only as a sorcery.'});
test('named completion watcher independently reads operation, producer label, and source', () => {
 const draft={faces:[{name:'Back witness',oracleText:'When you Revealform Back witness, draw a card.'}]};
 const face=compileDraftInput(draft,[rule]).faces[0];
 assert.equal(face.oracleText,'When this permanent transforms caused by a disclosure activated ability, draw a card.');
 assert.deepEqual(face.customKeywordIdentities || [],[]);
 assert.equal(face.displayOracleText,draft.faces[0].oracleText);
});
test('cross-card producer label is data and unrelated transform rules are unchanged', () => {
 const text='Whenever you Revealform a creature, you gain 2 life.\nWhenever a creature you control transforms, you gain 1 life.';
 const face=compileDraftInput({faces:[{name:'Observer',oracleText:text}]},[rule]).faces[0];
 assert.match(face.oracleText,/Whenever a creature you control transforms caused by a disclosure activated ability, you gain 2 life/);
 assert.match(face.oracleText,/Whenever a creature you control transforms, you gain 1 life/);
});
test('a marker without an activation and operation cannot invent a completion event', () => {
 const text='When you Revealform Back witness, draw a card.';
 for(const reminder of ['Transform this permanent.', '{Cost}: This is the disclosure activated ability. Draw a card.']) {
  const def=validateMechanic({keyword:'Revealform',reminder});
  assert.equal(compileDraftInput({faces:[{name:'Back witness',oracleText:text}]},[def]).faces[0].oracleText,text);
 }
});
