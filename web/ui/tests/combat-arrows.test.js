import assert from "node:assert/strict";
import test from "node:test";
import {
  ATTACKER_COLOR,
  BLOCKER_COLOR,
  buildCombatStateArrows,
  combatStateArrowSignature,
} from "../src/lib/combat-arrows.js";

test("no combat yields no arrows", () => {
  assert.deepEqual(buildCombatStateArrows(null), []);
  assert.deepEqual(buildCombatStateArrows(undefined), []);
  assert.deepEqual(buildCombatStateArrows({ attackers: [], blockers: [] }), []);
});

test("attackers point at players, planeswalkers and battles; blockers point at attackers", () => {
  const combat = {
    attackers: [
      { creature: 11, target: { kind: "player", player: 1 } },
      { creature: 12, target: { kind: "planeswalker", object: 40 } },
      { creature: 13, target: { kind: "battle", object: 41 } },
    ],
    blockers: [
      { blocker: 21, blocking: 11 },
      { blocker: 22, blocking: 11 },
    ],
  };
  const controllers = new Map([["40", 1], ["41", 0]]);
  const arrows = buildCombatStateArrows(combat, controllers);
  assert.deepEqual(arrows, [
    { fromId: 11, toId: null, toPlayerId: 1, toFallbackPlayerId: null, color: ATTACKER_COLOR, key: "atk-state-11" },
    { fromId: 12, toId: 40, toPlayerId: null, toFallbackPlayerId: 1, color: ATTACKER_COLOR, key: "atk-state-12" },
    { fromId: 13, toId: 41, toPlayerId: null, toFallbackPlayerId: 0, color: ATTACKER_COLOR, key: "atk-state-13" },
    { fromId: 21, toId: 11, toPlayerId: null, color: BLOCKER_COLOR, key: "blk-state-21-11" },
    { fromId: 22, toId: 11, toPlayerId: null, color: BLOCKER_COLOR, key: "blk-state-22-11" },
  ]);
  // Keys never collide with the declaring seat's in-progress arrows (atk-<id>, blk-<a>-<b>)
  // but still start with the prefixes that select the dashed combat stroke.
  for (const arrow of arrows) {
    assert.ok(/^(atk|blk)-state-/.test(arrow.key));
  }
});

test("malformed entries are skipped instead of producing dangling arrows", () => {
  const arrows = buildCombatStateArrows({
    attackers: [{ creature: 5, target: { kind: "planeswalker" } }, { target: { kind: "player", player: 0 } }],
    blockers: [{ blocker: 7 }, { blocking: 5 }],
  });
  assert.deepEqual(arrows, []);
});

test("signature changes when a blocker is added or an attacker retargets", () => {
  const base = buildCombatStateArrows({
    attackers: [{ creature: 1, target: { kind: "player", player: 1 } }],
    blockers: [],
  });
  const blocked = buildCombatStateArrows({
    attackers: [{ creature: 1, target: { kind: "player", player: 1 } }],
    blockers: [{ blocker: 2, blocking: 1 }],
  });
  const retargeted = buildCombatStateArrows({
    attackers: [{ creature: 1, target: { kind: "planeswalker", object: 9 } }],
    blockers: [],
  });
  assert.notEqual(combatStateArrowSignature(base), combatStateArrowSignature(blocked));
  assert.notEqual(combatStateArrowSignature(base), combatStateArrowSignature(retargeted));
  assert.equal(combatStateArrowSignature(base), combatStateArrowSignature(buildCombatStateArrows({
    attackers: [{ creature: "1", target: { kind: "player", player: "1" } }],
    blockers: [],
  })));
});
