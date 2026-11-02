import test from 'node:test';
import {readFileSync} from 'node:fs';
import assert from 'node:assert/strict';
import {validateMechanic,compileDraftInput} from '../src/lib/custom-mechanics.js';
import {expandCounterMenus} from '../src/lib/reminder-counter-menus.js';
const def=validateMechanic({keyword:'Upgrade',template:'{Object}',actionProgram:true,collectiveObject:true,completionRequirement:'all-choices-available',reminder:"Choose a kind of counter that {Object} doesn't have on it from among {CounterKinds}. Put a counter of that kind on {Object}."});
const name='Indecisive Sorceress';
const original="Whenever you cast a noncreature spell, upgrade Indecisive Sorceress. When you can't, remove all counters from it and it deals damage to any target equal to the number of counters removed this way.\n• +1/+1\n• Flying\n• First strike";
test('same-line closed menu failure stays a distinct targeted reflexive after the menu instruction',()=>{
 const fixture=JSON.parse(readFileSync(new URL('../../../fixtures/custom_mechanics/upgrade-unavailable-source.json',import.meta.url),'utf8'));
 assert.equal(original,fixture.original_source_record.rulesText);
 const face=compileDraftInput({faces:[{name,oracleText:original}]},[def]).faces[0];
 assert.equal(face.oracleText.trim(),fixture.executable_text.split('\n').slice(3).join('\n'));
 assert.match(face.oracleText,/from among \+1\/\+1, flying and first strike; Put a counter of that kind on the proposed objects\] When no choice was available this way, remove all counters from it and it deals damage to any target equal to the number of counters removed this way\./);
 assert.equal(face.displayOracleText,original);
 assert.doesNotMatch(face.oracleText,/When you can't|•/);
});
test('unrelated same-line prose cannot become a counter-choice failure gate',()=>{
 for(const malformed of [original.replace("When you can't,","When you can't cast it,"),original.replace('upgrade Indecisive Sorceress.','upgrade each creature you control.'),original.replace('upgrade Indecisive Sorceress.','upgrade it and this creature.'),original.replace('• First strike','• Flying')]){
  assert.throws(()=>compileDraftInput({faces:[{name,oracleText:malformed}]},[def]));
 }
});

test('result gate cannot borrow a different or compound custom menu program',()=>{
 const compound={...def,reminder:def.reminder+' Draw a card.',expansion:def.expansion+' Draw a card.'};
 assert.equal(expandCounterMenus(original,[compound],name),original);
});
