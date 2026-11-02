import test from 'node:test';
import assert from 'node:assert/strict';
import {compileDraftInput,validateMechanic,draftMechanics} from '../../web/ui/src/lib/custom-mechanics.js';
import {expandConditionalKeywordSharing} from '../../web/ui/src/lib/reminder-conditional-keyword-sharing.js';
const supplied=validateMechanic({keyword:'Echoing step',abilityDesignation:'echoing step',reminder:'Whenever this creature attacks, if it has fewer than three +1/+1 counters on it, put a +1/+1 counter on it.'});
test('a designated triggered program remains grantable and does not mark its Aura as the recipient',()=>{
 const result=compileDraftInput({faces:[{name:'Training gift',oracleText:'Enchanted creature has echoing step.'}]},[supplied]).faces[0];
 assert.match(result.oracleText,/Enchanted creature has custom keyword "echoing step", and "echoing step — Whenever this creature attacks/);
 assert.equal(result.customKeywordIdentities,undefined);
});
test('a source-owned same-is-true list splits only a supplied custom name with conditional full program',()=>{
 const oracle='As long as a creature card with flying is exiled by Archive, it has flying. The same is true for first strike, echoing step, and vigilance.';
 const defs=draftMechanics({faces:[{name:'Archive',oracleText:oracle}]},[supplied]); assert.equal(defs.length,1);
 const result=compileDraftInput({faces:[{name:'Archive',oracleText:oracle}]},defs).faces[0];
 assert.match(result.oracleText,/The same is true for first strike and vigilance/);
 assert.match(result.oracleText,/creature card with custom keyword "echoing step" is exiled by this permanent/);
 assert.match(result.oracleText,/this permanent has custom keyword "echoing step", and "echoing step — Whenever/);
 assert.equal(result.customKeywordIdentities,undefined);
});
test('foreign owners, names and unknown custom words are not silently classified',()=>{
 const original='As long as a creature card with flying is exiled by Another source, it has flying. The same is true for echoing step and vigilance.';
 assert.equal(expandConditionalKeywordSharing(original,[supplied],'Archive'),original);
 assert.equal(expandConditionalKeywordSharing('Create a token named Echoing Step.',[supplied],'Archive'),'Create a token named Echoing Step.');
 assert.equal(expandConditionalKeywordSharing(original,[],'Another source'),original);
});
