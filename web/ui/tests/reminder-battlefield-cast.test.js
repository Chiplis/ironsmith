import test from 'node:test';
import assert from 'node:assert/strict';
import {compileDraftInput,validateMechanic} from '../src/lib/custom-mechanics.js';
const definition=validateMechanic({keyword:'Relocate',template:'{Cost}',reminder:'You may cast this spell from the battlefield for {Cost} rather than pay its mana cost.'});
test('battlefield-origin replacement cost binds complete mana and nonmana components',()=>{
 const face=compileDraftInput({faces:[{name:'Example',oracleText:'Relocate—{1}{B}, Pay 4 life.'}]},[definition]).faces[0];
 assert.equal(face.oracleText,'You may cast this spell from the battlefield for {1}{B}, Pay 4 life rather than pay its mana cost.');
 assert.deepEqual(face.customKeywordIdentities,['relocate']);assert.deepEqual(face.customKeywordPayments,['relocate']);
});
test('granted replacement price preserves recipient attack-history filter and quoted current-recipient source',()=>{
 const text='Nontoken creatures you control that have attacked this turn have relocate {2}.';
 const face=compileDraftInput({faces:[{name:'Grantor',oracleText:text}]},[definition]).faces[0];
 assert.equal(face.oracleText,'Nontoken creatures you control that have attacked this turn have custom keyword "relocate", and "You may cast this spell from the battlefield for {2} rather than pay its mana cost.".');
 assert.equal(face.customKeywordIdentities,undefined);
});
test('negated cost action becomes a selected-price prohibition without gaining that keyword',()=>{
 const text="Enchanted permanent can't attack, block, or relocate.";
 const face=compileDraftInput({faces:[{name:'Restriction',oracleText:text}]},[definition]).faces[0];
 assert.equal(face.oracleText,"Enchanted permanent can't attack or block.\nEnchanted permanent can't be cast using \"relocate\".");
 assert.equal(face.customKeywordIdentities,undefined);assert.equal(face.displayOracleText,text);
});
test('external past-tense receipt reference does not grant the mechanic to its watcher',()=>{
 const text='Whenever a creature you control enters, if it relocated, you gain 3 life.';
 const face=compileDraftInput({faces:[{name:'Watcher',oracleText:text}]},[definition]).faces[0];
 assert.equal(face.oracleText,'Whenever a creature you control enters, if it was cast using "relocate", you gain 3 life.');
 assert.equal(face.customKeywordIdentities,undefined);assert.equal(face.displayOracleText,text);
});
