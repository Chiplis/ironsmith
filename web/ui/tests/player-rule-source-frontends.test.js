import test from 'node:test';
import assert from 'node:assert/strict';
import {compileDraftInput,validateMechanic} from '../src/lib/custom-mechanics.js';
const reminder='Exile the top card of your library face up. Give it the charted status while in exile. Register a persistent rule named "charted retrieval" for you with condition "You own a card in exile with the charted status" and ability "Whenever a creature you control deals combat damage to an opponent, put a card you own in exile with the charted status into your hand."';
test('whole draft binds other-player action and status consumers while preserving display',()=>{
 const definition=validateMechanic({keyword:'Exploreplace',reminder});
 const original="Put this card on top of its owner's library, then they exploreplace.\nWhenever you put a charted card into your hand, draw a card.";
 const face=compileDraftInput({faces:[{name:'Rale',oracleText:original}]},[definition]).faces[0];
 assert.equal(face.displayOracleText,original);
 assert.match(face.oracleText,/they exile the top card of that player's library/);
 assert.match(face.oracleText,/for that player with condition "You own a card/);
 assert.match(face.oracleText,/Whenever a creature you control/);
 assert.match(face.oracleText,/put a card with the charted status into your hand/);
});
test('inline player meaning works with no custom keyword definitions',()=>{
 const original="Target player gets three restlessness counters. *(At your first main phase, for each restlessness counter on you, mill a card. If it's nonland, you lose 1 life and the counter.)*";
 const face=compileDraftInput({faces:[{name:'Tavo',oracleText:original}]},[]).faces[0];
 assert.equal(face.displayOracleText,'Target player gets three restlessness counters.');
 assert.match(face.oracleText,/Register a persistent rule/);
 assert.match(face.oracleText,/for that player with event condition/);
 assert.match(face.oracleText,/If a nonland card was milled this way/);
 assert.equal(face.customKeywordIdentities,undefined);
});
test('ordinary rule declarations never bypass official keyword collision rejection',()=>{
 assert.throws(()=>validateMechanic({keyword:'Flying',reminder}),/official keywords cannot be redefined/);
});

test('stripped reminder italics do not leave delimiters or erase characteristic stars',()=>{
 const original="Type: Creature\nPower/Toughness: */*\nWhen this creature enters, target player gets three restlessness counters. *(At your first main phase, for each restlessness counter on you, mill a card. If it's nonland, you lose 1 life and the counter.)*";
 const face=compileDraftInput({faces:[{name:'Tavo',oracleText:original}]},[]).faces[0];
 assert.equal(face.displayOracleText,"Type: Creature\nPower/Toughness: */*\nWhen this creature enters, target player gets three restlessness counters.");
 assert.match(face.oracleText,/Power\/Toughness: \*\/\*/);
 assert.doesNotMatch(face.displayOracleText,/counters\. \*\*/);
});
