import test from 'node:test';
import assert from 'node:assert/strict';
import {expandStatusReferences} from '../src/lib/custom-status-mechanics.js';
const definition={keyword:'Bright',reminder:'Give this creature the bright status.'};
test('a definition-bound contracted state remains a state after an unrelated attack event',()=>{
 assert.equal(expandStatusReferences("Whenever this creature attacks, if it's bright, draw a card.",definition),
  'Whenever this creature attacks, if it has the bright status, draw a card.');
 assert.equal(expandStatusReferences('Whenever this creature attacks, if it’s bright, draw a card.',definition),
  'Whenever this creature attacks, if it has the bright status, draw a card.');
});
test('a contracted status event and literal token name have distinct ownership',()=>{
 assert.equal(expandStatusReferences("When it's bright, draw a card.",definition),'When it gains the bright status, draw a card.');
 const literal='Create a token named "It\'s bright".';
 assert.equal(expandStatusReferences(literal,definition),literal);
 const unrelated="Whenever this creature attacks, if it's gloomy, draw a card.";
 assert.equal(expandStatusReferences(unrelated,definition),unrelated);
});
