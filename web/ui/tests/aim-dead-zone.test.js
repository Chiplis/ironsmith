import assert from "node:assert/strict";
import test from "node:test";

import {
  aimPointIsOccupied,
  deadZoneAimPoint,
  insideBattlefieldDropGrid,
} from "../src/lib/aim-dead-zone.js";

const viewport = { innerWidth: 1600, innerHeight: 1000 };
const clear = () => false;

test("an arrow from the hand is lifted towards the board", () => {
  const point = deadZoneAimPoint({ from: { x: 800, y: 900 }, viewport, occupied: clear });
  assert.deepEqual(point, { x: 800, y: 768 }, "132px up, straight ahead");
});

test("an arrow from the far side of the board is lifted the other way", () => {
  const point = deadZoneAimPoint({ from: { x: 800, y: 120 }, viewport, occupied: clear });
  assert.deepEqual(point, { x: 800, y: 252 });
});

test("busy space is stepped over sideways first, then further out", () => {
  const asked = [];
  const busy = (x, y) => {
    asked.push([x, y]);
    // Everything within 200px of the source's own lane is taken.
    return Math.abs(x - 800) < 200 && y > 700;
  };
  const point = deadZoneAimPoint({ from: { x: 800, y: 900 }, viewport, occupied: busy });
  assert.deepEqual(point, { x: 596, y: 768 }, "the same rise, the first lane that is clear");
  assert.deepEqual(asked.slice(0, 3), [[800, 768], [692, 768], [908, 768]], "nearest lanes first");
});

test("a board with no dead space still gets a visible arrow", () => {
  const point = deadZoneAimPoint({ from: { x: 800, y: 900 }, viewport, occupied: () => true });
  assert.deepEqual(point, { x: 800, y: 768 }, "the first candidate, never the source itself");
});

test("the arrow stays inside the viewport", () => {
  assert.deepEqual(
    deadZoneAimPoint({ from: { x: 10, y: 980 }, viewport, occupied: clear }),
    { x: 24, y: 848 },
  );
  assert.deepEqual(
    deadZoneAimPoint({ from: { x: 1590, y: 60 }, viewport, occupied: clear }),
    { x: 1576, y: 192 },
  );
  // A viewport with no room for the margin collapses onto it rather than
  // reporting a point off screen.
  assert.deepEqual(
    deadZoneAimPoint({ from: { x: 20, y: 30 }, viewport: { innerWidth: 40, innerHeight: 40 }, occupied: clear }),
    { x: 24, y: 24 },
  );
});

test("an unmeasurable source has no aim point", () => {
  assert.equal(deadZoneAimPoint({ from: null, viewport, occupied: clear }), null);
  assert.equal(deadZoneAimPoint({ from: { x: 5 }, viewport, occupied: clear }), null);
  assert.equal(deadZoneAimPoint(), null);
});

test("the battlefield drop grid counts as occupied, since it stages a slot", () => {
  const grid = {
    getBoundingClientRect: () => ({ left: 100, top: 200, right: 900, bottom: 600 }),
  };
  const root = { querySelector: (selector) => (selector.includes("battlefield-drop-grid") ? grid : null) };
  assert.equal(insideBattlefieldDropGrid(500, 400, root), true);
  assert.equal(insideBattlefieldDropGrid(500, 700, root), false);
  assert.equal(insideBattlefieldDropGrid(50, 400, root), false);
  assert.equal(insideBattlefieldDropGrid(500, 400, { querySelector: () => null }), false);
  // Occupancy also covers whatever a click would pick at that point.
  const empty = { querySelector: () => null, elementsFromPoint: () => [] };
  assert.equal(aimPointIsOccupied(500, 400, empty), false);
  assert.equal(aimPointIsOccupied(500, 400, root), true);
});
