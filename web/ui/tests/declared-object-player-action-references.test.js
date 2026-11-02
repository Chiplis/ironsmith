import {readFileSync} from 'node:fs';
import {materializeCard} from '../../../scripts/custom-mechanics/cohort-input.mjs';
import test from 'node:test';
import assert from 'node:assert/strict';
import {validateMechanic,compileDraftInput,draftMechanics,mergeMechanics,prepareCustomCard} from '../src/lib/custom-mechanics.js';
import {expandDeclaredObjectReferences,normalizeNamedObjectReferences} from '../src/lib/reminder-declared-object-references.js';
import {expandNamedActionReferences} from '../src/lib/reminder-action-references.js';
import {definitionsForSource,indexSourceDefinitions} from '../../../scripts/custom-mechanics/source-definitions.mjs';
const favored={keyword:'Favored',reminder:'You gain 1 life.',namedObjectReferences:[{reference:'your Champion',name:'The Champion',controller:'you'}]};
const indulgence={keyword:'Indulgence',reminder:'At the beginning of your end step, perform the indulge action.',forms:{eventVerb:'indulge',eventLabel:'indulge',participle:'indulged'}};
const face=oracleText=>({name:'Consumer witness',oracleText});
const assertNoKeywordIdentities=face=>{
 assert.ok(!Object.hasOwn(face,'customKeywordIdentities')||Array.isArray(face.customKeywordIdentities),'present marker metadata must remain a typed array');
 assert.deepEqual(face.customKeywordIdentities??[],[],'a reference-only consumer must not acquire any keyword identity');
};
test('authored object references become current name/controller queries without marker grants',()=>{
 const definition=validateMechanic(favored);
 assert.deepEqual(definition.namedObjectReferences,favored.namedObjectReferences);
 const draft={customMechanics:[definition],faces:[face('Whenever your Champion attacks, draw a card.')]};
 const retained=draftMechanics(draft);
 assert.equal(retained.length,1);
 const output=compileDraftInput(draft,retained).faces[0];
 assert.equal(output.oracleText,'Whenever a permanent you control named "The Champion" attacks, draw a card.');
 assertNoKeywordIdentities(output);
 assert.equal(output.originalPrintedRulesText,draft.faces[0].oracleText);
 assert.equal(expandDeclaredObjectReferences(output.oracleText,[definition]),output.oracleText);
 assert.equal(expandDeclaredObjectReferences('Put a counter on your Champion.',[definition]),'Put a counter on a permanent you control named "The Champion".');
 assert.equal(expandDeclaredObjectReferences('Your Champion has deathtouch.',[definition]),'A permanent you control named "The Champion" has deathtouch.');
});
test('reference operands stay literal while explicit granted rule programs remain executable',()=>{
 for(const original of ['Choose a card named "your Champion".','Choose a card with "your Champion" in its rules text.','A creature named your Champion gets +1/+1.','Create a token named "your Champion".']){
  assert.equal(expandDeclaredObjectReferences(original,[favored]),original);
 }
 const grant='Creatures you control have "Whenever your Champion attacks, draw a card."';
 assert.equal(expandDeclaredObjectReferences(grant,[favored]),'Creatures you control have "Whenever a permanent you control named \'The Champion\' attacks, draw a card."');
 const nestedLiteral='Creatures have "Search for a card named \'your Champion\'."';
 assert.equal(expandDeclaredObjectReferences(nestedLiteral,[favored]),nestedLiteral);
 assert.equal(expandDeclaredObjectReferences('your Champions attack',[favored]),'your Champions attack');
});
test('object query meaning comes only from supplied names and explicit controller data',()=>{
 const arbitrary={...favored,namedObjectReferences:[{reference:'their Guide',name:"Aster's Compass",controller:'opponent'}]};
 assert.equal(expandDeclaredObjectReferences('Whenever their Guide attacks, scry 1.',[arbitrary]),'Whenever a permanent an opponent controls named "Aster\'s Compass" attacks, scry 1.');
 for(const malformed of [{reference:'your Guide',name:'Guide'},{reference:'your Guide',name:'Guide',controller:'owner'},{reference:'your Guide',name:'Guide',controller:'you',status:'special'},{reference:'your Guide',name:'"malformed"',controller:'you'}]){
  assert.throws(()=>normalizeNamedObjectReferences([malformed]));
 }
 assert.throws(()=>normalizeNamedObjectReferences([{reference:'your Guide',name:'First',controller:'you'},{reference:'your Guide',name:'Second',controller:'you'}]));
 assert.throws(()=>validateMechanic({...favored,keyword:'Champion'}),'official keyword remains reserved');
});
test('zero-operand player events and explicit past forms retain player history windows',()=>{
 const definition=validateMechanic(indulgence);
 for(const [input,expected] of [
  ['Whenever you indulge, draw a card.','Whenever you perform the indulge action, draw a card.'],
  ['When an opponent indulges, you gain 1 life.','When an opponent performs the indulge action, you gain 1 life.'],
  ["This creature can't attack unless you indulged during your last turn.","This creature can't attack unless you have performed the indulge action during your last turn."],
  ["If you haven't indulged this combat, draw a card.",'If you have not performed the indulge action this combat, draw a card.'],
  ['Creatures have "Whenever you indulge, draw a card."','Creatures have "Whenever you perform the indulge action, draw a card."'],
 ]){
  assert.equal(expandNamedActionReferences(input,[definition]),expected);
  assert.equal(expandNamedActionReferences(expected,[definition]),expected);
 }
 const conflict=validateMechanic({...indulgence,keyword:'Another ritual',forms:{eventVerb:'indulge',eventLabel:'other receipt',participle:'ritualized'}});
 assert.throws(()=>expandNamedActionReferences('Whenever you indulge, draw a card.',[definition,conflict]),/Ambiguous named action reference/);
 const literal='Name a card named "you indulged during your last turn".';
 assert.equal(expandNamedActionReferences(literal,[definition]),literal);
 const draft={customMechanics:[definition],faces:[face("This creature can't attack unless you indulged during your last turn.")]};
 const retained=draftMechanics(draft);assert.equal(retained.length,1);
 assertNoKeywordIdentities(compileDraftInput(draft,retained).faces[0]);
 const renamed=validateMechanic({...indulgence,keyword:'Shared celebration',forms:{eventVerb:'celebrate',eventLabel:'feast receipt',participle:'celebrated'}});
 assert.equal(expandNamedActionReferences('Whenever you celebrate, draw a card.',[renamed]),'Whenever you perform the feast receipt action, draw a card.');
});
test('existing object-packet event references keep performer and affected-object binding',()=>{
 const definition=validateMechanic({keyword:'Vision',reminder:'Draw a card.',forms:{eventVerb:'vision',eventLabel:'vision',participle:'visioned'}});
 const output=expandNamedActionReferences('Whenever this creature visions creatures, it deals damage equal to the number of creatures it visioned to each opponent.',[definition]);
 assert.match(output,/performs the vision action with creatures/);assert.match(output,/this permanent deals damage equal to that many/);
});
test('source adapters fingerprint alias meaning and saved consumer drafts retain the declaration',async()=>{
 const card={id:'consumer-source',name:'Consumer',source_card_key:'consumer-source'};
 const fixture={mechanics:[{id:'provider',executable_definitions:[favored],representative_cards:[card]}]};
 const index=indexSourceDefinitions([{fixture,path:'source'}]);
 assert.deepEqual(definitionsForSource(card,[favored],index)[0].namedObjectReferences,favored.namedObjectReferences);
 const changed={...favored,namedObjectReferences:[{reference:'your Champion',name:'Another Champion',controller:'you'}]};
 assert.throws(()=>definitionsForSource(card,[changed],index),/Conflicting executable definitions/);
 assert.throws(()=>mergeMechanics([favored],[changed]));
 assert.throws(()=>mergeMechanics([indulgence],[{...indulgence,forms:{...indulgence.forms,eventLabel:'other receipt'}}]));
 const draft={faces:[face('Whenever your Champion attacks, draw a card.')]};
 const saved=await prepareCustomCard(draft,async input=>{assert.match(input.faces[0].oracleText,/named "The Champion"/);return {canCreate:true};},{known:[favored]});
 assert.deepEqual(saved.draft.customMechanics[0].namedObjectReferences,favored.namedObjectReferences);
 assertNoKeywordIdentities(saved.input.faces[0]);
});

test('all exact Favored and Indulgence sources materialize from raw authored declarations',()=>{
 const fixture=JSON.parse(readFileSync(new URL('../../../fixtures/custom_mechanics/revolution-favored-indulgence-complete.json',import.meta.url),'utf8'));
 let count=0;
 for(const row of fixture.mechanics){
  const definitions=row.executable_definitions.map(({expansion,...authored})=>authored);
  assert.ok(definitions.every(definition=>definition.expansion===undefined),'exercise the ordinary raw source-import declaration path');
  for(const card of row.representative_cards){
   const faces=materializeCard(card,definitions,row.scope);
   assert.equal(faces.length,card.source_faces.length,card.source_card_key);
   for(const face of faces){
    count++;
    if(card.source_role!=='named_reference_consumer')assert.ok(face.executable_text.includes(row.keyword==='Favored'?'champion selection action':'once rather than once for each permanent with this ability'),card.source_card_key);
    if(card.source_role==='named_reference_consumer'){
     assert.deepEqual(face.custom_keyword_identities??[],[],`${card.source_card_key}: reference-only source cannot acquire Favored`);
     assert.ok(face.executable_text.includes('named "The Champion"'),card.source_card_key);
    }
   }
  }
 }
 assert.equal(count,42);
});
