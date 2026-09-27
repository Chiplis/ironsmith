import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const source = readFileSync(new URL('../src/hooks/peer-lobby/crypto-resync.js', import.meta.url), 'utf8');
function extract(startText, endText) {
  const start = source.indexOf(startText), end = source.indexOf(endText, start);
  assert.ok(start >= 0 && end > start);
  return source.slice(start, end);
}

test('shuffle randomness is queued before final private openings without collecting preliminary cards', async () => {
  const calls = [];
  const context = {
    gameRef: { current: { injectTranscriptRandomSeeds: async material => calls.push(['randomness', material]) } },
    normalizeShuffleOrder: value => value || [],
    shuffleProofMatchesRequirement: (proof, requirement) => proof.requirementId === requirement.id,
    privateOpeningsForLocalViewer: async requirements => {
      const cards = requirements.filter(requirement => requirement.type === 'private_open');
      calls.push(['private', cards]); return cards;
    },
    revealPrivateOpeningsForInjection: async openings => calls.push(['reveal', openings]),
  };
  const inject = new Function(...Object.keys(context), `${extract(
    '  async function injectCryptoMaterialForRequirements(',
    '\n\t  const buildLocalPrivateViewProofsForRequirements'
  )}\nreturn injectCryptoMaterialForRequirements;`)(...Object.values(context));
  const shuffle = { id: 'mulligan', type: 'verifiable_shuffle', owner: 0 };
  const stale = { type: 'private_open', commitment: 'old-preview-card' };
  const final = { type: 'private_open', commitment: 'final-drawn-card' };
  const audit = { shuffleProofs: [{ requirementId: 'mulligan', owner: 0, deckHash: 'verified-seed',
    beforeOrder: [1, 2], afterOrder: [2, 1] }] };
  await inject([shuffle, stale], audit, { randomnessOnly: true });
  assert.deepEqual(calls, [['randomness', { seeds: ['verified-seed'], libraryShuffles: [{ owner: 0,
    beforeOrder: [1, 2], afterOrder: [2, 1] }] }]]);
  await inject([shuffle, final], audit, { skipRandomness: true });
  assert.deepEqual(calls.slice(1), [['private', [final]], ['reveal', [final]]]);
});

test('finalized action requirements replace preliminary identities and retain other action history', () => {
  const history = new Map([[7, [{ id: 'earlier', commitment: 'earlier-card' }]]]);
  const context = { actionCryptoRequirementsRef: { current: history },
    canonicalMultiplayerPayload: JSON.stringify, cloneMultiplayerPayload: structuredClone };
  const remember = new Function(...Object.keys(context), `${extract(
    '  function rememberActionCryptoRequirements(', '\n  function cryptoRequirementReplayKey('
  )}\nreturn rememberActionCryptoRequirements;`)(...Object.values(context));
  remember(8, [{ id: 'shuffle', order: 'preliminary' }, { id: 'old-opening', commitment: 'wrong-card' }]);
  const finalized = [{ id: 'shuffle', order: 'final' }, { id: 'new-opening', commitment: 'drawn-card' }];
  remember(8, finalized, { replace: true });
  assert.deepEqual(history.get(8), finalized);
  assert.deepEqual(history.get(7), [{ id: 'earlier', commitment: 'earlier-card' }]);
  remember(8, [{ id: 'post-opening', commitment: 'post-card' }]);
  assert.deepEqual(history.get(8), [...finalized, { id: 'post-opening', commitment: 'post-card' }]);
});

function freshRequirementsHarness(history) {
  const context = {
    actionCryptoRequirementsRef: { current: history },
    canonicalMultiplayerPayload: JSON.stringify,
    normalizeShuffleOrder: value => value || [],
  };
  return new Function(...Object.keys(context), `${extract(
    '  function cryptoRequirementReplayKey(', '\n  function shuffleProofReplayKey('
  )}\nreturn freshCryptoRequirementsForSequence;`)(...Object.values(context));
}

test('a second library search reopens cards resealed by an earlier fetch shuffle', () => {
  const firstSearch = {
    id: 'private_open:1:library:8:69', type: 'private_open', owner: 1, viewer: 1,
    zone: 'library', slot: 8, objectId: 69, commitment: 'salted-deck-slot-commitment',
    publicSlot: 17, publicCommitment: 'ziffle:first-search:17',
    visibility: 'viewer', reason: 'Search library',
  };
  const secondSearch = {
    ...firstSearch, publicSlot: 3, publicCommitment: 'ziffle:after-fetch-shuffle:3',
  };
  const history = new Map([[50, [firstSearch]]]);
  const fresh = freshRequirementsHarness(history);
  assert.deepEqual(fresh(51, [{ ...firstSearch }]), [], 'the same ciphertext is already opened');
  assert.deepEqual(fresh(99, [secondSearch]), [secondSearch],
    'resealing preserves the deck slot but requires a new private opening');
  history.set(99, [secondSearch]);
  assert.deepEqual(fresh(100, [{ ...secondSearch }]), [], 'the new ciphertext is deduplicated after opening');
});

test('private opening replay keys normalize public and origin identity aliases', () => {
  const opening = {
    id: 'private_open:1:library:8:69', type: 'private_open', owner: 1, viewer: 1,
    zone: 'library', slot: 8, objectId: 69, commitment: 'salted-deck-slot-commitment',
    publicSlot: 3, publicCommitment: 'ziffle:after-fetch-shuffle:3',
    originSlot: 17, originCommitment: 'ziffle:genesis:17',
  };
  const { publicSlot, publicCommitment, originSlot, originCommitment, ...base } = opening;
  const aliases = {
    ...base, public_slot: publicSlot, public_commitment: publicCommitment,
    origin_slot: originSlot, origin_commitment: originCommitment,
  };
  const fresh = freshRequirementsHarness(new Map([[50, [opening]]]));
  assert.deepEqual(fresh(99, [aliases]), [], 'camelCase and snake_case describe the same identity');
  for (const changed of [
    { ...opening, publicSlot: 4 },
    { ...opening, publicCommitment: 'ziffle:next-shuffle:3' },
    { ...opening, originSlot: 18 },
    { ...opening, originCommitment: 'ziffle:other-origin:17' },
  ]) {
    assert.deepEqual(fresh(99, [changed]), [changed], 'each identity component participates in the replay key');
  }
});
