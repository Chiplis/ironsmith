import test from 'node:test';
import assert from 'node:assert/strict';
import {expandNamedStatusCardReferences,expandPersistentPlayerActionPrograms} from '../src/lib/reminder-persistent-player-programs.js';
const reminder='Exile the top card of your library face up. Give it the charted status while in exile. Register a persistent rule named "charted retrieval" for you with condition "You own a card in exile with the charted status" and ability "Whenever a creature you control deals combat damage to an opponent, put a card you own in exile with the charted status into your hand."';
const definition={keyword:'exploreplace',reminder};
test('named status card descriptor follows declaration data without granting keyword',()=>{
 assert.equal(expandNamedStatusCardReferences('Whenever you put a charted card into your hand, draw a card.',[definition]),'Whenever you put a card with the charted status into your hand, draw a card.');
 assert.equal(expandNamedStatusCardReferences('A different card',[definition]),'A different card');
});
test('other player binds immediate actions while quoted rule keeps its own you',()=>{
 const text=expandPersistentPlayerActionPrograms('Put it on top of its owner\'s library, then they exploreplace.',definition);
 assert.match(text,/then they exile/);assert.match(text,/that player\'s library/);assert.match(text,/for that player with condition/);assert.match(text,/"You own a card/);assert.match(text,/Whenever a creature you control/);
});
