import test from 'node:test';
import assert from 'node:assert/strict';
import {compileDraftInput,validateMechanic} from '../src/lib/custom-mechanics.js';
import {expandActionParameterDeclarations} from '../src/lib/reminder-action-parameters.js';
const rule=validateMechanic({keyword:'Impartshade',template:'{Player}',reminder:"If this permanent's back face is a Blessing, create a token that's a copy of its back face attached to {Player}."});
test('declared player action preserves any-number targeted group and scopes a complete conditional per selected player',()=>{
 const face=compileDraftInput({faces:[{name:'Caller',oracleText:'When Caller enters, impartshade any number of target players.'}]},[rule]).faces[0];
 assert.equal(face.oracleText,"When Caller enters, For each of any number of target players, do the following: [If this permanent's back face is a Blessing, create a token that's a copy of its back face attached to that player].");
});
test('singular player action still selects one exact target and quoted references remain opaque',()=>{
 const text='When Caller enters, impartshade target player.';
 const face=compileDraftInput({faces:[{name:'Caller',oracleText:text}]},[rule]).faces[0];
 assert.match(face.oracleText,/For each target player, do the following: \[If .*attached to that player\]/);
 const quoted='This creature has "When this creature enters, impartshade any number of target players."';
 assert.equal(expandActionParameterDeclarations(quoted,rule,'Observer'),quoted);
});

test('a singular target action keeps its whole program beneath an enclosing condition',()=>{
 const text='Whenever Caller deals combat damage to a player, you get an experience counter. Then if you have three or more experience counters, impartshade target player.';
 const face=compileDraftInput({faces:[{name:'Caller',oracleText:text}]},[rule]).faces[0];
 assert.match(face.oracleText,/Then if you have three or more experience counters, For each target player, do the following: \[If .*attached to that player\]/);
 assert.doesNotMatch(face.oracleText,/Choose target player\./);
 assert.equal((face.oracleText.match(/target player/g)||[]).length,1);
});
