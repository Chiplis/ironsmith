import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {definitionsForSource,indexSourceDefinitions} from '../../../scripts/custom-mechanics/source-definitions.mjs';
import {materializeCard} from '../../../scripts/custom-mechanics/cohort-input.mjs';
import {aliasSourceKeywordFaces} from '../../../scripts/custom-mechanics/source-keyword-aliases.mjs';
import {prepareCustomCard, validateMechanic} from '../src/lib/custom-mechanics.js';
const fixture=JSON.parse(readFileSync(new URL('../../../fixtures/custom_mechanics/msem-eureka-complete.json',import.meta.url),'utf8'));
const row=fixture.mechanics[0];
test('all nine complete operative source cards retain formal token definitions after expansion',()=>{
 const index=indexSourceDefinitions([{fixture,path:'complete-source'}]);
 assert.equal(row.representative_cards.length,9);
 for(const card of row.representative_cards){
  const frozen=JSON.stringify(card.source_faces);
  const definitions=definitionsForSource(card,row.executable_definitions,index);
  const faces=materializeCard(card,definitions,'msem');
  assert.equal(JSON.stringify(card.source_faces),frozen);
  assert.ok(faces[0].executable_text.includes('Choose one of the following:'));
  assert.equal(faces[0].custom_token_definitions.length,3);
  assert.ok(faces[0].custom_subtypes.some(definition=>definition.singular.toLowerCase()==='masterpiece' && definition.family==='Artifact'));
  for(const token of faces[0].custom_token_definitions){assert.ok(token.executable_text.includes('Type: Artifact — Masterpiece'));assert.ok(token.executable_text.includes('\n'));}
  if(card.source_dependencies.includes('Archive'))assert.ok(faces[0].executable_text.includes('archived library'));
  if(card.source_dependencies.includes('Discovery'))assert.ok(faces[0].executable_text.includes('III, Discovery'));
 }
});
test('punctuated source declarations require exact evidence and whole lexical spelling',()=>{
 const definitions=row.executable_definitions;
 const reminder=definitions[0].sourceReminders[0];
 const spellings=[`Eureka! (${reminder})`,`Eureka!! (${reminder})`,`Eureka!wards (${reminder})`,`Eureka! (You gain 1 life.)`,`Name a card named "Eureka! (${reminder})".`];
 const results=aliasSourceKeywordFaces(spellings.map((oracleText,index)=>({name:`Case ${index}`,oracleText})),definitions);
 const canonicalKeyword=validateMechanic(definitions[0]).keyword;
 assert.equal(results[0].oracleText,`${canonicalKeyword} (${reminder})`);
 for(let i=1;i<results.length;i++)assert.equal(results[i].oracleText,spellings[i]);
 const discovery=aliasSourceKeywordFaces([{name:'Progress card',oracleText:`III, Discovery — Eureka! (${reminder})`}],definitions);
 assert.equal(discovery[0].oracleText,`III, Discovery — ${canonicalKeyword} (${reminder})`);
});
test('normal custom-card preparation carries tokens referenced only by expanded programs',async()=>{
 const card=row.representative_cards[0];
 const canonical=row.executable_definitions[0];
 const draft={faces:[{name:'Indirect token producer',oracleText:`Eureka (${canonical.reminder})`}],customMechanics:[{keyword:'Eureka',reminder:canonical.reminder}],customTokens:card.custom_tokens};
 let calls=0;
 const prepared=await prepareCustomCard(draft,async input=>{calls++;assert.equal(input.customTokenFaces.length,3);return {canCreate:true};});
 assert.equal(calls,1);assert.equal(prepared.draft.customTokens.length,3);assert.ok(prepared.input.customSubtypes.some(definition=>definition.singular.toLowerCase()==='masterpiece'));
});

test('all nine exact Eureka clause proofs retain source spans while only seven whole cards have no new-rule dependency',()=>{
 const index=indexSourceDefinitions([{fixture,path:'complete-source'}]);let whole=0,bounded=0;
 for(const card of row.representative_cards){
  const clause=card.independent_eureka_clause;assert.equal(clause.claim_scope,'eureka_ability_only_not_whole_card');
  assert.equal(Array.from(card.original_text).slice(clause.source_span.start,clause.source_span.end).join(''),clause.source_span.text);
  const definitions=definitionsForSource(clause,row.executable_definitions,index);const output=materializeCard(clause,definitions,'msem');assert.equal(output[0].custom_token_definitions.length,3);assert.ok(output[0].executable_text.includes('Choose one of the following:'));
  if(card.source_dependencies.length){bounded++;assert.equal(card.whole_source_support_status,'unsupported_requires_new_rules');assert.ok(card.whole_source_rule_boundaries.every(item=>item.classification==='unsupported_requires_new_rules'));}else{whole++;}
 }
 assert.equal(whole,7);assert.equal(bounded,2);
});
