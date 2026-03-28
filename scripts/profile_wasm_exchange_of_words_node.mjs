#!/usr/bin/env node

import initWasm, { WasmGame } from "../pkg/ironsmith.js";
import { readFile } from "node:fs/promises";
import { performance } from "node:perf_hooks";

const PLAYER_NAMES = ["Alice", "Bob", "Charlie", "Diana"];
const LAND_PRESET = [
  "Forest",
  "Island",
  "Mountain",
  "Plains",
  "Swamp",
  "Tropical Island",
  "Volcanic Island",
];
const CREATURE_PRESET = [
  "Myr Moonvessel",
  "Ornithopter",
  "Yawgmoth, Thran Physician",
  "Omniscience",
];

function parseArgs(argv) {
  return {
    double: argv.includes("--double"),
    json: argv.includes("--json"),
  };
}

function safePerfRead(fn) {
  if (typeof fn !== "function") return null;
  try {
    return fn();
  } catch {
    return null;
  }
}

function capturePerf(game) {
  return {
    snapshot: safePerfRead(() => game.lastSnapshotPerf()),
    dispatch: safePerfRead(() => game.lastDispatchPerf()),
    replayExecution: safePerfRead(() => game.lastReplayExecutionPerf()),
    advanceUntilDecision: safePerfRead(() => game.lastAdvanceUntilDecisionPerf()),
  };
}

function dispatchWithPerf(game, command, label) {
  const startedAt = performance.now();
  const result = game.dispatch(command);
  const totalMs = performance.now() - startedAt;
  return {
    label,
    command,
    totalMs,
    result,
    perf: capturePerf(game),
  };
}

function addBattlefieldPreset(game, playerIndex) {
  for (const name of CREATURE_PRESET) {
    game.addCardToZone(playerIndex, name, "Battlefield", true);
  }
  for (const name of LAND_PRESET) {
    game.addCardToZone(playerIndex, name, "Battlefield", true);
  }
}

function actionByPredicate(state, predicate, description) {
  const actions = state?.decision?.actions || [];
  const action = actions.find(predicate);
  if (!action) {
    throw new Error(
      `Could not find action for ${description}. Available: ${actions
        .map((candidate) => candidate.label)
        .join(" | ")}`
    );
  }
  return action;
}

function firstPassAction(state) {
  return actionByPredicate(
    state,
    (action) => action?.action_ref?.kind === "pass_priority",
    "pass priority"
  );
}

function targetObjectId(state, playerIndex, namePrefix) {
  const battlefield = state.players?.[playerIndex]?.battlefield || [];
  const match = battlefield.find((card) => card.name.startsWith(namePrefix));
  if (!match) {
    throw new Error(`Could not find battlefield object with prefix "${namePrefix}"`);
  }
  return Number(match.id);
}

function summarizeAutoPassStep(step) {
  const priorityAdvance = step.perf.advanceUntilDecision?.replayExecution?.priorityAdvance;
  const detail = priorityAdvance?.compute_legal_actions_detail;
  return {
    label: step.label,
    totalMs: round(step.totalMs),
    fromPriorityPlayer: step.fromPriorityPlayer,
    toPriorityPlayer: step.toPriorityPlayer,
    beforeStack: step.beforeStack,
    afterStack: step.afterStack,
    phase: step.afterPhase,
    step: step.afterStep,
    computeLegalActionsMs: priorityAdvance?.compute_legal_actions_ms ?? null,
    handCastsMs: detail?.hand_casts_ms ?? null,
    handAlternativesMs: detail?.hand_alternatives_ms ?? null,
    battlefieldAbilitiesMs: detail?.battlefield_abilities_ms ?? null,
  };
}

function round(value) {
  return Number.isFinite(value) ? Number(value.toFixed(3)) : value;
}

async function initGame() {
  const wasmBytes = await readFile(new URL("../pkg/ironsmith_bg.wasm", import.meta.url));
  await initWasm({ module_or_path: wasmBytes });
  return new WasmGame();
}

function setupScenario(game) {
  game.reset(PLAYER_NAMES, 20);
  addBattlefieldPreset(game, 0);
  addBattlefieldPreset(game, 1);
  return game.addCardToHand(0, "Exchange of Words");
}

function advancePregameToFirstMain(game, steps) {
  let state = game.uiState();
  let safety = 0;
  while (!(state.phase === "first main phase" && state.priority_player === 0)) {
    const action = firstResolvablePregameOrPassAction(state);
    const step = dispatchWithPerf(
      game,
      { type: "priority_action", action_index: action.index },
      `pregame/${action.label}`
    );
    steps.push(step);
    state = step.result;
    safety += 1;
    if (safety > 32) {
      throw new Error("Pregame/main-phase advancement exceeded safety limit");
    }
  }
  return state;
}

function firstResolvablePregameOrPassAction(state) {
  const actions = state?.decision?.actions || [];
  const preferred = ["Keep hand", "Continue", "Begin game", "Pass priority"];
  for (const label of preferred) {
    const match = actions.find((action) => action.label === label);
    if (match) return match;
  }
  throw new Error(
    `Could not find pregame/pass action. Available: ${actions.map((a) => a.label).join(" | ")}`
  );
}

function autoPassUntil(game, predicate, labelPrefix) {
  const steps = [];
  let state = game.uiState();
  let safety = 0;
  while (!predicate(state)) {
    const pass = firstPassAction(state);
    const beforePriorityPlayer = state.priority_player ?? null;
    const beforeStack = state.stack_size ?? null;
    const step = dispatchWithPerf(
      game,
      { type: "priority_action", action_index: pass.index },
      `${labelPrefix}/pass_${safety + 1}`
    );
    step.fromPriorityPlayer = beforePriorityPlayer;
    step.toPriorityPlayer = step.result.priority_player ?? null;
    step.beforeStack = beforeStack;
    step.afterStack = step.result.stack_size ?? null;
    step.afterPhase = step.result.phase ?? null;
    step.afterStep = step.result.step ?? null;
    steps.push(step);
    state = step.result;
    safety += 1;
    if (safety > 32) {
      throw new Error(`Auto-pass loop "${labelPrefix}" exceeded safety limit`);
    }
  }
  return { steps, state };
}

function castAndResolveExchange(game, exchangeId, passIndexPrefix) {
  const records = [];
  let state = game.uiState();

  const castAction = actionByPredicate(
    state,
    (action) =>
      action.kind === "cast_spell" &&
      Number(action.object_id) === Number(exchangeId) &&
      action.label.includes("Exchange of Words"),
    `cast action for Exchange of Words ${exchangeId}`
  );
  records.push(
    dispatchWithPerf(
      game,
      { type: "priority_action", action_index: castAction.index },
      `${passIndexPrefix}/choose_cast_method`
    )
  );

  state = records.at(-1).result;
  const freeOption = (state.decision?.options || []).find((option) =>
    option.description.includes("Without paying mana cost")
  );
  if (!freeOption) {
    throw new Error("Could not find free-cast option for Exchange of Words");
  }

  records.push(
    dispatchWithPerf(
      game,
      { type: "select_options", option_indices: [freeOption.index] },
      `${passIndexPrefix}/choose_free_cast`
    )
  );

  let auto = autoPassUntil(
    game,
    (current) => current.decision?.kind === "targets",
    `${passIndexPrefix}/to_targets`
  );
  records.push(...auto.steps);
  state = auto.state;

  const yawgmothId = targetObjectId(state, 0, "Yawgmoth");
  const ornithopterId = targetObjectId(state, 0, "Ornithopter");
  records.push(
    dispatchWithPerf(
      game,
      {
        type: "select_targets",
        targets: [
          { kind: "object", object: yawgmothId },
          { kind: "object", object: ornithopterId },
        ],
      },
      `${passIndexPrefix}/select_targets`
    )
  );

  auto = autoPassUntil(
    game,
    (current) =>
      current.phase === "first main phase" &&
      current.priority_player === 0 &&
      current.stack_size === 0 &&
      current.decision?.kind === "priority",
    `${passIndexPrefix}/post_targets`
  );
  records.push(...auto.steps);

  return {
    records,
    finalState: auto.state,
  };
}

function summarizeTimeline(records) {
  return records.map((record) => ({
    label: record.label,
    totalMs: round(record.totalMs),
    dispatch: record.perf.dispatch || null,
    replayExecution: record.perf.replayExecution || null,
    advanceUntilDecision: record.perf.advanceUntilDecision || null,
    snapshot: record.perf.snapshot || null,
    ...(record.fromPriorityPlayer !== undefined
      ? {
          fromPriorityPlayer: record.fromPriorityPlayer,
          toPriorityPlayer: record.toPriorityPlayer,
          beforeStack: record.beforeStack,
          afterStack: record.afterStack,
          afterPhase: record.afterPhase,
          afterStep: record.afterStep,
        }
      : {}),
  }));
}

function printHumanSummary(summary) {
  console.log(`Scenario: ${summary.double ? "double" : "single"} Exchange of Words`);
  console.log("Runtime: release WASM via Node (no browser render/worker transfer)");
  console.log("");

  const keySteps = summary.timeline.filter(
    (entry) =>
      entry.label.includes("choose_cast_method") ||
      entry.label.includes("choose_free_cast") ||
      entry.label.includes("select_targets") ||
      entry.label.includes("post_targets/pass")
  );

  for (const entry of keySteps) {
    console.log(`${entry.label}: ${entry.totalMs}ms`);
    const priorityAdvance = entry.advanceUntilDecision?.replayExecution?.priorityAdvance;
    if (priorityAdvance?.compute_legal_actions_ms != null) {
      console.log(
        `  compute_legal_actions_ms=${priorityAdvance.compute_legal_actions_ms}, hand_casts_ms=${priorityAdvance.compute_legal_actions_detail?.hand_casts_ms ?? "n/a"}, hand_alternatives_ms=${priorityAdvance.compute_legal_actions_detail?.hand_alternatives_ms ?? "n/a"}, battlefield_abilities_ms=${priorityAdvance.compute_legal_actions_detail?.battlefield_abilities_ms ?? "n/a"}`
      );
    }
  }

  console.log("");
  const hottestAutoPass = [...summary.timeline]
    .filter((entry) => entry.label.includes("post_targets/pass"))
    .sort(
      (left, right) =>
        (right.advanceUntilDecision?.replayExecution?.priorityAdvance?.compute_legal_actions_ms ?? -1) -
        (left.advanceUntilDecision?.replayExecution?.priorityAdvance?.compute_legal_actions_ms ?? -1)
    )[0];
  if (hottestAutoPass) {
    const priorityAdvance = hottestAutoPass.advanceUntilDecision?.replayExecution?.priorityAdvance;
    console.log(`Hottest auto-pass: ${hottestAutoPass.label}`);
    console.log(
      `  compute_legal_actions_ms=${priorityAdvance?.compute_legal_actions_ms ?? "n/a"}`
    );
    console.log(
      `  total step ms=${hottestAutoPass.totalMs}, from priority ${hottestAutoPass.fromPriorityPlayer} -> ${hottestAutoPass.toPriorityPlayer}, stack ${hottestAutoPass.beforeStack} -> ${hottestAutoPass.afterStack}`
    );
  }
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  const game = await initGame();
  const setupExchangeId = setupScenario(game);
  const pregameSteps = [];
  advancePregameToFirstMain(game, pregameSteps);

  const first = castAndResolveExchange(game, setupExchangeId, "exchange_1");
  const timeline = [...pregameSteps, ...first.records];

  if (options.double) {
    const secondExchangeId = game.addCardToHand(0, "Exchange of Words");
    const second = castAndResolveExchange(game, secondExchangeId, "exchange_2");
    timeline.push(...second.records);
  }

  const summary = {
    double: options.double,
    timeline: summarizeTimeline(timeline),
  };

  if (options.json) {
    console.log(JSON.stringify(summary, null, 2));
    return;
  }

  printHumanSummary(summary);
}

main().catch((error) => {
  console.error(error instanceof Error ? error.stack || error.message : String(error));
  process.exitCode = 1;
});
