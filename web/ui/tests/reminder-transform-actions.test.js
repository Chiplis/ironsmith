import test from 'node:test';
import assert from 'node:assert/strict';
import {bindSelfTransformActivationLabel as bind,expandSelfTransformActionReferences as refs} from '../src/lib/reminder-transform-actions.js';
const definition={keyword:'Glimmer',reminder:'{Cost}: Transform this permanent. Activate only as a sorcery.'};
test('only the explicitly expanded self-transform activation owns its action name',()=>{
 assert.equal(bind('{2}: Transform this permanent. Activate only as a sorcery.',definition),'{2}: Transform this permanent. Activate only as a sorcery. This is the glimmer activated ability.');
 const ordinary={keyword:'Glimmer',reminder:'{Cost}: Transform target permanent.'};
 assert.equal(bind('{2}: Transform target permanent.',ordinary),'{2}: Transform target permanent.');
});
test('a supplied action name binds the current face source and immutable activation cause',()=>{
 assert.equal(refs('When you glimmer Other Face, draw a card.',definition,'Other Face'),'When this permanent transforms caused by its glimmer activated ability you activated, draw a card.');
 assert.equal(refs('When you glimmer Other Face, draw a card.',definition,'Unrelated Face'),'When you glimmer Other Face, draw a card.');
 assert.equal(refs('When you transform Other Face, draw a card.',definition,'Other Face'),'When you transform Other Face, draw a card.');
 const quoted='This creature has "When you glimmer Other Face, draw a card."';assert.equal(refs(quoted,definition,'Other Face'),quoted);
});

test('single-activation ownership rejects a sibling activation colon',()=>{
 const multiple={keyword:'Glimmer',reminder:'{Cost}: Transform this permanent. {1}: Draw a card.'};
 const source='{2}: Transform this permanent. {1}: Draw a card.';
 assert.equal(bind(source,multiple),source);
 assert.equal(refs('When you glimmer Other Face, draw a card.',multiple,'Other Face'),'When you glimmer Other Face, draw a card.');
 const quoted={keyword:'Glimmer',reminder:'{Cost}: Transform this permanent. It gains "{1}: Draw a card."'};
 assert.match(bind('{2}: Transform this permanent.',quoted),/This is the glimmer activated ability/);
});
test('self action source operand is an exact literal including regexp punctuation',()=>{
 const name='Other Face (A+B)';
 assert.equal(refs(`When you glimmer ${name}, draw a card.`,definition,name),'When this permanent transforms caused by its glimmer activated ability you activated, draw a card.');
 const near='When you glimmer Other Face AAB, draw a card.';
 assert.equal(refs(near,definition,name),near);
});
