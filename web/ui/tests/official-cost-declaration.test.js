import test from 'node:test';
import assert from 'node:assert/strict';
import {validateMechanic} from '../src/lib/custom-mechanics.js';
test('paid declarations cannot bypass official keyword reservation', () => {
  assert.throws(() => validateMechanic({keyword:'Ascend', template:'{Cost}', officialCostDeclaration:true, reminder:'{Cost}: Draw a card.'}), /official keywords/);
});
