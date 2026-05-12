import test from "node:test";
import assert from "node:assert/strict";
import {
  describeDecisionCommandMismatch,
  findPriorityActionForCommand,
  isDecisionCommandCompatible,
  priorityCommandForAction,
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

test("disconnect forfeits can be submitted without a pending decision", () => {
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
