import test from 'node:test';
import assert from 'node:assert/strict';
import { REMINDER_TEMPLATES } from '../src/lib/mechanics-catalog.js';
import { validateMechanic, compileDraftInput } from '../src/lib/custom-mechanics.js';

test('combat casting template binds the price under arbitrary mechanic names', () => {
  const template = REMINDER_TEMPLATES.find(t => t.kind === 'Combat casting');
  for (const keyword of ['Ambush', 'Night arrival']) {
    const definition = validateMechanic({ keyword: `${keyword} {Cost}`, reminder: template.reminder });
    const result = compileDraftInput({ faces: [{ name: 'Entrant', oracleText: `${keyword} {3}{R}` }] }, [definition]);
    assert.equal(result.faces[0].oracleText, template.reminder.replace('{Cost}', '{3}{R}'));
  }
});
