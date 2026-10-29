import test from 'node:test';
import assert from 'node:assert/strict';
import { priorityAdvanceButtonLabel } from '../src/lib/constants.js';
import { localeCatalogs, interpolate } from '../src/i18n/catalog.js';
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
  // "I can act" (the default) depends on the board, so it never names a destination.
  const stops = { 'phase:Main': 'always', 'phase:Main2': 'always', 'phase:End': 'can_act' };
  assert.deepEqual(nextSelectedPriorityStop(state, stops), ['Main2']);
  assert.equal(selectedPriorityAdvanceLabel(state, stops, false, translator('en')), 'Go to Main II');
  assert.equal(selectedPriorityAdvanceLabel(state, {}, false, translator('en')), 'Go to Next Turn');
  assert.equal(selectedPriorityAdvanceLabel({phase:'Combat',step:'BeginCombat'}, stops, true, translator('en')), 'Skipping Begin Combat');
  assert.equal(selectedPriorityAdvanceLabel(state, stops, false, translator('es')), 'Ir a Principal II');
  assert.deepEqual(nextSelectedPriorityStop(state, {'step:DeclareAttackers':'always'}), ['Combat','DeclareAttackers']);
  assert.equal(selectedPriorityAdvanceLabel(state, {'step:DeclareAttackers':'always'}, false, translator('en')), 'Declare Attackers');
  assert.equal(selectedPriorityAdvanceLabel({phase:'Combat',step:'DeclareAttackers'}, {'step:DeclareBlockers':'always'}, false, translator('en')), 'Declare Blockers');
  assert.deepEqual(nextSelectedPriorityStop({phase:'Combat',step:'DeclareAttackers'}, {'step:DeclareAttackers':'always','step:DeclareBlockers':'always'}), ['Combat','DeclareBlockers']);
});

test('Go To names your own attack declaration when you control a creature that can attack', async () => {
  const { selectedPriorityAdvanceLabel } = await import('../src/lib/priority-stops.js');
  const board = (cards, active = 0) => ({ phase: 'FirstMain', step: null, active_player: active, perspective: 0,
    players: [{ id: 0, battlefield: cards }, { id: 1, battlefield: [] }] });
  const creature = { card_types: ['creature'], tapped: false, summoning_sick: false };
  const stops = { 'phase:Main2': 'always' };
  assert.equal(selectedPriorityAdvanceLabel(board([creature]), stops, false, translator('en')), 'Declare Attackers');
  for (const cards of [[], [{ ...creature, summoning_sick: true }], [{ ...creature, tapped: true }], [{ ...creature, card_types: ['land'] }]]) {
    assert.equal(selectedPriorityAdvanceLabel(board(cards), stops, false, translator('en')), 'Go to Main II');
  }
  assert.equal(selectedPriorityAdvanceLabel(board([creature], 1), stops, false, translator('en')), 'Go to Main II',
    "an opponent's creatures never stop your advance");
});
