import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {validateMechanic} from '../../web/ui/src/lib/custom-mechanics.js';
import {materializeCard} from './cohort-input.mjs';
const load = file => JSON.parse(readFileSync(new URL(`../../fixtures/custom_mechanics/${file}`,import.meta.url),'utf8'));
const source=load('revolution.json').mechanics.find(row=>row.id==='rev-experience-074');
const candidate=load('revolution-experience-source.json').mechanics[0];
test('misassociated Experience retains exact inventory evidence and complete Bless source faces',()=>{
  for(const key of ['id','keyword','original_reminder','rule_reference','rule_text','source_cards']) assert.deepEqual(candidate[key],source[key],key);
  for (let i=0;i<source.representative_cards.length;i++) for(const key of Object.keys(source.representative_cards[i])) if(key !== 'executable_text') assert.deepEqual(candidate.representative_cards[i][key],source.representative_cards[i][key],key);
  assert.equal(candidate.corrected_keyword,'Bless');
  assert.equal(candidate.executable_definitions[0].keyword,'Bless');
  assert.match(candidate.executable_definitions[0].reminder,/back face is a Blessing/);
});
test('full Callaphe materialization retains player counters, player recipient and complete back static grant',()=>{
  const faces=materializeCard(candidate.representative_cards[0],candidate.executable_definitions.map(validateMechanic),'revolution');
  assert.equal(faces.length,2);
  assert.match(faces[0].executable_text,/you get an experience counter/i);
  assert.match(faces[0].executable_text,/three or more experience counters/i);
  assert.match(faces[0].executable_text,/Choose target player[.] If .*attached to that player/i);
  assert.doesNotMatch(faces[0].executable_text,/Whenever this creature attacks/i);
  assert.ok(!faces[0].custom_keyword_identities.some(value=>value.toLowerCase()==='experience'));
  assert.match(faces[1].executable_text,/Enchantment.*Aura Blessing/);
  assert.match(faces[1].executable_text,/Enchant player/i);
  assert.match(candidate.representative_cards[0].source_faces[1].original_text,/may also cast this side from your hand/i);
  assert.match(faces[1].executable_text,/Enchanted player may cast blue spells with mana value less than or equal to their devotion to blue without paying their mana costs/i);
});
