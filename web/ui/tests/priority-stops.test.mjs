import test from 'node:test';
import assert from 'node:assert/strict';
import { CHECKING_PLAYABLE_ACTIONS, createPriorityStops } from '../src/lib/priority-stops.js';

// Conditional stops stay quiet unless a test says the seat can act.
const state = (changes = {}) => ({
  turn_number: 2, active_player: 1, perspective: 0, phase: 'beginning phase', step: 'upkeep',
  stack_size: 0, stack_objects: [], decision: { kind: 'priority', player: 0, actions: [], can_act: false }, ...changes,
});
const pass = { type: 'priority_action', action_ref: { kind: 'pass_priority' } };
const cast = { type: 'priority_action', action_ref: { kind: 'cast_spell' } };
const onStack = (id = 10) => state({ stack_size: 1, stack_objects: [{ id, controller: 0 }] });
test('tracker cycles off (default) / I can act / every time', () => {
  const stops = createPriorityStops({ phaseStopsEnabled: true });
  for (const expected of ['can_act', 'always', undefined]) {
    stops.cycleStop('phase:Upkeep', state());
    assert.equal(stops.getSnapshot().stops['phase:Upkeep'], expected);
  }
});

test('I can act stops hold only when the seat can act, and wait while that is unknown', () => {
  const stops = createPriorityStops({ phaseStopsEnabled: true, initialStops: { 'phase:Upkeep': 'can_act' } });
  const canAct = value => state({ decision: { kind: 'priority', player: 0, actions: [], can_act: value } });
  assert.equal(stops.stopReason(state({ decision: { kind: 'priority', player: 1 } })), null);
  assert.equal(stops.stopReason(canAct(false)), null);
  assert.equal(stops.stopReason(canAct(null)), CHECKING_PLAYABLE_ACTIONS);
  assert.equal(stops.awaitsCanAct(canAct(null)), true);
  assert.equal(stops.stopReason(canAct(null), false), null, 'an engine answer overrides the view');
  assert.match(stops.stopReason(canAct(true)), /Upkeep/);
  assert.equal(stops.stopReason(state({ decision: { kind: 'blockers', player: 0 } })), null,
    'required decisions are not priority stops');
  stops.beforeCommand(pass, canAct(true));
  assert.equal(stops.stopReason(canAct(true)), null, 'acting consumes the stop for this step');
  stops.cycleStop('phase:Upkeep', state());
  stops.cycleStop('phase:Upkeep', state());
  assert.equal(stops.stopReason(canAct(true)), null, 'off never stops');
  assert.equal(stops.awaitsCanAct(canAct(null)), false);
});

test('I can act counts down on an opponent turn only once the seat can act', () => {
  const stops = createPriorityStops({ phaseStopsEnabled: true, initialStops: { 'phase:Upkeep': 'can_act' } });
  for (const value of [false, null]) {
    stops.observe(state({ decision: { kind: 'priority', player: 0, actions: [], can_act: value } }));
    assert.equal(stops.getSnapshot().window, null);
  }
  stops.observe(state({ decision: { kind: 'priority', player: 0, actions: [], can_act: true } }));
  assert.equal(stops.getSnapshot().window?.kind, 'phase-stop');
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
  stops.cycleStop('phase:Draw', state());
  stops.beforeCommand(cast, state());
  stops.cancelAction();
  stops.observe(state());
  assert.equal(stops.getSnapshot().window, null);
  assert.match(stops.stopReason(state({ step: 'draw' })), /Draw/);
});

test('failed submissions restore a stop instead of silently consuming it', () => {
  const stops = createPriorityStops({ phaseStopsEnabled: true });
  stops.cycleStop('phase:Upkeep', state());
  stops.cycleStop('phase:Upkeep', state());
  const checkpoint = stops.beforeCommand(pass, state());
  assert.equal(stops.stopReason(state()), null);
  stops.rollbackAction(checkpoint, state());
  stops.resume(state());
  assert.match(stops.stopReason(state()), /Upkeep/);
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
  const stops = createPriorityStops({ phaseStopsEnabled: true, initialStops: { 'phase:Upkeep': 'always' } });
  stops.observe(state());
  assert.equal(stops.hold(state()), true);
  stops.observe(state());
  assert.equal(stops.getSnapshot().window, null);
  assert.match(stops.stopReason(state()), /held/);
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
  const stops = createPriorityStops({ phaseStopsEnabled: true, now: () => clock, initialStops: { 'phase:Upkeep': 'always' } });
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

test('seat stops override defaults independently and persist every mode', async () => {
  const { DEFAULT_PRIORITY_STOPS, priorityStopMode, priorityStopsForPlayer, readStoredPriorityStops, PRIORITY_STOP_KEYS } = await import('../src/lib/priority-stops.js');
  let saved;
  const storage = { getItem: () => saved ?? null, setItem: (_, value) => { saved = value; } };
  const controller = createPriorityStops({ storage });
  assert.deepEqual(controller.getSnapshot().stops, DEFAULT_PRIORITY_STOPS);
  const mainPhases = new Set(['phase:Main', 'phase:Main2']);
  for (const player of [0, 1, 2, 3]) {
    for (const key of PRIORITY_STOP_KEYS) assert.equal(priorityStopMode(controller.getSnapshot().stops, key, player),
      mainPhases.has(key) ? 'always' : undefined);
  }
  controller.cycleStop('phase:Main', state(), 1);
  let stops = controller.getSnapshot().stops;
  assert.equal(priorityStopMode(stops, 'phase:Main', 1), undefined);
  assert.equal(priorityStopMode(stops, 'phase:Main', 0), 'always');
  assert.equal(priorityStopMode(readStoredPriorityStops(storage), 'phase:Main', 1), undefined);
  controller.cycleStop('phase:End', state(), 0);
  controller.cycleStop('phase:End', state(), 0);
  stops = readStoredPriorityStops(storage);
  assert.equal(priorityStopMode(stops, 'phase:End', 0), 'always');
  assert.equal(priorityStopMode(stops, 'phase:End', 1), undefined);
  assert.equal(priorityStopsForPlayer(stops, 1)['phase:Main'], undefined);
  assert.equal(priorityStopsForPlayer(stops, 0)['phase:End'], 'always');
  controller.cycleStop('phase:Main', state(), 1);
  controller.cycleStop('phase:Main', state(), 1);
  assert.equal(controller.getSnapshot().stops['player:1:phase:Main'], undefined, 'back to the default');
});

test('version 2 saves keep deliberate every-time stops; the main phases keep their every-time default', async () => {
  const { priorityStopMode, readStoredPriorityStops } = await import('../src/lib/priority-stops.js');
  const storage = { getItem: () => JSON.stringify({ version: 2, stops: {
    'phase:Main': 'always', 'phase:Main2': 'always', 'step:DeclareBlockers': 'always', 'player:1:phase:End': 'off' } }) };
  const stops = readStoredPriorityStops(storage);
  assert.equal(priorityStopMode(stops, 'phase:Main', 0), 'always');
  assert.equal(priorityStopMode(stops, 'step:DeclareBlockers', 0), 'always');
  assert.equal(priorityStopMode(stops, 'phase:End', 1), undefined);
});
