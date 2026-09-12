import test from "node:test";
import assert from "node:assert/strict";
import {
  buildStackTargetPresentation,
  getVisibleStackObjects,
  stackInspectObjectId,
  stackSelectionKeys,
} from "../src/lib/stack-targets.js";

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
