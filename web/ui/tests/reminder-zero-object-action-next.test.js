import test from 'node:test';
import assert from 'node:assert/strict';
import {compileDraftInput,mergeMechanics} from '../src/lib/custom-mechanics.js';
const definitions=mergeMechanics([],[{keyword:'Advanceguard',reminder:'When this creature enters, perform the attack practice action.',forms:{eventVerb:'drill',eventLabel:'attack practice'}}]);
test('a zero-object source event watcher retains performer-relative pronouns',()=>{
 const original='Advanceguard\nWhen Practicing Guard drills, it gains flying until end of turn.';
 const face=compileDraftInput({faces:[{name:'Practicing Guard',oracleText:original}]},definitions).faces[0];
 assert.match(face.oracleText,/When this permanent performs the attack practice action, this permanent gains flying until end of turn\./);
 assert.equal(face.displayOracleText,original);
 assert.equal(compileDraftInput({faces:[{name:'Practicing Guard',oracleText:'When Other Guard drills, it gains flying until end of turn.'}]},definitions).faces[0].oracleText,'When Other Guard drills, it gains flying until end of turn.');
});
