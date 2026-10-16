import test from 'node:test';
import assert from 'node:assert/strict';
import { visibleLibraryCards } from '../src/lib/library-pile.js';

test('ongoing top visibility excludes hand and exile views and expires with permission', () => {
  const top = {id: 1, name: 'Island'};
  const hand = {id: 2, name: 'Island'};
  const exile = {id: 3, name: 'Forest', face_down: true};
  const player = {id: 0, can_view_library_top: true, library_top: 'Island',
    persistent_look_cards: [top, hand, exile], hand_cards: [hand], exile_cards: [exile]};
  assert.deepEqual(visibleLibraryCards({players: [player]}, player), [{...top, face_down: false, is_face_down: false}]);
  assert.deepEqual(visibleLibraryCards({players: [player]}, {...player, can_view_library_top: false}), []);
});

test('multiple revealed library cards belong only to their library and exclude cards already moved', () => {
  const player = {id: 0, hand_cards: [{id: 3, name: 'Plains'}]};
  const state = {players: [player], viewed_cards: {zone: 'library', subject: 0,
    cards: [{id: 1, name: 'Island'}, {id: 2, name: 'Forest'}, {id: 3, name: 'Plains'}, {id: 4, name: 'Hidden card'}]}};
  assert.deepEqual(visibleLibraryCards(state, player).map(card => card.id), [1, 2]);
  assert.deepEqual(visibleLibraryCards(state, {id: 1}), []);
  assert.deepEqual(visibleLibraryCards({...state, viewed_cards: null}, player), []);
});
