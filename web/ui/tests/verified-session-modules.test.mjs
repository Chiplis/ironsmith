import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { webcrypto } from 'node:crypto';
import { createTrustedSequencer } from '../src/lib/verified-session/trusted-sequencer.js';
import {
  createEndOfMatchDisclosure,
  endOfMatchDisclosurePayload,
  DISCLOSURE_STATUS_CHEAT,
  DISCLOSURE_STATUS_VERIFIED,
  DISCLOSURE_STATUS_WITHHELD,
} from '../src/lib/verified-session/end-of-match-disclosure.js';
import {
  createAuditSessionKey, exportAuditPublicKey, importAuditPublicKey, signAuditPayload,
} from '../src/lib/multiplayer-audit.js';

// Everything under lib/verified-session must stay loadable by a host that has
// no React, DOM or PeerJS (Manabrew's adapter, node harnesses).
test('verified-session modules import no framework or hook-layer code', () => {
  const dir = new URL('../src/lib/verified-session/', import.meta.url);
  for (const file of readdirSync(dir)) {
    const source = readFileSync(new URL(file, dir), 'utf8');
    for (const [, specifier] of source.matchAll(/from\s+["']([^"']+)["']/g)) {
      // The session entry composes usePeerLobby; nothing else reaches the hook layer.
      const allowed = file === 'index.js' && specifier.endsWith('hooks/usePeerLobby.js');
      assert.ok(!/^(react|peerjs)$/.test(specifier) && (allowed || !specifier.includes('hooks/')), `${file} imports ${specifier}`);
    }
    assert.ok(!/\bwindow\./.test(source.replace(/\/\/.*$/gm, '')), `${file} touches window`);
  }
});

// A manual clock stands in for setInterval/setTimeout.
function manualClock() {
  const timers = new Map();
  let nextId = 1;
  return {
    setInterval: (fn) => { const id = nextId++; timers.set(id, { fn, once: false }); return id; },
    clearInterval: (id) => { timers.delete(id); },
    setTimeout: (fn) => { const id = nextId++; timers.set(id, { fn, once: true }); return id; },
    clearTimeout: (id) => { timers.delete(id); },
    tick: async () => {
      for (const [id, timer] of [...timers]) {
        if (timer.once) timers.delete(id);
        await timer.fn();
      }
    },
    size: () => timers.size,
  };
}

function sequencerFixture({ role }) {
  const sent = [];
  const diagnostics = [];
  const clock = manualClock();
  const hostConn = { peer: 'host', open: true };
  const guestConn = { peer: 'guest', open: true };
  const session = {
    role, matchStarted: true, securityMode: 'trusted', localPeerId: role === 'host' ? 'host' : 'guest',
    localPlayerIndex: role === 'host' ? 0 : 1, lastAppliedSequence: 0,
    players: [{ index: 0, peerId: 'host' }, { index: 1, peerId: 'guest' }],
  };
  const base = {
    multiplayerRef: { current: session },
    actionHistoryRef: { current: [] },
    matchStartPayloadRef: { current: { auditMatchId: 'm1', lobbyId: 'lobby', seed: 7 } },
    clientMessageQueueRef: { current: Promise.resolve() },
    hostConnectionRef: { current: hostConn },
    clientConnectionsRef: { current: new Map([['guest', guestConn]]) },
    gameRef: { current: { uiState: async () => ({}) } },
  };
  const resyncs = [];
  const servicesRef = { current: {
    buildMatchClockAuditForCommand: async () => ({ clock: true }),
    applySequencedActionMessage: async (message) => {
      session.lastAppliedSequence = message.seq;
      base.actionHistoryRef.current.push({ seq: message.seq, actorIndex: message.actorIndex,
        commandId: message.commandId, command: message.command, prefixHash: `h${message.seq}` });
    },
    requestResync: (reason) => resyncs.push(reason),
  } };
  const host = {
    send: (conn, payload) => { sent.push({ to: conn?.peer, payload }); return Boolean(conn?.open); },
    recordDiagnosticEvent: (name, payload) => diagnostics.push({ name, payload }),
    clock,
    randomUUID: () => 'uuid-1',
  };
  const sequencer = createTrustedSequencer(base, servicesRef, host);
  return { sequencer, sent, diagnostics, clock, session, base, resyncs };
}

test('trusted host accepts a local command, deduplicates a retry, and refuses stale or reused ids', async () => {
  const f = sequencerFixture({ role: 'host' });
  const { api } = f.sequencer;
  const entry = await api.submitTrustedIntent({ type: 'pass' }, 'pass');
  assert.equal(entry.seq, 1);
  assert.equal(entry.commandId, 'host:uuid-1');
  assert.deepEqual(f.diagnostics.map(d => d.name), ['trusted:host_accept']);

  // The same command id with the same content returns the recorded entry.
  const retried = await api.acceptTrustedCommand(null, {
    matchId: 'm1', commandId: entry.commandId, actorIndex: 0, expectedSequence: 0,
    command: { type: 'pass' }, label: 'pass',
  });
  assert.equal(retried, entry);
  await assert.rejects(api.acceptTrustedCommand(null, {
    matchId: 'm1', commandId: entry.commandId, actorIndex: 0, expectedSequence: 0,
    command: { type: 'other' },
  }), /reused with different content/);
  await assert.rejects(api.acceptTrustedCommand(null, {
    matchId: 'm1', commandId: 'host:fresh', actorIndex: 0, expectedSequence: 0, command: { type: 'pass' },
  }), /outdated prompt/);
  await assert.rejects(api.acceptTrustedCommand(null, {
    matchId: 'm1', commandId: 'host:wrong-seat', actorIndex: 1, expectedSequence: 1, command: { type: 'pass' },
  }), /seat does not match/);
});

test('trusted host redelivers an unacknowledged entry on the timer until the ack arrives', async () => {
  const f = sequencerFixture({ role: 'host' });
  const { api, start, stop } = f.sequencer;
  start();
  start(); // idempotent
  assert.equal(f.clock.size(), 1);
  const entry = await api.submitTrustedIntent({ type: 'pass' }, 'pass');
  api.publishTrustedAction({ seq: entry.seq });
  const published = f.sent.filter(m => m.payload.type === 'apply_action');
  assert.equal(published.length, 1);
  assert.equal(published[0].to, 'guest');
  assert.equal(published[0].payload.securityMode, 'trusted');

  await f.clock.tick();
  assert.equal(f.sent.filter(m => m.payload.type === 'apply_action').length, 2, 'redelivered');
  api.receiveTrustedAck({ peer: 'guest' }, { matchId: 'm1', seq: entry.seq, prefixHash: entry.prefixHash });
  await f.clock.tick();
  assert.equal(f.sent.filter(m => m.payload.type === 'apply_action').length, 2, 'acknowledged, no more retries');
  stop();
  assert.equal(f.clock.size(), 0);
});

test('trusted guest waits for host acceptance, resolves on the accepted entry, and rejects on stop', async () => {
  const f = sequencerFixture({ role: 'guest' });
  const { api, start, stop } = f.sequencer;
  start();
  const submitted = api.submitTrustedIntent({ type: 'pass' }, 'pass');
  assert.equal(f.sent[0].payload.type, 'trusted_command');
  assert.equal(f.sent[0].to, 'host');
  await assert.rejects(api.submitTrustedIntent({ type: 'pass' }, 'again'), /Waiting for host acceptance/);

  const commandId = f.sent[0].payload.commandId;
  const entry = { seq: 1, commandId, prefixHash: 'h1' };
  f.base.actionHistoryRef.current.push(entry);
  api.acknowledgeTrustedAction({ peer: 'host', open: true }, { seq: 1, prefixHash: 'h1' }, { trusted: true });
  assert.equal(await submitted, entry);
  assert.equal(f.sent.at(-1).payload.type, 'trusted_action_ack');

  const rejected = api.submitTrustedIntent({ type: 'pass' }, 'next');
  const rejectedId = f.sent.at(-1).payload.commandId;
  api.rejectTrustedIntent({ commandId: rejectedId, reason: 'stale' });
  await assert.rejects(rejected, /stale/);
  assert.equal(f.resyncs.length, 1);

  const dangling = api.submitTrustedIntent({ type: 'pass' }, 'last');
  stop();
  await assert.rejects(dangling, /Session closed/);
});

async function disclosureFixture({ playerCount = 2, localSeat = 0, game = {}, state = { game_over: true } } = {}) {
  const clock = manualClock();
  const notices = [];
  const perf = [];
  const statuses = [];
  const keys = [];
  for (let seat = 0; seat < playerCount; seat += 1) keys.push(await createAuditSessionKey(webcrypto));
  const publicKeys = [];
  for (const key of keys) publicKeys.push(await exportAuditPublicKey(key, webcrypto));
  const session = {
    matchStarted: true, securityMode: 'verified', localPeerId: `p${localSeat}`,
    players: keys.map((_, index) => ({ index, name: `P${index}`, peerId: `p${index}` })),
  };
  const sentDirect = [];
  const base = {
    gameRef: { current: { verifyEndOfMatchDisclosure: async () => ({ violations: [], missing: [] }),
      endOfMatchDisclosureRequirements: async () => [], endOfMatchDisclosureObligations: async () => ({ claimSubjects: 0, libraryAnchors: 0 }),
      ...game } },
    stateRef: { current: state },
    multiplayerRef: { current: session },
    setStatus: (text, isError) => statuses.push({ text, isError }),
  };
  const servicesRef = { current: {
    updateMultiplayer: (update) => { const next = update(session); if (next !== session) Object.assign(session, next); },
    currentAuditMatchId: () => 'm1',
    resolveLocalCryptoPlayerIndex: () => localSeat,
    publicKeyForAuditSigner: (seat) => publicKeys[seat],
    importCachedAuditPublicKey: (hex) => importAuditPublicKey(hex, webcrypto),
    verifyAuditOpeningsAgainstManifests: async () => {},
    ensureAuditIdentity: async () => ({ keyPair: keys[localSeat] }),
    buildLocalOpeningsForCommand: async () => [],
    buildLocalRequirementOpeningsForRequirements: async () => [],
    routePeerIdForPlayer: (player) => player.peerId,
    sendDirectPeerMessage: (peerId, message) => sentDirect.push({ peerId, message }),
  } };
  const host = {
    recordPerf: (label, payload) => perf.push({ label, payload }),
    notice: (title, body) => notices.push({ title, body }),
    mergeAuditOpenings: (...lists) => lists.flat(),
    clock,
  };
  const module = createEndOfMatchDisclosure(base, servicesRef, host);
  const signed = async (seat, fields = {}) => {
    const payload = endOfMatchDisclosurePayload({ matchId: 'm1', player: seat, openings: [] });
    return { ...payload, ...fields, signature: await signAuditPayload(keys[seat], { ...payload, ...fields }, webcrypto) };
  };
  return { module, session, notices, perf, statuses, sentDirect, clock, signed, base };
}

test('end-of-match disclosure: signs and broadcasts the local disclosure once, verifies a peer disclosure', async () => {
  const f = await disclosureFixture();
  const { api } = f.module;
  await api.onEndOfMatchRuntimeState({ game_over: true });
  await api.onEndOfMatchRuntimeState({ game_over: true });
  assert.equal(f.sentDirect.length, 1, 'sent once per match');
  assert.equal(f.sentDirect[0].peerId, 'p1');
  assert.equal(f.sentDirect[0].message.type, 'end_of_match_disclosure');
  assert.equal(f.session.endOfMatchDisclosure.byPlayer[0].status, DISCLOSURE_STATUS_VERIFIED);

  await api.handleEndOfMatchDisclosureMessage({ senderIndex: 1, disclosure: await f.signed(1) });
  assert.equal(f.session.endOfMatchDisclosure.byPlayer[1].status, DISCLOSURE_STATUS_VERIFIED);
  const exported = api.endOfMatchDisclosuresForExport('m1');
  assert.deepEqual(exported.map(entry => [entry.player, entry.verdict.status]), [[0, 'verified'], [1, 'verified']]);
  f.module.dispose();
});

test('end-of-match disclosure: a forged signature is a detected cheat and a final verdict is never downgraded', async () => {
  const f = await disclosureFixture();
  const { api } = f.module;
  const forged = await f.signed(1);
  forged.openings = [{ owner: 1, slot: 3 }]; // payload changed after signing
  await api.handleEndOfMatchDisclosureMessage({ senderIndex: 1, disclosure: forged });
  assert.equal(f.session.endOfMatchDisclosure.byPlayer[1].status, DISCLOSURE_STATUS_CHEAT);
  assert.match(f.session.endOfMatchDisclosure.byPlayer[1].reason, /signature is invalid/);
  assert.equal(f.notices[0].title, 'Cheat detected');
  assert.equal(f.statuses[0].isError, true);

  await api.onEndOfMatchRuntimeState({ game_over: true });
  await f.clock.tick(); // the missing-disclosure timer fires but cannot override the verdict
  assert.equal(f.session.endOfMatchDisclosure.byPlayer[1].status, DISCLOSURE_STATUS_CHEAT);
  f.module.dispose();
});

test('end-of-match disclosure: a silent player with pending claims is withheld, with none it is only missing', async () => {
  for (const [claims, expected] of [[2, DISCLOSURE_STATUS_WITHHELD], [0, 'missing']]) {
    const f = await disclosureFixture({ game: { endOfMatchDisclosureObligations: async () => ({ claimSubjects: claims, libraryAnchors: 0 }) } });
    await f.module.api.onEndOfMatchRuntimeState({ game_over: true });
    assert.equal(f.clock.size(), 1, 'timeout armed');
    await f.clock.tick();
    assert.equal(f.session.endOfMatchDisclosure.byPlayer[1].status, expected);
    f.module.dispose();
  }
});

test('end-of-match disclosure: dispose cancels the pending timeout', async () => {
  const f = await disclosureFixture();
  await f.module.api.onEndOfMatchRuntimeState({ game_over: true });
  assert.equal(f.clock.size(), 1);
  f.module.dispose();
  assert.equal(f.clock.size(), 0);
});
