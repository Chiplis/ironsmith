import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {validateMechanic} from '../src/lib/custom-mechanics.js';
import {materializeCard} from '../../../scripts/custom-mechanics/cohort-input.mjs';
const fixture=JSON.parse(readFileSync(new URL('../../../fixtures/custom_mechanics/msem-compose-source-complete.json',import.meta.url),'utf8'));
const row=fixture.mechanics[0];
const definition=validateMechanic({keyword:'Compose',reminder:row.executable_reminder});
const cards=fixture.mechanics.flatMap(row=>row.representative_cards);
const expand=card=>materializeCard(card,[definition]).find(face=>face.name===card.name);

test('all34 exact Compose source bodies preserve originals and independently expand the declaration or grant',()=>{
 assert.equal(cards.length,34);
 assert.equal(new Set(cards.map(card=>card.source_card_key)).size,34);
 for(const card of cards){
  const before=JSON.stringify(card);
  const face=expand(card);
  assert.ok(face,card.name);
  assert.equal(JSON.stringify(card),before,`${card.name}: immutable source input`);
  assert.match(face.executable_text,/instead of putting it into your graveyard as it resolves/,card.name);
  assert.match(face.executable_text,/Copy the exiled card/,card.name);
  if(card.name.startsWith('Paul,')){
   assert.match(face.executable_text,/without custom keyword "compose"/);
   assert.match(face.executable_text,/gains custom keyword "compose"/);
   assert.deepEqual(face.custom_keyword_identities,[]);
  }else{
   assert.deepEqual(face.custom_keyword_identities,['compose'],card.name);
  }
 }
});

test('Digital Encores keeps its excluded Function clause independent of ordinary Compose',()=>{
 const card=cards.find(card=>card.name==='Digital Encores');
 assert.ok(card.excluded_dependent_clause.ordinary_compose_still_required);
 assert.match(card.excluded_dependent_clause.text,/create a Function token/);
 const face=expand(card);
 assert.match(face.executable_text,/create a Function token with [“"]Compose\.[”"]/);
 assert.match(face.executable_text,/instead of putting it into your graveyard as it resolves/);
 assert.match(card.original_text,/As you cast an instant or sorcery spell, sacrifice any number of Functions/);
});
