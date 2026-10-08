import test from 'node:test';
import assert from 'node:assert/strict';
import { createPriorityStops } from '../src/lib/priority-stops.js';

const state = (changes = {}) => ({
  turn_number: 2, active_player: 1, perspective: 0, phase: 'beginning phase', step: 'upkeep',
  stack_size: 0, stack_objects: [], decision: { kind: 'priority', player: 0, actions: [] }, ...changes,
});
const pass = { type: 'priority_action', action_ref: { kind: 'pass_priority' } };
const cast = { type: 'priority_action', action_ref: { kind: 'cast_spell' } };
const onStack = (id = 10) => state({ stack_size: 1, stack_objects: [{ id, controller: 0 }] });
function castWindow() {
  let clock = 100;
  const stops = createPriorityStops({ now: () => clock });
  stops.beforeCommand(cast, state());
  stops.observe(state({ decision: { kind: 'mana_payment', player: 0 } }));
  assert.equal(stops.getSnapshot().window, null);
  stops.observe(onStack());
  const id = stops.getSnapshot().window.id;
  stops.armWindow(id);
  return { stops, id, tick: ms => { clock += ms; } };
}

test('tracker cycles off / once / recurring / off', () => {
  const stops = createPriorityStops();
  for (const expected of ['once', 'always', undefined]) {
    stops.cycleStop('phase:Upkeep', state());
    assert.equal(stops.getSnapshot().stops['phase:Upkeep'], expected);
  }
});

test('one-time stops wait for local priority and consume only on explicit action', () => {
  const stops = createPriorityStops();
  stops.cycleStop('phase:Upkeep', state({ step: 'draw' }));
  assert.equal(stops.stopReason(state({ decision: { kind: 'priority', player: 1 } })), null);
  assert.match(stops.stopReason(state()), /Upkeep/);
  assert.match(stops.stopReason(state()), /Upkeep/);
  assert.equal(stops.getSnapshot().stops['phase:Upkeep'], 'once');
  stops.beforeCommand(pass, state());
  assert.equal(stops.stopReason(state()), null);
  assert.equal(stops.getSnapshot().stops['phase:Upkeep'], undefined);
});

test('recurring stops do not retrigger in the same step, but return next turn', () => {
  const stops = createPriorityStops();
  stops.cycleStop('phase:Upkeep', state());
  stops.cycleStop('phase:Upkeep', state());
  stops.beforeCommand(pass, state());
  assert.equal(stops.stopReason(onStack()), null);
  assert.match(stops.stopReason(state({ turn_number: 3 })), /Upkeep/);
});

test('combat step stops remain independent of a broad combat stop and rearm for extra combat', () => {
  const stops = createPriorityStops();
  const combat = step => state({ phase: 'combat phase', step });
  stops.cycleStop('phase:Combat', combat('begin combat'));
  stops.cycleStop('phase:Combat', combat('begin combat'));
  stops.cycleStop('step:DeclareBlockers', combat('begin combat'));
  assert.match(stops.stopReason(combat('begin combat')), /Combat/);
  stops.beforeCommand(pass, combat('begin combat'));
  assert.equal(stops.stopReason(combat('declare attackers')), null);
  assert.match(stops.stopReason(combat('declare blockers')), /DeclareBlockers/);
  stops.beforeCommand(pass, combat('declare blockers'));
  stops.observe(combat('end combat'));
  assert.match(stops.stopReason(combat('begin combat')), /Combat/);
});

test('post-cast delay starts at presentation, and unrelated analysis does not reset it', () => {
  const { stops, id, tick } = castWindow();
  tick(1999);
  stops.observe({ ...onStack(), snapshot_id: 50, __priority_revision: 5 });
  assert.equal(stops.expire(id, onStack()), false);
  tick(1);
  assert.equal(stops.expire(id, onStack()), true);
  assert.equal(stops.getSnapshot().window, null);
});

test('clicking Hold Priority cancels the deadline until the user explicitly acts', () => {
  const { stops, id, tick } = castWindow();
  assert.equal(stops.hold(onStack()), true);
  tick(3000);
  assert.equal(stops.expire(id, onStack()), false);
  assert.match(stops.stopReason(onStack()), /held/);
  stops.beforeCommand(pass, onStack());
  assert.equal(stops.stopReason(onStack()), null);
});

test('foreign priority or a changed stack cancels stale timers', () => {
  const { stops, id, tick } = castWindow();
  tick(2000);
  const foreign = onStack(); foreign.decision.player = 1;
  assert.equal(stops.expire(id, foreign), false);
  assert.equal(stops.getSnapshot().window, null);
  const next = castWindow();
  next.tick(2000);
  assert.equal(next.stops.expire(next.id, onStack(11)), false);
});

test('mana activations never create a post-action timer', () => {
  const stops = createPriorityStops();
  stops.beforeCommand({ type: 'priority_action', action_ref: { kind: 'activate_mana_ability' } }, state());
  stops.observe(state());
  assert.equal(stops.getSnapshot().window, null);
  assert.equal(stops.stopReason(state()), null);
});

test('ordinary activated abilities use the same window as spells', () => {
  const stops = createPriorityStops();
  stops.beforeCommand({ type: 'priority_action', action_ref: { kind: 'activate_ability' } }, state());
  stops.observe(onStack());
  assert.ok(stops.getSnapshot().window);
});

test('echoes of the pre-command snapshot cannot clear an announcing action', () => {
  const stops = createPriorityStops();
  stops.beforeCommand(cast, state());
  stops.observe(state({ __priority_revision: 6 }));
  assert.match(stops.stopReason(state()), /window/);
  stops.observe(onStack());
  assert.ok(stops.getSnapshot().window);
});

test('cancelling a cast clears its pending window without consuming future step stops', () => {
  const stops = createPriorityStops();
  stops.cycleStop('phase:Draw', state());
  stops.beforeCommand(cast, state());
  stops.cancelAction();
  stops.observe(state());
  assert.equal(stops.getSnapshot().window, null);
  assert.match(stops.stopReason(state({ step: 'draw' })), /Draw/);
});

test('a stop set during the post-cast window prevents its automatic pass', () => {
  const { stops, id, tick } = castWindow();
  stops.cycleStop('phase:Upkeep', onStack());
  tick(2000);
  assert.equal(stops.expire(id, onStack()), false);
  assert.match(stops.stopReason(onStack()), /Upkeep/);
});

test('failed submissions restore a one-time stop instead of silently consuming it', () => {
  const stops = createPriorityStops();
  stops.cycleStop('phase:Upkeep', state());
  const checkpoint = stops.beforeCommand(pass, state());
  assert.equal(stops.getSnapshot().stops['phase:Upkeep'], undefined);
  stops.rollbackAction(checkpoint, state());
  assert.equal(stops.getSnapshot().stops['phase:Upkeep'], 'once');
});


test('first-strike and regular damage have distinct recurring stops', () => {
  const stops = createPriorityStops();
  const damage = state({ phase: 'combat phase', step: 'combat damage', combat_damage_step: 'first_strike' });
  for (const key of ['FirstStrikeDamage', 'CombatDamage']) {
    stops.cycleStop(`step:${key}`, state());
    stops.cycleStop(`step:${key}`, state());
  }
  assert.equal(stops.stopReason(damage), 'stop at FirstStrikeDamage');
  stops.beforeCommand(pass, damage);
  assert.equal(stops.stopReason(damage), null);
  const regular = { ...damage, combat_damage_step: 'regular' };
  assert.equal(stops.stopReason(regular), 'stop at CombatDamage');
  stops.beforeCommand(pass, regular);
  assert.equal(stops.stopReason(regular), null);
  assert.equal(stops.stopReason({ ...damage, turn_number: 3 }), 'stop at FirstStrikeDamage');
});

test('resuming phase passing releases a manual hold but preserves phase stops', () => {
  const { stops } = castWindow();
  stops.hold(onStack());
  stops.cycleStop('phase:Upkeep', onStack());
  stops.resume(onStack());
  assert.equal(stops.stopReason(onStack()), 'stop at Upkeep');
  assert.equal(stops.getSnapshot().stops['phase:Upkeep'], 'once');
});
