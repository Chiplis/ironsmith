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

async function setup(t, port, labels = ['host', 'guest']) {
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
  for (const label of labels) {
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
  console.error(label, JSON.stringify({ mode: snap?.multiplayer?.mode, role: snap?.multiplayer?.role, started: snap?.multiplayer?.matchStarted, seq: snap?.multiplayer?.lastAppliedSequence, hostPeerId: snap?.multiplayer?.hostPeerId, localPeerId: snap?.multiplayer?.localPeerId, statuses: snap?.statusEvents?.slice(-14), notices: snap?.noticeEvents?.slice(-6) }));
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

test('a Verified relay match moves to the guest when the host leaves and stays verifiable after the host returns', { timeout: 240000 }, async t => {
  const { host, guest, witnessKey } = await startTournamentMatch(t, 5196);
  // The harness engine cannot submit Verified actions, so this covers the
  // takeover and the reconnect handshake at the match's genesis.
  const before = await lobby(guest);
  const hostContext = host.context(), fixtureUrl = host.url();
  await host.close();
  const promoted = await waitFor(guest, s => s.multiplayer.role === 'host', 'guest takes over the Verified match', 90000);
  assert.equal(promoted.multiplayer.hostPeerId, before.multiplayer.localPeerId);
  assert.equal(promoted.multiplayer.securityMode, 'verified');
  assert.equal(promoted.multiplayer.lastAppliedSequence, 0);

  const returning = await hostContext.newPage();
  returning.on('pageerror', e => console.error('returning host page error:', e.message));
  await returning.goto(fixtureUrl);
  await returning.waitForFunction(() => window.__peerHarness?.ready);
  await returning.evaluate(lobbyId => window.__peerHarness.joinLobby({ name: 'ignored', lobbyId }), before.multiplayer.lobbyId);
  const rejoined = await waitFor(returning, s => s.multiplayer.role === 'client' && s.multiplayer.matchStarted
    && s.multiplayer.lastAppliedSequence === 0, 'original host rejoins as a guest', 90000);
  assert.equal(rejoined.multiplayer.hostPeerId, before.multiplayer.localPeerId);
  assert.equal(rejoined.multiplayer.localPlayerIndex, 0);
  await waitFor(guest, s => s.multiplayer.players.every(p => p.connected !== false), 'promoted host sees the seat return', 60000);
  const report = await guest.evaluate(async () => {
    const { verifyLiveAuditTranscript } = await import('/src/lib/multiplayer-audit.js');
    const { auditTranscript } = await window.__peerHarness.snapshot();
    return (await verifyLiveAuditTranscript(auditTranscript, globalThis.crypto, { requireEngineReplay: false })).witness;
  });
  assert.equal(report.witnessPublicKey, witnessKey);
  assert.deepEqual(report.players.map(p => p.name), ['Alice', 'Bob']);
});

test('a three-seat Verified match splits every seat key and rebuilds a forfeited seat\'s key', { timeout: 240000 }, async t => {
  const env = await setup(t, 5198, ['host', 'second', 'third']);
  const [host, second, third] = env.pages;
  const names = ['Alice', 'Bob', 'Cara'];
  const { tournamentId, codes } = await organizerInvites(names);
  for (const [index, page] of env.pages.entries()) {
    await page.evaluate(code => window.__peerHarness.redeemTournamentInvite(code), codes[index]);
  }
  const deck = { deckText: '99 Plains', commanderText: '1 Isamaru, Hound of Konda' };
  await host.evaluate(([tournamentId, deck]) => window.__peerHarness.createLobby({
    name: 'ignored', transport: 'websocket', format: 'commander', desiredPlayers: 3, tournamentId, ...deck,
  }), [tournamentId, deck]);
  const hosted = await waitFor(host, s => s.multiplayer.mode === 'lobby' && s.multiplayer.lobbyId, 'host opens tournament lobby');
  for (const [index, page] of [second, third].entries()) {
    await page.evaluate(([lobbyId, deck]) => window.__peerHarness.joinLobby({ name: 'ignored', lobbyId, ...deck }), [hosted.multiplayer.lobbyId, deck]);
    await waitFor(host, s => s.multiplayer.players.length === index + 2, `seat ${index + 2} joins`);
  }
  await waitFor(host, s => s.canStartHostedMatch, 'all seats ready');
  await host.evaluate(() => window.__peerHarness.startHostedMatch());
  for (const page of env.pages) await waitFor(page, s => s.multiplayer.matchStarted, 'match starts');

  // Every seat accepted a genesis that carries one deal per seat, each sealed
  // to the two other seats and needing both of them.
  for (const page of env.pages) {
    const deals = (await lobby(page)).auditTranscript.match.ziffleKeyShares;
    assert.deepEqual(deals.map(deal => [deal.dealer, deal.threshold, deal.shares.map(share => share.holder)]),
      [[0, 2, [1, 2]], [1, 2, [0, 2]], [2, 2, [0, 1]]]);
  }

  // The third seat goes silent and is forfeited; the two remaining seats
  // exchange their shares of its key and rebuild it.
  await third.close();
  await host.evaluate(() => window.__peerHarness.openWitnessChallenge({ accusedSeat: 2, reason: 'disconnect' }));
  for (const page of [host, second]) {
    await waitFor(page, s => s.syncEvents.some(e => e.command?.reason === WITNESS_FORFEIT_REASON), 'witness forfeit applies', 60000);
    await waitFor(page, s => s.statusEvents.some(e => /Rebuilt Cara's hidden-card key/.test(e.message)), 'forfeited key rebuilt', 60000);
  }
  const verified = await host.evaluate(async () => {
    const { verifyLiveAuditTranscript } = await import('/src/lib/multiplayer-audit.js');
    const { auditTranscript } = await window.__peerHarness.snapshot();
    return (await verifyLiveAuditTranscript(auditTranscript, globalThis.crypto, { requireEngineReplay: false })).verifiedActions;
  });
  assert.ok(verified >= 1);
});
