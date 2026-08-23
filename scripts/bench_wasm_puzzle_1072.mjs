#!/usr/bin/env node

import { createHash } from "node:crypto";
import { mkdir, readFile, stat, writeFile } from "node:fs/promises";
import { cpus } from "node:os";
import { performance } from "node:perf_hooks";
import {
  ensureCardSourcesForNames,
  initWasmGame,
} from "./wasm-test-harness.mjs";

const CREATURE_GROUPS = [
  ["Goblin Guide", 250],
  ["Llanowar Elves", 250],
  ["Elite Vanguard", 250],
  ["Walking Corpse", 250],
];

const EFFECT_SOURCES = [
  "Mycosynth Lattice",
  "Akroma's Memorial",
  "Always Watching",
  "Fervor",
  "Glorious Anthem",
  "Honor of the Pure",
  "Bad Moon",
  "Favorable Winds",
  "Gaea's Anthem",
  "Intangible Virtue",
  "Spidersilk Armor",
  "Dictate of Heliod",
];

const SCENARIO_HAND_SPELLS = {
  cast_mana: "Llanowar Elves",
  cast_targeted: "Giant Growth",
};

function parseArgs(argv) {
  const options = {
    runs: 10,
    warmups: 1,
    phasePasses: 12,
    scenario: "phase",
    thresholdMs: 1000,
    enforceThreshold: false,
    pkg: "root",
    output: null,
    json: false,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--runs") options.runs = positiveInteger(argv[++index], "--runs");
    else if (arg === "--warmups") options.warmups = nonnegativeInteger(argv[++index], "--warmups");
    else if (arg === "--phase-passes") options.phasePasses = nonnegativeInteger(argv[++index], "--phase-passes");
    else if (arg === "--scenario") options.scenario = argv[++index];
    else if (arg === "--threshold-ms") options.thresholdMs = positiveNumber(argv[++index], "--threshold-ms");
    else if (arg === "--enforce-threshold") options.enforceThreshold = true;
    else if (arg === "--pkg") options.pkg = argv[++index];
    else if (arg === "--output") options.output = argv[++index];
    else if (arg === "--json") options.json = true;
    else throw new Error(`Unknown argument: ${arg}`);
  }

  if (!["root", "demo", "bench"].includes(options.pkg)) {
    throw new Error("--pkg must be root, demo, or bench");
  }
  if (
    ![
      "phase",
      "no_attack",
      "round_trip",
      "all_attack",
      "cast_mana",
      "cast_targeted",
    ].includes(options.scenario)
  ) {
    throw new Error(
      "--scenario must be phase, no_attack, round_trip, all_attack, cast_mana, or cast_targeted",
    );
  }
  return options;
}

function positiveInteger(value, flag) {
  const parsed = Number(value);
  if (!Number.isInteger(parsed) || parsed < 1) throw new Error(`${flag} must be a positive integer`);
  return parsed;
}

function nonnegativeInteger(value, flag) {
  const parsed = Number(value);
  if (!Number.isInteger(parsed) || parsed < 0) throw new Error(`${flag} must be a nonnegative integer`);
  return parsed;
}

function positiveNumber(value, flag) {
  const parsed = Number(value);
  if (!Number.isFinite(parsed) || parsed <= 0) throw new Error(`${flag} must be positive`);
  return parsed;
}

function round(value) {
  return Number.isFinite(value) ? Number(value.toFixed(3)) : value;
}

function decisionActionKind(action) {
  return action?.action_ref?.kind || action?.kind || "unknown";
}

function firstActionByKind(state, kind) {
  const action = (state?.decision?.actions || []).find(
    (candidate) => decisionActionKind(candidate) === kind,
  );
  if (!action) {
    const available = (state?.decision?.actions || [])
      .map((candidate) => `${candidate.index}:${decisionActionKind(candidate)}:${candidate.label}`)
      .join(" | ");
    throw new Error(`Missing ${kind} action. Available: ${available}`);
  }
  return action;
}

function safeCall(fn) {
  try {
    return fn();
  } catch {
    return null;
  }
}

function metric(
  perf,
  camelName,
  snakeName = camelName.replace(/[A-Z]/g, (char) => `_${char.toLowerCase()}`),
) {
  return perf?.[camelName] ?? perf?.[snakeName];
}

function numberMetric(perf, camelName, fallback = 0) {
  return round(Number(metric(perf, camelName) ?? fallback));
}

function summarizeComputeLegalActionsDetail(perf) {
  if (!perf) return null;
  return {
    totalMs: numberMetric(perf, "totalMs"),
    actionCount: metric(perf, "actionCount") ?? null,
    prewarmMs: numberMetric(perf, "prewarmMs"),
    handCastsMs: numberMetric(perf, "handCastsMs"),
    graveyardCastsMs: numberMetric(perf, "graveyardCastsMs"),
    exileCastsMs: numberMetric(perf, "exileCastsMs"),
    battlefieldAbilitiesMs: numberMetric(perf, "battlefieldAbilitiesMs"),
    canCastSpellWithViewMs: numberMetric(perf, "canCastSpellWithViewMs"),
    computePotentialManaWithViewMs: numberMetric(perf, "computePotentialManaWithViewMs"),
    handCastsCostAdjustmentMs: numberMetric(perf, "handCastsCostAdjustmentMs"),
    handCastsAffordabilityMs: numberMetric(perf, "handCastsAffordabilityMs"),
    handCastsTargetLegalityMs: numberMetric(perf, "handCastsTargetLegalityMs"),
    battlefieldAbilityPrecheckMs: numberMetric(perf, "battlefieldAbilityPrecheckMs"),
    battlefieldAbilityAffordabilityMs: numberMetric(perf, "battlefieldAbilityAffordabilityMs"),
  };
}

function summarizePriorityAdvance(perf) {
  if (!perf) return null;
  return {
    totalMs: numberMetric(perf, "totalMs"),
    resultKind: metric(perf, "resultKind") ?? null,
    priorityPlayer: metric(perf, "priorityPlayer") ?? null,
    actionCount: metric(perf, "actionCount") ?? null,
    stateBasedActionsMs: numberMetric(perf, "stateBasedActionsMs"),
    putTriggersMs: numberMetric(perf, "putTriggersMs"),
    computeLegalActionsMs: numberMetric(perf, "computeLegalActionsMs"),
    computeCommanderActionsMs: numberMetric(perf, "computeCommanderActionsMs"),
    computeLegalActionsDetail: summarizeComputeLegalActionsDetail(
      metric(perf, "computeLegalActionsDetail"),
    ),
  };
}

function summarizePriorityAction(perf) {
  if (!perf) return null;
  return {
    totalMs: numberMetric(perf, "totalMs"),
    actionKind: metric(perf, "actionKind") ?? null,
    priorityResult: metric(perf, "priorityResult") ?? null,
    passPriorityMs: numberMetric(perf, "passPriorityMs"),
    responseApplyMs: numberMetric(perf, "responseApplyMs"),
    advancePriorityMs: numberMetric(perf, "advancePriorityMs"),
    resolveStackEntryMs: numberMetric(perf, "resolveStackEntryMs"),
    resetPriorityMs: numberMetric(perf, "resetPriorityMs"),
    nestedPriorityAdvance: summarizePriorityAdvance(metric(perf, "nestedPriorityAdvance")),
  };
}

function numericDelta(after, before) {
  if (!after || !before) return null;
  return Object.fromEntries(
    Object.entries(after)
      .filter(([, value]) => typeof value === "number")
      .map(([key, value]) => {
        const prior = Number(before[key] || 0);
        // Replay checkpoint restoration can rewind runtime-cache counters.
        // Treat the post-restore value as the new local baseline in that case.
        return [key, value >= prior ? value - prior : value];
      }),
  );
}

function stateSummary(state) {
  const requirements = state?.decision?.requirements || [];
  return {
    turnNumber: state?.turn_number ?? state?.turnNumber ?? null,
    activePlayer: state?.active_player ?? state?.activePlayer ?? null,
    phase: state?.phase ?? null,
    step: state?.step ?? null,
    priorityPlayer: state?.priority_player ?? null,
    stackSize: state?.stack_size ?? state?.stackSize ?? null,
    battlefieldSize: state?.battlefield_size ?? state?.battlefieldSize ?? null,
    decision: state?.decision?.kind ?? null,
    decisionActionCount: state?.decision?.actions?.length ?? null,
    decisionOptionCount: state?.decision?.options?.length ?? null,
    legalTargetCount: requirements.reduce(
      (total, requirement) => total + (requirement.legal_targets?.length || 0),
      0,
    ),
    gameOver: state?.game_over ?? state?.gameOver ?? null,
  };
}

function priorityCommand(action) {
  return {
    type: "priority_action",
    action_index: action.index,
    ...(action.action_ref ? { action_ref: action.action_ref } : {}),
  };
}

function priorityActionObjectId(action) {
  const raw = action?.object_id
    ?? action?.action_ref?.spell_id
    ?? action?.action_ref?.object_id
    ?? action?.action_ref?.permanent_id
    ?? action?.action_ref?.source;
  return raw == null ? null : Number(raw);
}

function spellCastAction(state, spellId, spellName) {
  const actions = state?.decision?.actions || [];
  const byId = actions.find(
    (action) =>
      decisionActionKind(action) === "cast_spell"
      && priorityActionObjectId(action) === Number(spellId),
  );
  const action = byId || actions.find(
    (candidate) =>
      decisionActionKind(candidate) === "cast_spell"
      && String(candidate.label || "").includes(spellName),
  );
  if (!action) {
    const available = actions
      .filter((candidate) => decisionActionKind(candidate) === "cast_spell")
      .slice(0, 20)
      .map((candidate) => ({
        index: candidate.index,
        objectId: priorityActionObjectId(candidate),
        label: candidate.label,
      }));
    throw new Error(
      `Missing cast action for ${spellName} (${spellId}): ${JSON.stringify(available)}`,
    );
  }
  return action;
}

function normalizeAttackTarget(target) {
  if (target?.kind === "player") {
    return { kind: "player", player: Number(target.player) };
  }
  if (target?.kind === "planeswalker") {
    return { kind: "planeswalker", object: Number(target.object ?? target.id) };
  }
  throw new Error(`Unsupported attack target: ${JSON.stringify(target)}`);
}

function assertCondition(condition, message, details = null) {
  if (!condition) {
    const suffix = details == null ? "" : `\n${JSON.stringify(details, null, 2)}`;
    throw new Error(`${message}${suffix}`);
  }
}

function timedCall(game, operation, fn) {
  const beforeCounters = safeCall(() => game.lastWorkCounters());
  const started = performance.now();
  const state = fn();
  const elapsedMs = performance.now() - started;
  const afterCounters = safeCall(() => game.lastWorkCounters());
  const snapshot = safeCall(() => game.lastSnapshotPerf());
  const dispatch = safeCall(() => game.lastDispatchPerf());
  const reportedDispatchMs = Number(
    dispatch?.totalDispatchMs ?? dispatch?.total_dispatch_ms ?? Number.NaN,
  );
  const dispatchMs = Number.isFinite(reportedDispatchMs) && reportedDispatchMs > 0
    ? reportedDispatchMs
    : elapsedMs;
  const snapshotMs = Number(
    snapshot?.totalSnapshotMs ?? snapshot?.total_snapshot_ms ?? 0,
  );
  return {
    operation,
    elapsedMs: round(elapsedMs),
    dispatchMs: round(dispatchMs),
    snapshotMs: round(snapshotMs),
    dispatchExcludingSnapshotMs: round(Math.max(0, dispatchMs - snapshotMs)),
    dispatchBreakdown: dispatch
      ? {
          routeKind: metric(dispatch, "routeKind") ?? null,
          outcomeKind: metric(dispatch, "outcomeKind") ?? null,
          commandToResponseMs: numberMetric(dispatch, "commandToResponseMs"),
          checkpointCaptureMs: numberMetric(dispatch, "checkpointCaptureMs"),
          executeWithReplayMs: numberMetric(dispatch, "executeWithReplayMs"),
          applyProgressMs: numberMetric(dispatch, "applyProgressMs"),
          advanceUntilDecisionMs: numberMetric(metric(dispatch, "advanceUntilDecision"), "totalMs"),
          replayExecutionMs: numberMetric(metric(dispatch, "replayExecution"), "totalMs"),
          priorityAction: summarizePriorityAction(
            metric(metric(dispatch, "replayExecution"), "priorityAction"),
          ),
          priorityAdvance: summarizePriorityAdvance(
            metric(metric(dispatch, "replayExecution"), "priorityAdvance"),
          ),
        }
      : null,
    counters: numericDelta(afterCounters, beforeCounters),
    state: stateSummary(state),
    result: state,
  };
}

function addPuzzleCards(game) {
  const cards = [];
  for (const [name, copies] of CREATURE_GROUPS) {
    for (let copy = 0; copy < copies; copy += 1) {
      cards.push({ playerIndex: 0, cardName: name, zoneName: "battlefield", skipTriggers: true });
    }
  }
  for (const name of EFFECT_SOURCES) {
    for (let copy = 0; copy < 6; copy += 1) {
      cards.push({ playerIndex: 0, cardName: name, zoneName: "battlefield", skipTriggers: true });
    }
  }
  // The URL has empty libraries. One inert card per player lets the benchmark
  // continue through the first draw step instead of ending on draw-from-empty.
  cards.push({ playerIndex: 0, cardName: "Walking Corpse", zoneName: "library", skipTriggers: true });
  cards.push({ playerIndex: 1, cardName: "Walking Corpse", zoneName: "library", skipTriggers: true });
  ensureCardSourcesForNames(game, [
    ...CREATURE_GROUPS.map(([name]) => name),
    ...EFFECT_SOURCES,
    ...Object.values(SCENARIO_HAND_SPELLS),
  ]);
  game.addCardsToZones(cards);
}

function labelPriorityTransition(record, before, after) {
  if (before.phase !== after.phase || before.step !== after.step) {
    record.operation = `phase_transition:${before.phase}/${before.step}->${after.phase}/${after.step}`;
  } else if (before.priorityPlayer !== after.priorityPlayer) {
    record.operation = "priority_handoff_same_step";
  } else {
    record.operation = "pass_priority_same_step";
  }
}

function passPriority(game, state, operation = "pass_priority") {
  assertCondition(state?.decision?.kind === "priority", `Expected priority decision for ${operation}`, stateSummary(state));
  const action = firstActionByKind(state, "pass_priority");
  return timedCall(game, operation, () => game.dispatch(priorityCommand(action)));
}

function driveToAttackers(game, initialState, records) {
  let state = initialState;
  for (let pass = 0; pass < 32; pass += 1) {
    if (state?.decision?.kind === "attackers") return state;
    const before = stateSummary(state);
    const record = passPriority(game, state, "pass_priority_to_attackers");
    state = record.result;
    const after = stateSummary(state);
    labelPriorityTransition(record, before, after);
    records.push(record);
  }
  throw new Error(`Did not reach declare attackers decision: ${JSON.stringify(stateSummary(state))}`);
}

function isAliceFirstMainPriority(state) {
  const summary = stateSummary(state);
  return Number(summary.activePlayer) === 0
    && summary.phase === "first main phase"
    && summary.step == null
    && Number(summary.priorityPlayer) === 0
    && summary.decision === "priority";
}

function driveToAliceFirstMain(game, initialState, records) {
  let state = initialState;
  for (let pass = 0; pass < 16; pass += 1) {
    if (isAliceFirstMainPriority(state)) return state;
    const before = stateSummary(state);
    const record = passPriority(game, state, "pass_priority_to_first_main");
    state = record.result;
    labelPriorityTransition(record, before, stateSummary(state));
    records.push(record);
  }
  throw new Error(`Did not reach Alice's first main phase: ${JSON.stringify(stateSummary(state))}`);
}

function isManaPaymentDecision(state) {
  return state?.decision?.kind === "mana_payment" && state?.mana_payment != null;
}

function llanowarPaymentSources(state) {
  return (state?.mana_payment?.available_sources || []).filter(
    (source) => String(source.source_name || "").includes("Llanowar Elves"),
  );
}

function confirmManaPayment(game, state, records, spellSlug) {
  assertCondition(
    isManaPaymentDecision(state),
    `Expected ${spellSlug} whole-cost mana payment decision`,
    {
      state: stateSummary(state),
      decision: state?.decision ?? null,
    },
  );
  const paymentSources = llanowarPaymentSources(state);
  assertCondition(
    paymentSources.length === 250,
    `Expected 250 available Llanowar Elves sources for ${spellSlug}, got ${paymentSources.length}`,
    (state.mana_payment.available_sources || []).slice(0, 5),
  );
  const payment = timedCall(
    game,
    `mana_payment:${spellSlug}_confirm_whole_cost_plan`,
    () => game.dispatch({
      type: "mana_payment",
      response: {
        action: "confirm",
        plan_id: String(state.decision.plan_id),
        request_hash: String(state.decision.request_hash),
      },
    }),
  );
  records.push(payment);
  return payment.result;
}

function resolveCastSpell(game, initialState, records, spellSlug) {
  let state = initialState;
  assertCondition(
    state?.decision?.kind === "priority" && Number(stateSummary(state).stackSize) === 1,
    `Expected ${spellSlug} on the stack after payment`,
    stateSummary(state),
  );

  const handoff = passPriority(game, state, `priority_handoff:${spellSlug}_on_stack`);
  handoff.operation = `priority_handoff:${spellSlug}_on_stack`;
  records.push(handoff);
  state = handoff.result;
  assertCondition(
    state?.decision?.kind === "priority" && Number(stateSummary(state).stackSize) === 1,
    `Expected opponent priority with ${spellSlug} on the stack`,
    stateSummary(state),
  );

  const resolution = passPriority(game, state, `resolve:${spellSlug}`);
  resolution.operation = `resolve:${spellSlug}`;
  records.push(resolution);
  state = resolution.result;
  assertCondition(
    Number(stateSummary(state).stackSize) === 0,
    `Expected ${spellSlug} to resolve`,
    stateSummary(state),
  );
  return state;
}

function runCastManaScenario(game, initialState, records, spellId) {
  let state = driveToAliceFirstMain(game, initialState, records);
  const castAction = spellCastAction(state, spellId, SCENARIO_HAND_SPELLS.cast_mana);
  const cast = timedCall(game, "cast_spell:llanowar_elves_build_mana_options", () =>
    game.dispatch(priorityCommand(castAction)),
  );
  records.push(cast);
  state = cast.result;

  state = confirmManaPayment(game, state, records, "llanowar_elves");
  return resolveCastSpell(game, state, records, "llanowar_elves");
}

function firstLegalCreatureTarget(state) {
  for (const requirement of state?.decision?.requirements || []) {
    const target = (requirement.legal_targets || []).find(
      (candidate) => candidate.kind === "object" && candidate.object != null,
    );
    if (target) return target;
  }
  return null;
}

function runCastTargetedScenario(game, initialState, records, spellId) {
  let state = driveToAliceFirstMain(game, initialState, records);
  const castAction = spellCastAction(state, spellId, SCENARIO_HAND_SPELLS.cast_targeted);
  const cast = timedCall(game, "cast_spell:giant_growth_enumerate_1000_targets", () =>
    game.dispatch(priorityCommand(castAction)),
  );
  records.push(cast);
  state = cast.result;

  assertCondition(
    state?.decision?.kind === "targets",
    "Expected Giant Growth target decision",
    stateSummary(state),
  );
  const legalTargetCount = stateSummary(state).legalTargetCount;
  assertCondition(
    legalTargetCount === 1000,
    `Expected 1,000 Giant Growth targets, got ${legalTargetCount}`,
  );
  const target = firstLegalCreatureTarget(state);
  assertCondition(target, "Giant Growth target decision has no legal creature target");

  const selection = timedCall(game, "select_target:giant_growth_build_mana_options", () =>
    game.dispatch({ type: "select_targets", targets: [target] }),
  );
  records.push(selection);
  state = selection.result;

  state = confirmManaPayment(game, state, records, "giant_growth");
  return resolveCastSpell(game, state, records, "giant_growth");
}

function isLaterTurnUpkeep(state, startingTurn, activePlayer) {
  const summary = stateSummary(state);
  return Number(summary.turnNumber) > Number(startingTurn)
    && Number(summary.activePlayer) === activePlayer
    && summary.phase === "beginning phase"
    && summary.step === "upkeep step";
}

function runNoAttackScenario(game, initialState, records) {
  let state = driveToAttackers(game, initialState, records);
  const startingTurn = stateSummary(state).turnNumber;
  const attackerOptions = state.decision.attacker_options || [];
  assertCondition(attackerOptions.length === 1000, `Expected 1,000 legal attackers, got ${attackerOptions.length}`);

  const declaration = timedCall(game, "declare_attackers:none", () =>
    game.dispatch({ type: "declare_attackers", declarations: [] }),
  );
  records.push(declaration);
  state = declaration.result;

  for (let pass = 0; pass < 32 && !isLaterTurnUpkeep(state, startingTurn, 1); pass += 1) {
    const before = stateSummary(state);
    const record = passPriority(game, state, "pass_priority_no_attack");
    state = record.result;
    const after = stateSummary(state);
    if (isLaterTurnUpkeep(state, startingTurn, 1)) {
      record.operation = "transition:end_step->next_turn_upkeep_including_cleanup";
    } else {
      labelPriorityTransition(record, before, after);
    }
    records.push(record);
  }
  assertCondition(
    isLaterTurnUpkeep(state, startingTurn, 1),
    "No-attack scenario did not reach Bob's upkeep",
    stateSummary(state),
  );
  return state;
}

function runRoundTripScenario(game, initialState, records) {
  let state = runNoAttackScenario(game, initialState, records);
  const bobTurn = stateSummary(state).turnNumber;
  state = driveToAttackers(game, state, records);
  const attackerOptions = state.decision.attacker_options || [];
  assertCondition(
    attackerOptions.length === 0,
    `Expected Bob to have no legal attackers, got ${attackerOptions.length}`,
  );

  const declaration = timedCall(game, "declare_attackers:bob_none", () =>
    game.dispatch({ type: "declare_attackers", declarations: [] }),
  );
  records.push(declaration);
  state = declaration.result;

  for (let pass = 0; pass < 32 && !isLaterTurnUpkeep(state, bobTurn, 0); pass += 1) {
    const before = stateSummary(state);
    const record = passPriority(game, state, "pass_priority_round_trip");
    state = record.result;
    const after = stateSummary(state);
    if (isLaterTurnUpkeep(state, bobTurn, 0)) {
      record.operation = "transition:bob_end_step->alice_upkeep_including_cleanup_and_untap";
    } else {
      labelPriorityTransition(record, before, after);
    }
    records.push(record);
  }
  assertCondition(
    isLaterTurnUpkeep(state, bobTurn, 0),
    "Round-trip scenario did not reach Alice's next upkeep",
    stateSummary(state),
  );
  return state;
}

function runAllAttackScenario(game, initialState, records) {
  let state = driveToAttackers(game, initialState, records);
  const attackerOptions = state.decision.attacker_options || [];
  assertCondition(attackerOptions.length === 1000, `Expected 1,000 legal attackers, got ${attackerOptions.length}`);
  const declarations = attackerOptions.map((option) => {
    const target = option.valid_targets?.[0];
    assertCondition(target, "Attacker has no legal target", option);
    const normalized = normalizeAttackTarget(target);
    assertCondition(
      normalized.kind === "player" && normalized.player === 1,
      "Exact puzzle attacker did not target Bob",
      { option, normalized },
    );
    return { creature: Number(option.creature), target: normalized };
  });

  const declaration = timedCall(game, "declare_attackers:all_1000", () =>
    game.dispatch({ type: "declare_attackers", declarations }),
  );
  records.push(declaration);
  state = declaration.result;
  assertCondition(
    Number(stateSummary(state).stackSize) === 250,
    "Expected 250 Goblin Guide triggers after declaring all attackers",
    stateSummary(state),
  );

  let priorityDispatches = 0;
  for (; priorityDispatches < 600 && state?.decision?.kind !== "blockers"; priorityDispatches += 1) {
    const before = stateSummary(state);
    const record = passPriority(game, state, "pass_priority_attack_trigger_stack");
    state = record.result;
    const after = stateSummary(state);
    if (state?.decision?.kind === "blockers") {
      record.operation = "transition:declare_attackers->declare_blockers";
    } else if (Number(after.stackSize) < Number(before.stackSize)) {
      record.operation = "resolve:goblin_guide_trigger";
    } else if (before.priorityPlayer !== after.priorityPlayer) {
      record.operation = "priority_handoff:declare_attackers_stack";
    } else {
      labelPriorityTransition(record, before, after);
    }
    records.push(record);
  }
  assertCondition(state?.decision?.kind === "blockers", "All-attack scenario did not reach blockers", stateSummary(state));
  assertCondition(priorityDispatches === 502, `Expected 502 priority dispatches before blockers, got ${priorityDispatches}`);

  const blockerOptions = state.decision.blocker_options || [];
  assertCondition(blockerOptions.length === 1000, `Expected 1,000 blocker options, got ${blockerOptions.length}`);
  assertCondition(
    blockerOptions.every((option) => (option.valid_blockers || []).length === 0),
    "Bob unexpectedly has a legal blocker",
    blockerOptions.slice(0, 3),
  );
  const blockers = timedCall(game, "declare_blockers:none", () =>
    game.dispatch({ type: "declare_blockers", declarations: [] }),
  );
  records.push(blockers);
  state = blockers.result;

  for (let pass = 0; pass < 8 && !stateSummary(state).gameOver; pass += 1) {
    const before = stateSummary(state);
    const record = passPriority(game, state, "pass_priority_to_first_strike_damage");
    state = record.result;
    const after = stateSummary(state);
    record.operation = after.gameOver
      ? "combat:first_strike_damage_and_sbas"
      : "priority_handoff:declare_blockers";
    if (!after.gameOver && before.priorityPlayer === after.priorityPlayer) {
      labelPriorityTransition(record, before, after);
    }
    records.push(record);
  }
  const gameOver = stateSummary(state).gameOver;
  assertCondition(
    gameOver?.kind === "winner" && Number(gameOver.player) === 0,
    "Expected Alice to win during first-strike combat damage",
    stateSummary(state),
  );
  return state;
}

async function runOnce(game, packagePath, runIndex) {
  let state = game.reset(["Alice", "Bob"], 20);
  addPuzzleCards(game);
  const scenarioSpellName = SCENARIO_HAND_SPELLS[options.scenario] ?? null;
  const scenarioSpellId = scenarioSpellName == null
    ? null
    : Number(game.addCardToHand(0, scenarioSpellName));

  const initial = timedCall(game, "initial_ui_state", () => game.uiState());
  state = initial.result;
  const records = [initial];

  for (const kind of ["keep_opening_hand", "keep_opening_hand", "continue_pregame", "begin_game"]) {
    const action = firstActionByKind(state, kind);
    const record = timedCall(game, kind, () =>
      game.dispatch(priorityCommand(action)),
    );
    records.push(record);
    state = record.result;
  }

  if (state?.decision?.kind !== "select_objects") {
    throw new Error(`Expected legend-rule object selection, got ${JSON.stringify(state?.decision)}`);
  }
  const keepId = state.decision.candidates?.[0]?.id;
  if (keepId == null) throw new Error("Legend-rule decision has no candidates");
  const legend = timedCall(game, "legend_rule_choice", () =>
    game.dispatch({ type: "select_objects", object_ids: [keepId] }),
  );
  records.push(legend);
  state = legend.result;

  if (options.scenario === "cast_mana") {
    assertCondition(scenarioSpellId != null, "cast_mana scenario spell was not added to hand");
    state = runCastManaScenario(game, state, records, scenarioSpellId);
  } else if (options.scenario === "cast_targeted") {
    assertCondition(scenarioSpellId != null, "cast_targeted scenario spell was not added to hand");
    state = runCastTargetedScenario(game, state, records, scenarioSpellId);
  } else if (options.scenario === "no_attack") {
    state = runNoAttackScenario(game, state, records);
  } else if (options.scenario === "round_trip") {
    state = runRoundTripScenario(game, state, records);
  } else if (options.scenario === "all_attack") {
    state = runAllAttackScenario(game, state, records);
  } else {
    for (let pass = 0; pass < options.phasePasses; pass += 1) {
      if (state?.decision?.kind !== "priority") break;
      const before = stateSummary(state);
      const record = passPriority(game, state);
      state = record.result;
      labelPriorityTransition(record, before, stateSummary(state));
      records.push(record);
    }
  }

  for (const record of records) delete record.result;
  return {
    runIndex,
    scenario: options.scenario,
    packagePath,
    battlefieldCards: 1072,
    libraryFillers: 2,
    scenarioHandSpell: scenarioSpellName == null
      ? null
      : { name: scenarioSpellName, objectId: scenarioSpellId },
    records,
    finalState: stateSummary(state),
  };
}

function percentile(sorted, percentileValue) {
  if (sorted.length === 0) return null;
  const index = Math.max(0, Math.ceil(percentileValue * sorted.length) - 1);
  return round(sorted[index]);
}

function summarize(runs) {
  const grouped = new Map();
  for (const run of runs) {
    for (const record of run.records) {
      const values = grouped.get(record.operation) || [];
      values.push(record.elapsedMs);
      grouped.set(record.operation, values);
    }
  }
  return Object.fromEntries(
    [...grouped.entries()].map(([operation, values]) => {
      const sorted = [...values].sort((left, right) => left - right);
      return [
        operation,
        {
          samples: sorted.length,
          minMs: round(sorted[0]),
          medianMs: percentile(sorted, 0.5),
          p95Ms: percentile(sorted, 0.95),
          maxMs: round(sorted.at(-1)),
        },
      ];
    }),
  );
}

function acceptance(runs, thresholdMs) {
  const measuredActions = runs.flatMap((run) =>
    run.records
      .filter((record) => record.operation !== "initial_ui_state")
      .map((record) => ({
        runIndex: run.runIndex,
        operation: record.operation,
        elapsedMs: record.elapsedMs,
        dispatchMs: record.dispatchMs,
      })),
  );
  const violations = measuredActions.filter((record) => record.elapsedMs >= thresholdMs);
  return {
    thresholdMs,
    measuredActions: measuredActions.length,
    passed: violations.length === 0,
    violations,
  };
}

const options = parseArgs(process.argv.slice(2));

async function main() {
  const { game, packagePath } = await initWasmGame({ pkg: options.pkg });
  const wasmArtifactUrl = new URL(`../${packagePath}/ironsmith_bg.wasm`, import.meta.url);
  const [wasmBytes, wasmStat] = await Promise.all([
    readFile(wasmArtifactUrl),
    stat(wasmArtifactUrl),
  ]);
  const artifact = {
    packagePath,
    wasmBytes: wasmStat.size,
    wasmModifiedAt: wasmStat.mtime.toISOString(),
    wasmSha256: createHash("sha256").update(wasmBytes).digest("hex"),
  };
  for (let index = 0; index < options.warmups; index += 1) {
    await runOnce(game, packagePath, `warmup-${index + 1}`);
  }
  const runs = [];
  for (let index = 0; index < options.runs; index += 1) {
    runs.push(await runOnce(game, packagePath, index + 1));
  }
  const acceptanceResult = acceptance(runs, options.thresholdMs);
  const report = {
    generatedAt: new Date().toISOString(),
    options,
    artifact,
    environment: {
      node: process.version,
      platform: process.platform,
      architecture: process.arch,
      cpu: cpus()[0]?.model ?? null,
    },
    board: {
      battlefieldCards: 1072,
      creatureGroups: Object.fromEntries(CREATURE_GROUPS),
      continuousEffectSources: EFFECT_SOURCES,
      copiesPerContinuousEffectSource: 6,
      libraryFillers: 2,
      scenarioHandSpell: SCENARIO_HAND_SPELLS[options.scenario] ?? null,
    },
    summary: summarize(runs),
    acceptance: acceptanceResult,
    runs,
  };

  if (options.output) {
    await mkdir(new URL("../reports/bench/", import.meta.url), { recursive: true });
    await writeFile(options.output, `${JSON.stringify(report, null, 2)}\n`);
  }
  if (options.json) console.log(JSON.stringify(report, null, 2));
  else console.table(report.summary);
  if (options.enforceThreshold && !acceptanceResult.passed) process.exitCode = 2;
}

main().catch((error) => {
  console.error(error?.stack || error);
  process.exitCode = 1;
});
