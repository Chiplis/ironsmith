import test from 'node:test';
import assert from 'node:assert/strict';
import { encodeCommunityShare, decodeCommunityShare, COMMUNITY_FILE_MAX_BYTES } from '../src/lib/community-sharing.js';
const mechanics = [{ keyword: 'stargaze', reminder: 'Draw a card.', expansion: 'Draw a card.', sourceCard: 'Star Grove', version: 1 }];
const card = { id: 'grove', draft: { layout: 'single', faces: [{ name: 'Star Grove', oracleText: '{T}: Stargaze.', cardTypes: ['Land'] }], customMechanics: mechanics } };
const set = { name: 'Sky set', cards: [card] };
const format = { name: 'Sky night', includedSets: [{ name: 'Dominaria', code: 'dom', cards: ['Island', 'Opt'] }], includedCards: ['Plains'], excludedCards: ['Opt'], customSets: [set], customCards: [card] };
test('set and format exports retain full mechanic definitions and card drafts', () => {
  const roundTripSet = decodeCommunityShare(encodeCommunityShare('set', set));
  assert.deepEqual(roundTripSet, { kind: 'set', data: set });
  const roundTrip = decodeCommunityShare(encodeCommunityShare('format', format)).data;
  assert.deepEqual(roundTrip.customSets[0].cards[0].draft, card.draft);
  assert.deepEqual(roundTrip.customCards[0].draft.customMechanics, mechanics);
  assert.deepEqual(roundTrip.excludedCards, ['Opt']);
  assert.deepEqual(roundTrip.includedSets, format.includedSets);
});
test('reject malformed, incompatible and oversized imports before storage', () => {
  const file = data => JSON.stringify({ schema: 'ironsmith-community', version: 1, kind: 'set', data });
  for (const text of ['oops', '{}', file({ name: 'Bad', cards: [null] }), file({ name: 'Bad', cards: [{ draft: { faces: [null] } }] }),
    file({ name: 'Bad', cards: [{ draft: { faces: [{ name: 'Bad' }], customMechanics: [{}] } }] }),
    JSON.stringify({ schema: 'ironsmith-community', version: 2, kind: 'set', data: set })]) assert.throws(() => decodeCommunityShare(text));
  assert.throws(() => decodeCommunityShare(' '.repeat(COMMUNITY_FILE_MAX_BYTES + 1)), /4 MB/);
});

test('parameter templates and custom word forms survive sharing unchanged', () => {
  const draft = structuredClone(card);
  draft.draft.customMechanics = [{ keyword: 'starstudy', template: '{Number}', reminder: 'Draw {Number} cards.', sourceCard: 'Star Grove', version: 1,
    forms: { participle: 'starstudied' } }];
  const data = { name: 'Parameter set', cards: [draft] };
  assert.deepEqual(decodeCommunityShare(encodeCommunityShare('set', data)).data, data);
});
