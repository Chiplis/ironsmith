import test from "node:test";
import assert from "node:assert/strict";
import {
  describeDecisionCommandMismatch,
  findPriorityActionForCommand,
  isDecisionCommandCompatible,
  priorityCommandForAction,
  resolveSyncedCommand,
  selectObjectCandidateRevealPolicy,
  selectObjectSyncMetadataForCommand,
} from "../src/lib/sync-commands.js";

test("priority decisions only accept priority commands", () => {
  const decision = {
    kind: "priority",
    player: 0,
    actions: [
      {
        index: 3,
        kind: "cast_spell",
        action_ref: {
          kind: "cast_spell",
          spell_id: 42,
          from_zone: "hand",
          casting_method: { kind: "normal" },
        },
      },
    ],
  };

  assert.equal(
    isDecisionCommandCompatible(decision, {
      type: "priority_action",
      action_index: 3,
    }),
    true,
  );
  assert.equal(
    isDecisionCommandCompatible(decision, {
      type: "select_options",
      option_indices: [0],
    }),
    false,
  );
});

test("priority action refs are matched against the live decision", () => {
  const action = {
    index: 1,
    kind: "cast_spell",
    action_ref: {
      from_zone: "hand",
      kind: "cast_spell",
      spell_id: 10,
      casting_method: { kind: "normal" },
    },
  };
  const decision = { kind: "priority", player: 0, actions: [action] };

  assert.equal(findPriorityActionForCommand(decision, priorityCommandForAction(action)), action);
  assert.equal(
    isDecisionCommandCompatible(decision, {
      type: "priority_action",
      action_ref: {
        kind: "cast_spell",
        spell_id: 11,
        from_zone: "hand",
        casting_method: { kind: "normal" },
      },
    }),
    false,
  );
});

test("priority action sync preserves source object remap metadata", () => {
  const command = resolveSyncedCommand({
    type: "priority_action",
    action_ref: {
      from_zone: "hand",
      kind: "cast_spell",
      spell_id: 160,
      casting_method: { kind: "normal" },
    },
    object_id: 160,
    object_stable_id: 13,
  });

  assert.deepEqual(command, {
    type: "priority_action",
    action_ref: {
      from_zone: "hand",
      kind: "cast_spell",
      spell_id: 160,
      casting_method: { kind: "normal" },
    },
    object_id: 160,
    object_stable_id: 13,
  });
});

test("stale priority refs are rejected even when the index is reused", () => {
  const decision = {
    kind: "priority",
    player: 0,
    actions: [
      {
        index: 2,
        kind: "cast_spell",
        action_ref: {
          kind: "cast_spell",
          spell_id: 200,
          from_zone: "hand",
          casting_method: { kind: "normal" },
        },
      },
    ],
  };

  assert.equal(
    isDecisionCommandCompatible(decision, {
      type: "priority_action",
      action_index: 2,
      action_ref: {
        kind: "cast_spell",
        spell_id: 100,
        from_zone: "hand",
        casting_method: { kind: "normal" },
      },
    }),
    false,
  );
});

test("cancel commands can be applied during resync without a visible decision", () => {
  assert.equal(
    isDecisionCommandCompatible(null, { type: "cancel_decision" }),
    true,
  );
});

test("surrender forfeits must target the pending decision player", () => {
  assert.equal(
    isDecisionCommandCompatible(
      { kind: "priority", player: 1, actions: [] },
      { type: "forfeit_player", player: 1, reason: "surrender" },
    ),
    true,
  );
  assert.equal(
    isDecisionCommandCompatible(
      { kind: "priority", player: 0, actions: [] },
      { type: "forfeit_player", player: 1, reason: "surrender" },
    ),
    false,
  );
});

test("disconnect timeout policy forfeits can be submitted without a pending decision", () => {
  assert.equal(
    isDecisionCommandCompatible(null, {
      type: "forfeit_player",
      player: 0,
      reason: "disconnect_timeout_policy",
    }),
    true,
  );
  assert.equal(
    isDecisionCommandCompatible(null, {
      type: "forfeit_player",
      player: 0,
      reason: "peer_claimed_disconnect_timeout",
    }),
    true,
  );
});

test("mismatch descriptions include command and pending decision kind", () => {
  assert.equal(
    describeDecisionCommandMismatch(
      { kind: "priority" },
      { type: "select_options" },
    ),
    "Synced command select_options does not match pending priority decision",
  );
});

test("select object sync metadata comes from decision candidates, not prompt text", () => {
  const state = {
    decision: {
      kind: "select_objects",
      description: "Put this somewhere totally innocuous",
      selection_identity: "object_id",
      reveal_policy: "none",
      candidates: [
        {
          id: 10,
          name: "Forest",
          selection_identity: "stable_id",
          stable_id: 1000,
          reveal_policy: "none",
        },
        {
          id: 11,
          name: "Hidden card",
          selection_identity: "hidden_reference",
          reveal_policy: "public",
          hidden_ref: {
            owner: 0,
            zone: "hand",
            slot: 3,
            commitment: "slot-3",
            publicSlot: 8,
            publicCommitment: "position-8",
          },
        },
      ],
    },
  };

  const metadata = selectObjectSyncMetadataForCommand(
    { type: "select_objects", object_ids: [10, 11] },
    state,
  );

  assert.deepEqual(metadata.stableIds, [1000, null]);
  assert.deepEqual(metadata.hiddenRefs, [
    null,
    {
      owner: 0,
      zone: "hand",
      slot: 3,
      commitment: "slot-3",
    },
  ]);
  assert.equal(
    selectObjectCandidateRevealPolicy(state.decision, state.decision.candidates[1]),
    "public",
  );
});

test("resolveSyncedCommand preserves aligned select object identity metadata", () => {
  assert.deepEqual(
    resolveSyncedCommand({
      type: "select_objects",
      object_ids: ["10", "11"],
      object_stable_ids: ["1000", null],
      object_hidden_refs: [
        null,
        {
          owner: "0",
          zone: "hand",
          slot: "3",
          commitment: "slot-3",
        },
      ],
    }),
    {
      type: "select_objects",
      object_ids: [10, 11],
      object_stable_ids: [1000, null],
      object_hidden_refs: [
        null,
        {
          owner: 0,
          zone: "hand",
          slot: 3,
          commitment: "slot-3",
        },
      ],
    },
  );
});

test("resolveSyncedCommand strips private identity from public library ziffle hidden refs", () => {
  assert.deepEqual(
    resolveSyncedCommand({
      type: "select_objects",
      object_ids: [11],
      object_hidden_refs: [
        {
          owner: "1",
          zone: "library",
          slot: "29",
          commitment: "private-slot-29",
          public_slot: "58",
          public_commitment: "ziffle:deckhash:58",
        },
      ],
    }),
    {
      type: "select_objects",
      object_ids: [11],
      object_hidden_refs: [
        {
          owner: 1,
          zone: "library",
          public_slot: 58,
          public_commitment: "ziffle:deckhash:58",
        },
      ],
    },
  );
});

test("resolveSyncedCommand keeps non-library ziffle refs private", () => {
  assert.deepEqual(
    resolveSyncedCommand({
      type: "select_objects",
      object_ids: [11, 12],
      object_hidden_refs: [
        {
          owner: "1",
          zone: "hand",
          slot: "1",
          commitment: "ziffle:deckhash:1",
          public_slot: "58",
          public_commitment: "ziffle:old-library:58",
        },
        {
          owner: "1",
          zone: "hand",
          public_slot: "1",
          public_commitment: "ziffle:deckhash:1",
        },
      ],
    }),
    {
      type: "select_objects",
      object_ids: [11, 12],
      object_hidden_refs: [
        {
          owner: 1,
          zone: "hand",
          slot: 1,
          commitment: "ziffle:deckhash:1",
        },
        {
          owner: 1,
          zone: "hand",
          slot: 1,
          commitment: "ziffle:deckhash:1",
        },
      ],
    },
  );
});
