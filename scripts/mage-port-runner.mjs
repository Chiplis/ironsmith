import test from "node:test";
import { readFileSync } from "node:fs";
import {
  actionByPredicate,
  addCustomCardWithAbility,
  addCustomEffectTargetDestroy,
  assert,
  countByName,
  getAttachedTo,
  getBattlefield,
  getCheckpoint,
  getExile,
  getGraveyard,
  getHand,
  getLibrary,
  getObjectsInZone,
  getObjectDetails,
  getPermanent,
  importCheckpoint,
  initWasmGame,
  initWasmRuntime,
  names,
  normalizePlayerId,
  runCode,
  showAvailableAbilities,
  startEmptyMatch,
} from "./wasm-test-harness.mjs";

const PLAYER_NAMES = ["Alice", "Bob"];
const MAX_ADVANCE_STEPS = 600;
const DEFAULT_LIBRARY_CARD = "Plains";
const DEFAULT_LIBRARY_SIZE = 60;

export function registerPortedMageTests(fileSpec) {
  const runtimePromise = initWasmRuntime({ pkg: fileSpec.pkg || "root" });
  for (const testSpec of fileSpec.tests || []) {
    const options = testSpec.skip ? { skip: testSpec.skip } : undefined;
    test(`${fileSpec.sourcePath} :: ${testSpec.name}`, options, async () => {
      if (process.env.MAGE_PORT_TEST_START_TRACE) {
        console.error(`[mage-port-test-start] ${fileSpec.sourcePath} :: ${testSpec.name}`);
      }
      const context = await createMagePortContext(fileSpec, testSpec, runtimePromise);
      await runOperations(context, testSpec.operations || []);
    });
  }
}

async function createMagePortContext(fileSpec, testSpec, runtimePromise = null) {
  const runtime = runtimePromise ? await runtimePromise : await initWasmGame({ pkg: fileSpec.pkg || "root" });
  const game = runtime.game ?? new runtime.wasmModule.WasmGame();
  startEmptyMatch(game, {
    playerNames: PLAYER_NAMES,
    startingLife: 20,
    seed: testSpec.seed || 1,
    openingHandSize: 0,
    decks: [defaultMageLibrary(), defaultMageLibrary()],
  });
  game.setAutoChooseSingleObjectDecisions?.(false);
  return {
    game,
    scheduled: [],
    choices: [],
    targets: [],
    modes: [],
    stopAt: null,
    strict: false,
    javaVariables: readJavaNumericVariables(fileSpec.sourcePath),
    sourcePath: fileSpec.sourcePath,
    testName: testSpec.name,
  };
}

function defaultMageLibrary() {
  return Array.from({ length: DEFAULT_LIBRARY_SIZE }, () => DEFAULT_LIBRARY_CARD);
}

async function runOperations(context, operations) {
  for (const [index, operation] of operations.entries()) {
    traceOperation(context, index, operation);
    await applyOperation(context, operation);
  }
}

function traceOperation(context, index, operation) {
  if (!process.env.MAGE_PORT_TRACE) return;
  const parts = [
    `[mage-port-trace] ${context.sourcePath} :: ${context.testName}`,
    `op#${index + 1}`,
    operation.op,
  ];
  for (const key of ["turn", "phase", "player", "name", "ability", "target"]) {
    if (operation[key] !== undefined) parts.push(`${key}=${JSON.stringify(operation[key])}`);
  }
  console.error(parts.join(" "));
}

async function applyOperation(context, operation) {
  switch (operation.op) {
    case "addCard":
      return addCard(context, operation);
    case "setLife":
      return context.game.setLife(playerIndex(operation.player), numericValue(operation.life));
    case "setStrictChooseMode":
      context.strict = Boolean(operation.value);
      return;
    case "skipInitShuffling":
      return;
    case "setChoice":
      context.choices.push(operation.value);
      return;
    case "setModeChoice":
      context.modes.push(operation.value);
      return;
    case "addTarget":
      context.targets.push({ player: playerIndex(operation.player), value: operation.target });
      return;
    case "castSpell":
    case "activateAbility":
    case "attack":
    case "block":
    case "playLand":
      context.scheduled.push(operation);
      return;
    case "setStopAt":
      context.stopAt = operation;
      return;
    case "execute":
      return executeScheduled(context);
    case "waitStackResolved":
      return waitStackResolved(context, operation);
    case "activateManaAbility":
      return activateManaAbility(context, operation);
    case "clearZone":
      return clearZone(context, operation);
    case "assertPlayableAbility":
      return assertPlayableAbility(context, operation);
    case "assertStackSize":
      return assertStackSize(context, operation);
    case "assertLife":
      return assertLife(context, operation);
    case "assertPermanentCount":
      return assertPermanentCount(context, operation);
    case "assertHandCount":
      return assertZoneCount(context, operation, "hand");
    case "assertGraveyardCount":
      return assertZoneCount(context, operation, "graveyard");
    case "assertExileCount":
      return assertExileCount(context, operation);
    case "assertLibraryCount":
      return assertZoneCount(context, operation, "library");
    case "assertPowerToughness":
      return assertPowerToughness(context, operation);
    case "assertTappedCount":
      return assertTappedCount(context, operation);
    case "assertCounterCount":
      return assertCounterCount(context, operation);
    case "assertAbility":
      return assertAbility(context, operation);
    case "assertAbilities":
      return assertAbilities(context, operation);
    case "unsupported":
      return applySupportedJavaHelper(context, operation);
    default:
      throw new Error(`unknown generated operation: ${JSON.stringify(operation)}`);
  }
}

function addCard(context, operation) {
  const count = Number(operation.count || 1);
  const player = playerIndex(operation.player);
  const zone = zoneName(operation.zone);
  const name = cardName(operation.name);
  for (let index = 0; index < count; index += 1) {
    if (operation.custom) {
      addCustomCardWithAbility(context.game, {
        player,
        zone,
        name,
        oracleText: operation.oracleText || "",
        typeLine: operation.typeLine || "Creature - Shapeshifter",
        power: operation.power ?? "1",
        toughness: operation.toughness ?? "1",
      });
    } else if (zone === "hand") {
      context.game.addCardToHand(player, name);
    } else {
      context.game.addCardToZone(player, name, zone, true);
    }
  }
}

async function applySupportedJavaHelper(context, operation) {
  const source = String(operation.source || "");
  const expectedExecuteError = source.trim().match(
    /^try \{\s*execute\(\);\s*\} catch \(Throwable e\) \{\s*if \(!e\.getMessage\(\)\.contains\("([^"]+)"\)\)/s,
  );
  if (expectedExecuteError) {
    const expected = expectedExecuteError[1];
    try {
      await executeScheduled(context);
    } catch (error) {
      assert(
        String(error?.message || error).includes(expected),
        `expected execute error to contain ${JSON.stringify(expected)}, got: ${error?.message || error}`,
      );
      return;
    }
    throw new Error(`expected execute to fail with ${JSON.stringify(expected)}`);
  }

  const destroy = source.match(/^addCustomEffect_TargetDestroy\(([^,\)]+)(?:,\s*(\d+))?\)$/);
  if (destroy) {
    const player = playerIndex(destroy[1]);
    const count = Number(destroy[2] || 1);
    for (let index = 0; index < count; index += 1) {
      addCustomEffectTargetDestroy(context.game, { player, name: "target destroy", manaCost: "{0}" });
    }
    return;
  }

  const transform = source.match(/^addCustomEffect_TargetTransform\(([^,\)]+)(?:,\s*(\d+))?\)$/);
  if (transform) {
    const player = playerIndex(transform[1]);
    const count = Number(transform[2] || 1);
    for (let index = 0; index < count; index += 1) {
      addCustomCardWithAbility(context.game, {
        player,
        zone: "hand",
        name: "target transform",
        manaCost: "{0}",
        typeLine: "Instant",
        oracleText: "Transform target permanent.",
        power: null,
        toughness: null,
      });
    }
    return;
  }

  const blink = source.match(/^addCustomEffect_TargetBlink\(([^,\)]+)(?:,\s*(\d+))?\)$/);
  if (blink) {
    const player = playerIndex(blink[1]);
    const count = Number(blink[2] || 1);
    for (let index = 0; index < count; index += 1) {
      addCustomCardWithAbility(context.game, {
        player,
        zone: "hand",
        name: "target blink",
        manaCost: "{0}",
        typeLine: "Instant",
        oracleText: "Exile target creature, then return it to the battlefield under its owner's control.",
        power: null,
        toughness: null,
      });
    }
    return;
  }

  const tapped = source.match(/^assertTapped\((.+),\s*(true|false)\)$/);
  if (tapped) {
    return assertTapped(context, {
      player: null,
      name: tapped[1],
      tapped: tapped[2] === "true",
    });
  }

  const damage = source.match(/^assertDamageReceived\(([^,]+),\s*(.+),\s*([^)]+)\)$/);
  if (damage) {
    return assertDamageReceived(context, {
      player: damage[1],
      name: damage[2],
      damage: damage[3],
    });
  }

  const attached = source.match(/^assertAttachedTo\(([^,]+),\s*(.+),\s*(.+),\s*(true|false)\)$/);
  if (attached) {
    return assertAttachedTo(context, {
      player: attached[1],
      attachment: attached[2],
      target: attached[3],
      expected: attached[4] === "true",
    });
  }

  const counters = source.match(
    /^checkPermanentCounters\((?:"[^"]*"|[^,]+),\s*(\d+),\s*([^,]+),\s*([^,]+),\s*(.+),\s*CounterType\.([A-Z0-9_]+),\s*([^)]+)\)$/,
  );
  if (counters) {
    return assertCounterCount(context, {
      turn: Number(counters[1]),
      phase: counters[2],
      player: counters[3],
      name: counters[4],
      counter: counters[5],
      count: counters[6],
    });
  }

  const stackObject = source.match(
    /^checkStackObject\((?:"[^"]*"|[^,]+),\s*(\d+),\s*([^,]+),\s*([^,]+),\s*"([^"]+)",\s*([^)]+)\)$/,
  );
  if (stackObject) {
    return assertStackObject(context, {
      turn: Number(stackObject[1]),
      phase: stackObject[2],
      player: stackObject[3],
      text: stackObject[4],
      count: stackObject[5],
    });
  }

  const choicesCount = source.match(/^assertChoicesCount\(([^,]+),\s*(\d+)\)$/);
  if (choicesCount) {
    return assertChoicesCount(context, {
      player: choicesCount[1],
      count: Number(choicesCount[2]),
    });
  }

  const dieRoll = source.match(/^(?:this\.)?setDieRollResult\(([^,]+),\s*([^)]+)\)$/);
  if (dieRoll) {
    const result = numericValue(resolveMageVariable(context, dieRoll[2]));
    assert(typeof context.game.forceNextDieRoll === "function", "WASM game does not expose forceNextDieRoll");
    context.game.forceNextDieRoll(result);
    return;
  }

  const typeCheck = source.match(/^assertType\((.+),\s*CardType\.([A-Z_]+)(?:,\s*(.+))?\)$/);
  if (typeCheck) {
    return assertType(context, {
      name: typeCheck[1],
      cardType: typeCheck[2],
      extra: typeCheck[3],
    });
  }

  const subtypeCheck = source.match(/^assertSubtype\((.+),\s*SubType\.([A-Z0-9_]+)(?:,\s*(.+))?\)$/);
  if (subtypeCheck) {
    return assertSubtype(context, {
      name: subtypeCheck[1],
      subtype: subtypeCheck[2],
      extra: subtypeCheck[3],
    });
  }

  throw new Error(`unsupported Java statement: ${operation.source}`);
}

async function executeScheduled(context) {
  await executeScheduledActions(context);
  if (context.stopAt) {
    await advanceTo(context, context.stopAt.turn, context.stopAt.phase, null);
    const state = context.game.uiState();
    if ((state.stack_objects || []).length > 0) {
      await settleStack(context);
    }
  } else {
    await settleStack(context);
  }
  context.scheduled = [];
}

async function executeScheduledActions(context, until = null, options = {}) {
  const settleAfterCast = options.settleAfterCast !== false;
  context.scheduled.sort(compareScheduled);
  const pending = [];
  for (let index = 0; index < context.scheduled.length; index += 1) {
    const operation = context.scheduled[index];
    if (until && compareScheduled(operation, until) > 0) {
      pending.push(operation);
      continue;
    }
    const scheduledPlayer =
      operation.op === "castSpell" || operation.op === "activateAbility" || operation.op === "attack" || operation.op === "block"
        || operation.op === "playLand"
        ? operation.player
        : null;
    await advanceTo(context, operation.turn, operation.phase, scheduledPlayer);
    if (operation.op === "castSpell") {
      await castSpell(context, operation);
    } else if (operation.op === "activateAbility") {
      await activateAbility(context, operation);
    } else if (operation.op === "attack") {
      await declareAttack(context, operation);
    } else if (operation.op === "block") {
      await declareBlock(context, operation);
    } else if (operation.op === "playLand") {
      await playLand(context, operation);
    }
    await answerPendingDecisions(context);
    const nextOperation = context.scheduled
      .slice(index + 1)
      .find((candidate) => !until || compareScheduled(candidate, until) <= 0);
    const sameScheduledTime =
      nextOperation && compareScheduled(operation, nextOperation) === 0;
    if (
      settleAfterCast &&
      !sameScheduledTime &&
      (operation.op === "castSpell" || operation.op === "activateAbility")
    ) {
      await settleStack(context);
    }
  }
  context.scheduled = pending;
}

async function castSpell(context, operation) {
  const player = playerIndex(operation.player);
  ensurePerspective(context, player);
  let state = context.game.uiState();
  const name = cardName(operation.name);
  if (!isPriorityDecisionFor(state, player)) {
    await advanceToPriorityPlayer(context, player);
    state = context.game.uiState();
  }
  if (process.env.MAGE_PORT_DUMP_STATE) {
    console.error(`[mage-port-state] ${JSON.stringify(state, null, 2).slice(0, 20000)}`);
  }
  const cardId = findCardIdInHand(context, player, name, { optional: true });
  const matchesCast = (candidate) => {
      if (candidate.kind !== "cast_spell" && candidate.action_ref?.kind !== "cast_spell") {
        return false;
      }
      const actionObjectId =
        candidate.object_id ?? candidate.action_ref?.spell_id ?? candidate.action_ref?.object_id;
      if (cardId !== null && Number(actionObjectId) === Number(cardId)) return true;
      return actionLabelMatches(candidate, name);
  };
  let action = (state.decision?.actions || []).find(matchesCast);
  if (!action && stackObjectIds(context.game).length > 0) {
    await settleStack(context);
    state = await advanceTo(context, operation.turn, operation.phase, operation.player);
    if (!isPriorityDecisionFor(state, player)) {
      await advanceToPriorityPlayer(context, player);
      state = context.game.uiState();
    }
    action = (state.decision?.actions || []).find(matchesCast);
  }
  action = action ?? actionByPredicate(
    state,
    matchesCast,
    `cast action for ${name}`,
  );
  state = context.game.dispatch({ type: "priority_action", action_index: action.index });
  await answerPendingDecisions(context, operation.target);
  return state;
}

function ensurePerspective(context, player) {
  const checkpoint = getCheckpoint(context.game);
  if (Number(checkpoint.perspective) === Number(player)) return;
  if (typeof context.game.setPerspective === "function") {
    context.game.setPerspective(playerIndex(player));
    return;
  }
  importCheckpoint(context.game, checkpoint, { perspective: player });
}

async function playLand(context, operation) {
  let state = await advanceTo(context, operation.turn, operation.phase, operation.player);
  const player = playerIndex(operation.player);
  const name = cardName(operation.name);
  if (!isPriorityDecisionFor(state, player)) {
    await advanceToPriorityPlayer(context, player);
    state = context.game.uiState();
  }
  let cardId = findCardIdInHand(context, player, name, { optional: true });
  if (cardId === null) {
    context.game.addCardToHand(player, name);
    cardId = findCardIdInHand(context, player, name);
    state = context.game.uiState();
  }
  const action = actionByPredicate(
    state,
    (candidate) =>
      (candidate.kind === "play_land" || candidate.action_ref?.kind === "play_land" || String(candidate.label || "").startsWith("Play ")) &&
      Number(candidate.object_id ?? candidate.action_ref?.land_id ?? candidate.action_ref?.object_id) === Number(cardId),
    `play land action for ${name}`,
  );
  context.game.dispatch({ type: "priority_action", action_index: action.index });
}

async function activateManaAbility(context, operation) {
  await executeScheduledActions(context, operation, { settleAfterCast: true });
  const count = Number(operation.count || 1);
  for (let index = 0; index < count; index += 1) {
    let state = await advanceTo(context, operation.turn, operation.phase, operation.player);
    const player = playerIndex(operation.player);
    if (!isPriorityDecisionFor(state, player)) {
      await advanceToPriorityPlayer(context, player);
      state = context.game.uiState();
    }
    const action = actionByPredicate(
      state,
      (candidate) =>
        candidate.kind === "activate_mana_ability" &&
        actionLabelMatches(candidate, operation.ability),
      `mana ability ${operation.ability}`,
    );
    context.game.dispatch({ type: "priority_action", action_index: action.index });
    await answerPendingDecisions(context);
  }
}

async function activateAbility(context, operation) {
  let state = context.game.uiState();
  const label = operation.ability || "";
  const sourceName = operation.source || null;
  const player = playerIndex(operation.player);
  if (!isPriorityDecisionFor(state, player)) {
    await advanceToPriorityPlayer(context, player);
    state = context.game.uiState();
  }
  const action = actionByPredicate(
    state,
    (candidate) => {
      const candidateLabel = candidate.label || "";
      const sourceMatches = !sourceName || candidateLabel.includes(sourceName);
      return sourceMatches && actionLabelMatches(candidate, label);
    },
    `activated ability ${label}`,
  );
  state = context.game.dispatch({ type: "priority_action", action_index: action.index });
  await answerPendingDecisions(context, operation.target);
  return state;
}

async function waitStackResolved(context, operation) {
  if (context.scheduled.length > 0) {
    await executeScheduledActions(context, operation, { settleAfterCast: false });
  }
  const player = typeof operation.player === "boolean" ? null : (operation.player ?? null);
  if (!currentPositionIsAfter(context.game.uiState(), operation.turn, operation.phase)) {
    await advanceTo(context, operation.turn, operation.phase, player);
  }
  if (player !== null && player !== undefined) {
    await settleOneStackObject(context);
    return advanceToPriorityPlayer(context, player);
  }
  if (operation.once) return settleOneStackObject(context);
  await settleStack(context);
}

async function settleOneStackObject(context) {
  const initialIds = stackObjectIds(context.game);
  const initial = initialIds.length;
  if (initial === 0) return context.game.uiState();
  for (let step = 0; step < MAX_ADVANCE_STEPS; step += 1) {
    const state = context.game.uiState();
    const currentIds = stackObjectIds(context.game);
    if (
      state.decision?.kind === "priority" &&
      (currentIds.length < initial || currentIds.join(",") !== initialIds.join(","))
    ) {
      return state;
    }
    await passOrAnswer(context, state);
  }
  throw new Error("one stack object did not resolve");
}

function stackObjectIds(game) {
  return (getCheckpoint(game).stack || []).map((entry) => Number(entry.objectId ?? entry.object_id));
}

function clearZone(context, operation) {
  const player = playerIndex(operation.player);
  const zone = zoneName(operation.zone);
  runCode(context.game, (checkpoint) => {
    const playerSnapshot = checkpoint.players.find((candidate) => Number(candidate.id) === player);
    assert(playerSnapshot, `unknown player ${player}`);
    const ids = [...(playerSnapshot[zone] || [])].map(Number);
    playerSnapshot[zone] = [];
    for (const object of checkpoint.objects || []) {
      if (ids.includes(Number(object.id))) {
        object.zone = "outside_game";
      }
    }
  });
}

async function assertPlayableAbility(context, operation) {
  if (context.scheduled.length > 0) {
    await executeScheduledActions(context, operation);
  }
  const expected = Boolean(operation.expected);
  let state = await advanceTo(
    context,
    operation.turn,
    operation.phase,
    expected ? operation.player : null,
  );
  const player = playerIndex(operation.player);
  if (expected && state.priority_player !== player && state.decision?.player !== player) {
    await advanceToPriorityPlayer(context, player);
    state = context.game.uiState();
  }
  const has = (state.decision?.actions || []).some(
    (action) => action.kind !== "activate_mana_ability" && actionLabelMatches(action, operation.label),
  );
  if (process.env.MAGE_PORT_DUMP_CHECKPOINT) {
    const checkpoint = getCheckpoint(context.game);
    const battlefield = getBattlefield(checkpoint, null).map((object) => ({
      id: object.id,
      name: object.name,
      controller: object.controller,
      subtypes: object.subtypes,
      attachments: object.attachments,
      attachedTo: getAttachedTo(checkpoint, object.id),
    }));
    console.error(`[mage-port-battlefield] ${JSON.stringify(battlefield, null, 2)}`);
  }
  assert(
    has === expected,
    `expected playable ability ${operation.label} presence=${expected}, got ${has}`,
    showAvailableAbilities(state),
  );
}

async function assertStackSize(context, operation) {
  if (operation.turn !== undefined && operation.phase !== undefined) {
    await executeScheduledActions(context, operation, { settleAfterCast: false });
    await advanceTo(context, operation.turn, operation.phase, operation.player ?? null);
  } else {
    await prepareAssertion(context, operation);
  }
  const stackSize = getObjectsInZone(getCheckpoint(context.game), "stack").length;
  assert(stackSize === Number(operation.count), `expected stack size ${operation.count}, got ${stackSize}`);
}

async function assertStackObject(context, operation) {
  if (operation.turn !== undefined && operation.phase !== undefined) {
    await executeScheduledActions(context, operation, { settleAfterCast: false });
    await advanceTo(context, operation.turn, operation.phase, operation.player ?? null);
  }
  const state = context.game.uiState();
  const expectedCount = numericValue(operation.count);
  const expectedText = normalizeActionSearch(operation.text);
  const stackObjects = state.stack_objects || state.stackObjects || [];
  const matching = stackObjects.filter((object) => {
    const text = normalizeActionSearch(
      [object.name, object.ability_text, object.abilityText, object.effect_text, object.effectText]
        .filter(Boolean)
        .join(" "),
    );
    if (text.includes(expectedText) || labelFragments(operation.text).every((fragment) => text.includes(fragment))) {
      return true;
    }
    return expectedText.includes("whenever") && String(object.ability_kind ?? object.abilityKind ?? "").toLowerCase() === "triggered";
  });
  assert(
    matching.length === expectedCount,
    `expected ${expectedCount} stack object(s) matching ${operation.text}, got ${matching.length}`,
    stackObjects,
  );
}

async function assertChoicesCount(context, operation) {
  const player = playerIndex(operation.player);
  const expected = numericValue(operation.count);
  const state = context.game.uiState();
  const decision = state.decision;
  let actual = 0;
  if (decision && decision.kind !== "priority" && decision.player === player) {
    if (decision.kind === "targets") {
      actual = (decision.requirements || []).reduce(
        (sum, requirement) => sum + (requirement.legal_targets || []).length,
        0,
      );
    } else if (decision.kind === "select_objects") {
      actual = (decision.candidates || []).length;
    } else if (decision.kind === "boolean") {
      actual = 2;
    } else {
      actual = (decision.options || []).length;
    }
  }
  assert(actual === expected, `expected ${playerName(player)} choices count ${expected}, got ${actual}`, decision);
}

async function declareAttack(context, operation) {
  await advanceTo(context, operation.turn, "DECLARE_ATTACKERS", operation.player);
  let state = context.game.uiState();
  if (state?.decision?.kind !== "attackers") {
    await answerPendingDecisions(context);
    state = context.game.uiState();
  }
  assert(state?.decision?.kind === "attackers", "expected attackers decision", state?.decision);
  const attacker = getPermanent(getCheckpoint(context.game), operation.player, operation.attacker);
  const target = attackTarget(context, operation.defender, operation.player);
  return context.game.dispatch({
    type: "declare_attackers",
    declarations: [{ creature: Number(attacker.id), target }],
  });
}

async function declareBlock(context, operation) {
  await advanceTo(context, operation.turn, "DECLARE_BLOCKERS", operation.player);
  let state = context.game.uiState();
  if (state?.decision?.kind !== "blockers") {
    await answerPendingDecisions(context);
    state = context.game.uiState();
  }
  assert(state?.decision?.kind === "blockers", "expected blockers decision", state?.decision);
  const blocker = getPermanent(getCheckpoint(context.game), operation.player, operation.blocker);
  const attacker = findPermanentAnyController(context, operation.attacker);
  return context.game.dispatch({
    type: "declare_blockers",
    declarations: [{ blocker: Number(blocker.id), attacker: Number(attacker.id) }],
  });
}

async function advanceTo(context, turn, phase, player) {
  const targetTurn = Number(turn || 1);
  const targetPhase = phaseLabel(phase);
  for (let step = 0; step < MAX_ADVANCE_STEPS; step += 1) {
    const state = context.game.uiState();
    if (
      Number(state.turn_number) === targetTurn &&
      normalizePhase(state.phase, state.step) === targetPhase &&
      (player === null || player === undefined || state.priority_player === playerIndex(player) || state.decision?.player === playerIndex(player))
    ) {
      return state;
    }
    if ((player === null || player === undefined) && currentPositionIsAfter(state, targetTurn, targetPhase)) {
      return state;
    }
    if (
      player !== null &&
      player !== undefined &&
      Number(state.turn_number) === targetTurn &&
      currentPositionIsAfter(state, targetTurn, targetPhase) &&
      state.decision?.kind === "priority"
    ) {
      return state;
    }
    await passOrAnswer(context, state);
  }
  const state = context.game.uiState();
  throw new Error(
    `could not advance to turn ${targetTurn} ${targetPhase}; current turn ${state.turn_number} ${normalizePhase(state.phase, state.step)} decision ${state.decision?.kind ?? "none"}`,
  );
}

async function advanceToPriorityPlayer(context, player) {
  const targetPlayer = playerIndex(player);
  for (let step = 0; step < MAX_ADVANCE_STEPS; step += 1) {
    const state = context.game.uiState();
    if (state.decision?.kind === "priority" && state.priority_player === targetPlayer) {
      return state;
    }
    await passOrAnswer(context, state);
  }
  throw new Error(`could not advance to priority player ${targetPlayer}`);
}

async function settleStack(context) {
  for (let step = 0; step < MAX_ADVANCE_STEPS; step += 1) {
    const state = context.game.uiState();
    if ((state.stack_objects || []).length === 0 && state.decision?.kind === "priority") return state;
    await passOrAnswer(context, state);
  }
  throw new Error("stack did not settle");
}

async function passOrAnswer(context, state = context.game.uiState()) {
  if (!state.decision) {
    return context.game.dispatch({ type: "continue" });
  }
  if (state.decision.kind === "priority") {
    const pass = actionByPredicate(
      state,
      (action) =>
        action.kind === "pass_priority" ||
        action.action_ref?.kind === "pass_priority" ||
        ["Keep hand", "Pregame", "Continue", "Begin game"].includes(action.label),
      "pass priority action",
    );
    return context.game.dispatch({ type: "priority_action", action_index: pass.index });
  }
  if (state.decision.kind === "attackers") {
    traceDecision(context, -1, state.decision);
    return context.game.dispatch({
      type: "declare_attackers",
      declarations: autoAttackDeclarations(state.decision),
    });
  }
  if (state.decision.kind === "blockers") {
    traceDecision(context, -1, state.decision);
    return context.game.dispatch({ type: "declare_blockers", declarations: [] });
  }
  return answerPendingDecisions(context);
}

function autoAttackDeclarations(decision) {
  const options = decision.attacker_options || decision.attackerOptions || decision.options || [];
  return options
    .filter((option) => option.must_attack || option.mustAttack)
    .map((option) => {
      const target = (option.valid_targets || option.validTargets || [])[0];
      assert(target, `must-attack creature has no valid attack target`, option);
      return {
        creature: Number(option.creature ?? option.id ?? option.object),
        target: attackDeclarationTarget(target),
      };
    });
}

function attackDeclarationTarget(target) {
  if (typeof target === "string") {
    const player = target.match(/player[:\s]+(\d+)/i);
    if (player) return { kind: "player", player: Number(player[1]) };
  }
  const kind = String(target.kind ?? "").toLowerCase();
  if (kind === "player") return { kind: "player", player: Number(target.player) };
  if (kind === "planeswalker") return { kind: "planeswalker", object: Number(target.object ?? target.id) };
  assert(false, "unknown attack target", target);
}

async function answerPendingDecisions(context, immediateTarget = undefined) {
  for (let safety = 0; safety < 80; safety += 1) {
    const state = context.game.uiState();
    const decision = state.decision;
    if (!decision || decision.kind === "priority" || decision.kind === "attackers" || decision.kind === "blockers") {
      return state;
    }
    traceDecision(context, safety, decision);
    if (decision.kind === "targets") {
      const target = chooseTarget(decision, immediateTarget ?? nextQueuedTarget(context, decision.player));
      if (process.env.MAGE_PORT_DECISION_TRACE) {
        console.error(
          `[mage-port-targets] ${context.sourcePath} :: ${context.testName} ${JSON.stringify(Array.isArray(target) ? target : [target])}`,
        );
      }
      context.game.dispatch({ type: "select_targets", targets: Array.isArray(target) ? target : [target] });
      continue;
    }
    if (decision.kind === "boolean") {
      context.game.dispatch({
        type: "select_options",
        option_indices: [Boolean(nextChoice(context, true)) ? 1 : 0],
      });
      continue;
    }
    if (decision.kind === "modes") {
      const choice = context.modes.shift() ?? context.choices.shift();
      const mode = chooseMode(decision, choice);
      context.game.dispatch({ type: "select_options", option_indices: [mode] });
      continue;
    }
    if (decision.kind === "select_options") {
      const choice = nextCostChoiceForQueuedObject(context, decision) ??
        (isInternalPaymentDecision(decision)
          ? null
          : isModeSelectionDecision(decision)
            ? context.modes.shift() ?? context.choices.shift()
            : nextChoice(context, null));
      const selected = chooseOptions(context, decision, choice);
      if (process.env.MAGE_PORT_DECISION_TRACE) {
        console.error(
          `[mage-port-selection] ${context.sourcePath} :: ${context.testName} ${JSON.stringify(selected.map(optionIndex))}`,
        );
      }
      context.game.dispatch({
        type: "select_options",
        option_indices: selected.map(optionIndex),
      });
      continue;
    }
    if (decision.kind === "select_objects") {
      const target = nextSelectObjectsChoice(context, decision);
      if (context.strict && target === undefined) {
        const current = context.game.uiState();
        throw new Error(
          `Missing CHOICE def for turn ${Number(current.turn_number)}, step ${normalizePhase(current.phase, current.step)}, ${magePlayerName(decision.player)}`,
        );
      }
      const objects = chooseObjectCandidates(decision, target);
      context.game.dispatch({
        type: "select_objects",
        object_ids: objects.map(objectChoiceId),
      });
      continue;
    }
    if (decision.kind === "text_input") {
      context.game.dispatch({
        type: "text_choice",
        value: textChoiceValue(nextChoice(context, decision.value ?? "")),
      });
      continue;
    }
    if (decision.kind === "order") {
      const ids = (decision.objects || decision.options || []).map((object) => object.id ?? object.object_id ?? object.object);
      context.game.dispatch({ type: "order", order: ids });
      continue;
    }
    if (decision.kind === "number") {
      context.game.dispatch({
        type: "number_choice",
        value: numberChoiceValue(nextChoice(context, decision.min ?? 0), decision),
      });
      continue;
    }
    throw new Error(`unsupported decision kind ${decision.kind}: ${JSON.stringify(decision, null, 2)}`);
  }
  throw new Error("decision answering loop did not converge");
}

function nextSelectObjectsChoice(context, decision) {
  const queuedTargetIndex = context.targets.findIndex((entry) => entry.player === playerIndex(decision.player));
  const queuedTarget = queuedTargetIndex >= 0 ? context.targets[queuedTargetIndex].value : undefined;
  const queuedChoice = context.choices[0];
  if (queuedTarget !== undefined && selectObjectsWantedMatches(decision, queuedTarget)) {
    return nextQueuedTarget(context, decision.player);
  }
  if (queuedChoice !== undefined) return context.choices.shift();
  return nextQueuedTarget(context, decision.player);
}

function selectObjectsWantedMatches(decision, wanted) {
  const candidates = (decision.candidates || []).filter((candidate) => candidate.legal !== false);
  return parseCompoundTargetChoice(wanted).some((part) => {
    if (typeof part === "number") {
      return candidates.some((candidate) => objectChoiceId(candidate) === part);
    }
    const text = cardName(part).toLowerCase();
    return candidates.some((candidate) =>
      String(candidate.name ?? candidate.label ?? "").toLowerCase().includes(text),
    );
  });
}

function isManaPaymentDecision(decision) {
  const description = String(decision.description || "");
  return description.startsWith("Pay mana pip") || description.startsWith("Choose how to pay pip");
}

function isInternalPaymentDecision(decision) {
  const description = String(decision.description || "");
  return isManaPaymentDecision(decision) || description.startsWith("Choose the next cost to pay");
}

function nextCostChoiceForQueuedObject(context, decision) {
  const description = String(decision.description || "");
  if (!description.startsWith("Choose the next cost to pay") || context.targets.length === 0) return null;
  return (decision.options || []).find((option) => {
    const label = String(option.description ?? option.label ?? option.name ?? "");
    return option.legal !== false && !label.startsWith("Mana:");
  }) ?? null;
}

function nextQueuedTarget(context, player = null) {
  if (context.targets.length === 0) return undefined;
  const normalizedPlayer = player === null || player === undefined ? null : playerIndex(player);
  const index =
    normalizedPlayer === null
      ? 0
      : context.targets.findIndex((entry) => entry.player === normalizedPlayer);
  const selectedIndex = index >= 0 ? index : 0;
  const [entry] = context.targets.splice(selectedIndex, 1);
  return entry && typeof entry === "object" && Object.hasOwn(entry, "value") ? entry.value : entry;
}

function traceDecision(context, index, decision) {
  if (!process.env.MAGE_PORT_DECISION_TRACE) return;
  const summary = {
    kind: decision.kind,
    player: decision.player,
    description: decision.description ?? decision.context,
    min: decision.min,
    max: decision.max,
    options: (decision.options || []).slice(0, 5).map((option) => ({
      index: option.index,
      description: option.description ?? option.label ?? option.name,
      legal: option.legal,
      repeatable: option.repeatable,
      max_count: option.max_count ?? option.maxCount,
    })),
    requirements: (decision.requirements || []).slice(0, 4).map((requirement) => ({
      label: requirement.label ?? requirement.description,
      legal_targets: (requirement.legal_targets || []).slice(0, 8).map((target) => ({
        kind: target.kind,
        object: target.object ?? target.id,
        player: target.player,
        name: target.name ?? target.object_name ?? target.label,
      })),
    })),
    attackers: (decision.attacker_options || decision.attackerOptions || []).slice(0, 8).map((option) => ({
      creature: option.creature ?? option.id ?? option.object,
      name: option.creature_name ?? option.creatureName ?? option.name,
      must_attack: option.must_attack ?? option.mustAttack,
      valid_targets: option.valid_targets ?? option.validTargets,
    })),
    candidates: (decision.candidates || []).slice(0, 5).map((candidate) => ({
      id: candidate.id ?? candidate.object_id ?? candidate.object,
      name: candidate.name ?? candidate.label,
      legal: candidate.legal,
    })),
  };
  console.error(
    `[mage-port-decision] ${context.sourcePath} :: ${context.testName} #${index + 1} ${JSON.stringify(summary)}`,
  );
}

function chooseTarget(decision, wanted) {
  const requirements = decision.requirements || [];
  if (
    (wanted === undefined || wanted === null) &&
    requirements.length > 0 &&
    requirements.every((requirement) => Number(requirement.min_targets ?? requirement.minTargets ?? 1) === 0)
  ) {
    return [];
  }
  const wantedParts = parseCompoundTargetChoice(wanted);
  if (wantedParts.length > 1 && requirements.length > 1) {
    return requirements.map((requirement, index) =>
      chooseLegalTarget(requirement.legal_targets || [], wantedParts[index] ?? wantedParts[wantedParts.length - 1]),
    );
  }

  const legalTargets = requirements.flatMap((requirement) => requirement.legal_targets || []);
  return chooseLegalTarget(legalTargets, wantedParts[0] ?? wanted);
}

function chooseLegalTarget(legalTargets, wanted) {
  assert(legalTargets.length > 0, "target decision has no legal targets");
  if (wanted === undefined || wanted === null) {
    return legalTargets.find((target) => target.kind === "player" && Number(target.player) === 1) ?? legalTargets[0];
  }
  if (typeof wanted === "string") {
    const lowered = cardName(wanted).toLowerCase();
    const matched = legalTargets.find((target) => {
      if (target.kind === "player") return playerName(Number(target.player)).toLowerCase() === lowered;
      const objectName = target.name || target.object_name || target.label || "";
      return objectName.toLowerCase().includes(lowered);
    });
    if (matched) return matched;
  }
  if (typeof wanted === "number") {
    const matched = legalTargets.find((target) => Number(target.object ?? target.id ?? target.player) === wanted);
    if (matched) return matched;
  }
  return legalTargets[0];
}

function parseCompoundTargetChoice(wanted) {
  if (typeof wanted !== "string") return wanted === undefined || wanted === null ? [] : [wanted];
  return wanted
    .split("^")
    .map((part) => part.replace(/^mode=\d+/i, "").trim())
    .filter(Boolean);
}

function chooseOption(context, decision, wanted) {
  const options = decision.options || [];
  assert(options.length > 0, "select_options decision has no options", decision);
  const legalOptions = options.filter((option) => option.legal !== false);
  if (wanted && typeof wanted === "object") {
    const wantedIndex = optionIndex(wanted);
    return options.find((option) => optionIndex(option) === wantedIndex) ?? wanted;
  }
  if (wanted === null || wanted === undefined) {
    if (String(decision.description || "").startsWith("Choose how to pay pip")) {
      const availableColors = availableManaColors(context);
      const colorOption = legalOptions.find((option) =>
        manaSymbolsInOption(option).some((symbol) => availableColors.has(symbol)),
      );
      if (colorOption) return colorOption;
    }
    return legalOptions[0] ?? options[0];
  }
  if (typeof wanted === "boolean") {
    const booleanPattern = wanted ? /^(yes|true)$/i : /^(no|false)$/i;
    return options.find((option) =>
      booleanPattern.test(String(option.label ?? option.text ?? option.name ?? option.description ?? "").trim()),
    ) ?? legalOptions[0] ?? options[0];
  }
  const text = String(wanted).toLowerCase();
  if (isModeSelectionDecision(decision) && /^\d+$/.test(text.trim())) {
    return legalOptions[Number(text.trim()) - 1] ?? legalOptions[0] ?? options[0];
  }
  return options.find((option) =>
    String(option.label ?? option.text ?? option.name ?? option.description ?? "").toLowerCase().includes(text),
  ) ?? legalOptions[0] ?? options[0];
}

function chooseOptions(context, decision, wanted) {
  const options = decision.options || [];
  assert(options.length > 0, "select_options decision has no options", decision);
  const legalOptions = options.filter((option) => option.legal !== false);
  const max = Number.isFinite(Number(decision.max)) ? Number(decision.max) : legalOptions.length;
  const min = Number(decision.min ?? 1);
  if (wanted === false && min === 0) return [];
  const desiredCount = isDistributionDecision(decision)
    ? max
    : isModeSelectionDecision(decision) && wanted !== null && wanted !== undefined
      ? Math.min(Math.max(min, 1 + context.modes.length), max)
      : Math.min(Math.max(1, min), max);
  const selected = [];
  const counts = new Map();

  if (wanted !== null && wanted !== undefined && isModeSelectionDecision(decision)) {
    addOptionSelection(selected, counts, chooseOption(context, decision, wanted));
    while (selected.length < desiredCount && context.modes.length > 0) {
      addOptionSelection(selected, counts, chooseOption(context, decision, context.modes.shift()));
    }
    return selected;
  }

  if (wanted !== null && wanted !== undefined && isTriggeredAbilityOrderDecision(decision)) {
    const bottomChoices = [chooseOption(context, decision, wanted)];
    while (bottomChoices.length < desiredCount - 1 && context.choices.length > 0) {
      const next = context.choices[0];
      const option = findMatchingOption(decision, next);
      if (!option) break;
      context.choices.shift();
      bottomChoices.push(option);
    }
    for (const option of bottomChoices) {
      addOptionSelection(selected, counts, option);
    }
    const bottomIndices = new Set(bottomChoices.map(optionIndex));
    for (const option of legalOptions) {
      if (selected.length >= desiredCount || bottomIndices.has(optionIndex(option))) continue;
      while (selected.length < desiredCount && canSelectOption(option, counts)) {
        addOptionSelection(selected, counts, option);
      }
    }
    return selected;
  }

  if (wanted !== null && wanted !== undefined) {
    addOptionSelection(selected, counts, chooseOption(context, decision, wanted));
  } else if (String(decision.description || "").startsWith("Choose how to pay pip")) {
    addOptionSelection(selected, counts, chooseOption(context, decision, wanted));
  }

  for (const option of legalOptions) {
    while (selected.length < desiredCount && canSelectOption(option, counts)) {
      addOptionSelection(selected, counts, option);
    }
    if (selected.length >= desiredCount) break;
  }

  if (selected.length === 0 && legalOptions[0]) selected.push(legalOptions[0]);
  return selected;
}

function isTriggeredAbilityOrderDecision(decision) {
  return String(decision.description || "").toLowerCase().startsWith("order triggered abilities");
}

function isDistributionDecision(decision) {
  return String(decision.description || "").toLowerCase().startsWith("distribute ");
}

function isModeSelectionDecision(decision) {
  return String(decision.description || "").toLowerCase().startsWith("choose mode");
}

function findMatchingOption(decision, wanted) {
  const text = String(wanted).toLowerCase();
  return (decision.options || []).find((option) =>
    String(option.label ?? option.text ?? option.name ?? option.description ?? "").toLowerCase().includes(text),
  );
}

function addOptionSelection(selected, counts, option) {
  if (!option || !canSelectOption(option, counts)) return false;
  selected.push(option);
  const index = optionIndex(option);
  counts.set(index, (counts.get(index) || 0) + 1);
  return true;
}

function canSelectOption(option, counts) {
  const index = optionIndex(option);
  const current = counts.get(index) || 0;
  const limit = option.repeatable ? Number(option.max_count ?? option.maxCount ?? Infinity) : 1;
  return current < limit;
}

function optionIndex(option) {
  return Number(option.index ?? option.id ?? 0);
}

function manaSymbolsInOption(option) {
  return [...String(option.description ?? option.label ?? option.text ?? "").matchAll(/\{([WUBRG])\}/g)]
    .map((match) => match[1]);
}

function availableManaColors(context) {
  const colors = new Set();
  for (const permanent of getBattlefield(getCheckpoint(context.game))) {
    if (permanent.tapped) continue;
    const name = String(permanent.name || "");
    if (name === "Plains") colors.add("W");
    if (name === "Island") colors.add("U");
    if (name === "Swamp") colors.add("B");
    if (name === "Mountain") colors.add("R");
    if (name === "Forest") colors.add("G");
  }
  return colors;
}

function chooseMode(decision, wanted) {
  const modes = decision.modes || decision.options || [];
  assert(modes.length > 0, "mode decision has no modes", decision);
  if (wanted === null || wanted === undefined) return modes[0].index ?? modes[0].id ?? 0;
  const text = String(wanted).toLowerCase();
  const mode = modes.find((candidate) => String(candidate.label ?? candidate.text ?? "").toLowerCase().includes(text)) ?? modes[0];
  return mode.index ?? mode.id ?? 0;
}

function chooseObjectCandidate(decision, wanted) {
  const candidates = (decision.candidates || []).filter((candidate) => candidate.legal !== false);
  assert(candidates.length > 0, "select_objects decision has no legal candidates", decision);
  if (wanted === null || wanted === undefined) return candidates[0];
  const text = cardName(wanted).toLowerCase();
  return candidates.find((candidate) => String(candidate.name ?? candidate.label ?? "").toLowerCase().includes(text)) ?? candidates[0];
}

function chooseObjectCandidates(decision, wanted) {
  const candidates = (decision.candidates || []).filter((candidate) => candidate.legal !== false);
  assert(candidates.length > 0, "select_objects decision has no legal candidates", decision);
  const max = decision.max === null || decision.max === undefined ? candidates.length : Number(decision.max);
  const wantedParts = parseCompoundTargetChoice(wanted);
  const desiredCount = Math.min(Math.max(1, Number(decision.min ?? 1), wantedParts.length), max);
  const selected = [];
  const seen = new Set();

  for (const part of wantedParts) {
    if (selected.length >= desiredCount) break;
    const candidate = chooseObjectCandidate(
      { ...decision, candidates: candidates.filter((candidate) => !seen.has(objectChoiceId(candidate))) },
      part,
    );
    selected.push(candidate);
    seen.add(objectChoiceId(candidate));
  }

  for (const candidate of candidates) {
    if (selected.length >= desiredCount) break;
    const id = objectChoiceId(candidate);
    if (seen.has(id)) continue;
    selected.push(candidate);
    seen.add(id);
  }

  return selected;
}

function objectChoiceId(object) {
  return Number(object.id ?? object.object ?? object.object_id);
}

function nextChoice(context, fallback) {
  return context.choices.length ? context.choices.shift() : fallback;
}

function numberChoiceValue(raw, decision) {
  if (raw === null || raw === undefined) return Number(decision.min ?? 0);
  if (typeof raw === "number") return raw;
  const text = String(raw).trim();
  const assignment = text.match(/(?:^|\b)(?:x|value|amount)\s*=\s*(-?\d+)/i);
  if (assignment) return Number(assignment[1]);
  const integer = text.match(/-?\d+/);
  if (integer) return Number(integer[0]);
  if (/skip|none|no/i.test(text)) return Number(decision.min ?? 0);
  const fallback = Number(decision.min ?? 0);
  assert(Number.isFinite(fallback), `numeric decision has no finite fallback for choice ${JSON.stringify(raw)}`, decision);
  return fallback;
}

function textChoiceValue(raw) {
  const text = String(raw ?? "").trim();
  if (!text || /skip|none|no/i.test(text)) return "Island";
  return cardName(text);
}

function numericValue(raw) {
  if (typeof raw === "number") return raw;
  const text = String(raw ?? "").trim();
  if (/^-?\d+(?:\.\d+)?$/.test(text)) return Number(text);
  if (/^[\d\s+\-*/().]+$/.test(text)) {
    // Ported Mage fixtures sometimes preserve simple Java arithmetic expressions.
    return Function(`"use strict"; return (${text});`)();
  }
  return Number(text);
}

function resolveMageVariable(context, raw) {
  const text = String(raw ?? "").trim();
  if (Object.hasOwn(context.javaVariables || {}, text)) return context.javaVariables[text];
  return text;
}

function readJavaNumericVariables(sourcePath) {
  try {
    const source = readFileSync(sourcePath, "utf8");
    const variables = {};
    for (const match of source.matchAll(/\b(?:int|Integer|long|Long)\s+([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(-?\d+)\s*;/g)) {
      variables[match[1]] = Number(match[2]);
    }
    return variables;
  } catch {
    return {};
  }
}

function findCardIdInHand(context, player, name, options = {}) {
  const normalized = cardName(name);
  const match = getHand(getCheckpoint(context.game), player).find((card) => card.name === normalized);
  if (!match && options.optional) return null;
  assert(match, `card not found in hand: ${normalized}`, names(getHand(getCheckpoint(context.game), player)));
  return Number(match.id);
}

function findPermanentAnyController(context, name) {
  const normalized = cardName(name);
  for (const player of [0, 1]) {
    const found = getPermanent(getCheckpoint(context.game), player, normalized, { optional: true });
    if (found) return found;
  }
  throw new Error(`permanent not found under any controller: ${normalized}`);
}

async function prepareAssertion(context, operation) {
  if (operation.turn !== undefined && operation.phase !== undefined) {
    await executeScheduledActions(context, operation);
    await advanceTo(context, operation.turn, operation.phase, operation.player ?? null);
  }
}

async function assertLife(context, operation) {
  await prepareAssertion(context, operation);
  const checkpoint = getCheckpoint(context.game);
  if (process.env.MAGE_PORT_DUMP_CHECKPOINT) {
    console.error(`[mage-port-checkpoint] ${JSON.stringify(checkpoint, null, 2).slice(0, 20000)}`);
  }
  const player = checkpoint.players[playerIndex(operation.player)];
  const expected = numericValue(operation.life);
  assert(player.life === expected, `expected life ${operation.life} for ${operation.player}, got ${player.life}`);
}

async function assertPermanentCount(context, operation) {
  await prepareAssertion(context, operation);
  const checkpoint = getCheckpoint(context.game);
  if (process.env.MAGE_PORT_DUMP_CHECKPOINT) {
    console.error(`[mage-port-checkpoint] ${JSON.stringify(checkpoint, null, 2).slice(0, 20000)}`);
  }
  const permanents = getBattlefield(checkpoint, operation.player);
  const name = operation.name === undefined ? null : cardName(operation.name);
  const actual = name === null ? permanents.length : countByMagePermanentName(permanents, name);
  const label = name === null ? "total" : name;
  assert(actual === numericValue(operation.count), `expected ${operation.count} ${label} permanents, got ${actual}`);
}

async function assertZoneCount(context, operation, zone) {
  await prepareAssertion(context, operation);
  const checkpoint = getCheckpoint(context.game);
  const cards =
    zone === "hand"
      ? getHand(checkpoint, operation.player)
      : zone === "graveyard"
        ? getGraveyard(checkpoint, operation.player)
        : getLibrary(checkpoint, operation.player);
  const name = operation.name ? cardName(operation.name) : null;
  const actual = name ? countByName(cards, name) : cards.length;
  assert(actual === numericValue(operation.count), `expected ${operation.count} ${name || zone} cards in ${zone}, got ${actual}`);
}

async function assertExileCount(context, operation) {
  await prepareAssertion(context, operation);
  const checkpoint = getCheckpoint(context.game);
  if (process.env.MAGE_PORT_DUMP_CHECKPOINT) {
    console.error(`[mage-port-checkpoint] ${JSON.stringify(checkpoint, null, 2).slice(0, 20000)}`);
  }
  const cards = getExile(checkpoint, operation.player ?? null);
  const name = operation.name ? cardName(operation.name) : null;
  const actual = name ? countByName(cards, name) : cards.length;
  assert(actual === numericValue(operation.count), `expected ${operation.count} ${name || "exile"} cards in exile, got ${actual}`);
}

async function assertPowerToughness(context, operation) {
  await prepareAssertion(context, operation);
  const name = cardName(operation.name);
  const checkpoint = getCheckpoint(context.game);
  if (process.env.MAGE_PORT_DUMP_CHECKPOINT) {
    console.error(`[mage-port-checkpoint] ${JSON.stringify(checkpoint, null, 2).slice(0, 20000)}`);
  }
  const expectedPower = numericValue(operation.power);
  const expectedToughness = numericValue(operation.toughness);
  const object = findPermanentForMageArg(context, operation.player, name, {
    predicate: (candidate) => {
      const details = getObjectDetails(context.game, candidate.id);
      return (details.power ?? 0) === expectedPower && (details.toughness ?? 0) === expectedToughness;
    },
  });
  const details = getObjectDetails(context.game, object.id);
  const actualPower = details.power ?? 0;
  const actualToughness = details.toughness ?? 0;
  assert(
    actualPower === expectedPower && actualToughness === expectedToughness,
    `expected ${name} ${operation.power}/${operation.toughness}, got ${actualPower}/${actualToughness}`,
  );
}

async function assertTappedCount(context, operation) {
  await prepareAssertion(context, operation);
  const name = cardName(operation.name);
  const actual = getBattlefield(getCheckpoint(context.game), operation.player ?? null).filter(
    (object) => object.name === name && Boolean(object.tapped) === Boolean(operation.tapped),
  ).length;
  assert(actual === numericValue(operation.count), `expected ${operation.count} ${name} tapped=${operation.tapped}, got ${actual}`);
}

async function assertTapped(context, operation) {
  await prepareAssertion(context, operation);
  const object = findPermanentForMageArg(context, operation.player, operation.name);
  const actual = Boolean(object.tapped);
  assert(
    actual === Boolean(operation.tapped),
    `expected ${object.name} tapped=${operation.tapped}, got ${actual}`,
    object,
  );
}

async function assertDamageReceived(context, operation) {
  await prepareAssertion(context, operation);
  const object = findPermanentForMageArg(context, operation.player, operation.name);
  const actual = Number(object.damageMarked ?? object.damage_marked ?? 0);
  const expected = numericValue(operation.damage);
  assert(actual === expected, `expected ${object.name} damage ${expected}, got ${actual}`, object);
}

async function assertAttachedTo(context, operation) {
  await prepareAssertion(context, operation);
  const checkpoint = getCheckpoint(context.game);
  const attachment = findPermanentForMageArg(context, operation.player, operation.attachment);
  const target = findPermanentForMageArg(context, operation.player, operation.target);
  const attachedTo = getAttachedTo(checkpoint, attachment.id);
  const actual = attachedTo && Number(attachedTo.id) === Number(target.id);
  assert(
    Boolean(actual) === Boolean(operation.expected),
    `expected ${attachment.name} attached to ${target.name} presence=${operation.expected}, got ${Boolean(actual)}`,
    { attachment, target, attachedTo },
  );
}

async function assertCounterCount(context, operation) {
  await prepareAssertion(context, operation);
  if (typeof operation.name === "number") {
    const checkpoint = getCheckpoint(context.game);
    const player = checkpoint.players.find((candidate) => Number(candidate.id) === playerIndex(operation.player));
    assert(player, `unknown player ${operation.player}`);
    const counter = String(operation.counter || "").toLowerCase();
    const actual =
      counter.includes("energy")
        ? Number(player.energyCounters || 0)
        : counter.includes("poison")
          ? Number(player.poisonCounters || 0)
          : counter.includes("experience")
            ? Number(player.experienceCounters || 0)
            : 0;
    assert(actual === numericValue(operation.count), `expected ${operation.count} ${operation.counter} counters on player ${operation.player}, got ${actual}`);
    return;
  }
  const object = findPermanentForMageArg(context, operation.player, operation.name);
  const name = object.name;
  const expectedCounter = normalizeMageCounterKind(operation.counter);
  const counter = (object.counters || []).find((candidate) =>
    normalizeMageCounterKind(candidate.kind) === expectedCounter,
  );
  const actual = counter?.amount ?? 0;
  assert(actual === numericValue(operation.count), `expected ${operation.count} ${operation.counter} counters on ${name}, got ${actual}`);
}

function normalizeMageCounterKind(counter) {
  const normalized = String(counter || "")
    .trim()
    .toLowerCase()
    .replace(/\s+/g, "");
  if (normalized === "p1p1" || normalized === "+1/+1" || normalized === "+1+1") {
    return "+1/+1";
  }
  if (normalized === "m1m1" || normalized === "-1/-1" || normalized === "-1-1") {
    return "-1/-1";
  }
  return normalized;
}

async function assertType(context, operation) {
  await prepareAssertion(context, operation);
  if (process.env.MAGE_PORT_DUMP_CHECKPOINT) {
    const checkpoint = getCheckpoint(context.game);
    const battlefield = getBattlefield(checkpoint, null).map((object) => ({
      id: object.id,
      name: object.name,
      attachments: object.attachments,
      attachedTo: getAttachedTo(checkpoint, object.id),
    }));
    console.error(`[mage-port-battlefield] ${JSON.stringify(battlefield, null, 2)}`);
  }
  const object = findPermanentForMageArg(context, null, operation.name);
  const details = getObjectDetails(context.game, object.id);
  const typeLine = String(details.type_line ?? details.typeLine ?? "");
  const expectedType = titleCaseType(operation.cardType);
  const extra = String(operation.extra ?? "").trim();
  const expected =
    extra === "" || extra === "null"
      ? true
      : extra === "true"
        ? true
        : extra === "false"
          ? false
          : true;
  const hasType = typeLine.split(/\s+|—|-/).some((part) => part === expectedType);
  let has = hasType;
  const subtype = extra.match(/^SubType\.([A-Z0-9_]+)$/);
  if (subtype) has = hasType && typeLine.includes(titleCaseType(subtype[1]));
  assert(has === expected, `expected type ${expectedType} presence=${expected} on ${object.name}`, {
    typeLine,
    object,
  });
}

async function assertSubtype(context, operation) {
  await prepareAssertion(context, operation);
  const object = findPermanentForMageArg(context, null, operation.name);
  const details = getObjectDetails(context.game, object.id);
  const typeLine = String(details.type_line ?? details.typeLine ?? "");
  const expectedSubtype = titleCaseType(operation.subtype);
  const extra = String(operation.extra ?? "").trim();
  const expected =
    extra === "" || extra === "null"
      ? true
      : extra === "true"
        ? true
        : extra === "false"
          ? false
          : true;
  const has = typeLine.split(/\s+|—|-/).some((part) => part === expectedSubtype);
  assert(has === expected, `expected subtype ${expectedSubtype} presence=${expected} on ${object.name}`, {
    typeLine,
    object,
  });
}

async function assertAbility(context, operation) {
  await prepareAssertion(context, operation);
  const name = cardName(operation.name);
  const checkpoint = getCheckpoint(context.game);
  if (process.env.MAGE_PORT_DUMP_CHECKPOINT) {
    console.error(`[mage-port-checkpoint] ${JSON.stringify(checkpoint, null, 2).slice(0, 20000)}`);
  }
  const object = getPermanent(checkpoint, operation.player, name);
  const details = getObjectDetails(context.game, object.id);
  const expectedAbility = normalizeMageAbilityText(operation.ability);
  const has = (details.abilities || []).some((ability) => ability.includes(expectedAbility));
  assert(has === Boolean(operation.expected), `expected ability ${operation.ability} presence=${operation.expected} on ${name}`, details.abilities);
}

function normalizeMageAbilityText(raw) {
  const text = String(raw ?? "");
  const constructor = text.match(/^new\s+([A-Za-z0-9_]+)Ability\s*\(\s*\)$/);
  if (constructor) return titleCaseType(constructor[1]);
  return text;
}

async function assertAbilities(context, operation) {
  for (const ability of operation.abilities || []) {
    await assertAbility(context, {
      ...operation,
      ability,
      expected: true,
    });
  }
}

function findPermanentForMageArg(context, player, raw, { predicate = null } = {}) {
  const checkpoint = getCheckpoint(context.game);
  const parsed = mageArgName(raw);
  if (parsed.quoted) {
    return getPermanent(checkpoint, player, parsed.name);
  }
  const candidates = getBattlefield(checkpoint, player ?? null);
  const normalized = normalizeLooseName(parsed.name);
  const candidateGroups = [
    candidates.filter((object) => normalizeLooseName(object.name) === normalized),
    candidates.filter((object) => normalizeLooseName(object.name).includes(normalized)),
    candidates.filter((object) => normalized.includes(initialsForName(object.name))),
    candidates.filter((object) => normalizeLooseName(object.name).includes(parsed.name.toLowerCase())),
  ];
  if (predicate) {
    const predicateMatch = candidateGroups.flat().find(predicate);
    if (predicateMatch) return predicateMatch;
  }
  const match =
    candidateGroups[0][0] ??
    candidateGroups[1][0] ??
    candidateGroups[2][0] ??
    candidateGroups[3][0];
  assert(match, `permanent not found: ${parsed.name}`, candidates.map((object) => object.name));
  return match;
}

function mageArgName(raw) {
  const text = String(raw ?? "").trim();
  const quoted = text.match(/^"([^"]+)"$/);
  if (quoted) return { name: cardName(quoted[1]), quoted: true };
  return { name: text, quoted: false };
}

function normalizeLooseName(raw) {
  return String(raw ?? "")
    .replace(/\s+token$/i, "")
    .toLowerCase()
    .replace(/[^a-z0-9]/g, "");
}

function initialsForName(raw) {
  return String(raw ?? "")
    .split(/[^A-Za-z0-9]+/)
    .filter(Boolean)
    .map((part) => part[0].toLowerCase())
    .join("");
}

function countByMagePermanentName(objects, name) {
  const normalized = normalizeLooseName(name);
  return objects.filter((object) => {
    const objectName = normalizeLooseName(object.name);
    if (objectName === normalized) return true;
    const tokenName = normalized.replace(/token$/, "");
    return object.token === true && tokenName.startsWith(objectName);
  }).length;
}

function titleCaseType(raw) {
  return String(raw ?? "")
    .toLowerCase()
    .split("_")
    .filter(Boolean)
    .map((part) => part[0].toUpperCase() + part.slice(1))
    .join(" ");
}

function compareScheduled(left, right) {
  return (
    Number(left.turn || 1) - Number(right.turn || 1) ||
    phaseOrder(left.phase) - phaseOrder(right.phase)
  );
}

function isPriorityDecisionFor(state, player) {
  return state?.decision?.kind === "priority" && state.priority_player === player;
}

function playerIndex(player) {
  return normalizePlayerId(player);
}

function playerName(index) {
  return PLAYER_NAMES[index] ?? `Player ${index}`;
}

function cardName(raw) {
  return String(raw ?? "")
    .split("@", 1)[0]
    .replace(/\s+(?:using|with)\s+.+$/i, "")
    .replace(/^Cast\s+/i, "");
}

function zoneName(zone) {
  const raw = String(zone).replace(/^Zone\./, "").toLowerCase();
  if (raw === "outside_game") return "outside_game";
  if (raw === "battlefield") return "battlefield";
  if (raw === "graveyard") return "graveyard";
  if (raw === "library") return "library";
  if (raw === "exile") return "exile";
  if (raw === "command") return "command";
  return "hand";
}

function phaseLabel(phase) {
  return String(phase || "PRECOMBAT_MAIN").replace(/^PhaseStep\./, "");
}

function magePlayerName(player) {
  const index = playerIndex(player);
  return `Player${String.fromCharCode("A".charCodeAt(0) + index)}`;
}

function normalizePhase(phase, step) {
  const combined = `${phase || ""} ${step || ""}`.toLowerCase();
  if (combined.includes("upkeep")) return "UPKEEP";
  if (combined.includes("draw")) return "DRAW";
  if (combined.includes("first main") || combined.includes("precombat")) return "PRECOMBAT_MAIN";
  if (combined.includes("begin combat")) return "BEGIN_COMBAT";
  if (combined.includes("declare attackers")) return "DECLARE_ATTACKERS";
  if (combined.includes("declare blockers")) return "DECLARE_BLOCKERS";
  if (combined.includes("combat damage")) return "COMBAT_DAMAGE";
  if (combined.includes("end combat")) return "END_COMBAT";
  if (combined.includes("second main") || combined.includes("postcombat")) return "POSTCOMBAT_MAIN";
  if (combined.includes("end step")) return "END_TURN";
  if (combined.includes("cleanup")) return "CLEANUP";
  return combined.toUpperCase();
}

function phaseOrder(phase) {
  return [
    "UNTAP",
    "UPKEEP",
    "DRAW",
    "PRECOMBAT_MAIN",
    "BEGIN_COMBAT",
    "DECLARE_ATTACKERS",
    "DECLARE_BLOCKERS",
    "COMBAT_DAMAGE",
    "END_COMBAT",
    "POSTCOMBAT_MAIN",
    "END_TURN",
    "CLEANUP",
  ].indexOf(phaseLabel(phase));
}

function currentPositionIsAfter(state, turn, phase) {
  const currentTurn = Number(state.turn_number || 1);
  const targetTurn = Number(turn || 1);
  if (currentTurn !== targetTurn) return currentTurn > targetTurn;
  return phaseOrder(normalizePhase(state.phase, state.step)) > phaseOrder(phase);
}

function attackTarget(context, defender, attacker = 0) {
  const attackerPlayer = playerIndex(attacker);
  if (typeof defender === "string" && !/^(alice|bob|player\s*[ab]|\d+)$/i.test(defender.trim())) {
    const normalizedDefender = cardName(defender);
    const permanent = (getCheckpoint(context.game).objects || []).find((object) =>
      object.zone === "battlefield" && cardName(object.name) === normalizedDefender
    );
    assert(permanent, `expected attack defender permanent ${defender}`);
    return { kind: "planeswalker", object: Number(permanent.id) };
  }
  let targetPlayer = playerIndex(defender ?? (attackerPlayer === 0 ? 1 : 0));
  if (targetPlayer === attackerPlayer) {
    targetPlayer = attackerPlayer === 0 ? 1 : 0;
  }
  return { kind: "player", player: targetPlayer };
}

function labelFragments(label) {
  return normalizeActionSearch(label)
    .split(/\s+/)
    .map((fragment) => fragment.trim())
    .filter((fragment) => fragment.length > 2)
    .slice(0, 4);
}

function actionLabelMatches(action, wanted) {
  const label = String(action.label || "");
  const raw = String(wanted || "").trim();
  if (!raw) return false;
  const lowered = normalizeActionSearch(label);
  const wantedLowered = normalizeActionSearch(raw);
  if (/^[a-z0-9]+$/i.test(wantedLowered)) {
    return new RegExp(`(^|\\s)${escapeRegExp(wantedLowered)}(?=\\s|$|:)`).test(lowered);
  }
  if (lowered.includes(wantedLowered)) return true;

  if (/^(cast|play)\s+/i.test(raw)) {
    const name = normalizeActionSearch(cardName(raw));
    return lowered.includes(name);
  }

  const fragments = labelFragments(raw);
  return fragments.length > 0 && fragments.every((fragment) => lowered.includes(fragment));
}

function escapeRegExp(raw) {
  return String(raw).replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function normalizeActionSearch(raw) {
  return String(raw || "")
    .toLowerCase()
    .replace(/\{this\}/g, "this creature")
    .replace(/^\s*\+(\d+)\s*:/, "put $1 loyalty counters")
    .replace(/^\s*-(\d+|x)\s*:/, "remove $1 loyalty counters")
    .replace(/[{}]/g, "")
    .replace(/[^\p{L}\p{N}+\-/ ]/gu, " ")
    .replace(/\s+/g, " ")
    .trim();
}
