import test from 'node:test';
import assert from 'node:assert/strict';
import {compileDraftInput,validateMechanic} from '../src/lib/custom-mechanics.js';
import {expandSimplePlayerActionDeclarations} from '../src/lib/reminder-inline-parameters.js';
const definition=validateMechanic({keyword:'Forebode',reminder:'Create a tapped colorless Omen artifact token with "{T}, Discard a card, Sacrifice this artifact: Draw a card.".'});
const compile=oracleText=>compileDraftInput({faces:[{name:'Ordinary source',oracleText}]},[definition]).faces[0].oracleText;
test('third-person bare action retains its explicit player and coordinated actor',()=>{
 assert.match(compile('That player forebodes.'),/That player Create a tapped colorless Omen artifact token/i);
 const source='Target opponent reveals their hand. You may choose a nonland card from it. If you do, that player discards that card, then forebodes. You lose 1 life.';
 assert.match(compile(source),/that player discards that card, then that player Create a tapped colorless Omen artifact token/i);
 assert.match(compile(source),/You lose 1 life\.$/);
});
test('coordination cannot cross another clause or rewrite a literal name',()=>{
 const literal='Choose a card named "that player discards that card, then forebodes".';
 assert.equal(compile(literal),literal);
 const source='That player discards a card, you draw a card, then forebodes.';
 assert.doesNotMatch(compile(source),/then that player Create/i);
});

test('another explicit player clause joined by and invalidates initial actor carry',()=>{
 for (const [source, wrongActor] of [
  ['You reveal a card and that player draws a card, then forebodes.','you'],
  ['They reveal a card and you draw a card, then forebode.','they'],
 ]) {
  assert.equal(expandSimplePlayerActionDeclarations(source,definition),source,'ambiguous actor carry must decline without inventing an earlier actor');
  assert.doesNotMatch(compile(source),new RegExp(`then ${wrongActor} Create`,'i'));
 }
 const literal='That player reveals a card named "and you draw a card", then forebodes.';
 const result=expandSimplePlayerActionDeclarations(literal,definition);
 assert.match(result,/named "and you draw a card"/);
 assert.match(result,/then that player Create a tapped colorless Omen artifact token/i);
});

test('a quoted comma and then keyword never becomes an action coordinator',()=>{
 const source='That player reveals a card named "foo, then forebodes.", then draws a card.';
 assert.equal(expandSimplePlayerActionDeclarations(source,definition),source);
 assert.equal(compile(source),source,'literal card-name bytes must remain unchanged');
});
test('or and but with another explicit player also invalidate actor carry',()=>{
 for (const [source, wrongActor] of [
  ['That player discards a card or you sacrifice a permanent, then forebodes.','that player'],
  ['They reveal a card but you draw a card, then forebode.','they'],
 ]) {
  assert.equal(expandSimplePlayerActionDeclarations(source,definition),source);
  assert.doesNotMatch(compile(source),new RegExp(`then ${wrongActor} Create`,'i'));
 }
});

test('all explicit player reference subjects invalidate earlier actor evidence',()=>{
 for (const source of [
  'You reveal a card and target opponent draws a card, then forebodes.',
  'You reveal a card and another player draws a card, then forebodes.',
  "You reveal a card and that creature's controller draws a card, then forebodes.",
  'You reveal a card and your teammate draws a card, then forebodes.',
 ]) {
  assert.equal(expandSimplePlayerActionDeclarations(source,definition),source);
  assert.doesNotMatch(compile(source),/then you Create/i);
 }
 const sameActor='That player reveals a card and draws a card, then forebodes.';
 assert.match(expandSimplePlayerActionDeclarations(sameActor,definition),/then that player Create/i);
});
