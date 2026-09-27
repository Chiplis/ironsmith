// Tournament witness: invite redemption, genesis attestations and challenge/forfeit disputes.
// Game state never reaches the relay; only signed protocol payloads from witness-protocol.js.
import { canonicalJson, importAuditPublicKey, randomAuditHex, verifyAuditPayload } from '../../ui/src/lib/multiplayer-audit.js';
import {
  WITNESS_ANSWER_WINDOW_MS, WITNESS_MAX_ANSWER_BYTES, WITNESS_MAX_OPEN_CHALLENGES, WITNESS_MAX_REQUEST_BYTES,
  answerPayload, byteLength, certificatePayload, challengePayload, claimPayload, forfeitPayload, genesisAttestationPayload,
  payloadHash, redeemPayload, signWitnessPayload, statusRequestPayload, verifyGenesisRequest, verifyInvite, witnessPublicKeyFromPrivateJwk,
} from '../../ui/src/lib/tournament/witness-protocol.js';

const PEER = /^ws-([a-f0-9]{32})-([a-f0-9]{32})$/;
const KEY = /^[a-f0-9]{130}$/;
const REDEEM_SKEW = 10 * 60_000;
const CLAIM_SKEW = 2 * 60_000;
const STATUS_SKEW = 5 * 60_000;
// Protocol requests the accused must actually answer (not just point at
// another seat) for the witness to accept its answer.
const PROTOCOL_RESPONSES = {
  crypto_material_request: 'crypto_material_response',
  ziffle_reveal_token_request: 'ziffle_reveal_token_response',
  ziffle_shuffle_step_request: 'ziffle_shuffle_step_response',
  rng_commit_request: 'rng_commit_response',
  rng_reveal_request: 'rng_reveal_response',
};
const MAX_REDEEM_BODY = 16 * 1024;
const MAX_GENESIS = 4;
const MAX_CHALLENGES = 32;
const IDLE = 7 * 24 * 60 * 60 * 1000;
const json = (body, status = 200) => Response.json(body, { status });
const canonical = (build, value) => {
  try { return !!value && typeof value === 'object' && canonicalJson(build(value)) === canonicalJson(value); } catch { return false; }
};
const post = (stub, path, body) => stub.fetch(`https://internal/${path}`, { method: 'POST', body: JSON.stringify(body) });

let cached;
export async function witnessKey(env) {
  const secret = env.WITNESS_SIGNING_KEY;
  if (!secret) throw new Error('Witness is not configured');
  if (cached?.secret !== secret) cached = { secret, key: (async () => {
    const jwk = JSON.parse(secret);
    const privateKey = await crypto.subtle.importKey('jwk', { kty: jwk.kty, crv: jwk.crv, x: jwk.x, y: jwk.y, d: jwk.d },
      { name: 'ECDSA', namedCurve: 'P-256' }, false, ['sign']);
    return { keyPair: { privateKey }, publicKey: await witnessPublicKeyFromPrivateJwk(jwk) };
  })() };
  try { return await cached.key; } catch { cached = null; throw new Error('Witness is not configured'); }
}
const sign = async (env, payload) => signWitnessPayload((await witnessKey(env)).keyPair, payload);
async function verifySignature(publicKey, payload, signature) {
  if (!KEY.test(publicKey || '') || typeof signature !== 'string') return false;
  try { return await verifyAuditPayload(await importAuditPublicKey(publicKey), payload, signature); } catch { return false; }
}

export async function witnessKeyResponse(env) {
  try { return json({ publicKey: (await witnessKey(env)).publicKey }); } catch (error) { return json({ error: error.message }, 503); }
}

// Body is JSON sent as text/plain (no CORS preflight): {invite, redeem, redeemSignature}.
export async function redeemInvite(request, env) {
  let key;
  try { key = await witnessKey(env); } catch (error) { return json({ error: error.message }, 503); }
  const text = await request.text();
  if (text.length > MAX_REDEEM_BODY) return json({ error: 'Request too large' }, 413);
  const now = Date.now();
  let invite, tournamentId, redeem;
  try {
    const body = JSON.parse(text);
    ({ invite, tournamentId } = await verifyInvite(body?.invite, now));
    redeem = body.redeem;
    if (!canonical(redeemPayload, redeem)) throw new Error('Redeem request is not canonical');
    if (redeem.tournamentId !== tournamentId || redeem.inviteId !== invite.inviteId) throw new Error('Redeem request does not match the invite');
    if (!(Math.abs(redeem.requestedAt - now) <= REDEEM_SKEW)) throw new Error('Redeem request is stale');
    if (!await verifySignature(redeem.auditPublicKey, redeem, body.redeemSignature)) throw new Error('Redeem signature is invalid');
  } catch (error) { return json({ error: error instanceof SyntaxError ? 'Invalid JSON' : error.message }, 400); }
  const registry = env.TOURNAMENTS.get(env.TOURNAMENTS.idFromName(tournamentId));
  const bound = await post(registry, 'bind', { inviteId: invite.inviteId, auditPublicKey: redeem.auditPublicKey, expiresAt: invite.expiresAt });
  if (!bound.ok) return json(await bound.json(), bound.status);
  const certificate = await signWitnessPayload(key.keyPair, certificatePayload({ tournamentId, tournament: invite.tournament,
    inviteId: invite.inviteId, playerName: invite.playerName, auditPublicKey: redeem.auditPublicKey, issuedAt: now, expiresAt: invite.expiresAt }));
  return json({ certificate, witnessPublicKey: key.publicKey });
}

// One per tournament id; binds invite <-> audit key. Reachable only through the TOURNAMENTS binding.
export class TournamentRegistry {
  constructor(ctx) { this.ctx = ctx; }
  async fetch(request) {
    const { inviteId, auditPublicKey, expiresAt } = await request.json();
    const storage = this.ctx.storage;
    const holder = await storage.get(`invite:${inviteId}`);
    if (holder && holder !== auditPublicKey) return json({ error: 'Invite already redeemed' }, 409);
    const held = await storage.get(`key:${auditPublicKey}`);
    if (held && held !== inviteId) return json({ error: 'This browser already holds an invite for this tournament' }, 409);
    if (!holder) await storage.put({ [`invite:${inviteId}`]: auditPublicKey, [`key:${auditPublicKey}`]: inviteId });
    // Bindings are useless once every redeemed invite (and so every certificate) has expired.
    const alarm = await storage.getAlarm();
    if (!alarm || alarm < expiresAt) await storage.setAlarm(expiresAt);
    return json({ ok: true });
  }
  async alarm() { await this.ctx.storage.deleteAll(); }
}

const view = r => ({ challenge: r.challenge, status: r.status, claim: r.claim, claimSignature: r.claimSignature,
  ...(r.answer ? { answer: r.answer } : {}), ...(r.forfeit ? { forfeit: r.forfeit } : {}) });
const claimantOf = r => r.players[r.claim.claimantSeat].peerId;
const accusedOf = r => r.players[r.claim.accusedSeat];
const delivery = r => ({ to: claimantOf(r), message: { type: 'witness_event', event: 'forfeit', challenge: r.challenge, forfeit: r.forfeit } });

// One per room+match; owns challenge state and the forfeit alarm. Reachable only through the DISPUTES binding.
export class WitnessDisputes {
  constructor(ctx, env) { this.ctx = ctx; this.env = env; this.tail = Promise.resolve(); }
  // Serialize state changes across crypto awaits (which reopen the input gate) without blockConcurrencyWhile.
  lock(fn) { const run = this.tail.then(fn); this.tail = run.catch(() => {}); return run; }
  async records() { return [...(await this.ctx.storage.list({ prefix: 'challenge:' })).values()]; }
  async save(record) { record.updatedAt = Date.now(); await this.ctx.storage.put(`challenge:${record.challengeId}`, record); }
  async arm(records) {
    const open = records.filter(r => r.status === 'open').map(r => r.challenge.payload.deadline);
    await this.ctx.storage.setAlarm(open.length ? Math.min(...open)
      : Math.max(0, ...records.map(r => r.updatedAt)) + IDLE);
  }
  // Forfeits every overdue open challenge; the same rule serves the alarm and lazy reads.
  async decide(records, now = Date.now()) {
    const decided = [];
    for (const r of records) if (r.status === 'open' && now >= r.challenge.payload.deadline) {
      r.status = 'forfeited';
      r.forfeit = await sign(this.env, forfeitPayload(r.challenge.payload, now));
      await this.save(r); decided.push(r);
    }
    if (decided.length) await this.arm(records);
    return decided;
  }
  async fetch(request) {
    const path = new URL(request.url).pathname;
    const body = await request.json();
    try {
      return json(await this.lock(async () => {
        const now = Date.now(); const records = await this.records();
        const decided = (await this.decide(records, now)).map(delivery);
        if (path === '/status') return { decided,
          challenges: records.filter(r => body.seat != null
            ? r.claim.claimantSeat === body.seat || r.claim.accusedSeat === body.seat
            : claimantOf(r) === body.peer || accusedOf(r).peerId === body.peer).map(view) };
        if (path === '/open') return { decided, ...await this.open(records, body, now) };
        if (path === '/answer') return { decided, ...await this.answer(records, body) };
        throw new Error('Not found');
      }));
    } catch (error) { return json({ error: error.message }, 400); }
  }
  async open(records, { roomId, claim, claimSignature, players, genesisHash }, now) {
    const open = records.filter(r => r.status === 'open');
    const same = open.find(r => r.claim.claimantSeat === claim.claimantSeat && r.claim.accusedSeat === claim.accusedSeat);
    if (same) return { record: view(same) };
    if (open.length >= WITNESS_MAX_OPEN_CHALLENGES) throw new Error('Too many open challenges for this match');
    if (records.length >= MAX_CHALLENGES) throw new Error('Challenge limit reached for this match');
    const window = Number(this.env.WITNESS_ANSWER_WINDOW_MS) || WITNESS_ANSWER_WINDOW_MS;
    const challengeId = randomAuditHex(16); const claimHash = await payloadHash(claim);
    const challenge = await sign(this.env, challengePayload({ challengeId, claim, claimHash, openedAt: now, deadline: now + window, genesisHash }));
    const record = { challengeId, roomId, players, claim, claimSignature, claimHash, genesisHash, challenge, status: 'open' };
    await this.save(record); records.push(record); await this.arm(records);
    return { record: view(record) };
  }
  async answer(records, { answer, answerSignature, peer }) {
    const r = records.find(x => x.challengeId === answer?.challengeId);
    if (!r) throw new Error('Unknown challenge');
    if (r.status !== 'open') throw new Error(`Challenge is already ${r.status}`);
    if (!canonical(answerPayload, answer)) throw new Error('Answer is not canonical');
    if (answer.matchId !== r.claim.matchId || answer.accusedSeat !== r.claim.accusedSeat) throw new Error('Answer does not match the challenge');
    const accused = accusedOf(r);
    if (byteLength(answer) > WITNESS_MAX_ANSWER_BYTES) throw new Error('Answer is too large');
    // The seat's attested signing key authenticates the answer; the socket's
    // peer id is not required to match (a victim whose route changed can
    // still answer for its seat).
    if (!await verifySignature(accused.auditPublicKey, answer, answerSignature)) {
      throw new Error(peer === accused.peerId ? 'Answer signature is invalid' : 'Only the accused seat may answer: answer signature is invalid');
    }
    // A challenge over an unanswered protocol request is only answered by
    // the response itself (possibly an error), not by pointing at another
    // seat, unless the accused claims the game has since moved on.
    const request = r.claim.request;
    const responseType = request && typeof request === 'object' ? PROTOCOL_RESPONSES[request.type] : null;
    if (responseType && !(answer.headSequence > r.claim.basisSequence) && !answer.responses.some(response =>
      response && response.type === responseType && response.requestId === request.requestId)) {
      throw new Error('Answer does not include the outstanding protocol response');
    }
    r.status = 'answered'; r.answer = { payload: answer, signature: answerSignature };
    await this.save(r); await this.arm(records);
    return { record: view(r), claimant: claimantOf(r) };
  }
  async alarm() {
    const { decided, idle } = await this.lock(async () => {
      const records = await this.records(); const decided = await this.decide(records);
      const idle = !records.some(r => r.status === 'open') && Date.now() >= Math.max(0, ...records.map(r => r.updatedAt)) + IDLE;
      if (idle) await this.ctx.storage.deleteAll(); else if (!decided.length) await this.arm(records);
      return { decided, idle };
    });
    if (idle) return;
    await Promise.all(decided.map(r => post(this.env.ROOMS.get(this.env.ROOMS.idFromName(r.roomId)), 'deliver', delivery(r))
      .catch(() => {})));
  }
}

async function genesis(lobby, state, config, body) {
  if (state.peer !== config.host) throw new Error('Only the host may request a genesis attestation');
  const { keyPair, publicKey } = await witnessKey(lobby.env);
  const certificates = await verifyGenesisRequest(body, publicKey);
  const { request } = body;
  if (request.tournamentId !== config.tournamentId) throw new Error('Genesis request is for another tournament');
  if (request.format !== config.format) throw new Error('Genesis request is for another format');
  if (request.lobbyId !== config.host) throw new Error('Genesis request is for another lobby');
  if (!request.matchId || request.matchId.length > 128) throw new Error('Genesis request match id is invalid');
  if (request.players.some(p => p.peerId.match(PEER)?.[1] !== state.room)) throw new Error('Genesis request seats a peer outside this room');
  if (new Set(request.players.map(p => p.peerId)).size !== request.players.length) throw new Error('A peer occupies two seats');
  if (request.players[request.hostSeat]?.peerId !== config.host) throw new Error('Genesis request host seat is not the lobby host');
  const storage = lobby.ctx.storage;
  // The first attestation for a match id is immutable: a host cannot later
  // re-seat a victim (e.g. behind a sock-puppet peer id) or swap the genesis.
  // An identical re-request is idempotent and returns the stored attestation.
  const lockKey = `witness:genesis-lock:${request.matchId}`;
  const locked = await storage.get(lockKey);
  const fingerprint = await payloadHash(genesisAttestationPayload(request, certificates, 0));
  if (locked && locked !== fingerprint) throw new Error('A different genesis is already attested for this match id');
  const existing = locked && await storage.get(`witness:genesis:${request.matchId}`);
  if (existing) return { attestation: existing, witnessPublicKey: publicKey };
  const attestation = await signWitnessPayload(keyPair, genesisAttestationPayload(request, certificates, Date.now()));
  if (!locked) await storage.put(lockKey, fingerprint);
  await storage.put(`witness:genesis:${request.matchId}`, attestation);
  const stored = [...await storage.list({ prefix: 'witness:genesis:' })].sort(([, a], [, b]) => b.payload.issuedAt - a.payload.issuedAt);
  if (stored.length > MAX_GENESIS) await storage.delete(stored.slice(MAX_GENESIS).map(([key]) => key));
  return { attestation, witnessPublicKey: publicKey };
}

async function challenge(lobby, state, config, { claim, claimSignature }) {
  if (!canonical(claimPayload, claim)) throw new Error('Claim is not canonical');
  const attestation = typeof claim.matchId === 'string' && claim.matchId.length <= 128
    && await lobby.ctx.storage.get(`witness:genesis:${claim.matchId}`);
  if (!attestation) throw new Error('Unknown match');
  if (claim.tournamentId !== config.tournamentId) throw new Error('Claim is for another tournament');
  const players = attestation.payload.players.map(({ seat, peerId, auditPublicKey }) => ({ seat, peerId, auditPublicKey }));
  const claimant = players[claim.claimantSeat], accused = players[claim.accusedSeat];
  if (!claimant || !accused || claimant === accused) throw new Error('Claim seats are invalid');
  if (state.peer !== claimant.peerId) throw new Error('Only the claimant seat may open this challenge');
  if (!(Math.abs(claim.claimedAt - Date.now()) <= CLAIM_SKEW)) throw new Error('Claim is stale');
  if (byteLength(claim.request) > WITNESS_MAX_REQUEST_BYTES) throw new Error('Claim request is too large');
  if (!await verifySignature(claimant.auditPublicKey, claim, claimSignature)) throw new Error('Claim signature is invalid');
  await witnessKey(lobby.env);
  const genesisHash = String(attestation.payload.genesisHash || '');
  const out = await disputes(lobby, state, claim.matchId, genesisHash, 'open', { roomId: state.room, claim, claimSignature, players, genesisHash });
  lobby.deliver(accused.peerId, { type: 'witness_event', event: 'challenge', challenge: out.record.challenge, claim, claimSignature });
  return { challenge: out.record.challenge };
}

// Dispute state is keyed by the attested genesis too, so challenges and
// forfeits of one game can never surface in another that shares a match id.
async function disputes(lobby, state, matchId, genesisHash, path, body) {
  const env = lobby.env;
  const response = await post(env.DISPUTES.get(env.DISPUTES.idFromName(`${state.room}:${matchId}:${genesisHash}`)), path, body);
  const out = await response.json();
  if (!response.ok) throw new Error(out.error || 'Witness request failed');
  for (const { to, message } of out.decided) lobby.deliver(to, message);
  return out;
}

// Handles {type:'witness', id, op, body} for an authenticated socket of a tournament room.
export async function witnessOp(lobby, state, msg) {
  const config = await lobby.ctx.storage.get('config');
  if (!config?.tournamentId) throw new Error('Witness is only available in tournament rooms');
  const body = msg.body && typeof msg.body === 'object' ? msg.body : {};
  if (msg.op === 'genesis') return genesis(lobby, state, config, body);
  if (msg.op === 'challenge') return challenge(lobby, state, config, body);
  const matchId = msg.op === 'answer' ? body.answer?.matchId : body.matchId;
  if (typeof matchId !== 'string' || !matchId || matchId.length > 128) throw new Error('Unknown match');
  const attestation = await lobby.ctx.storage.get(`witness:genesis:${matchId}`);
  if (!attestation) throw new Error('Unknown match');
  const genesisHash = String(attestation.payload.genesisHash || '');
  if (msg.op === 'answer') {
    await witnessKey(lobby.env);
    const out = await disputes(lobby, state, matchId, genesisHash, 'answer', { answer: body.answer, answerSignature: body.answerSignature, peer: state.peer });
    lobby.deliver(out.claimant, { type: 'witness_event', event: 'answer', challenge: out.record.challenge, answer: out.record.answer });
    return { status: 'answered' };
  }
  if (msg.op === 'status') {
    // A seat-signed status request lists that seat's challenges whatever
    // peer id it connects from; otherwise fall back to the socket's peer id.
    let seat = null;
    if (body.statusRequest) {
      const request = body.statusRequest;
      if (!canonical(statusRequestPayload, request) || request.matchId !== matchId) throw new Error('Status request is not canonical');
      if (!(Math.abs(request.requestedAt - Date.now()) <= STATUS_SKEW)) throw new Error('Status request is stale');
      const player = attestation.payload.players[request.seat];
      if (!player || !await verifySignature(player.auditPublicKey, request, body.statusSignature)) throw new Error('Status request signature is invalid');
      seat = request.seat;
    }
    return (await disputes(lobby, state, matchId, genesisHash, 'status', { peer: state.peer, seat })).challenges;
  }
  throw new Error('Unknown witness op');
}
