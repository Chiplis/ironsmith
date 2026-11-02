import test from 'node:test';
import assert from 'node:assert/strict';
import {compileDraftInput, mergeMechanics} from '../src/lib/custom-mechanics.js';
const expand=(text,definition)=>compileDraftInput({faces:[{name:'Guide',oracleText:text}]},mergeMechanics([],[definition])).faces[0].oracleText;
test('quoted numeric actions expand only inside explicitly granted triggers',()=>{
 const def={keyword:'Nourish',template:'{Number}',reminder:'You gain {Number} life.'};
 assert.equal(expand('Other nontoken creatures you control have “When this creature enters, nourish 1.”',def),'Other nontoken creatures you control have “When this creature enters, You gain 1 life.”');
 assert.equal(expand('Name a card named “Nourish 1”.',def),'Name a card named “Nourish 1”.');
});
test('repeated counted additional costs remain a scaled cost',()=>{
 const def={keyword:'Reappropriate',reminder:'Exile exactly two cards from your hand and/or graveyard.'};
 assert.equal(expand('As an additional cost to cast Guide, reappropriate X times.',def),'As an additional cost to cast Guide, Exile exactly two times X cards from your hand and/or graveyard.');
 assert.equal(expand('Reappropriate X times.',def),'Repeat the following process X times: Exile exactly two cards from your hand and/or graveyard.');
});
test('optional complete actions preserve the outer decision and inner fallback',()=>{
 const def={keyword:'Crystallize',reminder:"You may put a +1/+1 counter on a creature you control. If you don't, create a Treasure token."};
 assert.equal(expand('Whenever a creature dies, you may crystallize. You gain 1 life.',def),"Whenever a creature dies, You may do the following: You may put a +1/+1 counter on a creature you control; If you don't, create a Treasure token. You gain 1 life.");
});
test('optional subject actions bind a literal suffix after the verb',()=>{
 const def={keyword:'Consume',template:'{Subject} a land',reminder:'Choose {Subject}. Choose one —\n• Sacrifice a land you control other than the chosen creature.\n• Exile a land card from your graveyard.'};
 assert.equal(expand('Whenever Guide attacks, you may have it consume a land. If you do, draw a card.',def),'Whenever Guide attacks, You may do the following: Choose it; Choose one —\n• Sacrifice a land you control other than the chosen creature;\n• Exile a land card from your graveyard. If you do, draw a card.');
});
