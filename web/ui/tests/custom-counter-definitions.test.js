import test from 'node:test';import assert from 'node:assert/strict';
import {validateMechanic,compileDraftInput} from '../src/lib/custom-mechanics.js';
import {normalizeDraftForApi} from '../src/lib/custom-card-draft.js';
const action=validateMechanic({keyword:'Develop {Object}',reminder:"Choose a kind of counter that {Object} doesn't have on it from among {CounterKinds}. Put a counter of that kind on {Object}."});
test('source-defined counter programs travel as complete independent definitions without printed ability grants',()=>{
 const ability=validateMechanic({keyword:'Second chance',reminder:"When this creature dies, return this card to its owner's hand."});
 const original='Whenever this creature attacks, develop it.\n• +1/+1\n• Second chance';
 const expanded=compileDraftInput({faces:[{name:'Student',oracleText:original}]},[action,ability]);const face=expanded.faces[0];
 assert.deepEqual(face.customKeywordCounterDefinitions,[{name:'second chance',reminder:ability.expansion}]);assert.ok(!face.customKeywordIdentities.includes('second chance'));assert.ok(!face.oracleText.includes('When this creature dies'));assert.equal(face.displayOracleText,original);
 const normalized=normalizeDraftForApi(expanded);assert.deepEqual(normalized.faces[0].customKeywordCounterDefinitions,face.customKeywordCounterDefinitions);normalized.faces[0].customKeywordCounterDefinitions[0].name='changed';assert.equal(face.customKeywordCounterDefinitions[0].name,'second chance');
});
test('ordinary declarations and stale supplied counter metadata do not infer counter semantics',()=>{
 const ability=validateMechanic({keyword:'Second chance',reminder:'Whenever this creature attacks, you gain 1 life.'});
 const face=compileDraftInput({faces:[{name:'Student',oracleText:'Second chance',customKeywordCounterDefinitions:[{name:'stale',reminder:'Flying'}]}]},[ability]).faces[0];assert.equal(face.customKeywordCounterDefinitions,undefined);assert.ok(face.customKeywordIdentities.includes('second chance'));
});

test('ordinary modal custom actions are not counter definitions',()=>{
 const renew=validateMechanic({keyword:'Renew',reminder:'Draw a card.'});
 const source='Choose one —\n• Renew\n• Gain 1 life.';
 const face=compileDraftInput({faces:[{name:'Modal choice',oracleText:source}]},[renew]).faces[0];
 assert.equal(face.customKeywordCounterDefinitions,undefined);assert.ok(face.oracleText.includes('Draw a card'));assert.ok(face.customKeywordIdentities.includes('renew'));
});
test('conflicting programs cannot silently share one source counter identity',()=>{
 const first=validateMechanic({keyword:'Second chance',reminder:'Whenever this creature attacks, draw a card.'});
 const second=validateMechanic({keyword:'Second chance',reminder:'Whenever this creature attacks, you gain 1 life.'});
 assert.throws(()=>compileDraftInput({faces:[{name:'Ambiguous counter',oracleText:'Develop it.\n• Second chance'}]},[action,first,second]),/Conflicting source-defined counter programs/);
});

test('a counter menu does not hide the same label in a separate modal instruction',()=>{
 const renew=validateMechanic({keyword:'Renew',reminder:'Draw a card.'});
 const source='Develop it.\n• Renew\n\nChoose one —\n• Renew\n• Gain 1 life.';
 const face=compileDraftInput({faces:[{name:'Mixed instructions',oracleText:source}]},[action,renew]).faces[0];
 assert.deepEqual(face.customKeywordCounterDefinitions,[{name:'renew',reminder:renew.expansion}]);
 assert.ok(face.customKeywordIdentities.includes('renew'));
 assert.ok(face.oracleText.includes('Draw a card'));
});
