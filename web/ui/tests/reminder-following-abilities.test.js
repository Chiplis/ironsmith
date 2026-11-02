import test from 'node:test';
import assert from 'node:assert/strict';
import {compileDraftInput,validateMechanic} from '../src/lib/custom-mechanics.js';
import {expandFollowingAbilities} from '../src/lib/reminder-following-abilities.js';
const definition=validateMechanic({keyword:'Kinship',reminder:'Each other creature you control with counters on it has the following abilities.'});
test('a positional reminder grants only subsequent abilities and preserves all source abilities',()=>{
 const text='Flying\nKinship\nTrample\n{1}: Draw a card.';
 const expanded=compileDraftInput({faces:[{name:'Example',oracleText:text}]},[definition]).faces[0];
 assert.equal(expanded.oracleText,'Flying\nEach other creature you control with counters on it has “Trample” and “{1}: Draw a card”.\nTrample\n{1}: Draw a card.');
 assert.deepEqual(expanded.customKeywordIdentities,['kinship']);
 assert.equal(expanded.displayOracleText,text);
});
test('positional reminders reject absent or ambiguous following abilities',()=>{
 assert.throws(()=>expandFollowingAbilities('Kinship',definition),/subsequent ability/);
 assert.throws(()=>expandFollowingAbilities('Kinship\nFlying\nKinship\nTrample',definition),/unambiguous position/);
 assert.equal(expandFollowingAbilities('Flying',definition),null);
});
test('activation payment reminder modifies arbitrary named activation without consuming colored costs',()=>{
 const payment=validateMechanic({keyword:'Collaboration',reminder:"For each mana in this ability's activation cost, you may tap an untapped creature you control rather than pay that mana."});
 const face=compileDraftInput({faces:[{name:'Example',oracleText:'Collaboration — {3}{U}: Draw a card.'}]},[payment]).faces[0];
 assert.equal(face.oracleText,"{3}{U}: Draw a card. For each mana in this ability's activation cost, you may tap an untapped creature you control rather than pay that mana.");
 assert.deepEqual(face.customKeywordIdentities,['collaboration']);
});
test('costed donor scope binds only the non-designated source tail',()=>{
 const donor=validateMechanic({keyword:'Bestowal',template:'{Cost}',reminder:'{Cost}, Exile this card from your hand: Put a +1/+1 counter on target creature. It gains the non-bestowal abilities printed below this ability until end of turn.'});
 const face=compileDraftInput({faces:[{name:'Donor',oracleText:'Flying\nBestowal {2}{R}\nMenace\nWhenever this creature attacks, draw a card.'}]},[donor]).faces[0];
 assert.equal(face.oracleText,'Flying\n{2}{R}, Exile this card from your hand: Put a +1/+1 counter on target creature. It gains “Menace” and “Whenever this creature attacks, draw a card” until end of turn.\nMenace\nWhenever this creature attacks, draw a card.');
});
