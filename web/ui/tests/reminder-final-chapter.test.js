import test from 'node:test';
import assert from 'node:assert/strict';
import {compileDraftInput,validateMechanic} from '../src/lib/custom-mechanics.js';
const reminder="As this Saga's final chapter ability resolves, remove all lore counters from it.";
const definition=validateMechanic({keyword:'Recur',reminder});
test('final chapter resolution instruction remains a live static ability instead of being captured in chapter effects',()=>{
 const face=compileDraftInput({faces:[{name:'Saga',oracleText:'Recur\nI — Draw a card.\nII, III — Gain 2 life.'}]},[definition]).faces[0];
 assert.equal(face.oracleText,reminder.replace(/[.]$/,'')+'\nI — Draw a card.\nII, III — Gain 2 life.');
 assert.deepEqual(face.customKeywordIdentities,['recur']);
});
