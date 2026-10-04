import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { webcrypto } from 'node:crypto';
import { buildSignedResyncEnvelope, verifySignedResyncEnvelope, createAuditSessionKey } from '../src/lib/multiplayer-audit.js';
import { assertResyncTranscriptCarrier } from '../src/lib/resync-transcript-carrier.js';

const source = readFileSync(new URL('../src/hooks/peer-lobby/crypto-resync.js', import.meta.url), 'utf8');
const begin = source.indexOf('  const sendHostedStateMessage = useCallback(');
const end = source.indexOf('\n  function sequencedActionRelayKey', begin);
assert.ok(begin >= 0 && end > begin);

async function sender({ trusted = false } = {}) {
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

test('verified recovery sends only signed actions without exporting engine state', async () => {
  const { message, exports, keys } = await sender();
  assert.equal(exports, 0);
  assert.equal(message.replayOnly, true);
  assert.equal('checkpoint' in message, false);
  assert.equal(message.actions.length, 1);
  const report = await verifySignedResyncEnvelope({ envelope: message.resyncEnvelope,
    publicKey: keys.publicKey, actions: message.actions }, webcrypto);
  assert.equal(report.valid, true);
  assertResyncTranscriptCarrier(message, { trusted: false, verified: true });
  await assert.rejects(verifySignedResyncEnvelope({ envelope: message.resyncEnvelope,
    publicKey: keys.publicKey, actions: [] }, webcrypto), /action|sequence/i);
});

test('trusted recovery also sends actions without engine serialization', async () => {
  const { message, exports } = await sender({ trusted: true });
  assert.equal(exports, 0);
  assert.equal(message.replayOnly, true);
  assert.equal('checkpoint' in message, false);
  assertResyncTranscriptCarrier(message, { trusted: true, verified: false });
});

test('recovery rejects foreign engine state and unsupported carrier modes', () => {
  const mode = { trusted: false, verified: true };
  assert.throws(() => assertResyncTranscriptCarrier({}, mode), /replay-only/);
  assert.throws(() => assertResyncTranscriptCarrier({ replayOnly: true, checkpoint: {} }, mode), /serialized engine/);
  assert.throws(() => assertResyncTranscriptCarrier({ replayOnly: true,
    resyncEnvelope: { checkpointSequence: 1 } }, mode), /cannot claim/);
});
