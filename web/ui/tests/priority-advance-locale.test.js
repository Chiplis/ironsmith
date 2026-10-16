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

test('Go To follows selected phase and combat stops; running transitions say Skipping', async () => {
  const { nextSelectedPriorityStop, selectedPriorityAdvanceLabel } = await import('../src/lib/priority-stops.js');
  const state = { phase: 'FirstMain', step: null };
  const stops = { 'phase:Main': 'always', 'phase:Main2': 'once' };
  assert.deepEqual(nextSelectedPriorityStop(state, stops), ['Main2']);
  assert.equal(selectedPriorityAdvanceLabel(state, stops, false, translator('en')), 'Go to Main II');
  assert.equal(selectedPriorityAdvanceLabel(state, {}, false, translator('en')), 'Go to Next Turn');
  assert.equal(selectedPriorityAdvanceLabel({phase:'Combat',step:'BeginCombat'}, stops, true, translator('en')), 'Skipping Begin Combat');
  assert.equal(selectedPriorityAdvanceLabel(state, stops, false, translator('es')), 'Ir a Principal II');
  assert.deepEqual(nextSelectedPriorityStop(state, {'step:DeclareAttackers':'always'}), ['Combat','DeclareAttackers']);
  assert.equal(selectedPriorityAdvanceLabel(state, {'step:DeclareAttackers':'always'}, false, translator('en')), 'Declare Attackers');
  assert.equal(selectedPriorityAdvanceLabel({phase:'Combat',step:'DeclareAttackers'}, {'step:DeclareBlockers':'once'}, false, translator('en')), 'Declare Blockers');
  assert.deepEqual(nextSelectedPriorityStop({phase:'Combat',step:'DeclareAttackers'}, {'step:DeclareAttackers':'always','step:DeclareBlockers':'once'}), ['Combat','DeclareBlockers']);
});
