import test from 'node:test';
import assert from 'node:assert/strict';
import {validateMechanic,compileDraftInput} from '../src/lib/custom-mechanics.js';
const group=validateMechanic({keyword:'Retinue — {TokenSizes}',reminder:'When this permanent enters, create {TokenSizes} red Army creature tokens. Group the tokens created this way under this permanent so all members attack and block if any member or the owner does, if able.'});
test('size lists remain one triggered program with a shared token blueprint',()=>{
 const face=compileDraftInput({faces:[{name:'Bearer',oracleText:'Retinue — 1/1, 2/2, 3/3'}]},[group]).faces[0];
 assert.equal((face.oracleText.match(/When this permanent enters/g)||[]).length,1);
 assert.match(face.oracleText,/create a 1\/1 red Army creature token and a 2\/2 red Army creature token and a 3\/3 red Army creature token/);
 assert.match(face.oracleText,/Group the tokens created this way under this permanent/);
});
test('variable size-list quantities bind their explicit value expression',()=>{
 const face=compileDraftInput({faces:[{name:'Bearer',oracleText:'Retinue — X 1/1s, where X is the number of lands you control.'}]},[group]).faces[0];
 assert.match(face.oracleText,/create X 1\/1 red Army creature tokens, where X is the number of lands you control/);
});
test('group references use supplied group semantics without granting a declaration identity',()=>{
 const original="Creatures in Bearer's retinue have reach.";
 const face=compileDraftInput({faces:[{name:'Bearer',oracleText:original}]},[group]).faces[0];
 assert.equal(face.oracleText,'Creatures in group owned by Bearer have reach.');
 assert.equal(face.displayOracleText,original);
});

test('each size receives the whole shared token blueprint, including its abilities',()=>{
 const def=validateMechanic({keyword:'Assembly — {TokenSizes}',reminder:'When this permanent enters, create {TokenSizes} blue Bird creature tokens with flying.'});
 const face=compileDraftInput({faces:[{name:'Maker',oracleText:'Assembly — 1/1, 2/2'}]},[def]).faces[0];
 assert.match(face.oracleText,/a 1\/1 blue Bird creature token with flying and a 2\/2 blue Bird creature token with flying/);
});
