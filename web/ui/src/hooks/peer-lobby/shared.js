import { useCallback, useEffect, useRef, useState } from "react";
import Peer from "peerjs";
import {
  auditStateHash,
  actionQuorumThreshold,
  assertResyncActionsExtendLocalTranscript,
  authorizeCryptoMaterialRequestRequirements,
  cryptoMaterialResponsibleSeat,
  buildActionForkDisputeEvidence,
  buildSignedDisconnectForfeitVote,
  buildSignedActionEnvelope,
  buildSignedActionQuorumVote,
  buildSignedProtocolResponseTimeoutVote,
  buildDeckSlotOpening,
  buildPrivateDeckManifest,
  buildSignedMatchGenesis,
  buildSignedPlayerGenesis,
  buildSignedResyncEnvelope,
  buildZiffleOpeningProof,
  canonicalJson,
  CURRENT_AUDIT_MAX_PLAYERS,
  CURRENT_AUDIT_MIN_PLAYERS,
  CURRENT_AUDIT_PROTOCOL_VERSION,
  DISCONNECT_AUTO_FORFEIT_MS,
  DISCONNECT_FORFEIT_REASON,
  PROTOCOL_RESPONSE_TIMEOUT_MS,
  PROTOCOL_RESPONSE_TIMEOUT_REASON,
  createAuditEncryptionKey,
  createAuditSessionKey,
  decryptPrivateAuditPayload,
  encryptPrivateAuditPayload,
  decklistHashForCards,
  exportAuditEncryptionKeyPair,
  exportAuditEncryptionPublicKey,
  exportAuditKeyPair,
  exportAuditPublicKey,
  fairRandomCombinedSeedHex,
  importAuditEncryptionKeyPair,
  importAuditKeyPair,
  importAuditPublicKey,
  isCurrentAuditPlayerCount,
  isDisconnectForfeitReason,
  isProtocolResponseTimeoutForfeitReason,
  protocolResponseTimeoutVoteThreshold,
  publicCheckpointHash,
  publicDeckManifest,
  randomAuditHex,
  rngCommitmentPayload,
  rngRevealPayload,
  sha256Hex,
  signAuditPayload,
  verifyLiveAuditTranscript,
  verifyActionQuorumCertificate,
  verifyActionQuorumVote,
  verifyAuditPayload,
  verifyCardOpeningAgainstManifest,
  verifyDisconnectForfeitCertificate,
  verifyDisconnectForfeitVote,
  verifyProtocolResponseTimeoutCertificate,
  verifyProtocolResponseTimeoutVote,
  verifySignedMatchGenesis,
  verifySignedResyncEnvelope,
} from "@/lib/multiplayer-audit";
import {
  MATCH_FORMAT_COMMANDER,
  MATCH_FORMAT_NORMAL,
  MATCH_FORMAT_PLANECHASE,
  evaluateLobbyDeckSubmission,
  normalizeMatchFormat,
  parseCommanderList,
  parseDeckList,
  parseDeckPrintPreferences,
  parseSideboardList,
  saveDefaultLobbyDeck,
} from "@/lib/decklists";
import { setPreferredCardPrints } from "@/lib/scryfall";
import { emitSyncFailureNotice } from "@/lib/ui-notices";
import { isDisadvantageousActivePlayerClockAdvance } from "@/lib/match-clock";
import {
  isDecisionCommandCompatible,
  normalizeSelectObjectHiddenRef,
  selectObjectCandidateForId,
  selectObjectCandidateRevealPolicy,
  selectObjectSyncMetadataForCommand,
} from "@/lib/sync-commands";
import {
  isSupportedZiffleDeckCount,
  normalizeZiffleCardPositions,
  pendingActionIntentHardTimeoutMs,
  ZIFFLE_REVEAL_TOKEN_TIMEOUT_MS_PER_CARD,
  ziffleRevealTokenTimeoutMs,
} from "@/lib/ziffle-timeouts";
import { preloadCardArt } from "@/lib/scryfall";
import {
  MULTIPLAYER_SECURITY_TRUSTED,
  MULTIPLAYER_SECURITY_VERIFIED,
  isTrustedMultiplayerSecurityMode,
  isVerifiedMultiplayerSecurityMode,
  normalizeMultiplayerSecurityMode,
} from "@/lib/multiplayer-security";

export { CURRENT_AUDIT_MAX_PLAYERS, CURRENT_AUDIT_MIN_PLAYERS, CURRENT_AUDIT_PROTOCOL_VERSION, DISCONNECT_AUTO_FORFEIT_MS, DISCONNECT_FORFEIT_REASON, MATCH_FORMAT_COMMANDER, MATCH_FORMAT_NORMAL, MATCH_FORMAT_PLANECHASE, MULTIPLAYER_SECURITY_TRUSTED, MULTIPLAYER_SECURITY_VERIFIED, PROTOCOL_RESPONSE_TIMEOUT_MS, PROTOCOL_RESPONSE_TIMEOUT_REASON, Peer, ZIFFLE_REVEAL_TOKEN_TIMEOUT_MS_PER_CARD, actionQuorumThreshold, assertResyncActionsExtendLocalTranscript, auditStateHash, authorizeCryptoMaterialRequestRequirements, buildActionForkDisputeEvidence, buildDeckSlotOpening, buildPrivateDeckManifest, buildSignedActionEnvelope, buildSignedActionQuorumVote, buildSignedDisconnectForfeitVote, buildSignedMatchGenesis, buildSignedPlayerGenesis, buildSignedProtocolResponseTimeoutVote, buildSignedResyncEnvelope, buildZiffleOpeningProof, canonicalJson, createAuditEncryptionKey, createAuditSessionKey, cryptoMaterialResponsibleSeat, decklistHashForCards, decryptPrivateAuditPayload, emitSyncFailureNotice, encryptPrivateAuditPayload, evaluateLobbyDeckSubmission, exportAuditEncryptionKeyPair, exportAuditEncryptionPublicKey, exportAuditKeyPair, exportAuditPublicKey, fairRandomCombinedSeedHex, importAuditEncryptionKeyPair, importAuditKeyPair, importAuditPublicKey, isCurrentAuditPlayerCount, isDecisionCommandCompatible, isDisadvantageousActivePlayerClockAdvance, isDisconnectForfeitReason, isProtocolResponseTimeoutForfeitReason, isSupportedZiffleDeckCount, isTrustedMultiplayerSecurityMode, isVerifiedMultiplayerSecurityMode, normalizeMatchFormat, normalizeMultiplayerSecurityMode, normalizeSelectObjectHiddenRef, normalizeZiffleCardPositions, parseCommanderList, parseDeckList, parseDeckPrintPreferences, parseSideboardList, pendingActionIntentHardTimeoutMs, preloadCardArt, protocolResponseTimeoutVoteThreshold, publicCheckpointHash, publicDeckManifest, randomAuditHex, rngCommitmentPayload, rngRevealPayload, saveDefaultLobbyDeck, selectObjectCandidateForId, selectObjectCandidateRevealPolicy, selectObjectSyncMetadataForCommand, setPreferredCardPrints, sha256Hex, signAuditPayload, useCallback, useEffect, useRef, useState, verifyActionQuorumCertificate, verifyActionQuorumVote, verifyAuditPayload, verifyCardOpeningAgainstManifest, verifyDisconnectForfeitCertificate, verifyDisconnectForfeitVote, verifyLiveAuditTranscript, verifyProtocolResponseTimeoutCertificate, verifyProtocolResponseTimeoutVote, verifySignedMatchGenesis, verifySignedResyncEnvelope, ziffleRevealTokenTimeoutMs };


export const PROTOCOL_VERSION = CURRENT_AUDIT_PROTOCOL_VERSION;
export const DEFAULT_OPENING_HAND_SIZE = 7;
export const INITIAL_AUDIT_STATE_HASH = "0".repeat(64);
export const INITIAL_MATCH_CLOCK_HASH = "0".repeat(64);
export const PEER_OPEN_TIMEOUT_MS = 10000;
export const PEER_CONNECT_TIMEOUT_MS = 15000;
export const PEER_HEARTBEAT_INTERVAL_MS = 3000;
export const PEER_HEARTBEAT_TIMEOUT_MS = 10000;
export const DEFAULT_PLAYER_CLOCK_MS = 40 * 60 * 1000;
export const MATCH_CLOCK_TICK_MS = 1000;
export const MATCH_CLOCK_CLAIM_SKEW_MS = 2000;
export const MATCH_CLOCK_ELAPSED_UNDERREPORT_SKEW_MS = 15000;
export const ACTION_SUBMISSION_IDLE_WAIT_MS = 5000;
export const PROTOCOL_RESPONSE_TIMEOUT_VOTE_WAIT_MS = 15000;
export const MAX_PENDING_ACTION_INTENT_MS = pendingActionIntentHardTimeoutMs(PROTOCOL_RESPONSE_TIMEOUT_MS);
export const MATCH_CLOCK_AUDIT_TYPE = "match_clock_v1";
export const MATCH_CLOCK_POLICY_TYPE = "per_player_match_clock_v1";
export const MATCH_CLOCK_AUDIT_DOMAIN = "ironsmith-match-clock-audit-v1";
export const TIMEOUT_VOTE_DOMAIN = "ironsmith-match-timeout-vote-v1";
export const ACTION_INTENT_DOMAIN = "ironsmith-action-intent-v1";
export const RECONNECT_PROOF_DOMAIN = "ironsmith-reconnect-proof-v1";
export const AUDIT_DECK_MANIFEST_STORAGE_PREFIX = "ironsmith.auditDeckManifest.v1";
export const AUDIT_REVEALED_OPENING_STORAGE_PREFIX = "ironsmith.auditRevealedOpening.v1";
export const ACTION_QUORUM_VOTE_STORAGE_PREFIX = "ironsmith.actionQuorumVote.v1";
export const AUDIT_IDENTITY_STORAGE_KEY = "ironsmith.auditIdentity.v1";
export const ZIFFLE_IDENTITY_STORAGE_PREFIX = "ironsmith.ziffleIdentity.v1";
export const preloadedPrivateDeckArtKeys = new Set();
export const CURRENT_PLAYER_STORAGE_KEY = "currentPlayer";
export const CURRENT_LOBBY_STORAGE_KEY = "currentLobby";
export const MATCH_SEED_OFFSET = 0xcbf29ce484222325n;
export const MATCH_SEED_PRIME = 0x100000001b3n;
export const MATCH_SEED_MASK = 0xffffffffffffffffn;
export const ZIFFLE_OPENING_PREVIEW_BATCH_SIZE = 8;
export const matchSeedEncoder = new TextEncoder();
export const sleep = (ms) => new Promise((resolve) => globalThis.setTimeout(resolve, ms));

export function matchPayloadSecurityMode(payload, fallback = MULTIPLAYER_SECURITY_TRUSTED) {
  if (payload && Object.prototype.hasOwnProperty.call(payload, "securityMode")) {
    return normalizeMultiplayerSecurityMode(payload.securityMode, fallback);
  }
  if (payload?.genesis || (Array.isArray(payload?.ziffleCeremonies) && payload.ziffleCeremonies.length > 0)) {
    return MULTIPLAYER_SECURITY_VERIFIED;
  }
  return normalizeMultiplayerSecurityMode(fallback);
}

export function sessionSecurityMode(session, fallback = MULTIPLAYER_SECURITY_TRUSTED) {
  return normalizeMultiplayerSecurityMode(session?.securityMode, fallback);
}

export function sequencedActionSecurityMode(message, session) {
  return normalizeMultiplayerSecurityMode(
    message?.securityMode,
    sessionSecurityMode(session, MULTIPLAYER_SECURITY_VERIFIED)
  );
}

export function normalizeActionOpeningPreview(value) {
  if (!value || typeof value !== "object") return null;
  const owner = Number(value.owner);
  const card = String(value.card ?? value.name ?? "").trim();
  if (!Number.isSafeInteger(owner) || owner < 0 || !card) return null;
  const objectId = Number(value.objectId ?? value.object_id);
  const stableId = Number(value.stableId ?? value.stable_id);
  const slot = Number(value.slot);
  const position = Number(value.position);
  return {
    owner,
    card,
    zone: String(value.zone || value.toZone || value.to_zone || "exile"),
    ...(Number.isSafeInteger(objectId) && objectId >= 0 ? { objectId } : {}),
    ...(Number.isSafeInteger(stableId) && stableId >= 0 ? { stableId } : {}),
    ...(Number.isSafeInteger(slot) && slot >= 0 ? { slot } : {}),
    ...(Number.isSafeInteger(position) && position >= 0 ? { position } : {}),
  };
}

export function actionOpeningPreviewKey(value) {
  const preview = normalizeActionOpeningPreview(value);
  if (!preview) return "";
  return [
    preview.owner,
    preview.slot ?? "",
    preview.objectId ?? "",
    preview.stableId ?? "",
    preview.position ?? "",
    preview.zone || "",
    preview.card || "",
  ].join(":");
}

export function mergeActionOpeningPreviews(existing = [], additions = [], limit = 240) {
  const merged = [];
  const seen = new Set();
  const add = (value) => {
    const preview = normalizeActionOpeningPreview(value);
    const key = actionOpeningPreviewKey(preview);
    if (!preview || !key || seen.has(key)) return;
    seen.add(key);
    merged.push(preview);
  };
  for (const preview of existing || []) add(preview);
  for (const preview of additions || []) add(preview);
  return merged.length > limit ? merged.slice(merged.length - limit) : merged;
}

export function chunkList(values = [], size = 1) {
  const chunkSize = Math.max(1, Math.floor(Number(size) || 1));
  const chunks = [];
  for (let index = 0; index < values.length; index += chunkSize) {
    chunks.push(values.slice(index, index + chunkSize));
  }
  return chunks;
}

export function actionOpeningPreviewFromOpening(opening, options = {}) {
  return normalizeActionOpeningPreview({
    owner: opening?.owner,
    card: opening?.card,
    slot: opening?.slot,
    objectId: opening?.objectId ?? opening?.object_id,
    stableId: opening?.stableId ?? opening?.stable_id,
    position: opening?.position,
    zone: opening?.zone || opening?.toZone || options.zone || options.previewZone || "exile",
  });
}

export function notifyOpeningBuilt(options = {}, opening, metadata = {}) {
  if (typeof options?.onOpeningBuilt !== "function") return;
  try {
    options.onOpeningBuilt(opening, metadata);
  } catch {
    // Progress previews must never affect action generation.
  }
}

export function createEmptyState() {
  const matchClock = createMatchClockSnapshot({
    policy: buildMatchClockConfig(),
  });
  return {
    role: null,
    mode: "idle",
    lobbyId: "",
    hostPeerId: "",
    localPeerId: "",
    localName: "",
    localPlayerIndex: null,
    desiredPlayers: 0,
    startingLife: 20,
    format: MATCH_FORMAT_NORMAL,
    securityMode: MULTIPLAYER_SECURITY_TRUSTED,
    signalingServer: "",
    localDeckText: "",
    localCommanderText: "",
    localDeckCount: 0,
    localCommanderCount: 0,
    players: [],
    connectionWarnings: [],
    rematch: null,
    matchStarted: false,
    lastAppliedSequence: 0,
    submittingAction: false,
    peerWait: null,
    matchClock,
    actionTimer: actionTimerSnapshotFromMatchClock(matchClock),
  };
}

export function nowMonotonicMs() {
  const now = globalThis.performance?.now?.();
  return Number.isFinite(now) ? now : Date.now();
}

export function createMatchClockSnapshot({
  policy = buildMatchClockConfig(),
  playerCount = 0,
  baseRemainingMsByPlayer = [],
  activePlayerIndex = null,
  epochStartedAtMs = null,
  clockHash = INITIAL_MATCH_CLOCK_HASH,
  lastSequence = 0,
  nowMs = nowMonotonicMs(),
} = {}) {
  const normalizedPolicy = normalizeMatchClockPolicy(policy);
  const normalizedPlayerCount = Math.max(0, Number(playerCount || baseRemainingMsByPlayer.length || 0));
  const remainingMsByPlayer = normalizeMatchClockRemaining(
    baseRemainingMsByPlayer,
    normalizedPlayerCount,
    normalizedPolicy.initialMs
  );
  const activePlayer = activePlayerIndex == null ? null : Number(activePlayerIndex);
  const activeIndex = Number.isInteger(activePlayer)
    && activePlayer >= 0
    && activePlayer < normalizedPlayerCount
    ? activePlayer
    : null;
  const startedAt = Number(epochStartedAtMs);
  const hasStartedAt = activeIndex != null && Number.isFinite(startedAt) && startedAt >= 0;
  if (hasStartedAt) {
    const elapsedMs = Math.max(0, Math.floor(Number(nowMs || 0) - startedAt));
    remainingMsByPlayer[activeIndex] = Math.max(
      0,
      Number(remainingMsByPlayer[activeIndex] || 0) - elapsedMs
    );
  }
  const activeRemaining = activeIndex == null ? null : remainingMsByPlayer[activeIndex];
  const deadlineAtMs = hasStartedAt
    ? startedAt + Number(baseRemainingMsByPlayer[activeIndex] ?? normalizedPolicy.initialMs)
    : null;
  return {
    enabled: normalizedPolicy.initialMs > 0,
    type: MATCH_CLOCK_POLICY_TYPE,
    initialMs: normalizedPolicy.initialMs,
    timeoutMs: normalizedPolicy.initialMs,
    graceMs: normalizedPolicy.graceMs,
    currentPlayerIndex: activeIndex,
    activePlayerIndex: activeIndex,
    startedAtMs: hasStartedAt ? startedAt : null,
    deadlineAtMs,
    remainingMs: activeRemaining,
    remainingMsByPlayer,
    expired: activeRemaining === 0 && activeIndex != null,
    clockHash: String(clockHash || INITIAL_MATCH_CLOCK_HASH),
    lastSequence: Number(lastSequence || 0),
  };
}

export function actionTimerSnapshotFromMatchClock(matchClock) {
  return {
    enabled: Boolean(matchClock?.enabled),
    timeoutMs: Number(matchClock?.initialMs ?? matchClock?.timeoutMs ?? 0),
    currentPlayerIndex: matchClock?.activePlayerIndex ?? matchClock?.currentPlayerIndex ?? null,
    startedAtMs: matchClock?.startedAtMs ?? null,
    deadlineAtMs: matchClock?.deadlineAtMs ?? null,
    remainingMs: matchClock?.remainingMs ?? null,
    expired: Boolean(matchClock?.expired),
  };
}

export function toErrorMessage(err, fallback = "Action rejected") {
  const message = String(err?.message || err || "").trim();
  return message || fallback;
}

export function sanitizePlayerName(raw, fallback = "Player") {
  const trimmed = String(raw || "").trim();
  return trimmed || fallback;
}

export function mixMatchSeedBytes(hash, bytes) {
  let next = hash;
  for (const byte of bytes) {
    next ^= BigInt(byte);
    next = (next * MATCH_SEED_PRIME) & MATCH_SEED_MASK;
  }
  next ^= 0xffn;
  return (next * MATCH_SEED_PRIME) & MATCH_SEED_MASK;
}

export function mixMatchSeedString(hash, value) {
  return mixMatchSeedBytes(hash, matchSeedEncoder.encode(String(value ?? "")));
}

export function mixMatchSeedNumber(hash, value) {
  return mixMatchSeedString(hash, Number(value ?? 0));
}

export function mixMatchSeedCardLists(hash, lists) {
  let next = mixMatchSeedNumber(hash, lists?.length ?? 0);
  for (const cards of lists || []) {
    next = mixMatchSeedNumber(next, cards.length);
    for (const card of cards) {
      next = mixMatchSeedString(next, card);
    }
  }
  return next;
}

export function createMatchSeed({
  players,
  format,
  decks,
  commanders,
  planarDecks,
  sideboards,
  startingLife,
  openingHandSize,
}) {
  let hash = MATCH_SEED_OFFSET;
  hash = mixMatchSeedString(hash, "ironsmith-match-seed-v1");
  hash = mixMatchSeedString(hash, format || MATCH_FORMAT_NORMAL);
  hash = mixMatchSeedNumber(hash, startingLife ?? 20);
  hash = mixMatchSeedNumber(hash, openingHandSize ?? DEFAULT_OPENING_HAND_SIZE);
  hash = mixMatchSeedNumber(hash, players?.length ?? 0);
  for (const player of players || []) {
    hash = mixMatchSeedString(hash, player?.name || "");
    hash = mixMatchSeedNumber(hash, player?.index ?? -1);
  }
  hash = mixMatchSeedCardLists(hash, decks);
  hash = mixMatchSeedCardLists(hash, commanders);
  hash = mixMatchSeedCardLists(
    hash,
    (planarDecks || []).map((cards) =>
      (cards || []).map((card) => String(card?.name || card || ""))
    )
  );
  hash = mixMatchSeedCardLists(hash, sideboards);

  const seed = Number(hash & BigInt(Number.MAX_SAFE_INTEGER));
  return seed > 0 ? seed : 1;
}

export function readPeerEnv(name) {
  const value = import.meta.env?.[name];
  return typeof value === "string" ? value.trim() : "";
}

export function parseBooleanEnv(value, fallback) {
  if (!value) return fallback;
  const normalized = value.trim().toLowerCase();
  if (["1", "true", "yes", "on"].includes(normalized)) return true;
  if (["0", "false", "no", "off"].includes(normalized)) return false;
  return fallback;
}

export function parseNumberEnv(value, fallback) {
  if (!value) return fallback;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : fallback;
}

export function formatPeerError(err, fallback = "Peer connection failed") {
  const type = String(err?.type || "").trim();
  const message = String(err?.message || err || "").trim();

  if (type === "peer-unavailable") {
    return "Lobby host was not found on the current signaling server. The code can still be correct if the host disconnected or the two machines are using different VITE_PEER_* settings.";
  }
  if (type === "network" || type === "server-error" || type === "socket-error") {
    return "Could not reach the PeerJS signaling server.";
  }
  if (type === "socket-closed" || type === "disconnected") {
    return "Disconnected from the PeerJS signaling server.";
  }
  if (type === "browser-incompatible") {
    return "This browser does not support the required WebRTC data-channel features.";
  }
  if (type === "webrtc") {
    return message || "The browser could not establish a WebRTC peer connection.";
  }
  if (message) {
    return `${fallback}: ${message}`;
  }
  return fallback;
}

export function isRecoverablePeerError(err) {
  const type = String(err?.type || "").trim();
  return (
    type === "network" ||
    type === "socket-error" ||
    type === "socket-closed" ||
    type === "disconnected"
  );
}

export function parseIceConfig() {
  const raw = readPeerEnv("VITE_PEER_ICE_SERVERS");
  if (!raw) return null;

  try {
    const parsed = JSON.parse(raw);
    if (Array.isArray(parsed) && parsed.length > 0) {
      return {
        iceServers: parsed,
        sdpSemantics: "unified-plan",
      };
    }
  } catch (err) {
    console.warn("Failed to parse VITE_PEER_ICE_SERVERS:", err);
  }

  return null;
}

export function describePeerServer(options) {
  const host = options?.host || "0.peerjs.com";
  const port = options?.port || 443;
  return `${host}:${port}`;
}

export function buildPeerOptions() {
  const host = readPeerEnv("VITE_PEER_HOST");
  const path = readPeerEnv("VITE_PEER_PATH");
  const key = readPeerEnv("VITE_PEER_KEY");
  const port = parseNumberEnv(readPeerEnv("VITE_PEER_PORT"), 0);
  const debug = parseNumberEnv(
    readPeerEnv("VITE_PEER_DEBUG"),
    import.meta.env.DEV ? 2 : 1
  );
  const pingInterval = parseNumberEnv(readPeerEnv("VITE_PEER_PING_INTERVAL"), 0);
  const iceConfig = parseIceConfig();

  const options = {
    debug,
  };

  if (iceConfig) {
    options.config = iceConfig;
  }

  if (host) {
    options.host = host;
  }
  if (path) {
    options.path = path;
  }
  if (key) {
    options.key = key;
  }
  if (port > 0) {
    options.port = port;
  }
  if (pingInterval > 0) {
    options.pingInterval = pingInterval;
  }
  if (readPeerEnv("VITE_PEER_SECURE")) {
    options.secure = parseBooleanEnv(readPeerEnv("VITE_PEER_SECURE"), true);
  }

  return options;
}

export function buildPeerHeartbeatConfig() {
  const intervalMs = Math.max(
    0,
    parseNumberEnv(
      readPeerEnv("VITE_PEER_HEARTBEAT_INTERVAL_MS"),
      PEER_HEARTBEAT_INTERVAL_MS
    )
  );
  const timeoutMs = Math.max(
    intervalMs * 2,
    parseNumberEnv(
      readPeerEnv("VITE_PEER_HEARTBEAT_TIMEOUT_MS"),
      PEER_HEARTBEAT_TIMEOUT_MS
    )
  );
  return { intervalMs, timeoutMs };
}

export function normalizeMatchClockPolicy(policy = {}) {
  const initialMs = Math.max(
    0,
    Number(policy.initialMs ?? policy.timeoutMs ?? DEFAULT_PLAYER_CLOCK_MS)
  );
  const graceMs = Math.max(
    0,
    Number(policy.graceMs ?? MATCH_CLOCK_CLAIM_SKEW_MS)
  );
  return {
    type: MATCH_CLOCK_POLICY_TYPE,
    initialMs,
    graceMs,
  };
}

export function buildMatchClockConfig() {
  const initialMs = Math.max(
    0,
    parseNumberEnv(
      readPeerEnv("VITE_MULTIPLAYER_PLAYER_CLOCK_MS")
        || readPeerEnv("VITE_MULTIPLAYER_ACTION_TIMEOUT_MS"),
      DEFAULT_PLAYER_CLOCK_MS
    )
  );
  const graceMs = Math.max(
    0,
    parseNumberEnv(
      readPeerEnv("VITE_MULTIPLAYER_CLOCK_GRACE_MS"),
      MATCH_CLOCK_CLAIM_SKEW_MS
    )
  );
  return normalizeMatchClockPolicy({ initialMs, graceMs });
}

export function normalizeMatchClockRemaining(remaining, playerCount, initialMs) {
  const count = Math.max(0, Number(playerCount || 0));
  return Array.from({ length: count }, (_, index) => {
    const value = Number(Array.isArray(remaining) ? remaining[index] : undefined);
    return Math.max(0, Number.isFinite(value) ? Math.floor(value) : Math.floor(Number(initialMs || 0)));
  });
}

export function matchClockPolicyFromPayload(payload, fallbackPolicy = buildMatchClockConfig()) {
  return normalizeMatchClockPolicy({
    initialMs:
      payload?.matchClockPolicy?.initialMs
      ?? payload?.matchClockMs
      ?? payload?.timeoutMs
      ?? fallbackPolicy.initialMs,
    graceMs:
      payload?.matchClockPolicy?.graceMs
      ?? payload?.matchClockGraceMs
      ?? fallbackPolicy.graceMs,
  });
}

export function matchClockPolicyPayload(policy = {}) {
  const normalized = normalizeMatchClockPolicy(policy);
  return {
    type: MATCH_CLOCK_POLICY_TYPE,
    initialMs: normalized.initialMs,
    graceMs: normalized.graceMs,
  };
}

export function isForfeitCommand(command) {
  return command?.type === "forfeit_player";
}

// Commands that are not engine decision commands: the WASM `UiCommand` enum
// cannot deserialize them, so they must never be routed through engine dispatch
// (previewCryptoRequirements/dispatch). They produce no hidden-card material —
// `cancel_decision` is a local rollback and `forfeit_player` removes a seat.
export function isNonDispatchSyncCommand(command) {
  const type = String(command?.type || "");
  return type === "cancel_decision" || type === "forfeit_player";
}

export function isActionTimeoutForfeitCommand(command) {
  const reason = String(command?.reason || "");
  return isForfeitCommand(command)
    && (
      reason === "peer_claimed_match_clock_timeout"
      || reason === "match_clock_timeout"
      || reason === "peer_claimed_action_timeout"
      || reason === "action_timeout"
    );
}

export function isDisconnectTimeoutForfeitCommand(command) {
  return isForfeitCommand(command)
    && isDisconnectForfeitReason(command?.reason);
}

export function isProtocolResponseTimeoutForfeitCommand(command) {
  return isForfeitCommand(command)
    && isProtocolResponseTimeoutForfeitReason(command?.reason);
}

export function isSelfForfeitCommand(command, actorIndex) {
  return isForfeitCommand(command)
    && !isActionTimeoutForfeitCommand(command)
    && !isDisconnectTimeoutForfeitCommand(command)
    && !isProtocolResponseTimeoutForfeitCommand(command)
    && Number(command.player) === Number(actorIndex);
}

export function normalizedTurnToken(value) {
  return String(value || "").trim().toLowerCase().replace(/[\s_-]+/g, "");
}

export function isMainPhaseName(phase) {
  const normalized = normalizedTurnToken(phase);
  return (
    normalized === "firstmain"
    || normalized === "firstmainphase"
    || normalized === "nextmain"
    || normalized === "nextmainphase"
    || normalized === "secondmain"
    || normalized === "secondmainphase"
  );
}

export function isSorcerySpeedForfeitState(uiState, playerIndex) {
  const player = Number(playerIndex);
  const decision = uiState?.decision || null;
  return (
    decision?.kind === "priority"
    && Number(decision.player) === player
    && Number(uiState?.active_player ?? uiState?.activePlayer ?? player) === player
    && Number(uiState?.stack_size || 0) === 0
    && isMainPhaseName(uiState?.phase)
  );
}

export function disconnectCertificateFromCommand(command) {
  const certificate = command?.disconnect_certificate || command?.disconnectCertificate || null;
  return certificate && typeof certificate === "object" ? certificate : null;
}

export function disconnectForfeitVoteThreshold(nonTargetPlayerCount) {
  const count = Math.max(0, Number(nonTargetPlayerCount || 0));
  return count;
}

export function timeoutVotePayload({
  matchId,
  basisSequence,
  forfeitedPlayer,
  activePlayer,
  clockHash,
  remainingMs,
  voter,
}) {
  return {
    domain: TIMEOUT_VOTE_DOMAIN,
    matchId: String(matchId || ""),
    basisSequence: Number(basisSequence || 0),
    forfeitedPlayer: Number(forfeitedPlayer),
    activePlayer: Number(activePlayer),
    clockHash: String(clockHash || ""),
    remainingMs: Math.max(0, Math.floor(Number(remainingMs || 0))),
    voter: Number(voter),
  };
}

export function timeoutCertificateFromCommand(command) {
  const certificate = command?.timeout_certificate || command?.timeoutCertificate || null;
  return certificate && typeof certificate === "object" ? certificate : null;
}

export function expectedTimeoutVoters(players = [], forfeitedPlayer) {
  return reindexPlayers(players)
    .map((player) => Number(player.index))
    .filter((index) => Number.isInteger(index) && index !== Number(forfeitedPlayer))
    .sort((left, right) => left - right);
}

export function isOwnerPrivateViewRequirement(requirement) {
  const type = String(requirement?.type || "");
  return (
    (type === "private_open" || type === "private_view_window")
    && requirement?.owner != null
    && requirement?.viewer != null
    && Number(requirement.owner) === Number(requirement.viewer)
  );
}

export function hasOnlyOwnerPrivateViewRequirements(requirements = []) {
  return (
    Array.isArray(requirements)
    && requirements.length > 0
    && requirements.every(isOwnerPrivateViewRequirement)
  );
}

export function shouldRequestRemoteCryptoPreview(command, state, previewedRequirements = []) {
  if (!command || command.type !== "priority_action") return false;
  const kind = String(command.action_ref?.kind || "");
  if (kind === "private_search_action") return true;
  if (kind !== "pass_priority") return false;
  const resolvingStackObject = Boolean(
    Number(state?.stack_size || 0) > 0
    || (Array.isArray(state?.stack_preview) && state.stack_preview.length > 0)
    || state?.resolving_stack_object
  );
  if (!resolvingStackObject) return false;
  return !hasOnlyOwnerPrivateViewRequirements(previewedRequirements);
}

export function commandMayProducePostApplyOpenings(command, state, previewedRequirements = []) {
  if (!command) return false;
  if (shouldRequestRemoteCryptoPreview(command, state, previewedRequirements)) return true;
  if (command.type === "priority_action") {
    const kind = String(command.action_ref?.kind || command.actionRef?.kind || "").toLowerCase();
    if (/(^|_)(open|reveal|look|search|draw|mill|manifest|cloak|discover|cascade|scry|surveil)(_|$)/.test(kind)) {
      return true;
    }
  }
  if (
    (previewedRequirements || []).some((requirement) =>
      ["public_open", "private_open", "public_view_window", "private_view_window"].includes(
        String(requirement?.type || requirement?.requirement_type || "")
      )
      && !isOwnerPrivateViewRequirement(requirement)
    )
  ) {
    return true;
  }
  return ["select_options", "select_objects", "targets"].includes(String(command.type || ""));
}

export function isUnauthorizedAddCardCommand(command) {
  return command?.type === "add_card_to_zone";
}

export function isRejectedActionCheatReason(reason) {
  const normalized = String(reason || "").toLowerCase();
  return normalized.includes("invalid priority action ref")
    || normalized.includes("priority action is no longer available")
    || normalized.includes("does not match pending")
    || normalized.includes("action is no longer available");
}

export function matchClockActivePlayerFromState(uiState) {
  const decision = uiState?.decision || null;
  if (!decision || uiState?.game_over || decision.player == null) return null;
  const player = Number(decision.player);
  return Number.isInteger(player) && player >= 0 ? player : null;
}

export function debitMatchClockRemaining(remaining, activePlayerIndex, elapsedMs) {
  const next = normalizeMatchClockRemaining(remaining, remaining.length, 0);
  const player = activePlayerIndex == null ? null : Number(activePlayerIndex);
  if (player != null && player >= 0 && player < next.length) {
    next[player] = Math.max(
      0,
      Number(next[player] || 0) - Math.max(0, Math.floor(Number(elapsedMs || 0)))
    );
  }
  return next;
}

export async function matchClockAuditHash(clock) {
  if (!clock || typeof clock !== "object") return "";
  const payload = { ...clock };
  delete payload.clockHash;
  return sha256Hex(canonicalJson({
    domain: MATCH_CLOCK_AUDIT_DOMAIN,
    clock: payload,
  }));
}

export function playerNameForIndex(players, playerIndex) {
  const target = (players || []).find((player) => Number(player?.index) === Number(playerIndex));
  return String(target?.name || `Player ${Number(playerIndex) + 1}`);
}

export function publicCheckpointWinner(checkpoint) {
  const players = Array.isArray(checkpoint?.players) ? checkpoint.players : [];
  const winners = players.filter((player) => Boolean(player?.hasWon ?? player?.has_won));
  if (winners.length === 1) {
    return {
      player: Number(winners[0].id ?? winners[0].index ?? winners[0].seat),
      name: String(winners[0].name || ""),
    };
  }
  const active = players.filter((player) =>
    !(player?.hasLost ?? player?.has_lost)
    && !(player?.hasLeftGame ?? player?.has_left_game)
  );
  if (active.length === 1 && players.length > 1) {
    return {
      player: Number(active[0].id ?? active[0].index ?? active[0].seat),
      name: String(active[0].name || ""),
    };
  }
  return null;
}

export function buildExportedMatchOutcome({
  uiState,
  finalPublicCheckpoint,
  finalStateHash,
  finalPublicCheckpointHash,
  matchDisputed,
  disputes = [],
}) {
  if (matchDisputed || disputes.length > 0) {
    const accusedPlayers = Array.from(new Set([
      ...(matchDisputed?.accusedPlayers || []),
      ...disputes.flatMap((dispute) => dispute?.accusedPlayers || []),
    ].map(Number))).sort((left, right) => left - right);
    return {
      status: "disputed",
      disputed: true,
      reason: String(matchDisputed?.reason || "Match transcript fork detected"),
      accusedPlayers,
      finalStateHash,
      finalPublicCheckpointHash,
    };
  }

  const gameOver = uiState?.game_over || null;
  if (gameOver?.kind === "winner") {
    return {
      status: "winner",
      winner: Number(gameOver.player),
      winnerName: String(gameOver.name || ""),
      finalStateHash,
      finalPublicCheckpointHash,
    };
  }
  if (gameOver?.kind === "draw") {
    return {
      status: "draw",
      finalStateHash,
      finalPublicCheckpointHash,
    };
  }
  const checkpointWinner = publicCheckpointWinner(finalPublicCheckpoint);
  if (checkpointWinner) {
    return {
      status: "winner",
      winner: checkpointWinner.player,
      winnerName: checkpointWinner.name,
      finalStateHash,
      finalPublicCheckpointHash,
    };
  }
  return {
    status: "stalled_or_incomplete",
    stalled: true,
    finalStateHash,
    finalPublicCheckpointHash,
  };
}

export function safeSend(conn, payload) {
  if (!conn || conn.open === false) return;
  try {
    conn.send(payload);
  } catch {
    // PeerJS can report stale connections as open until the next send.
  }
}

export function createPeer(peerId, options) {
  const requestedPeerId = String(peerId || "").trim();
  return requestedPeerId ? new Peer(requestedPeerId, options) : new Peer(options);
}

export function connectionHeartbeatKey(kind, peerId) {
  return `${kind}:${String(peerId || "")}`;
}

export function cloneMultiplayerPayload(value) {
  if (value == null) return value;
  return JSON.parse(JSON.stringify(value));
}

export function stripTransientZifflePositionOpeningFields(opening) {
  if (!opening || typeof opening !== "object") return opening;
  const stripped = cloneMultiplayerPayload(opening);
  delete stripped.position;
  delete stripped.positionCommitment;
  delete stripped.position_commitment;
  delete stripped.ziffleReveal;
  delete stripped.ziffleProof;
  delete stripped.positionOpeningProof;
  delete stripped.shuffleObjectId;
  delete stripped.shuffle_object_id;
  delete stripped.publicSlot;
  delete stripped.public_slot;
  delete stripped.publicCommitment;
  delete stripped.public_commitment;
  delete stripped.reportedSlot;
  delete stripped.reported_slot;
  delete stripped.objectId;
  delete stripped.object_id;
  delete stripped.ziffleContext;
  delete stripped.ziffle_context;
  return stripped;
}

export function compactZiffleCeremonyForDiagnostics(ceremony) {
  if (!ceremony || typeof ceremony !== "object") return null;
  return {
    owner: ceremony.owner == null ? null : Number(ceremony.owner),
    deckCount: ceremony.deckCount == null ? null : Number(ceremony.deckCount),
    context: String(ceremony.context || ""),
    keyContext: String(ceremony.keyContext || ceremony.context || ""),
    deckHash: String(ceremony.deckHash || ""),
    keyPlayers: Array.isArray(ceremony.keys)
      ? ceremony.keys.map((key) => Number(key?.player)).filter((player) => Number.isFinite(player))
      : [],
    stepCount: Array.isArray(ceremony.steps) ? ceremony.steps.length : 0,
  };
}

export function ziffleKeyContextForCeremony(ceremony) {
  return String(ceremony?.keyContext || ceremony?.context || "");
}

export function compactZiffleDiagnosticsJson(diagnostics) {
  try {
    return JSON.stringify(diagnostics);
  } catch {
    return "{}";
  }
}

export function ziffleDiagnosticNoticeBody(message, diagnostics) {
  const text = String(message || "Unknown ziffle ceremony").trim();
  const requestId = String(diagnostics?.requestId || diagnostics?.requester?.requestId || "");
  const owner =
    diagnostics?.requestedOwner
    ?? diagnostics?.requester?.ceremony?.owner
    ?? diagnostics?.requester?.targetPlayerIndex
    ?? null;
  const context =
    diagnostics?.requestedContext
    || diagnostics?.requester?.ceremony?.context
    || diagnostics?.local?.auditMatchId
    || "";
  return [
    text,
    requestId ? `request ${requestId}` : "",
    owner == null ? "" : `owner ${owner}`,
    context ? `context ${context}` : "",
    "click Copy diagnostics for full JSON",
  ].filter(Boolean).join(" | ");
}

export function collectCommandObjectIds(command, output = new Set(), uiState = null) {
  if (!command || typeof command !== "object") return output;
  if (command.type === "priority_action" && command.action_ref) {
    const objectId = actionRefObjectId(command.action_ref);
    const numeric = Number(objectId);
    if (Number.isSafeInteger(numeric) && numeric > 0) {
      output.add(numeric);
    }
  }
  if (
    command.type === "select_objects"
    && Array.isArray(command.object_ids)
  ) {
    for (const objectId of command.object_ids) {
      const candidate = selectObjectCandidateForId(uiState?.decision, objectId);
      if (selectObjectCandidateRevealPolicy(uiState?.decision, candidate) !== "public") {
        continue;
      }
      const numeric = Number(objectId);
      if (Number.isSafeInteger(numeric) && numeric > 0) {
        output.add(numeric);
      }
    }
  }
  return output;
}

export function cryptoRequirementsFromState(state) {
  return Array.isArray(state?.crypto_requirements)
    ? state.crypto_requirements
    : Array.isArray(state?.cryptoRequirements)
      ? state.cryptoRequirements
      : [];
}

export function priorityActionKindForCommand(command, uiState = null) {
  const direct = String(command?.action_ref?.kind || command?.actionRef?.kind || "").trim();
  if (direct) return direct;
  const index = Number(command?.action_index ?? command?.actionIndex);
  const actions = uiState?.decision?.actions;
  if (!Number.isSafeInteger(index) || !Array.isArray(actions)) return "";
  return String(actions[index]?.action_ref?.kind || actions[index]?.actionRef?.kind || "").trim();
}

export function isPregameNonShuffleCommand(command, uiState = null) {
  const kind = priorityActionKindForCommand(command, uiState);
  const decisionActions = Array.isArray(uiState?.decision?.actions)
    ? uiState.decision.actions
    : [];
  const decisionActionKinds = new Set(decisionActions.map((action) =>
    String(action?.action_ref?.kind || action?.actionRef?.kind || "").trim()
  ));
  const decisionActionLabels = new Set(decisionActions.map((action) =>
    String(action?.label || "").trim().toLowerCase()
  ));
  const openingHandDecision =
    (
      decisionActionKinds.has("keep_opening_hand")
      && decisionActionKinds.has("take_mulligan")
    )
    || (
      decisionActionLabels.has("keep hand")
      && decisionActionLabels.has("mulligan")
    );
  if (
    command?.type === "priority_action"
    && (
      kind === "keep_opening_hand"
      || kind === "continue_pregame"
      || kind === "begin_game"
      || (openingHandDecision && kind !== "take_mulligan")
    )
  ) {
    return true;
  }
  const decision = uiState?.decision || null;
  const description = String(decision?.description || decision?.context_text || "").toLowerCase();
  if (
    (command?.type === "select_objects" || command?.type === "select_options")
    && /bottom of (your|their|his|her|its)?\s*library/.test(description)
  ) {
    return true;
  }
  return false;
}

export function filterCryptoRequirementsForCommand(command, uiState, requirements = []) {
  const list = Array.isArray(requirements) ? requirements : [];
  if (!isPregameNonShuffleCommand(command, uiState)) return list;
  return list.filter((requirement) =>
    String(requirement?.type || requirement?.requirement_type || "") !== "verifiable_shuffle"
  );
}

export function openingMatchesRequirement(opening, requirement) {
  if (!opening || !requirement) return false;
  if (requirement.owner != null && Number(opening.owner) !== Number(requirement.owner)) {
    return false;
  }
  const requirementCommitments = [
    requirement.commitment,
    requirement.positionCommitment,
    requirement.position_commitment,
    requirement.publicCommitment,
    requirement.public_commitment,
  ]
    .map((entry) => String(entry || ""))
    .filter((entry, index, list) => entry && list.indexOf(entry) === index);
  const openingCommitments = [
    opening.commitment,
    opening.positionCommitment,
    opening.position_commitment,
    opening.publicCommitment,
    opening.public_commitment,
  ]
    .map((entry) => String(entry || ""))
    .filter(Boolean);
  const requirementSlot = Number(requirement.slot);
  const openingSlot = Number(opening.slot);
  const requirementHasPositionIdentity = Boolean(
    requirementCommitments.length > 0
    || requirement.publicSlot != null
    || requirement.public_slot != null
    || requirement.position != null
  );
  if (
    !requirementHasPositionIdentity
    && !openingHasZifflePosition(opening)
    && Number.isSafeInteger(requirementSlot)
    && requirementSlot >= 0
    && Number.isSafeInteger(openingSlot)
    && openingSlot === requirementSlot
    && (
      !requirement.card
      || !opening.card
      || String(opening.card || "") === String(requirement.card || "")
    )
  ) {
    return true;
  }
  if (
    requirement.objectId != null
    && opening.objectId != null
    && Number(opening.objectId) !== Number(requirement.objectId)
    && requirement.slot == null
    && requirement.publicSlot == null
    && requirement.public_slot == null
    && requirement.position == null
    && requirementCommitments.length === 0
  ) {
    return false;
  }
  const requirementObjectId = Number(requirement.objectId ?? requirement.object_id);
  const openingObjectId = Number(opening.objectId ?? opening.object_id);
  const objectIdsMatch =
    Number.isSafeInteger(requirementObjectId)
    && requirementObjectId > 0
    && Number.isSafeInteger(openingObjectId)
    && openingObjectId === requirementObjectId;
  const cardsCompatible =
    !requirement.card
    || !opening.card
    || String(opening.card || "") === String(requirement.card || "");
  if (objectIdsMatch && cardsCompatible) {
    const requirementSlots = [
      requirement.slot,
      requirement.publicSlot,
      requirement.public_slot,
      requirement.position,
    ]
      .map((entry) => Number(entry))
      .filter((entry, index, list) =>
        Number.isSafeInteger(entry) && entry >= 0 && list.indexOf(entry) === index
      );
    if (requirementSlots.length === 0) return true;
    const openingSlots = [
      opening.slot,
      opening.publicSlot,
      opening.public_slot,
      opening.position,
    ]
      .map((entry) => Number(entry))
      .filter((entry) => Number.isSafeInteger(entry) && entry >= 0);
    if (requirementSlots.some((slot) => openingSlots.includes(slot))) {
      return true;
    }
  }
  if (requirementCommitments.length > 0) {
    if (!requirementCommitments.some((commitment) =>
      openingCommitments.includes(commitment)
    )) {
      return false;
    }
    return true;
  }
  if (requirementHasZifflePosition(requirement) || openingHasZifflePosition(opening)) {
    return false;
  }
  const requirementSlots = [
    requirement.slot,
    requirement.publicSlot,
    requirement.public_slot,
    requirement.position,
  ]
    .map((entry) => Number(entry))
    .filter((entry, index, list) =>
      Number.isSafeInteger(entry) && entry >= 0 && list.indexOf(entry) === index
    );
  if (requirementSlots.length === 0) return true;
  const openingSlots = [
    opening.slot,
    opening.publicSlot,
    opening.public_slot,
    opening.position,
  ]
    .map((entry) => Number(entry))
    .filter((entry) => Number.isSafeInteger(entry) && entry >= 0);
  return requirementSlots.some((slot) => openingSlots.includes(slot));
}

export function cachedOpeningMatchesZifflePosition(opening, position, positionCommitment) {
  if (!opening || typeof opening !== "object") return false;
  const expectedCommitment = String(positionCommitment || "");
  const openingCommitment = String(
    opening.positionCommitment
    || opening.position_commitment
    || opening.publicCommitment
    || opening.public_commitment
    || ""
  );
  if (expectedCommitment && openingCommitment !== expectedCommitment) {
    return false;
  }
  const expectedPosition =
    zifflePositionFromCommitment(expectedCommitment)
    ?? (position == null ? null : Number(position));
  const openingPosition =
    zifflePositionFromCommitment(openingCommitment)
    ?? (opening.position == null ? null : Number(opening.position));
  if (
    expectedPosition != null
    && Number.isSafeInteger(expectedPosition)
    && openingPosition != null
    && Number.isSafeInteger(openingPosition)
    && openingPosition !== expectedPosition
  ) {
    return false;
  }
  return Boolean(
    openingHasZifflePosition(opening)
    || opening.shuffleObjectId != null
    || opening.shuffle_object_id != null
  );
}

export function ziffleOpeningLinkKey(opening) {
  if (!openingHasZifflePosition(opening)) return null;
  const owner = Number(opening.owner);
  const slot = Number(opening.slot);
  if (
    !Number.isSafeInteger(owner)
    || owner < 0
    || !Number.isSafeInteger(slot)
    || slot < 0
  ) {
    return null;
  }
  const positionCommitment = String(
    opening.positionCommitment
    || opening.position_commitment
    || opening.publicCommitment
    || opening.public_commitment
    || ""
  );
  if (!ziffleDeckHashFromCommitment(positionCommitment)) return null;
  const position = zifflePositionFromCommitment(positionCommitment) ?? Number(opening.position);
  if (!Number.isSafeInteger(position) || position < 0) return null;
  return JSON.stringify([
    owner,
    slot,
    String(opening.card || ""),
    position,
    positionCommitment,
  ]);
}

export function openingShuffleSourceId(opening) {
  const id = Number(
    opening?.shuffleObjectId
    ?? opening?.shuffle_object_id
  );
  return Number.isSafeInteger(id) && id >= 0 ? id : null;
}

export function normalizeMergedZiffleOpeningShuffleIds(openings = []) {
  const preSourceByKey = new Map();
  for (const opening of openings) {
    if (String(opening?.timing || "pre") !== "pre") continue;
    const key = ziffleOpeningLinkKey(opening);
    const sourceId = openingShuffleSourceId(opening);
    if (key && sourceId != null) {
      preSourceByKey.set(key, {
        sourceId,
        ziffleContext: ziffleContextFromOpening(opening),
      });
    }
  }
  return openings.map((opening) => {
    if (String(opening?.timing || "pre") === "pre") return opening;
    const key = ziffleOpeningLinkKey(opening);
    const source = key ? preSourceByKey.get(key) : null;
    if (source == null) return opening;
    const sourceId = source.sourceId;
    const current = Number(opening.shuffleObjectId ?? opening.shuffle_object_id);
    const context = ziffleContextFromOpening(opening);
    if (
      Number.isSafeInteger(current)
      && current === sourceId
      && (context || !source.ziffleContext)
    ) {
      return opening;
    }
    return {
      ...opening,
      shuffleObjectId: sourceId,
      ...(source.ziffleContext && !context ? { ziffleContext: source.ziffleContext } : {}),
    };
  });
}

export function mergeAuditOpenings(...openingLists) {
  const merged = new Map();
  for (const opening of openingLists.flat()) {
    if (!opening || opening.owner == null || opening.slot == null) continue;
    const key = `${Number(opening.owner)}:${Number(opening.slot)}:${Number(opening.objectId ?? -1)}`;
    const existing = merged.get(key);
    if (existing?.timing === "pre" && opening.timing !== "pre") continue;
    if (openingHasZifflePosition(existing) && !openingHasZifflePosition(opening)) continue;
    merged.set(key, opening);
  }
  return normalizeMergedZiffleOpeningShuffleIds([...merged.values()]);
}

export function hasPostTimedOpenings(...openingLists) {
  return openingLists.flat().some((opening) =>
    String(opening?.timing || "pre") === "post"
  );
}

export function mergePrivateViewProofs(...proofLists) {
  const merged = new Map();
  for (const proof of proofLists.flat()) {
    if (!proof) continue;
    const key = [
      String(proof.requirementId || ""),
      String(proof.type || ""),
      Number(proof.owner ?? -1),
      Number(proof.viewer ?? -1),
      Number(proof.objectId ?? -1),
    ].join(":");
    merged.set(key, proof);
  }
  return [...merged.values()];
}

export function missingRemotePublicOpenRequirements(requirements = [], material = {}, localSeat = null) {
  return (requirements || []).filter((requirement) => {
    if (String(requirement?.type || "") !== "public_open") return false;
    const owner = Number(requirement.owner);
    if (!Number.isInteger(owner) || owner === Number(localSeat)) return false;
    return !(material.openings || []).some((opening) =>
      openingMatchesRequirement(opening, requirement)
    );
  });
}

export function expectedLocalPublicOpeningPreviewCount(requirements = [], localSeat = null) {
  const seen = new Set();
  for (const requirement of requirements || []) {
    if (String(requirement?.type || "") !== "public_open") continue;
    const owner = Number(requirement.owner);
    if (!Number.isInteger(owner) || owner !== Number(localSeat)) continue;
    const key = [
      owner,
      requirement.objectId ?? requirement.object_id ?? "",
      requirement.stableId ?? requirement.stable_id ?? "",
      requirement.position ?? "",
      requirement.publicSlot ?? requirement.public_slot ?? "",
      requirement.publicCommitment ?? requirement.public_commitment ?? "",
      requirement.slot ?? "",
      requirement.commitment ?? "",
      requirement.card ?? "",
    ].join(":");
    seen.add(key);
  }
  return seen.size;
}

export function shuffleProofMatchesRequirement(proof, requirement) {
  if (!proof || !requirement) return false;
  const proofRequirementId = String(proof?.requirementId || proof?.requirement_id || "");
  const requirementId = String(requirement.id || requirement.requirementId || requirement.requirement_id || "");
  if (proofRequirementId && requirementId) {
    return proofRequirementId === requirementId;
  }
  return (
    Number(proof?.owner) === Number(requirement.owner)
    && String(proof?.zone || "library") === String(requirement.zone || "library")
  );
}

export function shuffleProofSameOwnerZone(proof, requirement) {
  if (!proof || !requirement) return false;
  return (
    Number(proof?.owner) === Number(requirement.owner)
    && String(proof?.zone || "library") === String(requirement.zone || "library")
  );
}

export function mergeShuffleProofs(...proofLists) {
  const merged = [];
  const seen = new Set();
  for (const proof of proofLists.flat()) {
    if (!proof || typeof proof !== "object") continue;
    const key = [
      String(proof.requirementId || ""),
      Number(proof.owner),
      String(proof.zone || "library"),
      String(proof.deckHash || ""),
    ].join(":");
    if (seen.has(key)) continue;
    seen.add(key);
    merged.push(proof);
  }
  return merged;
}

export function missingShuffleRequirements(requirements = [], shuffleProofs = []) {
  return (requirements || []).filter((requirement) =>
    String(requirement?.type || "") === "verifiable_shuffle"
    && !(shuffleProofs || []).some((proof) => shuffleProofMatchesRequirement(proof, requirement))
  );
}

export function normalizeShuffleOrder(value) {
  return (Array.isArray(value) ? value : [])
    .map((entry) => Number(entry))
    .filter((entry) => Number.isSafeInteger(entry) && entry >= 0);
}

export function playerLibraryOrderFromCheckpoint(checkpoint, owner) {
  const normalizedOwner = Number(owner);
  if (!Number.isSafeInteger(normalizedOwner) || normalizedOwner < 0) return [];
  const player = (checkpoint?.players || []).find((entry) =>
    Number(entry?.id ?? entry?.index ?? entry?.player) === normalizedOwner
  );
  return normalizeShuffleOrder(player?.library);
}

export function projectShuffleOrderToCurrentLibrary(order, currentLibrary) {
  const normalizedOrder = normalizeShuffleOrder(order);
  const normalizedLibrary = normalizeShuffleOrder(currentLibrary);
  if (normalizedLibrary.length === 0 || normalizedOrder.length === 0) return normalizedOrder;
  const librarySet = new Set(normalizedLibrary);
  const projected = normalizedOrder.filter((objectId) => librarySet.has(objectId));
  const projectedSet = new Set(projected);
  if (
    projected.length === normalizedLibrary.length
    && projectedSet.size === librarySet.size
    && normalizedLibrary.every((objectId) => projectedSet.has(objectId))
  ) {
    return projected;
  }
  return null;
}

export function sameShuffleOrder(left, right) {
  const normalizedLeft = normalizeShuffleOrder(left);
  const normalizedRight = normalizeShuffleOrder(right);
  if (normalizedLeft.length !== normalizedRight.length) return false;
  return normalizedLeft.every((entry, index) => entry === normalizedRight[index]);
}

export function shuffleOrderIdMap(proofBeforeOrder, localBeforeOrder) {
  const proofBefore = normalizeShuffleOrder(proofBeforeOrder);
  const localBefore = normalizeShuffleOrder(localBeforeOrder);
  if (proofBefore.length === 0 || localBefore.length === 0) return null;
  if (proofBefore.length !== localBefore.length) return null;
  if (new Set(proofBefore).size !== proofBefore.length) return null;
  if (new Set(localBefore).size !== localBefore.length) return null;
  const mapping = new Map();
  const mappedLocalIds = new Set();
  for (let index = 0; index < proofBefore.length; index += 1) {
    const proofId = proofBefore[index];
    const localId = localBefore[index];
    if (mappedLocalIds.has(localId)) return null;
    mapping.set(proofId, localId);
    mappedLocalIds.add(localId);
  }
  return mapping;
}

export function localizeShuffleOrder(order, idMap) {
  const normalized = normalizeShuffleOrder(order);
  if (!idMap) return normalized;
  const localized = [];
  for (const entry of normalized) {
    if (!idMap.has(entry)) return null;
    localized.push(idMap.get(entry));
  }
  return localized;
}

export function shuffleProofWithRequirementOrder(proof, requirement) {
  if (!proof || !requirement) return proof;
  const beforeOrder = normalizeShuffleOrder(requirement.beforeOrder ?? requirement.before_order);
  const afterOrder = normalizeShuffleOrder(requirement.afterOrder ?? requirement.after_order);
  return {
    ...proof,
    requirementId: String(requirement.id || proof.requirementId || ""),
    owner: Number(proof.owner ?? requirement.owner),
    zone: String(proof.zone || requirement.zone || "library"),
    deckCount: Number(afterOrder.length || beforeOrder.length || proof.deckCount || 0),
    beforeOrder,
    before_order: beforeOrder,
    afterOrder,
    after_order: afterOrder,
  };
}

export function alignShuffleProofsWithRequirements(shuffleProofs = [], requirements = []) {
  const shuffleRequirements = (requirements || []).filter((requirement) =>
    String(requirement?.type || requirement?.requirement_type || "") === "verifiable_shuffle"
  );
  if (shuffleRequirements.length === 0) return shuffleProofs || [];
  const aligned = [];
  const usedProofs = new Set();
  for (const requirement of shuffleRequirements) {
    const proof = (shuffleProofs || []).find((candidate) =>
      !usedProofs.has(candidate) && shuffleProofMatchesRequirement(candidate, requirement)
    ) || (shuffleProofs || []).find((candidate) =>
      !usedProofs.has(candidate) && shuffleProofSameOwnerZone(candidate, requirement)
    );
    if (!proof) continue;
    usedProofs.add(proof);
    aligned.push(shuffleProofWithRequirementOrder(proof, requirement));
  }
  return aligned.length > 0 ? aligned : (shuffleProofs || []);
}

export function wasmObjectIdArg(objectId) {
  const normalized = Number(objectId);
  if (!Number.isSafeInteger(normalized) || normalized < 0) {
    throw new Error(`Invalid object id: ${objectId}`);
  }
  return BigInt(normalized);
}

export function actionRefObjectId(actionRef) {
  if (!actionRef || typeof actionRef !== "object") return null;
  switch (String(actionRef.kind || "")) {
    case "play_land":
      return actionRef.land_id;
    case "cast_spell":
      return actionRef.spell_id;
    case "use_pregame_action":
      return actionRef.card_id;
    case "activate_ability":
    case "activate_mana_ability":
      return actionRef.source;
    case "turn_face_up":
      return actionRef.creature_id;
    case "special_action": {
      const action = actionRef.action || {};
      return action.card_id ?? action.permanent_id;
    }
    default:
      return null;
  }
}

export function actionRefWithObjectId(actionRef, objectId) {
  if (!actionRef || typeof actionRef !== "object") return actionRef;
  const next = cloneMultiplayerPayload(actionRef);
  switch (String(next.kind || "")) {
    case "play_land":
      next.land_id = Number(objectId);
      break;
    case "cast_spell":
      next.spell_id = Number(objectId);
      break;
    case "use_pregame_action":
      next.card_id = Number(objectId);
      break;
    case "activate_ability":
    case "activate_mana_ability":
      next.source = Number(objectId);
      break;
    case "turn_face_up":
      next.creature_id = Number(objectId);
      break;
    case "special_action":
      if (next.action?.card_id != null) {
        next.action.card_id = Number(objectId);
      } else if (next.action?.permanent_id != null) {
        next.action.permanent_id = Number(objectId);
      }
      break;
  }
  return next;
}

export function hiddenOpeningMatchesExport(opening, exported) {
  if (!opening || !exported) return false;
  if (opening.owner != null && Number(opening.owner) !== Number(exported.owner)) return false;
  if (exported.commitment && (opening.commitment || opening.positionCommitment)) {
    const exportedCommitment = String(exported.commitment);
    const openingCommitment = String(opening.commitment || "");
    const positionCommitment = String(opening.positionCommitment || "");
    if (openingCommitment !== exportedCommitment && positionCommitment !== exportedCommitment) {
      return false;
    }
    if (opening.card && exported.card && String(opening.card) !== String(exported.card)) {
      return false;
    }
    return true;
  }
  if (opening.slot != null && Number(opening.slot) !== Number(exported.slot)) return false;
  if (opening.card && exported.card && String(opening.card) !== String(exported.card)) {
    return false;
  }
  return true;
}

export function hiddenCardMetadataForObjectFromCheckpoint(checkpoint, objectId) {
  const normalized = Number(objectId);
  if (!Number.isSafeInteger(normalized) || normalized < 0) return null;
  const object = (checkpoint?.objects || []).find(
    (entry) => Number(entry?.id) === normalized
  );
  const hidden = object?.hiddenCard || object?.hidden_card || null;
  if (!hidden) return null;
  return {
    objectId: normalized,
    owner: hidden.owner == null ? null : Number(hidden.owner),
    zone: String(object?.zone || ""),
    slot: hidden.slot == null ? null : Number(hidden.slot),
    commitment: String(hidden.commitment || ""),
    publicSlot: hidden.publicSlot ?? hidden.public_slot ?? null,
    publicCommitment: String(hidden.publicCommitment || hidden.public_commitment || ""),
  };
}

export function hiddenMetadataMatchesZifflePosition(metadata, position, positionCommitment = "") {
  if (!metadata) return false;
  const normalizedPosition = Number(position);
  if (!Number.isSafeInteger(normalizedPosition) || normalizedPosition < 0) return false;
  const publicSlotRaw = metadata.publicSlot ?? metadata.public_slot ?? null;
  const publicSlot = publicSlotRaw == null ? null : Number(publicSlotRaw);
  const publicCommitment = String(
    metadata.publicCommitment ?? metadata.public_commitment ?? ""
  );
  const publicDeckHash = ziffleDeckHashFromCommitment(publicCommitment);
  const expectedCommitment = String(positionCommitment || "");
  const expectedDeckHash = ziffleDeckHashFromCommitment(expectedCommitment);
  const hasPublicPosition = publicSlot != null || Boolean(publicDeckHash);
  if (!hasPublicPosition) return true;
  if (publicSlot != null && publicSlot !== normalizedPosition) return false;
  if (publicDeckHash) {
    if (expectedDeckHash && publicDeckHash !== expectedDeckHash) return false;
    if (expectedCommitment && publicCommitment !== expectedCommitment) return false;
    const committedPosition = zifflePositionFromCommitment(publicCommitment);
    if (committedPosition != null && committedPosition !== normalizedPosition) return false;
  }
  return true;
}

export function hiddenObjectIdForOpeningFromCheckpoint(checkpoint, opening) {
  if (!opening || opening.owner == null) return null;
  const owner = Number(opening.owner);
  const slot = opening.slot == null ? null : Number(opening.slot);
  const commitment = String(opening.commitment || "");
  const positionCommitment = String(opening.positionCommitment || "");
  const position =
    zifflePositionFromCommitment(positionCommitment)
    ?? (opening.position == null ? null : Number(opening.position));
  const openingCard = String(opening.card || "").trim();
  for (const object of checkpoint?.objects || []) {
    const hidden = object?.hiddenCard || object?.hidden_card || null;
    if (!hidden || Number(hidden.owner) !== owner) continue;
    const objectName = checkpointObjectName(object);
    const objectIsRedactedHidden = checkpointObjectIsRedactedHidden(object);
    if (
      !objectIsRedactedHidden
      && openingCard
      && objectName
      && objectName !== openingCard
    ) {
      continue;
    }
    const hiddenSlot = hidden.slot == null ? null : Number(hidden.slot);
    const hiddenCommitment = String(hidden.commitment || "");
    const publicSlot = hidden.publicSlot ?? hidden.public_slot ?? null;
    const publicCommitment = String(hidden.publicCommitment || hidden.public_commitment || "");
    if (
      position != null
      && ziffleDeckHashFromCommitment(positionCommitment)
      && !hiddenMetadataMatchesZifflePosition(
        {
          publicSlot,
          publicCommitment,
        },
        position,
        positionCommitment
      )
    ) {
      continue;
    }
    const matchesSlot =
      slot != null
      && hiddenSlot === slot
      && (
        !commitment
        || hiddenCommitment === commitment
        || publicCommitment === commitment
      );
    const matchesPosition =
      position != null
      && (
        Number(publicSlot) === position
        || hiddenSlot === position
      )
      && (
        !positionCommitment
        || hiddenCommitment === positionCommitment
        || publicCommitment === positionCommitment
      );
    const matchesCommitment =
      Boolean(commitment)
      && (
        hiddenCommitment === commitment
        || publicCommitment === commitment
      );
    const matchesPositionCommitment =
      Boolean(positionCommitment)
      && (
        hiddenCommitment === positionCommitment
        || publicCommitment === positionCommitment
      );
    const matchesPrivateIdentity = matchesSlot || matchesCommitment;
    const matchesOnlyPublicZifflePosition = Boolean(
      (matchesPosition || matchesPositionCommitment)
      && !matchesPrivateIdentity
    );
    if (matchesOnlyPublicZifflePosition && !objectIsRedactedHidden) {
      continue;
    }
    if (
      matchesSlot
      || matchesPosition
      || matchesCommitment
      || matchesPositionCommitment
    ) {
      const objectId = Number(object.id);
      return Number.isSafeInteger(objectId) && objectId > 0 ? objectId : null;
    }
  }
  return null;
}

export function hiddenObjectIdForHiddenRefFromCheckpoint(checkpoint, hiddenRef) {
  const ref = normalizeSelectObjectHiddenRef(hiddenRef);
  if (!ref) return null;
  const matches = [];
  for (const object of checkpoint?.objects || []) {
    const hidden = object?.hiddenCard || object?.hidden_card || null;
    const owner = hidden?.owner ?? object?.owner;
    if (ref.owner != null && Number(owner) !== Number(ref.owner)) continue;
    if (ref.zone && String(object?.zone || "") !== String(ref.zone)) continue;
    const hiddenSlot = hidden?.slot == null ? null : Number(hidden.slot);
    const hiddenCommitment = String(hidden?.commitment || "");
    const publicSlot = hidden?.publicSlot ?? hidden?.public_slot ?? null;
    const publicCommitment = String(hidden?.publicCommitment || hidden?.public_commitment || "");
    if (ref.slot != null && hiddenSlot !== Number(ref.slot)) continue;
    if (ref.public_slot != null && Number(publicSlot) !== Number(ref.public_slot)) continue;
    if (
      ref.commitment
      && hiddenCommitment !== String(ref.commitment)
      && publicCommitment !== String(ref.commitment)
    ) {
      continue;
    }
    if (
      ref.public_commitment
      && hiddenCommitment !== String(ref.public_commitment)
      && publicCommitment !== String(ref.public_commitment)
    ) {
      continue;
    }
    const objectId = Number(object?.id);
    if (Number.isSafeInteger(objectId) && objectId > 0) matches.push(objectId);
  }
  return matches.length === 1 ? matches[0] : null;
}

export function checkpointObjectForId(checkpoint, objectId) {
  const normalized = Number(objectId);
  if (!Number.isSafeInteger(normalized) || normalized <= 0) return null;
  return (checkpoint?.objects || []).find((entry) => Number(entry?.id) === normalized) || null;
}

export function checkpointObjectHiddenCard(object) {
  return object?.hiddenCard || object?.hidden_card || null;
}

export function checkpointObjectName(object) {
  return String(object?.name || object?.identity?.name || "").trim();
}

export function checkpointObjectIsRedactedHidden(object) {
  const name = checkpointObjectName(object);
  return !name || name === "Hidden Card";
}

export function knownCheckpointObjectMatchesOpening(object, opening) {
  if (!object || checkpointObjectHiddenCard(object)) return false;
  const objectName = checkpointObjectName(object);
  const openingCard = String(opening?.card || "").trim();
  return Boolean(objectName) && (!openingCard || objectName === openingCard);
}

export function canonicalMultiplayerPayload(value) {
  return canonicalJson(cloneMultiplayerPayload(value));
}

export function peerSyncPerfNow() {
  return globalThis.performance && typeof globalThis.performance.now === "function"
    ? globalThis.performance.now()
    : Date.now();
}

export function payloadSizeBytes(value) {
  try {
    return new TextEncoder().encode(canonicalMultiplayerPayload(value)).length;
  } catch {
    return null;
  }
}

export function summarizePeerCommand(command) {
  if (!command || typeof command !== "object") return null;
  const summary = { type: String(command.type || "") };
  if (command.type === "priority_action") {
    summary.action_kind = String(command.action_ref?.kind || command.actionRef?.kind || "");
  }
  if (command.type === "text_choice") {
    summary.value_length = String(command.value || "").length;
  }
  if (Array.isArray(command.object_ids)) {
    summary.object_count = command.object_ids.length;
  }
  if (Array.isArray(command.targets)) {
    summary.target_count = command.targets.length;
  }
  return summary;
}

export function summarizeCryptoRequirementsForPerf(requirements = []) {
  const byType = {};
  const byOwner = {};
  const slots = [];
  for (const requirement of requirements || []) {
    const type = String(requirement?.type || requirement?.requirement_type || "");
    byType[type] = (byType[type] || 0) + 1;
    const owner = requirement?.owner == null ? "none" : String(Number(requirement.owner));
    byOwner[owner] = (byOwner[owner] || 0) + 1;
    const slot = requirement?.slot ?? requirement?.publicSlot ?? requirement?.public_slot;
    if (slot != null && slots.length < 12) {
      slots.push(Number(slot));
    }
  }
  return {
    total: Array.isArray(requirements) ? requirements.length : 0,
    by_type: byType,
    by_owner: byOwner,
    sample_slots: slots,
  };
}

export function summarizeCryptoMaterialForPerf(material = {}) {
  const openings = Array.isArray(material?.openings) ? material.openings : [];
  const privateViewProofs = Array.isArray(material?.privateViewProofs)
    ? material.privateViewProofs
    : [];
  return {
    openings: openings.length,
    private_view_proofs: privateViewProofs.length,
    bytes: payloadSizeBytes(material),
  };
}

export function summarizeSequencedActionForPerf(message = {}) {
  const audit = message?.audit || {};
  return {
    seq: Number(message?.seq || 0),
    actor: message?.actorIndex == null ? null : Number(message.actorIndex),
    command: summarizePeerCommand(message?.command),
    openings: Array.isArray(audit.openings) ? audit.openings.length : 0,
    private_view_proofs: Array.isArray(audit.privateViewProofs) ? audit.privateViewProofs.length : 0,
    shuffle_proofs: Array.isArray(audit.shuffleProofs) ? audit.shuffleProofs.length : 0,
    rng_reveals: Array.isArray(audit.rngReveals) ? audit.rngReveals.length : 0,
    bytes: payloadSizeBytes(message),
  };
}

export function recordPeerSyncPerf(label, payload = {}) {
  const event = {
    label: `peer sync:${label}`,
    payload,
    recorded_at_ms: peerSyncPerfNow(),
  };
  try {
    console.info("[ironsmith] peer sync", event);
  } catch {
    // Ignore logging failures in restricted consoles.
  }
  if (typeof window !== "undefined") {
    const shared = Array.isArray(window.__ironsmithPerfEvents)
      ? window.__ironsmithPerfEvents
      : [];
    shared.push(event);
    window.__ironsmithPerfEvents = shared.slice(-200);
    const peerOnly = Array.isArray(window.__ironsmithPeerSyncEvents)
      ? window.__ironsmithPeerSyncEvents
      : [];
    peerOnly.push(event);
    window.__ironsmithPeerSyncEvents = peerOnly.slice(-200);
  }
  return event;
}

export async function timePeerSyncPhase(label, payload, task) {
  const startedAt = peerSyncPerfNow();
  recordPeerSyncPerf(`${label}:start`, payload);
  try {
    const result = await task();
    recordPeerSyncPerf(`${label}:done`, {
      ...payload,
      duration_ms: peerSyncPerfNow() - startedAt,
    });
    return result;
  } catch (err) {
    recordPeerSyncPerf(`${label}:error`, {
      ...payload,
      duration_ms: peerSyncPerfNow() - startedAt,
      error: toErrorMessage(err),
    });
    throw err;
  }
}

export function signedActionIntentPayload(intent = {}) {
  return {
    domain: ACTION_INTENT_DOMAIN,
    matchId: String(intent.matchId || ""),
    seq: Number(intent.seq || 0),
    actorIndex: Number(intent.actorIndex ?? intent.actor ?? 0),
    prevStateHash: String(intent.prevStateHash || ""),
    preActionPublicCheckpointHash: String(
      intent.preActionPublicCheckpointHash
      || intent.publicCheckpointHash
      || ""
    ),
    command: cloneMultiplayerPayload(intent.command || null),
  };
}

export function actionIntentKey(intent = {}) {
  const payload = signedActionIntentPayload(intent);
  return [
    payload.matchId,
    payload.seq,
    payload.actorIndex,
  ].join(":");
}

export function actionIntentFingerprint(intent = {}) {
  return canonicalMultiplayerPayload(signedActionIntentPayload(intent));
}

export function reconnectProofPayload(proof = {}) {
  return {
    domain: RECONNECT_PROOF_DOMAIN,
    matchId: String(proof.matchId || ""),
    challengeId: String(proof.challengeId || proof.requestId || ""),
    nonce: String(proof.nonce || ""),
    playerIndex: Number(proof.playerIndex),
    peerId: String(proof.peerId || ""),
    hostPeerId: String(proof.hostPeerId || ""),
    transcriptHash: String(proof.transcriptHash || ""),
  };
}

export function isProtocolResponseWaitTimeout(err) {
  return String(err?.message || err || "").startsWith("Timed out waiting for ");
}

export function protocolResponseTimeoutClaimFromError(err) {
  return err?.protocolResponseTimeoutClaim || null;
}

export function enqueueAsync(queueRef, task) {
  const next = queueRef.current.catch(() => undefined).then(task);
  queueRef.current = next.catch(() => undefined);
  return next;
}

export function sanitizeCardList(cards) {
  if (!Array.isArray(cards)) return [];
  return cards
    .map((card) => String(card || "").trim())
    .filter(Boolean);
}

export function sanitizeDeckSlotOpenings(openings) {
  if (!Array.isArray(openings)) return [];
  return openings
    .map((opening) => ({
      slot: Number(opening?.slot),
      card: String(opening?.card || "").trim(),
      salt: String(opening?.salt || ""),
      commitment: String(opening?.commitment || ""),
    }))
    .filter((opening) =>
      Number.isSafeInteger(opening.slot)
      && opening.slot >= 0
      && opening.card
      && opening.salt
      && opening.commitment
    )
    .sort((left, right) => Number(left.slot) - Number(right.slot));
}

export function deckSlotOpeningsForManifest(manifest) {
  return sanitizeDeckSlotOpenings(manifest?.slotSecrets);
}

export function openDecklistPlayerFields({
  deck = [],
  sideboard = [],
  commanders = [],
  deckSlotOpenings = [],
} = {}) {
  return {
    deck: sanitizeCardList(deck),
    sideboard: sanitizeCardList(sideboard),
    commanders: sanitizeCardList(commanders),
    deckSlotOpenings: sanitizeDeckSlotOpenings(deckSlotOpenings),
  };
}

export function parseDeckSubmission(format, deckText, commanderText = "") {
  const deck = sanitizeCardList(parseDeckList(deckText));
  const sideboard = sanitizeCardList(parseSideboardList(deckText));
  const commanders = sanitizeCardList(parseCommanderList(commanderText));
  setPreferredCardPrints([
    ...parseDeckPrintPreferences(deckText),
    ...parseDeckPrintPreferences(commanderText),
  ]);
  const status = evaluateLobbyDeckSubmission(format, deck, commanders);
  return {
    deck,
    sideboard,
    commanders,
    deckCount: status.deckCount,
    sideboardCount: sideboard.length,
    commanderCount: status.commanderCount,
    ready: status.ready,
  };
}

export function rememberDefaultLobbyDeck(deckText, commanderText = "") {
  saveDefaultLobbyDeck({ deckText, commanderText });
}

export function withDeckState(player, format, deck, commanders = [], sideboard = []) {
  const normalizedDeck = sanitizeCardList(deck);
  const normalizedCommanders = sanitizeCardList(commanders);
  const normalizedSideboard = sanitizeCardList(sideboard);
  const status = evaluateLobbyDeckSubmission(
    format,
    normalizedDeck,
    normalizedCommanders
  );
  return {
    ...player,
    deck: normalizedDeck,
    sideboard: normalizedSideboard,
    commanders: normalizedCommanders,
    deckCount: status.deckCount,
    sideboardCount: normalizedSideboard.length,
    commanderCount: status.commanderCount,
    ready: status.ready,
  };
}

export function buildRematchStateFromPayload(payload, localPeerId, readyOverrides = new Map()) {
  const players = reindexPlayers(payload?.players || []).map((player) => {
    const index = Number(player.index || 0);
    const ready = readyOverrides.has(player.peerId)
      ? Boolean(readyOverrides.get(player.peerId))
      : false;
    return {
      ...player,
      ready,
      deck: sanitizeCardList(player.deck?.length ? player.deck : payload?.decks?.[index]),
      sideboard: sanitizeCardList(player.sideboard?.length ? player.sideboard : payload?.sideboards?.[index]),
      commanders: sanitizeCardList(player.commanders?.length ? player.commanders : payload?.commanders?.[index]),
      deckSlotOpenings: sanitizeDeckSlotOpenings(player.deckSlotOpenings),
    };
  });
  const localPlayer = players.find((player) => player.peerId === localPeerId) || null;
  return {
    phase: "sideboarding",
    players,
    localDeck: sanitizeCardList(localPlayer?.deck),
    localSideboard: sanitizeCardList(localPlayer?.sideboard),
    localReady: Boolean(localPlayer?.ready),
  };
}

export function rematchPlayersReady(players) {
  const entries = Array.isArray(players) ? players : [];
  return entries.length > 0 && entries.every((player) => (
    player.connected !== false && player.ready
  ));
}

export function reindexPlayers(players) {
  return players.map((player, index) => ({ ...player, index }));
}

export function normalizePlayerIndex(value) {
  if (value === null || value === undefined || value === "") return null;
  const parsed = Number(value);
  if (!Number.isInteger(parsed) || parsed < 0 || parsed > 3) return null;
  return parsed;
}

export function resolveLocalPlayerIndex(session) {
  const explicitIndex = normalizePlayerIndex(session?.localPlayerIndex);
  if (explicitIndex != null) return explicitIndex;

  const localPeerId = String(session?.localPeerId || "").trim();
  if (!localPeerId) return null;
  const localPlayer = (session?.players || []).find(
    (player) => String(player?.peerId || "") === localPeerId
  );
  return normalizePlayerIndex(localPlayer?.index);
}

export function resolveLocalPlayerIndexFromPeer(session, players = null) {
  const localPeerId = String(session?.localPeerId || "").trim();
  if (!localPeerId) return null;
  const entries = Array.isArray(players) && players.length > 0
    ? players
    : session?.players || [];
  const localPlayer = entries.find(
    (player) => String(player?.peerId || "") === localPeerId
  );
  return normalizePlayerIndex(localPlayer?.index);
}

export function findLocalMatchPlayer(players, session, auditPublicKey = "", auditEncryptionPublicKey = "") {
  const entries = Array.isArray(players) ? players : [];
  const localPeerId = String(session?.localPeerId || "").trim();
  return entries.find((player) => String(player?.peerId || "").trim() === localPeerId)
    || entries.find((player) => String(player?.currentPeerId || "").trim() === localPeerId)
    || entries.find((player) =>
      playerMatchesPresentedAuditIdentity(player, auditPublicKey, auditEncryptionPublicKey)
    )
    || null;
}

export function getPeerSessionStorage() {
  if (typeof window === "undefined") return null;
  return window.localStorage || window.sessionStorage || null;
}

export function readStoredPlayerIndex(lobbyId) {
  const storage = getPeerSessionStorage();
  if (!storage) return null;

  try {
    const storedLobbyId = String(storage.getItem(CURRENT_LOBBY_STORAGE_KEY) || "").trim();
    const targetLobbyId = String(lobbyId || "").trim();
    if (!storedLobbyId || !targetLobbyId || storedLobbyId !== targetLobbyId) {
      return null;
    }

    const raw = String(storage.getItem(CURRENT_PLAYER_STORAGE_KEY) || "").trim();
    if (!raw) return null;
    const parsed = Number.parseInt(raw, 10);
    if (!Number.isInteger(parsed) || parsed < 1 || parsed > 4) return null;
    return parsed - 1;
  } catch {
    return null;
  }
}

export function readStoredAuditIdentity() {
  const storage = getPeerSessionStorage();
  if (!storage) return null;
  try {
    const raw = String(storage.getItem(AUDIT_IDENTITY_STORAGE_KEY) || "").trim();
    return raw ? JSON.parse(raw) : null;
  } catch {
    return null;
  }
}

export function writeStoredAuditIdentity(identity) {
  const storage = getPeerSessionStorage();
  if (!storage || !identity) return;
  try {
    storage.setItem(AUDIT_IDENTITY_STORAGE_KEY, JSON.stringify(identity));
  } catch {
    // Ignore localStorage failures.
  }
}

export function clearStoredAuditIdentity() {
  const storage = getPeerSessionStorage();
  if (!storage) return;
  try {
    storage.removeItem(AUDIT_IDENTITY_STORAGE_KEY);
  } catch {
    // Ignore localStorage failures.
  }
}

export function actionQuorumVoteStorageKey(matchId, seq, voter) {
  return [
    ACTION_QUORUM_VOTE_STORAGE_PREFIX,
    String(matchId || ""),
    Number(seq || 0),
    Number(voter),
  ].join(":");
}

export function readStoredActionQuorumVote(matchId, seq, voter) {
  const storage = getPeerSessionStorage();
  if (!storage) return null;
  try {
    const raw = String(
      storage.getItem(actionQuorumVoteStorageKey(matchId, seq, voter)) || ""
    ).trim();
    return raw ? JSON.parse(raw) : null;
  } catch {
    return null;
  }
}

export function writeStoredActionQuorumVote(vote) {
  const storage = getPeerSessionStorage();
  if (!storage || !vote?.matchId || vote.seq == null || vote.voter == null) return;
  try {
    storage.setItem(
      actionQuorumVoteStorageKey(vote.matchId, vote.seq, vote.voter),
      JSON.stringify(vote)
    );
  } catch {
    // Ignore localStorage failures.
  }
}

export function privateDeckManifestStorageKey(matchId, owner) {
  return `${AUDIT_DECK_MANIFEST_STORAGE_PREFIX}:${String(matchId || "")}:${Number(owner)}`;
}

export function readStoredPrivateDeckManifest(matchId, owner) {
  const storage = getPeerSessionStorage();
  if (!storage) return null;
  try {
    const raw = String(storage.getItem(privateDeckManifestStorageKey(matchId, owner)) || "").trim();
    return raw ? JSON.parse(raw) : null;
  } catch {
    return null;
  }
}

export function writeStoredPrivateDeckManifest(manifest) {
  const storage = getPeerSessionStorage();
  if (!storage || !manifest || manifest.owner == null || !manifest.matchId) return;
  try {
    storage.setItem(
      privateDeckManifestStorageKey(manifest.matchId, manifest.owner),
      JSON.stringify(manifest)
    );
  } catch {
    // Ignore localStorage failures.
  }
}

export function privateDeckManifestCardNames(manifest) {
  return [...new Set((manifest?.slotSecrets || [])
    .map((entry) => String(entry?.card || "").trim())
    .filter(Boolean))];
}

export function preloadPrivateDeckManifestArt(manifest) {
  const key = [
    String(manifest?.matchId || ""),
    Number(manifest?.owner),
    String(manifest?.decklistHash || manifest?.commitmentRoot || ""),
  ].join(":");
  if (preloadedPrivateDeckArtKeys.has(key)) return;
  const cardNames = privateDeckManifestCardNames(manifest);
  if (cardNames.length === 0) return;
  preloadedPrivateDeckArtKeys.add(key);
  const preload = () => {
    void preloadCardArt(cardNames, {
      versions: ["normal"],
      concurrency: 4,
    });
  };
  if (typeof globalThis.requestIdleCallback === "function") {
    globalThis.requestIdleCallback(preload, { timeout: 1000 });
  } else {
    globalThis.setTimeout(preload, 0);
  }
}

export function revealedOpeningStorageKey(indexKey) {
  return `${AUDIT_REVEALED_OPENING_STORAGE_PREFIX}:${String(indexKey || "")}`;
}

export function readStoredRevealedOpening(indexKey) {
  const storage = getPeerSessionStorage();
  if (!storage) return null;
  try {
    const raw = String(storage.getItem(revealedOpeningStorageKey(indexKey)) || "").trim();
    return raw ? JSON.parse(raw) : null;
  } catch {
    return null;
  }
}

export function writeStoredRevealedOpening(indexKey, opening) {
  const storage = getPeerSessionStorage();
  if (!storage || !indexKey || !opening) return;
  try {
    storage.setItem(revealedOpeningStorageKey(indexKey), JSON.stringify(opening));
  } catch {
    // Ignore localStorage failures.
  }
}

export function removeStoredRevealedOpening(indexKey) {
  const storage = getPeerSessionStorage();
  if (!storage || !indexKey) return;
  try {
    storage.removeItem(revealedOpeningStorageKey(indexKey));
  } catch {
    // Ignore localStorage failures.
  }
}

export function clearStoredRevealedOpeningsForMatch(matchId) {
  const storage = getPeerSessionStorage();
  if (!storage) return;
  const prefix = revealedOpeningStorageKey(`${String(matchId || "")}:`);
  try {
    const keys = [];
    for (let index = 0; index < storage.length; index += 1) {
      const key = storage.key(index);
      if (key && key.startsWith(prefix)) keys.push(key);
    }
    for (const key of keys) storage.removeItem(key);
  } catch {
    // Ignore localStorage failures.
  }
}

export function clearStoredRevealedOpeningsForMatchOwner(matchId, owner) {
  const storage = getPeerSessionStorage();
  if (!storage) return;
  const normalizedOwner = Number(owner);
  const prefix = revealedOpeningStorageKey(`${String(matchId || "")}:`);
  try {
    const keys = [];
    for (let index = 0; index < storage.length; index += 1) {
      const key = storage.key(index);
      if (!key || !key.startsWith(prefix)) continue;
      let parsed = null;
      try {
        parsed = JSON.parse(storage.getItem(key) || "null");
      } catch {
        parsed = null;
      }
      if (Number(parsed?.owner) === normalizedOwner) keys.push(key);
    }
    for (const key of keys) storage.removeItem(key);
  } catch {
    // Ignore localStorage failures.
  }
}

export function ziffleIdentityStorageKey(context) {
  return `${ZIFFLE_IDENTITY_STORAGE_PREFIX}:${String(context || "")}`;
}

export function readStoredZiffleIdentity(context) {
  const storage = getPeerSessionStorage();
  if (!storage) return null;
  try {
    const raw = String(storage.getItem(ziffleIdentityStorageKey(context)) || "").trim();
    return raw ? JSON.parse(raw) : null;
  } catch {
    return null;
  }
}

export function writeStoredZiffleIdentity(context, keyPair) {
  const storage = getPeerSessionStorage();
  if (!storage || !context || !keyPair) return;
  try {
    storage.setItem(ziffleIdentityStorageKey(context), JSON.stringify(keyPair));
  } catch {
    // Ignore localStorage failures.
  }
}

export function resolveReconnectPlayerIndex(session, lobbyId) {
  const localIndex = resolveLocalPlayerIndex(session);
  if (localIndex != null) return localIndex;
  return readStoredPlayerIndex(lobbyId);
}

export function writeStoredPlayerIndex(lobbyId, playerIndex) {
  const storage = getPeerSessionStorage();
  const normalizedIndex = normalizePlayerIndex(playerIndex);
  if (!storage || normalizedIndex == null) return;

  try {
    const normalizedLobbyId = String(lobbyId || "").trim();
    if (normalizedLobbyId) {
      storage.setItem(CURRENT_LOBBY_STORAGE_KEY, normalizedLobbyId);
    }
    storage.setItem(CURRENT_PLAYER_STORAGE_KEY, String(normalizedIndex + 1));
  } catch {
    // Ignore localStorage failures.
  }
}

export function clearStoredPlayerIndex(lobbyId = "") {
  const storage = getPeerSessionStorage();
  if (!storage) return;

  try {
    const storedLobbyId = String(storage.getItem(CURRENT_LOBBY_STORAGE_KEY) || "").trim();
    const normalizedLobbyId = String(lobbyId || "").trim();
    if (normalizedLobbyId && storedLobbyId && storedLobbyId !== normalizedLobbyId) return;
    storage.removeItem(CURRENT_PLAYER_STORAGE_KEY);
    storage.removeItem(CURRENT_LOBBY_STORAGE_KEY);
  } catch {
    // Ignore localStorage failures.
  }
}

export function createOfflinePeerId(lobbyId, playerIndex) {
  return `offline:${String(lobbyId || "lobby")}:${Number(playerIndex || 0)}`;
}

export function isOfflinePeerId(peerId) {
  return String(peerId || "").trim().startsWith("offline:");
}

export function firstOnlinePeerId(...peerIds) {
  for (const peerId of peerIds) {
    const normalized = String(peerId || "").trim();
    if (normalized && !isOfflinePeerId(normalized)) return normalized;
  }
  return "";
}

export function markHostPeerDisconnected(players, hostPeerId, lobbyId, fallbackHostIndex = null) {
  const hostId = String(hostPeerId || "").trim();
  const stableLobbyId = String(lobbyId || hostId || "").trim();
  const indexedPlayers = reindexPlayers(players);
  const matchedHost = indexedPlayers.some((player) => hostId && player.peerId === hostId);
  const fallbackIndex = matchedHost ? null : normalizePlayerIndex(fallbackHostIndex);

  return reindexPlayers(
    indexedPlayers.map((player) => {
      const isHost =
        (hostId && player.peerId === hostId) ||
        (fallbackIndex != null && Number(player.index) === fallbackIndex);
      if (!isHost) return player;
      return {
        ...player,
        peerId:
          stableLobbyId && player.peerId === stableLobbyId
            ? createOfflinePeerId(stableLobbyId, player.index)
            : player.peerId,
        connected: false,
        disconnectedAtMs: Date.now(),
        autoForfeitAtMs: Date.now() + DISCONNECT_AUTO_FORFEIT_MS,
      };
    })
  );
}

export function findNextHostPlayer(players) {
  return [...(players || [])]
    .filter((player) => player.connected !== false)
    .sort((left, right) => Number(left.index || 0) - Number(right.index || 0))[0] || null;
}

export function ensurePromotedLocalPlayer(players, session, lobbyId, localPlayerIndex) {
  const normalizedIndex = normalizePlayerIndex(localPlayerIndex);
  if (normalizedIndex == null) return reindexPlayers(players || []);

  const format = normalizeMatchFormat(session?.format);
  const deckSubmission = parseDeckSubmission(
    format,
    session?.localDeckText,
    session?.localCommanderText
  );
  const fallbackPlayer = withDeckState(
    {
      peerId:
        String(session?.localPeerId || "").trim()
        || createOfflinePeerId(lobbyId, normalizedIndex),
      name: sanitizePlayerName(
        session?.localName,
        normalizedIndex === 0 ? "Host" : `Player ${normalizedIndex + 1}`
      ),
      index: normalizedIndex,
      connected: true,
    },
    format,
    deckSubmission.deck,
    deckSubmission.commanders,
    deckSubmission.sideboard
  );

  let matched = false;
  const nextPlayers = reindexPlayers(players || []).map((player) => {
    if (Number(player.index) !== normalizedIndex) return player;
    matched = true;
    return withDeckState(
      {
        ...fallbackPlayer,
        ...player,
        peerId:
          String(session?.localPeerId || "").trim()
          || player.peerId
          || fallbackPlayer.peerId,
        name: sanitizePlayerName(player.name, fallbackPlayer.name),
        connected: true,
      },
      format,
      deckSubmission.deck,
      deckSubmission.commanders,
      deckSubmission.sideboard
    );
  });

  if (!matched) {
    nextPlayers.push(fallbackPlayer);
  }

  return reindexPlayers(
    nextPlayers.sort((left, right) => Number(left.index || 0) - Number(right.index || 0))
  );
}

export function toPublicPlayer(player) {
  return {
    peerId: player.peerId,
    currentPeerId: player.currentPeerId || undefined,
    name: player.name,
    index: player.index,
    auditPublicKey: String(player.auditPublicKey || ""),
    auditEncryptionPublicKey: String(player.auditEncryptionPublicKey || ""),
    playerGenesisSignature: player.playerGenesisSignature || null,
    deckAuditManifest: publicDeckManifest(player.deckAuditManifest),
    ziffleKey: player.ziffleKey || null,
    connected: player.connected !== false,
    disconnectedAtMs: player.disconnectedAtMs == null ? undefined : Number(player.disconnectedAtMs),
    autoForfeitAtMs: player.autoForfeitAtMs == null ? undefined : Number(player.autoForfeitAtMs),
    ready: Boolean(player.ready),
    deckCount: Number(player.deckCount || 0),
    sideboardCount: Number(player.sideboardCount || 0),
    commanderCount: Number(player.commanderCount || 0),
    ...openDecklistPlayerFields({
      deck: player.deck,
      sideboard: player.sideboard,
      commanders: player.commanders,
      deckSlotOpenings: player.deckSlotOpenings,
    }),
  };
}

export function toPublicPlayers(players) {
  return reindexPlayers(players).map(toPublicPlayer);
}

export function toLobbyPlayer(player) {
  return {
    ...toPublicPlayer(player),
  };
}

export function toLobbyPlayers(players) {
  return reindexPlayers(players).map(toLobbyPlayer);
}

export function markPlayerConnectionState(players, peerId, connected, observedAtMs = Date.now()) {
  const targetPeerId = String(peerId || "").trim();
  return reindexPlayers((players || []).map((player) => {
    if (
      String(player?.peerId || "").trim() !== targetPeerId
      && String(player?.currentPeerId || "").trim() !== targetPeerId
    ) return player;
    if (connected) {
      const {
        disconnectedAtMs: _disconnectedAtMs,
        autoForfeitAtMs: _autoForfeitAtMs,
        disconnectRemainingMs: _disconnectRemainingMs,
        ...rest
      } = player;
      return {
        ...rest,
        connected: true,
      };
    }
    const existingStartedAt = Number(player.disconnectedAtMs);
    const disconnectedAtMs = Number.isFinite(existingStartedAt) && existingStartedAt > 0
      ? existingStartedAt
      : Math.max(0, Math.floor(Number(observedAtMs || Date.now())));
    const autoForfeitAtMs = disconnectedAtMs + DISCONNECT_AUTO_FORFEIT_MS;
    return {
      ...player,
      connected: false,
      disconnectedAtMs,
      autoForfeitAtMs,
      disconnectRemainingMs: Math.max(0, autoForfeitAtMs - Date.now()),
    };
  }));
}

export function buildConnectionWarnings(session) {
  if (!session || session.mode === "idle") return [];
  const warnings = [];
  const localPeerId = String(session.localPeerId || "").trim();
  const hostPeerId = String(session.hostPeerId || "").trim();
  const nowMs = Date.now();
  for (const player of session.players || []) {
    if (player.connected !== false) continue;
    const peerId = String(player.peerId || "").trim();
    const disconnectedAtMs = Math.max(0, Math.floor(Number(player.disconnectedAtMs || nowMs)));
    const autoForfeitAtMs = Math.max(
      disconnectedAtMs,
      Math.floor(Number(player.autoForfeitAtMs || disconnectedAtMs + DISCONNECT_AUTO_FORFEIT_MS))
    );
    const remainingMs = Math.max(0, autoForfeitAtMs - nowMs);
    warnings.push({
      kind: peerId === hostPeerId ? "host_disconnected" : "peer_disconnected",
      playerIndex: Number(player.index || 0),
      peerId,
      name: sanitizePlayerName(player.name, `Player ${Number(player.index || 0) + 1}`),
      local: peerId === localPeerId,
      disconnectedAtMs,
      autoForfeitAtMs,
      remainingMs,
      expired: remainingMs <= 0,
      message:
        peerId === hostPeerId
          ? `${sanitizePlayerName(player.name, "Host")} disconnected`
          : `${sanitizePlayerName(player.name, `Player ${Number(player.index || 0) + 1}`)} disconnected`,
    });
  }
  return warnings;
}

export function withConnectionWarnings(session) {
  const next = {
    ...session,
    players: Array.isArray(session?.players) ? session.players : [],
  };
  const connectionWarnings = buildConnectionWarnings(next);
  return {
    ...next,
    connectionWarnings,
    disconnectTimeouts: connectionWarnings,
  };
}

export function ziffleRuntimeCommitment(deckHash, position) {
  return `ziffle:${String(deckHash || "")}:${Number(position)}`;
}

export function ziffleDeckHashFromCommitment(commitment) {
  const normalized = String(commitment || "");
  if (!normalized.startsWith("ziffle:")) return "";
  const lastColon = normalized.lastIndexOf(":");
  return lastColon > "ziffle:".length
    ? normalized.slice("ziffle:".length, lastColon)
    : "";
}

export function zifflePositionFromCommitment(commitment) {
  const normalized = String(commitment || "");
  if (!normalized.startsWith("ziffle:")) return null;
  const lastColon = normalized.lastIndexOf(":");
  if (lastColon <= "ziffle:".length) return null;
  const position = Number(normalized.slice(lastColon + 1));
  return Number.isSafeInteger(position) && position >= 0 ? position : null;
}

export function ziffleContextFromOpening(opening) {
  if (!opening || typeof opening !== "object") return "";
  const proof = opening.ziffleReveal || opening.ziffleProof || opening.positionOpeningProof || {};
  return String(
    opening.ziffleContext
    || opening.ziffle_context
    || proof.context
    || ""
  );
}

export function ziffleContextFromCeremony(ceremony) {
  return String(ceremony?.context || "");
}

export function ziffleContextForCommitment(context, sourceCommitment, targetCommitment) {
  const normalizedContext = String(context || "");
  if (!normalizedContext) return "";
  const sourceDeckHash = ziffleDeckHashFromCommitment(sourceCommitment);
  const targetDeckHash = ziffleDeckHashFromCommitment(targetCommitment);
  if (sourceDeckHash && targetDeckHash && sourceDeckHash !== targetDeckHash) {
    return "";
  }
  return normalizedContext;
}

export function ziffleIdentityPositionFromSources(...sources) {
  for (const source of sources || []) {
    if (!source || typeof source !== "object") continue;
    const commitment = String(source.commitment || "");
    const deckHash = ziffleDeckHashFromCommitment(commitment);
    if (!deckHash) continue;
    const committedPosition = zifflePositionFromCommitment(commitment);
    const sourceSlot = Number(source.slot);
    const position = committedPosition != null
      ? committedPosition
      : Number.isSafeInteger(sourceSlot) && sourceSlot >= 0
        ? sourceSlot
        : null;
    if (position == null) continue;
    return {
      position,
      positionCommitment: commitment,
      deckHash,
    };
  }
  return null;
}

export function hiddenSourceZone(source) {
  return String(source?.zone ?? source?.hiddenZone ?? source?.hidden_zone ?? "").trim().toLowerCase();
}

export function zifflePublicPositionFromSources(...sources) {
  const knownZones = (sources || [])
    .map(hiddenSourceZone)
    .filter(Boolean);
  const useAsPosition = !knownZones.some((zone) => zone !== "library");
  for (const source of sources || []) {
    if (!source || typeof source !== "object") continue;
    const publicCommitment = String(
      source.publicCommitment
      ?? source.public_commitment
      ?? ""
    );
    const deckHash = ziffleDeckHashFromCommitment(publicCommitment);
    if (!deckHash) continue;
    const committedPosition = zifflePositionFromCommitment(publicCommitment);
    const publicSlotRaw = source.publicSlot ?? source.public_slot ?? null;
    const publicSlot = publicSlotRaw == null ? null : Number(publicSlotRaw);
    const position = committedPosition ?? publicSlot;
    if (!Number.isSafeInteger(position) || position < 0) continue;
    return {
      position,
      publicSlot: position,
      positionCommitment: publicCommitment,
      publicCommitment,
      deckHash,
      useAsPosition,
    };
  }
  return null;
}

export function withPinnedPublicZifflePosition(opening, publicPosition) {
  if (!opening || !publicPosition) return opening;
  return {
    ...opening,
    publicSlot: Number(publicPosition.publicSlot ?? publicPosition.position),
    publicCommitment: String(publicPosition.publicCommitment || publicPosition.positionCommitment || ""),
    position: Number(publicPosition.position),
    positionCommitment: String(publicPosition.positionCommitment || publicPosition.publicCommitment || ""),
  };
}

export function openingHasZifflePosition(opening) {
  if (!opening || typeof opening !== "object") return false;
  const positionCommitment = String(
    opening.positionCommitment
    || opening.position_commitment
    || opening.publicCommitment
    || opening.public_commitment
    || ""
  );
  const publicSlot = Number(opening.publicSlot ?? opening.public_slot);
  return Boolean(ziffleDeckHashFromCommitment(positionCommitment))
    && (
      opening.position != null
      || (Number.isSafeInteger(publicSlot) && publicSlot >= 0)
      || zifflePositionFromCommitment(positionCommitment) != null
    );
}

export function requirementHasZifflePosition(requirement) {
  if (!requirement || typeof requirement !== "object") return false;
  return Boolean(
    ziffleDeckHashFromCommitment(requirement.commitment)
    || ziffleDeckHashFromCommitment(requirement.positionCommitment)
    || ziffleDeckHashFromCommitment(requirement.position_commitment)
    || zifflePublicPositionFromSources(requirement)?.useAsPosition
  );
}

export function exportedOpeningHasZifflePosition(exported) {
  if (!exported || typeof exported !== "object") return false;
  return Boolean(
    ziffleDeckHashFromCommitment(exported.commitment)
    || ziffleDeckHashFromCommitment(exported.positionCommitment)
    || ziffleDeckHashFromCommitment(exported.position_commitment)
    || ziffleDeckHashFromCommitment(exported.publicCommitment)
    || ziffleDeckHashFromCommitment(exported.public_commitment)
  );
}

export function ziffleCeremonyForOpeningProof(proof, fallbackCeremony = null) {
  const beforeOrder = normalizeShuffleOrder(proof?.beforeOrder ?? proof?.before_order);
  const afterOrder = normalizeShuffleOrder(proof?.afterOrder ?? proof?.after_order);
  const fallbackBefore = normalizeShuffleOrder(
    fallbackCeremony?.beforeOrder ?? fallbackCeremony?.before_order
  );
  const fallbackAfter = normalizeShuffleOrder(
    fallbackCeremony?.afterOrder ?? fallbackCeremony?.after_order
  );
  const hasProofOrder = beforeOrder.length > 0 && afterOrder.length > 0;
  return {
    ...(fallbackCeremony || {}),
    owner: Number(proof?.owner ?? fallbackCeremony?.owner),
    deckCount: Number(proof?.deckCount || fallbackCeremony?.deckCount || 0),
    context: String(proof?.context || fallbackCeremony?.context || ""),
    keyContext: String(proof?.keyContext || fallbackCeremony?.keyContext || proof?.context || ""),
    keys: Array.isArray(proof?.keys) && proof.keys.length > 0
      ? cloneMultiplayerPayload(proof.keys)
      : cloneMultiplayerPayload(fallbackCeremony?.keys || []),
    steps: Array.isArray(proof?.steps) && proof.steps.length > 0
      ? cloneMultiplayerPayload(proof.steps)
      : cloneMultiplayerPayload(fallbackCeremony?.steps || []),
    deckHash: String(proof?.deckHash || fallbackCeremony?.deckHash || ""),
    beforeOrder: hasProofOrder ? beforeOrder : fallbackBefore,
    before_order: hasProofOrder ? beforeOrder : fallbackBefore,
    afterOrder: hasProofOrder ? afterOrder : fallbackAfter,
    after_order: hasProofOrder ? afterOrder : fallbackAfter,
    authenticatedOrder: hasProofOrder
      ? true
      : fallbackCeremony?.authenticatedOrder === true,
  };
}

export function redactedMatchPayloadForPeer(payload, peerId, playerIndex = null) {
  if (!payload || typeof payload !== "object") return payload;
  if (payload.openDecklists) return cloneMultiplayerPayload(payload);
  const target = (payload.players || []).find((player) => player.peerId === peerId)
    || (payload.players || []).find((player) =>
      normalizePlayerIndex(player?.index) === normalizePlayerIndex(playerIndex)
    );
  const targetIndex = normalizePlayerIndex(target?.index);
  const redactLists = (lists) => {
    if (!Array.isArray(lists)) return lists;
    return lists.map((cards, index) =>
      targetIndex != null && index === targetIndex ? sanitizeCardList(cards) : []
    );
  };
  return {
    ...cloneMultiplayerPayload(payload),
    decks: redactLists(payload.decks),
    sideboards: redactLists(payload.sideboards),
  };
}

export function validationDecksForMatchPayload(payload) {
  if (payload?.openDecklists && Array.isArray(payload.players)) {
    return payload.players.map((player) => sanitizeCardList(player.deck));
  }
  return payload?.decks;
}

export function validationSideboardsForMatchPayload(payload) {
  if (payload?.openDecklists && Array.isArray(payload.players)) {
    return payload.players.map((player) => sanitizeCardList(player.sideboard));
  }
  return payload?.sideboards;
}

export function validationCommandersForMatchPayload(payload) {
  if (payload?.openDecklists && Array.isArray(payload.players)) {
    return payload.players.map((player) => sanitizeCardList(player.commanders));
  }
  return payload?.commanders;
}

export function validationPlanarDecksForMatchPayload(payload) {
  if (normalizeMatchFormat(payload?.format) !== MATCH_FORMAT_PLANECHASE) {
    return undefined;
  }
  if (payload?.openDecklists && Array.isArray(payload.players)) {
    return payload.players.map((player) =>
      sanitizeCardList(player.commanders).map((name) => ({ name }))
    );
  }
  return payload?.planarDecks;
}

export function canHostedMatchStart(session) {
  const playerCount = session.players.length;
  const lobbyReady = (
    session.role === "host" &&
    !session.matchStarted &&
    session.mode !== "starting" &&
    playerCount === session.desiredPlayers &&
    isCurrentAuditPlayerCount(playerCount) &&
    session.players.every((player) => player.connected !== false && player.ready)
  );
  if (!lobbyReady) return false;
  if (
    normalizeMatchFormat(session.format) === MATCH_FORMAT_PLANECHASE
    && !isTrustedMultiplayerSecurityMode(sessionSecurityMode(session))
  ) {
    return false;
  }
  if (isTrustedMultiplayerSecurityMode(sessionSecurityMode(session))) return true;
  return session.players.every((player) =>
    player.ziffleKey && isSupportedZiffleDeckCount(player.deckCount)
  );
}

export function playerCryptoSeatBindingReady(player) {
  if (!player) return false;
  const index = normalizePlayerIndex(player.index);
  if (index == null) return false;
  const manifest = publicDeckManifest(player.deckAuditManifest);
  const zifflePlayer = normalizePlayerIndex(player.ziffleKey?.player);
  const signer = normalizePlayerIndex(player.playerGenesisSignature?.signer);
  const deck = sanitizeCardList(player.deck);
  const sideboard = sanitizeCardList(player.sideboard);
  const commanders = sanitizeCardList(player.commanders);
  const deckSlotOpenings = sanitizeDeckSlotOpenings(player.deckSlotOpenings);
  return (
    manifest?.owner === index
    && String(manifest.matchId || "")
    && deck.length === Number(manifest.deckCount || 0)
    && sideboard.length === Number(manifest.sideboardCount || 0)
    && commanders.length === Number(manifest.commanderCount || 0)
    && deckSlotOpenings.length === deck.length
    && zifflePlayer === index
    && signer === index
    && Boolean(player.auditPublicKey)
    && Boolean(player.auditEncryptionPublicKey)
  );
}

export function playerMatchesPresentedAuditIdentity(player, auditPublicKey, auditEncryptionPublicKey) {
  const expectedAuditKey = String(player?.auditPublicKey || "").trim();
  const presentedAuditKey = String(auditPublicKey || "").trim();
  const expectedEncryptionKey = String(player?.auditEncryptionPublicKey || "").trim();
  const presentedEncryptionKey = String(auditEncryptionPublicKey || "").trim();
  return (
    Boolean(expectedAuditKey)
    && Boolean(presentedAuditKey)
    && presentedAuditKey === expectedAuditKey
    && Boolean(expectedEncryptionKey)
    && Boolean(presentedEncryptionKey)
    && presentedEncryptionKey === expectedEncryptionKey
  );
}

export async function waitForCryptoSeatBindingsFromSession(sessionRef, timeoutMs = 10000) {
  const started = Date.now();
  while (Date.now() - started < timeoutMs) {
    const players = reindexPlayers(sessionRef.current?.players || []);
    if (players.length > 0 && players.every(playerCryptoSeatBindingReady)) {
      return players;
    }
    await sleep(100);
  }
  return reindexPlayers(sessionRef.current?.players || []);
}

export function compactMatchValidationError(error) {
  return String(error || "")
    .replace(/\s+/g, " ")
    .trim();
}

export function summarizeMatchValidationIssues(issues) {
  const entries = Array.isArray(issues)
    ? issues.filter((issue) => issue && typeof issue === "object")
    : [];

  if (entries.length === 0) {
    return {
      status: "Match start blocked: submitted decks contain invalid cards.",
      notice: "Submitted decks contain invalid cards.",
    };
  }

  const first = entries[0];
  const playerName = String(first.playerName || "").trim()
    || `Player ${Number(first.playerIndex || 0) + 1}`;
  const section = String(first.section || "deck").trim();
  const cardName = String(first.cardName || "").trim() || "Unknown card";
  const reason = compactMatchValidationError(first.error) || "could not be loaded";
  const extraCount = entries.length - 1;

  const status = [
    `Match start blocked: ${playerName} ${section} includes "${cardName}"`,
    `(${reason})`,
    extraCount > 0 ? `+${extraCount} more issue${extraCount === 1 ? "" : "s"}.` : "",
  ]
    .filter(Boolean)
    .join(" ");

  const lines = entries.slice(0, 8).map((issue) => {
    const issuePlayerName = String(issue.playerName || "").trim()
      || `Player ${Number(issue.playerIndex || 0) + 1}`;
    const issueSection = String(issue.section || "deck").trim();
    const issueCardName = String(issue.cardName || "").trim() || "Unknown card";
    const issueReason = compactMatchValidationError(issue.error) || "could not be loaded";
    return `- ${issuePlayerName} ${issueSection}: ${issueCardName} (${issueReason})`;
  });
  if (entries.length > lines.length) {
    const hiddenCount = entries.length - lines.length;
    lines.push(`- ${hiddenCount} more issue${hiddenCount === 1 ? "" : "s"}`);
  }

  return {
    status,
    notice: [
      "Submitted decks contain cards the engine cannot start with:",
      ...lines,
    ].join("\n"),
  };
}
