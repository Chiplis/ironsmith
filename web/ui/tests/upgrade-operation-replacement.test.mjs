import test from 'node:test';
import assert from 'node:assert/strict';
import { validateMechanic, compileDraftInput } from '../src/lib/custom-mechanics.js';
import { namedActionProgramDefinition, expandNamedActionReplacementReferences } from '../src/lib/reminder-named-action-programs.js';
const reminder="Choose a kind of counter that {Object} doesn't have on it from among {CounterKinds}. Put a counter of that kind on {Object}.";
const definition=validateMechanic({keyword:'Upgrade',template:'{Object}',reminder,actionProgram:true,collectiveObject:true,completionRequirement:'all-choices-available'});
test('a closed menu remains inside its proposed operation replacement boundary',()=>{
 const original='Upgrade target creature.\n• Flying\n• Lifelink';
 const face=compileDraftInput({faces:[{name:'Operation source',oracleText:original}]},[definition]).faces[0];
 assert.match(face.oracleText,/Perform the upgrade action on target creature as follows, completing the action only if all choices were available: \[Choose a kind of counter that the proposed objects doesn't have on it from among flying and lifelink; Put a counter of that kind on the proposed objects\]/i);
 assert.equal(face.displayOracleText,original);
});
test('filtered singular replacement retains recipient and actor independently',()=>{
 const original='Flash\nIf you would upgrade a creature you control, you may put a +1/+1 counter on it instead.\n{3}{U}: Put a +1/+1 counter on Affluent Cetanthrope and draw a card.';
 const face=compileDraftInput({faces:[{name:'Affluent Cetanthrope',oracleText:original}]},[definition]).faces[0];
 assert.match(face.oracleText,/If you would perform the upgrade action with one or more creatures you control, you may put a \+1\/\+1 counter on it instead/i);
 assert.equal(face.displayOracleText,original);
});
test('source-declared arbitrary action and explicit other player preserve operation ownership',()=>{
 const alternate=validateMechanic({keyword:'Develop',template:'{Object}',reminder,actionProgram:true,collectiveObject:true,completionRequirement:'all-choices-available'});
 const text="At the beginning of enchanted player's upkeep, that player develops a creature you control.\n• Flying\n• Lifelink";
 const output=compileDraftInput({faces:[{name:'Curse',oracleText:text}]},[alternate]).faces[0].oracleText;
 assert.match(output,/that player chooses a creature you control\. That player performs the develop action on that creature as follows, completing the action only if all choices were available:/i);
 assert.match(output,/Choose a kind of counter that the proposed objects doesn't have on it/i);
});
test('quoted operation replacement is opaque and undeclared object programs fail closed',()=>{
 const original='This creature has "If you would upgrade a creature you control, you may put a +1/+1 counter on it instead."';
 const output=compileDraftInput({faces:[{name:'Quoted',oracleText:original}]},[definition]).faces[0].oracleText;
 assert.equal(output,original);
 const ordinary=validateMechanic({keyword:'Upgrade {Object}',reminder});
 assert.throws(()=>compileDraftInput({faces:[{name:'Undeclared',oracleText:'If you would upgrade a creature you control, you may put a +1/+1 counter on it instead.'}]},[ordinary]),/explicit collective object action program/);
});

test('explicit collective operation expansion is idempotent',()=>{
 const once=namedActionProgramDefinition(definition);
 assert.deepEqual(namedActionProgramDefinition(once),once);
 assert.throws(()=>namedActionProgramDefinition({...once,expansion:once.expansion.replace(', completing the action only if all choices were available','')}),/explicit completion requirement/);
});
test('singular operation filters pluralize their final object head, preserving compound types',()=>{
 for(const [singular,plural] of [['a creature card you control','creature cards you control'],['an artifact creature you control','artifact creatures you control']]){
  const input=`If you would upgrade ${singular}, you may put a +1/+1 counter on it instead.`;
  assert.equal(expandNamedActionReplacementReferences(input,[definition]),`If you would perform the upgrade action with one or more ${plural}, you may put a +1/+1 counter on it instead.`);
 }
});

test('completion contract is explicit executable definition identity',()=>{
 assert.equal(definition.completionRequirement,'all-choices-available');
 assert.throws(()=>validateMechanic({keyword:'Develop',template:'{Object}',reminder,completionRequirement:'all-choices-available'}),/explicit collective action program/);
 assert.throws(()=>validateMechanic({keyword:'Develop',template:'{Object}',reminder,actionProgram:true,collectiveObject:true,completionRequirement:'some-counters-placed'}),/all-choices-available/);
});
