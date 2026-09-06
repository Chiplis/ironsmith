import test from "node:test";
import assert from "node:assert/strict";
import { zoneHasLegalTargets, isFaceUpZoneCard, zonePileCards, zonePileDestination } from "../src/lib/zone-piles.js";
import { resolveGameAnimations } from "../src/lib/game-animations.js";

test("piles keep engine recency order and skip face-down covers", () => {
  const cards = [{ id: 3, name: "Known face-down card", face_down: true }, { id: 2, name: "Island" }, { id: 1, name: "Plains" }];
  assert.deepEqual(zonePileCards({ exile_cards: cards }, "exile").map((card) => card.id), [3, 2, 1]);
  assert.equal(cards.find(isFaceUpZoneCard).id, 2);
  assert.equal(isFaceUpZoneCard({ name: "Hidden card" }), false);
  assert.deepEqual(zonePileCards({}, "graveyard"), []);
});

test("zone destinations prefer the owner over the former controller", () => {
  assert.deepEqual(zonePileDestination({ toZone: "graveyard", playerKey: "0", card: { owner: 1, controller: 0 } }), { playerId: "1", zone: "graveyard" });
  assert.equal(zonePileDestination({ toZone: "hand" }), null);
});

test("simultaneous deaths route separate streams to each owner's graveyard", () => {
  const previews = [1, 2].map((owner) => ({
    token: `death-${owner}`, fromZone: "battlefield", toZone: "graveyard", playerKey: "0",
    fromObjectId: owner, objectId: owner + 10, trackingKeys: [`stable:${owner}`],
    card: { id: owner + 10, stable_id: owner, name: "Bear", owner, controller: 0 },
  }));
  const result = resolveGameAnimations({ previews, state: {}, previousCardRects: new Map([1, 2].map((id) => [
    `stable:${id}`, { left: id * 100, top: 100, width: 60, height: 84 },
  ])) });
  const flights = result.visualEffects.filter((effect) => effect.travelsToInspector);
  assert.deepEqual(flights.map((flight) => flight.targetToken), ["zone:1:graveyard", "zone:2:graveyard"]);
  assert.notEqual(flights[0].groupId, flights[1].groupId);
});

 test("shortcut previews identify eligible zones by owner and object, without choosing the cover", () => {
  const state = {players:[{id:0,graveyard_cards:[{id:1},{id:2}],exile_cards:[{id:3}]},{id:1,graveyard_cards:[{id:4}]}]};
  const decision = {kind:"targets",requirements:[{legal_targets:[{kind:"object",object:2},{kind:"object",object:3}]}]};
  assert.equal(zoneHasLegalTargets(state,decision,{kind:"zone",playerId:"0",zone:"graveyard"}),true);
  assert.equal(zoneHasLegalTargets(state,decision,{kind:"zone",playerId:"0",zone:"exile"}),true);
  assert.equal(zoneHasLegalTargets(state,decision,{kind:"zone",playerId:"1",zone:"graveyard"}),false);
  assert.equal(zoneHasLegalTargets(state,{kind:"priority"},{kind:"zone",playerId:"0",zone:"graveyard"}),false);
});
