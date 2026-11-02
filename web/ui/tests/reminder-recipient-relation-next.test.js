import test from 'node:test';import assert from 'node:assert/strict';
import{compileDraftInput,mergeMechanics}from'../src/lib/custom-mechanics.js';
const raw={keyword:'Heartening',reminder:'{T}: Another target creature you control gets +1/+1 until end of turn. Perform the encourage action with that creature. Activate only as a sorcery.',forms:{participle:'encouraged',eventVerb:'encourage',eventLabel:'encourage'}};
test('an explicit action participle binds the current source performer and preserves display',()=>{
 const definitions=mergeMechanics([],[raw]);const original='Heartening\nWhenever a creature deals combat damage to a player, if it was encouraged by Mentor this turn, draw a card.';
 const face=compileDraftInput({faces:[{name:'Mentor',oracleText:original}]},definitions).faces[0];assert.match(face.oracleText,/if it was affected by the encourage action performed by this permanent this turn/);assert.equal(face.displayOracleText,original);
 const other='Whenever a creature deals combat damage to a player, if it was encouraged by Other Mentor this turn, draw a card.';assert.equal(compileDraftInput({faces:[{name:'Mentor',oracleText:other}]},definitions).faces[0].oracleText,other);
 const status=mergeMechanics([],[{...raw,forms:{participle:'encouraged'}}]);assert.match(compileDraftInput({faces:[{name:'Mentor',oracleText:original}]},status).faces[0].oracleText,/if it was encouraged by Mentor this turn/);
});
