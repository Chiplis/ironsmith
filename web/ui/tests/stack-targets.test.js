import test from "node:test";
import assert from "node:assert/strict";
import {
  buildStackTargetPresentation,
  getVisibleStackObjects,
  stackEntryIsLegalTarget,
  stackEntryTargetObjectIds,
  stackInspectObjectId,
  stackSelectionKeys,
  stackEntryRenderKeys,
} from "../src/lib/stack-targets.js";

test("stack render keys distinguish shared ids and survive top pushes and pops", () => {
  const entries = [{ id: 3 }, { id: 3 }, { id: 1 }];
  const keys = stackEntryRenderKeys(entries);
  assert.equal(new Set(keys).size, entries.length);
  assert.deepEqual(stackEntryRenderKeys(entries.slice(1)), keys.slice(1));
  assert.deepEqual(stackEntryRenderKeys([{ id: 3 }, ...entries]).slice(1), keys);
});

test("stack inspector id prefers the linked card object", () => {
  const stackEntry = {
    id: 2001,
    inspect_object_id: 42,
  };

  assert.equal(stackInspectObjectId(stackEntry), 42);
  assert.deepEqual(stackSelectionKeys(stackEntry), ["2001", "42"]);
});

test("stack target presentation can focus a stack object by linked card id", () => {
  const state = {
    perspective: 1,
    players: [
      { id: 1, battlefield: [{ id: 42, name: "Lightning Bolt" }] },
      { id: 2, battlefield: [{ id: 99, name: "Target" }] },
    ],
    stack_objects: [
      {
        id: 2001,
        inspect_object_id: 42,
        controller: 1,
        targets: [{ kind: "object", object: 99 }],
      },
    ],
  };

  const presentation = buildStackTargetPresentation(state, ["battlefield"], 42);

  assert.equal(presentation.activeStackObject.id, 2001);
  assert.equal(presentation.arrows[0].fromId, 2001);
  assert.equal(presentation.arrows[0].toId, 99);
});


test("a completed resolving entry cannot reappear at priority", () => {
  const old = { id: 81, name: "Resolved spell" };
  const next = { id: 82, name: "Next spell" };
  const prompt = { stack_objects: [next], resolving_stack_object: old,
    decision: { kind: "select_options", player: 0 } };
  assert.deepEqual(getVisibleStackObjects(prompt), [old, next], "keep the active resolution during its prompt");
  const priority = { ...prompt, decision: { kind: "priority", player: 0 } };
  assert.deepEqual(getVisibleStackObjects(priority), [next]);
  assert.deepEqual(getVisibleStackObjects({ ...priority, stack_objects: [] }), []);
  assert.deepEqual(getVisibleStackObjects({ ...priority, stack_objects: [old] }), [old], "live entries remain authoritative");
});

test("a spell on the stack is targeted by the object it inspects to; an ability by nothing", () => {
  const spell = { id: 272, inspect_object_id: 136, name: "Lightning Bolt" };
  const ability = { id: 45, inspect_object_id: 134, name: "Mogg Fanatic", ability_kind: "Activated" };
  assert.deepEqual(stackEntryTargetObjectIds(spell), [136]);
  assert.deepEqual(stackEntryTargetObjectIds(ability), []);
  assert.deepEqual(stackEntryTargetObjectIds(null), []);

  const counterspell = {
    kind: "targets",
    player: 0,
    requirements: [{ legal_targets: [{ kind: "object", object: 136, name: "Lightning Bolt" }] }],
  };
  assert.equal(stackEntryIsLegalTarget(counterspell, spell), true);
  // Bolt aimed at the Fanatic permanent (134) must not light up its ability.
  const bolt = {
    kind: "targets",
    player: 0,
    requirements: [{ legal_targets: [{ kind: "object", object: 134 }, { kind: "object", object: 272 }] }],
  };
  assert.equal(stackEntryIsLegalTarget(bolt, ability), false);
  // ...nor a spell whose presentation id collides with a legal permanent.
  assert.equal(stackEntryIsLegalTarget(bolt, spell), false);
  assert.equal(stackEntryIsLegalTarget({ kind: "priority" }, spell), false);
});

test("a spell targeting another spell draws its arrow to that spell's stack tile", () => {
  const state = {
    perspective: 0,
    players: [{ id: 0, battlefield: [] }, { id: 1, battlefield: [{ id: 99, name: "Grizzly Bears" }] }],
    stack_objects: [
      { id: 274, inspect_object_id: 137, name: "Counterspell", controller: 0, targets: [{ kind: "object", object: 136 }] },
      { id: 272, inspect_object_id: 136, name: "Lightning Bolt", controller: 0, targets: [{ kind: "object", object: 99 }] },
    ],
  };
  const presentation = buildStackTargetPresentation(state, ["battlefield"], 274);
  assert.equal(presentation.arrows.length, 1);
  assert.equal(presentation.arrows[0].toId, 272, "the arrow lands on the Bolt tile, drawn under its own id");
  assert.deepEqual(presentation.temporaryZoneViews, []);
});
