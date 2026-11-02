import test from 'node:test';import assert from 'node:assert/strict';
import {validateMechanic,compileDraftInput} from '../src/lib/custom-mechanics.js';
test('a derived granted text-addition price receives its own complete validated payment label',()=>{
 const def=validateMechanic({keyword:'Moon reecho {Cost}',reminder:"As you cast an instant or sorcery spell, you may exile this card from your graveyard and pay {Cost}. If you do, add this card's effects to that spell. You may exile only one card this way per spell."});
 const source='Each instant and sorcery card in your graveyard has moon reecho. The moon reecho cost is equal to its mana cost.';const face=compileDraftInput({faces:[{name:'Teacher',oracleText:source}]},[def]).faces[0];assert.ok(face.oracleText.includes('This text-adding cost is named moon reecho."'));assert.equal(face.customKeywordIdentities,undefined);assert.equal(face.customKeywordPayments,undefined);assert.equal(face.displayOracleText,source);
});

test('a granted text-addition label remains inside its price program before an independent subtype sibling',()=>{
 const def=validateMechanic({keyword:'Moon reecho {Cost}',reminder:"As you cast an instant or sorcery spell, you may exile this card from your graveyard and pay {Cost}. If you do, add this card's effects to that spell. You may exile only one card this way per spell. This card is an Arcane in addition to its other types."});
 const source='Each instant and sorcery card in your graveyard has moon reecho. The moon reecho cost is equal to its mana cost.';
 const face=compileDraftInput({faces:[{name:'Teacher',oracleText:source}]},[def]).faces[0];
 assert.match(face.oracleText,/You may exile only one card this way per spell\. This text-adding cost is named moon reecho\."[,]? and "This card is an Arcane in addition to its other types\."/);
 assert.equal((face.oracleText.match(/This text-adding cost is named moon reecho\./g)||[]).length,1);
 assert.doesNotMatch(face.oracleText,/other types\. This text-adding cost is named/);
 assert.equal(face.customKeywordIdentities,undefined);assert.equal(face.customKeywordPayments,undefined);assert.equal(face.displayOracleText,source);
});
