import test from 'node:test';import assert from 'node:assert/strict';import {readFileSync} from 'node:fs';
import {compileDraftInput,validateMechanic} from '../src/lib/custom-mechanics.js';
import {definitionsForSource,indexSourceDefinitions,executableDefinitions} from '../../../scripts/custom-mechanics/source-definitions.mjs';
import {materializeCard} from '../../../scripts/custom-mechanics/cohort-input.mjs';
test('numeric granted programs keep the full cast-time suffix outside the quoted entry program',()=>{
 const definition=validateMechanic({keyword:'Vigorate',reminder:'If you attacked this turn, this creature enters with {Number} +1/+1 counters on it.'});
 for(const suffix of ['it','them','that spell']){const input=`Creature spells you cast this turn gain vigorate 1 as you cast ${suffix}.`;const output=compileDraftInput({faces:[{name:'Independent grant witness',oracleText:input}]},[definition]).faces[0].oracleText;assert.match(output,/gain custom keyword "Vigorate", and "If you attacked this turn,[^"]+" as you cast/i);const body=output.match(/and "([^"]+)"/)[1];assert.ok(body.includes('1 +1/+1 counter'));assert.ok(!body.includes('as you cast'));assert.ok(output.endsWith(`as you cast ${suffix}.`));assert.doesNotMatch(output,/\bvigorate 1\b/i);}
});
test('actual complete Fanatical Vanguard source expands the numeric consumer without losing its activation body',()=>{
 const fixture=JSON.parse(readFileSync(new URL('../../../fixtures/custom_mechanics/msem-malign.json',import.meta.url)));const row=fixture.mechanics.find(row=>row.id==='msem-970');const card=row.representative_cards.find(card=>card.name==='Fanatical Vanguard');const before=JSON.stringify(card.source_faces);const index=indexSourceDefinitions([{fixture,path:'msem-malign'}]);const definitions=definitionsForSource(card,executableDefinitions(row),index);const text=materializeCard(card,definitions,'msem')[0].executable_text;assert.equal(JSON.stringify(card.source_faces),before);assert.doesNotMatch(text,/gain malign 1 as you cast them/i);assert.match(text,/custom keyword "Malign"/i);assert.match(text,/as you cast them/);assert.match(text,/Exile Fanatical Vanguard from your graveyard/);
});
