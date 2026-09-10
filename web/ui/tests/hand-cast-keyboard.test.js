import assert from "node:assert/strict";
import test from "node:test";

import {
  HAND_KEYBOARD_CAST_EVENT,
  handKeyboardCastNeedsPointer,
  handKeyboardCastPlan,
  keyboardPlacementDragArgs,
} from "../src/lib/hand-cast-keyboard.js";

const cast = (overrides = {}) => ({
  index: 0,
  kind: "cast_spell",
  label: "Cast Lightning Bolt",
  action_ref: { kind: "cast_spell", spell_id: 7 },
  drag_requires_targets: false,
  drag_requires_modes: false,
  ...overrides,
});

const bolt = { id: 7, name: "Lightning Bolt", card_types: ["instant"] };
const bears = { id: 8, name: "Grizzly Bears", card_types: ["creature"] };
const mountain = { id: 9, name: "Mountain", card_types: ["land"] };

test("a targeted spell hands the mouse to the targeting arrow", () => {
  const plan = handKeyboardCastPlan({
    actions: [cast({ drag_requires_targets: true })],
    card: bolt,
  });
  assert.equal(plan.kind, "target");
  assert.equal(plan.action.index, 0);
  assert.equal(handKeyboardCastNeedsPointer(plan), false, "the engine's own arrow follows the mouse");
});

test("a permanent keeps the pointer so its battlefield slot can be aimed", () => {
  for (const [card, actions] of [
    [bears, [cast()]],
    [mountain, [cast({ kind: "play_land", label: "Play Mountain" })]],
  ]) {
    const plan = handKeyboardCastPlan({ actions, card });
    assert.equal(plan.kind, "place", card.name);
    assert.equal(handKeyboardCastNeedsPointer(plan), true, card.name);
  }
});

test("a spell with nothing to aim casts straight away", () => {
  const plan = handKeyboardCastPlan({ actions: [cast()], card: bolt });
  assert.equal(plan.kind, "cast");
  assert.equal(handKeyboardCastNeedsPointer(plan), false);
  // Modal spells are decided by the engine's own prompt, never by aiming.
  assert.equal(
    handKeyboardCastPlan({
      actions: [cast({ drag_requires_targets: true, drag_requires_modes: true })],
      card: bolt,
    }).kind,
    "cast",
  );
});

test("several ways to play a card go to the picker first, whatever they are", () => {
  for (const card of [bolt, bears]) {
    const plan = handKeyboardCastPlan({
      actions: [cast({ drag_requires_targets: true }), cast({ index: 1, label: "Cast with kicker" })],
      card,
    });
    assert.equal(plan.kind, "choose", card.name);
    assert.equal(plan.action, null, "the chosen action decides what happens next");
    assert.equal(plan.actions.length, 2);
    assert.equal(handKeyboardCastNeedsPointer(plan), false);
  }
});

test("a card with no way to play it has no plan", () => {
  assert.equal(handKeyboardCastPlan({ actions: [], card: bolt }), null);
  assert.equal(handKeyboardCastPlan({ actions: [null, undefined], card: bolt }), null);
  assert.equal(handKeyboardCastPlan(), null);
  assert.equal(handKeyboardCastNeedsPointer(null), false);
});

test("the held card is anchored on itself, outside a hand of no extent", () => {
  const rect = { left: 200, top: 600, right: 320, bottom: 780, width: 120, height: 180 };
  const args = keyboardPlacementDragArgs({
    card: bears,
    actions: [cast()],
    glowKind: "creature",
    rect,
    viewport: { innerWidth: 1440, innerHeight: 900 },
  });
  const [objectId, cardName, actions, glowKind, x, y, sourceRect, card, containerRect, sourcePoint, options] = args;
  assert.equal(objectId, 8);
  assert.equal(cardName, "Grizzly Bears");
  assert.equal(actions.length, 1);
  assert.equal(glowKind, "creature");
  assert.deepEqual([x, y], [260, 690], "held at the card's own centre");
  assert.deepEqual(sourceRect, rect);
  assert.equal(card.id, 8);
  // A hand with no extent counts as already left, so the placement preview
  // tracks the mouse from the first frame instead of waiting for an exit.
  assert.deepEqual(containerRect, { left: 0, top: 0, right: 0, bottom: 0 });
  assert.deepEqual(sourcePoint, { x: 260, y: 600 }, "the arrow leaves from the card's top edge");
  assert.equal(options.keyboard, true);
  // The arrow opens aimed at dead space above the card, not at the resting
  // pointer, which would stage whatever slot it happened to be over.
  assert.deepEqual(options.aim, { x: 260, y: 468 });
});

test("a card with no measurable rect is held at the middle of the viewport", () => {
  const args = keyboardPlacementDragArgs({
    card: bears,
    actions: [cast()],
    glowKind: "creature",
    rect: null,
    viewport: { innerWidth: 1000, innerHeight: 800 },
  });
  assert.deepEqual([args[4], args[5]], [500, 400]);
  assert.equal(args[6], null);
  assert.equal(args[9], null, "and no printed edge for the arrow to leave from");
  assert.deepEqual(args[10].aim, { x: 500, y: 532 }, "the aim is taken from the viewport instead");
});

test("the hand publishes keyboard casts under a namespaced event", () => {
  assert.equal(HAND_KEYBOARD_CAST_EVENT, "ironsmith:hand-card-keyboard-cast");
});
