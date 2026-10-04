import test from "node:test";
import assert from "node:assert/strict";
import { anchorFloatingDock } from "../src/lib/floating-dock-position.js";

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
