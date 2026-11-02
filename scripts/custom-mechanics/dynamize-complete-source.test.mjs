import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {materializeCard} from './cohort-input.mjs';
import {validateMechanic} from '../../web/ui/src/lib/custom-mechanics.js';
test('all twelve complete Dynamize sources expand using their own authored face names',()=>{
 const fixture=JSON.parse(fs.readFileSync(new URL('../../fixtures/custom_mechanics/msem-dynamize-complete.json',import.meta.url),'utf8'));
 const cards=fixture.mechanics.flatMap(row=>row.representative_cards.map(card=>({row,card})));
 assert.equal(cards.length,12);const failures=[];
 for(const {row,card} of cards){
  const before=JSON.stringify(card);const definitions=row.executable_definitions.map(validateMechanic);
  try{
   const faces=materializeCard(card,definitions,row.scope);
   assert.equal(faces.length,card.source_faces.length);
   for(const face of faces){
    assert.doesNotMatch(face.executable_text,/\bdynamize\b/i,card.source_card_key);
    assert.match(face.executable_text,/dynamized status/i,card.source_card_key);
    assert.ok(face.custom_keyword_identities.includes('dynamize'),card.source_card_key);
   }
   assert.equal(JSON.stringify(card),before,'materialization must preserve original source data');
  }catch(error){failures.push(`${card.source_card_key}: ${error.message}`);}
 }
 assert.deepEqual(failures,[],failures.join('\n'));
});
