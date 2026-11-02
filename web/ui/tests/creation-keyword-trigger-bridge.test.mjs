import test from 'node:test';
import assert from 'node:assert/strict';
import {expandCardCreationPrograms} from '../src/lib/reminder-card-creation.js';
const definition={keyword:'Conjure',declarationKind:'card-creation',forms:{variants:['Conjuring']}};
test('declared operation gerund invokes actual official keyword programs',()=>{
 assert.equal(expandCardCreationPrograms('Prowess\nConjuring one or more cards causes prowess abilities of permanents you control to trigger.',[definition]),'Prowess\nAs a player performs the conjure action with an amount of 1 or more, trigger each triggered ability with keyword prowess from permanents you control.');
 assert.equal(expandCardCreationPrograms('Conjuring one or more cards causes mystery abilities of permanents you control to trigger.',[definition]),'Conjuring one or more cards causes mystery abilities of permanents you control to trigger.');
 assert.equal(expandCardCreationPrograms('Choose a card named "Conjuring one or more cards causes prowess abilities of permanents you control to trigger."',[definition]),'Choose a card named "Conjuring one or more cards causes prowess abilities of permanents you control to trigger."');
});

test('classification is official ability identity, with no inferred custom gerund or action identity',()=>{
 const custom={keyword:'Call',declarationKind:'card-creation',forms:{variants:['Calling']}};
 assert.equal(expandCardCreationPrograms('Calling one or more cards causes prowess abilities of permanents you control to trigger.',[custom]),'As a player performs the call action with an amount of 1 or more, trigger each triggered ability with keyword prowess from permanents you control.');
 const undeclared={keyword:'Conjure',declarationKind:'card-creation'};
 const source='Conjuring one or more cards causes prowess abilities of permanents you control to trigger.';
 assert.equal(expandCardCreationPrograms(source,[undeclared]),source,'gerund must come from explicit source vocabulary');
 const action='Conjuring one or more cards causes draw abilities of permanents you control to trigger.';
 assert.equal(expandCardCreationPrograms(action,[definition]),action,'a keyword action is not an official keyword ability');
});
