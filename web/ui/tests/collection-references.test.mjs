import test from 'node:test';
import assert from 'node:assert/strict';
import { validateMechanic, mergeMechanics, compileDraftInput } from '../src/lib/custom-mechanics.js';
import { normalizeCollectionReferences, expandCollectionReferences } from '../src/lib/reminder-collection-references.js';
const reference = { participle: 'salvaged', label: 'salvaged' };
const declaration = { keyword: 'Salvage', reminder: 'Exile a card from your graveyard.', collectionReferences: [reference] };
test('explicit collection references preserve Mettalia source and canonicalize only declared relations', () => {
  const definition = validateMechanic(declaration);
  const original = "Mettalia has protection from each color among cards she's salvaged in exile.";
  assert.equal(expandCollectionReferences(original, [definition]), 'Mettalia has protection from each color among cards in exile from the salvaged collection.');
  assert.equal(original, "Mettalia has protection from each color among cards she's salvaged in exile.");
  for (const subject of ['it has', 'she has', 'he has', 'this creature has', "it's", 'she’s', "he's"]) {
    assert.equal(expandCollectionReferences(`a card ${subject} salvaged in exile`, [definition]), 'a card in exile from the salvaged collection');
  }
  assert.equal(expandCollectionReferences(original, [{ ...definition, collectionReferences: [] }]), original);
  assert.equal(expandCollectionReferences('cards she has exiled in exile', [definition]), 'cards she has exiled in exile');
  assert.equal(expandCollectionReferences('Create a token named "cards she has salvaged in exile".', [definition]), 'Create a token named "cards she has salvaged in exile".');
});
test('compiler input rewrites declared collection references without requiring a same-card action call', () => {
  const definition = validateMechanic(declaration);
  const original = "Type: Creature\nPower/Toughness: 4/3\nMettalia has protection from each color among cards she's salvaged in exile.";
  const draft = { customMechanics: [definition], faces: [{ name: 'Mettalia', oracleText: original }] };
  const result = compileDraftInput(draft, [definition]);
  assert.match(result.faces[0].oracleText, /cards in exile from the salvaged collection/);
  assert.equal(draft.faces[0].oracleText, original);
});
test('collection declarations are validated, retained across inline reminders, and conflict checked', () => {
  assert.deepEqual(validateMechanic(declaration).collectionReferences, [reference]);
  assert.deepEqual(mergeMechanics([declaration], [{ keyword: 'Salvage', reminder: declaration.reminder }])[0].collectionReferences, [reference]);
  assert.throws(() => mergeMechanics([declaration], [{ ...declaration, collectionReferences: [{ participle: 'salvaged', label: 'other' }] }]));
  assert.throws(() => normalizeCollectionReferences([{ participle: 'salvaged', label: '' }]));
  assert.throws(() => normalizeCollectionReferences([reference, reference]));
  assert.throws(() => validateMechanic({ ...declaration, keyword: 'Flying' }));
  assert.throws(() => expandCollectionReferences('cards it has salvaged in exile', [{ collectionReferences: [reference] }, { collectionReferences: [{ participle: 'salvaged', label: 'other' }] }]));
});
