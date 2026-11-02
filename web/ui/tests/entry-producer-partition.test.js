import test from 'node:test';
import assert from 'node:assert/strict';
import {validateMechanic,compileDraftInput} from '../src/lib/custom-mechanics.js';
import {bindLinkedEntryExileReference as bind} from '../src/lib/linked-entry-exile-references.js';
const definition=validateMechanic({keyword:'Moon Spark',linkedExileVerb:'sampled',reminder:'As this creature enters, mill three cards, then exile exactly one card in your graveyard milled this way.'});
test('a named entry verb records and reads its own generic producer designation',()=>{
 assert.match(definition.expansion,/^Keyword ability "moon spark" — As this creature enters/);
 const original='Moon Spark\nWhen Test Source enters, if it sampled a creature card, draw a card.';
 const face=compileDraftInput({faces:[{name:'Test Source',oracleText:original}]},[definition]).faces[0];
 assert.match(face.oracleText,/was exiled as this creature entered by its ability with designation "moon spark"/);
 assert.equal(face.displayOracleText,original);
 const text='Spells you cast that share a type with the card it sampled cost {1} less to cast.';
 assert.equal(bind(text,definition,'Test Source'),'Spells you cast that share a type with the card exiled as this creature entered by its ability with designation "moon spark" cost {1} less to cast.');
});
test('both token quantities and optional alternatives keep the complete producer name',()=>{
 const value=bind("Create a black Zombie creature token with power equal to the sampled card's power and toughness equal to that card's toughness.",definition,'Test Source');
 assert.equal((value.match(/by its ability with designation "moon spark"/g)||[]).length,2);
 const literal='A token named "the card it sampled". Another Creature sampled a creature card.';
 assert.equal(bind(literal,definition,'Test Source'),literal);
});
