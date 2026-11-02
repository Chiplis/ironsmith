import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {materializeCard} from './cohort-input.mjs';
import {executableDefinitions,definitionsForSource,indexSourceDefinitions} from './source-definitions.mjs';
const fixture=JSON.parse(readFileSync(new URL('../../fixtures/custom_mechanics/msem-catalyze-fullsource.json',import.meta.url)));
const costs=JSON.parse(readFileSync(new URL('../../fixtures/custom_mechanics/msem-casting-costs.json',import.meta.url)));
const index=indexSourceDefinitions([{fixture},{fixture:costs}]);
test('all 16 actual Catalyze source faces retain every sibling and one-mana action semantics',()=>{
 let count=0;
 for(const row of fixture.mechanics) for(const card of row.representative_cards) {
  const before=JSON.stringify(card.source_faces);
  const definitions=definitionsForSource(card,executableDefinitions(row),index);
  assert.equal(definitions.find(def=>def.keyword.toLowerCase()==='catalyze').actionProgram,true);
  const [face]=materializeCard(card,definitions,'msem');
  assert.equal(JSON.stringify(card.source_faces),before);
  assert.doesNotMatch(face.executable_text,/costs \{2\} less to cast/);
  if(card.name==='Bazaar Trickster') {
   assert.match(face.executable_text,/If you would perform the catalyze action/i);
   assert.match(face.executable_text,/You may pay an additional \{2\} as you cast a spell\. If you do, reveal the bottom card of your library\. You may put it into your graveyard or on top of your library\./);
  }
  if(card.name==='Sigil Slinger') assert.equal((face.executable_text.match(/Perform the catalyze action as follows:/gi)||[]).length,2);
  count++;
 }
 assert.equal(count,16);
});
