import test from 'node:test';
import assert from 'node:assert/strict';
import {compileDraftInput} from '../../web/ui/src/lib/custom-mechanics.js';
const defs=[
 {keyword:'Judge {Object}',actionProgram:true,reminder:'You choose honor or condemn. If you chose honor, perform the honor action with {Object}, then put a +1/+1 counter on {Object}. If you chose condemn, perform the condemn action with {Object}, then sacrifice {Object}, then draw a card.'},
 {keyword:'Honor {Object}',reminder:'Perform the honor action with {Object}.',forms:{eventVerb:'honor',eventLabel:'honor'}},
 {keyword:'Condemn {Object}',reminder:'Perform the condemn action with {Object}.',forms:{eventVerb:'condemn',eventLabel:'condemn'}},
];
const compile=text=>compileDraftInput({faces:[{name:'Receipt witness',oracleText:text}]},defs).faces[0].oracleText;
test('a real complete Judge invocation binds its honor receipt without an undefined quote guard',()=>{
 const output=compile('Judge target noncreature artifact you control. If you honor it, it becomes a 0/0 Construct artifact creature with flying.');
 assert.match(output,/If the preceding program included the honor action/i);
});
test('a quoted branch condition is never rebound to an enclosing prior action',()=>{
 const literal='If you honor it, draw a card.';
 const output=compile(`Judge target creature you control. Create a 1/1 white Soldier creature token named "${literal}".`);
 assert.ok(output.includes(`named "${literal}"`),output);
});
test('a quoted invocation cannot provide the original program receipt for a later condition',()=>{
 const output=compile('Create a 1/1 white Soldier creature token named "Judge target creature you control." If you honor it, draw a card.');
 assert.doesNotMatch(output,/preceding program included/);
 assert.ok(output.includes('If you honor it'),output);
});
