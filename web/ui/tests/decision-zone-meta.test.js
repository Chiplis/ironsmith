import test from 'node:test';
import assert from 'node:assert/strict';
import { buildObjectLocationById, decisionObjectLocation } from '../src/lib/decision-object-meta.js';

test('locations separate ownership from battlefield control and include grouped members', () => {
  const locations = buildObjectLocationById({players:[{id:1,battlefield:[{id:10,owner:0,member_ids:[10,11]}],graveyard_cards:[{id:12}]}]});
  assert.deepEqual(decisionObjectLocation(locations,11),{zone:'battlefield',owner:0,controller:1});
  assert.deepEqual(decisionObjectLocation(locations,12),{zone:'graveyard',owner:1,controller:1});
  assert.equal(decisionObjectLocation(locations,99),null);
});
test('viewed cards do not overwrite a newer visible location or expose inspector-only cards', () => {
  const locations = buildObjectLocationById({players:[{id:0,hand_cards:[{id:2}]}],viewed_cards:{subject:1,zone:'Library',cards:[{id:2},{id:3}]}});
  assert.equal(locations.get('2').zone,'hand');
  assert.deepEqual(locations.get('3'),{zone:'library',owner:1,controller:1});
  assert.equal(buildObjectLocationById({viewed_cards:{inspector_only:true,subject:1,zone:'library',cards:[{id:3}]}}).size,0);
});
test('unknown ownership stays unknown and explicit decision metadata wins', () => {
  const locations = buildObjectLocationById({players:[{id:1,battlefield:[{id:10}]}]});
  assert.equal(locations.get('10').owner,null);
  assert.deepEqual(decisionObjectLocation(locations,10,{object_owner:0,object_controller:2}),{zone:'battlefield',owner:0,controller:2});
  assert.deepEqual(decisionObjectLocation(locations,44,{hidden_ref:{owner:0,zone:'Hand'}}),{zone:'hand',owner:0,controller:null});
});

test('stack abilities do not relabel their source or a card sharing a synthetic display id', () => {
  const locations = buildObjectLocationById({players:[{id:0,battlefield:[{id:1,owner:0},{id:3,owner:0}]}],stack_objects:[{id:3,target_object_id:99,inspect_object_id:1,ability_kind:'Activated',owner:0,controller:0}]});
  assert.equal(locations.get('1').zone,'battlefield');
  assert.equal(locations.get('3').zone,'battlefield');
  assert.equal(locations.get('99').zone,'stack');
});
