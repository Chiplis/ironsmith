import test from 'node:test';import assert from 'node:assert/strict';import{readFileSync}from'node:fs';
import{indexSourceDefinitions,definitionsForSource}from'../../../scripts/custom-mechanics/source-definitions.mjs';import{materializeCard}from'../../../scripts/custom-mechanics/cohort-input.mjs';import{validateMechanic,compileDraftInput}from'../src/lib/custom-mechanics.js';
const fixture=JSON.parse(readFileSync(new URL('../../../fixtures/custom_mechanics/msem-harness-next.json',import.meta.url),'utf8'));
test('complete real source prices use typed independent payment recipes and retained objects',()=>{
 const index=indexSourceDefinitions([{fixture,path:'actual-source'}]);for(const row of fixture.mechanics)for(const card of row.representative_cards){const faces=materializeCard(card,definitionsForSource(card,row.executable_definitions,index),row.scope);assert.equal(faces[0].executable_text,card.executable_text);assert.equal(card.source_faces[0].original_text,card.original_text);}
 const grant=fixture.mechanics[1].representative_cards[0].executable_text;assert.match(grant,/have custom keyword "power price", and "As an additional cost named 'power price'.+This spell costs/);assert.match(grant,/untapped Ox you control/);assert.doesNotMatch(grant,/Only Oxen can/);
 const plow=fixture.mechanics[2].representative_cards[0].executable_text;assert.match(plow,/if an Ox was tapped to pay its power price additional cost/);assert.match(plow,/choose an Ox from the creatures tapped to pay its power price additional cost/);assert.match(plow,/untap the selected power price payment object/);
});
test('official Harness remains a separate action without custom collision exemptions',()=>{
 assert.throws(()=>validateMechanic({keyword:'Harness',reminder:'You gain 1 life.'}));const def=validateMechanic({keyword:'Power price',reminder:fixture.mechanics[0].executable_reminder});const original='Harness this permanent.';assert.equal(compileDraftInput({faces:[{name:'Official action',oracleText:original}]},[def]).faces[0].oracleText,original);
});
