import test from 'node:test';
import assert from 'node:assert/strict';
import {compileDraftInput,mergeMechanics} from '../src/lib/custom-mechanics.js';
const definitions=mergeMechanics([],[{keyword:'Artifact pledge',reminder:'When this permanent enters, you may tap three other untapped artifacts and/or creatures you control. If you do, perform the artifact pledge action with permanents tapped this way.',forms:{eventPaymentNoun:'artifact pledge',eventLabel:'artifact pledge'}}]);
test('an explicit passive payment noun binds only the current source watcher',()=>{
 const original="Artifact pledge\nWhen Payment Witness's artifact pledge is paid, draw a card.";
 const face=compileDraftInput({faces:[{name:'Payment Witness',oracleText:original}]},definitions).faces[0];
 assert.match(face.oracleText,/When this permanent performs the artifact pledge action, draw a card\./);
 assert.equal(face.displayOracleText,original);
 for(const text of ["When Other Witness's artifact pledge is paid, draw a card.","When Payment Witness's different debt is paid, draw a card.",'Other creatures have "When Payment Witness\'s artifact pledge is paid, draw a card."']){
  assert.equal(compileDraftInput({faces:[{name:'Payment Witness',oracleText:text}]},definitions).faces[0].oracleText,text);
 }
});
