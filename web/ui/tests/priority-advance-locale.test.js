import test from 'node:test';
import assert from 'node:assert/strict';
import { PRIORITY_STOP_KEYS, selectedPriorityAdvanceLabel } from '../src/lib/priority-stops.js';
import { priorityAdvanceButtonLabel } from '../src/lib/constants.js';
import { localeCatalogs, interpolate } from '../src/i18n/catalog.js';
const allOff = Object.fromEntries(PRIORITY_STOP_KEYS.map(key => [key, 'off']));
const translator = locale => (key, params) => interpolate(localeCatalogs[locale].messages[key], params);
test('main decision advance labels translate every destination and the prefix', () => {
  const cases = [
    ['Beginning','Untap','Ir a Mantenimiento'],
    ['Beginning','Upkeep','Ir a Robar'],
    ['Beginning','Draw','Ir a Principal I'],
    ['FirstMain',null,'Ir a Combate'],
    ['Combat','BeginCombat','Declarar atacantes'],
    ['Combat','DeclareAttackers','Declarar bloqueadores'],
    ['Combat','DeclareBlockers','Ir a Daño'],
    ['Combat','CombatDamage','Ir a Fin del combate'],
    ['Combat','EndCombat','Ir a Principal II'],
    ['NextMain',null,'Ir a Paso final'],
    ['Ending','End','Ir a Limpieza'],
    ['Ending','Cleanup','Ir a Siguiente turno'],
  ];
  for (const [phase, step, expected] of cases) {
    assert.equal(priorityAdvanceButtonLabel(phase, step, 0, translator('es')), expected, `${phase}/${step}`);
  }
  assert.equal(priorityAdvanceButtonLabel('FirstMain', null, 1, translator('es')), 'Resolver');
  assert.equal(priorityAdvanceButtonLabel('FirstMain', null, 0, translator('en')), 'Go to Combat');
});

test('Go To follows every-time phase and combat stops; running transitions say Skipping', async () => {
  const { nextSelectedPriorityStop, selectedPriorityAdvanceLabel } = await import('../src/lib/priority-stops.js');
  const state = { phase: 'FirstMain', step: null };
  // With conditional stops disabled, the destination is predictable.
  const stops = { ...allOff, 'phase:Main': 'always', 'phase:Main2': 'always', 'phase:End': 'can_act' };
  assert.deepEqual(nextSelectedPriorityStop(state, stops), ['Main2']);
  assert.equal(selectedPriorityAdvanceLabel(state, stops, false, translator('en')), 'Go to Main II');
  assert.equal(selectedPriorityAdvanceLabel(state, allOff, false, translator('en')), 'Go to Next Turn');
  assert.equal(selectedPriorityAdvanceLabel({phase:'Combat',step:'BeginCombat'}, stops, true, translator('en')), 'Skipping Begin Combat');
  assert.equal(selectedPriorityAdvanceLabel(state, stops, false, translator('es')), 'Ir a Principal II');
  assert.deepEqual(nextSelectedPriorityStop(state, {'step:DeclareAttackers':'always'}), ['Combat','DeclareAttackers']);
  assert.equal(selectedPriorityAdvanceLabel(state, {...allOff, 'step:DeclareAttackers':'always'}, false, translator('en')), 'Declare Attackers');
  assert.equal(selectedPriorityAdvanceLabel({phase:'Combat',step:'DeclareAttackers'}, {'step:DeclareBlockers':'always'}, false, translator('en')), 'Declare Blockers');
  assert.deepEqual(nextSelectedPriorityStop({phase:'Combat',step:'DeclareAttackers'}, {'step:DeclareAttackers':'always','step:DeclareBlockers':'always'}), ['Combat','DeclareBlockers']);
});

test('Go To names your own attack declaration when you control a creature that can attack', async () => {
  const { selectedPriorityAdvanceLabel } = await import('../src/lib/priority-stops.js');
  const board = (cards, active = 0) => ({ phase: 'FirstMain', step: null, active_player: active, perspective: 0,
    players: [{ id: 0, battlefield: cards }, { id: 1, battlefield: [] }] });
  const creature = { card_types: ['creature'], tapped: false, summoning_sick: false };
  const stops = { ...allOff, 'phase:Main2': 'always' };
  assert.equal(selectedPriorityAdvanceLabel(board([creature]), stops, false, translator('en')), 'Declare Attackers');
  for (const cards of [[], [{ ...creature, summoning_sick: true }], [{ ...creature, tapped: true }], [{ ...creature, card_types: ['land'] }]]) {
    assert.equal(selectedPriorityAdvanceLabel(board(cards), stops, false, translator('en')), 'Go to Main II');
  }
  assert.equal(selectedPriorityAdvanceLabel(board([creature], 1), stops, false, translator('en')), 'Go to Main II',
    "an opponent's creatures never stop your advance");
});


test('conditional stops do not promise an uninterrupted advance at any phase or step', () => {
  const cases = [
    ['Beginning', 'Untap', 'Untap'],
    ['Beginning', 'Upkeep', 'Upkeep'],
    ['Beginning', 'Draw', 'Draw'],
    ['FirstMain', null, 'Main I'],
    ['Combat', 'BeginCombat', 'Begin Combat'],
    ['Combat', 'DeclareAttackers', 'Declare Attackers'],
    ['Combat', 'DeclareBlockers', 'Declare Blockers'],
    ['Combat', 'FirstStrikeDamage', 'First Strike Damage'],
    ['Combat', 'CombatDamage', 'Combat Damage'],
    ['Combat', 'EndCombat', 'End Combat'],
    ['NextMain', null, 'Main II'],
    ['Ending', 'End', 'End'],
  ];
  // End is conditional and Cleanup is guaranteed; every earlier step has a
  // possible interruption, even with both default main-phase stops disabled.
  const stops = { ...Object.fromEntries(PRIORITY_STOP_KEYS.map(key => [key, 'can_act'])),
    'phase:Main': 'off', 'phase:Main2': 'off', 'phase:Cleanup': 'always' };
  for (const [phase, step, label] of cases) {
    const state = { phase, step };
    const expected = step === 'End' ? 'Go to Cleanup' : `Continue from ${label}`;
    assert.equal(selectedPriorityAdvanceLabel(state, stops, false, translator('en')), expected);
    assert.equal(selectedPriorityAdvanceLabel(state, allOff, false, translator('en')), 'Go to Next Turn');
  }
  assert.equal(selectedPriorityAdvanceLabel({ phase: 'Ending', step: 'Cleanup' }, stops, false, translator('en')), 'Go to Next Turn');
  assert.equal(selectedPriorityAdvanceLabel({ phase: 'Combat', step: 'CombatDamage', combat_damage_step: 'first_strike' }, stops, false, translator('en')), 'Continue from First Strike Damage');
});

test('combat advance changes labels across the precombat priority window and respects seat overrides', () => {
  const main = { phase: 'FirstMain', active_player: 0, perspective: 0,
    players: [{ id: 0, battlefield: [{ card_types: ['creature'] }] }] };
  const combat = { ...main, phase: 'Combat', step: 'BeginCombat' };
  assert.equal(selectedPriorityAdvanceLabel(main, {}, false, translator('en')), 'Declare Attackers');
  const conditional = { 'phase:Combat': 'can_act' };
  assert.equal(selectedPriorityAdvanceLabel(main, conditional, false, translator('en')), 'Continue from Main I');
  assert.equal(selectedPriorityAdvanceLabel(main, conditional, false, translator('es')), 'Continuar desde Principal I');
  assert.equal(selectedPriorityAdvanceLabel(combat, {}, false, translator('en')), 'Declare Attackers');
  const stops = { 'player:0:phase:Combat': 'off', 'player:0:step:BeginCombat': 'off' };
  assert.equal(selectedPriorityAdvanceLabel(main, stops, false, translator('en')), 'Declare Attackers');
  assert.equal(selectedPriorityAdvanceLabel(main, { ...stops, 'player:0:step:BeginCombat': 'always' }, false, translator('en')), 'Go to Begin Combat');
  assert.equal(selectedPriorityAdvanceLabel(main, { ...stops, 'player:0:step:BeginCombat': 'can_act' }, false, translator('en')), 'Continue from Main I');
});
