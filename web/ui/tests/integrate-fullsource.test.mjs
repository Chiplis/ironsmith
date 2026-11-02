import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {compileDraftInput,validateMechanic} from '../src/lib/custom-mechanics.js';
import {materializeCard} from '../../../scripts/custom-mechanics/cohort-input.mjs';
import {executableDefinitions} from '../../../scripts/custom-mechanics/source-definitions.mjs';
const fixture=JSON.parse(readFileSync(new URL('../../../fixtures/custom_mechanics/revolution-integrate-complete.json',import.meta.url)));
test('all exact physical sources preserve evidence and expand collective actions',()=>{
 const keys=new Set();
 for(const row of fixture.mechanics)for(const card of row.representative_cards){
  const before=JSON.stringify(card.source_faces);
  const faces=card.source_faces;
  const output=materializeCard(card,executableDefinitions(row),'revolution');
  assert.equal(JSON.stringify(card.source_faces),before);
  assert.equal(output.length,faces.length);
  assert.doesNotMatch(output[0].executable_text,/\b(?:integrate|integrated)\b(?! action| relationship)/i,card.name);
  keys.add(card.source_card_key);
 }
 assert.equal(keys.size,25);
});
test('collective object declaration is explicit and cannot alter arbitrary templates',()=>{
 assert.throws(()=>validateMechanic({keyword:'Assimilate',template:'{Player}',reminder:'Draw a card.',actionProgram:true,collectiveObject:true}),/collectiveObject/);
});
