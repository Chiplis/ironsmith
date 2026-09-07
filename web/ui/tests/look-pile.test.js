import test from "node:test";
import assert from "node:assert/strict";
import { persistentLookCards, temporaryLookView, mergeLookCards } from "../src/lib/look-pile.js";

test("ongoing permissions use authoritative cards and do not retain a revoked permission", () => {
  const player = {can_view_library_top: true, library_top: "Island", persistent_look_cards: [{id: 1, name: "Island"}]};
  assert.equal(persistentLookCards({players: [player]})[0].id, 1);
  assert.deepEqual(persistentLookCards({players: [{...player, persistent_look_cards: []}]}), []);
});
test("temporary hidden-zone visibility is not an ongoing permission", () => {
  const card = {id: 1, name: "Island", face_down: true};
  const state = {players: [{id: 1, can_view_hand: true, hand_cards: [card], exile_cards: [card], persistent_look_cards: []}], viewed_cards: {cards: [card]}};
  assert.deepEqual(persistentLookCards(state), []);
  assert.equal(temporaryLookView(state).cards[0], card);
  assert.equal(mergeLookCards([card])[0].face_down, false);
});
test("Look merges identical objects and excludes hidden names", () => {
  assert.equal(mergeLookCards([{id: 1, name: "Island"}], [{id: 1, name: "Island"}, {id: 2, name: "Hidden card"}]).length, 1);
});
