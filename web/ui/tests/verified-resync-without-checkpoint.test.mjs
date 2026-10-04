import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { webcrypto } from 'node:crypto';
import { buildSignedResyncEnvelope, verifySignedResyncEnvelope, createAuditSessionKey } from '../src/lib/multiplayer-audit.js';
import { assertResyncCheckpointCarrier } from '../src/lib/resync-checkpoint-carrier.js';

const source = readFileSync(new URL('../src/hooks/peer-lobby/crypto-resync.js', import.meta.url), 'utf8');
const begin = source.indexOf('  const sendHostedStateMessage = useCallback(');
const end = source.indexOf('\n  function sequencedActionRelayKey', begin);
assert.ok(begin >= 0 && end > begin);

async function sender({ cached = null, trusted = false } = {}) {
  const keys = await createAuditSessionKey(webcrypto);
  const actions = [{ seq: 1, command: { type: 'priority_action' } }];
  const sent = [];
  let exports = 0;
  const mode = trusted ? 'trusted' : 'verified';
  const context = {
    useCallback: fn => fn,
    multiplayerRef: { current: { role: 'host', players: [{ index: 1, peerId: 'guest' }] } },
    normalizePlayerIndex: value => Number.isInteger(value) ? value : null,
    gameRef: { current: {
      exportSyncCheckpoint() { exports++; throw new Error('registered continuous effect requires an approved executable identity graph'); },
      exportRedactedSyncCheckpoint() { exports++; throw new Error('registered continuous effect requires an approved executable identity graph'); },
    } },
    sessionSecurityMode: () => mode,
    matchPayloadSecurityMode: () => mode,
    MULTIPLAYER_SECURITY_VERIFIED: 'verified',
    isTrustedMultiplayerSecurityMode: value => value === 'trusted',
    isVerifiedMultiplayerSecurityMode: value => value === 'verified',
    relayMatchId: () => 'match1',
    matchingActionPrefix: () => false,
    actionHistoryRef: { current: actions },
    selectResyncReplayCheckpoint: () => cached,
    wireStablePayload: value => value === undefined ? null : structuredClone(value),
    buildSignedResyncEnvelope: payload => buildSignedResyncEnvelope(payload, webcrypto),
    auditKeyPairRef: { current: keys },
    currentAuditMatchId: () => 'match1',
    resolveLocalPlayerIndex: () => 0,
    auditStateHashRef: { current: 'final-state' },
    INITIAL_AUDIT_STATE_HASH: 'initial-state',
    safeSend: (_conn, message) => sent.push(message),
    redactedMatchPayloadForPeer: match => match,
  };
  const send = new Function(...Object.keys(context), source.slice(begin, end) + '\nreturn sendHostedStateMessage;')(...Object.values(context));
  await send({ peer: 'guest' }, { type: 'state_sync', match: { auditMatchId: 'match1', securityMode: mode } });
  return { message: sent[0], exports, keys };
}

test('verified resync sends a signed complete transcript without requiring checkpoint export', async () => {
  const { message, exports, keys } = await sender();
  assert.equal(exports, 0);
  assert.equal(message.replayOnly, true);
  assert.equal(message.checkpoint, null);
  assert.equal(message.actions.length, 1);
  assert.equal(message.resyncEnvelope.checkpointSequence, undefined);
  const verified = await verifySignedResyncEnvelope({ envelope: message.resyncEnvelope,
    publicKey: keys.publicKey, checkpoint: message.checkpoint, actions: message.actions }, webcrypto);
  assert.equal(verified.valid, true);
  assert.equal(verified.checkpointSequence, null);
  assert.equal(message.resyncEnvelope.lastSequence, 1);
  assertResyncCheckpointCarrier(message, { trusted: false, verified: true });
  await assert.rejects(verifySignedResyncEnvelope({ envelope: message.resyncEnvelope,
    publicKey: keys.publicKey, checkpoint: {}, actions: message.actions }, webcrypto), /checkpoint hash/i);
  await assert.rejects(verifySignedResyncEnvelope({ envelope: message.resyncEnvelope,
    publicKey: keys.publicKey, checkpoint: null, actions: [] }, webcrypto), /action|sequence/i);
});

test('receiver rejects ambiguous and missing checkpoint carriers', () => {
  const mode = { trusted: false, verified: true };
  assert.throws(() => assertResyncCheckpointCarrier({}, mode), /missing WASM checkpoint/);
  assert.throws(() => assertResyncCheckpointCarrier({ checkpoint: [] }, mode), /missing WASM checkpoint/);
  assert.throws(() => assertResyncCheckpointCarrier({ replayOnly: true, checkpoint: {} }, mode), /cannot contain/);
  assert.throws(() => assertResyncCheckpointCarrier({ replayOnly: true, checkpoint: null,
    resyncEnvelope: { checkpointSequence: 1 } }, mode), /cannot claim/);
  assert.throws(() => assertResyncCheckpointCarrier({ replayOnly: true }, { trusted: false, verified: false }), /security mode/);
  assertResyncCheckpointCarrier({ replayOnly: true }, { trusted: true, verified: false });
  assertResyncCheckpointCarrier({ checkpoint: { version: 2 } }, mode);
});

test('verified resync retains a cached importable checkpoint and its signed sequence', async () => {
  const cached = { seq: 1, checkpoint: { version: 2, perspective: 1, players: [] } };
  const { message, exports, keys } = await sender({ cached });
  assert.equal(exports, 0);
  assert.equal(message.replayOnly, undefined);
  assert.deepEqual(message.checkpoint, cached.checkpoint);
  assert.equal(message.resyncEnvelope.checkpointSequence, 1);
  await verifySignedResyncEnvelope({ envelope: message.resyncEnvelope,
    publicKey: keys.publicKey, checkpoint: message.checkpoint, actions: message.actions }, webcrypto);
});

test('trusted resync still sends its existing replay-only message', async () => {
  const { message, exports } = await sender({ trusted: true });
  assert.equal(exports, 0);
  assert.equal(message.replayOnly, true);
  assert.equal(message.resyncEnvelope, undefined);
});
