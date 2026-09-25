// Browser-side tournament credentials: organizer tournaments (with the key
// that signs invites) and the witness certificates this browser redeemed.
import {
  createAuditSessionKey,
  exportAuditKeyPair,
  exportAuditPublicKey,
  importAuditKeyPair,
  randomAuditHex,
  signAuditPayload,
} from "../multiplayer-audit.js";
import { relayBaseUrl } from "../relay/formats.js";
import {
  decodeInviteCode,
  encodeInviteCode,
  invitePayload,
  pinnedWitnessKeys,
  redeemPayload,
  tournamentDescriptor,
  tournamentIdForDescriptor,
  verifyCertificate,
  verifyInvite,
} from "./witness-protocol.js";

const ORGANIZER_KEY = "ironsmith-tournament-organizer-v1";
const CERTIFICATES_KEY = "ironsmith-tournament-certificates-v1";
const DAY_MS = 24 * 60 * 60 * 1000;

function readJson(key, fallback) {
  try { return JSON.parse(localStorage.getItem(key)) ?? fallback; } catch { return fallback; }
}
function writeJson(key, value) {
  // Losing either store loses the organizer key or a redeemed seat: fail loudly.
  localStorage.setItem(key, JSON.stringify(value));
}

// Invite names become the certified player name, which must equal the lobby
// name byte for byte; normalize them the same way everywhere.
export function normalizeTournamentPlayerName(name) {
  return String(name || "").trim().replace(/\s+/g, " ").slice(0, 40);
}

export function listOrganizedTournaments() {
  return Object.values(readJson(ORGANIZER_KEY, {})).map(({ tournamentId, descriptor, invites }) => ({
    tournamentId, name: descriptor.name, createdAt: descriptor.createdAt, invites: invites || [],
  })).sort((left, right) => right.createdAt - left.createdAt);
}

export async function createOrganizedTournament(name) {
  const trimmed = String(name || "").trim().slice(0, 80);
  if (!trimmed) throw new Error("Tournament name is required");
  const keyPair = await createAuditSessionKey();
  const descriptor = tournamentDescriptor({
    name: trimmed,
    organizerPublicKey: await exportAuditPublicKey(keyPair),
    createdAt: Date.now(),
    nonce: randomAuditHex(16),
  });
  const tournamentId = await tournamentIdForDescriptor(descriptor);
  const all = readJson(ORGANIZER_KEY, {});
  all[tournamentId] = { tournamentId, descriptor, keyPair: await exportAuditKeyPair(keyPair), invites: [] };
  writeJson(ORGANIZER_KEY, all);
  return { tournamentId, name: descriptor.name };
}

// The code is a bearer credential: whoever redeems it first owns the seat,
// so organizers hand each one privately to its player.
export async function issueTournamentInvite(tournamentId, playerName, validDays = 14) {
  const all = readJson(ORGANIZER_KEY, {});
  const record = all[tournamentId];
  if (!record) throw new Error("This browser did not create that tournament");
  const name = normalizeTournamentPlayerName(playerName);
  if (!name) throw new Error("Player name is required");
  const payload = invitePayload({
    tournament: record.descriptor,
    inviteId: randomAuditHex(16),
    playerName: name,
    expiresAt: Date.now() + Math.max(1, Number(validDays) || 14) * DAY_MS,
  });
  const keyPair = await importAuditKeyPair(record.keyPair);
  const code = encodeInviteCode({ payload, signature: await signAuditPayload(keyPair, payload) });
  record.invites = [...(record.invites || []), { inviteId: payload.inviteId, playerName: name, expiresAt: payload.expiresAt, code }];
  writeJson(ORGANIZER_KEY, all);
  return code;
}

export function listTournamentCertificates() {
  return Object.values(readJson(CERTIFICATES_KEY, {}));
}

export function storedTournamentCertificate(tournamentId, auditPublicKey) {
  const entry = readJson(CERTIFICATES_KEY, {})[tournamentId];
  if (!entry || entry.certificate?.payload?.auditPublicKey !== auditPublicKey) return null;
  return entry;
}

export async function previewTournamentInvite(code) {
  const { invite, tournamentId } = await verifyInvite(decodeInviteCode(code));
  return { tournamentId, tournamentName: invite.tournament.name, playerName: invite.playerName, expiresAt: invite.expiresAt };
}

// A witness key is accepted when it is pinned in this build; unpinned builds
// (development) trust the relay's advertised key and say so.
export async function witnessPublicKey(url = relayBaseUrl()) {
  const pinned = pinnedWitnessKeys();
  const response = await fetch(`${url}/witness/key`, { signal: AbortSignal.timeout(10_000) });
  if (!response.ok) throw new Error("The tournament witness is not available");
  const { publicKey } = await response.json();
  const key = String(publicKey || "").toLowerCase();
  if (pinned.length && !pinned.includes(key)) throw new Error("The relay's witness key is not trusted by this build");
  return { publicKey: key, pinned: pinned.length > 0 };
}

export function isTrustedWitnessKey(key, { advertised } = {}) {
  const pinned = pinnedWitnessKeys();
  const normalized = String(key || "").toLowerCase();
  return pinned.length ? pinned.includes(normalized) : Boolean(advertised && normalized === advertised);
}

export async function redeemTournamentInvite(code, { keyPair, auditPublicKey, url = relayBaseUrl() }) {
  if (!url) throw new Error("Tournaments need the WebSocket lobby service");
  const signedInvite = decodeInviteCode(code);
  const { invite, tournamentId } = await verifyInvite(signedInvite);
  const redeem = redeemPayload({ tournamentId, inviteId: invite.inviteId, auditPublicKey, requestedAt: Date.now() });
  const response = await fetch(`${url}/witness/redeem`, {
    method: "POST",
    // text/plain keeps this a simple request: no CORS preflight to pay for.
    headers: { "Content-Type": "text/plain" },
    body: JSON.stringify({ invite: signedInvite, redeem, redeemSignature: await signAuditPayload(keyPair, redeem) }),
    signal: AbortSignal.timeout(15_000),
  });
  const body = await response.json().catch(() => ({}));
  if (!response.ok) throw new Error(body.error || "The witness rejected this invite");
  const witness = await witnessPublicKey(url);
  if (String(body.witnessPublicKey || "").toLowerCase() !== witness.publicKey) throw new Error("The witness answered with an unexpected key");
  const cert = await verifyCertificate(body.certificate, witness.publicKey, { tournamentId, auditPublicKey });
  const entry = {
    tournamentId,
    tournamentName: cert.tournamentName,
    playerName: cert.playerName,
    expiresAt: cert.expiresAt,
    certificate: body.certificate,
    witnessPublicKey: witness.publicKey,
  };
  const all = readJson(CERTIFICATES_KEY, {});
  all[tournamentId] = entry;
  writeJson(CERTIFICATES_KEY, all);
  return entry;
}
