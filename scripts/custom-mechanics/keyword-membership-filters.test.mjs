import test from 'node:test';
import assert from 'node:assert/strict';
import {validateMechanic} from '../../web/ui/src/lib/custom-mechanics.js';
import {expandGrantedTriggeredPrograms} from '../../web/ui/src/lib/reminder-ability-grants.js';
const def=validateMechanic({keyword:'Soul tether',reminder:'When this nontoken creature dies, draw a card.'});
test('known custom keyword membership retains object owner and movement qualifiers',()=>{
 const text='You may return an Aura with soul tether you owned that was exiled this way to the battlefield.';
 const result=expandGrantedTriggeredPrograms(text,[def]);
 assert.equal(result,'You may return an Aura with custom keyword "soul tether" you owned that was exiled this way to the battlefield.');
 assert.equal(expandGrantedTriggeredPrograms('Choose a creature without soul tether you control.',[def]),'Choose a creature without custom keyword "soul tether" you control.');
});
test('membership rewrite preserves quoted names and unrelated or unbounded prose',()=>{
 for(const text of ['Search for a card named "Aura with soul tether you owned".','An Aura with unknown tether you owned.','Soul tether you owned.']) assert.equal(expandGrantedTriggeredPrograms(text,[def]),text);
});
