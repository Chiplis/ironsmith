import test from 'node:test';
import assert from 'node:assert/strict';
import {compileDraftInput,validateMechanic} from '../src/lib/custom-mechanics.js';
const rule=validateMechanic({keyword:'Escalate',reminder:"You may pay {Cost} rather than pay this spell's mana cost.\nAs this permanent enters, if its escalate cost was paid, have this permanent perform the pulse action.\n{Cost}: Have this permanent perform the pulse action. Activate only as a sorcery."});
test('named action declaration and self watcher compose without keyword dispatch',()=>{
 const text='Escalate {2}\nWhenever Herald escalates, draw a card.';
 const face=compileDraftInput({faces:[{name:'Herald',oracleText:text}]},[rule]).faces[0];
 assert.match(face.oracleText,/As this permanent enters, if its escalate cost was paid, have this permanent perform the pulse action/);
 assert.match(face.oracleText,/Whenever this permanent performs the pulse action, draw a card/);
 assert.deepEqual(face.customKeywordPayments,['escalate']);assert.equal(face.displayOracleText,text);
});
test('separate entry and action event branches retain both triggers',()=>{
 const face=compileDraftInput({faces:[{name:'Herald',oracleText:'Whenever Herald enters or escalates, draw a card.'}]},[rule]).faces[0];
 assert.equal(face.oracleText,'Whenever this permanent enters or this permanent performs the pulse action, draw a card.');
 assert.deepEqual(face.customKeywordIdentities||[],[]);
});
test('per-turn object histories and exact action payment references remain independent',()=>{
 const face=compileDraftInput({faces:[{name:'Gear',oracleText:'Equipped creature gets +X/+0, where X is the number of times it and Gear have escalated this turn.\nWhenever a permanent you control escalates, if {R}{W}{B} was spent to escalate it, draw a card.'}]},[rule]).faces[0];
 assert.match(face.oracleText,/number of times it has performed the pulse action this turn plus the number of times this permanent has performed the pulse action this turn/);
 assert.match(face.oracleText,/Whenever a permanent you control performs the pulse action, if \{R\}\{W\}\{B\} was spent to perform the pulse action, draw a card/);
});
