import test from 'node:test';import assert from 'node:assert/strict';
import {validateMechanic,compileDraftInput} from '../src/lib/custom-mechanics.js';
const def=validateMechanic({keyword:'Engorge',reminder:'As this creature enters, mill three cards, then exile exactly one card in your graveyard milled this way.',linkedExileVerb:'engorged'});
test('actual Engorge source condition binds historical typed entry receipt',()=>{
 const text='Engorge\nWhen Abhorrent Exhumer enters, if it engorged a creature card, you may return target creature card from your graveyard to your hand.';
 const face=compileDraftInput({faces:[{name:'Abhorrent Exhumer',oracleText:text}]},[def]).faces[0];
 assert.match(face.oracleText,/a creature card was exiled as this creature entered/);assert.match(face.oracleText,/exile exactly one card in your graveyard milled this way/);assert.equal(face.displayOracleText,text);
});
test('explicit linked-entry verbs preserve unrelated actors and quoted rules',()=>{
 const text='Another Creature engorged a creature card. This creature gains "If it engorged a creature card, draw a card."';
 assert.equal(compileDraftInput({faces:[{name:'Witness',oracleText:text}]},[def]).faces[0].oracleText,text);
});
