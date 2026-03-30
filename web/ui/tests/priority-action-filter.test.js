import test from "node:test";
import assert from "node:assert/strict";
import {
  collectSelectedPriorityActionIndices,
  filterPriorityActionGroups,
} from "../src/lib/priority-action-filter.js";

test("selected battlefield object only matches actions for that exact object family", () => {
  const selectedObjectFamilyIds = new Set(["201"]);
  const actions = [
    { index: 0, object_id: 101, label: "Play Plains", kind: "play_land" },
    { index: 1, object_id: 102, label: "Play Plains", kind: "play_land" },
    { index: 2, object_id: 103, label: "Play Plains", kind: "play_land" },
    { index: 3, object_id: 201, label: "Tap Plains: Add {W}", kind: "activate_mana_ability" },
  ];

  const selectedActionIndices = collectSelectedPriorityActionIndices(actions, selectedObjectFamilyIds);

  assert.deepEqual([...selectedActionIndices], [3]);

  const groups = [
    {
      key: "play",
      linkedObjectIds: new Set(["101", "102", "103"]),
      actionIndices: new Set([0, 1, 2]),
    },
    {
      key: "tap",
      linkedObjectIds: new Set(["201"]),
      actionIndices: new Set([3]),
    },
  ];

  const visible = filterPriorityActionGroups(groups, selectedObjectFamilyIds, selectedActionIndices);
  assert.deepEqual(visible.map((group) => group.key), ["tap"]);
});
