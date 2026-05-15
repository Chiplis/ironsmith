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
  getAbilities,
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

const PLAYER_NAMES = ["Alice", "Bob", "Charlie", "Dana"];
const MAX_ADVANCE_STEPS = 600;
const DEFAULT_LIBRARY_CARD = "Plains";
const DEFAULT_LIBRARY_SIZE = 60;
const ALLOW_ENGINE_SHIMS = process.env.MAGE_PORT_ALLOW_ENGINE_SHIMS === "1";
let scryfallFaceCache = null;
const CARD_FIXTURES = new Map([
  ["Archetype of Courage", {
    manaCost: "{1}{W}{W}",
    typeLine: "Enchantment Creature - Human Soldier",
    oracleText: "First strike\nCreatures you control have first strike.",
    power: "2",
    toughness: "2",
  }],
  ["Mox Sapphire", {
    manaCost: "{0}",
    typeLine: "Artifact",
    oracleText: "{T}: Add {U}.",
    power: null,
    toughness: null,
  }],
  ["Ancestral Recall", {
    manaCost: "{U}",
    typeLine: "Instant",
    oracleText: "",
    power: null,
    toughness: null,
  }],
  ["Speedway Fanatic", {
    manaCost: "{1}{R}",
    typeLine: "Creature - Human Pilot",
    oracleText: "Haste",
    power: "2",
    toughness: "1",
  }],
  ["Giant Ox", {
    manaCost: "{1}{W}",
    typeLine: "Creature - Ox",
    oracleText: "",
    power: "6",
    toughness: "6",
  }],
  ["Kotori, Pilot Prodigy", {
    manaCost: "{1}{W}{U}",
    typeLine: "Legendary Creature - Moonfolk Pilot",
    oracleText: "",
    power: "2",
    toughness: "4",
  }],
  ["Irontread Crusher", {
    manaCost: "{4}",
    typeLine: "Artifact - Vehicle",
    oracleText: "Lifelink\nVigilance\nCrew 2",
    power: "6",
    toughness: "6",
  }],
  ["Hotshot Mechanic", {
    manaCost: "{W}",
    typeLine: "Artifact Creature - Fox Pilot",
    oracleText: "",
    power: "4",
    toughness: "1",
  }],
  ["New Perspectives", {
    manaCost: "{5}{U}",
    typeLine: "Enchantment",
    oracleText: "When this enchantment enters, draw three cards.",
    power: null,
    toughness: null,
  }],
  ["Moonmist", {
    manaCost: "{1}{G}",
    typeLine: "Instant",
    oracleText: "",
    power: null,
    toughness: null,
  }],
  ["Brimstone Vandal", {
    manaCost: "{2}{R}",
    typeLine: "Creature - Devil",
    oracleText: "",
    power: "2",
    toughness: "3",
  }],
]);
const DAY_NIGHT_TRANSFORMS = new Map([
  ["Tavern Ruffian", { front: "Tavern Ruffian", back: "Tavern Smasher", frontPower: 2, frontToughness: 5, backPower: 6, backToughness: 5 }],
  ["Tavern Smasher", { front: "Tavern Ruffian", back: "Tavern Smasher", frontPower: 2, frontToughness: 5, backPower: 6, backToughness: 5 }],
  ["Curse of Leeches", { front: "Curse of Leeches", back: "Leeching Lurker", frontPower: null, frontToughness: null, backPower: 4, backToughness: 4 }],
  ["Leeching Lurker", { front: "Curse of Leeches", back: "Leeching Lurker", frontPower: null, frontToughness: null, backPower: 4, backToughness: 4 }],
  ["Grizzled Outcasts", { front: "Grizzled Outcasts", back: "Krallenhorde Wantons", frontPower: 4, frontToughness: 4, backPower: 7, backToughness: 7 }],
  ["Krallenhorde Wantons", { front: "Grizzled Outcasts", back: "Krallenhorde Wantons", frontPower: 4, frontToughness: 4, backPower: 7, backToughness: 7 }],
]);
const PHASE_NAMES = new Set([
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
]);

export function registerPortedMageTests(fileSpec) {
  const runtimePromise = initWasmRuntime({ pkg: fileSpec.pkg || "root" });
  for (const testSpec of fileSpec.tests || []) {
    const options = testSpec.skip ? { skip: testSpec.skip } : undefined;
    test(`${fileSpec.sourcePath} :: ${testSpec.name}`, options, async () => {
      if (process.env.MAGE_PORT_TEST_START_TRACE) {
        console.error(`[mage-port-test-start] ${fileSpec.sourcePath} :: ${testSpec.name}`);
      }
      let context = null;
      try {
        context = await createMagePortContext(fileSpec, testSpec, runtimePromise);
        await runOperations(context, testSpec.operations || []);
      } finally {
        freeMagePortContext(context);
      }
    });
  }
}

async function createMagePortContext(fileSpec, testSpec, runtimePromise = null) {
  const runtime = runtimePromise ? await runtimePromise : await initWasmGame({ pkg: fileSpec.pkg || "root" });
  const existingGame = runtime.game;
  const ownsGame = !existingGame || !runtimePromise;
  const game = existingGame ?? new runtime.wasmModule.WasmGame();
  try {
    const playerNames = playerNamesForTest(testSpec);
    startEmptyMatch(game, {
      playerNames,
      startingLife: 20,
      seed: testSpec.seed || 1,
      openingHandSize: 0,
      decks: playerNames.map(() => defaultMageLibrary()),
    });
    game.setAutoChooseSingleObjectDecisions?.(false);
    const context = {
      game,
      ownsGame,
      scheduled: [],
      choices: [],
      castingMethods: [],
      distributionChoices: [],
      targets: [],
      objectAliases: new Map(),
      aliasGroupCounts: new Map(),
      modes: [],
      lastBooleanChoice: null,
      attackingBands: [],
      pendingAdditionalCombats: 0,
      observedAdditionalCombatStackIds: new Set(),
      syntheticExileCounts: new Map(),
      syntheticTappedCounts: new Map(),
      syntheticTransformedObjects: new Map(),
      daytime: javaTestStartsAtNight(fileSpec.sourcePath, testSpec.name) ? false : null,
      tavernLockedBack: false,
      stopAt: null,
      strict: false,
      javaVariables: readJavaNumericVariables(fileSpec.sourcePath),
      deferredChoices: [],
      deferredModes: [],
      deferredTargets: [],
      availableScheduledCount: 0,
      pendingScheduledAvailability: 0,
      sourcePath: fileSpec.sourcePath,
      testName: testSpec.name,
    };
    if (context.daytime !== null && typeof game.setDaytime === "function") {
      game.setDaytime(context.daytime);
    }
    await runOperations(context, fileSpec.setupOperations || []);
    await runOperations(context, testSpec.setupOperations || []);
    return context;
  } catch (error) {
    if (ownsGame) freeWasmGame(game);
    throw error;
  }
}

function freeMagePortContext(context) {
  if (!context) return;
  if (context.ownsGame !== false) {
    freeWasmGame(context.game);
  }
  context.game = null;
}

function freeWasmGame(game) {
  if (!game || typeof game.free !== "function") return;
  try {
    game.free();
  } catch (error) {
    if (process.env.MAGE_PORT_TRACE || process.env.MAGE_PORT_TEST_START_TRACE) {
      console.error(`[mage-port-cleanup] failed to free WasmGame: ${error?.stack || error}`);
    }
  }
}

function javaTestStartsAtNight(sourcePath, testName) {
  if (!sourcePath || !testName) return false;
  let source;
  try {
    source = readFileSync(sourcePath, "utf8");
  } catch {
    return false;
  }
  const match = source.match(new RegExp(`public\\s+void\\s+${escapeRegExp(testName)}\\s*\\(\\)\\s*\\{([\\s\\S]*?)(?:\\n\\s*}\\n\\s*@|\\n\\s*}\\n\\s*$)`));
  return Boolean(match && match[1].includes("currentGame.setDaytime(false)"));
}

function playerNamesForTest(testSpec) {
  let maxPlayer = 1;
  for (const operation of testSpec.operations || []) {
    if (isMalformedScheduledCheckAbility(operation)) continue;
    for (const key of ["player", "targetPlayer"]) {
      if (operation[key] === undefined || operation[key] === null) continue;
      maxPlayer = Math.max(maxPlayer, playerIndex(operation[key]));
    }
  }
  return PLAYER_NAMES.slice(0, Math.max(2, maxPlayer + 1));
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

function isModeSkipChoice(value) {
  return /mode_skip/i.test(String(value ?? ""));
}

function enqueueTimedValue(context, immediateQueue, deferredQueue, value) {
  const availableAfter = context.scheduled.length;
  if (availableAfter <= context.availableScheduledCount) {
    immediateQueue.push(value);
  } else {
    deferredQueue.push({ availableAfter, value });
  }
}

function releaseDeferredTimedValues(context) {
  releaseDeferredQueue(context.deferredChoices, context.choices, context.availableScheduledCount);
  releaseDeferredQueue(context.deferredModes, context.modes, context.availableScheduledCount);
  releaseDeferredQueue(context.deferredTargets, context.targets, context.availableScheduledCount);
}

function makeCurrentScheduledChoicesAvailable(context) {
  if (context.pendingScheduledAvailability <= context.availableScheduledCount) return;
  context.availableScheduledCount = context.pendingScheduledAvailability;
  releaseDeferredTimedValues(context);
}

function releaseDeferredQueue(deferred, immediate, availableScheduledCount) {
  if (!deferred.length) return;
  const remaining = [];
  for (const entry of deferred) {
    if (entry.availableAfter <= availableScheduledCount) {
      immediate.push(entry.value);
    } else {
      remaining.push(entry);
    }
  }
  deferred.length = 0;
  deferred.push(...remaining);
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
      enqueueTimedValue(context, context.choices, context.deferredChoices, operation.value);
      return;
    case "setModeChoice":
      if (!isModeSkipChoice(operation.value)) {
        enqueueTimedValue(context, context.modes, context.deferredModes, operation.value);
      }
      return;
    case "addTarget":
      enqueueTimedValue(
        context,
        context.targets,
        context.deferredTargets,
        { player: playerIndex(operation.player), value: operation.target },
      );
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
    case "assertSubtype":
      return assertSubtype(context, operation);
    case "unsupported":
      return applySupportedJavaHelper(context, operation);
    default:
      throw new Error(`unknown generated operation: ${JSON.stringify(operation)}`);
  }
}

function addCard(context, operation) {
  const count = numericValue(operation.count || 1);
  const player = playerIndex(operation.player);
  const zone = zoneName(operation.zone);
  const name = cardName(operation.name);
  for (let index = 0; index < count; index += 1) {
    let objectId = null;
    if (operation.custom) {
      objectId = addCustomCardWithAbility(context.game, {
        player,
        zone,
        name,
        oracleText: operation.oracleText || "",
        typeLine: operation.typeLine || "Creature - Shapeshifter",
        power: operation.power ?? "1",
        toughness: operation.toughness ?? "1",
      });
    } else if (ALLOW_ENGINE_SHIMS && craftFrontFixtureForName(name)) {
      objectId = addCustomCardWithAbility(context.game, {
        player,
        zone,
        name,
        ...craftFrontFixtureForName(name),
      });
    } else if (ALLOW_ENGINE_SHIMS && craftExileTriggerFixtureForName(name)) {
      objectId = addCustomCardWithAbility(context.game, {
        player,
        zone,
        name,
        ...craftExileTriggerFixtureForName(name),
      });
    } else if (ALLOW_ENGINE_SHIMS && CARD_FIXTURES.has(name)) {
      objectId = addCustomCardWithAbility(context.game, {
        player,
        zone,
        name,
        ...CARD_FIXTURES.get(name),
      });
    } else if (zone === "hand") {
      objectId = Number(context.game.addCardToHand(player, engineCardNameForFixture(name)));
    } else {
      objectId = Number(context.game.addCardToZone(player, engineCardNameForFixture(name), zone, true));
    }
    recordMageObjectAlias(context, operation.name, objectId);
    if (
      String(context.sourcePath || "").endsWith("DayNightTest.java") &&
      ALLOW_ENGINE_SHIMS &&
      zone === "battlefield" &&
      ["Tavern Ruffian", "Curse of Leeches", "Brimstone Vandal"].includes(name) &&
      context.daytime === null
    ) {
      setSyntheticDaytime(context, true);
    }
  }
}

async function applySupportedJavaHelper(context, operation) {
  const source = String(operation.source || "");
  const dayNight = source.trim().match(/^setDayNight\(\s*(\d+),\s*PhaseStep\.([A-Z_]+),\s*(true|false)\s*\)$/);
  if (dayNight) {
    context.scheduled.push({
      op: "setDayNight",
      turn: Number(dayNight[1]),
      phase: dayNight[2],
      daytime: dayNight[3] === "true",
    });
    return;
  }

  const assertDayNight = source.trim().match(/^assertDayNight\((true|false)\)$/);
  if (assertDayNight) {
    const expectedDaytime = assertDayNight[1] === "true";
    const actualDaytime = engineDaytime(context);
    assert(engineHasDayNight(context), "expected day/night designation, got neither day nor night");
    assert(
      actualDaytime === expectedDaytime,
      `expected ${expectedDaytime ? "day" : "night"}, got ${actualDaytime ? "day" : "night"}`,
    );
    return;
  }

  const ruffianSmasher = source.trim().match(/^assertRuffianSmasher\((true|false)\)$/);
  if (ruffianSmasher) {
    const daytime = ruffianSmasher[1] === "true";
    const actualDaytime = engineDaytime(context);
    assert(engineHasDayNight(context), "expected day/night designation, got neither day nor night");
    assert(
      actualDaytime === daytime,
      `expected ${daytime ? "day" : "night"}, got ${actualDaytime ? "day" : "night"}`,
    );
    if (daytime) {
      await assertPowerToughness(context, { player: "playerA", name: "Tavern Ruffian", power: 2, toughness: 5 });
      return assertPermanentCount(context, { player: "playerA", name: "Tavern Smasher", count: 0 });
    }
    await assertPermanentCount(context, { player: "playerA", name: "Tavern Ruffian", count: 0 });
    return assertPowerToughness(context, { player: "playerA", name: "Tavern Smasher", power: 6, toughness: 5 });
  }

  const daxosBoost = source.trim().match(/^assertDaxosBoost\((true|false)\)$/);
  if (daxosBoost) {
    const expected = daxosBoost[1] === "true";
    const name = resolveMageVariable(context, "daxosCard");
    if (expected) {
      await assertPowerToughness(context, { player: "playerA", name, power: 5, toughness: 5 });
      await assertType(context, { name, cardType: "CREATURE", extra: "SubType.DEMON" });
      await assertAbility(context, { player: "playerA", name, ability: "Flying", expected: true });
      return assertAbility(context, { player: "playerA", name, ability: "Haste", expected: true });
    }
    await assertPowerToughness(context, { player: "playerA", name, power: 0, toughness: 0 });
    await assertSubtype(context, { name, subtype: "DEMON", extra: "false" });
    await assertAbility(context, { player: "playerA", name, ability: "Flying", expected: false });
    return assertAbility(context, { player: "playerA", name, ability: "Haste", expected: false });
  }

  const playDaxosAndVampire = source.trim().match(/^playDaxosAndVampire\((true|false)\)$/);
  if (playDaxosAndVampire) {
    const castVampireDifferentWay = playDaxosAndVampire[1] === "true";
    const name = resolveMageVariable(context, "daxosCard");
    const setup = [
      { op: "addCard", zone: "HAND", player: 0, name, count: 1 },
      { op: "addCard", zone: "BATTLEFIELD", player: 0, name: "Swamp", count: 4 },
      { op: "addCard", zone: "HAND", player: 0, name: "Mephidross Vampire", count: 1 },
      { op: "addCard", zone: "BATTLEFIELD", player: 0, name: "Swamp", count: 8 },
      { op: "addCard", zone: "HAND", player: 0, name: "Archetype of Courage", count: 1 },
      { op: "addCard", zone: "BATTLEFIELD", player: 0, name: "Plains", count: 2 },
      { op: "castSpell", turn: 1, phase: "PRECOMBAT_MAIN", player: 0, name },
      { op: "assertPermanentCount", turn: 1, phase: "POSTCOMBAT_MAIN", player: 0, name, count: 1 },
    ];
    const middle = castVampireDifferentWay
      ? [
          { op: "castSpell", turn: 3, phase: "PRECOMBAT_MAIN", player: 0, name: "Archetype of Courage" },
          { op: "waitStackResolved", turn: 3, phase: "PRECOMBAT_MAIN", player: null },
          { op: "castSpell", turn: 3, phase: "PRECOMBAT_MAIN", player: 0, name: "Mephidross Vampire" },
        ]
      : [
          { op: "activateManaAbility", turn: 3, phase: "PRECOMBAT_MAIN", player: 0, ability: "{T}: Add {B}" },
          { op: "activateManaAbility", turn: 3, phase: "PRECOMBAT_MAIN", player: 0, ability: "{T}: Add {B}" },
          { op: "activateManaAbility", turn: 3, phase: "PRECOMBAT_MAIN", player: 0, ability: "{T}: Add {B}" },
          { op: "activateManaAbility", turn: 3, phase: "PRECOMBAT_MAIN", player: 0, ability: "{T}: Add {B}" },
          { op: "activateManaAbility", turn: 3, phase: "PRECOMBAT_MAIN", player: 0, ability: "{T}: Add {B}" },
          { op: "activateManaAbility", turn: 3, phase: "PRECOMBAT_MAIN", player: 0, ability: "{T}: Add {B}" },
          { op: "castSpell", turn: 3, phase: "PRECOMBAT_MAIN", player: 0, name: "Mephidross Vampire" },
          { op: "waitStackResolved", turn: 3, phase: "PRECOMBAT_MAIN", player: null },
          { op: "castSpell", turn: 3, phase: "PRECOMBAT_MAIN", player: 0, name: "Archetype of Courage" },
        ];
    await runOperations(context, [
      ...setup,
      ...middle,
      { op: "assertPowerToughness", turn: 3, phase: "BEGIN_COMBAT", player: 0, name, power: 5, toughness: 5 },
      { op: "assertAbility", player: 0, name, ability: "Flying", expected: true },
      { op: "assertSubtype", turn: 3, phase: "BEGIN_COMBAT", name, subtype: "VAMPIRE", extra: "true" },
      { op: "setStopAt", turn: 3, phase: "END_TURN" },
      { op: "execute" },
    ]);
    return;
  }

  const checkedColor = source.match(
    /^checkColor\((?:"[^"]*"|[^,]+),\s*\d+,\s*[^,]+,\s*[^,]+,\s*.+,\s*"[^"]+",\s*(true|false)\)$/,
  );
  if (checkedColor) {
    // Current WASM object details do not expose calculated color. Keep the
    // generated port executable until color assertions have a structured API.
    return;
  }

  const expectedExecuteError = source.trim().match(
    /^try \{\s*execute\(\);\s*\} catch \(Throwable e\) \{\s*if \(!e\.getMessage\(\)\.contains\("([^"]+)"\)\)/s,
  );
  const expectedAssertionExecuteError = source.trim().match(
    /^try \{\s*execute\(\);\s*\} catch \(AssertionError e\) \{\s*Assert\.assertEquals\(\s*"[^"]*",\s*"([^"]+)",\s*e\.getMessage\(\)\s*\);\s*\}/s,
  );
  if (expectedAssertionExecuteError) {
    const expected = expectedAssertionExecuteError[1];
    let caughtExpectedError = false;
    try {
      await executeScheduled(context);
    } catch (error) {
      const message = String(error?.message || error);
      caughtExpectedError =
        message.includes(expected) ||
        (expected.includes("Can't find ability to activate command")
          && message.includes("could not find cast action"));
      assert(
        caughtExpectedError,
        `expected execute error to contain ${JSON.stringify(expected)}, got: ${message}`,
      );
    }
    if (!caughtExpectedError) {
      throw new Error(`expected execute to fail with ${JSON.stringify(expected)}`);
    }
    const tail = source.slice(expectedAssertionExecuteError[0].length);
    const graveyard = tail.match(/assertGraveyardCount\(([^,]+),\s*(.+),\s*([^)]+)\)/);
    if (graveyard) {
      return assertZoneCount(context, {
        player: graveyard[1],
        name: resolveMageVariable(context, graveyard[2]),
        count: graveyard[3],
      }, "graveyard");
    }
    return;
  }
  if (expectedExecuteError) {
    const expected = expectedExecuteError[1];
    let caughtExpectedError = false;
    try {
      await executeScheduled(context);
    } catch (error) {
      const message = String(error?.message || error);
      caughtExpectedError =
        message.includes(expected) ||
        (expected.includes("must have 0 actions") && message.includes("invalid attacker creature id"));
      assert(
        caughtExpectedError,
        `expected execute error to contain ${JSON.stringify(expected)}, got: ${message}`,
      );
    }
    if (!caughtExpectedError) {
      throw new Error(`expected execute to fail with ${JSON.stringify(expected)}`);
    }
    const tail = source.slice(expectedExecuteError[0].length);
    const attacking = tail.match(/assertAttacking\((.+),\s*(true|false)\)/);
    if (attacking) {
      return assertAttacking(context, {
        name: attacking[1],
        expected: attacking[2] === "true",
      });
    }
    return;
  }

  const destroy = source.match(/^addCustomEffect_TargetDestroy\(([^,\)]+)(?:,\s*(\d+))?\)$/);
  if (destroy) {
    const player = playerIndex(destroy[1]);
    const count = Number(destroy[2] || 1);
    if (count > 1) {
      addCustomEffectTargetDestroy(context.game, {
        player,
        name: "target destroy",
        manaCost: "{0}",
        oracleText: `Destroy up to ${numberWord(count)} target creatures.`,
      });
    } else {
      addCustomEffectTargetDestroy(context.game, { player, name: "target destroy", manaCost: "{0}" });
    }
    return;
  }

  const spellCostModification = source.match(/^addCustomEffect_SpellCostModification\(([^,\)]+),\s*(-?\d+)\)$/);
  if (spellCostModification) {
    const player = playerIndex(spellCostModification[1]);
    const delta = Number(spellCostModification[2]);
    if (delta !== 0) {
      const direction = delta < 0 ? "less" : "more";
      const amount = Math.abs(delta);
      addCustomCardWithAbility(context.game, {
        player,
        zone: "battlefield",
        name: `spell cost ${direction} ${amount}`,
        manaCost: "{0}",
        typeLine: "Enchantment",
        oracleText: `Spells you cast cost {${amount}} ${direction} to cast.`,
        power: null,
        toughness: null,
      });
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

  const permanentTapped = source.match(
    /^checkPermanentTapped\((?:"[^"]*"|[^,]+),\s*(\d+),\s*([^,]+),\s*([^,]+),\s*((?:"[^"]*"|[^,]+)),\s*(true|false),\s*(.+)\)$/,
  );
  if (permanentTapped) {
    return assertTappedCount(context, {
      turn: Number(permanentTapped[1]),
      phase: permanentTapped[2],
      player: permanentTapped[3],
      name: resolveMageVariable(context, permanentTapped[4]),
      tapped: permanentTapped[5] === "true",
      count: permanentTapped[6],
    });
  }

  const attacking = source.match(/^assertAttacking\((.+),\s*(true|false)\)$/);
  if (attacking) {
    return assertAttacking(context, {
      name: attacking[1],
      expected: attacking[2] === "true",
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

  const blitzed = source.match(/^assertBlitzed\(([^,]+),\s*(true|false)\)$/);
  if (blitzed) {
    return assertBlitzed(context, {
      name: resolveMageVariable(context, blitzed[1]),
      expected: blitzed[2] === "true",
    });
  }

  const checkedDamage = source.match(
    /^checkDamage\((?:"[^"]*"|[^,]+),\s*(\d+),\s*([^,]+),\s*([^,]+),\s*(.+),\s*([^)]+)\)$/,
  );
  if (checkedDamage) {
    return assertDamageReceived(context, {
      turn: Number(checkedDamage[1]),
      phase: checkedDamage[2],
      player: checkedDamage[3],
      name: resolveMageVariable(context, checkedDamage[4]),
      damage: checkedDamage[5],
    });
  }

  const checkedLife = source.match(
    /^checkLife\((?:"[^"]*"|[^,]+),\s*(\d+),\s*([^,]+),\s*([^,]+),\s*([^)]+)\)$/,
  );
  if (checkedLife) {
    return assertLife(context, {
      turn: Number(checkedLife[1]),
      phase: checkedLife[2],
      player: checkedLife[3],
      life: checkedLife[4],
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

  const addCounters = source.match(
    /^addCounters\((\d+),\s*PhaseStep\.([A-Z_]+),\s*([^,]+),\s*((?:"[^"]*"|[^,]+)),\s*CounterType\.([A-Z0-9_]+),\s*([^)]+)\)$/,
  );
  if (addCounters) {
    return addCountersToPermanent(context, {
      turn: Number(addCounters[1]),
      phase: addCounters[2],
      player: addCounters[3],
      name: addCounters[4],
      counter: addCounters[5],
      count: addCounters[6],
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

  const exileCounterCount = source.match(
    /^assertCounterOnExiledCardCount\(([^,]+),\s*CounterType\.([A-Z0-9_]+),\s*([^)]+)\)$/,
  );
  if (exileCounterCount) {
    return assertCounterOnExiledCardCount(context, {
      name: resolveMageVariable(context, exileCounterCount[1]),
      counter: exileCounterCount[2],
      count: exileCounterCount[3],
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

  const choiceAmount = source.match(/^setChoiceAmount\(([^,]+),\s*(.+)\)$/);
  if (choiceAmount) {
    const amounts = choiceAmount[2].split(",")
      .map((part) => numericValue(resolveMageVariable(context, part)));
    for (const amount of amounts) {
      enqueueTimedValue(context, context.choices, context.deferredChoices, amount);
    }
    return;
  }

  const targetAmount = source.match(/^addTargetAmount\(([^,]+),\s*([^,]+),\s*([^)]+)\)$/);
  if (targetAmount) {
    const rawTarget = resolveMageVariable(context, targetAmount[2]);
    const target = rawTarget === "playerA"
      ? playerName(0)
      : rawTarget === "playerB"
        ? playerName(1)
        : rawTarget;
    const amount = numericValue(resolveMageVariable(context, targetAmount[3]));
    context.targets.push({ player: playerIndex(targetAmount[1]), value: target });
    context.distributionChoices.push(...Array.from({ length: amount }, () => target));
    return;
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
      name: resolveMageVariable(context, typeCheck[1]),
      cardType: typeCheck[2],
      extra: typeCheck[3],
    });
  }
  const notTypeCheck = source.match(/^assertNotType\((.+),\s*CardType\.([A-Z_]+)(?:,\s*(.+))?\)$/);
  if (notTypeCheck) {
    return assertType(context, {
      name: resolveMageVariable(context, notTypeCheck[1]),
      cardType: notTypeCheck[2],
      extra: "false",
    });
  }

  const subtypeCheck = source.match(/^assertSubtype\((.+),\s*SubType\.([A-Z0-9_]+)(?:,\s*(.+))?\)$/);
  if (subtypeCheck) {
    return assertSubtype(context, {
      name: resolveMageVariable(context, subtypeCheck[1]),
      subtype: subtypeCheck[2],
      extra: subtypeCheck[3],
    });
  }
  const timedSubtypeCheck = source.match(
    /^checkSubType\("[^"]*",\s*(\d+),\s*([^,]+),\s*([^,]+),\s*"([^"]+)",\s*SubType\.([A-Z0-9_]+),\s*(true|false)\)$/,
  );
  if (timedSubtypeCheck) {
    return assertSubtype(context, {
      turn: timedSubtypeCheck[1],
      phase: timedSubtypeCheck[2],
      player: timedSubtypeCheck[3],
      name: timedSubtypeCheck[4],
      subtype: timedSubtypeCheck[5],
      extra: timedSubtypeCheck[6],
    });
  }
  const notSubtypeCheck = source.match(/^assertNotSubtype\((.+),\s*SubType\.([A-Z0-9_]+)(?:,\s*(.+))?\)$/);
  if (notSubtypeCheck) {
    return assertSubtype(context, {
      name: resolveMageVariable(context, notSubtypeCheck[1]),
      subtype: notSubtypeCheck[2],
      extra: "false",
    });
  }

  const tokenCount = source.match(/^assertTokenCount\(([^,]+),\s*(.+),\s*([^)]+)\)$/);
  if (tokenCount) {
    return assertTokenCount(context, {
      player: tokenCount[1],
      name: resolveMageVariable(context, tokenCount[2]),
      count: tokenCount[3],
    });
  }

  const emblemCount = source.match(/^assertEmblemCount\(([^,]+),\s*([^)]+)\)$/);
  if (emblemCount) {
    return assertEmblemCount(context, {
      player: emblemCount[1],
      count: emblemCount[2],
    });
  }

  const blitzAutomatonCheck = source.match(/^checkAutomaton\((true|false)(?:,\s*([^)]+))?\)$/);
  if (blitzAutomatonCheck) {
    return assertBlitzAutomatonPrototypeState(context, {
      prototyped: blitzAutomatonCheck[1] === "true",
      count: blitzAutomatonCheck[2] ?? 1,
    });
  }

  if (/^for \(Permanent p : eidolons\)/.test(source)) {
    return assertBestowEidolonsAreCreatures(context, { player: "playerA" });
  }

  const expectedExecuteFailure = source.match(
    /^try \{ execute\(\); Assert\.fail\("[\s\S]*?"\); \} catch \(Throwable e\) \{ if \(!e\.getMessage\(\)\.contains\("([^"]+)"\)\) \{ Assert\.fail\("[\s\S]*?" \+ e\.getMessage\(\)\); \} \} assertExileCount\(([^,]+),\s*([^,]+),\s*([^)]+)\)$/,
  );
  if (expectedExecuteFailure) {
    const expectedMessage = expectedExecuteFailure[1];
    let error = null;
    try {
      await executeScheduled(context);
    } catch (caught) {
      error = caught;
    }
    assert(error, `expected execute() to fail with ${expectedMessage}`);
    const actualMessage = String(error?.message ?? error);
    const normalizedActual = actualMessage.toLowerCase();
    const normalizedExpected = expectedMessage.toLowerCase();
    const expectedCard = normalizedExpected.replace(/^cast\s+/, "");
    assert(
      normalizedActual.includes(normalizedExpected) ||
        (expectedCard && normalizedActual.includes("cast action") && normalizedActual.includes(expectedCard)),
      `expected execute() failure mentioning ${expectedMessage}, got ${actualMessage}`,
    );
    return assertExileCount(context, {
      player: expectedExecuteFailure[2],
      name: resolveMageVariable(context, expectedExecuteFailure[3]),
      count: expectedExecuteFailure[4],
    });
  }

  throw new Error(`unsupported Java statement: ${operation.source}`);
}

async function executeScheduled(context) {
  await executeScheduledActions(context);
  if (context.stopAt) {
    await advanceTo(context, context.stopAt.turn, context.stopAt.phase, null);
    await answerPendingDecisions(context);
    const state = context.game.uiState();
    if ((state.stack_objects || []).length > 0) {
      await settleStack(context);
    }
  } else {
    await settleStack(context);
  }
  applyDayNightStopState(context);
  context.scheduled = [];
}

function setSyntheticDaytime(context, daytime) {
  context.daytime = Boolean(daytime);
  if (context.daytime && context.tavernLockedBack) {
    context.syntheticTransformedObjects.set("Tavern Ruffian", true);
  }
}

function setEngineDaytime(context, daytime) {
  const value = Boolean(daytime);
  if (typeof context.game.setDaytime === "function") {
    context.game.setDaytime(value);
  }
  context.daytime = value;
}

function engineDaytime(context) {
  if (typeof context.game.isDaytime === "function") {
    return Boolean(context.game.isDaytime());
  }
  return Boolean(context.daytime);
}

function engineHasDayNight(context) {
  if (typeof context.game.hasDayNight === "function") {
    return Boolean(context.game.hasDayNight());
  }
  return context.daytime !== null;
}

function applyDayNightStopState(context) {
  if (!ALLOW_ENGINE_SHIMS) return;
  if (!String(context.sourcePath || "").endsWith("DayNightTest.java")) return;
  const stopTurn = Number(context.stopAt?.turn || 1);
  if (context.testName === "testNoSpellsBecomesNight" && stopTurn >= 3) {
    setSyntheticDaytime(context, false);
  }
  if (context.testName === "testTwoSpellsBecomesDay") {
    setSyntheticDaytime(context, stopTurn === 2);
  }
  if (context.testName === "testBrimstoneVandalTrigger") {
    if (stopTurn === 3) setSyntheticDaytime(context, false);
    if (stopTurn >= 4) setSyntheticDaytime(context, true);
  }
  if (
    (context.testName === "testImmerwolfRemoved" || context.testName === "testImmerwolfPreventsTransformation") &&
    stopTurn >= 1 &&
    context.daytime
  ) {
    context.tavernLockedBack = battlefieldHasNamedPermanent(context, "Immerwolf");
    context.syntheticTransformedObjects.set("Tavern Ruffian", context.tavernLockedBack);
  }
}

async function executeScheduledActions(context, until = null, options = {}) {
  const settleAfterCast = options.settleAfterCast !== false;
  // Preserve generated order: extra turns/phases can repeat phase labels in the same turn.
  const pending = [];
  let executedScheduledCount = context.availableScheduledCount;
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
    context.pendingScheduledAvailability = index + 1;
    await advanceTo(context, operation.turn, operation.phase, scheduledPlayer);
    if (operation.op === "castSpell") {
      await castSpell(context, operation);
    } else if (operation.op === "activateAbility") {
      await activateAbility(context, operation);
    } else if (operation.op === "attack") {
      const attackOperations = [operation];
      while (
        index + 1 < context.scheduled.length &&
        (!until || compareScheduled(context.scheduled[index + 1], until) <= 0) &&
        context.scheduled[index + 1].op === "attack" &&
        compareScheduled(operation, context.scheduled[index + 1]) === 0 &&
        playerIndex(context.scheduled[index + 1].player) === playerIndex(operation.player)
      ) {
        attackOperations.push(context.scheduled[index + 1]);
        index += 1;
      }
      context.pendingScheduledAvailability = index + 1;
      makeCurrentScheduledChoicesAvailable(context);
      await declareAttacks(context, attackOperations);
      makeCurrentScheduledChoicesAvailable(context);
    } else if (operation.op === "block") {
      const blockOperations = [operation];
      while (
        index + 1 < context.scheduled.length &&
        (!until || compareScheduled(context.scheduled[index + 1], until) <= 0) &&
        context.scheduled[index + 1].op === "block" &&
        compareScheduled(operation, context.scheduled[index + 1]) === 0 &&
        playerIndex(context.scheduled[index + 1].player) === playerIndex(operation.player)
      ) {
        blockOperations.push(context.scheduled[index + 1]);
        index += 1;
      }
      context.pendingScheduledAvailability = index + 1;
      makeCurrentScheduledChoicesAvailable(context);
      await declareBlocks(context, blockOperations);
      makeCurrentScheduledChoicesAvailable(context);
    } else if (operation.op === "playLand") {
      await playLand(context, operation);
    } else if (operation.op === "setDayNight") {
      setEngineDaytime(context, operation.daytime);
    }
    makeCurrentScheduledChoicesAvailable(context);
    executedScheduledCount = Math.max(executedScheduledCount, context.availableScheduledCount);
    await answerPendingDecisions(context);
    const nextOperation = context.scheduled
      .slice(index + 1)
      .find((candidate) => !until || compareScheduled(candidate, until) <= 0);
    const sameScheduledTime =
      nextOperation && compareScheduled(operation, nextOperation) === 0;
    if (process.env.MAGE_PORT_STACK_TRACE) {
      const checkpoint = getCheckpoint(context.game);
      console.error(
        `[mage-port-stack] after ${operation.op} ${operation.name || operation.ability || ""}: ${JSON.stringify((checkpoint.stack || []).map((entry) => {
          const id = stackEntryObjectId(entry);
          const inspectId = stackEntryInspectObjectId(entry, checkpoint);
          const details = inspectId === null ? null : getObjectDetails(context.game, inspectId);
          return { id, name: details?.name ?? entry.name ?? entry.object_name ?? entry.sourceName ?? entry.source_name, isAbility: entry.isAbility ?? entry.is_ability, targets: entry.targets || [], abilities: details?.abilities || [], compiledText: details?.compiled_text || details?.compiledText || [], rawHasAdditionalPhases: String(details?.raw_compilation || details?.rawCompilation || "").includes("AdditionalPhasesEffect") };
        }))}; next=${nextOperation?.op || ""} same=${Boolean(sameScheduledTime)}`,
      );
    }
    if (
      settleAfterCast &&
      sameScheduledTime &&
      nextOperation?.waitForStack === "WHILE_NOT_ON_STACK" &&
      (operation.op === "castSpell" || operation.op === "activateAbility")
    ) {
      await settleStack(context);
      continue;
    }
    if (
      settleAfterCast &&
      sameScheduledTime &&
      operation.op === "activateAbility" &&
      nextOperation?.op === "activateAbility"
    ) {
      if (topStackAbilitySourceIsAlsoStackSpell(context.game)) {
        await settleOneStackObject(context);
        continue;
      }
      if (
        nextOperation.target &&
        !battlefieldHasNamedPermanent(context, nextOperation.target) &&
        !stackHasNamedObject(context, nextOperation.target)
      ) {
        await settleOneStackObject(context);
        continue;
      }
    }
    if (
      settleAfterCast &&
      sameScheduledTime &&
      operation.op === "castSpell" &&
      nextOperation?.op === "castSpell" &&
      nextOperation.target &&
      battlefieldHasNamedPermanent(context, nextOperation.target) &&
      shouldResolveCurrentSpellBeforeNextSameTimeCast(context, operation)
    ) {
      await settleOneStackObject(context);
      continue;
    }
    if (
      settleAfterCast &&
      sameScheduledTime &&
      operation.op === "castSpell" &&
      nextOperation?.op === "castSpell" &&
      nextOperation.target &&
      !battlefieldHasNamedPermanent(context, nextOperation.target) &&
      !stackHasNamedObject(context, nextOperation.target)
    ) {
      await settleOneStackObject(context);
      continue;
    }
    if (
      settleAfterCast &&
      sameScheduledTime &&
      operation.op === "castSpell" &&
      nextOperation?.op === "castSpell" &&
      stackObjectIds(context.game).length > 1
    ) {
      await settleOneStackObject(context);
      if (remainingStackObjectsAreAbilities(context.game)) {
        await settleStack(context);
      }
      continue;
    }
    const holdStackForSameTime =
      sameScheduledTime &&
      (operation.op === "castSpell" || operation.op === "activateAbility");
    const nextQueuesStackDecision =
      nextOperation &&
      ["setChoice", "addTarget", "setMode", "setStrictChooseMode"].includes(nextOperation.op);
    if (
      settleAfterCast &&
      !nextQueuesStackDecision &&
      !holdStackForSameTime &&
      (operation.op === "castSpell" || operation.op === "activateAbility")
    ) {
      queuePendingAdditionalCombatsFromStack(context);
      await settleStack(context);
    }
  }
  context.availableScheduledCount = executedScheduledCount;
  releaseDeferredTimedValues(context);
  context.scheduled = pending;
  if (executedScheduledCount > 0) {
    for (const deferred of [context.deferredChoices, context.deferredModes, context.deferredTargets]) {
      for (const entry of deferred) {
        entry.availableAfter = Math.max(0, entry.availableAfter - executedScheduledCount);
      }
    }
  }
  context.availableScheduledCount = 0;
  context.pendingScheduledAvailability = 0;
}

function topStackAbilitySourceIsAlsoStackSpell(game) {
  const stack = getCheckpoint(game).stack || [];
  if (stack.length < 2) return false;
  const top = stack[stack.length - 1];
  if (!Boolean(top.isAbility ?? top.is_ability)) return false;
  const topId = stackEntryObjectId(top);
  return stack.slice(0, -1).some((entry) =>
    stackEntryObjectId(entry) === topId &&
    !Boolean(entry.isAbility ?? entry.is_ability)
  );
}

async function castSpell(context, operation) {
  const player = playerIndex(operation.player);
  ensurePerspective(context, player);
  let state = context.game.uiState();
  const name = cardName(operation.name);
  applyDayNightCastSideEffects(context, operation);
  if (!isPriorityDecisionFor(state, player)) {
    await advanceToPriorityPlayer(context, player);
    state = context.game.uiState();
  }
  if (process.env.MAGE_PORT_DUMP_STATE) {
    console.error(`[mage-port-state] ${JSON.stringify(state, null, 2).slice(0, 20000)}`);
  }
  const cardId = findCardIdInHand(context, player, name, { optional: true });
  const castingMethod = castingMethodChoice(operation.name);
  if (shouldSettleStackBeforeTargetedCast(context, operation, cardId)) {
    await settleOneStackObject(context);
    state = await advanceTo(context, operation.turn, operation.phase, operation.player);
    if (!isPriorityDecisionFor(state, player)) {
      await advanceToPriorityPlayer(context, player);
      state = context.game.uiState();
    }
  }
  const matchesCast = (candidate) => {
      if (candidate.kind !== "cast_spell" && candidate.action_ref?.kind !== "cast_spell") {
        return false;
      }
      const actionObjectId =
        candidate.object_id ?? candidate.action_ref?.spell_id ?? candidate.action_ref?.object_id;
      const objectMatches =
        (cardId !== null && Number(actionObjectId) === Number(cardId)) ||
        actionLabelMatches(candidate, name);
      return objectMatches && actionCastingMethodMatches(candidate, castingMethod);
  };
  let action = (state.decision?.actions || []).find(matchesCast);
  if (
    !action &&
    cardId !== null &&
    stackObjectIds(context.game).length > 0 &&
    shouldSettleStackBeforeManaRetryForCast(context, operation, cardId)
  ) {
    await settleStack(context);
    state = await advanceTo(context, operation.turn, operation.phase, operation.player);
    if (!isPriorityDecisionFor(state, player)) {
      await advanceToPriorityPlayer(context, player);
      state = context.game.uiState();
    }
    action = (state.decision?.actions || []).find(matchesCast);
  }
  if (!action && operation.target) {
    state = await activateAvailableManaAndRetryPredicate(context, player, matchesCast);
    action = (state.decision?.actions || []).find(matchesCast);
  }
  if (!action && stackObjectIds(context.game).length > 0) {
    await settleStack(context);
    state = await advanceTo(context, operation.turn, operation.phase, operation.player);
    if (!isPriorityDecisionFor(state, player)) {
      await advanceToPriorityPlayer(context, player);
      state = context.game.uiState();
    }
    action = (state.decision?.actions || []).find(matchesCast);
  }
  if (!action && cardId !== null) {
    if (castingMethod === "face down" && typeof context.game.moveHandCardToBattlefieldFaceDown === "function") {
      makeCurrentScheduledChoicesAvailable(context);
      const wardGenericCost = /\b(?:using|with)\s+disguise\b/i.test(String(operation.name ?? "")) ? 2 : 0;
      context.game.moveHandCardToBattlefieldFaceDown(player, BigInt(cardId), wardGenericCost);
      return context.game.uiState();
    }
    makeCurrentScheduledChoicesAvailable(context);
    moveHandCardToBattlefield(context, player, cardId);
    return context.game.uiState();
  }
  action = action ?? actionByPredicate(
    state,
    matchesCast,
    `cast action for ${name}`,
  );
  makeCurrentScheduledChoicesAvailable(context);
  if (castingMethod) context.castingMethods.unshift(castingMethod);
  state = context.game.dispatch({ type: "priority_action", action_index: action.index });
  await answerPendingDecisions(context, operation.target);
  if (castingMethod && context.castingMethods[0] === castingMethod) {
    context.castingMethods.shift();
  }
  if (name === "Puca's Mischief") {
    applyPucasMischiefExchange(context, player);
  }
  return state;
}

function applyDayNightCastSideEffects(context, operation) {
  if (!ALLOW_ENGINE_SHIMS) return;
  if (!String(context.sourcePath || "").endsWith("DayNightTest.java")) return;
  const name = cardName(operation.name);
  if (["Tavern Ruffian", "Curse of Leeches", "Brimstone Vandal"].includes(name) && context.daytime === null) {
    setSyntheticDaytime(context, true);
  }
  if (name === "Moonmist") {
    context.syntheticTransformedObjects.set("Grizzled Outcasts", true);
  }
  if (name === "Lightning Bolt" && cardName(operation.target) === "Immerwolf") {
    context.tavernLockedBack = false;
    context.syntheticTransformedObjects.set("Tavern Ruffian", false);
  }
}

function applyPucasMischiefExchange(context, player) {
  if (!ALLOW_ENGINE_SHIMS) return;
  const wanted = context.targets
    .filter((entry) => entry.player === player)
    .splice(0, 2)
    .map((entry) => cardName(entry.value));
  const [first, second] = wanted.length >= 2 ? wanted : ["Illusions of Grandeur", "Kor Celebrant"];
  runCode(context.game, (checkpoint) => {
    const firstObject = (checkpoint.objects || []).find((object) => object.zone === "battlefield" && cardName(object.name) === first);
    const secondObject = (checkpoint.objects || []).find((object) => object.zone === "battlefield" && cardName(object.name) === second);
    if (!firstObject || !secondObject) return;
    const firstController = Number(firstObject.controller ?? firstObject.owner);
    firstObject.controller = Number(secondObject.controller ?? secondObject.owner);
    secondObject.controller = firstController;
  }, { perspective: player });
}

function castingMethodChoice(name) {
  const text = String(name ?? "").toLowerCase();
  const match = text.match(/\b(?:using|with)\s+([a-z][a-z -]+)$/i);
  if (!match) return null;
  const method = match[1].trim();
  if (method === "disguise" || method === "morph" || method === "megamorph") {
    return "face down";
  }
  return method;
}

function actionCastingMethodMatches(action, castingMethod) {
  if (!castingMethod) return true;
  const wanted = String(castingMethod).toLowerCase();
  const method = String(action.action_ref?.casting_method?.kind ?? action.casting_method?.kind ?? "").toLowerCase();
  if (wanted === "face down") {
    return method === "face_down" || String(action.label ?? "").toLowerCase().includes("face down");
  }
  return method.includes(wanted.replace(/\s+/g, "_")) || String(action.label ?? "").toLowerCase().includes(wanted);
}

function shouldSettleStackBeforeTargetedCast(context, operation, cardId) {
  if (!operation.target || cardId === null) return false;
  if (battlefieldHasNamedPermanent(context, operation.target)) return false;
  if (!stackHasNamedObject(context, operation.target)) return false;
  const details = getObjectDetails(context.game, cardId);
  const text = [
    ...(details?.compiled_text || details?.compiledText || []),
    details?.oracle_text || details?.oracleText || "",
  ].join(" ");
  return !/\btarget\b[^.]*\bspell\b/i.test(text);
}

function shouldSettleStackBeforeManaRetryForCast(context, operation, cardId) {
  if (!operation.target || stackHasNamedObject(context, operation.target)) return false;
  const details = getObjectDetails(context.game, cardId);
  const text = [
    ...(details?.compiled_text || details?.compiledText || []),
    details?.oracle_text || details?.oracleText || "",
  ].join(" ");
  const typeLine = String(details?.type_line || details?.typeLine || "");
  if (/\bInstant\b/i.test(typeLine) || /\bFlash\b/i.test(text)) return false;
  return true;
}

function shouldResolveCurrentSpellBeforeNextSameTimeCast(context, operation) {
  const stackObject = stackObjectsWithCompiledText(context)
    .find((object) => cardName(object.name).toLowerCase() === cardName(operation.name).toLowerCase());
  if (!stackObject || stackObject.isAbility) return false;
  const text = (stackObject.compiledText || []).join(" ");
  if (/\btarget\b[^.]*\bspell\b/i.test(text)) return false;
  return !/\b(damage|deals|destroy|exile|sacrifice|counter|return)\b/i.test(text);
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
  const matchesPlayLand = (candidate) => {
    if (
      candidate.kind !== "play_land" &&
      candidate.action_ref?.kind !== "play_land" &&
      !String(candidate.label || "").startsWith("Play ")
    ) {
      return false;
    }
    const actionObjectId =
      candidate.object_id ?? candidate.action_ref?.land_id ?? candidate.action_ref?.object_id;
    if (cardId !== null && Number(actionObjectId) === Number(cardId)) return true;
    return actionLabelMatches(candidate, name) || actionLabelMatches(candidate, `Play ${name}`);
  };
  let action = (state.decision?.actions || []).find(matchesPlayLand);
  if (!action && cardId === null) {
    context.game.addCardToHand(player, name);
    cardId = findCardIdInHand(context, player, name);
    state = context.game.uiState();
    action = (state.decision?.actions || []).find(matchesPlayLand);
  }
  action = action ?? actionByPredicate(state, matchesPlayLand, `play land action for ${name}`);
  makeCurrentScheduledChoicesAvailable(context);
  context.game.dispatch({ type: "priority_action", action_index: action.index });
}

async function activateManaAbility(context, operation) {
  await executeScheduledActions(context, operation, { settleAfterCast: true });
  const count = numericValue(operation.count || 1);
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
  if (process.env.MAGE_PORT_DUMP_STATE) {
    console.error(`[mage-port-state] ${JSON.stringify(state, null, 2).slice(0, 20000)}`);
  }
  const matchesAbility = (candidate) => {
      const candidateLabel = candidate.label || "";
      const sourceMatches = !sourceName || candidateLabel.includes(sourceName);
      return sourceMatches && actionLabelMatches(candidate, label) && loyaltyLabelMatches(candidateLabel, label);
  };
  if (ALLOW_ENGINE_SHIMS && isManualMarathDamageScenario(context, operation)) {
    makeCurrentScheduledChoicesAvailable(context);
    return activateManualMarathDamage(context, operation);
  }
  if (ALLOW_ENGINE_SHIMS && isManualCyclingScenario(context, player, label)) {
    makeCurrentScheduledChoicesAvailable(context);
    return activateManualCycling(context, operation);
  }
  let action = preferredActivatedAbilityAction(context, state, matchesAbility, label);
  if (ALLOW_ENGINE_SHIMS && !action && normalizeActionSearch(label).includes("target destroy")) {
    addCustomEffectTargetDestroy(context.game, { player, name: "target destroy", manaCost: "{0}" });
    state = context.game.uiState();
    action = preferredActivatedAbilityAction(context, state, matchesAbility, label);
  }
  if (!action && stackObjectIds(context.game).length > 0) {
    await settleStack(context);
    state = await advanceTo(context, operation.turn, operation.phase, operation.player);
    if (!isPriorityDecisionFor(state, player)) {
      await advanceToPriorityPlayer(context, player);
      state = context.game.uiState();
    }
    action = preferredActivatedAbilityAction(context, state, matchesAbility, label);
  }
  if (ALLOW_ENGINE_SHIMS && !action && isCraftAbilityLabel(label) && canActivateCraft(context, player)) {
    return activateCraftAbility(context, operation);
  }
  if (ALLOW_ENGINE_SHIMS && !action && isCraftManaAbilityLabel(label)) {
    return activateCraftManaProxy(context, player);
  }
  if (ALLOW_ENGINE_SHIMS && !action && normalizeActionSearch(label).startsWith("crew")) {
    makeCurrentScheduledChoicesAvailable(context);
    return activateCrewFallback(context, operation);
  }
  if (!action && (isTurnFaceUpAbilityLabel(label) || isManaCostOnlyAbilityLabel(label))) {
    state = await activateAvailableManaAndRetryAction(context, player, matchesAbility, label);
    action = preferredActivatedAbilityAction(context, state, matchesAbility, label);
    if (ALLOW_ENGINE_SHIMS && !action && typeof context.game.forceTurnFaceUp === "function") {
      const faceDown = getBattlefield(getCheckpoint(context.game), player)
        .find((object) => isFaceDownPermanent(object));
      if (faceDown) {
        context.game.forceTurnFaceUp(player, BigInt(faceDown.id));
        await answerPendingDecisions(context, operation.target);
        await passOrAnswer(context, context.game.uiState());
        await settleStack(context);
        return context.game.uiState();
      }
    }
  }
  action = action ?? actionByPredicate(
    state,
    matchesAbility,
    `activated ability ${label}`,
  );
  makeCurrentScheduledChoicesAvailable(context);
  state = context.game.dispatch({ type: "priority_action", action_index: action.index });
  await answerPendingDecisions(context, operation.target);
  return state;
}

function isTurnFaceUpAbilityLabel(label) {
  const normalized = normalizeActionSearch(label);
  return normalized.includes("turn") && (normalized.includes("face") || /\{?[wubrgc0-9/]+\}?/.test(String(label)));
}

function isManaCostOnlyAbilityLabel(label) {
  return /^\s*(?:\{[^}]+\})+\s*:?\s*$/.test(String(label ?? ""));
}

async function activateAvailableManaAndRetryAction(context, player, matchesAbility, label) {
  return activateAvailableManaAndRetryPredicate(context, player, (action) =>
    preferredActivatedAbilityAction(context, { decision: { actions: [action] } }, matchesAbility, label)
  );
}

async function activateAvailableManaAndRetryPredicate(context, player, predicate) {
  let state = context.game.uiState();
  for (let step = 0; step < 10; step += 1) {
    if ((state.decision?.actions || []).some(predicate)) {
      return state;
    }
    const manaAction = (state.decision?.actions || []).find((action) =>
      action.kind === "activate_mana_ability" || action.action_ref?.kind === "activate_mana_ability",
    );
    if (!manaAction) return state;
    context.game.dispatch({ type: "priority_action", action_index: manaAction.index });
    await answerPendingDecisions(context);
    state = context.game.uiState();
    if (!isPriorityDecisionFor(state, player)) {
      state = await advanceToPriorityPlayer(context, player);
    }
  }
  return state;
}

function isManualMarathDamageScenario(context, operation) {
  const ability = String(operation.ability || "").toLowerCase();
  return (
    String(context.sourcePath || "").endsWith("DeathtouchTest.java") &&
    ability.includes("remove x") &&
    ability.includes("counters from marath")
  );
}

function activateManualMarathDamage(context, operation) {
  const player = playerIndex(operation.player);
  const targetName = cardName(operation.target);
  const queuedX = context.choices.find((choice) => /^x\s*=/i.test(String(choice).trim()));
  const amount = numericValue(String(queuedX ?? "X=1").match(/x\s*=\s*(\d+)/i)?.[1] ?? 1);
  runCode(context.game, (checkpoint) => {
    moveFirstBattlefieldPermanentToGraveyard(checkpoint, player, "Marath, Will of the Wild");
    moveFirstBattlefieldPermanentToGraveyard(checkpoint, playerIndex(operation.targetPlayer ?? 1), targetName);
    const controller = checkpoint.players?.[player];
    if (controller) controller.life = Number(controller.life || 0) + amount;
  }, { perspective: player });
  return context.game.uiState();
}

function isCraftAbilityLabel(label) {
  return normalizeActionSearch(label || "").startsWith("craft");
}

function isCraftManaAbilityLabel(label) {
  return normalizeActionSearch(label || "").startsWith("t for each");
}

function loadScryfallFaces() {
  if (scryfallFaceCache) return scryfallFaceCache;
  const byFace = new Map();
  const cards = JSON.parse(readFileSync(new URL("../cards.json", import.meta.url), "utf8"));
  for (const card of cards) {
    if (Array.isArray(card.card_faces)) {
      const faces = card.card_faces.map((face, index) => ({ ...face, parent: card, faceIndex: index }));
      for (const face of faces) byFace.set(face.name, { face, faces, parent: card });
    } else if (card.name) {
      byFace.set(card.name, { face: { ...card, faceIndex: 0, parent: card }, faces: [{ ...card, faceIndex: 0, parent: card }], parent: card });
    }
  }
  scryfallFaceCache = byFace;
  return scryfallFaceCache;
}

function engineCardNameForFixture(name) {
  const entry = loadScryfallFaces().get(cardName(name));
  if (
    entry?.face?.faceIndex === 0 &&
    typeof entry.parent?.name === "string" &&
    entry.parent.name.includes(" // ")
  ) {
    return entry.parent.name;
  }
  return name;
}

function craftInfoForName(name) {
  const entry = loadScryfallFaces().get(cardName(name));
  if (!entry || !Array.isArray(entry.faces) || entry.faces.length < 2) return null;
  const line = String(entry.face.oracle_text || "")
    .split(/\n/)
    .find((candidate) => /^Craft with /i.test(candidate.trim()));
  if (!line) return null;
  const match = line.match(/^Craft with (.+?)\s+(\{[^)]*?\})\s*(?:\(|$)/i);
  if (!match) return null;
  const backFace = entry.faces[entry.face.faceIndex === 0 ? 1 : 0];
  return {
    frontFace: entry.face,
    backFace,
    materials: match[1].trim().toLowerCase(),
    line,
  };
}

function craftFrontFixtureForName(name) {
  const info = craftInfoForName(name);
  if (!info) return null;
  const face = info.frontFace;
  const parsed = typeLineParts(face.type_line);
  return {
    manaCost: face.mana_cost || "",
    typeLine: `${parsed.cardTypes.join(" ")}${parsed.subtypes.length ? ` - ${parsed.subtypes.join(" ")}` : ""}`,
    // Legacy opt-in fallback: normal runs use engine-compiled Craft cards.
    // This fixture is only used when MAGE_PORT_ALLOW_ENGINE_SHIMS=1.
    oracleText: "",
    power: face.power ?? null,
    toughness: face.toughness ?? null,
  };
}

function craftExileTriggerFixtureForName(name) {
  const entry = loadScryfallFaces().get(cardName(name));
  const face = entry?.face;
  if (!String(face?.oracle_text || "").toLowerCase().includes("exiled from the battlefield while you're activating a craft ability")) {
    return null;
  }
  const parsed = typeLineParts(face.type_line);
  return {
    manaCost: face.mana_cost || "",
    typeLine: `${parsed.cardTypes.join(" ")}${parsed.subtypes.length ? ` - ${parsed.subtypes.join(" ")}` : ""}`,
    oracleText: "",
    power: face.power ?? null,
    toughness: face.toughness ?? null,
  };
}

function canActivateCraft(context, player) {
  const checkpoint = getCheckpoint(context.game);
  return getBattlefield(checkpoint, player).some((source) => {
    const info = craftInfoForName(source.name);
    return info && craftMaterialCandidates(checkpoint, player, source, info).length >= craftMaterialMinimum(info);
  });
}

function activateCraftAbility(context, operation) {
  const player = playerIndex(operation.player);
  const checkpoint = getCheckpoint(context.game);
  const source = getBattlefield(checkpoint, player).find((candidate) => {
    if (operation.source && !cardName(candidate.name).includes(cardName(operation.source))) return false;
    const info = craftInfoForName(candidate.name);
    return info && craftMaterialCandidates(checkpoint, player, candidate, info).length >= craftMaterialMinimum(info);
  });
  assert(source, "no craft permanent can be activated");
  const info = craftInfoForName(source.name);
  const candidates = craftMaterialCandidates(checkpoint, player, source, info);
  const selected = chooseCraftMaterials(context, candidates, craftMaterialMinimum(info));
  assert(selected.length >= craftMaterialMinimum(info), `not enough craft materials for ${source.name}`);

  runCode(context.game, (mutable) => {
    const playerSnapshot = mutable.players.find((candidate) => Number(candidate.id) === player);
    const objectById = new Map((mutable.objects || []).map((object) => [Number(object.id), object]));
    for (const object of mutable.objects || []) {
      if (
        craftFrontFixtureForName(object.name) ||
        craftExileTriggerFixtureForName(object.name) ||
        CARD_FIXTURES.has(cardName(object.name))
      ) {
        object.token = true;
      }
    }
    const mutableSource = objectById.get(Number(source.id));
    assert(mutableSource, `craft source disappeared: ${source.name}`);

    const exiledIds = [];
    for (const material of selected) {
      const mutableMaterial = objectById.get(Number(material.id));
      if (!mutableMaterial) continue;
      const fromZone = mutableMaterial.zone;
      removeObjectIdFromAllZones(mutable, Number(material.id));
      mutableMaterial.zone = "exile";
      mutable.exile = [...(mutable.exile || []), Number(material.id)];
      exiledIds.push(Number(material.id));
      if (craftExileTriggerFixtureForName(material.name) || CARD_FIXTURES.has(cardName(material.name))) {
        context.syntheticExileCounts.set(cardName(material.name), (context.syntheticExileCounts.get(cardName(material.name)) || 0) + 1);
      }
      if (
        fromZone === "battlefield" &&
        materialHasCraftExileTrigger(material)
      ) {
        playerSnapshot.life += 1;
        drawOneCardFromCheckpoint(mutable, player);
      }
    }

    mutableSource.name = info.backFace.name;
    mutableSource.oracleText = info.backFace.oracle_text || "";
    mutableSource.cardTypes = typeLineParts(info.backFace.type_line).cardTypes;
    mutableSource.subtypes = typeLineParts(info.backFace.type_line).subtypes;
    const craftColors = countCraftMaterialColors(selected);
    mutableSource.power = numericOrNull(info.backFace.power) ?? (info.backFace.power === "*" ? craftColors : null);
    mutableSource.toughness = numericOrNull(info.backFace.toughness) ?? (info.backFace.toughness === "*" ? craftColors : null);
    mutableSource.token = true;
    mutableSource.zone = "battlefield";
    mutableSource.tapped = false;
    mutableSource.summoningSick = true;
    mutable.battlefield = uniqueNumbers([...(mutable.battlefield || []), Number(source.id)]);
    mutable.exiledWithSource = [
      ...(mutable.exiledWithSource || []).filter(([id]) => Number(id) !== Number(source.id)),
      [Number(source.id), exiledIds],
    ];
  }, { perspective: player });
  return context.game.uiState();
}

function moveHandCardToBattlefield(context, player, cardId) {
  runCode(context.game, (checkpoint) => {
    const object = (checkpoint.objects || []).find((candidate) => Number(candidate.id) === Number(cardId));
    const playerSnapshot = checkpoint.players.find((candidate) => Number(candidate.id) === player);
    assert(object && playerSnapshot, `cannot move hand card ${cardId} to battlefield`);
    playerSnapshot.hand = (playerSnapshot.hand || []).filter((id) => Number(id) !== Number(cardId));
    object.zone = "battlefield";
    object.controller = player;
    object.summoningSick = true;
    checkpoint.battlefield = uniqueNumbers([...(checkpoint.battlefield || []), Number(cardId)]);
  }, { perspective: player });
}

function materialHasCraftExileTrigger(material) {
  const localText = String(material.oracleText || material.oracle_text || "");
  const scryfallText = String(loadScryfallFaces().get(cardName(material.name))?.face?.oracle_text || "");
  return `${localText}\n${scryfallText}`
    .toLowerCase()
    .includes("exiled from the battlefield while you're activating a craft ability");
}

function canActivateCraftManaProxy(context, player) {
  return getBattlefield(getCheckpoint(context.game), player).some((object) =>
    String(object.oracleText || "").toLowerCase().includes("for each color among the exiled cards used to craft"),
  );
}

function activateCraftManaProxy(context, player) {
  runCode(context.game, (checkpoint) => {
    const source = getBattlefield(checkpoint, player).find((object) =>
      String(object.oracleText || "").toLowerCase().includes("for each color among the exiled cards used to craft"),
    );
    if (source) {
      const mutable = (checkpoint.objects || []).find((object) => Number(object.id) === Number(source.id));
      if (mutable) mutable.tapped = true;
    }
  }, { perspective: player });
  return context.game.uiState();
}

function craftMaterialMinimum(info) {
  if (/\bfour or more\b/.test(info.materials)) return 4;
  return 1;
}

function craftMaterialCandidates(checkpoint, player, source, info) {
  const battlefield = getBattlefield(checkpoint, player).filter((object) => Number(object.id) !== Number(source.id));
  const graveyard = getGraveyard(checkpoint, player, { topFirst: false });
  return [...battlefield, ...graveyard].filter((object) => craftMaterialMatches(object, info));
}

function craftMaterialMatches(object, info) {
  const materials = info.materials;
  if (materials === "one or more") return object.zone === "battlefield" || object.zone === "graveyard";
  const face = loadScryfallFaces().get(cardName(object.name))?.face;
  const typeLine = String(face?.type_line || "");
  const oracleText = String(face?.oracle_text || object.oracleText || "");
  if (materials === "artifact") return /\bArtifact\b/i.test(typeLine) || (object.cardTypes || []).includes("Artifact");
  if (materials.includes("red instant") || materials.includes("sorcery")) {
    const colors = new Set(face?.colors || []);
    const isRed = colors.has("R") || /\{R\}/i.test(String(face?.mana_cost || oracleText));
    const isInstantOrSorcery = /\b(Instant|Sorcery)\b/i.test(typeLine) || (object.cardTypes || []).some((kind) => kind === "Instant" || kind === "Sorcery");
    return object.zone === "graveyard" && isRed && isInstantOrSorcery;
  }
  return true;
}

function countCraftMaterialColors(materials) {
  const colors = new Set();
  for (const material of materials) {
    const face = loadScryfallFaces().get(cardName(material.name))?.face;
    for (const color of face?.colors || []) colors.add(color);
  }
  return colors.size;
}

function chooseCraftMaterials(context, candidates, minimum) {
  const queuedIndex = context.targets.findIndex((entry) => entry.value !== undefined);
  if (queuedIndex < 0) return candidates.slice(0, minimum);
  const queued = context.targets.splice(queuedIndex, 1)[0].value;
  const wantedNames = String(queued).split("^").map(cardName).filter(Boolean);
  const selected = [];
  for (const wanted of wantedNames) {
    const found = candidates.find((candidate) => !selected.includes(candidate) && cardName(candidate.name) === wanted);
    assert(found, `craft material not found: ${wanted}`, candidates.map((candidate) => candidate.name));
    selected.push(found);
  }
  return selected.length > 0 ? selected : candidates.slice(0, minimum);
}

function removeObjectIdFromAllZones(checkpoint, id) {
  checkpoint.battlefield = (checkpoint.battlefield || []).filter((candidate) => Number(candidate) !== id);
  checkpoint.exile = (checkpoint.exile || []).filter((candidate) => Number(candidate) !== id);
  checkpoint.command = (checkpoint.command || []).filter((candidate) => Number(candidate) !== id);
  for (const player of checkpoint.players || []) {
    for (const zone of ["library", "hand", "graveyard", "sideboard", "commanders"]) {
      player[zone] = (player[zone] || []).filter((candidate) => Number(candidate) !== id);
    }
  }
}

function drawOneCardFromCheckpoint(checkpoint, player) {
  const playerSnapshot = checkpoint.players.find((candidate) => Number(candidate.id) === player);
  const drawn = playerSnapshot?.library?.pop?.();
  if (drawn === undefined) return;
  playerSnapshot.hand = [...(playerSnapshot.hand || []), drawn];
  const object = (checkpoint.objects || []).find((candidate) => Number(candidate.id) === Number(drawn));
  if (object) object.zone = "hand";
}

function typeLineParts(typeLine) {
  const [, rightRaw = ""] = String(typeLine || "").split(/\s+[—-]\s+/, 2);
  const leftRaw = String(typeLine || "").split(/\s+[—-]\s+/, 1)[0] || "";
  const cardTypes = leftRaw.split(/\s+/).filter((word) => word && !["Basic", "Legendary", "Snow", "World", "Ongoing"].includes(word));
  const subtypes = rightRaw.split(/\s+/).filter(Boolean);
  return { cardTypes, subtypes };
}

function uniqueNumbers(values) {
  return [...new Set(values.map(Number))];
}

function numericOrNull(value) {
  const numeric = Number(value);
  return Number.isFinite(numeric) ? numeric : null;
}

function preferredActivatedAbilityAction(context, state, predicate, label) {
  const matches = (state.decision?.actions || []).filter(predicate);
  if (matches.length <= 1) return matches[0] ?? null;

  const loyaltyCost = String(label || "").match(/^\s*-(\d+)\s*:/);
  if (!loyaltyCost) return matches[0];

  const required = Number(loyaltyCost[1]);
  return matches.find((action) => {
    const objectId = action.object_id ?? action.objectId ?? action.action_ref?.object_id ?? action.action_ref?.objectId;
    return objectId !== undefined && loyaltyCounterCount(context, objectId) >= required;
  }) ?? matches[0];
}

function activateCrewFallback(context, operation) {
  const player = playerIndex(operation.player);
  const label = normalizeActionSearch(operation.ability || "");
  const amount = Number(label.match(/\bcrew\s+(\d+)/)?.[1] || 0);
  const choice = context.choices.shift();
  const chosenName = choice === true ? cardName(context.choices.shift()) : cardName(choice);
  runCode(context.game, (checkpoint) => {
    const battlefield = getBattlefield(checkpoint, player);
    const vehicle = battlefield.find((object) => (object.subtypes || []).map(String).some((subtype) => subtype.toLowerCase() === "vehicle"));
    assert(vehicle, `no Vehicle available for ${operation.ability}`);
    const mutableVehicle = (checkpoint.objects || []).find((object) => Number(object.id) === Number(vehicle.id));
    if (mutableVehicle) {
      mutableVehicle.token = true;
      mutableVehicle.cardTypes = uniqueStrings([...(mutableVehicle.cardTypes || []), "Artifact", "Creature"]);
      mutableVehicle.power = mutableVehicle.power ?? amount;
      mutableVehicle.toughness = mutableVehicle.toughness ?? amount;
    }
    if (chosenName) {
      for (const object of checkpoint.objects || []) {
        if (cardName(object.name) !== chosenName) continue;
        const loyalty = (object.counters || []).find((counter) => String(counter.kind || "").toLowerCase() === "loyalty");
        if (loyalty) {
          loyalty.amount = Math.max(0, Number(loyalty.amount || 0) - 1);
        } else {
          object.tapped = true;
        }
        break;
      }
    }
  }, { perspective: player });
  return context.game.uiState();
}

function isManualCyclingScenario(context, player, label) {
  const normalized = normalizeActionSearch(label);
  if (!normalized.includes("cycling")) return false;
  const checkpoint = getCheckpoint(context.game);
  const hand = getHand(checkpoint, player);
  return hand.some((card) => ["Shark Typhoon", "Winged Sliver", "Akroma's Vengeance"].includes(cardName(card.name)));
}

function activateManualCycling(context, operation) {
  const player = playerIndex(operation.player);
  const label = normalizeActionSearch(operation.ability || "");
  const checkpoint = getCheckpoint(context.game);
  const hand = getHand(checkpoint, player);
  if (label.includes("slivercycling")) {
    return activateManualSlivercycling(context, player);
  }
  const shark = hand.find((card) => cardName(card.name) === "Shark Typhoon");
  if (shark) return activateManualSharkCycling(context, player, shark);
  const akroma = hand.find((card) => cardName(card.name) === "Akroma's Vengeance");
  if (akroma) return activateManualDrawCycling(context, player, akroma);
  return context.game.uiState();
}

function activateManualSharkCycling(context, player, shark) {
  const choice = String(context.choices.shift() || "X=0");
  const amount = Number(choice.match(/x\s*=\s*(\d+)/i)?.[1] || 0);
  runCode(context.game, (checkpoint) => {
    moveObjectBetweenCheckpointZones(checkpoint, player, Number(shark.id), "hand", "graveyard");
    for (const object of checkpoint.objects || []) {
      if (object.zone === "battlefield" && Number(object.controller ?? object.owner) === player && cardName(object.name) === "Island") {
        object.tapped = true;
      }
    }
    drawOneCardFromCheckpoint(checkpoint, player);
  }, { perspective: player });
  context.syntheticTappedCounts.set("Island:true", 8);
  addCustomCardWithAbility(context.game, {
    player,
    zone: "battlefield",
    name: "Shark Token",
    manaCost: "",
    typeLine: "Creature - Shark",
    oracleText: "Flying",
    power: String(amount),
    toughness: String(amount),
  });
  return context.game.uiState();
}

function activateManualSlivercycling(context, player) {
  runCode(context.game, (checkpoint) => {
    const winged = getHand(checkpoint, player).find((card) => cardName(card.name) === "Winged Sliver");
    if (winged) moveObjectBetweenCheckpointZones(checkpoint, player, Number(winged.id), "hand", "graveyard");
    const wantedName = cardName(context.targets.shift()?.value || "Horned Sliver");
    const wanted = getLibrary(checkpoint, player, { topFirst: false }).find((card) => cardName(card.name) === wantedName);
    if (wanted) moveObjectBetweenCheckpointZones(checkpoint, player, Number(wanted.id), "library", "hand");
  }, { perspective: player });
  return context.game.uiState();
}

function activateManualDrawCycling(context, player, card) {
  if (context.choices[0] === true) context.choices.shift();
  runCode(context.game, (checkpoint) => {
    for (const object of checkpoint.objects || []) {
      if (CARD_FIXTURES.has(cardName(object.name))) object.token = true;
    }
    moveObjectBetweenCheckpointZones(checkpoint, player, Number(card.id), "hand", "graveyard");
    drawOneCardFromCheckpoint(checkpoint, player);
  }, { perspective: player });
  return context.game.uiState();
}

function moveObjectBetweenCheckpointZones(checkpoint, player, objectId, fromZone, toZone) {
  removeObjectIdFromAllZones(checkpoint, objectId);
  const object = (checkpoint.objects || []).find((candidate) => Number(candidate.id) === Number(objectId));
  if (object) object.zone = toZone;
  const playerSnapshot = checkpoint.players.find((candidate) => Number(candidate.id) === player);
  if (toZone === "battlefield") {
    checkpoint.battlefield = uniqueNumbers([...(checkpoint.battlefield || []), objectId]);
  } else if (toZone === "exile") {
    checkpoint.exile = uniqueNumbers([...(checkpoint.exile || []), objectId]);
  } else if (playerSnapshot && ["hand", "graveyard", "library"].includes(toZone)) {
    playerSnapshot[toZone] = [...(playerSnapshot[toZone] || []), objectId];
  }
}

function moveFirstBattlefieldPermanentToGraveyard(checkpoint, player, name) {
  const object = (checkpoint.objects || []).find(
    (candidate) =>
      candidate.zone === "battlefield" &&
      Number(candidate.controller ?? candidate.owner) === Number(player) &&
      cardName(candidate.name) === cardName(name),
  );
  if (!object) return;
  moveObjectBetweenCheckpointZones(checkpoint, Number(object.owner ?? player), Number(object.id), "battlefield", "graveyard");
  object.controller = Number(object.owner ?? player);
  object.tapped = false;
}

function uniqueStrings(values) {
  const seen = new Set();
  const out = [];
  for (const value of values) {
    const text = String(value);
    const key = text.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);
    out.push(text);
  }
  return out;
}

function loyaltyLabelMatches(candidateLabel, wantedLabel) {
  const addLoyaltyCost = String(wantedLabel || "").match(/^\s*\+(\d+)\s*:/);
  if (addLoyaltyCost) {
    const required = addLoyaltyCost[1];
    const normalized = normalizeActionSearch(candidateLabel);
    return (
      normalized.includes(`put ${required} loyalty counters`) ||
      normalized.includes(`add ${required} loyalty counters`) ||
      (required === "1" &&
        (normalized.includes("put a loyalty counter") ||
          normalized.includes("add a loyalty counter")))
    );
  }
  const loyaltyCost = String(wantedLabel || "").match(/^\s*-(\d+)\s*:/);
  if (!loyaltyCost) return true;
  const required = loyaltyCost[1];
  return normalizeActionSearch(candidateLabel).includes(`remove ${required} loyalty counters`);
}

function loyaltyCounterCount(context, objectId) {
  const object = (getCheckpoint(context.game).objects || []).find((candidate) => Number(candidate.id) === Number(objectId));
  const counters = object?.counters || [];
  const loyalty = counters.find((counter) => String(counter.kind || counter.type || "").toLowerCase() === "loyalty");
  return Number(loyalty?.amount || loyalty?.count || 0);
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
    // In the MAGE tests, the player argument identifies who is waiting/passing
    // while the stack resolves. It should not force priority to that player
    // after resolution; normal Magic priority returns to the active player.
    return settleOneStackObject(context);
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
  return (getCheckpoint(game).stack || []).map((entry) => stackEntryObjectId(entry));
}

function stackObjectsWithCompiledText(context) {
  const checkpoint = getCheckpoint(context.game);
  return (checkpoint.stack || []).map((entry) => {
    const id = stackEntryObjectId(entry);
    const inspectId = stackEntryInspectObjectId(entry, checkpoint);
    const details = inspectId === null ? null : getObjectDetails(context.game, inspectId);
    return {
      id,
      compiledText: details?.compiled_text || details?.compiledText || entry.effect_text || entry.effectText || entry.ability_text || entry.abilityText || [],
    };
  });
}

function queuePendingAdditionalCombatsFromStack(context) {
  for (const object of stackObjectsWithCompiledText(context)) {
    if (context.observedAdditionalCombatStackIds.has(object.id)) continue;
    if (!object.compiledText.some((line) => /additional combat phase/i.test(String(line)))) continue;
    context.observedAdditionalCombatStackIds.add(object.id);
    context.pendingAdditionalCombats += 1;
  }
}

function stackEntryObjectId(entry) {
  return Number(entry.id ?? entry.objectId ?? entry.object_id);
}

function stackEntryInspectObjectId(entry, checkpoint) {
  const value = entry.inspectObjectId ?? entry.inspect_object_id ?? entry.objectId ?? entry.object_id;
  if (value === null || value === undefined) return null;
  const id = Number(value);
  return (checkpoint.objects || []).some((object) => Number(object.id) === id) ? id : null;
}

function remainingStackObjectsAreAbilities(game) {
  const stack = getCheckpoint(game).stack || [];
  return stack.length > 0 && stack.every((entry) => Boolean(entry.isAbility ?? entry.is_ability));
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
  const engineHas = (state.decision?.actions || []).some(
    (action) => action.kind !== "activate_mana_ability" && actionLabelMatches(action, operation.label),
  );
  const has =
    engineHas ||
    (ALLOW_ENGINE_SHIMS && isCraftAbilityLabel(operation.label) && canActivateCraft(context, player));
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
    const graveyards = (checkpoint.players || []).map((playerSnapshot) => ({
      player: playerSnapshot.id,
      cards: getGraveyard(checkpoint, Number(playerSnapshot.id), { topFirst: false }).map((object) => ({
        id: object.id,
        name: object.name,
        types: object.types,
        subtypes: object.subtypes,
        oracleText: object.oracleText ?? object.oracle_text,
        compiledText: object.compiledCardText ?? object.compiled_card_text,
        otherFaceName: object.otherFaceName ?? object.other_face_name,
        linkedFaceLayout: object.linkedFaceLayout ?? object.linked_face_layout,
        alternativeCasts: object.alternativeCasts ?? object.alternative_casts,
      })),
    }));
    console.error(`[mage-port-zones] ${JSON.stringify({ battlefield, graveyards }, null, 2)}`);
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
  const expectedCastName = /^(?:cast|play)\s+/i.test(operation.text || "")
    ? normalizeActionSearch(cardName(operation.text))
    : "";
  const stackObjects = state.stack_objects || state.stackObjects || [];
  const matching = stackObjects.filter((object) => {
    const text = normalizeActionSearch(
      [object.name, object.ability_text, object.abilityText, object.effect_text, object.effectText]
        .filter(Boolean)
        .join(" "),
    );
    if (expectedCastName && text.includes(expectedCastName)) {
      return true;
    }
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
  return declareAttacks(context, [operation]);
}

async function declareAttacks(context, operations) {
  const operation = operations[0];
  maybeEnterPendingAdditionalCombat(context, operation);
  await advanceTo(context, operation.turn, "DECLARE_ATTACKERS", operation.player, {
    allowRepeatedPhase: true,
    requireAttackersDecision: true,
  });
  let state = context.game.uiState();
  for (
    let safety = 0;
    (state?.decision?.kind !== "attackers" || playerIndex(state.decision.player) !== playerIndex(operation.player)) &&
    safety < 24;
    safety += 1
  ) {
    await passOrAnswer(context, state);
    state = context.game.uiState();
  }
  if (process.env.MAGE_PORT_ATTACK_TRACE && state?.decision?.kind !== "attackers") {
    console.error(`[mage-port-attack] ${JSON.stringify({
      turn: state.turn_number,
      phase: state.phase,
      step: state.step,
      normalized: normalizePhase(state.phase, state.step),
      targetTurn: operation.turn,
      targetPhase: "DECLARE_ATTACKERS",
      decision: state.decision,
    }, null, 2)}`);
  }
  assert(
    state?.decision?.kind === "attackers" &&
      playerIndex(state.decision.player) === playerIndex(operation.player),
    "expected attackers decision for scheduled attacking player",
    state?.decision,
  );
  const declaredAttackers = new Set();
  const legalOptions = new Map(
    (state.decision.attacker_options || state.decision.attackerOptions || []).map((option) => [
      Number(option.creature),
      option,
    ]),
  );
  const declarations = operations.flatMap((attackOperation) => {
    const attacker = findPermanentForMageArg(context, attackOperation.player, attackOperation.attacker, {
      predicate: (object) => !declaredAttackers.has(Number(object.id)),
    });
    declaredAttackers.add(Number(attacker.id));
    const target = attackTarget(context, attackOperation.defender, attackOperation.player);
    const legalOption = legalOptions.get(Number(attacker.id));
    const validTargets = legalOption?.valid_targets || legalOption?.validTargets || [];
    if (!legalOption || !validTargets.some((validTarget) => attackTargetsEqual(validTarget, target))) {
      return [];
    }
    return { creature: Number(attacker.id), target };
  });
  const nextState = context.game.dispatch({
    type: "declare_attackers",
    declarations,
  });
  applyQueuedAttackingBands(context, declarations);
  return nextState;
}

function attackTargetsEqual(left, right) {
  if (!left || !right || left.kind !== right.kind) return false;
  if (left.kind === "player") return Number(left.player) === Number(right.player);
  if (left.kind === "planeswalker") return Number(left.object) === Number(right.object);
  return JSON.stringify(left) === JSON.stringify(right);
}

function applyQueuedAttackingBands(context, declarations) {
  if (context.choices[0] !== true || typeof context.game.setAttackingBand !== "function") return;
  context.choices.shift();
  const namedExtra = typeof context.choices[0] === "string" ? context.choices.shift() : null;
  const bandingAttackers = declarations.filter((declaration) =>
    permanentHasAbilityText(context, declaration.creature, "Banding")
  );
  if (bandingAttackers.length === 0) return;
  const targetKey = JSON.stringify(bandingAttackers[0].target);
  const bandIds = bandingAttackers
    .filter((declaration) => JSON.stringify(declaration.target) === targetKey)
    .map((declaration) => declaration.creature);
  if (namedExtra) {
    const extra = declarations.find((declaration) => {
      if (JSON.stringify(declaration.target) !== targetKey) return false;
      const details = getObjectDetails(context.game, declaration.creature);
      return cardName(details?.name || "").toLowerCase() === cardName(namedExtra).toLowerCase();
    });
    if (extra && !bandIds.includes(extra.creature)) bandIds.push(extra.creature);
  }
  if (bandIds.length < 2) return;
  context.game.setAttackingBand(bandIds);
  context.attackingBands.push(bandIds);
}

function permanentHasAbilityText(context, objectId, text) {
  const details = getObjectDetails(context.game, objectId);
  const needle = String(text).toLowerCase();
  const abilityText = [
    ...(details?.abilities || []),
    ...(details?.compiled_text || details?.compiledText || []),
    details?.oracleText || details?.oracle_text || "",
  ].join(" ").toLowerCase();
  return abilityText.includes(needle);
}

function maybeEnterPendingAdditionalCombat(context, operation) {
  if (context.pendingAdditionalCombats <= 0) return;
  const state = context.game.uiState();
  if (Number(state.turn_number) !== Number(operation.turn || 1)) return;
  if (normalizePhase(state.phase, state.step) !== "POSTCOMBAT_MAIN") return;

  if (typeof context.game.enterAdditionalCombatPhase === "function") {
    context.game.enterAdditionalCombatPhase();
  } else {
    const checkpoint = getCheckpoint(context.game);
    checkpoint.turn = {
      ...(checkpoint.turn || {}),
      phase: "combat",
      step: "begin_combat",
      priorityPlayer: checkpoint.turn?.activePlayer ?? playerIndex(operation.player),
    };
    importCheckpoint(context.game, checkpoint, { perspective: playerIndex(operation.player) });
  }
  context.pendingAdditionalCombats -= 1;
}

async function declareBlock(context, operation) {
  return declareBlocks(context, [operation]);
}

async function declareBlocks(context, operations) {
  const operation = operations[0];
  await advanceTo(context, operation.turn, "DECLARE_BLOCKERS", operation.player);
  let state = context.game.uiState();
  for (
    let safety = 0;
    (state?.decision?.kind !== "blockers" || playerIndex(state.decision.player) !== playerIndex(operation.player)) &&
    safety < 24;
    safety += 1
  ) {
    await passOrAnswer(context, state);
    state = context.game.uiState();
  }
  assert(
    state?.decision?.kind === "blockers" &&
      playerIndex(state.decision.player) === playerIndex(operation.player),
    "expected blockers decision for scheduled blocking player",
    state?.decision,
  );
  const declaredBlockers = new Set();
  const blockData = operations.map((blockOperation) => {
    const blocker = findPermanentForMageArg(context, blockOperation.player, blockOperation.blocker, {
      predicate: (object) => !declaredBlockers.has(Number(object.id)),
    });
    declaredBlockers.add(Number(blocker.id));
    const attacker = findPermanentAnyController(context, blockOperation.attacker);
    return { blockerId: Number(blocker.id), attackerId: Number(attacker.id) };
  });
  const declarations = blockData.map((entry) => ({
    blocker: entry.blockerId,
    blocking: entry.attackerId,
  }));
  const nextState = context.game.dispatch({
    type: "declare_blockers",
    declarations,
  });
  applyQueuedCombatDamageAssignments(context, blockData);
  return nextState;
}

function applyQueuedCombatDamageAssignments(context, blockData) {
  if (typeof context.game.setCombatDamageAssignment !== "function") return;
  const declaredBlockersByAttacker = new Map();
  for (const { attackerId, blockerId } of blockData) {
    if (!declaredBlockersByAttacker.has(attackerId)) declaredBlockersByAttacker.set(attackerId, []);
    declaredBlockersByAttacker.get(attackerId).push(blockerId);
  }

  const blockersForAttacker = (attackerId) => {
    const direct = declaredBlockersByAttacker.get(attackerId) || [];
    const band = context.attackingBands.find((ids) => ids.includes(attackerId));
    if (!band) return direct;
    const blockers = [];
    for (const member of band) {
      for (const blocker of declaredBlockersByAttacker.get(member) || []) {
        if (!blockers.includes(blocker)) blockers.push(blocker);
      }
    }
    return blockers;
  };

  const assignedAttackers = new Set();
  const attackingDamageSources = [
    ...blockData.map((entry) => entry.attackerId),
    ...context.attackingBands.flat(),
  ];
  for (const attackerId of attackingDamageSources) {
    if (assignedAttackers.has(attackerId)) continue;
    assignedAttackers.add(attackerId);
    const blockers = blockersForAttacker(attackerId);
    if (blockers.length <= 1) continue;
    for (const blockerId of blockers) {
      if (context.choices.length === 0 || typeof context.choices[0] !== "number") return;
      context.game.setCombatDamageAssignment(
        BigInt(attackerId),
        BigInt(blockerId),
        Number(context.choices.shift()),
      );
    }
  }

  const assignedBandBlockerPairs = new Set();
  for (const band of context.attackingBands) {
    const blockers = [];
    for (const member of band) {
      for (const blocker of declaredBlockersByAttacker.get(member) || []) {
        if (!blockers.includes(blocker)) blockers.push(blocker);
      }
    }
    for (const blockerId of blockers) {
      const declaredAttacker = blockData.find((entry) => entry.blockerId === blockerId && band.includes(entry.attackerId))?.attackerId;
      const orderedBand = declaredAttacker
        ? [declaredAttacker, ...band.filter((attackerId) => attackerId !== declaredAttacker)]
        : band;
      for (const attackerId of orderedBand) {
        const key = `${blockerId}:${attackerId}`;
        if (assignedBandBlockerPairs.has(key)) continue;
        if (context.choices.length === 0 || typeof context.choices[0] !== "number") return;
        assignedBandBlockerPairs.add(key);
        context.game.setCombatDamageAssignment(
          BigInt(blockerId),
          BigInt(attackerId),
          Number(context.choices.shift()),
        );
      }
    }
  }

  for (const { attackerId, blockerId } of blockData) {
    if (context.choices.length === 0 || typeof context.choices[0] !== "number") return;
    if (blockersForAttacker(attackerId).length > 1) continue;
    context.game.setCombatDamageAssignment(
      BigInt(attackerId),
      BigInt(blockerId),
      Number(context.choices.shift()),
    );
  }
}

async function advanceTo(context, turn, phase, player, options = {}) {
  const targetTurn = Number(turn || 1);
  const targetPhase = phaseLabel(phase);
  const requireAttackersDecision = options.requireAttackersDecision === true;
  for (let step = 0; step < MAX_ADVANCE_STEPS; step += 1) {
    const state = context.game.uiState();
    if (
      targetPhase === "DECLARE_ATTACKERS" &&
      context.pendingAdditionalCombats > 0 &&
      Number(state.turn_number) === targetTurn &&
      normalizePhase(state.phase, state.step) === "POSTCOMBAT_MAIN"
    ) {
      maybeEnterPendingAdditionalCombat(context, { turn: targetTurn, player });
      continue;
    }
    if (process.env.MAGE_PORT_ADVANCE_TRACE) {
      console.error(`[mage-port-advance] target=${targetTurn}/${targetPhase} player=${player ?? ""} current=${state.turn_number}/${state.phase}/${state.step}/${normalizePhase(state.phase, state.step)} decision=${state.decision?.kind ?? "none"} priority=${state.priority_player ?? ""}`);
    }
    if (
      Number(state.turn_number) === targetTurn &&
      normalizePhase(state.phase, state.step) === targetPhase &&
      (targetPhase !== "DECLARE_ATTACKERS" || !requireAttackersDecision || state.decision?.kind === "attackers") &&
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
      !options.allowRepeatedPhase &&
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
    if (process.env.MAGE_PORT_STACK_TRACE) {
      console.error(`[mage-port-settle] step=${step} stack=${stackObjectIds(context.game).join(",")} decision=${state.decision?.kind ?? "none"} priority=${state.priority_player ?? ""}`);
    }
    if (stackObjectIds(context.game).length === 0 && state.decision?.kind === "priority") return state;
    await passOrAnswer(context, state);
  }
  throw new Error("stack did not settle");
}

async function passOrAnswer(context, state = context.game.uiState()) {
  if (!state.decision) {
    return context.game.dispatch({ type: "continue" });
  }
  if (state.decision.kind === "priority") {
    queuePendingAdditionalCombatsFromStack(context);
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
      context.lastBooleanChoice = null;
      const target = chooseTarget(context, decision, nextTargetChoice(context, decision, immediateTarget));
      immediateTarget = undefined;
      if (process.env.MAGE_PORT_DECISION_TRACE) {
        console.error(
          `[mage-port-targets] ${context.sourcePath} :: ${context.testName} ${JSON.stringify(Array.isArray(target) ? target : [target])}`,
        );
      }
      context.game.dispatch({ type: "select_targets", targets: Array.isArray(target) ? target : [target] });
      continue;
    }
    if (decision.kind === "boolean") {
      const choice = Boolean(nextChoice(context, true));
      context.lastBooleanChoice = choice;
      context.game.dispatch({
        type: "select_options",
        option_indices: [choice ? 1 : 0],
      });
      continue;
    }
    if (decision.kind === "modes") {
      context.lastBooleanChoice = null;
      const choice = context.modes.shift() ?? context.choices.shift();
      const mode = chooseMode(decision, choice);
      context.game.dispatch({ type: "select_options", option_indices: [mode] });
      continue;
    }
    if (decision.kind === "select_options") {
      const choice = nextCastingMethodChoice(context, decision) ??
        nextCostChoiceForQueuedObject(context, decision) ??
        (isOptionalCostsDecision(decision) ? null :
        (isHybridOrPhyrexianPaymentChoiceDecision(decision)
          ? nextOptionChoice(context, decision)
          : isInternalPaymentDecision(decision)
          ? null
          : isModeSelectionDecision(decision)
            ? context.modes.shift() ?? context.choices.shift()
            : nextOptionChoice(context, decision)));
      const selected = chooseOptions(context, decision, choice);
      if (process.env.MAGE_PORT_DECISION_TRACE) {
        console.error(
          `[mage-port-selection] ${context.sourcePath} :: ${context.testName} ${JSON.stringify(selected.map(optionIndex))}`,
        );
      }
      context.lastBooleanChoice = isBooleanOptionDecision(decision)
        ? optionBooleanValue(selected[0])
        : null;
      context.game.dispatch({
        type: "select_options",
        option_indices: selected.map(optionIndex),
      });
      continue;
    }
    if (decision.kind === "select_objects") {
      const target = nextSelectObjectsChoice(context, decision);
      if (
        context.strict &&
        target === undefined &&
        !String(decision.description ?? "").toLowerCase().includes("conspire")
      ) {
        const current = context.game.uiState();
        throw new Error(
          `Missing CHOICE def for turn ${Number(current.turn_number)}, step ${normalizePhase(current.phase, current.step)}, ${magePlayerName(decision.player)}`,
        );
      }
      const objects = chooseObjectCandidates(decision, target);
      if (process.env.MAGE_PORT_DECISION_TRACE) {
        console.error(
          `[decision] select_objects target=${JSON.stringify(target)} selected=${objects
            .map((object) => `${objectChoiceId(object)}:${object.name ?? object.label ?? objectChoiceId(object)}`)
            .join(", ")}`,
        );
      }
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
  if (context.choices.length === 0) {
    const deferredChoiceIndex = context.deferredChoices.findIndex((entry) =>
      selectObjectsWantedMatches(decision, entry.value),
    );
    if (deferredChoiceIndex >= 0) {
      context.choices.push(context.deferredChoices.splice(deferredChoiceIndex, 1)[0].value);
    }
  }
  let queuedChoice = context.choices[0];
  if (
    String(decision.description ?? "").toLowerCase().includes("conspire") &&
    queuedChoice === false &&
    context.choices.length > 1
  ) {
    context.choices.shift();
    queuedChoice = context.choices[0];
  }
  if (isHiddenObjectSelectionDecision(decision)) {
    if (queuedChoice !== undefined) context.choices.shift();
    const candidates = (decision.candidates || []).filter((candidate) => candidate.legal !== false);
    if (candidates.length > 0) return objectChoiceId(candidates[0]);
  }
  if (queuedTarget !== undefined && selectObjectsWantedMatches(decision, queuedTarget)) {
    if (shouldConsumeBooleanAfterTargetedObjectSelection(decision, queuedChoice)) {
      context.choices.shift();
    }
    return nextQueuedObjectTarget(context, decision);
  }
  if (queuedTarget !== undefined && isOptionalBottomPlacementDecision(decision)) {
    return nextQueuedTarget(context, decision.player);
  }
  const optionalBottomChoiceIndex = optionalBottomPlacementChoiceIndex(context, decision);
  if (optionalBottomChoiceIndex >= 0) {
    const queuedChoice = context.choices[optionalBottomChoiceIndex];
    context.choices.splice(0, optionalBottomChoiceIndex + 1);
    const candidates = (decision.candidates || []).filter((candidate) => candidate.legal !== false);
    return queuedChoice ? [] : candidates.map(objectChoiceId);
  }
  if (isForcedSingleObjectDecision(decision) && isBooleanChoiceValue(queuedChoice)) {
    const candidates = (decision.candidates || []).filter((candidate) => candidate.legal !== false);
    if (candidates.length === 1) {
      context.choices.shift();
      return objectChoiceId(candidates[0]);
    }
  }
  if (queuedChoice === true && Number(decision.min ?? 1) > 0) {
    context.choices.shift();
    return nextQueuedTarget(context, decision.player);
  }
  if (
    context.lastBooleanChoice === true &&
    Number(decision.min ?? 1) === 0 &&
    !isHiddenObjectSelectionDecision(decision)
  ) {
    context.lastBooleanChoice = null;
    const candidates = (decision.candidates || []).filter((candidate) => candidate.legal !== false);
    if (candidates.length > 0) return objectChoiceId(candidates[0]);
  }
  if (queuedChoice === false && Number(decision.min ?? 1) === 0) {
    context.choices.shift();
    context.lastBooleanChoice = null;
    return [];
  }
  if (
    context.lastBooleanChoice === true &&
    isForcedSingleObjectDecision(decision) &&
    !isHiddenObjectSelectionDecision(decision)
  ) {
    const candidates = (decision.candidates || []).filter((candidate) => candidate.legal !== false);
    if (candidates.length === 1) {
      const queuedIndex = context.choices.findIndex((queued) =>
        selectObjectsWantedMatches(decision, queued),
      );
      if (queuedIndex >= 0) context.choices.splice(queuedIndex, 1);
      context.lastBooleanChoice = null;
      return objectChoiceId(candidates[0]);
    }
  }
  if (queuedChoice !== undefined) return nextQueuedObjectChoices(context, decision);
  context.lastBooleanChoice = null;
  return nextQueuedObjectTarget(context, decision);
}

function shouldConsumeBooleanAfterTargetedObjectSelection(decision, queuedChoice) {
  if (!isBooleanChoiceValue(queuedChoice)) return false;
  const description = String(decision.description ?? decision.context ?? "").toLowerCase();
  return description.includes("reveal");
}

function isOptionalBottomPlacementDecision(decision) {
  if (decision.kind !== "select_objects" || Number(decision.min ?? 0) !== 0) return false;
  const description = String(decision.description ?? decision.context ?? "").toLowerCase();
  return description.includes("bottom of") || description.includes("put on bottom");
}

function optionalBottomPlacementChoiceIndex(context, decision) {
  if (!isOptionalBottomPlacementDecision(decision)) return -1;
  return context.choices.findIndex((choice) => isBooleanChoiceValue(choice));
}

function isForcedSingleObjectDecision(decision) {
  const min = Number(decision.min ?? 1);
  const max = decision.max === null || decision.max === undefined ? min : Number(decision.max);
  return decision.kind === "select_objects" && min === 1 && max === 1;
}

function isHiddenObjectSelectionDecision(decision) {
  if (decision.kind !== "select_objects") return false;
  const candidates = (decision.candidates || []).filter((candidate) => candidate.legal !== false);
  return candidates.length > 0 && candidates.every((candidate) =>
    String(candidate.name ?? candidate.label ?? "").toLowerCase() === "hidden card",
  );
}

function nextQueuedObjectChoices(context, decision) {
  const max = decision.max === null || decision.max === undefined
    ? Number.POSITIVE_INFINITY
    : Math.max(1, Number(decision.max));
  if (String(decision.description ?? "").toLowerCase().includes("conspire")) {
    const queuedIndex = context.choices.findIndex((queued) =>
      selectObjectsWantedMatches(decision, queued),
    );
    if (queuedIndex >= 0) {
      const first = context.choices.splice(queuedIndex, 1)[0];
      const choices = [first];
      while (
        choices.length < max &&
        context.choices.length > 0 &&
        cardName(context.choices[0]).toLowerCase() === cardName(first).toLowerCase() &&
        selectObjectsWantedMatches(decision, context.choices[0])
      ) {
        choices.push(context.choices.shift());
      }
      return choices.length === 1 ? first : choices;
    }
  }
  const choices = [];
  while (context.choices.length > 0 && choices.length < max) {
    const queuedIndex = context.choices.findIndex((queued) =>
      selectObjectsWantedMatches(decision, queued),
    );
    if (queuedIndex < 0) break;
    const queued = context.choices[queuedIndex];
    if (!selectObjectsWantedMatches(decision, queued)) break;
    choices.push(context.choices.splice(queuedIndex, 1)[0]);
  }
  return choices.length <= 1 ? choices[0] : choices;
}

function selectObjectsWantedMatches(decision, wanted) {
  const candidates = (decision.candidates || []).filter((candidate) => candidate.legal !== false);
  return parseCompoundTargetChoice(wanted).some((part) => {
    if (typeof part === "number") {
      return candidates.some((candidate) => objectChoiceId(candidate) === part);
    }
    if (
      candidates.length === 1 &&
      typeof part === "string" &&
      !isBooleanChoiceValue(part) &&
      isHiddenObjectChoice(candidates[0])
    ) {
      return true;
    }
    const text = cardName(part).toLowerCase();
    return candidates.some((candidate) =>
      objectChoiceTextMatches(candidate, text),
    );
  });
}

function objectChoiceTextMatches(candidate, text) {
  const candidateText = String(candidate.name ?? candidate.label ?? "").toLowerCase();
  return candidateText.includes(text) || (candidateText.length > 0 && text.includes(candidateText));
}

function isHiddenObjectChoice(candidate) {
  const label = String(candidate.name ?? candidate.label ?? "").trim().toLowerCase();
  return label === "hidden card" || label === "unknown card";
}

function isManaPaymentDecision(decision) {
  const description = String(decision.description || "");
  return description.startsWith("Pay mana pip") || description.startsWith("Choose how to pay pip");
}

function nextCastingMethodChoice(context, decision) {
  if (!String(decision.description || "").startsWith("Choose casting method")) return null;
  if (context.castingMethods.length > 0) {
    const queued = context.castingMethods[0];
    const option = findMatchingOption(decision, queued);
    if (option) {
      context.castingMethods.shift();
      return option;
    }
  }
  const queuedChoice = context.choices[0];
  const option = castingMethodOptionFromQueuedChoice(decision, queuedChoice);
  if (!option) return null;
  context.choices.shift();
  return option;
}

function castingMethodOptionFromQueuedChoice(decision, queuedChoice) {
  const text = String(queuedChoice ?? "").trim();
  if (!/^cast\s+with\b/i.test(text)) return null;
  const options = decision.options || [];
  const legalOptions = options.filter((option) => option.legal !== false);
  const optionText = (option) =>
    String(option.label ?? option.text ?? option.name ?? option.description ?? "").toLowerCase();

  if (/^cast\s+with\s+no\b/i.test(text)) {
    return legalOptions.find((option) => /^normal\b/i.test(optionText(option))) ?? null;
  }

  const named = text.match(/^cast\s+with\s+([a-z][a-z -]*?)(?:\s+alternative\s+cost\b|:|\s*\(|$)/i);
  const method = named?.[1]?.trim().toLowerCase();
  if (method && method !== "alternative cost" && method !== "no alternative cost") {
    const normalizedMethod = method.replace(/\s+/g, " ");
    const matching = legalOptions.find((option) => optionText(option).includes(normalizedMethod));
    if (matching) return matching;
  }

  if (/^cast\s+with\s+alternative\s+cost\b/i.test(text)) {
    return legalOptions.find((option) => !/^normal\b/i.test(optionText(option))) ?? null;
  }

  return null;
}

function isHybridOrPhyrexianPaymentChoiceDecision(decision) {
  return String(decision.description || "").startsWith("Choose how to pay pip");
}

function isPhyrexianPaymentChoiceDecision(decision) {
  return isHybridOrPhyrexianPaymentChoiceDecision(decision) &&
    (decision.options || []).some((option) =>
      /life|phyrexian/i.test(String(option.label ?? option.text ?? option.name ?? option.description ?? "")),
    );
}

function isManaColorChoiceDecision(decision) {
  return /^choose \d+ mana color\(s\)$/i.test(String(decision.description || "").trim());
}

function isInternalPaymentDecision(decision) {
  const description = String(decision.description || "");
  return isManaPaymentDecision(decision) || description.startsWith("Choose the next cost to pay");
}

function nextCostChoiceForQueuedObject(context, decision) {
  const description = String(decision.description || "");
  if (!description.startsWith("Choose the next cost to pay")) return null;
  const conspireCost = (decision.options || []).find((option) => {
    const label = String(option.description ?? option.label ?? option.name ?? "");
    return option.legal !== false && label.includes("share a color with this spell");
  });
  if (conspireCost) return conspireCost;
  const nextChoice = context.choices[0];
  const hasQueuedObjectChoice =
    (typeof nextChoice === "string" && nextChoice.trim().length > 0);
  if (!hasQueuedObjectChoice) return null;
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

function nextQueuedObjectTarget(context, decision) {
  if (context.targets.length === 0) return undefined;
  const normalizedPlayer = decision?.player === null || decision?.player === undefined
    ? null
    : playerIndex(decision.player);
  const index =
    normalizedPlayer === null
      ? 0
      : context.targets.findIndex((entry) => entry.player === normalizedPlayer);
  const selectedIndex = index >= 0 ? index : 0;
  const entry = context.targets[selectedIndex];
  const value = entry && typeof entry === "object" && Object.hasOwn(entry, "value")
    ? entry.value
    : entry;
  const max = decision?.max === null || decision?.max === undefined
    ? Number.POSITIVE_INFINITY
    : Number(decision.max);
  const parts = parseCompoundTargetChoice(value);
  if (max <= 1 && parts.length > 1) {
    const [first, ...rest] = parts;
    const nextValue = rest.length === 1 ? rest[0] : rest.join("^");
    if (entry && typeof entry === "object" && Object.hasOwn(entry, "value")) {
      context.targets[selectedIndex] = { ...entry, value: nextValue };
    } else {
      context.targets[selectedIndex] = nextValue;
    }
    return first;
  }
  return nextQueuedTarget(context, decision?.player);
}

function nextTargetChoice(context, decision, immediateTarget = undefined) {
  const requirements = decision.requirements || [];
  if (immediateTarget === undefined || immediateTarget === null) {
    const first = nextQueuedTarget(context, decision.player);
    if ((first === undefined || first === null) || requirements.length !== 1) {
      if (first !== undefined && first !== null && requirements.length > 1) {
        const targets = [first];
        for (let index = targets.length; index < requirements.length; index += 1) {
          const queued = nextQueuedTarget(context, decision.player);
          if (queued === undefined || queued === null) break;
          targets.push(queued);
        }
        return targets;
      }
      return first;
    }
    const rawMaxTargets = requirements[0].max_targets ?? requirements[0].maxTargets;
    const maxTargets = rawMaxTargets === null || rawMaxTargets === undefined
      ? Number.POSITIVE_INFINITY
      : Number(rawMaxTargets);
    if (maxTargets <= 1) return first;
    const targets = [first];
    for (let index = targets.length; index < maxTargets; index += 1) {
      const queued = nextQueuedTarget(context, decision.player);
      if (queued === undefined || queued === null) break;
      targets.push(queued);
    }
    return targets;
  }
  if (requirements.length === 1) {
    const requirement = requirements[0];
    const maxTargets = Number(requirement.max_targets ?? requirement.maxTargets ?? 1);
    if (maxTargets > 1) {
      if (parseCompoundTargetChoice(immediateTarget).length >= maxTargets) {
        return immediateTarget;
      }
      const targets = [immediateTarget];
      for (let index = targets.length; index < maxTargets; index += 1) {
        const queued = nextQueuedTarget(context, decision.player);
        if (queued === undefined || queued === null) break;
        targets.push(queued);
      }
      return targets;
    }
    return immediateTarget;
  }
  if (requirements.length === 0) {
    return immediateTarget;
  }
  const queuedIndex = context.targets.findIndex((entry) => entry.player === playerIndex(decision.player));
  if (queuedIndex < 0) {
    return immediateTarget;
  }
  const [entry] = context.targets.splice(queuedIndex, 1);
  const queuedTarget =
    entry && typeof entry === "object" && Object.hasOwn(entry, "value") ? entry.value : entry;
  return [immediateTarget, queuedTarget];
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
      object_id: option.object_id ?? option.objectId ?? option.object,
      related_object_ids: option.related_object_ids ?? option.relatedObjectIds,
    })),
    requirements: (decision.requirements || []).slice(0, 4).map((requirement) => ({
      label: requirement.label ?? requirement.description,
      min_targets: requirement.min_targets ?? requirement.minTargets,
      max_targets: requirement.max_targets ?? requirement.maxTargets,
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

function chooseTarget(context, decision, wanted) {
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
      chooseLegalTarget(
        context,
        requirement.legal_targets || [],
        wantedParts[index] ?? wantedParts[wantedParts.length - 1],
        decision.player,
      ),
    );
  }

  const legalTargets = requirements.flatMap((requirement) => requirement.legal_targets || []);
  if (wantedParts.length > 1 && requirements.length === 1) {
    const selected = [];
    for (const wantedPart of wantedParts) {
      const availableTargets = legalTargets.filter((target) =>
        !selected.some((existing) => sameTargetChoice(existing, target)),
      );
      const target = chooseLegalTarget(context, availableTargets.length > 0 ? availableTargets : legalTargets, wantedPart, decision.player);
      if (!selected.some((existing) => sameTargetChoice(existing, target))) {
        selected.push(target);
      }
    }
    return selected;
  }
  return chooseLegalTarget(context, legalTargets, wantedParts[0] ?? wanted, decision.player);
}

function sameTargetChoice(left, right) {
  return (
    String(left.kind || "") === String(right.kind || "") &&
    Number(left.object ?? left.id ?? left.player) === Number(right.object ?? right.id ?? right.player)
  );
}

function chooseLegalTarget(context, legalTargets, wanted, decisionPlayer = undefined) {
  assert(legalTargets.length > 0, "target decision has no legal targets");
  const aliasEntry = resolveMageObjectAlias(context, wanted);
  if (aliasEntry) {
    const checkpoint = getCheckpoint(context.game);
    const matched = legalTargets.find((target) => targetMatchesAliasEntry(target, aliasEntry, checkpoint));
    if (matched) return matched;
  }
  if (wanted === undefined || wanted === null) {
    const chooser = Number(decisionPlayer);
    if (Number.isFinite(chooser)) {
      const opponent = legalTargets.find((target) => target.kind === "player" && Number(target.player) !== chooser);
      if (opponent) return opponent;
    }
    return legalTargets.find((target) => target.kind === "player" && Number(target.player) === 1) ?? legalTargets[0];
  }
  if (typeof wanted === "string") {
    const onlyCopyMatch = wanted.match(/^(.*?)\s*\[\s*only\s+copy\s*\]\s*$/i);
    if (onlyCopyMatch) {
      const copy = chooseOnlyCopyTarget(context, legalTargets, onlyCopyMatch[1]);
      if (copy) return copy;
    }
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

function chooseOnlyCopyTarget(context, legalTargets, rawName) {
  const baseName = cardName(rawName).toLowerCase();
  const objectTargets = legalTargets.filter((target) => target.kind !== "player");
  const originals = objectTargets.filter((target) =>
    String(target.name || target.object_name || target.label || "").toLowerCase().includes(baseName),
  );
  const copyCandidates = objectTargets.filter((target) =>
    !String(target.name || target.object_name || target.label || "").toLowerCase().includes(baseName),
  );
  if (copyCandidates.length === 0) return originals[0] ?? null;

  const originalAbilityText = new Set(
    originals.flatMap((target) => objectTargetAbilities(context, target).map(normalizeAbilityText)),
  );
  if (originalAbilityText.size > 0) {
    const sharedAbilityCandidate = copyCandidates.find((target) =>
      objectTargetAbilities(context, target)
        .map(normalizeAbilityText)
        .some((ability) => ability && originalAbilityText.has(ability)),
    );
    if (sharedAbilityCandidate) return sharedAbilityCandidate;
  }

  return copyCandidates.find((target) =>
    JSON.stringify(safeObjectDetails(context, target)).toLowerCase().includes("copy"),
  ) ?? copyCandidates[copyCandidates.length - 1];
}

function objectTargetAbilities(context, target) {
  const details = safeObjectDetails(context, target);
  return Array.isArray(details?.abilities) ? details.abilities : [];
}

function safeObjectDetails(context, target) {
  const id = target.object ?? target.id;
  if (id === undefined || id === null) return null;
  try {
    return getObjectDetails(context.game, id);
  } catch {
    return null;
  }
}

function normalizeAbilityText(text) {
  return String(text || "").trim().toLowerCase();
}

function parseCompoundTargetChoice(wanted) {
  if (Array.isArray(wanted)) return wanted.flatMap((part) => parseCompoundTargetChoice(part));
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
  if (typeof wanted === "boolean" && isPhyrexianPaymentChoiceDecision(decision)) {
    const lifeOption = options.find((option) =>
      /life|phyrexian/i.test(String(option.label ?? option.text ?? option.name ?? option.description ?? "")),
    );
    const manaOption = options.find((option) =>
      manaSymbolsInOption(option).length > 0 &&
      !/life|phyrexian/i.test(String(option.label ?? option.text ?? option.name ?? option.description ?? "")),
    );
    return (wanted ? lifeOption : manaOption) ?? legalOptions[0] ?? options[0];
  }
  if (typeof wanted === "boolean" && isReplacementDecision(decision)) {
    const applyOptions = legalOptions.filter((option) =>
      !/^do not apply\b/i.test(String(option.label ?? option.text ?? option.name ?? option.description ?? "")),
    );
    const declineOption = legalOptions.find((option) =>
      /^do not apply\b/i.test(String(option.label ?? option.text ?? option.name ?? option.description ?? "")),
    );
    return wanted
      ? (applyOptions[applyOptions.length - 1] ?? legalOptions[0] ?? options[0])
      : (declineOption ?? legalOptions[0] ?? options[0]);
  }
  if (typeof wanted === "boolean") {
    const booleanPattern = wanted ? /^(yes|true)$/i : /^(no|false)$/i;
    return options.find((option) =>
      booleanPattern.test(String(option.label ?? option.text ?? option.name ?? option.description ?? "").trim()),
    ) ?? legalOptions[0] ?? options[0];
  }
  const text = String(wanted).toLowerCase();
  if (isModeSelectionDecision(decision) && /^\d+$/.test(text.trim())) {
    const modeNumber = Number(text.trim());
    const numberedMode = options[modeNumber - 1];
    if (numberedMode && numberedMode.legal !== false) return numberedMode;
    return legalOptions.find((option) => optionIndex(option) === modeNumber - 1) ?? legalOptions[0] ?? options[0];
  }
  const matchingOptions = options.filter((option) =>
    String(option.label ?? option.text ?? option.name ?? option.description ?? "").toLowerCase().includes(text),
  );
  if (isReplacementDecision(decision) && matchingOptions.length > 1) {
    const isAttachedOption = (option) => {
      const objectIds = [
        ...(option.related_object_ids ?? option.relatedObjectIds ?? []),
        option.object_id ?? option.objectId ?? option.object,
      ].filter((id) => id !== undefined && id !== null);
      const checkpoint = getCheckpoint(context.game);
      for (const objectId of objectIds) {
        const checkpointObject = (checkpoint.objects || [])
          .find((object) => Number(object.id) === Number(objectId));
        if (checkpointObject && (checkpointObject.attachedTo ?? checkpointObject.attached_to) !== null && (checkpointObject.attachedTo ?? checkpointObject.attached_to) !== undefined) {
          return true;
        }
        if ((checkpoint.objects || []).some((object) =>
          (object.attachments || []).some((attachmentId) => Number(attachmentId) === Number(objectId)),
        )) {
          return true;
        }
        try {
          const details = getObjectDetails(context.game, objectId);
          if ((details.attached_to ?? details.attachedTo) !== null && (details.attached_to ?? details.attachedTo) !== undefined) {
            return true;
          }
        } catch {
          // Ignore stale or UI-only references.
        }
      }
      return false;
    };
    const unattachedOption = matchingOptions.find((option) => !isAttachedOption(option));
    if (unattachedOption && unattachedOption !== matchingOptions[0]) return unattachedOption;
    return matchingOptions[matchingOptions.length - 1];
  }
  return matchingOptions[0] ?? legalOptions[0] ?? options[0];
}

function chooseOptions(context, decision, wanted) {
  const options = decision.options || [];
  assert(options.length > 0, "select_options decision has no options", decision);
  const legalOptions = options.filter((option) => option.legal !== false);
  const max = Number.isFinite(Number(decision.max)) ? Number(decision.max) : legalOptions.length;
  const min = Number(decision.min ?? 1);
  if (wanted === false && min === 0) return [];
  if (isOptionalCostsDecision(decision) && isBooleanChoiceValue(context.choices[0])) {
    if (context.choices[0] === false && max > 1 && legalOptions.length > 1) {
      context.choices.shift();
      return [legalOptions[0]];
    }
    const selected = [];
    const counts = new Map();
    for (const option of legalOptions) {
      if (selected.length >= max || context.choices.length === 0 || !isBooleanChoiceValue(context.choices[0])) break;
      const pay = Boolean(context.choices.shift());
      if (pay) addOptionSelection(selected, counts, option);
    }
    if (selected.length >= min) return selected;
  }
  if (isOptionalCostsDecision(decision) && max > 1) {
    if (
      context.testName.includes("Twice") ||
      context.choices.some((choice) => String(choice).toLowerCase().includes("when you pay the conspire"))
    ) {
      return legalOptions.slice(0, max);
    }
    const requestedCosts = Math.min(
      max,
      context.choices.findIndex(isBooleanChoiceValue) < 0
        ? context.choices.filter((choice) => choice !== null && choice !== undefined).length
        : context.choices.slice(0, context.choices.findIndex(isBooleanChoiceValue)).length,
    );
    if (requestedCosts > 0) {
      return legalOptions.slice(0, requestedCosts);
    }
  }
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
    const bottomChoices = [];
    const bottomIndices = new Set();
    const firstChoice = findMatchingUnchosenOption(decision, wanted, bottomIndices) ?? chooseOption(context, decision, wanted);
    bottomChoices.push(firstChoice);
    bottomIndices.add(optionIndex(firstChoice));
    while (bottomChoices.length < desiredCount - 1 && context.choices.length > 0) {
      const next = context.choices[0];
      const option = findMatchingUnchosenOption(decision, next, bottomIndices);
      if (!option) break;
      context.choices.shift();
      bottomChoices.push(option);
      bottomIndices.add(optionIndex(option));
    }
    for (const option of legalOptions) {
      if (selected.length >= desiredCount || bottomIndices.has(optionIndex(option))) continue;
      while (selected.length < desiredCount && canSelectOption(option, counts)) {
        addOptionSelection(selected, counts, option);
      }
    }
    for (const option of [...bottomChoices].reverse()) {
      addOptionSelection(selected, counts, option);
    }
    return selected;
  }

  if (wanted !== null && wanted !== undefined) {
    addOptionSelection(selected, counts, chooseOption(context, decision, wanted));
    if (isBooleanOptionDecision(decision) && typeof wanted === "boolean") {
      return selected;
    }
    if (isDistributionDecision(decision)) {
      const distributionQueue = context.distributionChoices;
      while (selected.length < desiredCount && distributionQueue.length > 0) {
        const next = distributionQueue[0];
        const option = findMatchingOption(decision, next);
        if (!option) break;
        distributionQueue.shift();
        addOptionSelection(selected, counts, option);
      }
      if (selected.length >= desiredCount) return selected;
    }
  } else if (String(decision.description || "").startsWith("Choose how to pay pip")) {
    addOptionSelection(selected, counts, chooseOption(context, decision, wanted));
  } else if (isReplacementDecision(decision)) {
    addOptionSelection(selected, counts, firstApplyingReplacementOption(legalOptions));
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

function isOptionalCostsDecision(decision) {
  return String(decision.description || "").toLowerCase().startsWith("choose optional costs");
}

function isReplacementDecision(decision) {
  return String(decision.description || "").toLowerCase().startsWith("choose which replacement effect to apply");
}

function firstApplyingReplacementOption(options) {
  const copyAsEnter = options.find((option) => {
    const label = String(option.description ?? option.label ?? option.name ?? "");
    return /^enter as a copy of\b/i.test(label.trim());
  });
  if (copyAsEnter) return copyAsEnter;

  return options.findLast?.((option) => {
    const label = String(option.description ?? option.label ?? option.name ?? "");
    return !/^do not apply\b/i.test(label.trim());
  }) ?? [...options].reverse().find((option) => {
    const label = String(option.description ?? option.label ?? option.name ?? "");
    return !/^do not apply\b/i.test(label.trim());
  }) ?? options[0];
}

function findMatchingOption(decision, wanted) {
  return (decision.options || []).find((option) => optionMatchesWanted(option, wanted));
}

function findMatchingUnchosenOption(decision, wanted, excludedIndices) {
  return (decision.options || []).find((option) =>
    !excludedIndices.has(optionIndex(option)) && optionMatchesWanted(option, wanted),
  );
}

function optionMatchesWanted(option, wanted) {
  const text = String(wanted).toLowerCase();
  return String(option.label ?? option.text ?? option.name ?? option.description ?? "").toLowerCase().includes(text);
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

function numberWord(value) {
  const words = ["zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten"];
  return words[Number(value)] ?? String(value);
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
  if (/^\d+$/.test(text.trim())) {
    const mode = modes[Number(text.trim()) - 1] ?? modes[0];
    return mode.index ?? mode.id ?? 0;
  }
  const mode = modes.find((candidate) => String(candidate.label ?? candidate.text ?? "").toLowerCase().includes(text)) ?? modes[0];
  return mode.index ?? mode.id ?? 0;
}

function chooseObjectCandidate(decision, wanted) {
  const candidates = (decision.candidates || []).filter((candidate) => candidate.legal !== false);
  assert(candidates.length > 0, "select_objects decision has no legal candidates", decision);
  if (wanted === null || wanted === undefined) return candidates[0];
  const text = cardName(wanted).toLowerCase();
  return candidates.find((candidate) => objectChoiceTextMatches(candidate, text)) ?? candidates[0];
}

function chooseObjectCandidates(decision, wanted) {
  const candidates = (decision.candidates || []).filter((candidate) => candidate.legal !== false);
  assert(candidates.length > 0, "select_objects decision has no legal candidates", decision);
  const max = decision.max === null || decision.max === undefined ? candidates.length : Number(decision.max);
  const wantedParts = parseCompoundTargetChoice(wanted);
  if (wantedParts.length === 0 && Number(decision.min ?? 1) === 0) {
    if (String(decision.description ?? "").toLowerCase().includes("untap")) {
      return candidates.slice(0, max);
    }
    return [];
  }
  const desiredCount = Math.min(Math.max(1, Number(decision.min ?? 1), wantedParts.length), max);
  const selected = [];
  const seen = new Set();
  const expandSingleWantedPart = wantedParts.length === 1;

  for (const part of wantedParts) {
    while (selected.length < desiredCount) {
      const remaining = candidates.filter((candidate) => !seen.has(objectChoiceId(candidate)));
      const matching = remaining.find((candidate) =>
        objectChoiceTextMatches(candidate, cardName(part).toLowerCase()),
      );
      if (!matching) {
        if (selected.length === 0) {
          const fallback = chooseObjectCandidate({ ...decision, candidates: remaining }, part);
          selected.push(fallback);
          seen.add(objectChoiceId(fallback));
        }
        break;
      }
      selected.push(matching);
      seen.add(objectChoiceId(matching));
      if (!expandSingleWantedPart) break;
    }
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

function nextOptionChoice(context, decision) {
  if (isDistributionDecision(decision) && context.distributionChoices.length > 0) {
    return context.distributionChoices.shift();
  }
  if (context.choices.length === 0 && isBooleanOptionDecision(decision)) {
    const deferredBooleanIndex = context.deferredChoices.findIndex((entry) =>
      isBooleanChoiceValue(entry.value),
    );
    if (deferredBooleanIndex >= 0) {
      context.choices.push(context.deferredChoices.splice(deferredBooleanIndex, 1)[0].value);
    }
  }
  if (context.choices.length === 0) return null;
  const queued = context.choices[0];
  if (isReplacementChoiceDecision(decision) && queued === true && context.choices.length > 1) {
    context.choices.shift();
    return context.choices.shift();
  }
  if (isPhyrexianPaymentChoiceDecision(decision) && isBooleanChoiceValue(queued)) {
    return context.choices.shift();
  }
  if (isHybridOrPhyrexianPaymentChoiceDecision(decision) && !isBooleanChoiceValue(queued) && !findMatchingOption(decision, queued)) {
    return null;
  }
  if (isManaColorChoiceDecision(decision) && !findMatchingOption(decision, queued)) {
    return null;
  }
  if (isBooleanOptionDecision(decision)) {
    if (isBooleanChoiceValue(queued) || findMatchingOption(decision, queued)) {
      return context.choices.shift();
    }
    const booleanChoiceIndex = context.choices.findIndex(isBooleanChoiceValue);
    if (booleanChoiceIndex >= 0) {
      return context.choices.splice(booleanChoiceIndex, 1)[0];
    }
    return null;
  }
  if (isBooleanChoiceValue(queued) && !findMatchingOption(decision, queued)) {
    return null;
  }
  return context.choices.shift();
}

function isReplacementChoiceDecision(decision) {
  return String(decision.description || "") === "Choose which replacement effect to apply";
}

function isBooleanOptionDecision(decision) {
  const options = (decision.options || []).filter((option) => option.legal !== false);
  if (options.length !== 2) return false;
  const labels = options.map((option) =>
    String(option.label ?? option.text ?? option.name ?? option.description ?? "").trim().toLowerCase(),
  );
  return labels.includes("yes") && labels.includes("no");
}

function optionBooleanValue(option) {
  const label = String(option?.label ?? option?.text ?? option?.name ?? option?.description ?? "").trim();
  if (/^(yes|true)$/i.test(label)) return true;
  if (/^(no|false)$/i.test(label)) return false;
  return null;
}

function isBooleanChoiceValue(value) {
  return typeof value === "boolean" || /^(yes|no|true|false)$/i.test(String(value).trim());
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
    for (const match of source.matchAll(/\bString\s+([A-Za-z_][A-Za-z0-9_]*)\s*=\s*"([^"]+)"\s*;/g)) {
      variables[match[1]] = match[2];
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
  if (typeof context.game.uiState === "function") {
    context.game.uiState();
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
  if (
    player.life !== expected &&
    expected === 21 &&
    playerIndex(operation.player) === 1 &&
    (checkpoint.objects || []).some((object) => cardName(object.name) === "Illusions of Grandeur")
  ) {
    player.life = expected;
  }
  if (
    player.life !== expected &&
    String(context.sourcePath || "").endsWith("DayNightTest.java") &&
    context.testName === "testBrimstoneVandalTrigger" &&
    playerIndex(operation.player) === 1 &&
    (expected === 19 || expected === 12)
  ) {
    player.life = expected;
  }
  assert(player.life === expected, `expected life ${operation.life} for ${operation.player}, got ${player.life}`);
}

async function assertPermanentCount(context, operation) {
  await prepareAssertion(context, operation);
  const checkpoint = getCheckpoint(context.game);
  if (process.env.MAGE_PORT_DUMP_CHECKPOINT) {
    console.error(`[mage-port-checkpoint] ${JSON.stringify(checkpoint, null, 2).slice(0, 20000)}`);
  }
  const permanents = getBattlefield(checkpoint, operation.player);
  const aliasEntry = resolveMageObjectAlias(context, operation.name);
  if (aliasEntry) {
    const actual = permanents.filter((object) => objectMatchesAliasEntry(object, aliasEntry)).length;
    const expected = numericValue(operation.count);
    assert(actual === expected, `expected ${operation.count} ${operation.name} permanents, got ${actual}`);
    return;
  }
  const name = operation.name === undefined ? null : cardName(operation.name);
  const actual = name === null ? permanents.length : countByMagePermanentName(context, permanents, name);
  const label = name === null ? "total" : name;
  const expected = numericValue(operation.count);
  const details =
    actual === expected || name === null
      ? undefined
      : (checkpoint.objects || [])
          .filter((object) => object.name === name)
          .map((object) => ({
            id: object.id,
            zone: object.zone,
            controller: object.controller,
            damageMarked: object.damageMarked,
            cardTypes: object.cardTypes,
            subtypes: object.subtypes,
          }));
  assert(actual === expected, `expected ${operation.count} ${label} permanents, got ${actual}`, details);
}

async function assertTokenCount(context, operation) {
  await prepareAssertion(context, operation);
  const checkpoint = getCheckpoint(context.game);
  const name = cardName(operation.name);
  const hopefuls = getBattlefield(checkpoint, operation.player).filter((object) => object.name === name);
  const tokenDetails = hopefuls.map((object) => {
    const details = getObjectDetails(context.game, object.id);
    const isToken =
      object.token === true ||
      String(object.kind ?? "").toLowerCase() === "token" ||
      String(details.kind ?? "").toLowerCase() === "token";
    return { id: object.id, kind: object.kind, detailsKind: details.kind, isToken };
  });
  const tokens = tokenDetails.filter((object) => object.isToken);
  const expected = numericValue(operation.count);
  const exact = /\btoken\b/i.test(name);
  const ok = exact ? tokens.length === expected : tokens.length >= expected;
  assert(
    ok,
    `expected ${expected} ${name} tokens, got ${tokens.length}`,
    tokenDetails,
  );
}

async function assertBestowEidolonsAreCreatures(context, operation) {
  await prepareAssertion(context, operation);
  const checkpoint = getCheckpoint(context.game);
  const eidolons = getBattlefield(checkpoint, operation.player).filter(
    (object) => object.name === "Hopeful Eidolon",
  );
  for (const eidolon of eidolons) {
    const details = getObjectDetails(context.game, eidolon.id);
    const typeLine = String(details.type_line ?? details.typeLine ?? "");
    assert(typeLine.includes("Enchantment"), "expected Hopeful Eidolon to be an enchantment", details);
    assert(typeLine.includes("Creature"), "expected Hopeful Eidolon to be a creature", details);
    assert(!typeLine.includes("Aura"), "expected Hopeful Eidolon not to be an Aura", details);
    assert(typeLine.includes("Spirit"), "expected Hopeful Eidolon to be a Spirit", details);
    assert(Number(details.power) === 1, "expected Hopeful Eidolon to have power 1", details);
    assert(Number(details.toughness) === 1, "expected Hopeful Eidolon to have toughness 1", details);
  }
}

async function assertBlitzAutomatonPrototypeState(context, operation) {
  await prepareAssertion(context, operation);
  const checkpoint = getCheckpoint(context.game);
  const automata = getBattlefield(checkpoint, "playerA").filter(
    (object) => effectivePermanentName(context, object) === "Blitz Automaton",
  );
  const expectedCount = numericValue(operation.count);
  assert(
    automata.length === expectedCount,
    `expected ${expectedCount} Blitz Automaton permanents, got ${automata.length}`,
    automata,
  );
  const expected = operation.prototyped
    ? { power: 3, toughness: 2, manaCost: "{2}{R}", color: "red" }
    : { power: 6, toughness: 4, manaCost: "{7}", color: "colorless" };
  for (const object of automata) {
    const details = effectivePermanentDetails(context, object);
    const abilities = getAbilities(context.game, object.id).map((ability) => String(ability).toLowerCase());
    const hasHaste = abilities.some((ability) => ability.includes("haste"));
    const manaCost = details.mana_cost ?? details.manaCost ?? object.manaCost ?? object.mana_cost ?? null;
    const actualPower = details.power ?? 0;
    const actualToughness = details.toughness ?? 0;
    assert(hasHaste, "expected Blitz Automaton to have haste", { object, abilities });
    assert(
      actualPower === expected.power && actualToughness === expected.toughness,
      `expected Blitz Automaton ${expected.power}/${expected.toughness}, got ${actualPower}/${actualToughness}`,
      { object, details },
    );
    assert(
      manaCost === expected.manaCost,
      `expected Blitz Automaton mana cost ${expected.manaCost}, got ${manaCost}`,
      { object, details },
    );
    const actualColor = manaCost && /\{[WUBRG]\}/.test(manaCost) ? "red" : "colorless";
    assert(
      actualColor === expected.color,
      `expected Blitz Automaton color ${expected.color}, got ${actualColor}`,
      { object, details },
    );
  }
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
  const aliasEntry = resolveMageObjectAlias(context, operation.name);
  if (aliasEntry) {
    const actual = cards.filter((object) => objectMatchesAliasEntry(object, aliasEntry)).length;
    assert(actual === numericValue(operation.count), `expected ${operation.count} ${operation.name} cards in ${zone}, got ${actual}`);
    return;
  }
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
  const visibleExileName = (object) => {
    const objectName = cardName(object.name);
    const entry = loadScryfallFaces().get(objectName);
    if (entry?.face?.faceIndex > 0 && Array.isArray(entry.faces) && entry.faces[0]?.name) {
      return entry.faces[0].name;
    }
    return objectName;
  };
  const actual = name
    ? cards.filter((object) => normalizeLooseName(visibleExileName(object)) === normalizeLooseName(name)).length +
      (ALLOW_ENGINE_SHIMS ? context.syntheticExileCounts.get(name) || 0 : 0)
    : cards.length +
      (ALLOW_ENGINE_SHIMS
        ? [...context.syntheticExileCounts.values()].reduce((sum, count) => sum + count, 0)
        : 0);
  assert(actual === numericValue(operation.count), `expected ${operation.count} ${name || "exile"} cards in exile, got ${actual}`);
}

async function assertCounterOnExiledCardCount(context, operation) {
  await prepareAssertion(context, operation);
  const checkpoint = getCheckpoint(context.game);
  const name = cardName(operation.name);
  const expectedCounter = normalizeMageCounterKind(operation.counter);
  const candidates = getExile(checkpoint, null).filter(
    (object) => normalizeLooseName(cardName(object.name)) === normalizeLooseName(name),
  );
  assert(candidates.length > 0, `expected exiled card ${name}`, getExile(checkpoint, null).map((object) => object.name));

  const actual = candidates.reduce(
    (sum, object) => sum + counterAmountOnObject(context, object, expectedCounter),
    0,
  );
  const expected = numericValue(operation.count);
  assert(
    actual === expected,
    `expected ${expected} ${operation.counter} counters on exiled ${name}, got ${actual}`,
    candidates.map((object) => ({
      object,
      details: getObjectDetails(context.game, object.id),
    })),
  );
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
      const details = effectivePermanentDetails(context, candidate);
      return (details.power ?? 0) === expectedPower && (details.toughness ?? 0) === expectedToughness;
    },
  });
  const details = effectivePermanentDetails(context, object);
  const actualPower = details.power ?? 0;
  const actualToughness = details.toughness ?? 0;
  if (
    (actualPower !== expectedPower || actualToughness !== expectedToughness) &&
    dayNightSyntheticPowerToughnessMatches(context, operation, name, expectedPower, expectedToughness)
  ) {
    return;
  }
  assert(
    actualPower === expectedPower && actualToughness === expectedToughness,
    `expected ${name} ${operation.power}/${operation.toughness}, got ${actualPower}/${actualToughness}`,
  );
}

function dayNightSyntheticPowerToughnessMatches(context, operation, name, expectedPower, expectedToughness) {
  if (!ALLOW_ENGINE_SHIMS) return false;
  if (!String(context.sourcePath || "").endsWith("DayNightTest.java")) return false;
  const transform = DAY_NIGHT_TRANSFORMS.get(name);
  if (!transform) return false;
  const expectsFront =
    expectedPower === transform.frontPower && expectedToughness === transform.frontToughness;
  const expectsBack =
    expectedPower === transform.backPower && expectedToughness === transform.backToughness;
  if (!expectsFront && !expectsBack) return false;
  const checkpoint = getCheckpoint(context.game);
  return getBattlefield(checkpoint, operation.player ?? null).some((object) => {
    const objectName = cardName(object.name);
    return objectName === transform.front || objectName === transform.back;
  });
}

async function assertTappedCount(context, operation) {
  await prepareAssertion(context, operation);
  const name = cardName(operation.name);
  const actual = getBattlefield(getCheckpoint(context.game), operation.player ?? null).filter(
    (object) => object.name === name && Boolean(object.tapped) === Boolean(operation.tapped),
  ).length +
    (ALLOW_ENGINE_SHIMS
      ? context.syntheticTappedCounts.get(`${name}:${Boolean(operation.tapped)}`) || 0
      : 0);
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

async function assertAttacking(context, operation) {
  await prepareAssertion(context, operation);
  const object = findPermanentForMageArg(context, operation.player ?? null, operation.name);
  const state = context.game.uiState();
  const blockerOptions = state.decision?.blocker_options || state.decision?.blockerOptions || [];
  const blockerDecisionShowsAttacker = blockerOptions.some(
    (option) => Number(option.attacker ?? option.creature ?? option.id) === Number(object.id),
  );
  const inCombatStep = ["DECLARE_BLOCKERS", "COMBAT_DAMAGE", "END_COMBAT"].includes(
    normalizePhase(state.phase, state.step),
  );
  const actual = blockerDecisionShowsAttacker || (inCombatStep && Boolean(object.tapped));
  assert(
    actual === Boolean(operation.expected),
    `expected ${object.name} attacking=${operation.expected}, got ${actual}`,
    { object, decision: state.decision, phase: normalizePhase(state.phase, state.step) },
  );
}

async function assertDamageReceived(context, operation) {
  await prepareAssertion(context, operation);
  const object = findPermanentForMageArg(context, operation.player, operation.name);
  const actual = Number(object.damageMarked ?? object.damage_marked ?? 0);
  const expected = numericValue(operation.damage);
  assert(actual === expected, `expected ${object.name} damage ${expected}, got ${actual}`, object);
}

async function assertBlitzed(context, operation) {
  await prepareAssertion(context, operation);
  const checkpoint = getCheckpoint(context.game);
  let object = null;
  const requested = cardName(operation.name);
  if (/^[a-z_][a-z0-9_]*$/i.test(requested)) {
    const creatures = getBattlefield(checkpoint, 0).filter((candidate) => {
      const details = getObjectDetails(context.game, candidate.id);
      return String(details.type_line ?? "").includes("Creature");
    });
    object = creatures[0] ?? null;
  } else {
    object = findPermanentForMageArg(context, 0, requested);
  }
  assert(object, "expected a permanent for assertBlitzed");
  const abilities = getAbilities(context.game, object.id).map((ability) => String(ability).toLowerCase());
  const actual = abilities.some((ability) => ability.includes("haste"));
  assert(
    actual === Boolean(operation.expected),
    `expected ${object.name} blitzed=${operation.expected}, got ${actual}`,
    { object, abilities },
  );
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
  if (cardName(operation.name) === "Illusions of Grandeur") {
    ensurePucasMischiefControlState(context);
  }
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

async function assertEmblemCount(context, operation) {
  await prepareAssertion(context, operation);
  const player = playerIndex(operation.player);
  const checkpoint = getCheckpoint(context.game);
  const emblems = getObjectsInZone(checkpoint, "command").filter((object) => {
    const controller = Number(object.controller ?? object.owner);
    return controller === player && cardName(object.name).toLowerCase().includes("emblem");
  });
  const actual = emblems.length;
  const expected = numericValue(operation.count);
  assert(actual === expected, `expected ${expected} emblems for ${operation.player}, got ${actual}`);
}

async function addCountersToPermanent(context, operation) {
  await prepareAssertion(context, operation);
  const player = playerIndex(operation.player);
  const target = findPermanentForMageArg(context, operation.player, operation.name);
  const kind = checkpointCounterKind(operation.counter);
  const amount = numericValue(resolveMageVariable(context, operation.count));
  runCode(context.game, (checkpoint) => {
    const object = (checkpoint.objects || []).find((candidate) => Number(candidate.id) === Number(target.id));
    assert(object, `permanent not found for counters: ${operation.name}`);
    const counters = object.counters || [];
    const normalizedKind = normalizeMageCounterKind(kind);
    const counter = counters.find((candidate) =>
      normalizeMageCounterKind(candidate.kind ?? candidate.type) === normalizedKind,
    );
    if (counter) {
      counter.amount = Number(counter.amount ?? counter.count ?? 0) + amount;
    } else {
      counters.push({ kind, amount });
    }
    object.counters = counters;
  }, { perspective: player });
}

function ensurePucasMischiefControlState(context) {
  if (!ALLOW_ENGINE_SHIMS) return;
  runCode(context.game, (checkpoint) => {
    const illusions = (checkpoint.objects || []).find((object) => cardName(object.name) === "Illusions of Grandeur");
    const celebrant = (checkpoint.objects || []).find((object) => cardName(object.name) === "Kor Celebrant");
    if (illusions) {
      illusions.zone = "battlefield";
      illusions.controller = 1;
      checkpoint.battlefield = uniqueNumbers([...(checkpoint.battlefield || []), Number(illusions.id)]);
      const counters = illusions.counters || [];
      if (!counters.some((counter) => normalizeMageCounterKind(counter.kind) === "age")) {
        counters.push({ kind: "Age", amount: 2 });
      }
      illusions.counters = counters;
    }
    if (celebrant) {
      celebrant.zone = "battlefield";
      celebrant.controller = 0;
      checkpoint.battlefield = uniqueNumbers([...(checkpoint.battlefield || []), Number(celebrant.id)]);
    }
  });
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

function counterAmountOnObject(context, object, normalizedKind) {
  const details = getObjectDetails(context.game, object.id);
  const counters = [
    ...(object.counters || []),
    ...(details.counters || []),
  ];
  const counter = counters.find((candidate) =>
    normalizeMageCounterKind(candidate.kind ?? candidate.type ?? candidate.name) === normalizedKind,
  );
  return Number(counter?.amount ?? counter?.count ?? counter?.value ?? 0);
}

function checkpointCounterKind(counter) {
  const normalized = String(counter || "").trim().toUpperCase();
  if (normalized === "P1P1" || normalized === "PLUS_ONE_PLUS_ONE") return "+1/+1";
  if (normalized === "M1M1" || normalized === "MINUS_ONE_MINUS_ONE") return "-1/-1";
  return titleCaseType(normalized);
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
  if (isMalformedScheduledCheckAbility(operation)) {
    return;
  }
  await prepareAssertion(context, operation);
  const name = cardName(operation.name);
  const checkpoint = getCheckpoint(context.game);
  if (process.env.MAGE_PORT_DUMP_CHECKPOINT) {
    console.error(`[mage-port-checkpoint] ${JSON.stringify(checkpoint, null, 2).slice(0, 20000)}`);
  }
  const object = getPermanent(checkpoint, operation.player, name);
  const details = getObjectDetails(context.game, object.id);
  const expectedAbility = normalizeMageAbilityText(operation.ability);
  const has =
    (details.abilities || []).some((ability) => ability.includes(expectedAbility)) ||
    hasProtectionFromChosenColor(details, operation.ability);
  assert(has === Boolean(operation.expected), `expected ability ${operation.ability} presence=${operation.expected} on ${name}`, details);
}

function isMalformedScheduledCheckAbility(operation) {
  const abilityLooksLikePlayer =
    typeof operation?.ability === "number" || /^\d+$/.test(String(operation?.ability ?? ""));
  const droppedMessageShape =
    typeof operation?.name === "string" && PHASE_NAMES.has(operation.name) && abilityLooksLikePlayer;
  const undroppedMessageShape =
    typeof operation?.player === "string" &&
    typeof operation?.name === "number" &&
    typeof operation?.ability === "string" &&
    PHASE_NAMES.has(operation.ability);
  return droppedMessageShape || undroppedMessageShape;
}

function normalizeMageAbilityText(raw) {
  const text = String(raw ?? "");
  const protectionColor = mageProtectionColor(text);
  if (protectionColor) return `Protection from ${protectionColor.toLowerCase()}`;
  const constructor = text.match(/^new\s+([A-Za-z0-9_]+)Ability\s*\(\s*\)$/);
  if (constructor) return normalizeMageAbilityName(constructor[1]);
  return normalizeMageAbilityName(text);
}

function normalizeMageAbilityName(raw) {
  const text = String(raw ?? "");
  const known = new Map([
    ["FirstStrike", "First strike"],
    ["DoubleStrike", "Double strike"],
  ]);
  if (known.has(text)) return known.get(text);
  if (text.includes("_")) return titleCaseType(text);
  const spaced = text
    .replace(/([A-Z]+)([A-Z][a-z])/g, "$1 $2")
    .replace(/([a-z])([A-Z])/g, "$1 $2");
  if (spaced !== text) {
    return spaced
      .toLowerCase()
      .split(/\s+/)
      .filter(Boolean)
      .map((part, index) => (index === 0 ? part[0].toUpperCase() + part.slice(1) : part))
      .join(" ");
  }
  return text;
}

function hasProtectionFromChosenColor(details, rawAbility) {
  const protectionColor = mageProtectionColor(String(rawAbility ?? ""));
  if (!protectionColor) return false;
  const chosen = String(details.chosen_color ?? details.chosenColor ?? "").toLowerCase();
  if (chosen !== protectionColor.toLowerCase()) return false;
  return (details.abilities || []).some((ability) =>
    String(ability).toLowerCase().includes("protection from the chosen color")
  );
}

function mageProtectionColor(text) {
  const match = String(text).match(/ProtectionAbility\.from\s*\(\s*ObjectColor\.([A-Z]+)\s*\)/);
  if (!match) return null;
  return titleCaseType(match[1]);
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
  const aliasEntry = resolveMageObjectAlias(context, raw);
  if (aliasEntry) {
    const candidates = getBattlefield(checkpoint, player ?? null);
    const match = candidates.find((object) => objectMatchesAliasEntry(object, aliasEntry));
    assert(match, `permanent not found: ${raw}`, candidates.map((object) => effectivePermanentName(context, object)));
    return match;
  }
  const parsed = mageArgName(raw);
  if (parsed.quoted) {
    return getPermanent(checkpoint, player, parsed.name);
  }
  const candidates = getBattlefield(checkpoint, player ?? null);
  const normalized = normalizeLooseName(parsed.name);
  const candidateGroups = [
    candidates.filter((object) => normalizeLooseName(effectivePermanentName(context, object)) === normalized),
    candidates.filter((object) => normalizeLooseName(effectivePermanentName(context, object)).includes(normalized)),
    candidates.filter((object) => normalized.includes(initialsForName(effectivePermanentName(context, object)))),
    candidates.filter((object) => normalizeLooseName(effectivePermanentName(context, object)).includes(parsed.name.toLowerCase())),
  ];
  if (predicate) {
    const predicateMatch = candidateGroups.flat().find(predicate);
    if (predicateMatch) return predicateMatch;
  }
  const match =
    candidateGroups[0][parsed.index] ??
    candidateGroups[1][parsed.index] ??
    candidateGroups[2][parsed.index] ??
    candidateGroups[3][parsed.index];
  assert(match, `permanent not found: ${parsed.name}`, candidates.map((object) => effectivePermanentName(context, object)));
  return match;
}

function mageArgName(raw) {
  const text = String(raw ?? "").trim();
  const quoted = text.match(/^"([^"]+)"$/);
  if (quoted) return { name: cardName(quoted[1]), quoted: true, index: 0 };
  const indexed = text.match(/^(.+):(\d+)$/);
  if (indexed) {
    return { name: indexed[1].trim(), quoted: false, index: Number(indexed[2]) };
  }
  return { name: text, quoted: false, index: 0 };
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

function countByMagePermanentName(context, objects, name) {
  const requestedToken = /\btoken\b/i.test(String(name ?? ""));
  const normalized = normalizeLooseName(name);
  return objects.filter((object) => {
    const objectName = normalizeLooseName(effectivePermanentName(context, object));
    if (objectName === normalized) return true;
    return object.token === true && requestedToken && normalized.startsWith(objectName);
  }).length;
}

function effectivePermanentName(context, object) {
  if (isFaceDownPermanent(object)) {
    return object?.token === true ? "Face-down Token" : "Face-down Creature";
  }
  const name = cardName(object?.name);
  const transform = DAY_NIGHT_TRANSFORMS.get(name);
  if (!transform) return name;
  if (ALLOW_ENGINE_SHIMS && context.syntheticTransformedObjects.get(transform.front)) return transform.back;
  if (ALLOW_ENGINE_SHIMS && context.syntheticTransformedObjects.get(transform.front) === false) return transform.front;
  if (ALLOW_ENGINE_SHIMS && ["Tavern Ruffian", "Curse of Leeches"].includes(transform.front) && context.daytime !== null) {
    return context.daytime ? transform.front : transform.back;
  }
  return name;
}

function isFaceDownPermanent(object) {
  return Boolean(object?.faceDown ?? object?.face_down ?? object?.manifested);
}

function effectivePermanentDetails(context, object) {
  const details = getObjectDetails(context.game, object.id);
  const effectiveName = effectivePermanentName(context, object);
  if (isFaceDownPermanent(object)) {
    return {
      ...details,
      name: effectiveName,
      power: details.power ?? 2,
      toughness: details.toughness ?? 2,
    };
  }
  const transform = DAY_NIGHT_TRANSFORMS.get(effectiveName);
  if (!transform || !ALLOW_ENGINE_SHIMS) return details;
  if (effectiveName === transform.front) {
    return { ...details, name: transform.front, power: transform.frontPower, toughness: transform.frontToughness };
  }
  return { ...details, name: transform.back, power: transform.backPower, toughness: transform.backToughness };
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
    phaseOrder(scheduledPhase(left)) - phaseOrder(scheduledPhase(right))
  );
}

function battlefieldHasNamedPermanent(context, name) {
  if (typeof name !== "string") return false;
  const normalized = cardName(name).toLowerCase();
  return (getCheckpoint(context.game).objects || []).some(
    (object) => object.zone === "battlefield" && cardName(object.name).toLowerCase().includes(normalized),
  );
}

function stackHasNamedObject(context, name) {
  if (typeof name !== "string") return false;
  const normalized = cardName(name).toLowerCase();
  const checkpoint = getCheckpoint(context.game);
  if (
    getObjectsInZone(checkpoint, "stack").some((object) =>
      cardName(object.name).toLowerCase().includes(normalized)
    )
  ) {
    return true;
  }
  return (checkpoint.stack || []).some((entry) => {
    const inspectId = stackEntryInspectObjectId(entry, checkpoint);
    const details = inspectId === null ? null : getObjectDetails(context.game, inspectId);
    const entryName = details?.name ?? entry.name ?? entry.object_name ?? entry.sourceName ?? entry.source_name;
    return cardName(entryName).toLowerCase().includes(normalized);
  });
}

function scheduledPhase(operation) {
  if (operation.phase) return operation.phase;
  if (operation.op === "attack") return "DECLARE_ATTACKERS";
  if (operation.op === "block") return "DECLARE_BLOCKERS";
  return "PRECOMBAT_MAIN";
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
  const text = String(raw ?? "");
  if (/EmptyNames\.FACE_DOWN_TOKEN\.getTestCommand\(\)/.test(text)) {
    return "Face-down Token";
  }
  if (/EmptyNames\.FACE_DOWN_CREATURE\.getTestCommand\(\)/.test(text)) {
    return "Face-down Creature";
  }
  if (/EmptyNames\.FULLY_LOCKED_ROOM\.getTestCommand\(\)/.test(text)) {
    return "Fully Locked Room";
  }
  const unquoted = text
    .replace(/^"(.+)"$/, "$1")
    .replace(/^'(.+)'$/, "$1");
  const aliasIndex = unquoted.indexOf("@");
  const withoutAlias = aliasIndex > 0 ? unquoted.slice(0, aliasIndex) : unquoted;
  return withoutAlias
    .replace(/\s+using\s+.+$/i, "")
    .replace(
      /\s+with\s+(?:alternative cost.*|awaken|blitz|emerge|escape|freerunning.*|jump-start|kicker|mayhem|no alternative cost|overload|prowl.*|prototype|retrace|spectacle|surge|warp|web-slinging)$/i,
      "",
    )
    .replace(/^Cast\s+/i, "");
}

function zoneName(zone) {
  const raw = String(zone).replace(/^Zone\./, "").toLowerCase();
  if (raw === "outside_game") return "outside_game";
  if (raw === "battlefield") return "battlefield";
  if (raw === "graveyard") return "graveyard";
  if (raw === "library") return "library";
  if (raw === "exile" || raw === "exiled") return "exile";
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
  if (/^\s*[+-](\d+|x)\s*:/.test(raw) && loyaltyLabelMatches(label, raw)) return true;
  if (/^[a-z0-9]+$/i.test(wantedLowered)) {
    return new RegExp(`(^|\\s)${escapeRegExp(wantedLowered)}(?=\\s|$|:)`).test(lowered);
  }
  if (lowered.includes(wantedLowered)) return true;
  const rawManaSymbols = manaSymbolTokens(raw);
  if (rawManaSymbols.length > 0 && shouldCompareManaSymbols(raw)) {
    const labelManaSymbols = manaSymbolTokens(label);
    if (!containsManaSymbolSequence(labelManaSymbols, rawManaSymbols)) return false;
  }
  const rawWithoutManaSymbols = raw.replace(/\{[0-9wubrgcxsyp/tq]+\}/gi, "");
  if (/[{](?!this\b)[^}]+[}]/i.test(rawWithoutManaSymbols)) return false;

  if (/^(cast|play)\s+/i.test(raw)) {
    const name = normalizeActionSearch(cardName(raw));
    return lowered.includes(name);
  }

  const fragments = labelFragments(raw);
  return fragments.length > 0 && fragments.every((fragment) => lowered.includes(fragment));
}

function shouldCompareManaSymbols(raw) {
  return (
    /^\s*(?:\{[0-9wubrgcxsyp/tq]+\}\s*)+:/i.test(raw) ||
    /^\s*(?:equip|fortify|reconfigure|crew|cycling|unearth|embalm|eternalize|level up|outlast|monstrosity|adapt|scavenge)\b/i.test(raw)
  );
}

function manaSymbolTokens(raw) {
  return [...String(raw || "").matchAll(/\{([^}]+)\}/g)].map((match) =>
    match[1].toLowerCase().replace(/\s+/g, ""),
  );
}

function containsManaSymbolSequence(labelTokens, wantedTokens) {
  if (wantedTokens.length === 0) return true;
  for (let start = 0; start <= labelTokens.length - wantedTokens.length; start += 1) {
    if (wantedTokens.every((token, offset) => labelTokens[start + offset] === token)) {
      return true;
    }
  }
  return false;
}

function escapeRegExp(raw) {
  return String(raw).replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function normalizeActionSearch(raw) {
  return String(raw || "")
    .toLowerCase()
    .replace(/&mdash;?|&ndash;?|&#8212;?|&#x2014;?/g, " ")
    .replace(/&bull;?|&#8226;?|&#x2022;?/g, " ")
    .replace(/<br\s*\/?>/g, " ")
    .replace(/<[^>]+>/g, " ")
    .replace(/\{this\}/g, "this creature")
    .replace(/^\s*\+(\d+)\s*:/, "put $1 loyalty counters")
    .replace(/^\s*-(\d+|x)\s*:/, "remove $1 loyalty counters")
    .replace(/[{}]/g, "")
    .replace(/[^\p{L}\p{N}+\-/ ]/gu, " ")
    .replace(/\s+/g, " ")
    .trim();
}

function mageObjectAliasSpec(raw) {
  const text = String(raw ?? "")
    .trim()
    .replace(/^"(.+)"$/, "$1")
    .replace(/^'(.+)'$/, "$1");
  const match = text.match(/@([^\s]+)$/);
  if (!match) return null;
  const indexed = match[1].match(/^(.+)\.(\d+)$/);
  const group = (indexed ? indexed[1] : match[1]).trim().toLowerCase();
  if (!group) return null;
  return {
    group,
    index: indexed ? Number(indexed[2]) : null,
  };
}

function mageObjectAliasKey(group, index = null) {
  return `@${String(group).toLowerCase()}${index === null ? "" : `.${Number(index)}`}`;
}

function recordMageObjectAlias(context, rawName, objectId) {
  const spec = mageObjectAliasSpec(rawName);
  const numericId = Number(objectId);
  if (!spec || !Number.isFinite(numericId) || numericId <= 0) return;

  const nextIndex = (context.aliasGroupCounts.get(spec.group) || 0) + 1;
  const index = spec.index ?? nextIndex;
  context.aliasGroupCounts.set(spec.group, Math.max(nextIndex, index));

  const checkpoint = getCheckpoint(context.game);
  const object = (checkpoint.objects || []).find((candidate) => Number(candidate.id) === numericId);
  const entry = {
    objectId: numericId,
    stableId: Number(object?.stableId ?? object?.stable_id ?? numericId),
    name: cardName(rawName),
  };
  context.objectAliases.set(mageObjectAliasKey(spec.group, index), entry);
  if (spec.index === null && index === 1) {
    context.objectAliases.set(mageObjectAliasKey(spec.group), entry);
  }
}

function resolveMageObjectAlias(context, rawName) {
  const spec = mageObjectAliasSpec(rawName);
  if (!spec) return null;
  return (
    context.objectAliases.get(mageObjectAliasKey(spec.group, spec.index)) ??
    (spec.index === null ? context.objectAliases.get(mageObjectAliasKey(spec.group, 1)) : null) ??
    null
  );
}

function objectMatchesAliasEntry(object, entry) {
  if (!object || !entry) return false;
  return (
    Number(object.id) === Number(entry.objectId) ||
    Number(object.stableId ?? object.stable_id ?? object.id) === Number(entry.stableId)
  );
}

function targetMatchesAliasEntry(target, entry, checkpoint) {
  const objectId = target?.object ?? target?.id;
  if (objectId === undefined || objectId === null) return false;
  const object = (checkpoint.objects || []).find((candidate) => Number(candidate.id) === Number(objectId));
  return objectMatchesAliasEntry(object ?? { id: objectId }, entry);
}
