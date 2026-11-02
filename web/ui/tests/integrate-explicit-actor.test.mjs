import test from 'node:test';
import assert from 'node:assert/strict';
import {compileDraftInput,validateMechanic} from '../src/lib/custom-mechanics.js';
const definition=validateMechanic({keyword:'Ascribe',template:'{Object}',actionProgram:true,collectiveObject:true,reminder:'Exile {Object}. For each card exiled this way, do the following: [You may do the following: [Choose a nonland permanent; Link the iterated card to the chosen permanent under the archived relationship.]].'});
test('explicit you actor invokes an ordinary declared collective program',()=>{
 const output=compileDraftInput({faces:[{name:'Witness',oracleText:'Target opponent chooses a nontoken creature they control. You ascribe that creature.'}]},[definition]).faces[0].oracleText;
 assert.doesNotMatch(output,/\byou ascribe\b/i);
 assert.match(output,/Perform the Ascribe action on that creature as follows:/i);
 assert.doesNotMatch(output,/\bYou Perform/i);
});
