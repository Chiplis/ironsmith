import test from 'node:test';
import assert from 'node:assert/strict';
import { sourceFaces, materializeCard } from './cohort-input.mjs';
test('frozen normal complete-source records shape independent fields without changing evidence', () => {
  const raw = { cardName: 'Source witness', fullName: 'Source witness', shape: 'normal', manaCost: '{0}', typeLine: 'Artifact Creature — Construct', power: '0', toughness: '3', loyalty: '', rulesText: 'This creature has flying.\nCards named “Harmonize” remain named “Harmonize”.' };
  const card = { name: raw.cardName, source_shape: 'normal', source_faces: [raw] };
  const before = structuredClone(card);
  const [face] = sourceFaces(card);
  assert.equal(face.original_text, `Mana cost: {0}\nType: Artifact Creature — Construct\nPower/Toughness: 0/3\n${raw.rulesText}`);
  assert.equal(face.rulesText, raw.rulesText);
  assert.deepEqual(card, before);
});
test('unshaped linked source faces cannot silently fall back to only one face', () => {
  const card = { name: 'Front', source_shape: 'doubleface', source_faces: [
    { cardName: 'Front', shape: 'doubleface', typeLine: 'Creature', rulesText: 'Flying' },
    { cardName: 'Back', shape: 'doubleface', typeLine: 'Creature', rulesText: 'Reach' },
  ] };
  assert.throws(() => materializeCard(card, []), /requires name and original_text/);
});
