import test from "node:test";
import assert from "node:assert/strict";
import {
  applySupportedJavaHelper, assertLife, assertTokenCount, castSpell,
  maybeEnterPendingAdditionalCombat, queuePendingAdditionalCombatsFromStack,
  chooseObjectCandidates,
  assertAbility, assertAttacking, assertBlitzed,
  assertBlitzAutomatonPrototypeState,
  assertCounterCount, assertStackObject,
} from "./mage-port-runner.mjs";

function nativeInspectionMock(state, extra = {}) {
  const objects = () => (state.objects || []).map((object, index) => ({ ...object, id: object.id ?? index + 1,
    zone: object.zone ?? ((state.battlefield || []).includes(object.id) ? "battlefield" : "outside_game"),
  }));
  const details = extra.objectDetails;
  return {
    getHiddenCardState: () => ({ ...state, objects: objects(), players: (state.players || []).map((player, id) => ({ ...player, id: player.id ?? id })), exile: state.exile || [] }),
    exportPublicAuditCheckpoint: () => ({ ...state, objects: objects(), players: (state.players || []).map((player, id) => ({ ...player, id: player.id ?? id })) }),
    uiState: () => ({ perspective: state.perspective || 0, stack_objects: state.stack || [] }),
    ...extra,
    objectDetails: id => ({ ...objects().find(object => Number(object.id) === Number(id)), ...details?.(id) }),
  };
}

test("stack assertions distinguish player seats and same-name object identities", async () => {
  const objects = [
    { id: 10, stableId: 9, name: "Grizzly Bears" },
    { id: 12, stableId: 11, name: "Grizzly Bears" },
  ];
  const stackObjects = [{ name: "Test ability", ability_kind: "Triggered", targets: [
    { kind: "player", player: 2 },
    { kind: "object", object: 10, name: "Grizzly Bears" },
  ] }];
  const context = {
    game: nativeInspectionMock({ objects }, { uiState: () => ({ stack_objects: stackObjects }) }),
    objectAliases: new Map([
      ["@first", { objectId: 9, stableId: 9 }],
      ["@second", { objectId: 11, stableId: 11 }],
    ]),
  };
  const checkpoint = { source: "Test ability", count: 1, playerTarget: 2, objectTarget: "@first" };
  await assertStackObject(context, checkpoint);
  await assert.rejects(assertStackObject(context, { ...checkpoint, playerTarget: 3 }), /got 0/);
  await assert.rejects(assertStackObject(context, { ...checkpoint, objectTarget: "@second" }), /got 0/);
});

test("a cast can select permission granted by a permanent that changed zones", async () => {
  const cards = [
    { id: 10, stableId: 10, name: "Grizzly Bears", zone: "exile" },
    { id: 21, stableId: 20, name: "Dauthi Voidwalker", zone: "graveyard" },
  ];
  const actions = [30, 20].map(source => ({
    kind: "cast_spell",
    action_ref: {
      kind: "cast_spell",
      spell_id: 10,
      casting_method: { kind: "play_from", source },
    },
  }));
  const state = { perspective: 0, priority_player: 0, decision: { kind: "priority", player: 0, actions } };
  const commands = [];
  const context = {
    game: nativeInspectionMock({ objects: cards }, {
      uiState: () => state,
      dispatch: command => { commands.push(command); return state; },
    }),
    objectAliases: new Map([
      ["@bear", { objectId: 10, stableId: 10 }],
      ["@walker", { objectId: 20, stableId: 20 }],
    ]),
    pendingScheduledAvailability: 0,
    availableScheduledCount: 0,
  };
  await castSpell(context, {
    player: 0, name: "Grizzly Bears", objectAlias: "@bear", permissionSource: "@walker", holdStack: true,
  });
  assert.deepEqual(commands, [{ type: "priority_action", action_ref: actions[1].action_ref }]);
});

test("player counter assertions inspect the requested player and reject unavailable counter state", async () => {
  const checkpoint = { players: [
    { id: 0, counters: [{ kind: "rad", amount: 4 }] },
    { id: 1, counters: [{ kind: "poison", amount: 3 }] },
  ] };
  const context = { game: { snapshot: () => checkpoint } };
  const operation = { player: 0, name: 1, counter: "POISON", count: 3 };
  await assertCounterCount(context, operation);
  await assertCounterCount(context, { ...operation, name: 0, counter: "RAD", count: 4 });
  await assertCounterCount(context, { ...operation, counter: "RAD", count: 0 });
  await assert.rejects(assertCounterCount(context, { ...operation, count: 0 }), /player 1, got 3/);
  delete checkpoint.players[1].counters;
  await assert.rejects(assertCounterCount(context, { ...operation, counter: "ENERGY", count: 0 }), /unsupported player counter/);
});
import { ALLOW_ENGINE_SHIMS, DEFAULT_LIBRARY_CARD, DEFAULT_LIBRARY_SIZE } from "./mage-port-runner/names.mjs";
import { startEmptyMatch } from "./wasm-test-harness.mjs";

function contextWithLife(life, extra = {}) {
  const checkpoint = { players: [{ life: 20 }, { life }], objects: [] };
  return {
    checkpoint,
    context: { game: nativeInspectionMock(checkpoint), ...extra },
  };
}

test("life assertions reject Illusions of Grandeur mismatches without repairing the checkpoint", async () => {
  const { context, checkpoint } = contextWithLife(20);
  checkpoint.objects.push({ name: "Illusions of Grandeur" });
  await assert.rejects(assertLife(context, { player: 1, life: 21 }), /expected life 21.*got 20/);
  assert.equal(checkpoint.players[1].life, 20);
});

test("life assertions reject Brimstone Vandal mismatches independent of scenario name", async () => {
  for (const expected of [19, 12]) {
    const { context, checkpoint } = contextWithLife(20, {
      sourcePath: "scripts/cards/DayNightTest.java",
      testName: "testBrimstoneVandalTrigger",
    });
    await assert.rejects(assertLife(context, { player: 1, life: expected }), /got 20/);
    assert.equal(checkpoint.players[1].life, 20);
  }
});

test("matching life assertions accept observed state without changing it", async () => {
  const { context, checkpoint } = contextWithLife(21);
  await assertLife(context, { player: 1, life: 21 });
  assert.deepEqual(checkpoint.players, [{ life: 20 }, { life: 21 }]);
});

test("starting-life expressions read the fixture's declared starting life and reject unknown expressions", async () => {
  const { context, checkpoint } = contextWithLife(28);
  checkpoint.players[1].startingLife = 30;
  await assertLife(context, { player: 1, life: "currentGame.getStartingLife() - 2" });
  await assert.rejects(assertLife(context, { player: 1, life: "currentGame.getStartingLife()" }), /got 28/);
  await assert.rejects(assertLife(context, { player: 1, life: "unresolvedVariable" }), /unsupported numeric life assertion/);
  delete checkpoint.players[1].startingLife;
  await assert.rejects(assertLife(context, { player: 1, life: "currentGame.getStartingLife()" }), /unsupported starting-life assertion/);
  assert.equal(checkpoint.players[1].life, 28);
});

test("malformed color assertions cannot silently pass", async () => {
  await assert.rejects(applySupportedJavaHelper({}, {
    source: 'checkColor("fixture", 1, PhaseStep.PRECOMBAT_MAIN, playerA, "Grizzly Bears", "green", true)',
  }), /unsupported Java statement/);
});

test("named token assertions require exact counts including zero", async () => {
  const checkpoint = {
    players: [{ id: 0 }], battlefield: [1, 2],
    objects: [1, 2].map(id => ({ id, name: "Grizzly Bears", controller: 0, token: true })),
  };
  const context = { game: nativeInspectionMock(checkpoint, {
    objectDetails: () => ({ kind: "token" }),
  }) };
  await assert.rejects(assertTokenCount(context, { player: 0, name: "Grizzly Bears", count: 0 }), /expected 0.*got 2/);
  await assert.rejects(assertTokenCount(context, { player: 0, name: "Grizzly Bears", count: 1 }), /expected 1.*got 2/);
  await assertTokenCount(context, { player: 0, name: "Grizzly Bears", count: 2 });
});

test("a missing legal cast action cannot be repaired by moving the card directly into play", async () => {
  assert.equal(ALLOW_ENGINE_SHIMS, false, "integrity tests require engine shims disabled");
  const checkpoint = {
    perspective: 0, players: [{ id: 0, hand: [1] }], stack: [], battlefield: [],
    objects: [{ id: 1, name: "Grizzly Bears", owner: 0, zone: "hand" }],
  };
  let imports = 0;
  const context = { game: nativeInspectionMock(checkpoint, {
    dispatch: () => { imports++; },
    uiState: () => ({ perspective: 0, priority_player: 0, decision: { kind: "priority", player: 0, actions: [] } }),
  }) };
  await assert.rejects(castSpell(context, { player: 0, name: "Grizzly Bears" }), /could not find cast action/);
  assert.equal(imports, 0);
  assert.equal(checkpoint.objects[0].zone, "hand");
});

test("ordinary audit runs never synthesize additional combat from rendered text", () => {
  assert.equal(ALLOW_ENGINE_SHIMS, false, "integrity tests require engine shims disabled");
  const context = { pendingAdditionalCombats: 1, game: {
    getHiddenCardState: () => { throw new Error("must not read rendered stack text"); },
    uiState: () => { throw new Error("must not force an additional phase"); },
  } };
  queuePendingAdditionalCombatsFromStack(context);
  maybeEnterPendingAdditionalCombat(context, { turn: 1, player: 0 });
  assert.equal(context.pendingAdditionalCombats, 1);
});

test("empty match fixtures can pin the starting player without changing random-start defaults", () => {
  const game = { startMatch: options => options };
  assert.equal(startEmptyMatch(game, { startingPlayer: 0 }).startingPlayer, 0);
  assert.equal(Object.hasOwn(startEmptyMatch(game), "startingPlayer"), false);
});

test("MAGE default library preserves the upstream fetchable Mountain fixture", () => {
  assert.equal(DEFAULT_LIBRARY_CARD, "Mountain");
  assert.equal(DEFAULT_LIBRARY_SIZE, 71);
});

test("unspecified nonstrict optional object choices exercise the effect while explicit skips are honored", () => {
  const first = { id: 1, name: "Mountain", legal: true };
  const decision = { min: 0, max: 1, candidates: [first, { id: 2, name: "Mountain", legal: true }] };
  assert.deepEqual(chooseObjectCandidates(decision, undefined), [first]);
  assert.deepEqual(chooseObjectCandidates(decision, []), []);
});

test("malformed imported ability assertions and unavailable blitz state visibly fail", async () => {
  await assert.rejects(assertAbility({}, { name: "PRECOMBAT_MAIN", ability: 0 }),
    /unsupported ability assertion/);
  await assert.rejects(assertBlitzed({}, { name: "Hasty creature", expected: true }),
    /unsupported blitz assertion/);
});

test("attacking assertions read combat membership rather than tapped state", async () => {
  const checkpoint = { players: [{ id: 0 }], battlefield: [1],
    objects: [{ id: 1, name: "Grizzly Bears", controller: 0, tapped: true }] };
  let combat = null;
  const context = { game: nativeInspectionMock(checkpoint, {
    uiState: () => ({ combat, phase: "Combat", step: "EndCombat" }),
  }) };
  await assert.rejects(assertAttacking(context, { player: 0, name: "Grizzly Bears", expected: true }),
    /attacking=true, got false/);
  combat = { attackers: [{ creature: 1 }] };
  checkpoint.objects[0].tapped = false;
  await assertAttacking(context, { player: 0, name: "Grizzly Bears", expected: true });
  context.game.uiState = () => ({});
  await assert.rejects(assertAttacking(context, { player: 0, name: "Grizzly Bears", expected: false }),
    /unsupported attacking assertion/);
});

test("prototype color assertions cannot infer calculated colors from mana cost", async () => {
  const checkpoint = { players: [{ id: 0 }], battlefield: [1],
    objects: [{ id: 1, name: "Blitz Automaton", controller: 0 }] };
  const context = { game: nativeInspectionMock(checkpoint, {
    objectDetails: () => ({ name: "Blitz Automaton", power: 3, toughness: 2,
      mana_cost: "{2}{R}", abilities: ["Haste"] }),
  }) };
  await assert.rejects(assertBlitzAutomatonPrototypeState(context, { count: 1, prototyped: true }),
    /unsupported color assertion/);
});
