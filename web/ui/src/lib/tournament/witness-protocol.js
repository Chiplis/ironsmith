// Tournament witness protocol shared by browsers and the relay Worker.
//
// The witness never sees game state. It binds an organizer-issued invite to a
// player's persistent audit key (certificate), attests which certified keys
// sit in a match (genesis attestation), and turns an unanswered challenge
// into a signed forfeit. Everything is canonical JSON signed with the same
// ECDSA P-256 scheme as the multiplayer audit transcript.
import {
  canonicalJson,
  importAuditPublicKey,
  sha256Hex,
  signAuditPayload,
  verifyAuditPayload,
} from "../multiplayer-audit.js";

export const WITNESS_DOMAINS = Object.freeze({
  tournament: "ironsmith-tournament-v1",
  invite: "ironsmith-tournament-invite-v1",
  redeem: "ironsmith-tournament-redeem-v1",
  certificate: "ironsmith-witness-player-certificate-v1",
  genesisRequest: "ironsmith-witness-genesis-request-v1",
  genesis: "ironsmith-witness-genesis-v1",
  claim: "ironsmith-witness-challenge-claim-v1",
  challenge: "ironsmith-witness-challenge-v1",
  answer: "ironsmith-witness-challenge-answer-v1",
  forfeit: "ironsmith-witness-forfeit-v1",
});

export const WITNESS_FORFEIT_REASON = "witness_unanswered_challenge";
export const WITNESS_DISPUTE_TYPE = "witness_contradicted_answer";
export const WITNESS_ANSWER_WINDOW_MS = 120_000;
export const WITNESS_MAX_OPEN_CHALLENGES = 4;
// Witness ops ride one relay frame (128 KiB), so bodies stay well under it.
export const WITNESS_MAX_ANSWER_BYTES = 96 * 1024;
export const WITNESS_MAX_REQUEST_BYTES = 64 * 1024;
export const INVITE_CODE_PREFIX = "IST1.";

const HEX = /^[a-f0-9]+$/;
const isHex = (value, length) => typeof value === "string" && HEX.test(value)
  && (length == null || value.length === length);
const textEncoder = new TextEncoder();

export function byteLength(value) {
  return textEncoder.encode(typeof value === "string" ? value : canonicalJson(value)).byteLength;
}

export async function payloadHash(payload) {
  return sha256Hex(canonicalJson(payload));
}

export async function signWitnessPayload(keyPair, payload) {
  return { payload, signature: await signAuditPayload(keyPair, payload) };
}

// Verifies `signed = { payload, signature }` against a raw-hex P-256 key and
// the expected domain; throws with `label` on any mismatch.
export async function verifySignedWitnessPayload(signed, publicKeyHex, domain, label) {
  if (!signed?.payload || typeof signed.signature !== "string") throw new Error(`${label} is missing`);
  if (signed.payload.domain !== domain) throw new Error(`${label} has the wrong domain`);
  if (!isHex(publicKeyHex, 130)) throw new Error(`${label} signer key is invalid`);
  const key = await importAuditPublicKey(publicKeyHex);
  if (!await verifyAuditPayload(key, signed.payload, signed.signature)) {
    throw new Error(`${label} signature is invalid`);
  }
  return signed.payload;
}

export function tournamentDescriptor({ name, organizerPublicKey, createdAt, nonce }) {
  return {
    domain: WITNESS_DOMAINS.tournament,
    name: String(name || "").trim().slice(0, 80),
    organizerPublicKey: String(organizerPublicKey || ""),
    createdAt: Number(createdAt || 0),
    nonce: String(nonce || ""),
  };
}

// The id commits to the organizer key, so the witness can check invites
// without the tournament ever being registered.
export async function tournamentIdForDescriptor(descriptor) {
  return payloadHash(tournamentDescriptor(descriptor));
}

export function invitePayload({ tournament, inviteId, playerName, expiresAt }) {
  return {
    domain: WITNESS_DOMAINS.invite,
    tournament: tournamentDescriptor(tournament),
    inviteId: String(inviteId || ""),
    playerName: String(playerName || "").trim().slice(0, 40),
    expiresAt: Number(expiresAt || 0),
  };
}

function base64UrlEncode(text) {
  const bytes = textEncoder.encode(text);
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

function base64UrlDecode(text) {
  const normalized = String(text).replace(/-/g, "+").replace(/_/g, "/");
  const binary = atob(normalized + "=".repeat((4 - normalized.length % 4) % 4));
  return new TextDecoder().decode(Uint8Array.from(binary, (char) => char.charCodeAt(0)));
}

export function encodeInviteCode(signedInvite) {
  return INVITE_CODE_PREFIX + base64UrlEncode(canonicalJson(signedInvite));
}

export function decodeInviteCode(code) {
  const trimmed = String(code || "").replace(/\s+/g, "");
  if (!trimmed.startsWith(INVITE_CODE_PREFIX)) throw new Error("Not a tournament invite code");
  let signed;
  try { signed = JSON.parse(base64UrlDecode(trimmed.slice(INVITE_CODE_PREFIX.length))); }
  catch { throw new Error("Tournament invite code is malformed"); }
  return signed;
}

// Structural + organizer-signature checks shared by the witness and by a
// player previewing an invite before redeeming it.
export async function verifyInvite(signedInvite, now = Date.now()) {
  const payload = await verifySignedWitnessPayload(
    signedInvite,
    signedInvite?.payload?.tournament?.organizerPublicKey,
    WITNESS_DOMAINS.invite,
    "Tournament invite",
  );
  const normalized = invitePayload(payload);
  if (canonicalJson(normalized) !== canonicalJson(payload)) throw new Error("Tournament invite is not canonical");
  if (!isHex(payload.inviteId, 32)) throw new Error("Tournament invite id is invalid");
  if (!payload.playerName) throw new Error("Tournament invite has no player name");
  if (!payload.tournament.name || !isHex(payload.tournament.nonce, 32)) throw new Error("Tournament descriptor is invalid");
  if (!(payload.expiresAt > now)) throw new Error("Tournament invite has expired");
  return { invite: payload, tournamentId: await tournamentIdForDescriptor(payload.tournament) };
}

export function redeemPayload({ tournamentId, inviteId, auditPublicKey, requestedAt }) {
  return {
    domain: WITNESS_DOMAINS.redeem,
    tournamentId: String(tournamentId || ""),
    inviteId: String(inviteId || ""),
    auditPublicKey: String(auditPublicKey || ""),
    requestedAt: Number(requestedAt || 0),
  };
}

export function certificatePayload({ tournamentId, tournament, inviteId, playerName, auditPublicKey, issuedAt, expiresAt }) {
  return {
    domain: WITNESS_DOMAINS.certificate,
    tournamentId: String(tournamentId || ""),
    tournamentName: String(tournament?.name || ""),
    organizerPublicKey: String(tournament?.organizerPublicKey || ""),
    inviteId: String(inviteId || ""),
    playerName: String(playerName || ""),
    auditPublicKey: String(auditPublicKey || ""),
    issuedAt: Number(issuedAt || 0),
    expiresAt: Number(expiresAt || 0),
  };
}

export async function verifyCertificate(signed, witnessPublicKey, { tournamentId, auditPublicKey, now = Date.now() } = {}) {
  const cert = await verifySignedWitnessPayload(signed, witnessPublicKey, WITNESS_DOMAINS.certificate, "Player certificate");
  if (tournamentId != null && cert.tournamentId !== tournamentId) throw new Error("Player certificate is for another tournament");
  if (auditPublicKey != null && cert.auditPublicKey !== auditPublicKey) throw new Error("Player certificate is for another key");
  if (!(cert.expiresAt > now)) throw new Error("Player certificate has expired");
  return cert;
}

// Everything the witness binds into a match; `players` is seat-ordered.
export function genesisRequestPayload({ genesisHash, matchId, lobbyId, tournamentId, format, securityMode, hostSeat, players }) {
  return {
    domain: WITNESS_DOMAINS.genesisRequest,
    genesisHash: String(genesisHash || ""),
    matchId: String(matchId || ""),
    lobbyId: String(lobbyId || ""),
    tournamentId: String(tournamentId || ""),
    format: String(format || ""),
    securityMode: String(securityMode || ""),
    hostSeat: Number(hostSeat || 0),
    players: (players || []).map((player) => ({
      seat: Number(player.seat),
      peerId: String(player.peerId || ""),
      name: String(player.name || ""),
      auditPublicKey: String(player.auditPublicKey || ""),
    })),
  };
}

export function genesisAttestationPayload(request, certificates, issuedAt) {
  return {
    domain: WITNESS_DOMAINS.genesis,
    genesisHash: request.genesisHash,
    matchId: request.matchId,
    lobbyId: request.lobbyId,
    tournamentId: request.tournamentId,
    format: request.format,
    securityMode: request.securityMode,
    players: request.players.map((player, index) => ({
      ...player,
      inviteId: String(certificates[index]?.inviteId || ""),
    })),
    issuedAt: Number(issuedAt || 0),
  };
}

// Checks run by the witness before attesting, and by every peer (plus the
// transcript verifier) on the attestation it produced.
export async function verifyGenesisRequest({ request, hostSignature, certificates }, witnessPublicKey, now = Date.now()) {
  if (request?.domain !== WITNESS_DOMAINS.genesisRequest) throw new Error("Genesis request has the wrong domain");
  if (canonicalJson(genesisRequestPayload(request)) !== canonicalJson(request)) throw new Error("Genesis request is not canonical");
  if (!isHex(request.genesisHash, 64) || !isHex(request.tournamentId, 64)) throw new Error("Genesis request hashes are invalid");
  if (request.securityMode !== "verified") throw new Error("Tournament matches require Verified mode");
  const players = request.players;
  if (players.length < 2 || players.length > 4) throw new Error("Tournament matches need 2 to 4 players");
  if (players.some((player, index) => player.seat !== index)) throw new Error("Genesis request seats are not contiguous");
  if (new Set(players.map((player) => player.auditPublicKey)).size !== players.length) throw new Error("A key occupies two seats");
  if (!Array.isArray(certificates) || certificates.length !== players.length) throw new Error("Every seat needs a certificate");
  const verified = [];
  for (const [index, player] of players.entries()) {
    const cert = await verifyCertificate(certificates[index], witnessPublicKey, {
      tournamentId: request.tournamentId, auditPublicKey: player.auditPublicKey, now,
    });
    if (cert.playerName !== player.name) throw new Error(`Seat ${index + 1} name does not match its certificate`);
    verified.push(cert);
  }
  if (new Set(verified.map((cert) => cert.inviteId)).size !== players.length) throw new Error("One invite occupies two seats");
  const host = players[request.hostSeat];
  if (!host) throw new Error("Genesis request host seat is invalid");
  if (!await verifyAuditPayload(await importAuditPublicKey(host.auditPublicKey), request, String(hostSignature || ""))) {
    throw new Error("Genesis request host signature is invalid");
  }
  return verified;
}

export async function verifyGenesisAttestation(signed, witnessPublicKey, expected = {}) {
  const attestation = await verifySignedWitnessPayload(signed, witnessPublicKey, WITNESS_DOMAINS.genesis, "Witness genesis attestation");
  for (const field of ["genesisHash", "matchId", "lobbyId", "tournamentId", "format", "securityMode"]) {
    if (expected[field] != null && attestation[field] !== String(expected[field])) {
      throw new Error(`Witness genesis attestation ${field} does not match this match`);
    }
  }
  if (expected.players) {
    const actual = attestation.players.map(({ seat, peerId, name, auditPublicKey }) => ({ seat, peerId, name, auditPublicKey }));
    const wanted = expected.players.map((player) => ({
      seat: Number(player.seat), peerId: String(player.peerId || ""),
      name: String(player.name || ""), auditPublicKey: String(player.auditPublicKey || ""),
    }));
    if (canonicalJson(actual) !== canonicalJson(wanted)) throw new Error("Witness genesis attestation seats do not match this match");
  }
  return attestation;
}

export function claimPayload({ matchId, tournamentId, claimantSeat, accusedSeat, reason, basisSequence, headStateHash, request, claimedAt }) {
  return {
    domain: WITNESS_DOMAINS.claim,
    matchId: String(matchId || ""),
    tournamentId: String(tournamentId || ""),
    claimantSeat: Number(claimantSeat),
    accusedSeat: Number(accusedSeat),
    reason: String(reason || ""),
    basisSequence: Number(basisSequence || 0),
    headStateHash: String(headStateHash || ""),
    request: request ?? null,
    claimedAt: Number(claimedAt || 0),
  };
}

export function challengePayload({ challengeId, claim, claimHash, openedAt, deadline }) {
  return {
    domain: WITNESS_DOMAINS.challenge,
    challengeId: String(challengeId || ""),
    matchId: claim.matchId,
    tournamentId: claim.tournamentId,
    claimHash: String(claimHash || ""),
    claimantSeat: claim.claimantSeat,
    accusedSeat: claim.accusedSeat,
    reason: claim.reason,
    basisSequence: claim.basisSequence,
    headStateHash: claim.headStateHash,
    openedAt: Number(openedAt || 0),
    deadline: Number(deadline || 0),
  };
}

// `responses` are protocol messages the accused re-sends for a forwarded
// request; `actions` are its signed action envelopes past `basisSequence`.
// The witness forwards both without judging them; a false `awaitingSeat` is
// signed evidence the transcript verifier can check by replay.
export function answerPayload({ challengeId, matchId, accusedSeat, headSequence, headStateHash, awaitingSeat, responses, actions, answeredAt }) {
  return {
    domain: WITNESS_DOMAINS.answer,
    challengeId: String(challengeId || ""),
    matchId: String(matchId || ""),
    accusedSeat: Number(accusedSeat),
    headSequence: Number(headSequence || 0),
    headStateHash: String(headStateHash || ""),
    awaitingSeat: awaitingSeat == null ? null : Number(awaitingSeat),
    responses: Array.isArray(responses) ? responses : [],
    actions: Array.isArray(actions) ? actions : [],
    answeredAt: Number(answeredAt || 0),
  };
}

export function forfeitPayload(challenge, decidedAt) {
  return {
    domain: WITNESS_DOMAINS.forfeit,
    challengeId: challenge.challengeId,
    matchId: challenge.matchId,
    tournamentId: challenge.tournamentId,
    claimHash: challenge.claimHash,
    claimantSeat: challenge.claimantSeat,
    accusedSeat: challenge.accusedSeat,
    reason: challenge.reason,
    basisSequence: challenge.basisSequence,
    headStateHash: challenge.headStateHash,
    openedAt: challenge.openedAt,
    deadline: challenge.deadline,
    decidedAt: Number(decidedAt || 0),
  };
}

export async function verifyForfeitCertificate(signed, witnessPublicKey, expected = {}) {
  const forfeit = await verifySignedWitnessPayload(signed, witnessPublicKey, WITNESS_DOMAINS.forfeit, "Witness forfeit certificate");
  for (const field of ["matchId", "tournamentId", "accusedSeat", "claimantSeat"]) {
    if (expected[field] != null && String(forfeit[field]) !== String(expected[field])) {
      throw new Error(`Witness forfeit certificate ${field} does not match`);
    }
  }
  if (!(forfeit.decidedAt >= forfeit.deadline)) throw new Error("Witness forfeit certificate was decided before its deadline");
  return forfeit;
}

// Witness keys a client or verifier accepts. Build-time pinned keys win; the
// relay-advertised key is only used when nothing is pinned (development).
export function pinnedWitnessKeys(raw = import.meta.env?.VITE_WITNESS_PUBLIC_KEYS || "") {
  return String(raw).split(",").map((key) => key.trim().toLowerCase()).filter((key) => isHex(key, 130));
}

export async function witnessPublicKeyFromPrivateJwk(jwk) {
  const publicJwk = { kty: jwk.kty, crv: jwk.crv, x: jwk.x, y: jwk.y, ext: true };
  const key = await crypto.subtle.importKey("jwk", publicJwk, { name: "ECDSA", namedCurve: "P-256" }, true, ["verify"]);
  const raw = new Uint8Array(await crypto.subtle.exportKey("raw", key));
  return Array.from(raw, (byte) => byte.toString(16).padStart(2, "0")).join("");
}

// Transcript verifier hook: a tournament transcript must carry a witness
// attestation over exactly its genesis. `pinned` says whether this build
// recognizes the witness key; a verifier with no pinned keys reports the key
// so an organizer can compare it with the official one.
export async function verifyTranscriptWitness(match, genesisHash) {
  if (!match?.tournament && !match?.witnessGenesis) return null;
  const tournament = match.tournament || {};
  const witnessPublicKey = String(tournament.witnessPublicKey || "").toLowerCase();
  const pinned = pinnedWitnessKeys();
  if (pinned.length && !pinned.includes(witnessPublicKey)) throw new Error("Tournament transcript uses a witness key this build does not trust");
  const players = [...(match.players || [])]
    .sort((left, right) => Number(left.index) - Number(right.index))
    .map((player) => ({ seat: Number(player.index), peerId: player.peerId, name: player.name, auditPublicKey: player.auditPublicKey }));
  const attestation = await verifyGenesisAttestation(match.witnessGenesis, witnessPublicKey, {
    genesisHash,
    matchId: match.auditMatchId,
    lobbyId: match.lobbyId,
    tournamentId: tournament.tournamentId,
    format: match.format,
    securityMode: "verified",
    players,
  });
  return {
    tournamentId: attestation.tournamentId,
    tournamentName: String(tournament.tournamentName || ""),
    witnessPublicKey,
    witnessKeyPinned: pinned.length > 0,
    players: attestation.players.map(({ seat, name, inviteId }) => ({ seat, name, inviteId })),
    attestedAt: attestation.issuedAt,
  };
}

// A claimant's evidence that the accused told the witness it was waiting on
// the claimant. The signatures prove who said what; whether the statement was
// false is decided by replaying the transcript to `sequence`.
export async function verifyWitnessDispute(dispute, playerKeys, witness, matchId) {
  if (!witness) throw new Error("Witness dispute evidence outside a tournament match");
  const challenge = await verifySignedWitnessPayload(dispute.challenge, witness.witnessPublicKey, WITNESS_DOMAINS.challenge, "Disputed witness challenge");
  if (challenge.matchId !== matchId) throw new Error("Disputed witness challenge is for another match");
  if (canonicalJson(claimPayload(dispute.claim || {})) !== canonicalJson(dispute.claim) || await payloadHash(dispute.claim) !== challenge.claimHash) {
    throw new Error("Disputed witness challenge does not match its claim");
  }
  const accusedKey = playerKeys.get(challenge.accusedSeat)?.auditPublicKey;
  const answer = await verifySignedWitnessPayload(dispute.answer, accusedKey, WITNESS_DOMAINS.answer, "Disputed witness answer");
  if (answer.challengeId !== challenge.challengeId) throw new Error("Disputed witness answer is for another challenge");
  if (answer.awaitingSeat !== challenge.claimantSeat) throw new Error("Disputed witness answer does not blame the claimant");
  return {
    type: dispute.type,
    sequence: answer.headSequence,
    accusedPlayers: [challenge.accusedSeat],
    challengeId: challenge.challengeId,
    statement: { awaitingSeat: answer.awaitingSeat, headStateHash: answer.headStateHash },
  };
}
