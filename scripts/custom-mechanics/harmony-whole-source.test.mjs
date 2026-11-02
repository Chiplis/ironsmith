import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {materializeCard} from './cohort-input.mjs';
import {executableDefinitions,indexSourceDefinitions,definitionsForSource} from './source-definitions.mjs';
test('all 69 Harmony/Harmonize records retain all 68 original physical sources',()=>{
 const fixture=JSON.parse(fs.readFileSync(new URL('../../fixtures/custom_mechanics/msem-harmony-external-abilities.json',import.meta.url),'utf8'));
 const records=fixture.mechanics.flatMap(row=>row.representative_cards.map(card=>({row,card})));
 assert.equal(records.length,69);assert.equal(new Set(records.map(({card})=>card.source_card_key)).size,68);
 const failures=[];const index=indexSourceDefinitions([{fixture}]);
 for(const {row,card} of records){const original=JSON.stringify(card);try{
  const faces=materializeCard(card,definitionsForSource(card,executableDefinitions(row),index),row.scope);
  assert.equal(faces.length,card.source_faces.length);
  for(const face of faces){
   assert.ok(face.executable_text,card.source_card_key);
   assert.doesNotMatch(face.executable_text,/^Harmony\s*[—–]/im);
   if(/^Harmony\s*[—–]/im.test(card.original_text))assert.match(face.executable_text,/Externally triggered ability named harmony/i);
  }
  assert.equal(JSON.stringify(card),original,'source evidence must remain untouched');
 }catch(error){failures.push(`${row.id} / ${card.source_card_key}: ${error.message}`)}}
 assert.deepEqual(failures,[],failures.join('\n'));
});
