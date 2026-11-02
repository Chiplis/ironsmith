import test from 'node:test';
import assert from 'node:assert/strict';
import {validateMechanic} from '../src/lib/custom-mechanics.js';
test('a source namespace never permits an official keyword collision', () => {
  assert.throws(() => validateMechanic({keyword:'Harness', sourceScope:'revolution', reminder:'Draw a card.'}), /official keywords/);
  assert.equal(validateMechanic({keyword:'Gather strength', sourceScope:'revolution', reminder:'Draw a card.'}).sourceScope, 'revolution');
});
