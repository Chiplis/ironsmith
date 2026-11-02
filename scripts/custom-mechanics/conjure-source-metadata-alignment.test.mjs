import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {definitionsForSource,indexSourceDefinitions,executableDefinitions} from './source-definitions.mjs';
const files=['msem-conjure-source-complete.json','msem-conjure-whole-family-source-inputs.json','msem-sightsee-complete-source.json','msem-repository-warden-copy-source.json'];
const fixtures=files.map(path=>({path,fixture:JSON.parse(readFileSync(new URL(`../../fixtures/custom_mechanics/${path}`,import.meta.url),'utf8'))}));
test('overlapping physical sources have identical operation-form metadata rather than weakened conflicts',()=>{
 const index=indexSourceDefinitions(fixtures);
 for(const {path,fixture} of fixtures)for(const row of fixture.mechanics)for(const card of row.representative_cards){
  const definitions=definitionsForSource(card,executableDefinitions(row),index,path);
  const creation=definitions.find(def=>def.keyword.toLowerCase()==='conjure');
  assert.ok(creation.forms.variants.map(word=>word.toLowerCase()).includes('conjuring'));
 }
});
test('Campfire carries every Sightsee option while preserving its own named-definition pool',()=>{
 const card=fixtures[1].fixture.mechanics[0].representative_cards.find(card=>card.source_card_key==='Campfire Storyteller_AEK');
 const sight=fixtures[2].fixture.mechanics[0].representative_cards.find(other=>other.source_card_key===card.source_card_key);
 const names=new Set(card.supplied_card_definitions.map(card=>card.name));
 assert.equal(card.supplied_card_definitions.length,49);
 for(const definition of sight.supplied_card_definitions)assert.ok(names.has(definition.name),definition.name);
 assert.ok(names.has('Data-Point'));assert.ok(names.has('Snow-Covered Wastes'));
 assert.equal(card.supplied_card_definitions.find(card=>card.name==='Forest').sourceCardKey,'Forest_AFM');
});

test('all four canonical creation declarations retain their dedicated identity and destination interface',()=>{
 const declarations=fixtures.flatMap(({fixture})=>fixture.mechanics.flatMap(row=>row.executable_definitions||[]))
  .filter(definition=>definition.keyword.toLowerCase()==='conjure');
 assert.equal(declarations.length,4);
 for(const declaration of declarations){
  assert.deepEqual(declaration,declarations[0]);
  assert.equal(declaration.declarationKind,'card-creation');
  assert.equal(declaration.actionProgram,undefined);
  assert.equal(declaration.collectiveObject,undefined);
  assert.equal(declaration.template,undefined);
 }
});
