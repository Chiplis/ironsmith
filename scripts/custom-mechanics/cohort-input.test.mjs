import test from 'node:test';
import assert from 'node:assert/strict';
import { materializeCard, needsMissingFaces } from './cohort-input.mjs';
import { validateMechanic } from '../../web/ui/src/lib/custom-mechanics.js';

const definitions = [validateMechanic({keyword: 'Thought practice', reminder: 'Draw a card.'})];

test('a single-face planar card is expanded without inventing another face', () => {
  const card = {name: 'Practice grounds', source_layout: 'planar',
    original_text: 'Type: Plane — Test\nWhenever chaos ensues, thought practice.'};
  assert.equal(needsMissingFaces(card), false);
  const [face] = materializeCard(card, definitions);
  assert.match(face.executable_text, /^Type: Plane — Test\n/);
  assert.match(face.executable_text, /draw a card/i);
  assert.deepEqual(face.custom_keyword_identities, ['thought practice']);
});

test('both source faces retain their own metadata and expansion', () => {
  const card = {name: 'Second pupil', source_shape: 'doubleface', source_faces: [
    {name: 'First pupil', original_text: 'Mana cost: {1}\nType: Creature\nPower/Toughness: 1/1\nThought practice.'},
    {name: 'Second pupil', original_text: 'Type: Creature\nPower/Toughness: 3/3\nThought practice.\nFlying'},
  ]};
  const faces = materializeCard(card, definitions);
  assert.equal(faces.length, 2);
  assert.deepEqual(faces.map(face => face.name), ['First pupil', 'Second pupil']);
  assert.match(faces[0].executable_text, /Mana cost: \{1\}/);
  assert.match(faces[0].executable_text, /Power\/Toughness: 1\/1/);
  assert.match(faces[1].executable_text, /Power\/Toughness: 3\/3/);
  assert.match(faces[1].executable_text, /Flying/);
  for (const face of faces) assert.match(face.executable_text, /Draw a card/);
});

test('a known linked layout cannot silently use only a representative face', () => {
  const card = {name: 'Incomplete pupil', source_shape: 'doubleface', original_text: 'Type: Creature\nThought practice.'};
  assert.equal(needsMissingFaces(card), true);
  assert.throws(() => materializeCard(card, definitions), /all linked faces/);
});


test('source-supplied token definitions survive references introduced by mechanic expansion', () => {
  const token = {face:{name:'Shared banner'},input:{name:'Shared banner',manaCost:'',cardTypes:['Enchantment'],oracleText:'Attacking creatures you control get +1/+0.'}};
  const card = {name:'Banner bearer',original_text:'Type: Creature\nPower/Toughness: 1/1\nRaise banner.',customTokens:[token]};
  const before=structuredClone(card);
  const definitions=[validateMechanic({keyword:'Raise banner',reminder:'Create a Shared banner token.'})];
  const [face]=materializeCard(card,definitions);
  assert.match(face.executable_text,/Create a colorless artifact token named Shared banner/i);
  assert.equal(face.custom_token_definitions.length,1);
  assert.equal(face.custom_token_definitions[0].name,'Shared banner');
  assert.match(face.custom_token_definitions[0].executable_text,/Type: Enchantment/);
  assert.match(face.custom_token_definitions[0].executable_text,/Attacking creatures you control get \+1\/\+0/);
  assert.deepEqual(face.custom_tokens,[token]);
  assert.deepEqual(card,before);
});


test('source token aliases cannot silently discard a conflicting definition', () => {
 const token = name => ({face:{name},input:{name,manaCost:'',cardTypes:['Artifact'],oracleText:''}});
 assert.throws(() => materializeCard({name:'Conflicting supplier',original_text:'Type: Creature\nRaise banner.',custom_tokens:[token('First banner')],customTokens:[token('Second banner')]},[]),/Conflicting source and draft/);
});

import {readFileSync} from 'node:fs';
test('actual duplicated-article numeric source operand corrects only executable input and retains evidence',()=>{
 const fixture=JSON.parse(readFileSync(new URL('../../fixtures/custom_mechanics/revolution-gorge.json',import.meta.url),'utf8'));
 const row=fixture.mechanics.find(row=>row.representative_cards.some(card=>card.source_card_key==='Markov Imperator_IWH'));
 const card=row.representative_cards.find(card=>card.source_card_key==='Markov Imperator_IWH');
 const frozen=JSON.stringify(card);assert.match(card.original_text,/where X is the the number of creature cards/);
 const [face]=materializeCard(card,row.executable_definitions.map(validateMechanic),row.scope);
 assert.match(face.executable_text,/where X is the number of creature cards in your graveyard/);
 assert.ok(!/where X is (?:the the|double|twice)/i.test(face.executable_text));
 assert.equal(JSON.stringify(card),frozen,'source import must not mutate authoritative original fields');
 assert.equal(card.source_faces[0].original_text,card.original_text);
});
