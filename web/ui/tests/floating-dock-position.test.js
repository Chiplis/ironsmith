import test from "node:test";
import assert from "node:assert/strict";
import { findFloatingDockPosition } from "../src/lib/floating-dock-position.js";

test("keeps a floating dock in its preferred open area", () => {
  const position = findFloatingDockPosition({
    viewportWidth: 1200,
    viewportHeight: 800,
    dockWidth: 240,
    dockHeight: 80,
    preferredLeft: 946,
    preferredTop: 600,
  });

  assert.deepEqual(position, { left: 946, top: 600, overlaps: false });
});

test("moves the dock away from a right-side zone pile without covering cards", () => {
  const position = findFloatingDockPosition({
    viewportWidth: 1200,
    viewportHeight: 800,
    dockWidth: 240,
    dockHeight: 80,
    preferredLeft: 946,
    preferredTop: 600,
    obstacles: [{ left: 1110, top: 540, right: 1180, bottom: 690 }],
  });

  assert.ok(position);
  assert.equal(position.overlaps, false);
  assert.ok(position.left + 240 < 1110);
  assert.equal(position.top, 600);
});

test("prefers a safe placement over a closer placement that overlaps a hand card", () => {
  const handCard = { left: 500, top: 620, right: 660, bottom: 800 };
  const position = findFloatingDockPosition({
    viewportWidth: 1200,
    viewportHeight: 800,
    dockWidth: 240,
    dockHeight: 80,
    preferredLeft: 946,
    preferredTop: 700,
    obstacles: [handCard],
  });

  assert.ok(position);
  assert.equal(position.overlaps, false);
  const overlapsHand = position.left < handCard.right + 12
    && position.left + 240 > handCard.left - 12
    && position.top < handCard.bottom + 12
    && position.top + 80 > handCard.top - 12;
  assert.equal(overlapsHand, false);
});

test("keeps graveyard and exile clear when every placement overlaps another card", () => {
  const zone = { left: 1100, top: 470, right: 1190, bottom: 700, protected: true };
  const field = { left: 250, top: 380, right: 1090, bottom: 720 };
  const position = findFloatingDockPosition({
    viewportWidth: 1200,
    viewportHeight: 720,
    dockWidth: 430,
    dockHeight: 250,
    preferredLeft: 750,
    preferredTop: 450,
    obstacles: [zone, field],
  });

  assert.ok(position.left + 430 <= zone.left - 12 || position.top + 250 <= zone.top - 12);
});
