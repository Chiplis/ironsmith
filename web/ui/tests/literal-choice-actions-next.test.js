import test from 'node:test';
import assert from 'node:assert/strict';
import {validateMechanic,compileDraftInput} from '../src/lib/custom-mechanics.js';
const definition=()=>validateMechanic({keyword:'Perform',template:'for azure',reminder:'You choose actor or scholar. If you chose actor, create a 1/1 colorless Construct Actor creature token. If you chose scholar, draw a card.'});
const expand=text=>compileDraftInput({faces:[{name:'Choice source',oracleText:text}]},[definition()]).faces[0];
test('literal choice actions expand in triggers, chapters and repeated then sequences',()=>{
 for(const source of ['When this creature enters, perform for azure.','I, II — Perform for azure.','Perform for azure, then perform for azure again.']){
  const face=expand(source);assert.equal(face.displayOracleText,source);assert.ok(face.customKeywordIdentities.includes('perform'));assert.doesNotMatch(face.oracleText,/perform for azure/i);
  assert.equal((face.oracleText.match(/You choose actor or scholar/g)||[]).length,source.includes('again')?2:1);
 }
});
test('optional named choice procedure bounds all of its branches',()=>{
 const face=expand('Whenever this creature attacks, you may perform for azure.');
 assert.match(face.oracleText,/You may do the following: You choose actor or scholar; If you chose actor,/);
 assert.match(face.oracleText,/; If you chose scholar, draw a card\./);
});
test('quoted names and costs do not invoke the action',()=>{
 for(const source of ['Search your library for a card named "Perform for azure.".','As an additional cost to cast this spell, perform for azure.']){
  assert.equal(expand(source).oracleText,source);
 }
});
