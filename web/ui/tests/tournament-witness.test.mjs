import { fileURLToPath } from 'node:url';
import { webcrypto } from 'node:crypto';
import test from 'node:test';
import assert from 'node:assert/strict';
import { createServer } from 'vite';
import { chromium } from 'playwright';
import { startRelay } from '../../relay/tests/runtime.mjs';
import { createAuditSessionKey, exportAuditPublicKey, randomAuditHex, signAuditPayload } from '../src/lib/multiplayer-audit.js';
import {
  WITNESS_FORFEIT_REASON, encodeInviteCode, invitePayload, tournamentDescriptor, tournamentIdForDescriptor, witnessPublicKeyFromPrivateJwk,
} from '../src/lib/tournament/witness-protocol.js';

const DAY = 24 * 60 * 60 * 1000;

async function organizerInvites(names) {
  const keyPair = await createAuditSessionKey();
  const tournament = tournamentDescriptor({
    name: 'Friday Modern', organizerPublicKey: await exportAuditPublicKey(keyPair), createdAt: Date.now(), nonce: randomAuditHex(16),
  });
  const codes = [];
  for (const playerName of names) {
    const payload = invitePayload({ tournament, inviteId: randomAuditHex(16), playerName, expiresAt: Date.now() + DAY });
    codes.push(encodeInviteCode({ payload, signature: await signAuditPayload(keyPair, payload) }));
  }
  return { tournamentId: await tournamentIdForDescriptor(tournament), codes };
}

async function setup(t, port) {
  const base = `http://127.0.0.1:${port}`;
  const { privateKey } = await webcrypto.subtle.generateKey({ name: 'ECDSA', namedCurve: 'P-256' }, true, ['sign', 'verify']);
  const jwk = await webcrypto.subtle.exportKey('jwk', privateKey);
  const witnessKey = await witnessPublicKeyFromPrivateJwk(jwk);
  const relay = await startRelay(base, { WITNESS_SIGNING_KEY: JSON.stringify(jwk), WITNESS_ANSWER_WINDOW_MS: '3000' });
  t.after(() => relay.dispose());
  const url = String(await relay.ready).replace(/\/$/, '');
  const server = await createServer({ root: fileURLToPath(new URL('..', import.meta.url)),
    define: {
      'import.meta.env.VITE_LOBBY_RELAY_URL': JSON.stringify(url),
      'import.meta.env.VITE_WITNESS_PUBLIC_KEYS': JSON.stringify(witnessKey),
    },
    server: { host: '127.0.0.1', port, strictPort: true }, logLevel: 'error' });
  await server.listen(); t.after(() => server.close());
  const browser = await chromium.launch({ headless: true }); t.after(() => browser.close());
  const pages = [];
  for (const label of ['host', 'guest']) {
    const page = await (await browser.newContext()).newPage();
    page.on('pageerror', e => console.error(`${label} page error:`, e.message));
    await page.goto(`${base}/tests/fixtures/peer-lobby-harness.html`);
    await page.waitForFunction(() => window.__peerHarness?.ready);
    pages.push(page);
  }
  return { pages, witnessKey };
}

const lobby = page => page.evaluate(() => window.__peerHarness.snapshot());
async function waitFor(page, predicate, label, timeoutMs = 30000) {
  const end = Date.now() + timeoutMs;
  let snap;
  while (Date.now() < end) {
    snap = await lobby(page);
    if (predicate(snap)) return snap;
    await new Promise(r => setTimeout(r, 100));
  }
  console.error(label, JSON.stringify({ mode: snap?.multiplayer?.mode, statuses: snap?.statusEvents?.slice(-8) }, null, 1));
  assert.fail(`Timed out: ${label}`);
}

async function startTournamentMatch(t, port) {
  const env = await setup(t, port);
  const [host, guest] = env.pages;
  const { tournamentId, codes } = await organizerInvites(['Alice', 'Bob']);
  await host.evaluate(code => window.__peerHarness.redeemTournamentInvite(code), codes[0]);
  await guest.evaluate(code => window.__peerHarness.redeemTournamentInvite(code), codes[1]);
  await host.evaluate(tournamentId => window.__peerHarness.createLobby({
    name: 'ignored', transport: 'websocket', format: 'modern', tournamentId, deckText: '60 Island',
  }), tournamentId);
  const hosted = await waitFor(host, s => s.multiplayer.mode === 'lobby' && s.multiplayer.lobbyId, 'host opens tournament lobby');
  assert.equal(hosted.multiplayer.securityMode, 'verified');
  assert.equal(hosted.multiplayer.localName, 'Alice');
  await guest.evaluate(lobbyId => window.__peerHarness.joinLobby({ name: 'ignored', lobbyId, deckText: '60 Mountain' }), hosted.multiplayer.lobbyId);
  await waitFor(host, s => s.canStartHostedMatch, 'both seats ready');
  await host.evaluate(() => window.__peerHarness.startHostedMatch());
  for (const page of [host, guest]) await waitFor(page, s => s.multiplayer.matchStarted, 'match starts');
  return { ...env, host, guest, tournamentId };
}

test('tournament matches are Verified, witness-attested and verifiable', { timeout: 120000 }, async t => {
  const { host, guest, tournamentId, witnessKey } = await startTournamentMatch(t, 5195);
  const guestSnap = await lobby(guest);
  assert.equal(guestSnap.multiplayer.tournament.tournamentId, tournamentId);
  assert.deepEqual(guestSnap.multiplayer.players.map(p => p.name), ['Alice', 'Bob']);
  await host.evaluate(() => window.__peerHarness.submitMultiplayerCommand({ type: 'priority_action', action_ref: { kind: 'test_priority_action', actor: 0, sequence: 0 } }, 'host acts'));
  for (const page of [host, guest]) await waitFor(page, s => s.multiplayer.lastAppliedSequence >= 1, 'first action applies');
  const report = await guest.evaluate(async () => {
    const { verifyLiveAuditTranscript } = await import('/src/lib/multiplayer-audit.js');
    const { auditTranscript } = await window.__peerHarness.snapshot();
    // The fake harness engine cannot replay; signatures and witness evidence still verify.
    const result = await verifyLiveAuditTranscript(auditTranscript, globalThis.crypto, { requireEngineReplay: false });
    return result.witness;
  });
  assert.equal(report.tournamentId, tournamentId);
  assert.equal(report.witnessPublicKey, witnessKey);
  assert.equal(report.witnessKeyPinned, true);
  assert.deepEqual(report.players.map(p => p.name), ['Alice', 'Bob']);
});

test('an unanswered witness challenge forfeits the silent seat with a verifiable certificate', { timeout: 120000 }, async t => {
  const { host, guest } = await startTournamentMatch(t, 5196);
  // A closed tab is a silent seat: nothing answers the witness.
  await guest.close();
  await host.evaluate(() => window.__peerHarness.openWitnessChallenge({ accusedSeat: 1, reason: 'disconnect' }));
  const after = await waitFor(host, s => s.syncEvents.some(e => e.command?.reason === 'witness_unanswered_challenge'), 'witness forfeit applies', 30000);
  const forfeit = after.syncEvents.find(e => e.command?.reason === WITNESS_FORFEIT_REASON).command;
  assert.equal(forfeit.player, 1);
  const verified = await host.evaluate(async () => {
    const { verifyLiveAuditTranscript } = await import('/src/lib/multiplayer-audit.js');
    const { auditTranscript } = await window.__peerHarness.snapshot();
    // The fake harness engine cannot replay; signatures and witness evidence still verify.
    const result = await verifyLiveAuditTranscript(auditTranscript, globalThis.crypto, { requireEngineReplay: false });
    return { actions: result.verifiedActions, witness: Boolean(result.witness) };
  });
  assert.equal(verified.witness, true);
  assert.ok(verified.actions >= 1);
});

test('an online seat answers the witness automatically and keeps its seat', { timeout: 120000 }, async t => {
  const { host, guest } = await startTournamentMatch(t, 5197);
  await host.evaluate(() => window.__peerHarness.openWitnessChallenge({ accusedSeat: 1, reason: 'protocol_response_timeout' }));
  await waitFor(guest, s => s.statusEvents.some(e => /Answered a tournament witness challenge/.test(e.message)), 'guest answers');
  await waitFor(host, s => s.statusEvents.some(e => /answered the tournament witness/.test(e.message)), 'host sees the answer');
  await new Promise(r => setTimeout(r, 4500));
  const snap = await lobby(host);
  assert.equal(snap.syncEvents.some(e => e.command?.reason === WITNESS_FORFEIT_REASON), false);
  assert.equal(snap.multiplayer.matchStarted, true);
});
