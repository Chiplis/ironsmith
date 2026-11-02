import test from 'node:test';
import assert from 'node:assert/strict';
import {expandCardCreationPrograms} from '../src/lib/reminder-card-creation.js';
const defs=[{keyword:'Conjure',declarationKind:'card-creation'}];
test('singular and batch observers preserve typed participant multiplicity and actor',()=>{
 assert.equal(expandCardCreationPrograms('Whenever you conjure a card, draw a card.',defs),'Whenever a card participates in the conjure action performed by you, draw a card.');
 assert.equal(expandCardCreationPrograms('Whenever you conjure one or more land cards into your hand, create a Human token.',defs),'Whenever one or more land cards in your hand participate in the conjure action performed by you, create a Human token.');
 assert.equal(expandCardCreationPrograms('Whenever an opponent conjures a land card into their hand, gain 1 life.',defs),'Whenever a land card in a hand participates in the conjure action performed by an opponent, gain 1 life.');
});
test('literal names and unknown destination evidence remain unchanged',()=>{
 const literal='Choose a card named "Whenever you conjure a card,".';
 assert.equal(expandCardCreationPrograms(literal,defs),literal);
 const unsupported='Whenever you conjure a card underneath your library, draw a card.';
 assert.equal(expandCardCreationPrograms(unsupported,defs),unsupported);
});
