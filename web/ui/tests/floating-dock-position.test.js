import test from "node:test";
import assert from "node:assert/strict";
import { anchorDecisionDock, anchorFloatingDock, anchorManaPaymentDock, handSideReserve } from "../src/lib/floating-dock-position.js";

test("anchors the dock to the bottom-right corner above the hand", () => {
  const position = anchorFloatingDock({
    viewportWidth: 1200,
    viewportHeight: 800,
    dockWidth: 240,
    dockHeight: 80,
    bottomLimit: 680,
  });

  assert.deepEqual(position, { left: 942, top: 600 });
});

test('payment stays between the hand and piles and below opponent cards even when very tall', () => {
  const position = anchorManaPaymentDock({ viewportWidth: 1600, viewportHeight: 900, dockHeight: 1000,
    protectedZones: [{ left: 1510, top: 250, right: 1590, bottom: 700 }], opponentCardBottom: 220 });
  assert.ok(position.left >= 1600 - handSideReserve(1600) + 16);
  assert.ok(position.left + position.maxWidth <= 1510 - 12);
  assert.ok(position.top >= 220 + 12);
  assert.equal(position.maxHeight, 900 - 16 - 232);
  assert.deepEqual(anchorManaPaymentDock({ viewportWidth:1600, viewportHeight:900, dockHeight:30, protectedZones:[{left:1510}], opponentCardBottom:220 }), position, 'calculated contents cannot move or resize the payment panel');
});

test("stays anchored to the bottom even when cards sit in that corner", () => {
  const position = anchorFloatingDock({
    viewportWidth: 1200,
    viewportHeight: 800,
    dockWidth: 420,
    dockHeight: 300,
    bottomLimit: 680,
  });

  assert.equal(position.top, 380);
  assert.equal(position.left, 1200 - 420 - 18);
});

test("slides left of Graveyard/Exile instead of covering them", () => {
  const piles = { left: 1110, top: 360, right: 1180, bottom: 580 };
  const position = anchorFloatingDock({
    viewportWidth: 1200,
    viewportHeight: 800,
    dockWidth: 420,
    dockHeight: 300,
    bottomLimit: 680,
    protectedZones: [piles],
  });

  assert.equal(position.top, 380);
  assert.equal(position.left + 420, piles.left - 12);
});

test("keeps the right-edge anchor when the piles are above the dock", () => {
  const piles = { left: 1110, top: 200, right: 1180, bottom: 400 };
  const position = anchorFloatingDock({
    viewportWidth: 1200,
    viewportHeight: 800,
    dockWidth: 240,
    dockHeight: 80,
    bottomLimit: 680,
    protectedZones: [piles],
  });

  assert.deepEqual(position, { left: 942, top: 600 });
});

test("moves above the hand when zone piles push the dock into its lane", () => {
  const handBounds = { left: 330, right: 870, top: 590, bottom: 800 };
  const position = anchorFloatingDock({
    viewportWidth: 1200, viewportHeight: 800, dockWidth: 280, dockHeight: 150,
    bottomLimit: 784, handBounds,
    protectedZones: [{ left: 1100, right: 1190, top: 500, bottom: 790 }],
  });
  assert.ok(position.top + 150 <= handBounds.top - 12);
  assert.ok(position.left + 280 <= 1100 - 12);
});

test("steps beside a pile column above the hand instead of climbing it", () => {
  // 4-player desktop: local piles stack in the right column just above the
  // hand's corner, and a full-width dock sits within clearance of the hand.
  const handBounds = { left: 358, top: 804, right: 1236, bottom: 1006 };
  const protectedZones = [
    { left: 1535, top: 383, right: 1591, bottom: 481 },
    { left: 1535, top: 487, right: 1591, bottom: 585 },
    { left: 1535, top: 591, right: 1591, bottom: 696 },
  ];
  const position = anchorFloatingDock({
    viewportWidth: 1600, viewportHeight: 900, dockWidth: 340, dockHeight: 92,
    bottomLimit: 884, handBounds, protectedZones,
  });
  assert.deepEqual(position, { left: 1535 - 12 - 340, top: 804 - 12 - 92 });
});


test("decision width uses the gap after the hand and reserves zones at every height", () => {
  for (const dockHeight of [100, 900]) {
    const position = anchorDecisionDock({viewportWidth:1440,viewportHeight:900,dockWidth:500,dockHeight,
      handRight:1000,protectedZones:[{left:1350,top:200,right:1430,bottom:450}]});
    assert.equal(position.maxWidth, 322);
    assert.equal(position.left, 1016);
    assert.ok(position.left + position.maxWidth <= 1338);
    assert.equal(position.maxHeight, 684);
    assert.ok(position.top >= 200);
    assert.equal(position.top + Math.min(dockHeight, position.maxHeight), 884);
  }
});

test("a wide hand lifts decisions above the cards without covering the zone column", () => {
  const position = anchorDecisionDock({ viewportWidth: 1280, viewportHeight: 800,
    dockWidth: 500, dockHeight: 900, handRight: 1100, handTop: 650,
    protectedZones: [{left: 1190, top: 190, right: 1270, bottom: 600}] });
  assert.equal(position.left + position.maxWidth, 1178);
  assert.equal(position.top, 190);
  assert.equal(position.top + position.maxHeight, 634);
});
