import test from 'node:test';
import assert from 'node:assert/strict';
import {compileDraftInput,validateMechanic} from '../src/lib/custom-mechanics.js';
const definition=validateMechanic({keyword:'Collect support {Number}',costProgram:true,forms:{eventVerb:'collect support',eventLabel:'collect support'},reminder:'Choose {Number} untapped creatures you control or cards from your graveyard, each representing a different color it has. Tap the chosen creatures on the battlefield. Exile the chosen cards from your graveyard.'});
const expand=text=>compileDraftInput({faces:[{name:'Resource Source',oracleText:text}]},[definition]).faces[0];
test('activation and additional prices attach receipt to one mandatory native cost owner',()=>{
 for(const text of ['{R}, Collect support 2: Draw a card.','As an additional cost to cast this spell, collect support X.\nDraw a card.','As an additional cost to cast this spell, you may collect support 2.\nDraw a card.','As an additional cost to cast this spell, you may collect support X.\nDraw a card.']){
  const face=expand(text);assert.match(face.oracleText,/Pay the cost "Choose (?:2|X) [^"\n]+" as the collect support action/);assert.equal(face.displayOracleText,text);assert.doesNotMatch(face.oracleText,/Choose any number|following cost is paid/);
 }
});
test('resolving optional declarations retain their distinct acceptance owner',()=>{
 assert.match(expand('You may collect support X. If you do, scry X.').oracleText,/if the following cost is paid: You may pay the cost/);
});
