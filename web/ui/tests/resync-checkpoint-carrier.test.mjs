// Source-authored; unrun under the deferred-validation workflow.
import test from 'node:test';
import assert from 'node:assert/strict';
import { webcrypto } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { assertResyncCheckpointCarrier } from '../src/lib/resync-checkpoint-carrier.js';
import { buildSignedResyncEnvelope, verifySignedResyncEnvelope, createAuditSessionKey,
  exportAuditPublicKey, importAuditPublicKey } from '../src/lib/multiplayer-audit.js';

test('verified replay-only has no checkpoint or checkpoint sequence authority', () => {
  const mode = { trusted: false, verified: true };
  assert.doesNotThrow(() => assertResyncCheckpointCarrier({ replayOnly: true, checkpoint: null }, mode));
  for (const message of [
    { replayOnly: true, checkpoint: {} },
    { replayOnly: true, checkpoint: null, resyncEnvelope: { checkpointSequence: 5 } },
    { checkpoint: null }, { checkpoint: [] },
  ]) assert.throws(() => assertResyncCheckpointCarrier(message, mode));
  assert.throws(() => assertResyncCheckpointCarrier({ replayOnly: true }, { trusted: false, verified: false }));
});

test('null-checkpoint replay retains signed action-log and envelope verification', async () => {
  const keyPair = await createAuditSessionKey(webcrypto);
  const publicKey = await importAuditPublicKey(await exportAuditPublicKey(keyPair, webcrypto), webcrypto);
  const actions = [{ seq: 1, actorIndex: 0, command: { type: 'pass_priority' }, audit: { signature: 'action-envelope' } }];
  const envelope = await buildSignedResyncEnvelope({ keyPair, matchId: 'shield-match', signer: 0,
    lastSequence: 1, finalStateHash: 'a'.repeat(64), checkpoint: null, checkpointSequence: null, actions }, webcrypto);
  assertResyncCheckpointCarrier({ replayOnly: true, checkpoint: null, resyncEnvelope: envelope }, { trusted: false, verified: true });
  const verify = (overrides = {}) => verifySignedResyncEnvelope({ envelope, publicKey, checkpoint: null, actions, ...overrides }, webcrypto);
  assert.equal((await verify()).valid, true);
  assert.equal((await verify()).checkpointSequence, null);
  await assert.rejects(() => verify({ envelope: null }), /missing signed envelope/);
  await assert.rejects(() => verify({ envelope: { ...envelope, matchId: 'other-match' } }), /signature/);
  await assert.rejects(() => verify({ actions: [{ ...actions[0], command: { type: 'forfeit_player', player: 1 } }] }), /action log hash/);
  await assert.rejects(() => verify({ checkpoint: {} }), /checkpoint hash/);
});

// The authenticated public-board hash does not bind registered runtime effects.
// This source contract guards against reintroducing a host-signed shortcut before
// a complete rules-state proof exists; the peer scenario exercises real replay.
test('Verified receiver cannot import a forged empty-runtime checkpoint or skip its prefix', () => {
  const receiver = readFileSync(new URL('../src/hooks/peer-lobby/messaging.js', import.meta.url), 'utf8');
  const sender = readFileSync(new URL('../src/hooks/peer-lobby/crypto-resync.js', import.meta.url), 'utf8');
  assert.equal(receiver.includes('importForeignSyncCheckpoint'), false);
  assert.equal(receiver.includes('importVerifiedResyncCheckpoint'), false);
  assert.match(receiver, /await restartFromAcceptedGenesis\(replayMatchPayload\);\s+await replaySignedActions\(0\);/);
  for (const verification of ['await verifySignedMatchGenesis(matchPayload)', 'await verifyLiveAuditTranscript(', 'await verifySignedResyncEnvelope(']) {
    assert.ok(receiver.indexOf(verification) >= 0);
    assert.ok(receiver.indexOf(verification) < receiver.indexOf('await restartFromAcceptedGenesis(replayMatchPayload)'));
  }
  assert.match(sender, /const replayOnly = trusted \|\| verified;/);
  assert.match(sender, /checkpointSequence: null/);
  assert.equal(sender.includes('selectResyncReplayCheckpoint'), false);
});
