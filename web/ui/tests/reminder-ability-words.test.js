import test from 'node:test';
import assert from 'node:assert/strict';
import {compileDraftInput,validateMechanic} from '../src/lib/custom-mechanics.js';
const definition=validateMechanic({keyword:'Resonance',declarationKind:'ability-word',reminder:'Externally triggered ability named resonance'});
const expand=(oracleText,definitions=[definition])=>compileDraftInput({faces:[{name:'Heading witness',oracleText}]},definitions).faces[0];
test('generic ability-word headings preserve complete independent effect bodies',()=>{
 const text='Resonance — Attach this permanent to target creature. Draw a card.\nFlying\nResonance — Scry 2.';
 const face=expand(text);
 assert.equal(face.oracleText,'Externally triggered ability named resonance — Attach this permanent to target creature. Draw a card.\nFlying\nExternally triggered ability named resonance — Scry 2.');
 assert.equal(face.displayOracleText,text);assert.deepEqual(face.customKeywordIdentities,['resonance']);assert.equal(definition.declarationKind,'ability-word');
});
test('class references and quoted headings confer no source identity',()=>{
 for(const text of ['Search your library for a card with resonance, reveal it, put it into your hand, then shuffle.','This permanent has “Resonance — Draw a card.”','Resonance without its declaration delimiter']){
  const face=expand(text);assert.equal(face.oracleText,text);assert.equal(face.customKeywordIdentities,undefined);
 }
});
test('independent labels never select one anothers programs',()=>{
 const echo=validateMechanic({keyword:'Echoing',declarationKind:'ability-word',reminder:'Externally triggered ability named echoing'});
 assert.equal(expand('Resonance — Draw a card.\nEchoing — You gain 1 life.',[definition,echo]).oracleText,'Externally triggered ability named resonance — Draw a card.\nExternally triggered ability named echoing — You gain 1 life.');
});
test('ability-word declarations reject omitted body and incompatible operand templates',()=>{
 assert.throws(()=>expand('Resonance —\nFlying'),/complete ability body/);
 assert.throws(()=>validateMechanic({keyword:'Resonance {Cost}',declarationKind:'ability-word',reminder:'Externally triggered ability named resonance'}),/plain native heading/);
 assert.throws(()=>validateMechanic({keyword:'Resonance',declarationKind:'invented',reminder:'Draw a card.'}),/Unknown custom declaration kind/);
});
