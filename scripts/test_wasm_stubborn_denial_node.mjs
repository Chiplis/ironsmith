#!/usr/bin/env node

import { readFile } from "node:fs/promises";

const PLAYER_NAMES = ["Alice", "Bob"];

function parseArgs(argv) {
  const options = {
    pkg: "root",
    json: false,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--json") {
      options.json = true;
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

function assert(condition, message, details) {
  if (condition) return;
  const suffix = details === undefined ? "" : `\n${JSON.stringify(details, null, 2)}`;
  throw new Error(`${message}${suffix}`);
}

function actionSummaries(state) {
  return (state?.decision?.actions || []).map((action) => ({
    index: action.index,
    kind: action.kind,
    label: action.label,
    objectId: action.object_id === undefined ? null : Number(action.object_id),
  }));
}

function stackSummary(state) {
  return (state?.stack_objects || []).map((object) => ({
    id: Number(object.id),
    name: object.name,
    controller: object.controller,
    targets: object.targets || [],
  }));
}

function firstAction(state, labels) {
  const actions = state?.decision?.actions || [];
  for (const label of labels) {
    const action = actions.find((candidate) => candidate.label === label);
    if (action) return action;
  }
  throw new Error(
    `Could not find action ${labels.join(" / ")}. Available: ${actions
      .map((action) => action.label)
      .join(" | ")}`
  );
}

function actionByPredicate(state, predicate, description) {
  const action = (state?.decision?.actions || []).find(predicate);
  if (!action) {
    throw new Error(
      `Could not find ${description}. Available: ${actionSummaries(state)
        .map((action) => `${action.index}:${action.label}`)
        .join(" | ")}`
    );
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
    const action = firstAction(state, ["Keep hand", "Continue", "Begin game", "Pass priority"]);
    state = game.dispatch({ type: "priority_action", action_index: action.index });
    safety += 1;
    if (safety > 32) {
      throw new Error("Could not advance empty match to Alice's first main phase");
    }
  }
  return state;
}

function setupScenario(game) {
  advanceToFirstMain(game);

  const lightningBoltId = Number(game.addCardToHand(0, "Lightning Bolt"));
  const stubbornDenialId = Number(game.addCardToHand(0, "Stubborn Denial"));
  game.addCardToZone(0, "Omniscience", "Battlefield", true);
  game.addCardToZone(0, "Primeval Titan", "Battlefield", true);

  return {
    lightningBoltId,
    stubbornDenialId,
    state: game.uiState(),
  };
}

function castSpell(game, state, objectId, name) {
  const action = actionByPredicate(
    state,
    (candidate) => candidate.kind === "cast_spell" && Number(candidate.object_id) === Number(objectId),
    `${name} cast action`
  );
  return game.dispatch({ type: "priority_action", action_index: action.index });
}

function allLegalTargets(state) {
  return (state?.decision?.requirements || []).flatMap((requirement) => requirement.legal_targets || []);
}

function targetObjectId(target) {
  const id = target?.object ?? target?.id;
  return id === undefined ? null : Number(id);
}

function playerLife(state, playerIndex) {
  return state?.players?.[playerIndex]?.life ?? state?.life_totals?.[playerIndex] ?? null;
}

function zoneNames(state, playerIndex, zoneName) {
  const player = state?.players?.[playerIndex] || {};
  return (player[zoneName] || player[`${zoneName}_cards`] || []).map((card) => card.name);
}

function passPriority(game, state, label) {
  assert(
    state?.decision?.kind === "priority",
    `Expected priority before ${label}`,
    {
      decision: state?.decision?.kind,
      stack: stackSummary(state),
    }
  );
  const pass = actionByPredicate(
    state,
    (action) => action.action_ref?.kind === "pass_priority" || action.kind === "pass_priority",
    "pass priority action"
  );
  return game.dispatch({ type: "priority_action", action_index: pass.index });
}

function resolveUntilStackClear(game, state) {
  let passes = 0;
  while ((state.stack_objects || []).some((object) => object.name === "Stubborn Denial" || object.name === "Lightning Bolt")) {
    state = passPriority(game, state, `pass ${passes + 1}`);
    passes += 1;
    if (passes > 12) {
      throw new Error(`Stack did not clear after ${passes} priority passes: ${JSON.stringify(stackSummary(state), null, 2)}`);
    }
  }
  return { state, passes };
}

function runScenario(game) {
  let { state, lightningBoltId, stubbornDenialId } = setupScenario(game);

  state = castSpell(game, state, lightningBoltId, "Lightning Bolt");
  assert(
    state.decision?.kind === "targets",
    "Lightning Bolt should ask for a target",
    { decision: state.decision?.kind, stack: stackSummary(state) }
  );

  const bobTarget = allLegalTargets(state).find(
    (target) => target.kind === "player" && Number(target.player) === 1
  );
  assert(bobTarget, "Lightning Bolt should be able to target Bob", {
    legalTargets: allLegalTargets(state),
  });
  state = game.dispatch({ type: "select_targets", targets: [bobTarget] });

  const boltOnStack = (state.stack_objects || []).find((object) => object.name === "Lightning Bolt");
  assert(boltOnStack, "Lightning Bolt should be on the stack after choosing Bob", {
    stack: stackSummary(state),
  });

  state = castSpell(game, state, stubbornDenialId, "Stubborn Denial");
  assert(
    state.decision?.kind === "targets",
    "Stubborn Denial should ask which noncreature spell to counter",
    {
      decision: state.decision?.kind,
      stack: stackSummary(state),
      actions: actionSummaries(state),
    }
  );

  const denialTarget = allLegalTargets(state).find(
    (target) =>
      target.kind === "object" &&
      (targetObjectId(target) === Number(boltOnStack.id) || target.name === "Lightning Bolt")
  );
  assert(denialTarget, "Stubborn Denial should expose Lightning Bolt as a legal target", {
    boltOnStack: Number(boltOnStack.id),
    legalTargets: allLegalTargets(state),
  });

  state = game.dispatch({ type: "select_targets", targets: [denialTarget] });
  const denialOnStack = (state.stack_objects || []).find((object) => object.name === "Stubborn Denial");
  assert(
    denialOnStack?.targets?.some((target) => targetObjectId(target) === targetObjectId(denialTarget)),
    "Stubborn Denial should keep the selected Lightning Bolt target on the stack",
    { stack: stackSummary(state) }
  );

  const resolved = resolveUntilStackClear(game, state);
  state = resolved.state;

  assert(playerLife(state, 0) === 20, "Alice should not take damage from the countered Lightning Bolt", {
    aliceLife: playerLife(state, 0),
  });
  assert(playerLife(state, 1) === 20, "Bob should not take damage because Lightning Bolt was countered", {
    bobLife: playerLife(state, 1),
  });
  assert(zoneNames(state, 0, "graveyard").includes("Lightning Bolt"), "Lightning Bolt should be in Alice's graveyard", {
    aliceGraveyard: zoneNames(state, 0, "graveyard"),
  });
  assert(zoneNames(state, 0, "graveyard").includes("Stubborn Denial"), "Stubborn Denial should be in Alice's graveyard", {
    aliceGraveyard: zoneNames(state, 0, "graveyard"),
  });

  return {
    selectedTarget: denialTarget,
    passesAfterTarget: resolved.passes,
    lifeTotals: {
      Alice: playerLife(state, 0),
      Bob: playerLife(state, 1),
    },
    aliceGraveyard: zoneNames(state, 0, "graveyard"),
  };
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  const { game, packagePath } = await initGame(options.pkg);
  const result = runScenario(game);

  if (options.json) {
    console.log(JSON.stringify({ packagePath, ...result }, null, 2));
  } else {
    console.log(`PASS wasm Stubborn Denial countered Lightning Bolt through ${packagePath}`);
    console.log(
      `Target object ${targetObjectId(result.selectedTarget)} resolved after ${result.passesAfterTarget} priority passes; life Alice=${result.lifeTotals.Alice}, Bob=${result.lifeTotals.Bob}`
    );
  }
}

main().catch((error) => {
  console.error(error instanceof Error ? error.stack || error.message : String(error));
  process.exitCode = 1;
});
