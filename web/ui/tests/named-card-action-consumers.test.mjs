import test from 'node:test';
import assert from 'node:assert/strict';
import {expandNamedActionReferences} from '../src/lib/reminder-action-references.js';
import {expandStatusReferences,expandStatusActions} from '../src/lib/custom-status-mechanics.js';
const d={keyword:'Archive',perObjectProgram:true,template:'{Object}',reminder:'Give the most recently exiled card the archived status in exile and on the spell it becomes. Perform the archive action with the most recently exiled card.',forms:{eventVerb:'archive',eventLabel:'archive',participle:'archived',designation:'archived'}};
test('singular and shared-operand action watchers retain actual object receipt and alternative events',()=>{
 assert.equal(expandNamedActionReferences('Whenever you archive a card, draw a card.',[d]),'Whenever you perform the archive action with a card, draw a card.');
 assert.equal(expandNamedActionReferences('Whenever you draw or archive a card, you gain 1 life.',[d]),'Whenever you draw a card or perform the archive action with a card, you gain 1 life.');
});
test('zone card/spell designation references do not replace the complete per-object action',()=>{
 assert.equal(expandStatusReferences('Whenever you cast an archived spell, draw a card.',d),'Whenever you cast a spell with the archived status, draw a card.');
 assert.equal(expandStatusActions('Archive this creature.',d),'Archive this creature.');
});
