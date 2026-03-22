import test from "node:test";
import assert from "node:assert/strict";
import {
  LOCAL_STACK_MANUAL_HOLD_REASON,
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
