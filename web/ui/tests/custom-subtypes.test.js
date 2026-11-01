import test from 'node:test';
import assert from 'node:assert/strict';
import { mergeSubtypes, referencedSubtypes, suggestSubtypePlural, storeCustomSubtypes, listCustomSubtypes } from '../src/lib/custom-subtypes.js';
import { normalizeDraftForApi } from '../src/lib/custom-card-draft.js';
import { prepareCustomCard } from '../src/lib/custom-mechanics.js';
const moon = { singular: 'moonkin', plural: 'moonkin', family: 'Creature' };
const sky = { singular: 'skychild', plural: 'skychildren', family: 'Creature' };
const land = { singular: 'cloudbank', plural: 'cloudbanks', family: 'Land' };
test('singular, irregular and invariant plural references carry one definition', () => {
  for (const oracleText of ['Destroy target Skychild.', 'Skychildren you control get +1/+1.', 'Destroy target nonSkychild creature.', 'NonSkychildren you control get +1/+1.']) {
    assert.deepEqual(referencedSubtypes({ faces: [{ oracleText }] }, [moon, sky, land]), [sky]);
  }
  assert.deepEqual(referencedSubtypes({ faces: [{ subtypes: ['Moonkin'], oracleText: 'Moonkin you control have flying.' }] }, [moon, sky]), [moon]);
  assert.deepEqual(referencedSubtypes({ faces: [{ name: 'Skychildren', flavorText: 'Moonkin', oracleText: 'Flying (Skychild)' }] }, [moon, sky]), []);
});
test('official-only cards do not acquire the builder library', async () => {
  const prepared = await prepareCustomCard({ faces: [{ name: 'Bear', cardTypes: ['Creature'], subtypes: ['Bear'], oracleText: '' }] }, async input => {
    assert.deepEqual(input.customSubtypes, []); return { canCreate: true };
  }, { knownSubtypes: [moon, sky, land] });
  assert.deepEqual(prepared.draft.customSubtypes, []);
});
test('definitions survive saving, normalization and compilation without the library', async () => {
  const draft = { faces: [{ name: 'Scout', subtypes: ['Skychild'], oracleText: 'Skychildren you control have flying.' }] };
  const saved = await prepareCustomCard(draft, async () => ({ canCreate: true }), { knownSubtypes: [sky, moon] });
  const input = normalizeDraftForApi(JSON.parse(JSON.stringify(saved.draft)));
  assert.deepEqual(input.customSubtypes, [sky]);
  await prepareCustomCard(input, async input => { assert.deepEqual(input.customSubtypes, [sky]); return { canCreate: true }; }, { knownSubtypes: [] });
});
test('referenced tokens carry subtype dependencies', () => {
  assert.deepEqual(referencedSubtypes({ faces: [{ oracleText: 'Create a Scout token.' }], customTokens: [{ face: { subtypes: ['Skychild'] }, input: { subtypes: ['Skychild'] }, customSubtypes: [sky] }] }, []), [sky]);
});
test('aliases and definitions cannot conflict; plural suggestions remain overridable', () => {
  assert.equal(suggestSubtypePlural('Skychild'), 'Skychilds');
  assert.equal(suggestSubtypePlural('Faery'), 'Faeries');
  assert.equal(suggestSubtypePlural('Emberfox'), 'Emberfoxes');
  assert.deepEqual(mergeSubtypes([sky], [sky]), [sky]);
  assert.throws(() => mergeSubtypes([sky, { ...moon, plural: 'skychildren' }]), /Ambiguous/);
  assert.throws(() => mergeSubtypes([sky, { ...sky, family: 'Land' }]), /Conflicting/);
  assert.throws(() => mergeSubtypes([{ ...sky, singular: 'bad name' }]), /letters/);
});
test('library changes persist and do not mutate definitions already carried by a card', () => {
  const values = new Map(); const storage = { getItem: k => values.get(k), setItem: (k,v) => values.set(k,v) };
  storeCustomSubtypes([sky], storage);
  const carried = referencedSubtypes({ faces: [{ subtypes: ['Skychild'] }] }, listCustomSubtypes(storage));
  storeCustomSubtypes([], storage);
  assert.deepEqual(listCustomSubtypes(storage), []);
  assert.deepEqual(carried, [sky]);
});

test('mechanic-expanded subtype dependencies survive export and recompilation', async () => {
  const draft = { customSubtypes: [sky], customMechanics: [{ keyword: 'summonkin', reminder: 'Create a 1/1 white Skychild creature token.' }],
    faces: [{ name: 'Caller', oracleText: 'Summonkin.' }] };
  const prepared = await prepareCustomCard(draft, async input => {
    assert.deepEqual(input.customSubtypes, [sky]);
    assert.match(input.faces[0].oracleText, /Skychild/);
    return { canCreate: true };
  }, { knownSubtypes: [] });
  assert.deepEqual(prepared.draft.customSubtypes, [sky]);
});
