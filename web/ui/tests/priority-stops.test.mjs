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
test('tracker cycles off / once / recurring / off', () => {
  const stops = createPriorityStops({ phaseStopsEnabled: true });
  for (const expected of ['once', 'always', undefined]) {
    stops.cycleStop('phase:Upkeep', state());
    assert.equal(stops.getSnapshot().stops['phase:Upkeep'], expected);
  }
});

test('one-time stops wait for local priority and consume only on explicit action', () => {
  const stops = createPriorityStops({ phaseStopsEnabled: true });
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
  const stops = createPriorityStops({ phaseStopsEnabled: true });
  stops.cycleStop('phase:Upkeep', state());
  stops.cycleStop('phase:Upkeep', state());
  stops.beforeCommand(pass, state());
  assert.equal(stops.stopReason(onStack()), null);
  assert.match(stops.stopReason(state({ turn_number: 3 })), /Upkeep/);
});

test('combat step stops remain independent of a broad combat stop and rearm for extra combat', () => {
  const stops = createPriorityStops({ phaseStopsEnabled: true });
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

test('mana activations never create a post-action timer', () => {
  const stops = createPriorityStops({ phaseStopsEnabled: true });
  stops.beforeCommand({ type: 'priority_action', action_ref: { kind: 'activate_mana_ability' } }, state());
  stops.observe(state());
  assert.equal(stops.getSnapshot().window, null);
  assert.equal(stops.stopReason(state()), null);
});

test('cancelling a cast clears its pending window without consuming future step stops', () => {
  const stops = createPriorityStops({ phaseStopsEnabled: true });
  stops.cycleStop('phase:Draw', state());
  stops.beforeCommand(cast, state());
  stops.cancelAction();
  stops.observe(state());
  assert.equal(stops.getSnapshot().window, null);
  assert.match(stops.stopReason(state({ step: 'draw' })), /Draw/);
});

test('failed submissions restore a one-time stop instead of silently consuming it', () => {
  const stops = createPriorityStops({ phaseStopsEnabled: true });
  stops.cycleStop('phase:Upkeep', state());
  const checkpoint = stops.beforeCommand(pass, state());
  assert.equal(stops.getSnapshot().stops['phase:Upkeep'], undefined);
  stops.rollbackAction(checkpoint, state());
  assert.equal(stops.getSnapshot().stops['phase:Upkeep'], 'once');
});

test('first-strike and regular damage have distinct recurring stops', () => {
  const stops = createPriorityStops({ phaseStopsEnabled: true });
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

for (const [key, current] of [
  ['phase:Upkeep', state()],
  ['step:DeclareBlockers', state({ phase: 'combat phase', step: 'declare blockers' })],
]) {
  test(`${key} uses the shared countdown on an opponent's turn`, () => {
    let clock = 100;
    const stops = createPriorityStops({ phaseStopsEnabled: true, now: () => clock, initialStops: { [key]: 'always' } });
    stops.observe(current);
    const window = stops.getSnapshot().window;
    assert.equal(window.kind, 'phase-stop');
    clock += 3000;
    assert.equal(stops.expire(window.id, current), false, 'wait for presentation');
    stops.armWindow(window.id);
    clock += 1999;
    assert.equal(stops.expire(window.id, current), false);
    clock++;
    assert.equal(stops.expire(window.id, current), true);
    stops.beforeCommand(pass, current);
    assert.equal(stops.stopReason(current), null);
    stops.observe({ ...current, turn_number: 3 });
    assert.ok(stops.getSnapshot().window, 'recurring stop returns next turn');
  });
}

test('holding an opponent phase stop cancels its timer; disabling keeps the stop manual', () => {
  const stops = createPriorityStops({ phaseStopsEnabled: true, initialStops: { 'phase:Upkeep': 'once' } });
  stops.observe(state());
  assert.equal(stops.hold(state()), true);
  stops.observe(state());
  assert.equal(stops.getSnapshot().window, null);
  assert.match(stops.stopReason(state()), /held/);
  assert.equal(stops.getSnapshot().stops['phase:Upkeep'], 'once');
  stops.observe(state({ turn_number: 3 }));
  assert.ok(stops.getSnapshot().window);
  stops.setPhaseStopsEnabled(false);
  assert.equal(stops.stopReason(state({ turn_number: 3 })), 'stop at Upkeep');
  assert.equal(stops.getSnapshot().window, null);
});

test('phase timers never skip own-turn stops, blockers choices, or stack responses', () => {
  for (const current of [state({ active_player: 0 }), onStack(), state({ decision: { kind: 'blockers', player: 0 } })]) {
    const stops = createPriorityStops({ phaseStopsEnabled: true, initialStops: { 'phase:Upkeep': 'always' } });
    assert.equal(stops.stopReason(current), 'stop at Upkeep');
    assert.equal(stops.getSnapshot().window, null);
  }
});

for (const kind of ['cast_spell', 'activate_ability']) {
  test(`${kind} does not create a timed hold window`, () => {
    const stops = createPriorityStops({ phaseStopsEnabled: true });
    stops.beforeCommand({ type: 'priority_action', action_ref: { kind } }, state());
    stops.observe(onStack());
    assert.equal(stops.getSnapshot().window, null);
  });
}

test('phase countdown survives analysis updates but cannot pass stale priority', () => {
  let clock = 0;
  const stops = createPriorityStops({ phaseStopsEnabled: true, now: () => clock, initialStops: { 'phase:Upkeep': 'once' } });
  stops.observe(state());
  const id = stops.getSnapshot().window.id;
  stops.armWindow(id);
  clock = 1000;
  stops.observe(state({ snapshot_id: 5, __priority_revision: 2 }));
  assert.equal(stops.getSnapshot().window.id, id);
  assert.equal(stops.getSnapshot().window.startedAt, 0);
  clock = 2000;
  assert.equal(stops.expire(id, state({ decision: { kind: 'priority', player: 1 } })), false);
  assert.equal(stops.getSnapshot().window, null);
});

for (const mode of ['off', 'safe']) {
  test(`${mode} ability countdown respects hover, Hold, and expiration`, () => {
    let clock = 0;
    const stops = createPriorityStops({ resolveMode: mode, now: () => clock });
    const current = state({ stack_size: 1, stack_objects: [{ id: 10, ability_kind: 'triggered' }],
      decision: { kind: 'priority', player: 0, analysis_complete: false, actions: [{ kind: 'cast_spell' }] } });
    stops.observe(current);
    const id = stops.getSnapshot().window.id;
    stops.armWindow(id);
    clock = 500;
    stops.pauseWindow(id);
    clock = 3000;
    assert.equal(stops.expire(id, current), false);
    stops.resumeWindow(id);
    clock = 4500;
    assert.equal(stops.expire(id, current), true);
    stops.observe({ ...current, turn_number: 3 });
    stops.hold({ ...current, turn_number: 3 });
    assert.equal(stops.getSnapshot().window, null);
  });
}

test('off counts down spells for either controller even with playable responses', () => {
  for (const controller of [0, 1]) {
    let clock = 0;
    const stops = createPriorityStops({ resolveMode: 'off', now: () => clock });
    const current = state({ stack_size: 1, stack_objects: [{ id: 10, controller }],
      decision: { kind: 'priority', player: 0, actions: [{ kind: 'cast_spell' }] } });
    stops.observe(current);
    const id = stops.getSnapshot().window.id;
    stops.armWindow(id);
    clock = 2000;
    assert.equal(stops.expire(id, current), true);
  }
});

test('white holds whenever any spell is on the stack, including below abilities', () => {
  const stops = createPriorityStops({ resolveMode: 'safe' });
  for (const entries of [[{ id: 1 }], [{ id: 2, ability_kind: 'activated' }, { id: 1 }]]) {
    stops.observe(state({ stack_size: entries.length, stack_objects: entries }));
    assert.equal(stops.getSnapshot().window, null);
  }
});

test('yellow removes every countdown; changing modes applies immediately', () => {
  const stops = createPriorityStops({ resolveMode: 'off', phaseStopsEnabled: true,
    initialStops: { 'phase:Upkeep': 'always' } });
  stops.observe(onStack());
  assert.ok(stops.getSnapshot().window);
  stops.setResolveMode('safe');
  stops.observe(onStack());
  assert.equal(stops.getSnapshot().window, null);
  stops.setResolveMode('all');
  for (const current of [onStack(), state(), state({ stack_size: 1,
    stack_objects: [{ id: 20, ability_kind: 'triggered' }] })]) {
    stops.observe(current);
    assert.equal(stops.getSnapshot().window, null);
  }
  stops.setResolveMode('off');
  stops.observe(onStack());
  assert.ok(stops.getSnapshot().window);
});

test('seat stops override defaults independently, persist off, and consume once on that seat only', async () => {
  const { DEFAULT_PRIORITY_STOPS, priorityStopMode, priorityStopsForPlayer, readStoredPriorityStops } = await import('../src/lib/priority-stops.js');
  let saved;
  const storage = { getItem: () => saved ?? null, setItem: (_, value) => { saved = value; } };
  const controller = createPriorityStops({ storage });
  assert.deepEqual(controller.getSnapshot().stops, DEFAULT_PRIORITY_STOPS);
  for (const player of [0, 1, 2, 3]) {
    for (const key of ['phase:Main', 'phase:Main2']) assert.equal(priorityStopMode(controller.getSnapshot().stops, key, player), 'always');
  }
  controller.cycleStop('phase:Main', state(), 1);
  let stops = controller.getSnapshot().stops;
  assert.equal(priorityStopMode(stops, 'phase:Main', 1), undefined);
  assert.equal(priorityStopMode(stops, 'phase:Main', 0), 'always');
  assert.equal(priorityStopMode(readStoredPriorityStops(storage), 'phase:Main', 1), undefined);
  controller.cycleStop('phase:Main', state(), 1);
  const main = state({ phase: 'Main', step: null });
  assert.match(controller.stopReason(main), /Main/);
  controller.beforeCommand(pass, { ...main, active_player: 0 });
  assert.equal(priorityStopMode(controller.getSnapshot().stops, 'phase:Main', 1), 'once');
  controller.beforeCommand(pass, main);
  stops = controller.getSnapshot().stops;
  assert.equal(priorityStopMode(stops, 'phase:Main', 1), undefined);
  assert.equal(priorityStopMode(stops, 'phase:Main', 0), 'always');
  assert.deepEqual(Object.keys(priorityStopsForPlayer(stops, 1)), ['phase:Main2']);
  assert.deepEqual(Object.keys(priorityStopsForPlayer(stops, 0)), ['phase:Main', 'phase:Main2']);
});
