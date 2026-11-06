import test from 'node:test';
import assert from 'node:assert/strict';
import {compileDraftInput,mergeMechanics} from '../src/lib/custom-mechanics.js';
const definitions=mergeMechanics([],[{keyword:'Sight',reminder:'Whenever this creature attacks, draw a card.',forms:{eventVerb:'observe',eventLabel:'sighted card'}}]);
const expand=oracleText=>compileDraftInput({faces:[{name:'A Reader',oracleText}]},definitions).faces[0];
test('explicit event form maps source-qualified watcher without granting keyword identity',()=>{
 const original='Whenever A Reader observes a creature card, you gain 1 life.';
 const face=expand(original);
 assert.equal(face.oracleText,'Whenever this permanent performs the sighted card action with a creature card, you gain 1 life.');
 assert.equal(face.displayOracleText,original);
 assert.equal(face.customKeywordIdentities,undefined);
});
test('quoted names and unrelated verbs do not create action watchers',()=>{
 for(const original of ['Search your library for a card named "Whenever A Reader observes a creature card".','Whenever another creature observes a creature card, you gain 1 life.','Whenever A Reader reveals a creature card, you gain 1 life.'])assert.equal(expand(original).oracleText,original);
});
test('event label is explicit and independent of declaration name',()=>{
 const other=mergeMechanics([],[{keyword:'Material sight',reminder:'Whenever this artifact attacks, draw a card.',forms:{eventVerb:'observe',eventLabel:'sighted card'}}]);
 const face=compileDraftInput({faces:[{name:'Another Reader',oracleText:'Whenever Another Reader observes an artifact card, put a +1/+1 counter on it.'}]},other).faces[0];
 assert.match(face.oracleText,/performs the sighted card action with an artifact card/);
});
