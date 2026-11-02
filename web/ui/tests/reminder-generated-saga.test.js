import test from 'node:test';
import assert from 'node:assert/strict';
import {compileDraftInput,validateMechanic} from '../src/lib/custom-mechanics.js';
test('a generated Saga receives the live static instruction and separate quoted chapter',()=>{
 const def=validateMechanic({keyword:'Recur',reminder:"As this Saga's final chapter ability resolves, remove all lore counters from it."});
 const text='Create a colorless Saga enchantment token with recur and “r3— You gain 3 life.”';
 const face=compileDraftInput({faces:[{name:'Token maker',oracleText:text}]},[def]).faces[0];
 assert.match(face.oracleText,/with custom keyword "recur", and "As this Saga's final chapter ability resolves, remove all lore counters from it\.",? and “r3— You gain 3 life\.”/);
 assert.equal(face.customKeywordIdentities,undefined);
});
