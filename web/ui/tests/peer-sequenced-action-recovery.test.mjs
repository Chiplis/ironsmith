import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import { sequencedActionSignedPayload } from '../src/lib/multiplayer-audit.js';
const source = readFileSync(new URL('../src/hooks/peer-lobby/validation.js', import.meta.url), 'utf8');
function implementation(name) {
  const start = source.search(new RegExp(`^  (?:async )?function ${name}\\(`, 'm'));
  const end = source.indexOf('\n  }\n', start);
  assert.ok(start >= 0 && end > start);
  return source.slice(start, end + 4);
}
function harness() {
  const recovery = [], buffers = [];
  const c = {
    assertMatchNotDisputed() {}, multiplayerRef: { current: { lastAppliedSequence: 218, securityMode: 'verified' } },
    actionHistoryEntryForSequence: () => null, awaitingStateResyncRef: { current: false },
    pendingSequencedActionsRef: { current: new Map() },
    sequencedActionsEquivalent: () => false, sessionSecurityMode: s => s.securityMode,
    isVerifiedMultiplayerSecurityMode: mode => mode === 'verified',
    isTrustedMultiplayerSecurityMode: mode => mode === 'trusted',
    currentAuditMatchId: () => 'match', canonicalMultiplayerPayload: JSON.stringify,
    importCachedAuditPublicKey: async key => key, publicKeyForAuditSigner: () => 'key',
    sequencedActionSignedPayload,
    verifyAuditPayload: async (_key, _payload, signature) => signature === 'valid',
    bufferFutureSequencedAction: message => { buffers.push(message); return true; },
    servicesRef: { current: { requestMissingSequencedActions: message => recovery.push(message) } },
    setStatus() {}, reportSyncFailure() {},
  };
  vm.createContext(c); vm.runInContext(implementation('applySequencedActionMessageInner'), c);
  const command = { type: 'select_options', option_indices: [] };
  const message = { seq: 220, actorIndex: 1, command, audit: { seq: 220, actor: 1,
    matchId: 'match', command, signature: 'valid', prevStateHash: 'hash-219' } };
  return { c, message, recovery, buffers };
}
test('a future signed action starts recovery without applying or accepting it', async () => {
  const h = harness(); const result = await h.c.applySequencedActionMessageInner(h.message);
  assert.equal(result.buffered, true); assert.equal(h.recovery.length, 1);
  assert.equal(h.buffers.length, 1); assert.equal(h.c.multiplayerRef.current.lastAppliedSequence, 218);
});
test('unsigned future traffic cannot pause a Verified match or enter its action buffer', async () => {
  const h = harness(); h.message.audit.signature = 'forged';
  await assert.rejects(h.c.applySequencedActionMessageInner(h.message), /signature is invalid/);
  assert.equal(h.recovery.length, 0); assert.equal(h.buffers.length, 0);
});
test('recovery requests stay within a bounded action window', async () => {
  const h = harness(); h.message.seq = h.message.audit.seq = 500;
  await assert.rejects(h.c.applySequencedActionMessageInner(h.message), /bounded recovery window/);
  assert.equal(h.recovery.length, 0);
});
