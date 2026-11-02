import test from 'node:test';
import assert from 'node:assert/strict';
import { mergeSubtypes, referencedSubtypes, storeCustomSubtypes, listCustomSubtypes } from '../src/lib/custom-subtypes.js';
import { sourceSubtypeDefinitions } from '../../../scripts/custom-mechanics/source-subtypes.mjs';
const spell = { singular: 'inscription', plural: 'inscriptions', family: 'Spell' };
const artifact = { ...spell, family: 'Artifact' };
test('multiple explicit families survive registry storage and referenced-card transport', () => {
  const expected = [artifact, spell];
  assert.deepEqual(mergeSubtypes([spell, artifact, spell]), expected);
  for (const face of [
    { typeLine: 'Instant — Inscription', oracleText: '' },
    { typeLine: 'Artifact — Equipment Inscription', oracleText: '' },
    { typeLine: 'Enchantment', oracleText: 'Exile an Inscription card from your graveyard.' },
  ]) assert.deepEqual(referencedSubtypes({ faces: [face] }, [spell, artifact]), expected);
  const values = new Map();
  const storage = { getItem: key => values.get(key), setItem: (key, value) => values.set(key, value) };
  assert.deepEqual(storeCustomSubtypes([spell, artifact], storage), expected);
  assert.deepEqual(listCustomSubtypes(storage), expected);
  assert.throws(() => mergeSubtypes([spell, { ...artifact, plural: 'inscripted' }]), /Conflicting/);
  assert.throws(() => mergeSubtypes([spell, { singular: 'different', plural: 'inscriptions', family: 'Artifact' }]), /Ambiguous/);
});
test('source metadata transports all declared associations rather than choosing by current card type', () => {
  for (const typeLine of ['Artifact — Equipment Inscription', 'Instant — Inscription']) {
    const card = { source_card_key: 'Source witness', source_faces: [{ name: 'Source witness', original_text: `Type: ${typeLine}` }] };
    const before = structuredClone(card);
    assert.deepEqual(sourceSubtypeDefinitions(card, { faces: [{ typeLine, oracleText: '' }] }, 'msem'), [artifact, spell]);
    assert.deepEqual(card, before);
  }
  const counterDraft = { faces: [{ typeLine: 'Artifact', oracleText: '' }], customKeywordCounterDefinitions: [{ name: 'archive', reminder: 'This permanent is an Inscription in addition to its other types.' }] };
  assert.deepEqual(sourceSubtypeDefinitions({}, counterDraft, 'msem'), [artifact, spell], 'counter-only subtype dependencies retain all associations');
});
