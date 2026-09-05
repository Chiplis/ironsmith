import assert from "node:assert/strict";
import test from "node:test";

import {
  dropTargetCandidateFromElements,
  handCardSourcePoint,
  legalTargetForDropCandidate,
  legalTargetForDropCandidates,
  rectBoundaryPointToward,
  shouldBeginTargetCastIntent,
  targetDropCompletesDecision,
} from "../src/lib/hand-drag-intent.js";

test("drag arrows begin at the edge of the hand container", () => {
  const rect = { left: 100, top: 500, right: 900, bottom: 700, width: 800, height: 200 };
  assert.deepEqual(
    rectBoundaryPointToward(rect, 500, 620, 300, 200),
    { x: 500, y: 500 },
  );
  assert.deepEqual(
    rectBoundaryPointToward(rect, 500, 620, 1100, 600),
    { x: 500, y: 500 },
  );
});

test("permanent placement arrows begin at the card's collapsed fan slot", () => {
  assert.deepEqual(
    handCardSourcePoint({ left: 720, top: 640, right: 840, bottom: 808, width: 120, height: 168 }),
    { x: 780, y: 640 },
  );
});

test("non-modal targeted cast variants share a target drag on hand exit", () => {
  const targetedCast = {
    kind: "cast_spell",
    drag_requires_targets: true,
    drag_requires_modes: false,
  };
  assert.equal(shouldBeginTargetCastIntent([targetedCast]), true);
  assert.equal(shouldBeginTargetCastIntent([{ ...targetedCast, drag_requires_modes: true }]), false);
  assert.equal(shouldBeginTargetCastIntent([targetedCast, { ...targetedCast }]), true);
  assert.equal(shouldBeginTargetCastIntent([
    targetedCast,
    { ...targetedCast, drag_requires_targets: false },
  ]), false);
  assert.equal(shouldBeginTargetCastIntent([{ kind: "play_land" }]), false);
});

test("a board drop resolves grouped-card ids against the live legal targets", () => {
  const decision = {
    kind: "targets",
    requirements: [{
      min_targets: 1,
      max_targets: 1,
      legal_targets: [{ kind: "object", object: 22, name: "Merged half" }],
    }],
  };
  const target = legalTargetForDropCandidate(decision, {
    kind: "object",
    objectIds: [11, 22],
  });
  assert.deepEqual(target, { kind: "object", object: 22, name: "Merged half" });
  assert.equal(targetDropCompletesDecision(decision, target), true);
});

test("a card under the release point wins over its enclosing player target", () => {
  const decision = {
    kind: "targets",
    requirements: [{
      min_targets: 1,
      max_targets: 1,
      legal_targets: [
        { kind: "player", player: 0, name: "Alice" },
        { kind: "object", object: 42, name: "Llanowar Elves" },
      ],
    }],
  };
  const target = legalTargetForDropCandidates(decision, [
    { kind: "player", playerIds: [0] },
    { kind: "object", objectIds: [42] },
  ]);
  assert.deepEqual(target, { kind: "object", object: 42, name: "Llanowar Elves" });
});

test("stacked pointer hits prefer a battlefield card over its player surface", () => {
  const playerRoot = {
    getAttribute(name) {
      return name === "data-player-target" ? "0" : null;
    },
  };
  const cardRoot = {
    getAttribute(name) {
      if (name === "data-object-id") return "42";
      if (name === "data-member-object-ids") return "42";
      return null;
    },
  };
  const playerSurface = {
    closest(selector) {
      return selector.includes("data-player-target") ? playerRoot : null;
    },
  };
  const cardSurface = {
    closest(selector) {
      if (selector === ".game-card[data-object-id]") return cardRoot;
      return selector.includes("data-player-target") ? playerRoot : null;
    },
  };

  assert.deepEqual(
    dropTargetCandidateFromElements([playerSurface, cardSurface]),
    { kind: "object", objectIds: [42] },
  );
});

test("a drag target does not auto-submit while optional extra targets remain", () => {
  const decision = {
    kind: "targets",
    requirements: [{
      min_targets: 1,
      max_targets: 3,
      legal_targets: [
        { kind: "player", player: 1 },
        { kind: "player", player: 2 },
      ],
    }],
  };
  assert.equal(
    targetDropCompletesDecision(decision, { kind: "player", player: 1 }),
    false,
  );
});

test("targeted casts require an explicit self player box but allow opponent dead zones", () => {
  function surface({ explicit = false, self = false, id = 0 }) {
    const root = {
      getAttribute: (name) => name === (explicit ? "data-player-target" : "data-player-drop-target") ? String(id) : null,
      hasAttribute: (name) => name === "data-my-zone" && self,
    };
    return { closest: (selector) => selector.includes("data-player-target") ? root : null };
  }
  assert.equal(dropTargetCandidateFromElements([surface({ self: true })]), null);
  assert.deepEqual(dropTargetCandidateFromElements([surface({ self: true, explicit: true })]), {
    kind: "player", playerIds: [0],
  });
  assert.deepEqual(dropTargetCandidateFromElements([surface({ id: 1 })]), {
    kind: "player", playerIds: [1],
  });
});
