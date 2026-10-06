import test from 'node:test';
import assert from 'node:assert/strict';
import { startAuditTranscriptReplayWithGame, applyAuditReplayActionWithGame } from '../src/lib/audit-replay.js';
import { buildZiffleInputDeck } from '../src/lib/ziffle-private-epochs.js';
import { CURRENT_AUDIT_PROTOCOL_VERSION, CURRENT_PUBLIC_AUDIT_CHECKPOINT_VERSION } from '../src/lib/multiplayer-audit.js';

// Review reproductions: these deliberately assert the required rejection.
// The mocked engine isolates replay authorization, not cryptographic validity.
async function fixture({ omitAllProofs = false } = {}) {
  const root = { owner: 0, deckCount: 2, deckHash: 'root', context: 'match', steps: [] };
  const inputs = ['ziffle:root:0', 'ziffle:root:1'];
  const proof = { type: 'ziffle_shuffle', requirementId: 'first', owner: 0,
    epoch: 1, zone: 'library', context: 'match:action:1:first', deckCount: 2,
    deckHash: 'first', steps: [], inputDeck: buildZiffleInputDeck([root], inputs) };
  const firstRequirement = { id: 'first', type: 'verifiable_shuffle', owner: 0,
    zone: 'library', inputCommitments: inputs, randomCountBefore: 20 };
  const nextRequirement = { id: 'hidden-trigger', type: 'verifiable_shuffle', owner: 0,
    zone: 'library', inputCommitments: ['ziffle:first:0', 'ziffle:first:1'], randomCountBefore: 21 };
  let queued = false, opened = false, dispatched = false;
  const game = {
    startMatch: async () => {},
    getHiddenCardState: async () => ({ objects: [] }),
    exportPublicAuditCheckpoint: async () => ({ version: CURRENT_PUBLIC_AUDIT_CHECKPOINT_VERSION }),
    previewCryptoRequirements: async () => [firstRequirement,
      ...(queued ? [{ type: 'public_open', owner: 0, publicSlot: 1,
        publicCommitment: 'ziffle:first:1', timing: 'pre' }] : []),
      ...(opened ? [nextRequirement] : [])],
    queueVerifiedHiddenLibraryEpoch: async () => { queued = true; },
    queueVerifiedHiddenLibraryOpening: async () => { opened = true; },
    dispatch: async () => { dispatched = true; },
  };
  await startAuditTranscriptReplayWithGame({ game, transcript: {
    protocolVersion: CURRENT_AUDIT_PROTOCOL_VERSION,
    match: { protocolVersion: CURRENT_AUDIT_PROTOCOL_VERSION, players: [{ index: 0 }, { index: 1 }], ziffleCeremonies: [root] },
  } });
  const action = { seq: 1, command: { type: 'priority_action', action_ref: { kind: 'pass_priority' } },
    audit: { seq: 1, shuffleProofs: omitAllProofs ? [] : [proof], openings: omitAllProofs ? [] : [{
      owner: 0, position: 1, positionCommitment: 'ziffle:first:1',
      originPosition: 1, originPositionCommitment: 'ziffle:first:1',
      slot: 1, card: 'Nexus of Fate', commitment: 'manifest-commitment', timing: 'pre',
    }] } };
  return { game, action, dispatched: () => dispatched };
}

test('review: current replay must reject a required epoch when all shuffle proofs are omitted', async () => {
  const h = await fixture({ omitAllProofs: true });
  await assert.rejects(applyAuditReplayActionWithGame(h), /missing|shuffle|proof/i);
  assert.equal(h.dispatched(), false);
});

test('review: current replay must reject an omitted epoch discovered after opening the first epoch', async () => {
  const h = await fixture();
  await assert.rejects(applyAuditReplayActionWithGame(h), /missing|shuffle|proof/i);
  assert.equal(h.dispatched(), false);
});
