import test from 'node:test';
import assert from 'node:assert/strict';
import {validateMechanic,compileDraftInput} from '../src/lib/custom-mechanics.js';
const definition=validateMechanic({keyword:'Enspirit',reminder:'Exile a creature you control. Perform the enspirit action with creatures exiled this way.'});
const expand=text=>compileDraftInput({faces:[{name:'Observer',oracleText:text}]},[definition]).faces[0];
test('affected participle trigger reads the named operation without gaining its keyword',()=>{
 const face=expand('Whenever this creature becomes enspirited, draw a card.');
 assert.equal(face.oracleText,'Whenever this creature participates in the enspirit action, draw a card.');
 assert.equal(face.customKeywordIdentities,undefined);
});
test('same independent reader handles another operation and explicit participle',()=>{
 const def=validateMechanic({keyword:'Archive',forms:{participle:'archived'},reminder:'Exile a card from your graveyard. Perform the archive action with cards exiled this way.'});
 const face=compileDraftInput({faces:[{name:'Observer',oracleText:'Whenever one or more cards become archived, you gain 1 life.'}]},[def]).faces[0];
 assert.equal(face.oracleText,'Whenever one or more cards participate in the archive action, you gain 1 life.');
});
test('status prose, quoted ability text and unrelated words remain unchanged',()=>{
 for(const text of ['This is an enspirited creature.','"Whenever this creature becomes enspirited, draw a card."','Whenever this creature becomes inspired, draw a card.'])assert.equal(expand(text).oracleText,text);
});
