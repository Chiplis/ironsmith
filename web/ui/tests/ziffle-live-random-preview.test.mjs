import test from 'node:test';
import assert from 'node:assert/strict';
import { authorizationHarness } from './ziffle-reveal-authorization-harness.mjs';
import { buildZiffleInputDeck } from '../src/lib/ziffle-private-epochs.js';

// Production authorization preview, with deterministic engine and crypto mocks:
// a coin result exposes a shuffle; only that output authorizes the reveal.
function fixture({ invalid = false, install = false } = {}) {
  const genesis = { owner: 0, deckCount: 2, context: 'match', deckHash: 'root',
    steps: [{ shuffler: 0, deckHex: 'root', proofHex: 'valid' }] };
  const random = { id: 'coin', type: 'fair_random', count: 1 };
  const requirement = { id: 'coin-shuffle', type: 'verifiable_shuffle', owner: 0, zone: 'library',
    randomCountBefore: 13, inputCommitments: ['ziffle:root:0', 'ziffle:root:1'] };
  const proof = { owner: 0, zone: 'library', requirementId: requirement.id, deckCount: 2,
    context: 'match:action:8:shuffle:coin-shuffle:0:library', keyContext: 'match', deckHash: 'new',
    keys: ['signed-roster'], steps: [{ shuffler: 0, deckHex: 'new', proofHex: 'valid' }],
    inputDeck: buildZiffleInputDeck([genesis], requirement.inputCommitments) };
  const opening = { type: 'private_open', owner: 0, viewer: 0, zone: 'hand', slot: 1,
    commitment: 'ziffle:new:1', originSlot: 1, originCommitment: 'ziffle:new:1' };
  const calls = [];
  const game = {
    uiState: async () => ({ decision: { kind: 'priority', player: 0 } }),
    previewCryptoRequirementsWithMaterial: async (_command, material) => {
      calls.push(['preview', structuredClone(material)]);
      assert.equal(calls[0][0], 'authenticate', 'no unverified result reaches the engine preview');
      if (material.seeds[0] !== 'agreed-coin') return [random];
      return [random, requirement, ...(material.libraryEpochs.length ? [opening] : [])];
    },
    ziffleVerifyShuffle: async () => {
      calls.push(['shuffle']);
      return { deckHash: 'new', deckCount: 2, universeCount: 2, rootDeckHash: 'root', rootContext: 'match' };
    },
    injectTranscriptRandomSeeds: async material => calls.push(['inject', structuredClone(material)]),
    queueVerifiedHiddenLibraryEpoch: async material => calls.push(['epoch', material]),
  };
  const h = authorizationHarness({ requirements: [random],
    match: { protocolVersion: 15, ziffleCeremonies: [genesis] },
    overrides: { gameRef: { current: game },
      verifyAuditSatisfiesCryptoRequirements: async ({ requirements, audit }) => {
        calls.push(['authenticate']);
        assert.deepEqual(requirements, [random]);
        assert.equal(audit.seq, 8);
        if (invalid || audit.rngReveals[0]?.combinedSeedHex !== 'agreed-coin') throw new Error('Invalid committed random result');
      } },
  });
  Object.assign(h.message.actionAuthorization, { shuffleProofs: [proof],
    rngReveals: [{ requirementId: 'coin', combinedSeedHex: 'agreed-coin' }] });
  return { h, proof, random, calls, run: () => h.preview(h.message.actionAuthorization, [random], { install }) };
}

test('agreed fair randomness is verified before revealing a conditional private shuffle', async () => {
  const { h, proof, calls } = fixture();
  assert.equal(await h.authorize(h.message, 0, 0, [1], proof), true);
  assert.deepEqual(calls.filter(([type]) => type === 'authenticate'), [['authenticate']]);
  assert.deepEqual(calls.find(([type]) => type === 'preview')[1].seeds, ['agreed-coin']);
  assert.equal(calls.filter(([type]) => type === 'inject' || type === 'epoch').length, 0,
    'authorization is a temporary preview and does not append random seeds or install epochs');
});

test('invalid and omitted fair-random material cannot choose an authorization preview', async () => {
  for (const omit of [false, true]) {
    const { h, run, calls } = fixture({ invalid: !omit });
    if (omit) h.message.actionAuthorization.rngReveals = [];
    await assert.rejects(run(), /Invalid committed random result/);
    assert.deepEqual(calls, [['authenticate']]);
  }
});

test('receiver preparation installs each agreed random seed once before its private epoch', async () => {
  const { run, calls } = fixture({ install: true });
  await run();
  const installed = calls.filter(([type]) => type === 'inject' || type === 'epoch');
  assert.deepEqual(installed.map(([type]) => type), ['inject', 'epoch']);
  assert.deepEqual(installed[0][1].seeds, ['agreed-coin']);
});

test('signed action audit randomness uses the same authenticated preview path', async () => {
  const { h, run, calls } = fixture();
  const auth = h.message.actionAuthorization;
  auth.actionAudit = { signature: 'valid', rngReveals: auth.rngReveals };
  delete auth.rngReveals;
  await run();
  assert.equal(calls[0][0], 'authenticate');
});

test('same-epoch public openings converge when revealing one card exposes the next requirement', async () => {
  const genesis = { owner: 0, deckCount: 2, context: 'match', deckHash: 'root',
    steps: [{ shuffler: 0, deckHex: 'root', proofHex: 'valid' }] };
  const requirement = { id: 'shuffle', type: 'verifiable_shuffle', owner: 0, zone: 'library',
    randomCountBefore: 13, inputCommitments: ['ziffle:root:0', 'ziffle:root:1'] };
  const proof = { owner: 0, zone: 'library', requirementId: requirement.id, deckCount: 2,
    context: 'match:action:8:shuffle:shuffle:0:library', keyContext: 'match', deckHash: 'new',
    keys: ['signed-roster'], steps: [{ shuffler: 0, deckHex: 'new', proofHex: 'valid' }],
    inputDeck: buildZiffleInputDeck([genesis], requirement.inputCommitments) };
  const publicRequirement = position => ({ type: 'public_open', owner: 0, zone: 'library',
    publicSlot: position, publicCommitment: `ziffle:new:${position}` });
  const publicOpening = position => ({ owner: 0, slot: position, position,
    positionCommitment: `ziffle:new:${position}`, card: 'Island', timing: 'pre', salt: 'salt' });
  const authenticated = [];
  const h = authorizationHarness({ requirements: [requirement],
    match: { protocolVersion: 15, ziffleCeremonies: [genesis] }, overrides: {
      assertZiffleOpeningOriginMatchesMetadata: () => {},
      verifyCardOpeningAgainstManifest: async () => true,
      servicesRef: { current: { publicDeckManifestForOwner: () => ({}),
        verifyZiffleOpeningCryptographicProof: async opening => authenticated.push(opening.position) } },
      gameRef: { current: {
        ziffleVerifyShuffle: async () => ({ deckHash: 'new', deckCount: 2, universeCount: 2,
          rootDeckHash: 'root', rootContext: 'match' }),
        previewCryptoRequirementsWithMaterial: async (_command, material) => [requirement,
          publicRequirement(0), ...(material.libraryEpochOpenings.some(opening => opening.position === 0)
            ? [publicRequirement(1)] : [])],
      } },
    } });
  const auth = { ...h.message.actionAuthorization, shuffleProofs: [proof],
    openings: [publicOpening(1), publicOpening(0)] };
  await h.preview(auth, [requirement]);
  assert.deepEqual(authenticated, [0, 1], 'public audit ordering cannot bypass or block incremental local authorization');
  await assert.rejects(h.preview({ ...auth, openings: [publicOpening(1)] }, [requirement]), /not a locally authorized public reveal/);
});
