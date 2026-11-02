import test from 'node:test';
import assert from 'node:assert/strict';
import {expandNamedActionReferences} from '../src/lib/reminder-action-references.js';
const definitions=[{keyword:'Waste',reminder:'Draw a card.',forms:{eventVerb:'waste',eventLabel:'waste'}}];
test('player one-or-more clauses observe one completed grouped receipt',()=>{
 const source='Whenever you waste one or more lands, each opponent loses 2 life, you gain 2 life, and you draw a card.';
 assert.equal(expandNamedActionReferences(source,definitions),'Whenever you perform the waste action with lands, each opponent loses 2 life, you gain 2 life, and you draw a card.');
 assert.equal(expandNamedActionReferences('Whenever an opponent wastes one or more artifacts, draw a card.',definitions),'Whenever an opponent performs the waste action with artifacts, draw a card.');
});
test('singular per-object and unsupported thresholds do not silently change event multiplicity',()=>{
 for(const source of ['Whenever you waste a land, draw a card.','Whenever you waste two or more lands, draw a card.']) assert.equal(expandNamedActionReferences(source,definitions),source);
});
test('a spell-local discount reads completed objects rather than global action frequency',()=>{
 const source='This spell costs {1} less to cast for each land wasted this way.';
 assert.equal(expandNamedActionReferences(source,definitions),'This spell costs {1} less to cast for each land affected this way.');
 assert.equal(expandNamedActionReferences('This spell costs {1} less to cast for each action performed this turn.',definitions),'This spell costs {1} less to cast for each action performed this turn.');
});
