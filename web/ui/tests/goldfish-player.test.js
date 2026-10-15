import test from "node:test";
import assert from "node:assert/strict";
import { selectGoldfishPlayer } from "../src/lib/goldfish-player.js";
import { createPriorityStops } from "../src/lib/priority-stops.js";

const priority = player => ({ kind: "priority", player, actions: [
  { kind: "pass_priority", label: "Pass priority", index: 0, action_ref: { kind: "pass_priority" } },
] });
const snapshot = (active, player, stack = 0) => ({
  players: [{ id: 0 }, { id: 1 }, { id: 2 }],
  active_player: active, perspective: 0, turn_number: 1, phase: "Main", step: null,
  stack_size: stack, stack_objects: stack ? [{ id: 10 }] : [], decision: priority(player),
});
function fixture(states) {
  let index = 0;
  let perspective = 0;
  const commands = [];
  const game = {
    uiState: async () => ({ ...states[index], perspective }),
    setPerspective: async id => { perspective = id; },
    dispatch: async command => {
      commands.push(command);
      assert.ok(index + 1 < states.length, "must stop before passing the selected player's decision");
      index++;
      return game.uiState();
    },
  };
  return { game, commands };
}

test("occupied stack stops at selected player's priority during someone else's turn", async () => {
  const { game, commands } = fixture([snapshot(0, 0, 1), snapshot(0, 1, 1), snapshot(0, 2, 1)]);
  assert.deepEqual(await selectGoldfishPlayer(game, 2), { reached: true });
  assert.equal(commands.length, 2);
  assert.equal((await game.uiState()).stack_size, 1);
  assert.equal((await game.uiState()).active_player, 0);
});

test("empty stack passes off-turn priority and intervening turns", async () => {
  const { game, commands } = fixture([
    snapshot(0, 0), snapshot(0, 2), snapshot(1, 1), snapshot(1, 2), snapshot(2, 2),
  ]);
  assert.deepEqual(await selectGoldfishPlayer(game, 2), { reached: true });
  assert.equal(commands.length, 4);
});

test("selecting the current viewpoint still advances to its turn", async () => {
  const { game, commands } = fixture([snapshot(1, 1), snapshot(0, 0)]);
  assert.deepEqual(await selectGoldfishPlayer(game, 0), { reached: true });
  assert.equal(commands.length, 1);
});

test("required resolution choices pause with the deciding player's viewpoint", async () => {
  const choice = { ...snapshot(0, 1, 1), decision: { kind: "select_targets", player: 1 } };
  const { game, commands } = fixture([snapshot(0, 0, 1), choice]);
  assert.deepEqual(await selectGoldfishPlayer(game, 2), { reached: false });
  assert.equal(commands.length, 1);
  assert.equal((await game.uiState()).perspective, 1);
});

test("pregame/custom pass actions are not silently confirmed", async () => {
  const state = snapshot(0, 0);
  state.decision.actions[0].label = "Keep opening hand";
  const { game, commands } = fixture([state]);
  assert.deepEqual(await selectGoldfishPlayer(game, 1), { reached: false });
  assert.equal(commands.length, 0);
  assert.equal((await game.uiState()).perspective, 0);
});

test("a phase without a decision advances into the selected player's turn", async () => {
  const { game, commands } = fixture([{ ...snapshot(0, 0), decision: null }, snapshot(1, 1)]);
  game.advancePhase = () => game.dispatch({ type: "test_phase_advance" });
  assert.deepEqual(await selectGoldfishPlayer(game, 1), { reached: true });
  assert.equal(commands.length, 1);
});

test("optional combat passes without attacking; mandatory attackers pause", async () => {
  for (const mustAttack of [false, true]) {
    const combat = { ...snapshot(0, 0), decision: {
      kind: "attackers", player: 0, attacker_options: [{ must_attack: mustAttack }],
    } };
    const { game, commands } = fixture([combat, snapshot(1, 1)]);
    assert.deepEqual(await selectGoldfishPlayer(game, 1), { reached: !mustAttack });
    assert.deepEqual(commands, mustAttack ? [] : [{ type: "declare_attackers", declarations: [] }]);
  }
});

test("manual seat selection holds priority until the player acts without changing saved stops", () => {
  const stops = createPriorityStops();
  const state = { ...snapshot(0, 1, 1), perspective: 1 };
  stops.holdCurrentPriority(state);
  assert.ok(stops.stopReason(state));
  assert.deepEqual(stops.getSnapshot().stops, {});
  stops.beforeCommand({ type: "priority_action", action_index: 0 }, state);
  assert.equal(stops.stopReason(state), null);
});
