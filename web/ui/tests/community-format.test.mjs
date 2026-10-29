import test from 'node:test';
import assert from 'node:assert/strict';
import { normalizeCommunityFormat, validateCommunityDeck, assertCommunityMatch, communityCustomCards } from '../src/lib/community-format.js';
import { withSupportedCards } from '../src/lib/unsupported-card-substitution.js';
const custom = { id: 'custom:one', draft: { faces: [{ name: 'Community Mage', oracleText: 'Draw a card.' }] } };
const format = normalizeCommunityFormat({ name: 'Our format', includedSets: [{ code: 'tst', name: 'Test set', cards: ['Plains', 'Lightning Bolt', 'Fire // Ice'] }], excludedCards: ['Lightning Bolt'], includedCards: ['Island'], customSets: [{ name: 'Our set', cards: [custom] }], customCards: [custom] });
test('sets, individual and custom cards combine; exclusions always win', () => {
  assert.equal(validateCommunityDeck(format, ['Plains', 'Island', 'Community Mage', 'Fire']).ready, true);
  assert.match(validateCommunityDeck(format, ['Lightning Bolt']).errors[0], /excluded/);
  assert.equal(validateCommunityDeck({ ...format, includedCards: ['Lightning Bolt'] }, ['Lightning Bolt']).ready, false);
  assert.equal(validateCommunityDeck({ ...format, excludedSets: [{ cards: ['Plains'] }] }, ['Plains']).ready, false);
  assert.equal(validateCommunityDeck(format, ['Swamp']).ready, false);
  assert.equal(validateCommunityDeck(format, ['Plains'], [], ['Lightning Bolt']).ready, false);
  assert.equal(validateCommunityDeck(format, ['Plains'], ['Swamp']).ready, false);
  assert.equal(communityCustomCards(format).length, 1);
});
test('snapshots are independent and conflicting custom definitions fail', () => {
  const snapshot = normalizeCommunityFormat(format);
  snapshot.customSets[0].cards[0].draft.faces[0].name = 'Edited';
  assert.equal(format.customSets[0].cards[0].draft.faces[0].name, 'Community Mage');
  assert.throws(() => normalizeCommunityFormat({ ...format, customCards: [{ draft: { faces: [{ name: 'Community Mage', oracleText: 'Destroy target creature.' }] } }] }), /Conflicting/);
});
test('match start enforces the pool and Trusted mode', () => {
  const payload = { communityFormat: format, format: 'normal', securityMode: 'trusted', decks: [['Plains']] };
  assert.doesNotThrow(() => assertCommunityMatch(payload));
  assert.throws(() => assertCommunityMatch({ ...payload, securityMode: 'verified' }), /Trusted/);
  assert.throws(() => assertCommunityMatch({ ...payload, decks: [['Lightning Bolt']] }), /excluded/);
});
test('custom cards never silently become basic lands', async () => {
  const parsed = { deck: ['Community Mage'], sideboard: [] };
  assert.equal(await withSupportedCards(parsed, { communityFormat: format, game: { filterKnownCardNames: () => { throw new Error('must not substitute'); } } }), parsed);
});
