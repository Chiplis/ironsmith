import test from 'node:test';import assert from 'node:assert/strict';
import {validateMechanic,compileDraftInput,mergeMechanics,installCustomMechanics,uninstallCustomMechanic,customMechanicRevision} from '../src/lib/custom-mechanics.js';
import {normalizeResolutionProgramMultiplicity} from '../src/lib/reminder-resolution-recipe-policy.js';
const reminder='If this spell was cast from your hand, instead of putting it into your graveyard as it resolves, exile it and when you next cast a spell, you may cast the card exiled this way without paying its mana cost.';
const recipe={keyword:'Refrain recipe',reminder,resolutionProgramMultiplicity:{mode:'once-per-declared-key',key:'authored recipe'}};
test('explicit declared recipe keys survive normalization and expansion independent of keyword spelling',()=>{
 const definition=validateMechanic(recipe);assert.equal(definition.reminder,reminder);assert.deepEqual(definition.resolutionProgramMultiplicity,recipe.resolutionProgramMultiplicity);
 const face=compileDraftInput({faces:[{name:'Carrier witness',oracleText:'Refrain recipe.'}]},[definition]).faces[0];
 assert.ok(face.oracleText.includes('exile it once for the resolution recipe named "authored recipe" and when you next cast'));
 const ordinary=validateMechanic({...recipe,resolutionProgramMultiplicity:{mode:'per-instance'}});assert.ok(!ordinary.expansion.includes('once for the resolution recipe'));assert.equal(ordinary.resolutionProgramMultiplicity,undefined);
 const granted=compileDraftInput({faces:[{name:'Grant witness',oracleText:'Instant spells you control have Refrain recipe.'}]},[definition]).faces[0];
 assert.ok(granted.oracleText.includes("named 'authored recipe'"));
 assert.throws(()=>mergeMechanics([definition],[{...recipe,resolutionProgramMultiplicity:{mode:'once-per-declared-key',key:'different recipe'}}]));
});
test('multiplicity needs an authored key and the independently recognized destination operation',()=>{
 for(const value of [{mode:'once-per-declared-key'},{mode:'once-per-declared-key',key:'bad"key'},{mode:'per-instance',key:'ignored'},{mode:'guess'}])assert.throws(()=>normalizeResolutionProgramMultiplicity(value,reminder));
 assert.throws(()=>normalizeResolutionProgramMultiplicity(recipe.resolutionProgramMultiplicity,'Draw a card.'));
});

import {definitionsForSource,indexSourceDefinitions} from '../../../scripts/custom-mechanics/source-definitions.mjs';
test('normalized recipe identity participates in source imports and active compiler revision',()=>{
 const card={source_card_key:'recipe-source'};
 const indexed=(definition)=>indexSourceDefinitions([{fixture:{mechanics:[{id:'author',executable_definitions:[definition],representative_cards:[card]}]}}]);
 const ordinary={keyword:recipe.keyword,reminder};
 assert.equal(definitionsForSource(card,[ordinary],indexed({...ordinary,resolutionProgramMultiplicity:{mode:'per-instance'}})).length,1);
 assert.throws(()=>definitionsForSource(card,[recipe],indexed({...recipe,resolutionProgramMultiplicity:{mode:'once-per-declared-key',key:'different recipe'}})),/Conflicting executable definitions/);
 try {
  installCustomMechanics([ordinary]);const previous=customMechanicRevision();
  installCustomMechanics([recipe]);assert.ok(customMechanicRevision()>previous,'changing only recipe policy invalidates active compiler metadata caches');
 } finally {uninstallCustomMechanic(recipe.keyword);}
});
