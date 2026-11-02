import test from 'node:test';
import assert from 'node:assert/strict';
import { prepareSetCard, updateSetMechanic } from '../src/lib/set-workspace.js';
import { validateMechanic } from '../src/lib/custom-mechanics.js';
import { encodeCommunityShare, decodeCommunityShare } from '../src/lib/community-sharing.js';
const mechanic = reminder => validateMechanic({ keyword: 'Kindle', reminder });
const draft = { layout: 'single', faces: [{ name: 'Ember', cardTypes: ['Sorcery'], oracleText: 'Kindle' }], customMechanics: [mechanic('Draw a card.')] };

test('set definitions override carried copies and a failed edit leaves the entire set untouched', async () => {
  const original = { name: 'Embers', customCards: [{ id: 'ember', draft }], customMechanics: [mechanic('Draw a card.')] };
  const snapshot = structuredClone(original);
  let compiled;
  await prepareSetCard(draft, [mechanic('Draw two cards.')], async input => { compiled = input; return { canCreate: true }; });
  assert.match(compiled.faces[0].oracleText, /Draw two cards/i);
  await assert.rejects(updateSetMechanic(original, mechanic('Draw two cards.'), async () => { throw new Error('Unsupported effect'); }), /Ember: Unsupported effect/);
  assert.deepEqual(original, snapshot);
  const updated = await updateSetMechanic(original, mechanic('Draw two cards.'), async () => ({ canCreate: true }));
  assert.equal(updated.customCards[0].draft.customMechanics[0].reminder, 'Draw two cards.');
  assert.deepEqual(original, snapshot);
});

test('unused set mechanics survive set and format exports', () => {
  const set = { name: 'Embers', cards: [], customMechanics: [mechanic('Draw a card.')] };
  assert.deepEqual(decodeCommunityShare(encodeCommunityShare('set', set)).data, set);
  const format = decodeCommunityShare(encodeCommunityShare('format', { name: 'Night', customSets: [set] })).data;
  assert.deepEqual(format.customSets[0], set);
});


test('set subtypes travel through compilation and exports without using personal vocabulary', async () => {
  const subtype = { singular: 'emberkin', plural: 'emberkin', family: 'Creature' };
  const card = { layout: 'single', faces: [{ name: 'Kindred', cardTypes: ['Creature'], subtypes: ['Emberkin'], oracleText: '', power: '1', toughness: '1' }] };
  const prepared = await prepareSetCard(card, [], async input => {
    assert.deepEqual(input.customSubtypes, [subtype]);
    return { canCreate: true };
  }, [subtype]);
  assert.deepEqual(prepared.draft.customSubtypes, [subtype]);
  const set = { name: 'Embers', cards: [], customSubtypes: [subtype] };
  assert.deepEqual(decodeCommunityShare(encodeCommunityShare('set', set)).data, set);
  assert.deepEqual(decodeCommunityShare(encodeCommunityShare('format', { name: 'Night', customSets: [set] })).data.customSets[0], set);
});
