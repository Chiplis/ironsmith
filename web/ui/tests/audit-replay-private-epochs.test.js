import test from 'node:test';
import assert from 'node:assert/strict';
import { startAuditTranscriptReplayWithGame, applyAuditReplayActionWithGame } from '../src/lib/audit-replay.js';
import { buildZiffleInputDeck } from '../src/lib/ziffle-private-epochs.js';
import { CURRENT_AUDIT_PROTOCOL_VERSION, CURRENT_PUBLIC_AUDIT_CHECKPOINT_VERSION } from '../src/lib/multiplayer-audit.js';

async function fixture({ disclose = true, inputOverride = null, fairRandomFirst = false } = {}) {
  const genesis = { owner: 0, deckCount: 8, deckHash: 'genesis', context: 'private-replay', steps: [] };
  const firstInputs = [0, 2, 4, 7].map(position => `ziffle:genesis:${position}`);
  const first = { type: 'ziffle_shuffle', requirementId: 'shuffle-first', owner: 0, epoch: 1, zone: 'library',
    context: 'private-replay:action:1:shuffle:first', deckCount: 4, deckHash: 'first', steps: [],
    inputDeck: buildZiffleInputDeck([genesis], firstInputs) };
  const secondInputs = [...Array.from({ length: 4 }, (_, position) => `ziffle:first:${position}`), 'ziffle:genesis:1'];
  const second = { type: 'ziffle_shuffle', requirementId: 'shuffle-hidden-trigger', owner: 0, epoch: 1, zone: 'library',
    context: 'private-replay:action:1:shuffle:hidden-trigger', deckCount: 5, deckHash: 'second', steps: [],
    inputDeck: buildZiffleInputDeck([genesis, first], secondInputs) };
  const opening = { owner: 0, position: 3, positionCommitment: 'ziffle:first:3',
    originPosition: 3, originPositionCommitment: 'ziffle:first:3', slot: 7,
    card: 'Nexus of Fate', commitment: 'salted-nexus', timing: 'pre' };
  const calls = [], queued = [], openings = [];
  let randomReady = !fairRandomFirst;
  const firstRequirement = { id: first.requirementId, type: 'verifiable_shuffle', owner: 0,
    zone: 'library', inputCommitments: inputOverride || firstInputs, randomCountBefore: 20 };
  const secondRequirement = { id: second.requirementId, type: 'verifiable_shuffle', owner: 0,
    zone: 'library', inputCommitments: secondInputs, randomCountBefore: 21 };
  const game = {
    startMatch: async () => {}, setPerspective: async () => {},
    getHiddenCardState: async () => ({ objects: [] }),
    exportPublicAuditCheckpoint: async () => ({ version: CURRENT_PUBLIC_AUDIT_CHECKPOINT_VERSION }),
    previewCryptoRequirements: async () => {
      calls.push(['preview', queued.length, openings.length]);
      if (!randomReady) return [{ type: 'fair_random', id: 'coin-flip' }];
      return [firstRequirement,
        ...(queued.length && disclose ? [{ type: 'public_open', owner: 0,
          publicSlot: 3, publicCommitment: 'ziffle:first:3', timing: 'pre' }] : []),
        ...(openings.length ? [secondRequirement] : [])];
    },
    queueVerifiedHiddenLibraryEpoch: async value => { queued.push(value); calls.push(['queue-epoch', value.deckHash]); },
    queueVerifiedHiddenLibraryOpening: async value => { openings.push(value); calls.push(['queue-opening', value.cardName]); },
    injectTranscriptRandomSeeds: async ({ seeds }) => {
      if (!fairRandomFirst) throw new Error('Opaque shuffles cannot become publicly deterministic seeds');
      assert.deepEqual(seeds, ['coin-flip-seed']);
      randomReady = true; calls.push(['inject-randomness']);
    },
    applyVerifiedHiddenLibraryShuffle: async () => { throw new Error('Opaque shuffles cannot attach legacy public object orders'); },
    revealHiddenPosition: async () => { throw new Error('Future output does not exist before the shuffle boundary'); },
    dispatch: async command => calls.push(['dispatch', command]),
  };
  const transcript = { protocolVersion: CURRENT_AUDIT_PROTOCOL_VERSION,
    match: { protocolVersion: CURRENT_AUDIT_PROTOCOL_VERSION, players: [{ index: 0 }, { index: 1 }], ziffleCeremonies: [genesis] } };
  await startAuditTranscriptReplayWithGame({ game, transcript });
  const action = { seq: 1, command: { type: 'priority_action', action_ref: { kind: 'pass_priority' } },
    audit: { seq: 1, shuffleProofs: [first, second], openings: [opening],
      rngReveals: fairRandomFirst ? [{ requirementId: 'coin-flip', combinedSeedHex: 'coin-flip-seed' }] : [] } };
  return { game, action, calls, queued, openings, firstInputs, secondInputs };
}

test('replay discovers a second shuffle only after the first epoch public opening is hydrated', async () => {
  const h = await fixture();
  await applyAuditReplayActionWithGame({ game: h.game, action: h.action });
  assert.deepEqual(h.queued.map(epoch => epoch.deckHash), ['first', 'second']);
  assert.deepEqual(h.queued.map(epoch => epoch.randomCountBefore), [20, 21]);
  assert.deepEqual(h.queued[0].expectedInputs, h.firstInputs);
  assert.deepEqual([...h.queued[1].expectedInputs].sort(), [...h.secondInputs].sort());
  assert.deepEqual(h.openings, [{ owner: 0, deckHash: 'first', position: 3, cardName: 'Nexus of Fate',
    originalSlot: 7, commitment: 'salted-nexus' }]);
  assert.deepEqual(h.calls.filter(call => call[0] !== 'preview').map(call => call.slice(0, 2)), [
    ['queue-epoch', 'first'], ['queue-opening', 'Nexus of Fate'], ['queue-epoch', 'second'],
    ['dispatch', h.action.command],
  ]);
  assert.ok(h.calls.some(call => call[0] === 'preview' && call[1] === 1 && call[2] === 0),
    'public disclosure is authorized by a preview after the first epoch and before its opening');
});

test('replay rejects a valid private proof whose same-size input set differs from local rules', async () => {
  const h = await fixture({ inputOverride: [0, 2, 4, 6].map(position => `ziffle:genesis:${position}`) });
  await assert.rejects(applyAuditReplayActionWithGame({ game: h.game, action: h.action }), /locally required library/);
  assert.deepEqual(h.queued, []);
  assert.equal(h.calls.some(call => call[0] === 'dispatch'), false);
});

test('replay rejects a future ciphertext opening without a local public disclosure requirement', async () => {
  const h = await fixture({ disclose: false });
  await assert.rejects(applyAuditReplayActionWithGame({ game: h.game, action: h.action }), /not required for public disclosure/);
  assert.deepEqual(h.openings, []);
  assert.equal(h.calls.some(call => call[0] === 'dispatch'), false);
});

test('replay rejects a future opening that tries to retain another epoch origin', async () => {
  const h = await fixture();
  h.action.audit.openings[0].originPositionCommitment = 'ziffle:genesis:3';
  await assert.rejects(applyAuditReplayActionWithGame({ game: h.game, action: h.action }), /not bound to its new ciphertext position/);
  assert.deepEqual(h.openings, []);
  assert.equal(h.calls.some(call => call[0] === 'dispatch'), false);
});


test('replay injects agreed fair randomness before discovering dependent private shuffle boundaries', async () => {
  const h = await fixture({ fairRandomFirst: true });
  await applyAuditReplayActionWithGame({ game: h.game, action: h.action });
  const events = h.calls.filter(call => call[0] !== 'preview').map(call => call[0]);
  assert.deepEqual(events, ['inject-randomness', 'queue-epoch', 'queue-opening', 'queue-epoch', 'dispatch']);
  assert.deepEqual(h.queued.map(epoch => epoch.deckHash), ['first', 'second']);
});
