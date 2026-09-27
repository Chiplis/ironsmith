// Genesis binding for Verified matches: seed commit-reveal, the frozen
// genesis roster, one-shot match starts, and cross-peer genesis acks.
//
// State here is module-scoped (one browser tab == one seat), so it survives
// hook re-renders without adding refs to the shared lobby base.
import {
  buildSignedMatchGenesisAck,
  deriveMatchSeedFromReveals,
  matchSeedCommitment,
  randomAuditHex,
  verifySignedMatchGenesisAck,
} from "../../lib/multiplayer-audit.js";

const HEX_64 = /^[0-9a-f]{64}$/;
export const MATCH_SEED_REVEAL_TIMEOUT_MS = 30000;
const GENESIS_ACK_MISMATCH_GRACE_MS = 20000;

// ---------------------------------------------------------------------------
// Seed commit-reveal
// ---------------------------------------------------------------------------

// commitment -> { nonce, matchId, seat, hostCommitment }
const seedNonceByCommitment = new Map();
// `${matchId}:${seat}` -> commitment currently used for new genesis signatures
const currentSeedCommitmentBySeat = new Map();

function seatKey(matchId, seat) {
  return `${String(matchId || "")}:${Number(seat)}`;
}

// Best-effort persistence so a reload between readying and match start does
// not strand a signed commitment whose nonce this tab forgot.
const SEED_STORAGE_PREFIX = "ironsmith-match-seed-nonce:";

function storeSeedRecord(commitment, record) {
  try {
    globalThis.localStorage?.setItem(`${SEED_STORAGE_PREFIX}${commitment}`, JSON.stringify(record));
  } catch {
    // Storage may be unavailable; the in-memory copy still works.
  }
}

function seedRecordForCommitment(commitment) {
  const key = String(commitment || "");
  if (!key) return null;
  const cached = seedNonceByCommitment.get(key);
  if (cached) return cached;
  try {
    const raw = globalThis.localStorage?.getItem(`${SEED_STORAGE_PREFIX}${key}`);
    const parsed = raw ? JSON.parse(raw) : null;
    if (parsed && HEX_64.test(String(parsed.nonce || ""))) {
      const record = {
        nonce: String(parsed.nonce),
        matchId: String(parsed.matchId || ""),
        seat: Number(parsed.seat),
        hostCommitment: parsed.hostCommitment ? String(parsed.hostCommitment) : null,
      };
      seedNonceByCommitment.set(key, record);
      return record;
    }
  } catch {
    // Ignore unreadable storage.
  }
  return null;
}

// Returns the commitment to embed in this seat's signed player genesis. The
// nonce is reused across re-signs until it is revealed (guest) or the match
// starts (every seat), so an honest host retry keeps working while a revealed
// nonce is never committed again.
export async function genesisSeedCommitmentFor(matchId, seat) {
  const key = seatKey(matchId, seat);
  const existing = currentSeedCommitmentBySeat.get(key);
  if (existing && seedNonceByCommitment.has(existing)) return existing;
  const nonce = randomAuditHex(32).toLowerCase();
  const commitment = await matchSeedCommitment(nonce);
  const record = {
    nonce,
    matchId: String(matchId || ""),
    seat: Number(seat),
    hostCommitment: null,
  };
  seedNonceByCommitment.set(commitment, record);
  storeSeedRecord(commitment, record);
  currentSeedCommitmentBySeat.set(key, commitment);
  return commitment;
}

export function rotateGenesisSeedNonce(matchId, seat) {
  currentSeedCommitmentBySeat.delete(seatKey(matchId, seat));
}

export function localGenesisSeedNonce(commitment) {
  return seedRecordForCommitment(commitment)?.nonce || "";
}

function revealGenesisSeedNonce({ commitment, matchId, seat, hostCommitment }) {
  const record = seedRecordForCommitment(commitment);
  if (!record) throw new Error("Unknown match seed commitment; mark ready again");
  if (record.matchId !== String(matchId || "") || record.seat !== Number(seat)) {
    throw new Error("Match seed commitment belongs to a different match or seat");
  }
  if (record.hostCommitment && record.hostCommitment !== hostCommitment) {
    // Revealing the same nonce against a second host commitment would let the
    // host grind its own nonce after seeing ours.
    rotateGenesisSeedNonce(matchId, seat);
    throw new Error("Match seed nonce was already revealed to a different host commitment; mark ready again");
  }
  record.hostCommitment = hostCommitment;
  storeSeedRecord(commitment, record);
  // Future signatures must commit a fresh nonce.
  if (currentSeedCommitmentBySeat.get(seatKey(matchId, seat)) === commitment) {
    rotateGenesisSeedNonce(matchId, seat);
  }
  return record.nonce;
}

function hostPlayerForPayload(payload) {
  const players = Array.isArray(payload?.players) ? payload.players : [];
  const hostSeat = payload?.genesis?.hostSeat;
  return players.find((player) => String(player?.peerId || "") === String(payload?.hostPeerId || ""))
    || (hostSeat == null ? null : players.find((player) => Number(player?.index) === Number(hostSeat)))
    || null;
}

// Guest side: answer the host's reveal request for this seat's commitment.
export async function answerMatchSeedRevealRequest(conn, message, { session, safeSend, protocolVersion }) {
  const requestId = String(message?.requestId || "");
  const respond = (fields) => safeSend(conn, {
    type: "match_seed_reveal_response",
    protocolVersion,
    requestId,
    ...fields,
  });
  try {
    if (session?.role === "host") throw new Error("Host does not answer seed reveal requests");
    const matchId = String(message?.matchId || "");
    const expectedMatchId = String(session?.lobbyId || session?.hostPeerId || "");
    if (!matchId || matchId !== expectedMatchId) {
      throw new Error("Seed reveal request is for a different match");
    }
    const hostCommitment = String(message?.hostSeedCommitment || "");
    const commitment = String(message?.seedCommitment || "");
    if (!HEX_64.test(hostCommitment) || !HEX_64.test(commitment)) {
      throw new Error("Seed reveal request is malformed");
    }
    const seat = Number(message?.seat);
    const nonce = revealGenesisSeedNonce({ commitment, matchId, seat, hostCommitment });
    respond({ seat, seedCommitment: commitment, nonce });
  } catch (err) {
    respond({ error: String(err?.message || err || "Seed reveal refused") });
  }
}

const seedRevealWaiters = new Map();

export function resolveMatchSeedReveal(message, fromPeerId = "") {
  const requestId = String(message?.requestId || "");
  const waiter = seedRevealWaiters.get(requestId);
  if (!waiter) return false;
  if (waiter.peerIds.size > 0 && fromPeerId && !waiter.peerIds.has(String(fromPeerId))) return false;
  seedRevealWaiters.delete(requestId);
  window.clearTimeout(waiter.timer);
  if (message?.error) waiter.reject(new Error(String(message.error)));
  else waiter.resolve(message);
  return true;
}

function waitForMatchSeedReveal(requestId, peerIds, timeoutMs) {
  return new Promise((resolve, reject) => {
    const timer = window.setTimeout(() => {
      seedRevealWaiters.delete(requestId);
      reject(new Error("Timed out waiting for a player's match seed reveal"));
    }, timeoutMs);
    seedRevealWaiters.set(requestId, {
      resolve,
      reject,
      timer,
      peerIds: new Set(peerIds.filter(Boolean).map(String)),
    });
  });
}

// Host side: runs after the host has signed its own genesis (fixing its seed
// commitment) and before the match genesis is signed. Sets payload.seedReveals
// and payload.seed; every peer re-derives and checks both in
// verifySignedMatchGenesis.
export async function collectMatchSeedReveals(payload, {
  connectionForPlayer,
  safeSend,
  protocolVersion,
  timeoutMs = MATCH_SEED_REVEAL_TIMEOUT_MS,
}) {
  const players = Array.isArray(payload?.players) ? payload.players : [];
  const matchId = String(payload?.auditMatchId || "");
  const host = hostPlayerForPayload(payload);
  const hostCommitment = String(host?.playerGenesisSignature?.seedCommitment || "");
  const hostNonce = localGenesisSeedNonce(hostCommitment);
  if (!host || !HEX_64.test(hostCommitment) || !hostNonce) {
    throw new Error("Host is missing its committed match seed nonce");
  }
  const reveals = await Promise.all(players.map(async (player) => {
    const seat = Number(player?.index);
    if (player === host) return { seat, nonce: hostNonce };
    const commitment = String(player?.playerGenesisSignature?.seedCommitment || "");
    if (!HEX_64.test(commitment)) {
      throw new Error(`${player?.name || `Player ${seat + 1}`} has no match seed commitment; they must mark ready again`);
    }
    const conn = connectionForPlayer(player);
    if (!conn || conn.open === false) {
      throw new Error(`${player?.name || `Player ${seat + 1}`} is not connected for the seed reveal`);
    }
    const requestId = `seed:${Date.now().toString(36)}:${randomAuditHex(8)}`;
    const waiter = waitForMatchSeedReveal(
      requestId,
      [conn.peer, player?.peerId, player?.currentPeerId],
      timeoutMs,
    );
    safeSend(conn, {
      type: "match_seed_reveal_request",
      protocolVersion,
      requestId,
      matchId,
      seat,
      seedCommitment: commitment,
      hostSeedCommitment: hostCommitment,
    });
    const response = await waiter;
    const nonce = String(response?.nonce || "").toLowerCase();
    if (Number(response?.seat) !== seat || await matchSeedCommitment(nonce) !== commitment) {
      throw new Error(`${player?.name || `Player ${seat + 1}`} revealed a seed nonce that does not match their commitment`);
    }
    return { seat, nonce };
  }));
  reveals.sort((left, right) => left.seat - right.seat);
  payload.seedReveals = reveals;
  payload.seed = await deriveMatchSeedFromReveals({ matchId, reveals });
  return payload.seed;
}

// Guest side, at match start: the host's committed nonce in the genesis must
// be the one it showed us before we revealed ours. Skipped only when this tab
// never revealed (e.g. it reloaded), since nothing local can be compared.
export function assertGenesisSeedRevealBinding(payload, localEntry) {
  const own = String(localEntry?.playerGenesisSignature?.seedCommitment || "");
  const record = seedRecordForCommitment(own);
  if (!record?.hostCommitment) return;
  const host = hostPlayerForPayload(payload);
  if (String(host?.playerGenesisSignature?.seedCommitment || "") !== record.hostCommitment) {
    throw new Error("Cheat detected from the match host: the match seed commitment changed after players revealed their nonces");
  }
}

// ---------------------------------------------------------------------------
// Frozen roster + one-shot match starts
// ---------------------------------------------------------------------------

// Once a Verified match has a signed genesis, every key lookup must come from
// that roster; host lobby_state / resync `currentPlayers` cannot change keys.
export function genesisRosterPlayers(matchStartPayload, session) {
  if (matchStartPayload?.genesis?.payloadHash && Array.isArray(matchStartPayload.players)) {
    return matchStartPayload.players;
  }
  return Array.isArray(session?.players) ? session.players : [];
}

const PINNED_ROSTER_FIELDS = [
  "auditPublicKey",
  "auditEncryptionPublicKey",
  "ziffleKey",
  "deckAuditManifest",
  "playerGenesisSignature",
];

// Overlays the genesis-bound crypto fields onto host-supplied player entries.
export function pinPlayersToGenesisRoster(players, matchStartPayload) {
  const roster = matchStartPayload?.genesis?.payloadHash && Array.isArray(matchStartPayload.players)
    ? matchStartPayload.players
    : null;
  if (!roster || !Array.isArray(players)) return players;
  return players.map((player) => {
    const signed = roster.find((entry) => Number(entry?.index) === Number(player?.index));
    if (!signed) return player;
    const pinned = { ...player };
    for (const field of PINNED_ROSTER_FIELDS) {
      if (Object.prototype.hasOwnProperty.call(signed, field)) pinned[field] = signed[field];
    }
    return pinned;
  });
}

const startedGenesisHashes = new Set();

export function rememberStartedGenesis(payload) {
  const hash = String(payload?.genesis?.payloadHash || "");
  if (hash) startedGenesisHashes.add(hash);
}

// Finding: a re-sent match_start must not rewind a Verified match. A genesis
// can start at most once per tab, and a different genesis may replace a
// running match only through the rematch flow.
export function assertVerifiedMatchStartIsFresh(payload, { session, currentPayload, gameOver = false }) {
  const hash = String(payload?.genesis?.payloadHash || "");
  if (!hash) throw new Error("Match start payload is missing signed genesis");
  if (startedGenesisHashes.has(hash)
      || String(currentPayload?.genesis?.payloadHash || "") === hash) {
    throw new Error("Ignoring a repeated match start for a match that already started");
  }
  // A running match is only replaced by a rematch of a finished game.
  if (session?.matchStarted && currentPayload && (!session?.rematch || !gameOver)) {
    throw new Error("Host tried to replace a match that is still in progress");
  }
}

export function genesisAlreadyStarted(payload) {
  const hash = String(payload?.genesis?.payloadHash || "");
  return Boolean(hash) && startedGenesisHashes.has(hash);
}

// ---------------------------------------------------------------------------
// Cross-peer genesis acks (carried on heartbeats)
// ---------------------------------------------------------------------------

const localAckCache = { hash: "", ack: null, pending: false };
const ackCheckState = new Map(); // `${localHash}:${seat}` -> { firstMismatchAt, confirmed }
const localGenesisSeenAt = new Map(); // hash -> ms

export function localGenesisAck(payload, { keyPair, localSeat, localPeerId }) {
  const hash = String(payload?.genesis?.payloadHash || "");
  if (!hash || !keyPair || localSeat == null) return null;
  if (!localGenesisSeenAt.has(hash)) localGenesisSeenAt.set(hash, Date.now());
  if (localAckCache.hash === hash) return localAckCache.ack;
  if (localAckCache.pending) return null;
  localAckCache.pending = true;
  const targetHash = hash;
  void buildSignedMatchGenesisAck({
    keyPair,
    matchId: String(payload.auditMatchId || ""),
    genesisPayloadHash: targetHash,
    seat: Number(localSeat),
    peerId: String(localPeerId || ""),
  }).then((ack) => {
    localAckCache.hash = targetHash;
    localAckCache.ack = ack;
  }).catch(() => {}).finally(() => {
    localAckCache.pending = false;
  });
  return null;
}

// Verifies a peer's ack against OUR genesis. The ack is only attributed to a
// seat when it arrives on a connection from that seat's genesis peer id, so
// the host cannot speak for another seat. `onViolation(reason, seat)` fires on
// a key substitution (valid-looking ack that fails under the genesis key) or a
// persistent genesis disagreement.
export async function checkPeerGenesisAck(conn, ack, { payload, onViolation }) {
  const localHash = String(payload?.genesis?.payloadHash || "");
  if (!localHash || !ack || typeof ack !== "object") return;
  const fromPeer = String(conn?.peer || "");
  const player = (payload.players || []).find((entry) => String(entry?.peerId || "") === fromPeer);
  if (!player) return;
  const seat = Number(player.index);
  const key = `${localHash}:${seat}`;
  const state = ackCheckState.get(key) || { firstMismatchAt: 0, confirmed: false, flagged: false };
  ackCheckState.set(key, state);
  if (state.flagged) return;
  const ackHash = String(ack.genesisPayloadHash || "");
  if (ackHash !== localHash) {
    // A peer still on the previous match (or already on the next) is not a
    // violation; only a disagreement that persists past the grace window is.
    if (startedGenesisHashes.has(ackHash) && ackHash !== localHash) return;
    const nowMs = Date.now();
    if (!state.firstMismatchAt) state.firstMismatchAt = nowMs;
    const localSince = localGenesisSeenAt.get(localHash) || nowMs;
    if (nowMs - state.firstMismatchAt >= GENESIS_ACK_MISMATCH_GRACE_MS
        && nowMs - localSince >= GENESIS_ACK_MISMATCH_GRACE_MS) {
      state.flagged = true;
      onViolation?.(
        `${player.name || `Player ${seat + 1}`} was given a different signed match genesis than this seat`,
        seat,
      );
    }
    return;
  }
  state.firstMismatchAt = 0;
  if (state.confirmed) return;
  const valid = Number(ack.seat) === seat
    && String(ack.matchId || "") === String(payload.auditMatchId || "")
    && String(ack.peerId || "") === fromPeer
    && await verifySignedMatchGenesisAck({ ack, publicKeyHex: player.auditPublicKey });
  if (!valid) {
    state.flagged = true;
    onViolation?.(
      `${player.name || `Player ${seat + 1}`}'s genesis acknowledgement does not verify under the key the host put in the match genesis`,
      seat,
    );
    return;
  }
  state.confirmed = true;
}
