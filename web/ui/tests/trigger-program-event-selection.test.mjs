import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { compileDraftInput, validateMechanic, mergeMechanics } from '../src/lib/custom-mechanics.js';
const fixture=JSON.parse(readFileSync(new URL('../../../fixtures/custom_mechanics/msem-death-knell-complete-source.json',import.meta.url)));
const definition=validateMechanic(fixture.mechanics[0].executable_definitions[0]);
const compile=(oracleText,name='Other source')=>compileDraftInput({faces:[{name,oracleText}]},[definition]).faces[0];
test('declared external program gains exactly one source-authored independent selection rule',()=>{
 const face=compile('Death knell — You gain 2 life.');
 assert.match(face.oracleText,/Externally triggered ability named death knell — You gain 2 life/);
 assert.equal((face.oracleText.match(/As a creature you control dies, select up to one/g)||[]).length,1);
 assert.deepEqual(face.customKeywordIdentities,['death knell']);
 assert.equal(fixture.mechanics[0].representative_cards.length,27);
});
test('explicit external invocation does not install a rule or gain its identity',()=>{
 const face=compile('Destroy target creature. Trigger a death knell of a permanent you control.');
 assert.equal(face.oracleText.toLowerCase(),'destroy target creature. trigger one death knell triggered ability of a permanent you control.');
 assert.doesNotMatch(face.oracleText,/As a creature you control dies/);
 assert.equal(face.customKeywordIdentities,undefined);
});
test('source-authored entry and attack projections do not add an intermediate trigger',()=>{
 const entry=compile('Other source and other permanents you control entering the battlefield cause their death knell to trigger.\nDeath knell — Draw a card.');
 assert.match(entry.oracleText,/As a permanent you control enters, trigger each externally triggered ability named death knell of the event arrival/);
 const attack=compile('Creatures you control attacking causes their death knell to trigger.\nDeath knell — You gain 1 life.');
 assert.match(attack.oracleText,/As a creature you control attacks, trigger each externally triggered ability named death knell of the event source/);
});
test('declared class references preserve chooser authority and literal criteria',()=>{
 const text='Whenever a creature you control dies, enchanted player chooses a death knell ability of a permanent you control. Trigger that ability.';
 assert.match(compile(text).oracleText,/Trigger one death knell triggered ability of a permanent you control, chosen by enchanted player/);
 const literal='A card named "Trigger a death knell of a permanent you control".';
 assert.equal(compile(literal).oracleText,literal);
 assert.throws(()=>validateMechanic({...fixture.mechanics[0].executable_definitions[0],keyword:'Flying'}),/official keywords cannot be redefined/);
});

test('program selection rules remain part of the saved definition meaning',()=>{
 const changed={...fixture.mechanics[0].executable_definitions[0],programSelectionRules:[]};
 assert.throws(()=>mergeMechanics([definition],[changed]),/could not be added/);
});
