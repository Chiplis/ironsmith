#!/usr/bin/env node

import { readFile } from "node:fs/promises";
import { performance } from "node:perf_hooks";

const PLAYER_NAMES = ["ChipLis", "FY"];

const SELF_BATTLEFIELD = [
  "Indatha Triome",
  "Lush Portico",
  "Temple Garden",
  "Thundering Falls",
  "Ragavan, Nimble Pilferer",
  "Scion of Draco",
  "Territorial Kavu",
  "Leyline Binding",
];

const OPPONENT_BATTLEFIELD = [
  "Blood Crypt",
  "Indatha Triome",
  "Lush Portico",
  "Plains",
  "Territorial Kavu",
  "Territorial Kavu",
];

const SELF_GRAVEYARD = ["Flooded Strand", "Wooded Foothills"];
const OPPONENT_GRAVEYARD = ["Arid Mesa", "Arid Mesa"];
const SELF_HAND = ["Lightning Bolt", "Territorial Kavu", "Doorkeeper Thrull"];

function parseArgs(argv) {
  const options = {
    runs: 3,
    warmup: true,
    json: false,
    pkg: "demo",
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--json") {
      options.json = true;
    } else if (arg === "--no-warmup") {
      options.warmup = false;
    } else if (arg === "--runs") {
      const value = Number(argv[++index]);
      if (!Number.isInteger(value) || value < 1) {
        throw new Error("--runs must be a positive integer");
      }
      options.runs = value;
    } else if (arg === "--pkg") {
      const value = argv[++index];
      if (!["demo", "root"].includes(value)) {
        throw new Error("--pkg must be either demo or root");
      }
      options.pkg = value;
    } else {
      throw new Error(`Unknown argument: ${arg}`);
    }
  }

  return options;
}

function packageBase(pkg) {
  return pkg === "root" ? "../pkg" : "../web/wasm_demo/pkg";
}

async function initGame(pkg) {
  const base = packageBase(pkg);
  const wasmModule = await import(`${base}/ironsmith.js`);
  const wasmBytes = await readFile(new URL(`${base}/ironsmith_bg.wasm`, import.meta.url));
  await wasmModule.default({ module_or_path: wasmBytes });
  return {
    game: new wasmModule.WasmGame(),
    packagePath: base.replace(/^\.\.\//, ""),
  };
}

function round(value) {
  return Number.isFinite(value) ? Number(value.toFixed(3)) : value;
}

function safeRead(fn) {
  if (typeof fn !== "function") return null;
  try {
    return fn();
  } catch {
    return null;
  }
}

function snapshotPerf(game) {
  return safeRead(() => game.lastSnapshotPerf()) || null;
}

function dispatchPerf(game) {
  return safeRead(() => game.lastDispatchPerf()) || null;
}

function metric(perf, camelName, snakeName = camelName.replace(/[A-Z]/g, (char) => `_${char.toLowerCase()}`)) {
  return perf?.[camelName] ?? perf?.[snakeName];
}

function numberMetric(perf, camelName, fallback = 0) {
  const value = metric(perf, camelName);
  return round(Number(value ?? fallback));
}

function summarizePriorityAdvance(perf) {
  if (!perf) return null;
  return {
    totalMs: numberMetric(perf, "totalMs"),
    resultKind: metric(perf, "resultKind"),
    priorityPlayer: metric(perf, "priorityPlayer") ?? null,
    actionCount: metric(perf, "actionCount") ?? null,
    stateBasedActionsMs: numberMetric(perf, "stateBasedActionsMs"),
    putTriggersMs: numberMetric(perf, "putTriggersMs"),
    computeLegalActionsMs: numberMetric(perf, "computeLegalActionsMs"),
    computeCommanderActionsMs: numberMetric(perf, "computeCommanderActionsMs"),
    legalDetail: metric(perf, "computeLegalActionsDetail")
      ? {
          totalMs: numberMetric(metric(perf, "computeLegalActionsDetail"), "totalMs"),
          actionCount: metric(metric(perf, "computeLegalActionsDetail"), "actionCount") ?? null,
          prewarmMs: numberMetric(metric(perf, "computeLegalActionsDetail"), "prewarmMs"),
          handCastsMs: numberMetric(metric(perf, "computeLegalActionsDetail"), "handCastsMs"),
          graveyardCastsMs: numberMetric(metric(perf, "computeLegalActionsDetail"), "graveyardCastsMs"),
          exileCastsMs: numberMetric(metric(perf, "computeLegalActionsDetail"), "exileCastsMs"),
          battlefieldAbilitiesMs: numberMetric(metric(perf, "computeLegalActionsDetail"), "battlefieldAbilitiesMs"),
          canCastSpellWithViewMs: numberMetric(metric(perf, "computeLegalActionsDetail"), "canCastSpellWithViewMs"),
          computePotentialManaWithViewMs: numberMetric(metric(perf, "computeLegalActionsDetail"), "computePotentialManaWithViewMs"),
          handCastsCostAdjustmentMs: numberMetric(metric(perf, "computeLegalActionsDetail"), "handCastsCostAdjustmentMs"),
          handCastsAffordabilityMs: numberMetric(metric(perf, "computeLegalActionsDetail"), "handCastsAffordabilityMs"),
          handCastsTargetLegalityMs: numberMetric(metric(perf, "computeLegalActionsDetail"), "handCastsTargetLegalityMs"),
          battlefieldAbilityPrecheckMs: numberMetric(metric(perf, "computeLegalActionsDetail"), "battlefieldAbilityPrecheckMs"),
          battlefieldAbilityAffordabilityMs: numberMetric(metric(perf, "computeLegalActionsDetail"), "battlefieldAbilityAffordabilityMs"),
        }
      : null,
  };
}

function summarizeManaPipPayment(perf) {
  if (!perf) return null;
  return {
    pendingKind: metric(perf, "pendingKind"),
    remainingPipsBefore: metric(perf, "remainingPipsBefore") ?? null,
    remainingPipsAfter: metric(perf, "remainingPipsAfter") ?? null,
    cachedOptionCount: metric(perf, "cachedOptionCount") ?? null,
    builtOptionCount: metric(perf, "builtOptionCount") ?? null,
    usedCachedOptions: Boolean(metric(perf, "usedCachedOptions")),
    buildOptionsMs: numberMetric(perf, "buildOptionsMs"),
    executePaymentMs: numberMetric(perf, "executePaymentMs"),
    queueManaEventMs: numberMetric(perf, "queueManaEventMs"),
    drainTriggersMs: numberMetric(perf, "drainTriggersMs"),
    continueCastMs: numberMetric(perf, "continueCastMs"),
    continueActivationMs: numberMetric(perf, "continueActivationMs"),
    pipPaid: Boolean(metric(perf, "pipPaid")),
    resultKind: metric(perf, "resultKind"),
  };
}

function summarizePriorityAction(perf) {
  if (!perf) return null;
  return {
    totalMs: numberMetric(perf, "totalMs"),
    actionKind: metric(perf, "actionKind"),
    priorityResult: metric(perf, "priorityResult"),
    passPriorityMs: numberMetric(perf, "passPriorityMs"),
    responseApplyMs: numberMetric(perf, "responseApplyMs"),
    advancePriorityMs: numberMetric(perf, "advancePriorityMs"),
    resolveStackEntryMs: numberMetric(perf, "resolveStackEntryMs"),
    resetPriorityMs: numberMetric(perf, "resetPriorityMs"),
    manaPipPayment: summarizeManaPipPayment(metric(perf, "manaPipPayment")),
    nestedPriorityAdvance: summarizePriorityAdvance(metric(perf, "nestedPriorityAdvance")),
  };
}

function summarizeState(state) {
  return {
    decision: state?.decision?.kind ?? null,
    phase: state?.phase ?? null,
    step: state?.step ?? null,
    priorityPlayer: state?.priority_player ?? null,
    stackSize: state?.stack_size ?? null,
    battlefieldSize: state?.battlefield_size ?? null,
    actionCount: state?.decision?.actions?.length ?? null,
  };
}

function timeCall(game, label, fn) {
  const startedAt = performance.now();
  const state = fn();
  const totalMs = performance.now() - startedAt;
  const snapshot = snapshotPerf(game);
  const dispatch = dispatchPerf(game);
  const snapshotMs = Number(snapshot?.totalSnapshotMs ?? 0);
  return {
    label,
    totalMs: round(totalMs),
    snapshotMs: round(snapshotMs),
    snapshotBuildMs: round(Number(snapshot?.snapshotBuildMs ?? 0)),
    snapshotEncodeMs: round(Number(snapshot?.snapshotEncodeMs ?? 0)),
    pendingStackInsertMs: round(Number(snapshot?.pendingStackInsertMs ?? 0)),
    engineMinusSnapshotMs: round(totalMs - snapshotMs),
    dispatch: dispatch
      ? {
          routeKind: dispatch.routeKind,
          outcomeKind: dispatch.outcomeKind,
          commandToResponseMs: round(Number(dispatch.commandToResponseMs ?? 0)),
          checkpointCaptureMs: round(Number(dispatch.checkpointCaptureMs ?? 0)),
          executeWithReplayMs: round(Number(dispatch.executeWithReplayMs ?? 0)),
          applyProgressMs: round(Number(dispatch.applyProgressMs ?? 0)),
          totalDispatchMs: round(Number(dispatch.totalDispatchMs ?? 0)),
          snapshotMs: round(Number(dispatch.snapshot?.totalSnapshotMs ?? snapshotMs ?? 0)),
          advanceUntilDecisionMs: round(Number(dispatch.advanceUntilDecision?.totalMs ?? 0)),
          replayExecutionMs: round(Number(dispatch.replayExecution?.totalMs ?? 0)),
          priorityAction: summarizePriorityAction(dispatch.replayExecution?.priorityAction),
          priorityAdvance: summarizePriorityAdvance(dispatch.replayExecution?.priorityAdvance),
        }
      : null,
    state: summarizeState(state),
    result: state,
  };
}

function firstAction(state, labels) {
  const actions = state?.decision?.actions || [];
  for (const label of labels) {
    const action = actions.find((candidate) => candidate.label === label);
    if (action) return action;
  }
  throw new Error(`Could not find action ${labels.join(" / ")}. Available: ${actions.map((a) => a.label).join(" | ")}`);
}

function actionByPredicate(state, predicate, description) {
  const actions = state?.decision?.actions || [];
  const action = actions.find(predicate);
  if (!action) {
    throw new Error(`Could not find ${description}. Available: ${actions.map((a) => a.label).join(" | ")}`);
  }
  return action;
}

function advanceToFirstMain(game) {
  let state = game.startMatch({
    playerNames: PLAYER_NAMES,
    startingLife: 20,
    seed: 1,
    format: "normal",
    decks: [[], []],
    openingHandSize: 0,
  });

  let safety = 0;
  while (!(state.phase === "first main phase" && state.priority_player === 0 && state.decision?.kind === "priority")) {
    const action = firstAction(state, ["Keep hand", "Pregame", "Continue", "Begin game", "Pass priority"]);
    state = game.dispatch({ type: "priority_action", action_index: action.index });
    safety += 1;
    if (safety > 32) {
      throw new Error("Could not advance empty match to first main phase");
    }
  }
  return state;
}

function addCards(game, playerIndex, names, zone) {
  for (const name of names) {
    game.addCardToZone(playerIndex, name, zone, true);
  }
}

function setupScreenshotBoard(game) {
  advanceToFirstMain(game);
  addCards(game, 0, SELF_BATTLEFIELD, "Battlefield");
  addCards(game, 1, OPPONENT_BATTLEFIELD, "Battlefield");
  addCards(game, 0, SELF_GRAVEYARD, "Graveyard");
  addCards(game, 1, OPPONENT_GRAVEYARD, "Graveyard");
  for (const name of SELF_HAND) {
    game.addCardToHand(0, name);
  }
  const leylineBindingId = game.addCardToHand(0, "Leyline Binding");
  game.setLife(0, 17);
  game.setLife(1, 14);
  return { state: game.uiState(), leylineBindingId };
}

function selectWhiteLushPorticoOption(state) {
  return (
    state.decision?.options?.find((option) => option.description === "Tap Lush Portico: Add {W}") ||
    state.decision?.options?.find((option) => option.description.includes("{W}")) ||
    state.decision?.options?.[0]
  );
}

function firstLegalTarget(state) {
  const target = state.decision?.requirements?.[0]?.legal_targets?.[0];
  if (!target) {
    throw new Error(`Expected a target decision, got ${JSON.stringify(state.decision)}`);
  }
  return target;
}

function runScenario(game, runLabel) {
  const records = [];
  let { state, leylineBindingId } = setupScreenshotBoard(game);
  const setupSummary = summarizeState(state);

  const castAction = actionByPredicate(
    state,
    (action) => action.kind === "cast_spell" && Number(action.object_id) === Number(leylineBindingId),
    "Leyline Binding cast action"
  );
  let record = timeCall(game, `${runLabel}/cast_leyline_binding`, () =>
    game.dispatch({ type: "priority_action", action_index: castAction.index })
  );
  records.push(record);
  state = record.result;

  const whitePayment = selectWhiteLushPorticoOption(state);
  if (!whitePayment) {
    throw new Error(`No mana payment option found: ${JSON.stringify(state.decision)}`);
  }
  record = timeCall(game, `${runLabel}/pay_w_lush_portico`, () =>
    game.dispatch({ type: "select_options", option_indices: [whitePayment.index] })
  );
  records.push(record);
  state = record.result;

  record = timeCall(game, `${runLabel}/pass_self_with_binding_on_stack`, () =>
    game.dispatch({ type: "priority_action", action_index: firstAction(state, ["Pass priority"]).index })
  );
  records.push(record);
  state = record.result;

  record = timeCall(game, `${runLabel}/pass_opponent_to_trigger_target`, () =>
    game.dispatch({ type: "priority_action", action_index: firstAction(state, ["Pass priority"]).index })
  );
  records.push(record);
  state = record.result;

  const target = firstLegalTarget(state);
  record = timeCall(game, `${runLabel}/select_binding_target_${target.object}`, () =>
    game.dispatch({ type: "select_targets", targets: [target] })
  );
  records.push(record);
  state = record.result;

  record = timeCall(game, `${runLabel}/pass_self_after_target`, () =>
    game.dispatch({ type: "priority_action", action_index: firstAction(state, ["Pass priority"]).index })
  );
  records.push(record);
  state = record.result;

  record = timeCall(game, `${runLabel}/pass_opponent_resolve_binding`, () =>
    game.dispatch({ type: "priority_action", action_index: firstAction(state, ["Pass priority"]).index })
  );
  records.push(record);

  return {
    setup: setupSummary,
    target,
    records: records.map(({ result: _result, ...entry }) => entry),
  };
}

function summarizeRuns(runs) {
  const labels = runs[0]?.records.map((record) => record.label.replace(/^run_\d+\//, "")) || [];
  return labels.map((label, index) => {
    const values = runs.map((run) => run.records[index].totalMs);
    const sorted = [...values].sort((left, right) => left - right);
    const mean = values.reduce((sum, value) => sum + value, 0) / values.length;
    return {
      label,
      minMs: round(sorted[0]),
      meanMs: round(mean),
      maxMs: round(sorted[sorted.length - 1]),
    };
  });
}

function printHuman(summary) {
  console.log(`Package: ${summary.packagePath}`);
  console.log(`Warmup: ${summary.warmup ? "yes" : "no"}`);
  console.log(`Timed runs: ${summary.runs.length}`);
  console.log(`Board: battlefield=${summary.runs[0].setup.battlefieldSize}, starting actions=${summary.runs[0].setup.actionCount}`);
  console.log("");
  console.log("Per-action wall time, ms (min / mean / max):");
  for (const row of summary.aggregate) {
    console.log(`  ${row.label}: ${row.minMs} / ${row.meanMs} / ${row.maxMs}`);
  }
  console.log("");
  console.log("Last run details:");
  const lastRun = summary.runs.at(-1);
  for (const record of lastRun.records) {
    const label = record.label.replace(/^run_\d+\//, "");
    console.log(
      `  ${label}: total=${record.totalMs}ms, snapshot=${record.snapshotMs}ms, engine-minus-snapshot=${record.engineMinusSnapshotMs}ms, decision=${record.state.decision}, stack=${record.state.stackSize}`
    );
    const priorityAction = record.dispatch?.priorityAction;
    if (priorityAction) {
      const nested = priorityAction.nestedPriorityAdvance;
      const legal = nested?.legalDetail;
      console.log(
        `    action=${priorityAction.actionKind}, apply=${priorityAction.responseApplyMs}ms, advance=${priorityAction.advancePriorityMs}ms, result=${priorityAction.priorityResult}`
      );
      if (priorityAction.manaPipPayment) {
        const mana = priorityAction.manaPipPayment;
        console.log(
          `    mana: cached=${mana.cachedOptionCount}, build=${mana.buildOptionsMs}ms, execute=${mana.executePaymentMs}ms, drain=${mana.drainTriggersMs}ms, continueCast=${mana.continueCastMs}ms`
        );
      }
      if (nested) {
        console.log(
          `    nested advance: total=${nested.totalMs}ms, sba=${nested.stateBasedActionsMs}ms, triggers=${nested.putTriggersMs}ms, legal=${nested.computeLegalActionsMs}ms, actions=${nested.actionCount}`
        );
      }
      if (legal) {
        console.log(
          `    legal detail: total=${legal.totalMs}ms, hand=${legal.handCastsMs}ms, battlefield=${legal.battlefieldAbilitiesMs}ms, potentialMana=${legal.computePotentialManaWithViewMs}ms`
        );
      }
    }
  }
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  const { game, packagePath } = await initGame(options.pkg);

  if (options.warmup) {
    runScenario(game, "warmup");
  }

  const runs = [];
  for (let index = 0; index < options.runs; index += 1) {
    runs.push(runScenario(game, `run_${index + 1}`));
  }

  const summary = {
    packagePath,
    warmup: options.warmup,
    runs,
    aggregate: summarizeRuns(runs),
  };

  if (options.json) {
    console.log(JSON.stringify(summary, null, 2));
  } else {
    printHuman(summary);
  }
}

main().catch((error) => {
  console.error(error instanceof Error ? error.stack || error.message : String(error));
  process.exitCode = 1;
});
