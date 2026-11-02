import test from 'node:test';
import assert from 'node:assert/strict';
import { definitionsForSource, indexSourceDefinitions, executableDefinitions } from './source-definitions.mjs';
import { materializeCard } from './cohort-input.mjs';
const first = {keyword:'Thought practice',reminder:'Draw a card.'};
const second = {keyword:'Gentle practice',reminder:'You gain 1 life.'};
const row = (id, card, definitions) => ({id,keyword:definitions[0].keyword,executable_definitions:definitions,representative_cards:[card]});
const cohort = (path, mechanics) => ({path,fixture:{mechanics}});
test('stable source key imports only that card candidates and preserves complete faces/originals', () => {
  const card = {id:'temporary-row-id',source_card_key:'source_SET',name:'Back pupil',source_shape:'doubleface',source_faces:[
    {name:'Front pupil',original_text:'Type: Creature\nPower/Toughness: 1/1\nThought practice.'},
    {name:'Back pupil',original_text:'Type: Creature\nPower/Toughness: 2/2\nGentle practice.'},
  ]};
  const original = JSON.stringify(card);
  const index = indexSourceDefinitions([
    cohort('a.json',[row('first',{id:'source_SET',name:'Different display name'},[first])]),
    cohort('b.json',[row('second',{source_card_key:'source_SET',name:'Other name'},[second])]),
    cohort('unrelated.json',[row('other',{id:'other_SET',name:'Back pupil'},[{keyword:'Unrelated practice',reminder:'You gain 9 life.'}])]),
  ]);
  const definitions = definitionsForSource(card,[first],index);
  assert.deepEqual(definitions.map(d=>d.keyword),['thought practice','gentle practice']);
  const faces = materializeCard(card,definitions);
  assert.equal(faces.length,2);
  assert.match(faces[0].executable_text,/Draw a card/i);
  assert.match(faces[1].executable_text,/gain 1 life/i);
  assert.equal(JSON.stringify(card),original);
});
test('same normalized keyword differing reminder, template or forms fails with both origins', () => {
  for (const conflicting of [
    {...first,reminder:'Draw two cards.'},
    {keyword:'Thought practice {Type}',reminder:'Return a {Type} card from your graveyard to your hand.'},
    {...first,forms:{participle:'thoughtful'}},
  ]) {
    const index=indexSourceDefinitions([cohort('different.json',[row('variant',{id:'source_SET'},[conflicting])])]);
    assert.throws(()=>definitionsForSource({id:'source_SET'},[first],index,'own.json:original'), error=>error.code==='same_source_definition_conflict' && /own.json:original/.test(error.message) && /different.json:variant/.test(error.message));
  }
});
test('same display name, unkeyed records and invalid unrelated candidates do not import', () => {
  const index=indexSourceDefinitions([
    cohort('other.json',[row('other',{id:'other_SET',name:'Same name'},[{keyword:'Flying',reminder:'Draw a card.'}])]),
    cohort('unkeyed.json',[row('unkeyed',{name:'Same name'},[second])]),
  ]);
  assert.deepEqual(definitionsForSource({id:'source_SET',name:'Same name'},[first],index).map(d=>d.keyword),['thought practice']);
  assert.deepEqual(definitionsForSource({name:'Same name'},[first],index).map(d=>d.keyword),['thought practice']);
});
test('duplicate validated definitions ignore provenance but never leak across a different primary key', () => {
  const index=indexSourceDefinitions([cohort('same.json',[row('same',{id:'source_SET'},[{...first,keyword:'THOUGHT PRACTICE',sourceCard:'other provenance'}])])]);
  assert.equal(definitionsForSource({id:'source_SET'},[first],index).length,1);
  assert.deepEqual(definitionsForSource({id:'source_SET',source_card_key:'different_SET'},[second],index).map(d=>d.keyword),['gentle practice']);
});
test('different validated layouts of the same normalized keyword are a conflict', () => {
  const typed={keyword:'Type practice',template:'a {Type} card',reminder:'Return a {Type} card from your graveyard to your hand.'};
  const index=indexSourceDefinitions([cohort('layout.json',[row('layout',{id:'source_SET'},[{...typed,template:'{Type}'}])])]);
  assert.throws(()=>definitionsForSource({id:'source_SET'},[typed],index),error=>error.code==='same_source_definition_conflict');
});

test('physical-card declarations retain their own exact layouts inside a shared family row',()=>{
 const firstCard={source_card_key:'first_SET',executable_definitions:[first]};
 const secondCard={source_card_key:'second_SET',executable_definitions:[second]};
 const family={id:'family',keyword:'Family practice',executable_definitions:[{keyword:'Unrelated family default',reminder:'Draw two cards.'}],representative_cards:[firstCard,secondCard]};
 const original=JSON.stringify(family);
 const index=indexSourceDefinitions([cohort('physical.json',[family])]);
 assert.deepEqual(executableDefinitions(family,firstCard),[first]);
 assert.deepEqual(definitionsForSource(firstCard,[],index).map(d=>d.keyword),['thought practice']);
 assert.deepEqual(definitionsForSource(secondCard,[],index).map(d=>d.keyword),['gentle practice']);
 assert.equal(JSON.stringify(family),original);
 // A duplicate physical source's conflicting authored declaration is still rejected.
 const other={id:'conflict',keyword:'Thought practice',executable_definitions:[{...first,reminder:'Draw three cards.'}],representative_cards:[{source_card_key:'first_SET'}]};
 const realConflict=indexSourceDefinitions([cohort('physical.json',[family]),cohort('conflict.json',[other])]);
 assert.throws(()=>definitionsForSource(firstCard,[],realConflict),e=>e.code==='same_source_definition_conflict');
});
