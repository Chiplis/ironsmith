import test from 'node:test';
import assert from 'node:assert/strict';
import {compileDraftInput,validateMechanic} from '../src/lib/custom-mechanics.js';
const def=validateMechanic({keyword:'Glitch',template:'{Cost}',reminder:"As you cast this spell, you may pay the following additional cost any number of times: {Cost}.\nFor each time you pay this spell's optional additional cost, remove one bracketed text fragment from it."});
test('repeatable additional costs retain custom payment attribution and Boolean references',()=>{
 for(const cost of ['{1}','Discard a card','Pay 3 life','Sacrifice a creature']) {
  const result=compileDraftInput({faces:[{name:'Spell',oracleText:`Glitch—${cost}\nDraw a card. If this spell was glitched, draw a card.`}]},[def]).faces[0];
  assert.match(result.oracleText,/this spell's additional cost was paid/i);
  assert.deepEqual(result.customKeywordPayments,['glitch']);
 }
});
test('payment counts bind the triggering spell without declaring identity on the reader',()=>{
 const source='Whenever you cast a glitched spell, this creature gets +2/+0 until end of turn for each time that spell was glitched.';
 const result=compileDraftInput({faces:[{name:'Reader',oracleText:source}]},[def]).faces[0];
 assert.match(result.oracleText,/a spell using glitch/i);
 assert.match(result.oracleText,/for each time that spell's additional glitch cost was paid/i);
 assert.equal(result.customKeywordIdentities,undefined);
 const quoted=compileDraftInput({faces:[{name:'Quoted',oracleText:'It gains "Draw a card for each time that spell was glitched."'}]},[def]).faces[0];
 assert.match(quoted.oracleText,/that spell was glitched/);
});
