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
    ['Combat','BeginCombat','Ir a Atacantes'],
    ['Combat','DeclareAttackers','Ir a Bloqueadores'],
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
