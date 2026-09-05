import test from "node:test";
import assert from "node:assert/strict";
import { canHoverInspectorObject, objectExistsInState, resolveInspectorObjectId } from "../src/lib/inspector-selection.js";

test("casting retires the hand hover without automatically opening the stack preview", () => {
  const beforeCast = { players: [{ hand_cards: [{ id: 101 }] }] };
  const afterCast = {
    players: [{ hand_cards: [] }],
    stack_objects: [{ id: 202, inspect_object_id: 203, name: "Lightning Bolt" }],
  };
  assert.equal(objectExistsInState(beforeCast, 101), true);
  assert.equal(objectExistsInState(afterCast, 101), false);
  assert.equal(canHoverInspectorObject(afterCast, 101), false);
  for (const id of [202, 203]) {
    assert.equal(canHoverInspectorObject(afterCast, id), false);
    // Explicit clicks may still inspect either the tile or its linked card.
    assert.equal(objectExistsInState(afterCast, id), true);
    assert.equal(resolveInspectorObjectId({ pinnedObjectId: id }), String(id));
  }
});

test("battlefield previews still hover, but resolving stack entries require a click", () => {
  const state = {
    players: [{ battlefield: [{ id: 10, member_ids: [11] }] }],
    resolving_stack_object: { id: 20, inspect_object_id: 21 },
  };
  assert.equal(canHoverInspectorObject(state, 10), true);
  assert.equal(canHoverInspectorObject(state, 11), true);
  assert.equal(canHoverInspectorObject(state, 20), false);
  assert.equal(canHoverInspectorObject(state, 21), false);
  assert.equal(canHoverInspectorObject(state, null), false);
});

test("selected inspector object wins over hover", () => {
  assert.equal(
    resolveInspectorObjectId({
      selectedObjectId: 101,
      hoveredObjectId: 202,
    }),
    "101"
  );
});

test("pinned inspector object wins over hover when no selected object is active", () => {
  assert.equal(
    resolveInspectorObjectId({
      pinnedObjectId: 303,
      hoveredObjectId: 404,
    }),
    "303"
  );
});

test("hover drives inspector only when no selection lock is active", () => {
  assert.equal(
    resolveInspectorObjectId({
      hoveredObjectId: 505,
    }),
    "505"
  );
});
