import test from 'node:test';import assert from 'node:assert/strict';
import {validateMechanic} from '../src/lib/custom-mechanics.js';
import {expandGrantedParameterizedAbilities} from '../src/lib/reminder-granted-parameters.js';
const def=validateMechanic({keyword:'Awakening {Number}',reminder:'At the beginning of combat on your turn, if you spent at least {Number} mana to cast spells this turn, you may have this permanent become a creature in addition to its other types until end of turn.'});
test('copy exception binds numeric mana-value parameter to creation rather than current characteristics',()=>{
 const source="Create a token that's a copy of it except it's an artifact instead of its other types and it has awakening X, where X is its mana value.";
 const result=expandGrantedParameterizedAbilities(source,[def]);assert.match(result,/at least the mana value of this permanent as it was created mana/);assert.ok(!/awakening X|where X/i.test(result));
});
test('ongoing grants do not silently invent creation-time numeric capture',()=>{
 const source='Creatures you control have awakening X, where X is its mana value.';assert.equal(expandGrantedParameterizedAbilities(source,[def]),source);
});
