import test from "node:test";
import assert from "node:assert/strict";
import {
  loopReplayPosition,
  recordLoopStep,
  resolveLoopStep,
} from "../src/lib/loop-shortcut.js";

function stateWith(decision, battlefield = [], extra = {}) {
  return {
    players: [{ id: 0, battlefield, hand_cards: [] }, { id: 1, battlefield: [], hand_cards: [] }],
    decision,
    ...extra,
  };
}

const altar = { id: 10, stable_id: 900, name: "Ashnod's Altar" };
const tokenA = { id: 11, stable_id: 901, name: "Zombie", token: true };

function priority(actions) {
  return { kind: "priority", player: 0, actions };
}

test("priority actions are recorded by stable identity and replayed after zone changes", () => {
  const activate = { index: 3, kind: "activate_ability", label: "Ashnod's Altar: sacrifice", object_id: 10, ability_index: 0,
    action_ref: { kind: "activate_ability", source: 10, ability_index: 0 } };
  const recorded = recordLoopStep({ type: "priority_action", action_ref: activate.action_ref },
    stateWith(priority([{ index: 0, kind: "pass_priority", label: "Pass priority", action_ref: { kind: "pass_priority" } }, activate]), [altar]));
  assert.deepEqual(recorded.action.object, { stable_id: 900, name: "Ashnod's Altar" });
  assert.equal(JSON.stringify(recorded).includes('"source":10'), false, "no raw ObjectIds in a step");

  // The Altar flickered: new ObjectId, same stable id.
  const moved = { ...altar, id: 40 };
  const nextActivate = { ...activate, index: 1, object_id: 40, action_ref: { kind: "activate_ability", source: 40, ability_index: 0 } };
  const resolved = resolveLoopStep(recorded, stateWith(priority([nextActivate]), [moved]));
  assert.deepEqual(resolved.command, { type: "priority_action", action_index: 1, action_ref: nextActivate.action_ref });

  const gone = resolveLoopStep(recorded, stateWith(priority([{ index: 0, kind: "pass_priority", label: "Pass priority",
    action_ref: { kind: "pass_priority" } }]), []));
  assert.match(gone.error, /no longer available/);
});

test("a fresh token made by each repetition is matched by name when it is the only candidate", () => {
  const targets = {
    kind: "targets", player: 0,
    requirements: [{ legal_targets: [{ kind: "object", object: 11, name: "Zombie" }, { kind: "player", player: 1, name: "Bob" }] }],
  };
  const recorded = recordLoopStep({ type: "select_targets", targets: [{ kind: "object", object: 11 }, { kind: "player", player: 1 }] },
    stateWith(targets, [tokenA]));
  const tokenB = { id: 52, stable_id: 950, name: "Zombie", token: true };
  const next = {
    ...targets,
    requirements: [{ legal_targets: [{ kind: "object", object: 52, name: "Zombie" }, { kind: "player", player: 1, name: "Bob" }] }],
  };
  assert.deepEqual(resolveLoopStep(recorded, stateWith(next, [tokenB])).command, {
    type: "select_targets",
    targets: [{ kind: "object", object: 52 }, { kind: "player", player: 1 }],
  });
  const twoTokens = { ...next, requirements: [{ legal_targets: [
    { kind: "object", object: 52, name: "Zombie" }, { kind: "object", object: 53, name: "Zombie" }] }] };
  assert.match(resolveLoopStep(recorded, stateWith(twoTokens, [tokenB, { id: 53, stable_id: 951, name: "Zombie", token: true }])).error,
    /can no longer be targeted/);
});

test("options must still mean the same thing, payments re-plan, and the decision kind must match", () => {
  const options = { kind: "select_options", player: 0, options: [{ index: 0, description: "Draw a card" }, { index: 1, description: "Gain 2 life" }] };
  const step = recordLoopStep({ type: "select_options", option_indices: [1] }, stateWith(options));
  assert.deepEqual(resolveLoopStep(step, stateWith(options)).command, { type: "select_options", option_indices: [1] });
  const reordered = { ...options, options: [{ index: 0, description: "Draw a card" }, { index: 1, description: "Scry 1" }] };
  assert.match(resolveLoopStep(step, stateWith(reordered)).error, /no longer offered/);
  assert.match(resolveLoopStep(step, stateWith(priority([]))).error, /expected a select options decision/);

  const payment = recordLoopStep({ type: "mana_payment", response: { action: "confirm", plan_id: "1", request_hash: "a" } },
    stateWith({ kind: "mana_payment", player: 0 }));
  assert.deepEqual(resolveLoopStep(payment, stateWith({ kind: "mana_payment", player: 0 }, [], { mana_payment: { plan_id: 7, request_hash: "b" } })).command,
    { type: "mana_payment", response: { action: "confirm", plan_id: "7", request_hash: "b" } });
  assert.equal(recordLoopStep({ type: "propose_loop_shortcut", steps: [], iterations: 1 }, stateWith(priority([]))), null);
});

test("replay position cycles through the recorded steps", () => {
  assert.deepEqual(loopReplayPosition([1, 2, 3], 0), { iteration: 0, stepIndex: 0 });
  assert.deepEqual(loopReplayPosition([1, 2, 3], 7), { iteration: 2, stepIndex: 1 });
});

test("a missing Ghoul is never replaced by the other same-named Ghoul", () => {
  const first = { id: 100, stable_id: 1000, name: "Butcher Ghoul" };
  const second = { id: 101, stable_id: 1001, name: "Butcher Ghoul" };
  const targetDecision = ids => ({ kind: "targets", player: 0, requirements: [{
    legal_targets: ids.map(object => ({ kind: "object", object, name: "Butcher Ghoul" })),
  }] });
  const target = recordLoopStep({ type: "select_targets", targets: [{ kind: "object", object: first.id }] },
    stateWith(targetDecision([100, 101]), [first, second]));
  const sacrifice = recordLoopStep({ type: "select_objects", object_ids: [first.id] },
    stateWith({ kind: "select_objects", player: 0, candidates: [first, second] }, [first, second]));
  const next = stateWith(targetDecision([101]), [second]);
  // The original may still be in the graveyard waiting for undying, or may
  // have left all visible zones. Neither case permits choosing its partner.
  for (const graveyard_cards of [[{ ...first, id: 102 }], []]) {
    next.players[0].graveyard_cards = graveyard_cards;
    assert.match(resolveLoopStep(target, next).error, /can no longer be targeted/);
    assert.match(resolveLoopStep(sacrifice, { ...next,
      decision: { kind: "select_objects", player: 0, candidates: [second] },
    }).error, /can no longer be chosen/);
  }
  const returned = { ...first, id: 103 };
  assert.deepEqual(resolveLoopStep(target, stateWith(targetDecision([101, 103]), [second, returned])).command,
    { type: "select_targets", targets: [{ kind: "object", object: 103 }] });
});


test("recorded stack resolution uses ordinary passes so interruptions keep every priority window", () => {
  const step = {kind:"priority",type:"resolve_stack"};
  const state = stateWith(priority([{index:0,kind:"pass_priority",action_ref:{kind:"pass_priority"}}]),[],{stack_size:2});
  assert.deepEqual(resolveLoopStep(step,state).command,{type:"priority_action",action_ref:{kind:"pass_priority"}});
});
