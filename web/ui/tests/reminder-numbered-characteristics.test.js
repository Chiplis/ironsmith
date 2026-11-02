import test from 'node:test';
import assert from 'node:assert/strict';
import {validateMechanic,compileDraftInput} from '../src/lib/custom-mechanics.js';
const def=()=>validateMechanic({keyword:'Mimic',template:'{Cost}',reminder:'{Cost}: Choose a number for this permanent. Activate only as a sorcery.',numberedCharacteristics:true});
test('numbered sections become independent number-gated characteristics and ordinary grants',()=>{
 const original='Mimic {2} [3/3]\n[Shape 1] Flying, double strike [3/3]\n[Shape 2] Whenever this creature takes this shape, put three +1/+1 counters on it. [4/4]';
 const result=compileDraftInput({faces:[{name:'Example',oracleText:original}]},[def()]).faces[0];
 assert.match(result.oracleText,/\{2\}: Choose a number for this permanent/);
 assert.match(result.oracleText,/chosen for this permanent is 1, this permanent has base power and toughness 3\/3/);
 assert.match(result.oracleText,/has "Whenever a number is chosen for this permanent, put three \+1\/\+1 counters on it\."/);
 assert.equal(result.displayOracleText,original);
 assert.deepEqual(result.customKeywordIdentities,['mimic']);
});
test('a plain unconfigured section and unrelated bracket syntax remain untouched',()=>{
 const original='[Shape 1] Flying [3/3]';
 assert.equal(compileDraftInput({faces:[{name:'Unknown',oracleText:original}]},[]).faces[0].oracleText,original);
 const bracket='Mimic {2}\nTarget [noncreature] permanent [you control].';
 assert.match(compileDraftInput({faces:[{name:'Brackets',oracleText:bracket}]},[def()]).faces[0].oracleText,/\[noncreature\]/);
 assert.throws(()=>compileDraftInput({faces:[{name:'Unclosed',oracleText:'Mimic {2}\n[Shape 1] Flying'}]},[def()]),/power and toughness/);
});

test('zero section cannot masquerade as an unchosen selection',()=>{
 const original='Mimic {2} [3/3]\n[Shape 0] Flying [4/4]';
 assert.throws(()=>compileDraftInput({faces:[{name:'Zero section',oracleText:original}]},[def()]),/unchosen-state evidence/);
});
