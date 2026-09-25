import test from 'node:test';
import assert from 'node:assert/strict';
import { startRelay } from './runtime.mjs';
import { createAuditSessionKey, exportAuditPublicKey, randomAuditHex, signAuditPayload } from '../../ui/src/lib/multiplayer-audit.js';
import {
  answerPayload, claimPayload, genesisRequestPayload, invitePayload, redeemPayload, signWitnessPayload, tournamentDescriptor,
  tournamentIdForDescriptor, verifyCertificate, verifyForfeitCertificate, verifyGenesisAttestation,
} from '../../ui/src/lib/tournament/witness-protocol.js';

const origin = 'http://localhost:5173';
const WINDOW = 600;
const sleep = ms => new Promise(r => setTimeout(r, ms));
const witnessJwk = await crypto.subtle.exportKey('jwk', (await createAuditSessionKey()).privateKey);
const witnessEnv = { WITNESS_SIGNING_KEY: JSON.stringify(witnessJwk), WITNESS_ANSWER_WINDOW_MS: String(WINDOW) };
const player = async name => { const keyPair = await createAuditSessionKey(); return { name, keyPair, publicKey: await exportAuditPublicKey(keyPair) }; };

async function tournament(name = 'Friday Night') {
  const organizer = await player('organizer');
  const descriptor = tournamentDescriptor({ name, organizerPublicKey: organizer.publicKey, createdAt: Date.now(), nonce: randomAuditHex(16) });
  const id = await tournamentIdForDescriptor(descriptor);
  const invite = (playerName, expiresAt = Date.now() + 3600_000) => signWitnessPayload(organizer.keyPair,
    invitePayload({ tournament: descriptor, inviteId: randomAuditHex(16), playerName, expiresAt }));
  return { organizer, descriptor, id, invite };
}
async function redeemBody(t, invite, p, signer = p) {
  const redeem = redeemPayload({ tournamentId: t.id, inviteId: invite.payload.inviteId, auditPublicKey: p.publicKey, requestedAt: Date.now() });
  return { invite, redeem, redeemSignature: await signAuditPayload(signer.keyPair, redeem) };
}
const postRedeem = (mf, body, from = origin) => mf.dispatchFetch('http://localhost/witness/redeem', {
  method: 'POST', headers: { Origin: from, 'Content-Type': 'text/plain' }, body: JSON.stringify(body) });
async function certify(mf, t, p) {
  const response = await postRedeem(mf, await redeemBody(t, await t.invite(p.name), p));
  assert.equal(response.status, 200);
  return (await response.json()).certificate;
}

function inbox(socket) {
  const queue = []; const waits = [];
  socket.addEventListener('message', event => {
    const value = JSON.parse(event.data);
    const index = waits.findIndex(w => w.match(value));
    if (index >= 0) waits.splice(index, 1)[0].resolve(value); else queue.push(value);
  });
  return (match = () => true, ms = 5000) => {
    const index = queue.findIndex(match);
    if (index >= 0) return Promise.resolve(queue.splice(index, 1)[0]);
    return new Promise((resolve, reject) => {
      const waiter = { match, resolve: v => { clearTimeout(timer); resolve(v); } };
      const timer = setTimeout(() => { waits.splice(waits.indexOf(waiter), 1); reject(new Error('Timed out waiting for relay')); }, ms);
      waits.push(waiter);
    });
  };
}
async function connect(mf, room, peer, auth) {
  const response = await mf.dispatchFetch(`http://localhost/rooms/${room}/socket?peer=${peer}`, { headers: { Origin: origin, Upgrade: 'websocket' } });
  assert.equal(response.status, 101);
  const socket = response.webSocket; socket.accept(); const next = inbox(socket);
  socket.send(JSON.stringify({ type: 'auth', token: randomAuditHex(16), format: 'modern', desiredPlayers: 2, ...auth }));
  const client = { peer, socket, next, first: await next() };
  client.witness = async (op, body) => {
    const id = randomAuditHex(8);
    socket.send(JSON.stringify({ type: 'witness', id, op, body }));
    return next(m => m.type === 'witness_result' && m.id === id);
  };
  client.event = (event, ms) => next(m => m.type === 'witness_event' && m.event === event, ms);
  return client;
}
const peerOf = room => `ws-${room}-${randomAuditHex(16)}`;

// Tournament room with host (seat 0) and guest (seat 1), both certified, plus a genesis attestation.
async function match(mf, t) {
  const room = randomAuditHex(16);
  const host = await connect(mf, room, peerOf(room), { securityMode: 'verified', tournamentId: t.id });
  const guest = await connect(mf, room, peerOf(room));
  host.player = await player('Alice'); guest.player = await player('Bob');
  const certificates = [await certify(mf, t, host.player), await certify(mf, t, guest.player)];
  const request = genesisRequestPayload({ genesisHash: randomAuditHex(32), matchId: randomAuditHex(16), lobbyId: host.peer,
    tournamentId: t.id, format: 'modern', securityMode: 'verified', hostSeat: 0,
    players: [host, guest].map((c, seat) => ({ seat, peerId: c.peer, name: c.player.name, auditPublicKey: c.player.publicKey })) });
  const body = { request, hostSignature: await signAuditPayload(host.player.keyPair, request), certificates };
  return { room, host, guest, request, body };
}
async function claim(m, t, { claimant = m.guest, signer = claimant, claimantSeat = 1, accusedSeat = 0 } = {}) {
  const payload = claimPayload({ matchId: m.request.matchId, tournamentId: t.id, claimantSeat, accusedSeat, reason: 'unanswered_request',
    basisSequence: 3, headStateHash: randomAuditHex(32), request: { kind: 'choice' }, claimedAt: Date.now() });
  return claimant.witness('challenge', { claim: payload, claimSignature: await signAuditPayload(signer.player.keyPair, payload) });
}

test('rtc frames are forwarded, trusted rooms keep old auth, tournament rooms stay unlisted', async t => {
  const mf = await startRelay(origin, witnessEnv); t.after(() => mf.dispose());
  const trusted = randomAuditHex(16);
  const a = await connect(mf, trusted, peerOf(trusted)); const b = await connect(mf, trusted, peerOf(trusted));
  assert.deepEqual(a.first.config, { host: a.peer, format: 'modern', desiredPlayers: 2 });
  const connectionId = randomAuditHex(16); const signal = { type: 'offer', sdp: 'v=0' };
  b.socket.send(JSON.stringify({ type: 'rtc', to: a.peer, from: 'spoofed', connectionId, signal }));
  assert.deepEqual(await a.next(), { type: 'rtc', from: b.peer, connectionId, signal });
  b.socket.send(JSON.stringify({ type: 'rtc', to: peerOf(randomAuditHex(16)), connectionId, signal }));
  assert.equal((await b.next()).type, 'unavailable');
  assert.match((await a.witness('status', { matchId: 'x' })).error, /tournament rooms/);
  a.socket.send(JSON.stringify({ type: 'rtc', to: b.peer, connectionId, signal: ['nope'] }));
  assert.equal((await a.next()).message, 'Invalid frame');

  const tour = await tournament();
  const room = randomAuditHex(16);
  const bad = await connect(mf, room, peerOf(room), { securityMode: 'verified' });
  assert.match(bad.first.message, /tournament room/);
  const host = await connect(mf, room, peerOf(room), { securityMode: 'verified', tournamentId: tour.id });
  assert.deepEqual(host.first.config, { host: host.peer, format: 'modern', desiredPlayers: 2, securityMode: 'verified', tournamentId: tour.id });
  const guest = await connect(mf, room, peerOf(room));
  host.socket.send(JSON.stringify({ type: 'advertise', lobby: { name: 'Hidden', available: true, playerCount: 1 } }));
  guest.socket.send(JSON.stringify({ type: 'rtc', to: host.peer, connectionId, signal: { kind: 'switch' } }));
  assert.deepEqual(await host.next(), { type: 'rtc', from: guest.peer, connectionId, signal: { kind: 'switch' } });
  const listing = await (await mf.dispatchFetch('http://localhost/lobbies', { headers: { Origin: origin } })).json();
  assert.equal(listing.lobbies.length, 0);
});

test('invite redemption binds one invite to one key per tournament', async t => {
  const mf = await startRelay(origin, witnessEnv); t.after(() => mf.dispose());
  const { publicKey } = await (await mf.dispatchFetch('http://localhost/witness/key', { headers: { Origin: origin } })).json();
  assert.match(publicKey, /^[a-f0-9]{130}$/);
  const options = await mf.dispatchFetch('http://localhost/witness/redeem', { method: 'OPTIONS', headers: { Origin: origin } });
  assert.equal(options.headers.get('Access-Control-Allow-Methods'), 'GET, POST, OPTIONS');
  assert.equal((await mf.dispatchFetch('http://localhost/lobbies', { method: 'POST', headers: { Origin: origin } })).status, 405);
  const tour = await tournament(); const alice = await player('Alice'); const mallory = await player('Mallory');
  const invite = await tour.invite('Alice');
  const body = await redeemBody(tour, invite, alice);
  const response = await postRedeem(mf, body);
  assert.equal(response.status, 200); assert.equal(response.headers.get('Access-Control-Allow-Origin'), origin);
  const out = await response.json();
  assert.equal(out.witnessPublicKey, publicKey);
  const cert = await verifyCertificate(out.certificate, publicKey, { tournamentId: tour.id, auditPublicKey: alice.publicKey });
  assert.equal(cert.playerName, 'Alice'); assert.equal(cert.inviteId, invite.payload.inviteId);
  assert.equal((await postRedeem(mf, await redeemBody(tour, invite, alice))).status, 200);
  const stolen = await postRedeem(mf, await redeemBody(tour, invite, mallory));
  assert.equal(stolen.status, 409); assert.equal((await stolen.json()).error, 'Invite already redeemed');
  const second = await postRedeem(mf, await redeemBody(tour, await tour.invite('Alice 2'), alice));
  assert.equal(second.status, 409); assert.match((await second.json()).error, /already holds an invite/);
  const other = await tournament('Other');
  assert.equal((await postRedeem(mf, await redeemBody(other, await other.invite('Alice'), alice))).status, 200);

  const fresh = await tour.invite('Carol'); const carol = await player('Carol');
  const tampered = { ...fresh, payload: { ...fresh.payload, playerName: 'Mallory' } };
  assert.equal((await postRedeem(mf, await redeemBody(tour, tampered, carol))).status, 400);
  assert.equal((await postRedeem(mf, await redeemBody(tour, await tour.invite('Late', Date.now() - 1), carol))).status, 400);
  assert.equal((await postRedeem(mf, await redeemBody(tour, fresh, carol, mallory))).status, 400);
  const stale = await redeemBody(tour, fresh, carol);
  stale.redeem = { ...stale.redeem, requestedAt: Date.now() - 11 * 60_000 };
  stale.redeemSignature = await signAuditPayload(carol.keyPair, stale.redeem);
  assert.equal((await postRedeem(mf, stale)).status, 400);
  assert.equal((await postRedeem(mf, await redeemBody(tour, fresh, carol), 'https://evil.example')).status, 403);
  assert.equal((await postRedeem(mf, await redeemBody(tour, fresh, carol))).status, 200);

  const bare = await startRelay(origin); t.after(() => bare.dispose());
  assert.equal((await bare.dispatchFetch('http://localhost/witness/key', { headers: { Origin: origin } })).status, 503);
  const unconfigured = await postRedeem(bare, await redeemBody(tour, await tour.invite('Dan'), await player('Dan')));
  assert.equal(unconfigured.status, 503); assert.equal((await unconfigured.json()).error, 'Witness is not configured');
});

test('genesis attestation is issued only to the host for certified seats of this room', async t => {
  const mf = await startRelay(origin, witnessEnv); t.after(() => mf.dispose());
  const tour = await tournament(); const m = await match(mf, tour);
  const denied = await m.guest.witness('genesis', m.body);
  assert.equal(denied.ok, false); assert.match(denied.error, /Only the host/);
  const result = await m.host.witness('genesis', m.body);
  assert.equal(result.ok, true, result.error);
  const attestation = await verifyGenesisAttestation(result.result.attestation, result.result.witnessPublicKey,
    { matchId: m.request.matchId, tournamentId: tour.id, lobbyId: m.host.peer, players: m.request.players });
  assert.equal(attestation.players.length, 2); assert.match(attestation.players[1].inviteId, /^[a-f0-9]{32}$/);

  const other = await tournament('Other');
  const foreign = [m.body.certificates[0], await certify(mf, other, m.guest.player)];
  assert.match((await m.host.witness('genesis', { ...m.body, certificates: foreign })).error, /another tournament/);
  const outsider = genesisRequestPayload({ ...m.request, players: m.request.players.map((p, i) => i ? { ...p, peerId: peerOf(randomAuditHex(16)) } : p) });
  const outsiderBody = { ...m.body, request: outsider, hostSignature: await signAuditPayload(m.host.player.keyPair, outsider) };
  assert.match((await m.host.witness('genesis', outsiderBody)).error, /outside this room/);
});

test('an unanswered challenge becomes a signed forfeit after the window', async t => {
  const mf = await startRelay(origin, witnessEnv); t.after(() => mf.dispose());
  const tour = await tournament(); const m = await match(mf, tour);
  const { result: { witnessPublicKey } } = await m.host.witness('genesis', m.body);
  const opened = await claim(m, tour);
  assert.equal(opened.ok, true, opened.error);
  const pushed = await m.host.event('challenge');
  assert.deepEqual(pushed.challenge, opened.result.challenge);
  assert.equal((await claim(m, tour)).result.challenge.payload.challengeId, opened.result.challenge.payload.challengeId);
  // The storage alarm decides the forfeit and pushes it through the room.
  const event = await m.guest.event('forfeit', WINDOW + 5000);
  const forfeit = await verifyForfeitCertificate(event.forfeit, witnessPublicKey,
    { matchId: m.request.matchId, tournamentId: tour.id, claimantSeat: 1, accusedSeat: 0 });
  assert.equal(forfeit.challengeId, opened.result.challenge.payload.challengeId);
  const status = await m.host.witness('status', { matchId: m.request.matchId });
  assert.equal(status.result.length, 1); assert.equal(status.result[0].status, 'forfeited');
  assert.deepEqual(status.result[0].forfeit, event.forfeit);
});

test('a challenge answered in time is forwarded and never forfeits', async t => {
  const mf = await startRelay(origin, witnessEnv); t.after(() => mf.dispose());
  const tour = await tournament(); const m = await match(mf, tour);
  await m.host.witness('genesis', m.body);
  assert.match((await claim(m, tour, { signer: m.host })).error, /Claim signature is invalid/);
  assert.match((await claim(m, tour, { claimant: m.host })).error, /Only the claimant/);
  const opened = await claim(m, tour);
  const challenge = opened.result.challenge.payload;
  const answer = answerPayload({ challengeId: challenge.challengeId, matchId: challenge.matchId, accusedSeat: 0, headSequence: 3,
    headStateHash: challenge.headStateHash, awaitingSeat: 1, responses: [{ type: 'choice' }], actions: [], answeredAt: Date.now() });
  const byGuest = await m.guest.witness('answer', { answer, answerSignature: await signAuditPayload(m.guest.player.keyPair, answer) });
  assert.match(byGuest.error, /Only the accused/);
  assert.match((await m.host.witness('answer', { answer, answerSignature: await signAuditPayload(m.guest.player.keyPair, answer) })).error,
    /signature is invalid/);
  const answered = await m.host.witness('answer', { answer, answerSignature: await signAuditPayload(m.host.player.keyPair, answer) });
  assert.deepEqual(answered.result, { status: 'answered' });
  const event = await m.guest.event('answer');
  assert.deepEqual(event.answer.payload, answer); assert.deepEqual(event.challenge, opened.result.challenge);
  await sleep(WINDOW + 300);
  await assert.rejects(m.guest.event('forfeit', 300));
  const status = await m.guest.witness('status', { matchId: m.request.matchId });
  assert.equal(status.result[0].status, 'answered'); assert.equal(status.result[0].forfeit, undefined);
});
