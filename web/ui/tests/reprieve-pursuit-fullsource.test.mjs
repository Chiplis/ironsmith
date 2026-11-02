import test from 'node:test';import assert from 'node:assert/strict';import {readFileSync} from 'node:fs';
import {materializeCard} from '../../../scripts/custom-mechanics/cohort-input.mjs';
import {executableDefinitions,definitionsForSource,indexSourceDefinitions} from '../../../scripts/custom-mechanics/source-definitions.mjs';
const fixture=JSON.parse(readFileSync(new URL('../../../fixtures/custom_mechanics/revolution-reprieve-pursuit-complete.json',import.meta.url)));
const index=indexSourceDefinitions([{path:'revolution-reprieve-pursuit-complete',fixture}]);
const definitions=(card,row)=>definitionsForSource(card,executableDefinitions(row),index);
test('all24 exact physical sources retain their original evidence and ordinary programs',()=>{
 const names=new Set();for(const row of fixture.mechanics)for(const card of row.representative_cards){const before=JSON.stringify(card.source_faces);const faces=materializeCard(card,definitions(card,row),'revolution');assert.equal(JSON.stringify(card.source_faces),before);assert.equal(faces.length,card.source_faces.length);const text=faces[0].executable_text;
 if(row.keyword==='Reprieve'){assert.doesNotMatch(text,/\breprieved a creature\b/i);assert.match(text,/As this permanent enters/);if(card.name==='Choir Ascendant')assert.equal((text.match(/As this permanent enters/g)||[]).length,3);}
 else if(card.name!=='Gutter Rat'){assert.match(text,/cast a copy of this card from your graveyard/i);assert.match(text,/sacrificing a nonland permanent/i);if(card.name==="The Founder's Hunt Begins")assert.match(text,/3 life/i);}
 names.add(card.source_card_key);}
 assert.equal(names.size,24);
});
test('noncreature source preserves permanent entry and Architect source has no inserted turn window',()=>{
 const reprieve=fixture.mechanics.find(row=>row.id==='rev-reprieve-242');
 const sanctum=reprieve.representative_cards.find(card=>card.name==='Sanctum of Repression');const s=materializeCard(sanctum,definitions(sanctum,reprieve),'revolution')[0].executable_text;assert.match(s,/Type: Enchantment/);assert.match(s,/As this permanent enters/);
 const architect=reprieve.representative_cards.find(card=>card.name==='Architect of Resolve');const a=materializeCard(architect,definitions(architect,reprieve),'revolution')[0].executable_text;assert.match(a,/left the battlefield without dying/);assert.doesNotMatch(a,/without dying this turn/);
});
