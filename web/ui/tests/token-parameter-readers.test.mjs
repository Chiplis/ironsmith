import test from 'node:test';
import assert from 'node:assert/strict';
import { compileDraftInput, validateMechanic } from '../src/lib/custom-mechanics.js';

const arm = validateMechanic({ keyword: 'Outfit {Object}', reminder: 'Create a colorless Equipment artifact token with "Equipped creature gets +1/+0." and equip {2} attached to {Object}.' });
const expand = text => compileDraftInput({ faces: [{ name: 'Source', oracleText: text }] }, [arm]).faces[0].oracleText;

test('plural target action distributes complete creation to each announced target', () => {
  assert.equal(expand('Outfit up to two target creatures you control.'), 'For each of up to two target creatures you control, Create a colorless Equipment artifact token with "Equipped creature gets +1/+0." and equip {2} attached to that creature.');
  assert.equal(expand('Outfit three target creatures.'), 'For each of three target creatures, Create a colorless Equipment artifact token with "Equipped creature gets +1/+0." and equip {2} attached to that creature.');
});

test('plural selector never consumes a control-flow tail or quoted card name', () => {
  for (const text of ['Outfit up to two target creatures unless you pay {1}.', 'Search your library for a card named "Outfit three target creatures.".']) {
    assert.equal(expand(text), text);
  }
});
