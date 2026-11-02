import test from 'node:test';import assert from 'node:assert/strict';
import {validateMechanic,compileDraftInput} from '../src/lib/custom-mechanics.js';
const definition=validateMechanic({keyword:'Archive {Object}',reminder:'Exile {Object}. Put that card onto the battlefield under its owner\'s control. Tap that permanent.'});
const face=oracleText=>compileDraftInput({faces:[{name:'Scope source',oracleText}]},[definition]).faces[0];
test('optional Object actions enclose their whole multi-effect program',()=>{
 const original='Draw three cards, then you may archive a card from your hand.';const result=face(original);assert.match(result.oracleText,/Draw three cards, then You may do the following: \[Exile a card from your hand; Put that card onto the battlefield under its owner's control; Tap that permanent\]\./);assert.equal(result.displayOracleText,original);
});
test('compound foreach explicitly closes before an authored delayed sibling',()=>{
 const result=face('Archive each creature you control. At the beginning of the next end step, draw a card.');assert.match(result.oracleText,/For each creature you control, do the following: \[Exile that permanent; Put that card.*; Tap that permanent\]\. At the beginning/);
});
test('optional compound target groups retain nested program boundaries and select once',()=>{
 const result=face('You may archive up to two target creatures you control. Draw a card.');assert.equal((result.oracleText.match(/target creatures/g)||[]).length,1);assert.match(result.oracleText,/Choose up to two target creatures you control\. You may do the following: \[For each permanent chosen this way, do the following: \[Exile that permanent; Put that card.*; Tap that permanent\]\]\. Draw a card\./);
});
