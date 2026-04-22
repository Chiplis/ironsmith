import test from "node:test";
import assert from "node:assert/strict";
import {
  CUSTOM_PASS_ACTION_HOLD_REASON,
  LOCAL_STACK_MANUAL_HOLD_REASON,
  LOCAL_EMPTY_STACK_HOLD_REASON,
  OPPONENT_STACK_HOLD_REASON,
  buildMultiplayerSmartAutoPass,
  priorityHoldReason,
} from "../src/lib/priority-automation.js";

test("local priority with a stack item always holds for manual resolve", () => {
  const holdReason = priorityHoldReason({
    autoPassEnabled: true,
    holdRule: "never",
    decision: {
      kind: "priority",
      player: 1,
      actions: [{ kind: "pass_priority" }],
    },
    currentState: {
      perspective: 1,
      stack_size: 1,
      phase: "FirstMain",
    },
    perspectiveMode: "local",
    manualResolveOnLocalStack: true,
  });

  assert.equal(holdReason, LOCAL_STACK_MANUAL_HOLD_REASON);
});

test("local off-turn priority still auto-passes when the stack is empty", () => {
  const holdReason = priorityHoldReason({
    autoPassEnabled: true,
    holdRule: "never",
    decision: {
      kind: "priority",
      player: 1,
      actions: [{ kind: "pass_priority" }],
    },
    currentState: {
      perspective: 1,
      stack_size: 0,
      phase: "FirstMain",
    },
    perspectiveMode: "local",
    manualResolveOnLocalStack: true,
  });

  assert.equal(holdReason, null);
});

test("opponent priority respects always-hold stops", () => {
  const holdReason = priorityHoldReason({
    autoPassEnabled: true,
    holdRule: "always",
    decision: {
      kind: "priority",
      player: 2,
      actions: [{ kind: "pass_priority" }],
    },
    currentState: {
      perspective: 1,
      stack_size: 2,
      phase: "FirstMain",
    },
    perspectiveMode: "opponent",
  });

  assert.equal(holdReason, "always hold");
});

test("multiplayer smart auto-pass skips empty off-turn priority", () => {
  const result = buildMultiplayerSmartAutoPass({
    autoPassEnabled: true,
    holdRule: "never",
    decision: {
      kind: "priority",
      player: 1,
      actions: [{ index: 7, kind: "pass_priority", label: "Pass priority" }],
    },
    currentState: {
      perspective: 1,
      active_player: 2,
      stack_size: 0,
      stack_objects: [],
    },
  });

  assert.deepEqual(result.command, { type: "priority_action", action_index: 7 });
  assert.equal(result.holdReason, null);
});

test("multiplayer smart auto-pass holds own empty-stack priority", () => {
  const result = buildMultiplayerSmartAutoPass({
    autoPassEnabled: true,
    holdRule: "never",
    decision: {
      kind: "priority",
      player: 1,
      actions: [{ index: 3, kind: "pass_priority", label: "Pass priority" }],
    },
    currentState: {
      perspective: 1,
      active_player: 1,
      stack_size: 0,
      stack_objects: [],
    },
  });

  assert.equal(result.command, null);
  assert.equal(result.holdReason, LOCAL_EMPTY_STACK_HOLD_REASON);
});

test("multiplayer smart auto-pass skips priority after local stack actions", () => {
  const result = buildMultiplayerSmartAutoPass({
    autoPassEnabled: true,
    holdRule: "never",
    decision: {
      kind: "priority",
      player: 1,
      actions: [{ index: 4, kind: "pass_priority", label: "Pass priority" }],
    },
    currentState: {
      perspective: 1,
      active_player: 1,
      stack_size: 1,
      stack_objects: [{ controller: 1, name: "Lightning Bolt" }],
    },
  });

  assert.deepEqual(result.command, { type: "priority_action", action_index: 4 });
  assert.equal(result.holdReason, null);
});

test("multiplayer smart auto-pass holds for opponent stack actions", () => {
  const result = buildMultiplayerSmartAutoPass({
    autoPassEnabled: true,
    holdRule: "never",
    decision: {
      kind: "priority",
      player: 1,
      actions: [{ index: 2, kind: "pass_priority", label: "Pass priority" }],
    },
    currentState: {
      perspective: 1,
      active_player: 2,
      stack_size: 1,
      stack_objects: [{ controller: 2, name: "Counterspell" }],
    },
  });

  assert.equal(result.command, null);
  assert.equal(result.holdReason, OPPONENT_STACK_HOLD_REASON);
});

test("multiplayer smart auto-pass does not confirm custom pass actions", () => {
  const result = buildMultiplayerSmartAutoPass({
    autoPassEnabled: true,
    holdRule: "never",
    decision: {
      kind: "priority",
      player: 1,
      actions: [{ index: 0, kind: "pass_priority", label: "Keep hand" }],
    },
    currentState: {
      perspective: 1,
      active_player: 1,
      stack_size: 0,
    },
  });

  assert.equal(result.command, null);
  assert.equal(result.holdReason, CUSTOM_PASS_ACTION_HOLD_REASON);
});
