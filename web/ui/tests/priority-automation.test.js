import test from "node:test";
import assert from "node:assert/strict";
import {
  CUSTOM_PASS_ACTION_HOLD_REASON,
  LOCAL_STACK_MANUAL_HOLD_REASON,
  LOCAL_EMPTY_STACK_HOLD_REASON,
  OPPONENT_STACK_HOLD_REASON,
  VIEWED_CARDS_HOLD_REASON,
  buildMultiplayerSmartAutoPass,
  priorityHoldReason,
} from "../src/lib/priority-automation.js";

test("unfunded timing candidates do not keep auto-pass held after analysis finishes", () => {
  const options = {
    autoPassEnabled: true, holdRule: "if_actions",
    decision: { kind: "priority", player: 0, analysis_complete: true, actions: [
      { kind: "pass_priority" }, { kind: "cast_spell", payment_proven: false },
    ] },
    currentState: { perspective: 0, phase: "FirstMain", stack_size: 0 },
  };
  assert.equal(priorityHoldReason(options), null);
  assert.equal(priorityHoldReason({ ...options, decision: { ...options.decision, analysis_complete: false } }), "checking playable actions");
});

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

test("persistent hold stops multiplayer auto-pass after your own spell and on an empty opponent turn", () => {
  for (const stackSize of [0, 1]) {
    const result = buildMultiplayerSmartAutoPass({
      autoPassEnabled: true,
      holdRule: "always",
      decision: {
        kind: "priority",
        player: 1,
        actions: [{ index: 0, kind: "pass_priority", label: "Pass priority" }],
      },
      currentState: {
        perspective: 1,
        active_player: 2,
        stack_size: stackSize,
        stack_objects: stackSize ? [{ id: 100, controller: 1 }] : [],
      },
    });
    assert.equal(result.command, null);
    assert.equal(result.holdReason, "always hold");
  }
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

test("multiplayer smart auto-pass holds while viewed cards are pending", () => {
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
      stack_objects: [{ controller: 1, name: "Selvala, Explorer Returned" }],
      viewed_cards: {
        visibility: "public",
        card_ids: [11, 12],
      },
    },
  });

  assert.equal(result.command, null);
  assert.equal(result.holdReason, VIEWED_CARDS_HOLD_REASON);
});

test("multiplayer smart auto-pass ignores inspector-only reveal previews", () => {
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
      stack_objects: [{ controller: 1, name: "Selvala, Explorer Returned" }],
      viewed_cards: {
        visibility: "public",
        inspector_only: true,
        cards: [{ id: 11, name: "Swamp" }],
      },
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

test('off-turn never-hold passes even when background analysis stalls', () => {
  const decision = { kind: 'priority', player: 1, analysis_complete: false,
    actions: [{ index: 0, kind: 'pass_priority', label: 'Pass priority' }] };
  const result = buildMultiplayerSmartAutoPass({ autoPassEnabled: true, holdRule: 'never', decision,
    currentState: { perspective: 1, active_player: 0, phase: 'combat', stack_size: 0 } });
  assert.deepEqual(result.command, { type: 'priority_action', action_index: 0 });
  assert.equal(result.holdReason, null);
});

test('off-turn priority respects never-hold even with playable actions', () => {
  for (const phase of ['FirstMain', 'combat', 'SecondMain', 'ending']) {
    for (const kind of ['cast_spell', 'activate_ability', 'special_action']) {
      const decision = { kind: 'priority', player: 1, analysis_complete: true,
        actions: [{ index: 0, kind: 'pass_priority', label: 'Pass priority' }, { index: 1, kind }] };
      const result = buildMultiplayerSmartAutoPass({ autoPassEnabled: true, holdRule: 'never', decision,
        currentState: { perspective: 1, active_player: 0, phase, stack_size: 0 } });
      assert.deepEqual(result.command, { type: 'priority_action', action_index: 0 }, `${phase}: ${kind}`);
      assert.equal(result.holdReason, null);
    }
  }
});

test('off-turn combat priority holds for playable actions when requested', () => {
  for (const kind of ['cast_spell', 'activate_ability', 'special_action']) {
    const decision = { kind: 'priority', player: 1, analysis_complete: true,
      actions: [{ index: 0, kind: 'pass_priority', label: 'Pass priority' }, { index: 1, kind }] };
    const result = buildMultiplayerSmartAutoPass({ autoPassEnabled: true, holdRule: 'if_actions', decision,
      currentState: { perspective: 1, active_player: 0, phase: 'combat', stack_size: 0 } });
    assert.equal(result.command, null, kind);
    assert.equal(result.holdReason, 'playable actions available');
  }
});

test('off-turn priority still passes when only mana and undo actions remain', () => {
  const decision = { kind: 'priority', player: 1, analysis_complete: true,
    actions: [{ index: 0, kind: 'pass_priority', label: 'Pass priority' },
      { index: 1, kind: 'activate_mana_ability' }, { index: 2, kind: 'untap_land' }] };
  const result = buildMultiplayerSmartAutoPass({ autoPassEnabled: true, holdRule: 'never', decision,
    currentState: { perspective: 1, active_player: 0, phase: 'combat', stack_size: 0 } });
  assert.deepEqual(result.command, { type: 'priority_action', action_index: 0 });
});

test('an unrelated hold setting never blocks off-turn phase progression on analysis', () => {
  for (const [holdRule, phase] of [['never', 'combat'], ['stack', 'FirstMain'],
    ['main', 'combat'], ['combat', 'FirstMain'], ['ending', 'FirstMain']]) {
    const decision = { kind: 'priority', player: 1, analysis_complete: false,
      actions: [{ index: 0, kind: 'pass_priority', label: 'Pass priority', action_ref: { kind: 'pass_priority' } }] };
    const result = buildMultiplayerSmartAutoPass({ autoPassEnabled: true, holdRule, decision,
      currentState: { perspective: 1, active_player: 0, phase, stack_size: 0 } });
    assert.deepEqual(result.command, { type: 'priority_action', action_index: 0,
      action_ref: { kind: 'pass_priority' } }, `${holdRule} in ${phase}`);
  }
});

test('an explicit playable-action hold still waits for unfinished analysis', () => {
  const decision = { kind: 'priority', player: 1, analysis_complete: false,
    actions: [{ index: 0, kind: 'pass_priority', label: 'Pass priority' }] };
  const result = buildMultiplayerSmartAutoPass({ autoPassEnabled: true, holdRule: 'if_actions', decision,
    currentState: { perspective: 1, active_player: 0, phase: 'combat', stack_size: 0 } });
  assert.equal(result.command, null);
  assert.equal(result.holdReason, 'checking playable actions');
});
