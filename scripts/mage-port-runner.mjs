import test from "node:test";
import {
  actionByPredicate,
  addCustomCardWithAbility,
  assert,
  countByName,
  getBattlefield,
  getCheckpoint,
  getExile,
  getGraveyard,
  getHand,
  getLibrary,
  getObjectsInZone,
  getObjectDetails,
  getPermanent,
  initWasmGame,
  names,
  normalizePlayerId,
  runCode,
  showAvailableAbilities,
  startEmptyMatch,
} from "./wasm-test-harness.mjs";

const PLAYER_NAMES = ["Alice", "Bob"];
const MAX_ADVANCE_STEPS = 600;

export function registerPortedMageTests(fileSpec) {
  for (const testSpec of fileSpec.tests || []) {
    test(`${fileSpec.sourcePath} :: ${testSpec.name}`, async () => {
      const context = await createMagePortContext(fileSpec, testSpec);
      await runOperations(context, testSpec.operations || []);
    });
  }
}

async function createMagePortContext(fileSpec, testSpec) {
  const { game } = await initWasmGame({ pkg: fileSpec.pkg || "root" });
  startEmptyMatch(game, {
    playerNames: PLAYER_NAMES,
    startingLife: 20,
    seed: testSpec.seed || 1,
    openingHandSize: 0,
    decks: [[], []],
  });
  return {
    game,
    scheduled: [],
    choices: [],
    targets: [],
    modes: [],
    stopAt: null,
    strict: false,
    sourcePath: fileSpec.sourcePath,
    testName: testSpec.name,
  };
}

async function runOperations(context, operations) {
  for (const operation of operations) {
    await applyOperation(context, operation);
  }
}

async function applyOperation(context, operation) {
  switch (operation.op) {
    case "addCard":
      return addCard(context, operation);
    case "setLife":
      return context.game.setLife(playerIndex(operation.player), Number(operation.life));
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
      context.targets.push(operation.target);
      return;
    case "castSpell":
    case "activateAbility":
    case "attack":
    case "block":
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
    case "playLand":
      return playLand(context, operation);
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
    case "unsupported":
      throw new Error(`unsupported Java statement: ${operation.source}`);
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

async function executeScheduled(context) {
  context.scheduled.sort(compareScheduled);
  for (const operation of context.scheduled) {
    await advanceTo(context, operation.turn, operation.phase, operation.player);
    if (operation.op === "castSpell") {
      await castSpell(context, operation);
    } else if (operation.op === "activateAbility") {
      await activateAbility(context, operation);
    } else if (operation.op === "attack") {
      await declareAttack(context, operation);
    } else if (operation.op === "block") {
      await declareBlock(context, operation);
    }
    await answerPendingDecisions(context);
  }
  if (context.stopAt) {
    await advanceTo(context, context.stopAt.turn, context.stopAt.phase, null);
  } else {
    await settleStack(context);
  }
  context.scheduled = [];
}

async function castSpell(context, operation) {
  let state = context.game.uiState();
  const player = playerIndex(operation.player);
  const name = cardName(operation.name);
  if (state.priority_player !== player) {
    await advanceToPriorityPlayer(context, player);
    state = context.game.uiState();
  }
  const cardId = findCardIdInHand(context, player, name, { optional: true });
  const action = actionByPredicate(
    state,
    (candidate) => {
      if (candidate.kind !== "cast_spell" && candidate.action_ref?.kind !== "cast_spell") {
        return false;
      }
      const actionObjectId =
        candidate.object_id ?? candidate.action_ref?.spell_id ?? candidate.action_ref?.object_id;
      if (cardId !== null && Number(actionObjectId) === Number(cardId)) return true;
      return actionLabelMatches(candidate, name);
    },
    `cast action for ${name}`,
  );
  state = context.game.dispatch({ type: "priority_action", action_index: action.index });
  await answerPendingDecisions(context, operation.target);
  return state;
}

async function playLand(context, operation) {
  let state = await advanceTo(context, operation.turn, operation.phase, operation.player);
  const player = playerIndex(operation.player);
  const name = cardName(operation.name);
  if (state.priority_player !== player) {
    await advanceToPriorityPlayer(context, player);
    state = context.game.uiState();
  }
  const cardId = findCardIdInHand(context, player, name);
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
  const count = Number(operation.count || 1);
  for (let index = 0; index < count; index += 1) {
    let state = await advanceTo(context, operation.turn, operation.phase, operation.player);
    const player = playerIndex(operation.player);
    if (state.priority_player !== player && state.decision?.player !== player) {
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
  const player = typeof operation.player === "boolean" ? null : (operation.player ?? null);
  await advanceTo(context, operation.turn, operation.phase, player);
  if (operation.once) return settleOneStackObject(context);
  await settleStack(context);
}

async function settleOneStackObject(context) {
  const initial = (context.game.uiState().stack_objects || []).length;
  if (initial === 0) return context.game.uiState();
  for (let step = 0; step < MAX_ADVANCE_STEPS; step += 1) {
    const state = context.game.uiState();
    if ((state.stack_objects || []).length < initial && state.decision?.kind === "priority") return state;
    await passOrAnswer(context, state);
  }
  throw new Error("one stack object did not resolve");
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
  let state = await advanceTo(context, operation.turn, operation.phase, operation.player);
  const player = playerIndex(operation.player);
  if (state.priority_player !== player && state.decision?.player !== player) {
    await advanceToPriorityPlayer(context, player);
    state = context.game.uiState();
  }
  const expected = Boolean(operation.expected);
  const has = (state.decision?.actions || []).some((action) => actionLabelMatches(action, operation.label));
  assert(
    has === expected,
    `expected playable ability ${operation.label} presence=${expected}, got ${has}`,
    showAvailableAbilities(state),
  );
}

async function assertStackSize(context, operation) {
  await prepareAssertion(context, operation);
  const stackSize = getObjectsInZone(getCheckpoint(context.game), "stack").length;
  assert(stackSize === Number(operation.count), `expected stack size ${operation.count}, got ${stackSize}`);
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
  const target = attackTarget(operation.defender);
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
    await passOrAnswer(context, state);
  }
  throw new Error(`could not advance to turn ${targetTurn} ${targetPhase}`);
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
    return context.game.dispatch({ type: "declare_attackers", declarations: [] });
  }
  if (state.decision.kind === "blockers") {
    return context.game.dispatch({ type: "declare_blockers", declarations: [] });
  }
  return answerPendingDecisions(context);
}

async function answerPendingDecisions(context, immediateTarget = undefined) {
  for (let safety = 0; safety < 80; safety += 1) {
    const state = context.game.uiState();
    const decision = state.decision;
    if (!decision || decision.kind === "priority" || decision.kind === "attackers" || decision.kind === "blockers") {
      return state;
    }
    if (decision.kind === "targets") {
      const target = chooseTarget(decision, immediateTarget ?? context.targets.shift());
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
      const choice = nextChoice(context, null);
      const option = chooseOption(context, decision, choice);
      context.game.dispatch({
        type: "select_options",
        option_indices: [option.index ?? option.id ?? 0],
      });
      continue;
    }
    if (decision.kind === "select_objects") {
      const target = context.targets.shift() ?? context.choices.shift();
      const object = chooseObjectCandidate(decision, target);
      context.game.dispatch({
        type: "select_objects",
        object_ids: [object.id ?? object.object ?? object.object_id],
      });
      continue;
    }
    if (decision.kind === "order") {
      const ids = (decision.objects || decision.options || []).map((object) => object.id ?? object.object_id ?? object.object);
      context.game.dispatch({ type: "order", order: ids });
      continue;
    }
    if (decision.kind === "number") {
      context.game.dispatch({ type: "number_choice", value: Number(nextChoice(context, decision.min ?? 0)) });
      continue;
    }
    throw new Error(`unsupported decision kind ${decision.kind}: ${JSON.stringify(decision, null, 2)}`);
  }
  throw new Error("decision answering loop did not converge");
}

function chooseTarget(decision, wanted) {
  const legalTargets = (decision.requirements || []).flatMap((requirement) => requirement.legal_targets || []);
  assert(legalTargets.length > 0, "target decision has no legal targets", decision);
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

function chooseOption(context, decision, wanted) {
  const options = decision.options || [];
  assert(options.length > 0, "select_options decision has no options", decision);
  const legalOptions = options.filter((option) => option.legal !== false);
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
  const text = String(wanted).toLowerCase();
  return options.find((option) =>
    String(option.label ?? option.text ?? option.name ?? option.description ?? "").toLowerCase().includes(text),
  ) ?? legalOptions[0] ?? options[0];
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

function nextChoice(context, fallback) {
  return context.choices.length ? context.choices.shift() : fallback;
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
    await advanceTo(context, operation.turn, operation.phase, operation.player ?? null);
  }
}

async function assertLife(context, operation) {
  await prepareAssertion(context, operation);
  const player = getCheckpoint(context.game).players[playerIndex(operation.player)];
  assert(player.life === Number(operation.life), `expected life ${operation.life} for ${operation.player}, got ${player.life}`);
}

async function assertPermanentCount(context, operation) {
  await prepareAssertion(context, operation);
  const name = cardName(operation.name);
  const actual = countByName(getBattlefield(getCheckpoint(context.game), operation.player), name);
  assert(actual === Number(operation.count), `expected ${operation.count} ${name} permanents, got ${actual}`);
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
  assert(actual === Number(operation.count), `expected ${operation.count} ${name || zone} cards in ${zone}, got ${actual}`);
}

async function assertExileCount(context, operation) {
  await prepareAssertion(context, operation);
  const cards = getExile(getCheckpoint(context.game), operation.player ?? null);
  const name = operation.name ? cardName(operation.name) : null;
  const actual = name ? countByName(cards, name) : cards.length;
  assert(actual === Number(operation.count), `expected ${operation.count} ${name || "exile"} cards in exile, got ${actual}`);
}

async function assertPowerToughness(context, operation) {
  await prepareAssertion(context, operation);
  const name = cardName(operation.name);
  const object = getPermanent(getCheckpoint(context.game), operation.player, name);
  const details = getObjectDetails(context.game, object.id);
  assert(
    details.power === Number(operation.power) && details.toughness === Number(operation.toughness),
    `expected ${name} ${operation.power}/${operation.toughness}, got ${details.power}/${details.toughness}`,
  );
}

async function assertTappedCount(context, operation) {
  await prepareAssertion(context, operation);
  const name = cardName(operation.name);
  const actual = getBattlefield(getCheckpoint(context.game), operation.player ?? null).filter(
    (object) => object.name === name && Boolean(object.tapped) === Boolean(operation.tapped),
  ).length;
  assert(actual === Number(operation.count), `expected ${operation.count} ${name} tapped=${operation.tapped}, got ${actual}`);
}

async function assertCounterCount(context, operation) {
  await prepareAssertion(context, operation);
  const name = cardName(operation.name);
  const object = getPermanent(getCheckpoint(context.game), operation.player, name);
  const counter = (object.counters || []).find((candidate) =>
    String(candidate.kind).toLowerCase().includes(String(operation.counter).toLowerCase()),
  );
  const actual = counter?.amount ?? 0;
  assert(actual === Number(operation.count), `expected ${operation.count} ${operation.counter} counters on ${name}, got ${actual}`);
}

async function assertAbility(context, operation) {
  await prepareAssertion(context, operation);
  const name = cardName(operation.name);
  const object = getPermanent(getCheckpoint(context.game), operation.player, name);
  const details = getObjectDetails(context.game, object.id);
  const has = (details.abilities || []).some((ability) => ability.includes(operation.ability));
  assert(has === Boolean(operation.expected), `expected ability ${operation.ability} presence=${operation.expected} on ${name}`, details.abilities);
}

function compareScheduled(left, right) {
  return (
    Number(left.turn || 1) - Number(right.turn || 1) ||
    phaseOrder(left.phase) - phaseOrder(right.phase)
  );
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

function attackTarget(defender) {
  return { kind: "player", player: playerIndex(defender ?? 1) };
}

function labelFragments(label) {
  return String(label)
    .split(/\s+/)
    .map((fragment) => fragment.trim())
    .filter((fragment) => fragment.length > 2 && !/^\{.*\}$/.test(fragment))
    .slice(0, 4);
}

function actionLabelMatches(action, wanted) {
  const label = String(action.label || "");
  const raw = String(wanted || "").trim();
  if (!raw) return false;
  const lowered = label.toLowerCase();
  const wantedLowered = raw.toLowerCase();
  if (lowered.includes(wantedLowered)) return true;

  if (/^(cast|play)\s+/i.test(raw)) {
    const name = cardName(raw).toLowerCase();
    return lowered.includes(name);
  }

  return labelFragments(raw).every((fragment) => lowered.includes(fragment.toLowerCase()));
}
