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
