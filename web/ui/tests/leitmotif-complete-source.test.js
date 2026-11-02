import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {indexSourceDefinitions,definitionsForSource} from '../../../scripts/custom-mechanics/source-definitions.mjs';
import {materializeCard} from '../../../scripts/custom-mechanics/cohort-input.mjs';
const fixture=JSON.parse(readFileSync(new URL('../../../fixtures/custom_mechanics/msem-leitmotif-complete.json',import.meta.url),'utf8'));
test('all16physicalsources retain complete programs and immutable originals, including13new sources',()=>{
 const index=indexSourceDefinitions([{fixture,path:'complete-leitmotif'}]);let count=0,added=0;
 for(const row of fixture.mechanics)for(const card of row.representative_cards){
  const original=JSON.stringify(card.source_faces);const text=card.original_text;
  const faces=materializeCard(card,definitionsForSource(card,row.executable_definitions,index),'msem');
  assert.equal(faces.length,1);assert.equal(JSON.stringify(card.source_faces),original);assert.equal(card.original_text,text);
  assert.ok(faces[0].executable_text.includes('this spell was cast from your hand'));
  assert.ok(faces[0].executable_text.includes('when you next cast a spell'));
  assert.ok(faces[0].executable_text.includes('you may cast the card exiled this way without paying its mana cost'));
  if(card.variant==='grant'){
   assert.match(faces[0].executable_text,/Instant and sorcery spells you control have custom keyword "leitmotif"/);
  }else {
   const originalBody=card.original_text.split('\n').filter(line=>!/^Mana cost:|^Type:/.test(line)).join('\n');
   const primary=originalBody.slice(0,originalBody.search(/(?:Type )?Leitmotif \*/i)).trimEnd();
   // Source display layout trims line edges. Turning of the Cosmos has a
   // printed trailing space after its modal dash; compare the entire ordered
   // primary program after only that same layout normalization.
   const normalizedPrimary=primary.split('\n').map(line=>line.trim()).join('\n');
   const normalizedExecutable=faces[0].executable_text.split('\n').map(line=>line.trim()).join('\n');
   assert.ok(normalizedExecutable.includes(normalizedPrimary),card.name+' must retain every primary instruction');
   if(card.name==='Turning of the Cosmos'){
    assert.match(normalizedExecutable,/Choose one —\n• Surveil 1, then draw a card\.\n• Copy target spell you control\. You may choose new targets for the copy\./);
   }
  }
  count++;if(!card.previously_in_seed)added++;
 }
 assert.equal(count,16);assert.equal(added,13);
});
