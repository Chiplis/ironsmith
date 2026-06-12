import test from "node:test";
import assert from "node:assert/strict";
import {
  collectSelectedPriorityActionIndices,
  filterPriorityActionGroups,
  withoutManaAbilityActionGroups,
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

test("withoutManaAbilityActionGroups drops only mana-ability groups", () => {
  const groups = [
    { key: "tap-w", firstAction: { kind: "activate_mana_ability" } },
    { key: "play", firstAction: { kind: "play_land" } },
    { key: "yawgmoth", firstAction: { kind: "activate_ability" } },
    { key: "cast", firstAction: { kind: "cast_spell" } },
  ];

  const visible = withoutManaAbilityActionGroups(groups);
  assert.deepEqual(visible.map((group) => group.key), ["play", "yawgmoth", "cast"]);
});

test("withoutManaAbilityActionGroups tolerates malformed groups", () => {
  assert.deepEqual(withoutManaAbilityActionGroups([]), []);
  const odd = [{ key: "no-first-action" }, null];
  assert.deepEqual(withoutManaAbilityActionGroups(odd), odd);
});
