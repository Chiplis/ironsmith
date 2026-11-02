import test from 'node:test';
import assert from 'node:assert/strict';
import {validateMechanic,compileDraftInput,definitionPayment} from '../src/lib/custom-mechanics.js';
const defs=[
 {keyword:'Bloodprice',template:'{Cost}',reminder:"You may pay {Cost} and sacrifice a creature rather than pay this spell's mana cost."},
 {keyword:'Reappropriate',template:'{Type}',reminder:'Exile exactly two {Type} cards from your hand and/or graveyard.'},
].map(validateMechanic);
const expand=text=>compileDraftInput({faces:[{name:'Probe',oracleText:text}]},defs).faces[0];
test('action declaration used as an outer price composes both independent reminders',()=>{
 const face=expand('Bloodprice—Reappropriate red.');
 assert.equal(face.oracleText,"You may exile exactly two red cards from your hand and/or graveyard and sacrifice a creature rather than pay this spell's mana cost.");
 assert.deepEqual(face.customKeywordIdentities,['bloodprice']);
 assert.deepEqual(face.customKeywordPayments,['bloodprice']);
 assert.equal(face.displayOracleText,'Bloodprice—Reappropriate red.');
});
test('compound mana plus sacrifice price still identifies one alternative payment',()=>{
 const face=expand('Bloodprice {W}{W}{W}');
 assert.deepEqual(face.customKeywordPayments,['bloodprice']);
 assert.equal(definitionPayment(face.oracleText,defs[0]).cost,'{W}{W}{W} and sacrifice a creature');
});
test('generic nested action cost also binds Number and a differently named outer keyword',()=>{
 const other=[{keyword:'Trade',template:'{Cost}',reminder:"You may pay {Cost} rather than pay this spell's mana cost."},{keyword:'Dispose',template:'{Number}',reminder:'Discard {Number} cards.'}].map(validateMechanic);
 const face=compileDraftInput({faces:[{name:'Probe',oracleText:'Trade—Dispose 3'}]},other).faces[0];
 assert.equal(face.oracleText,"You may discard 3 cards rather than pay this spell's mana cost.");
});
test('arbitrary effects cannot be smuggled into a price',()=>{
 const other=[defs[0],validateMechanic({keyword:'Reappropriate',template:'{Type}',reminder:'Exile exactly two {Type} cards. Draw a card.'})];
 assert.throws(()=>compileDraftInput({faces:[{name:'Probe',oracleText:'Bloodprice—Reappropriate red'}]},other));
 assert.equal(expand('"Bloodprice—Reappropriate red"').oracleText,'"Bloodprice—Reappropriate red"');
});
test('paying slot binds a nonmana payment with the correct gerund',()=>{
 const other=[{keyword:'Paranoia',template:'{Cost}',reminder:'You may cast it by paying {Cost} rather than its mana cost.'},defs[1]].map(validateMechanic);
 const face=compileDraftInput({faces:[{name:'Probe',oracleText:'Paranoia—Reappropriate white'}]},other).faces[0];
 assert.equal(face.oracleText,'You may cast it by exiling exactly two white cards from your hand and/or graveyard rather than its mana cost.');
});
