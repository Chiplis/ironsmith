import { assertMatchNotDisputed } from "../src/hooks/peer-lobby/match-lifecycle.js";
import { isPrivateZiffleEpoch } from "../src/lib/ziffle-private-epochs.js";
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
    isPrivateZiffleEpoch,
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

test('private shuffle continuation deduplicates by ciphertext inputs despite retired runtime IDs', () => {
  const first = { id: 'verifiable_shuffle:0:library:10:14', type: 'verifiable_shuffle', owner: 0,
    zone: 'library', count: 3, randomCountBefore: 10, randomCountAfter: 14,
    inputCommitments: ['ziffle:previous:0', 'ziffle:previous:2', 'ziffle:previous:3'],
    beforeOrder: [10, 11, 12], afterOrder: [12, 10, 11] };
  const fresh = freshRequirementsHarness(new Map([[50, [first]]]));
  const replayed = { ...first, count: 2, beforeOrder: [90, 91, 92], afterOrder: [92, 90, 91] };
  assert.deepEqual(fresh(51, [replayed]), [], 'the prior shuffle event cannot create another epoch on replay');
  const differentInputs = { ...replayed, inputCommitments: ['ziffle:next:0', 'ziffle:next:2', 'ziffle:next:3'] };
  assert.deepEqual(fresh(51, [differentInputs]), [differentInputs], 'a distinct ciphertext frontier is not discarded');
  const { inputCommitments, ...withoutCamel } = replayed;
  assert.deepEqual(fresh(51, [{ ...withoutCamel, input_commitments: inputCommitments }]), []);
});

test('future private positions wait for epoch consumption before requesting or injecting their identities', async () => {
  const future = { type: 'private_open', owner: 0, viewer: 0, commitment: 'ziffle:future:1' };
  const current = { type: 'private_open', owner: 0, viewer: 0, commitment: 'ziffle:current:0' };
  let pending = true;
  const collected = [];
  const context = {
    resolveLocalCryptoPlayerIndex: () => 0,
    gameRef: { current: { pendingVerifiedHiddenLibraryPosition: async ({ deckHash }) =>
      pending && deckHash === 'future' ? { position: 1 } : null } },
    ziffleDeckHashFromCommitment: value => /^ziffle:([^:]+):/.exec(value)?.[1],
    zifflePositionFromCommitment: value => Number(value.split(':')[2]),
    batchedOwnerPrivateZiffleOpeningsForLocalViewer: async requirements => {
      collected.push(requirements);
      return { openings: [], handledRequirements: new Set(requirements) };
    },
  };
  const open = new Function(...Object.keys(context), `${extract(
    '  async function privateOpeningsForLocalViewer(', '\n\t  function hiddenPositionBatchRevealFromOpening('
  )}\nreturn privateOpeningsForLocalViewer;`)(...Object.values(context));
  await open([future, current]);
  assert.deepEqual(collected, [[current]], 'a future private identity never reaches public-opening fallback');
  pending = false;
  await open([future]);
  assert.deepEqual(collected[1], [future], 'the actual private hand can hydrate after its epoch is consumed');
});

test('receiver defers future encrypted private openings until the action installs their epoch', async () => {
  const calls = [];
  const context = {
    resolveLocalCryptoPlayerIndex: () => 1,
    ziffleDeckHashFromCommitment: value => /^ziffle:([^:]+):/.exec(value || '')?.[1],
    privateOpeningFromEncryptedProof: async proof => { calls.push('decrypt'); return { ...proof, card: 'Island', slot: 1 }; },
    sanitizeObjectBoundOpening: async opening => opening,
    ensureZiffleOpeningProof: async opening => { calls.push('verify'); return opening; },
    revealAuditOpenings: async () => calls.push('reveal'),
  };
  const reveal = new Function(...Object.keys(context), `${extract(
    '  async function revealPrivateAuditProofsForLocalViewer(', '\n  async function batchedOwnerPrivateZiffleOpeningsForLocalViewer('
  )}\nreturn revealPrivateAuditProofsForLocalViewer;`)(...Object.values(context));
  const audit = { seq: 8, privateViewProofs: [{ type: 'encrypted_private_opening', owner: 0,
    viewer: 1, positionCommitment: 'ziffle:future:1' }] };
  await reveal(audit, { deferPrivateEpochDeckHashes: ['future'] });
  assert.deepEqual(calls, []);
  await reveal(audit);
  assert.deepEqual(calls, ['decrypt', 'verify', 'reveal']);
});

test('an unsigned crypto-material request cannot reserve a private shuffle authorization lock', async () => {
  const declaration = extract('  const authorizedCryptoMaterialRequirementsForRequest =', '\n  const answerCryptoMaterialRequest =');
  const callbackStart = declaration.indexOf('async (conn, message) => {');
  const callbackEnd = declaration.lastIndexOf('}, [');
  assert.ok(callbackStart >= 0 && callbackEnd > callbackStart);
  const calls = [];
  const phases = [];
  const context = {
    assertMatchNotDisputed,
    summarizePeerCommand: command => ({ type: command.type }),
    timePeerSyncPhase: async (label, metadata, task) => {
      phases.push({ label, metadata });
      return task();
    },
    multiplayerRef: { current: { matchStarted: true, lastAppliedSequence: 7 } },
    currentAuditMatchId: () => 'match',
    playerIndexForPeerId: () => 0,
    normalizePlayerIndex: Number,
    auditStateHashRef: { current: 'head' },
    INITIAL_AUDIT_STATE_HASH: 'initial',
    gameRef: { current: { uiState: async () => ({ decision: { player: 0 } }) } },
    isDecisionCommandCompatible: () => true,
    verifyCurrentPublicCheckpointHash: async () => {},
    verifySignedActionIntent: async () => { calls.push('signature'); throw new Error('Invalid signature'); },
    resolveLocalCryptoPlayerIndex: () => 1,
    previewRequirementsForCommand: async () => { calls.push('preview'); return []; },
    freshCryptoRequirementsForSequence: (_seq, requirements) => requirements,
    filterCryptoRequirementsForCommand: (_command, _state, requirements) => requirements,
    servicesRef: { current: { previewZiffleActionRequirements: async () => calls.push('lock') } },
  };
  const authorize = new Function(...Object.keys(context), `return (${declaration.slice(callbackStart, callbackEnd)} });`)(...Object.values(context));
  await assert.rejects(authorize({ peer: 'actor' }, { matchId: 'match', requesterIndex: 0,
    actorIndex: 0, seq: 8, prevStateHash: 'head', publicCheckpointHash: 'checkpoint',
    command: { type: 'priority_action' }, actionIntent: { signature: 'invalid' } }), /Invalid signature/);
  assert.deepEqual(calls, ['signature']);
  assert.deepEqual(phases.map(({ label }) => label), [
    'wait_action_head', 'read_state', 'verify_checkpoint', 'verify_intent',
  ].map(phase => `crypto_material_request:authorize:${phase}`));
  for (const { metadata } of phases) {
    assert.equal(metadata.seq, 8);
    assert.equal(metadata.actor, 0);
    assert.deepEqual(metadata.command, { type: 'priority_action' });
    assert.equal('actionIntent' in metadata, false);
  }
});
