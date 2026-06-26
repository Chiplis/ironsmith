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
  evaluateLobbyDeckSubmission,
  normalizeMatchFormat,
  parseCommanderList,
  parseDeckList,
  parseDeckPrintPreferences,
  parseSideboardList,
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

const PROTOCOL_VERSION = CURRENT_AUDIT_PROTOCOL_VERSION;
const DEFAULT_OPENING_HAND_SIZE = 7;
const INITIAL_AUDIT_STATE_HASH = "0".repeat(64);
const INITIAL_MATCH_CLOCK_HASH = "0".repeat(64);
const PEER_OPEN_TIMEOUT_MS = 10000;
const PEER_CONNECT_TIMEOUT_MS = 15000;
const PEER_HEARTBEAT_INTERVAL_MS = 3000;
const PEER_HEARTBEAT_TIMEOUT_MS = 10000;
const DEFAULT_PLAYER_CLOCK_MS = 15 * 60 * 1000;
const MATCH_CLOCK_TICK_MS = 1000;
const MATCH_CLOCK_CLAIM_SKEW_MS = 2000;
const MATCH_CLOCK_ELAPSED_UNDERREPORT_SKEW_MS = 15000;
const ACTION_SUBMISSION_IDLE_WAIT_MS = 5000;
const PROTOCOL_RESPONSE_TIMEOUT_VOTE_WAIT_MS = 15000;
const MAX_PENDING_ACTION_INTENT_MS = pendingActionIntentHardTimeoutMs(PROTOCOL_RESPONSE_TIMEOUT_MS);
const MATCH_CLOCK_AUDIT_TYPE = "match_clock_v1";
const MATCH_CLOCK_POLICY_TYPE = "per_player_match_clock_v1";
const MATCH_CLOCK_AUDIT_DOMAIN = "ironsmith-match-clock-audit-v1";
const TIMEOUT_VOTE_DOMAIN = "ironsmith-match-timeout-vote-v1";
const ACTION_INTENT_DOMAIN = "ironsmith-action-intent-v1";
const RECONNECT_PROOF_DOMAIN = "ironsmith-reconnect-proof-v1";
const AUDIT_DECK_MANIFEST_STORAGE_PREFIX = "ironsmith.auditDeckManifest.v1";
const AUDIT_REVEALED_OPENING_STORAGE_PREFIX = "ironsmith.auditRevealedOpening.v1";
const ACTION_QUORUM_VOTE_STORAGE_PREFIX = "ironsmith.actionQuorumVote.v1";
const AUDIT_IDENTITY_STORAGE_KEY = "ironsmith.auditIdentity.v1";
const ZIFFLE_IDENTITY_STORAGE_PREFIX = "ironsmith.ziffleIdentity.v1";
const preloadedPrivateDeckArtKeys = new Set();
const CURRENT_PLAYER_STORAGE_KEY = "currentPlayer";
const CURRENT_LOBBY_STORAGE_KEY = "currentLobby";
const MATCH_SEED_OFFSET = 0xcbf29ce484222325n;
const MATCH_SEED_PRIME = 0x100000001b3n;
const MATCH_SEED_MASK = 0xffffffffffffffffn;
const ZIFFLE_OPENING_PREVIEW_BATCH_SIZE = 8;
const matchSeedEncoder = new TextEncoder();
const sleep = (ms) => new Promise((resolve) => globalThis.setTimeout(resolve, ms));

function matchPayloadSecurityMode(payload, fallback = MULTIPLAYER_SECURITY_TRUSTED) {
  if (payload && Object.prototype.hasOwnProperty.call(payload, "securityMode")) {
    return normalizeMultiplayerSecurityMode(payload.securityMode, fallback);
  }
  if (payload?.genesis || (Array.isArray(payload?.ziffleCeremonies) && payload.ziffleCeremonies.length > 0)) {
    return MULTIPLAYER_SECURITY_VERIFIED;
  }
  return normalizeMultiplayerSecurityMode(fallback);
}

function sessionSecurityMode(session, fallback = MULTIPLAYER_SECURITY_TRUSTED) {
  return normalizeMultiplayerSecurityMode(session?.securityMode, fallback);
}

function sequencedActionSecurityMode(message, session) {
  return normalizeMultiplayerSecurityMode(
    message?.securityMode,
    sessionSecurityMode(session, MULTIPLAYER_SECURITY_VERIFIED)
  );
}

function normalizeActionOpeningPreview(value) {
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

function actionOpeningPreviewKey(value) {
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

function mergeActionOpeningPreviews(existing = [], additions = [], limit = 240) {
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

function chunkList(values = [], size = 1) {
  const chunkSize = Math.max(1, Math.floor(Number(size) || 1));
  const chunks = [];
  for (let index = 0; index < values.length; index += chunkSize) {
    chunks.push(values.slice(index, index + chunkSize));
  }
  return chunks;
}

function actionOpeningPreviewFromOpening(opening, options = {}) {
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

function notifyOpeningBuilt(options = {}, opening, metadata = {}) {
  if (typeof options?.onOpeningBuilt !== "function") return;
  try {
    options.onOpeningBuilt(opening, metadata);
  } catch {
    // Progress previews must never affect action generation.
  }
}

function createEmptyState() {
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

function nowMonotonicMs() {
  const now = globalThis.performance?.now?.();
  return Number.isFinite(now) ? now : Date.now();
}

function createMatchClockSnapshot({
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

function actionTimerSnapshotFromMatchClock(matchClock) {
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

function toErrorMessage(err, fallback = "Action rejected") {
  const message = String(err?.message || err || "").trim();
  return message || fallback;
}

function sanitizePlayerName(raw, fallback = "Player") {
  const trimmed = String(raw || "").trim();
  return trimmed || fallback;
}

function mixMatchSeedBytes(hash, bytes) {
  let next = hash;
  for (const byte of bytes) {
    next ^= BigInt(byte);
    next = (next * MATCH_SEED_PRIME) & MATCH_SEED_MASK;
  }
  next ^= 0xffn;
  return (next * MATCH_SEED_PRIME) & MATCH_SEED_MASK;
}

function mixMatchSeedString(hash, value) {
  return mixMatchSeedBytes(hash, matchSeedEncoder.encode(String(value ?? "")));
}

function mixMatchSeedNumber(hash, value) {
  return mixMatchSeedString(hash, Number(value ?? 0));
}

function mixMatchSeedCardLists(hash, lists) {
  let next = mixMatchSeedNumber(hash, lists?.length ?? 0);
  for (const cards of lists || []) {
    next = mixMatchSeedNumber(next, cards.length);
    for (const card of cards) {
      next = mixMatchSeedString(next, card);
    }
  }
  return next;
}

function createMatchSeed({ players, format, decks, commanders, sideboards, startingLife, openingHandSize }) {
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
  hash = mixMatchSeedCardLists(hash, sideboards);

  const seed = Number(hash & BigInt(Number.MAX_SAFE_INTEGER));
  return seed > 0 ? seed : 1;
}

function readPeerEnv(name) {
  const value = import.meta.env?.[name];
  return typeof value === "string" ? value.trim() : "";
}

function parseBooleanEnv(value, fallback) {
  if (!value) return fallback;
  const normalized = value.trim().toLowerCase();
  if (["1", "true", "yes", "on"].includes(normalized)) return true;
  if (["0", "false", "no", "off"].includes(normalized)) return false;
  return fallback;
}

function parseNumberEnv(value, fallback) {
  if (!value) return fallback;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : fallback;
}

function formatPeerError(err, fallback = "Peer connection failed") {
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

function isRecoverablePeerError(err) {
  const type = String(err?.type || "").trim();
  return (
    type === "network" ||
    type === "socket-error" ||
    type === "socket-closed" ||
    type === "disconnected"
  );
}

function parseIceConfig() {
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

function describePeerServer(options) {
  const host = options?.host || "0.peerjs.com";
  const port = options?.port || 443;
  return `${host}:${port}`;
}

function buildPeerOptions() {
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

function buildPeerHeartbeatConfig() {
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

function normalizeMatchClockPolicy(policy = {}) {
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

function buildMatchClockConfig() {
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

function normalizeMatchClockRemaining(remaining, playerCount, initialMs) {
  const count = Math.max(0, Number(playerCount || 0));
  return Array.from({ length: count }, (_, index) => {
    const value = Number(Array.isArray(remaining) ? remaining[index] : undefined);
    return Math.max(0, Number.isFinite(value) ? Math.floor(value) : Math.floor(Number(initialMs || 0)));
  });
}

function matchClockPolicyFromPayload(payload, fallbackPolicy = buildMatchClockConfig()) {
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

function matchClockPolicyPayload(policy = {}) {
  const normalized = normalizeMatchClockPolicy(policy);
  return {
    type: MATCH_CLOCK_POLICY_TYPE,
    initialMs: normalized.initialMs,
    graceMs: normalized.graceMs,
  };
}

function isForfeitCommand(command) {
  return command?.type === "forfeit_player";
}

// Commands that are not engine decision commands: the WASM `UiCommand` enum
// cannot deserialize them, so they must never be routed through engine dispatch
// (previewCryptoRequirements/dispatch). They produce no hidden-card material —
// `cancel_decision` is a local rollback and `forfeit_player` removes a seat.
function isNonDispatchSyncCommand(command) {
  const type = String(command?.type || "");
  return type === "cancel_decision" || type === "forfeit_player";
}

function isActionTimeoutForfeitCommand(command) {
  const reason = String(command?.reason || "");
  return isForfeitCommand(command)
    && (
      reason === "peer_claimed_match_clock_timeout"
      || reason === "match_clock_timeout"
      || reason === "peer_claimed_action_timeout"
      || reason === "action_timeout"
    );
}

function isDisconnectTimeoutForfeitCommand(command) {
  return isForfeitCommand(command)
    && isDisconnectForfeitReason(command?.reason);
}

function isProtocolResponseTimeoutForfeitCommand(command) {
  return isForfeitCommand(command)
    && isProtocolResponseTimeoutForfeitReason(command?.reason);
}

function isSelfForfeitCommand(command, actorIndex) {
  return isForfeitCommand(command)
    && !isActionTimeoutForfeitCommand(command)
    && !isDisconnectTimeoutForfeitCommand(command)
    && !isProtocolResponseTimeoutForfeitCommand(command)
    && Number(command.player) === Number(actorIndex);
}

function normalizedTurnToken(value) {
  return String(value || "").trim().toLowerCase().replace(/[\s_-]+/g, "");
}

function isMainPhaseName(phase) {
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

function isSorcerySpeedForfeitState(uiState, playerIndex) {
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

function disconnectCertificateFromCommand(command) {
  const certificate = command?.disconnect_certificate || command?.disconnectCertificate || null;
  return certificate && typeof certificate === "object" ? certificate : null;
}

function disconnectForfeitVoteThreshold(nonTargetPlayerCount) {
  const count = Math.max(0, Number(nonTargetPlayerCount || 0));
  return count;
}

function timeoutVotePayload({
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

function timeoutCertificateFromCommand(command) {
  const certificate = command?.timeout_certificate || command?.timeoutCertificate || null;
  return certificate && typeof certificate === "object" ? certificate : null;
}

function expectedTimeoutVoters(players = [], forfeitedPlayer) {
  return reindexPlayers(players)
    .map((player) => Number(player.index))
    .filter((index) => Number.isInteger(index) && index !== Number(forfeitedPlayer))
    .sort((left, right) => left - right);
}

function isOwnerPrivateViewRequirement(requirement) {
  const type = String(requirement?.type || "");
  return (
    (type === "private_open" || type === "private_view_window")
    && requirement?.owner != null
    && requirement?.viewer != null
    && Number(requirement.owner) === Number(requirement.viewer)
  );
}

function hasOnlyOwnerPrivateViewRequirements(requirements = []) {
  return (
    Array.isArray(requirements)
    && requirements.length > 0
    && requirements.every(isOwnerPrivateViewRequirement)
  );
}

function shouldRequestRemoteCryptoPreview(command, state, previewedRequirements = []) {
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

function commandMayProducePostApplyOpenings(command, state, previewedRequirements = []) {
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

function isUnauthorizedAddCardCommand(command) {
  return command?.type === "add_card_to_zone";
}

function isRejectedActionCheatReason(reason) {
  const normalized = String(reason || "").toLowerCase();
  return normalized.includes("invalid priority action ref")
    || normalized.includes("priority action is no longer available")
    || normalized.includes("does not match pending")
    || normalized.includes("action is no longer available");
}

function matchClockActivePlayerFromState(uiState) {
  const decision = uiState?.decision || null;
  if (!decision || uiState?.game_over || decision.player == null) return null;
  const player = Number(decision.player);
  return Number.isInteger(player) && player >= 0 ? player : null;
}

function debitMatchClockRemaining(remaining, activePlayerIndex, elapsedMs) {
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

async function matchClockAuditHash(clock) {
  if (!clock || typeof clock !== "object") return "";
  const payload = { ...clock };
  delete payload.clockHash;
  return sha256Hex(canonicalJson({
    domain: MATCH_CLOCK_AUDIT_DOMAIN,
    clock: payload,
  }));
}

function playerNameForIndex(players, playerIndex) {
  const target = (players || []).find((player) => Number(player?.index) === Number(playerIndex));
  return String(target?.name || `Player ${Number(playerIndex) + 1}`);
}

function publicCheckpointWinner(checkpoint) {
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

function buildExportedMatchOutcome({
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

function safeSend(conn, payload) {
  if (!conn || conn.open === false) return;
  try {
    conn.send(payload);
  } catch {
    // PeerJS can report stale connections as open until the next send.
  }
}

function createPeer(peerId, options) {
  const requestedPeerId = String(peerId || "").trim();
  return requestedPeerId ? new Peer(requestedPeerId, options) : new Peer(options);
}

function connectionHeartbeatKey(kind, peerId) {
  return `${kind}:${String(peerId || "")}`;
}

function cloneMultiplayerPayload(value) {
  if (value == null) return value;
  return JSON.parse(JSON.stringify(value));
}

function stripTransientZifflePositionOpeningFields(opening) {
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

function compactZiffleCeremonyForDiagnostics(ceremony) {
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

function ziffleKeyContextForCeremony(ceremony) {
  return String(ceremony?.keyContext || ceremony?.context || "");
}

function compactZiffleDiagnosticsJson(diagnostics) {
  try {
    return JSON.stringify(diagnostics);
  } catch {
    return "{}";
  }
}

function ziffleDiagnosticNoticeBody(message, diagnostics) {
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

function collectCommandObjectIds(command, output = new Set(), uiState = null) {
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

function cryptoRequirementsFromState(state) {
  return Array.isArray(state?.crypto_requirements)
    ? state.crypto_requirements
    : Array.isArray(state?.cryptoRequirements)
      ? state.cryptoRequirements
      : [];
}

function priorityActionKindForCommand(command, uiState = null) {
  const direct = String(command?.action_ref?.kind || command?.actionRef?.kind || "").trim();
  if (direct) return direct;
  const index = Number(command?.action_index ?? command?.actionIndex);
  const actions = uiState?.decision?.actions;
  if (!Number.isSafeInteger(index) || !Array.isArray(actions)) return "";
  return String(actions[index]?.action_ref?.kind || actions[index]?.actionRef?.kind || "").trim();
}

function isPregameNonShuffleCommand(command, uiState = null) {
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

function filterCryptoRequirementsForCommand(command, uiState, requirements = []) {
  const list = Array.isArray(requirements) ? requirements : [];
  if (!isPregameNonShuffleCommand(command, uiState)) return list;
  return list.filter((requirement) =>
    String(requirement?.type || requirement?.requirement_type || "") !== "verifiable_shuffle"
  );
}

function openingMatchesRequirement(opening, requirement) {
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

function cachedOpeningMatchesZifflePosition(opening, position, positionCommitment) {
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

function ziffleOpeningLinkKey(opening) {
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

function openingShuffleSourceId(opening) {
  const id = Number(
    opening?.shuffleObjectId
    ?? opening?.shuffle_object_id
  );
  return Number.isSafeInteger(id) && id >= 0 ? id : null;
}

function normalizeMergedZiffleOpeningShuffleIds(openings = []) {
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

function mergeAuditOpenings(...openingLists) {
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

function hasPostTimedOpenings(...openingLists) {
  return openingLists.flat().some((opening) =>
    String(opening?.timing || "pre") === "post"
  );
}

function mergePrivateViewProofs(...proofLists) {
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

function missingRemotePublicOpenRequirements(requirements = [], material = {}, localSeat = null) {
  return (requirements || []).filter((requirement) => {
    if (String(requirement?.type || "") !== "public_open") return false;
    const owner = Number(requirement.owner);
    if (!Number.isInteger(owner) || owner === Number(localSeat)) return false;
    return !(material.openings || []).some((opening) =>
      openingMatchesRequirement(opening, requirement)
    );
  });
}

function expectedLocalPublicOpeningPreviewCount(requirements = [], localSeat = null) {
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

function shuffleProofMatchesRequirement(proof, requirement) {
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

function shuffleProofSameOwnerZone(proof, requirement) {
  if (!proof || !requirement) return false;
  return (
    Number(proof?.owner) === Number(requirement.owner)
    && String(proof?.zone || "library") === String(requirement.zone || "library")
  );
}

function mergeShuffleProofs(...proofLists) {
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

function missingShuffleRequirements(requirements = [], shuffleProofs = []) {
  return (requirements || []).filter((requirement) =>
    String(requirement?.type || "") === "verifiable_shuffle"
    && !(shuffleProofs || []).some((proof) => shuffleProofMatchesRequirement(proof, requirement))
  );
}

function normalizeShuffleOrder(value) {
  return (Array.isArray(value) ? value : [])
    .map((entry) => Number(entry))
    .filter((entry) => Number.isSafeInteger(entry) && entry >= 0);
}

function playerLibraryOrderFromCheckpoint(checkpoint, owner) {
  const normalizedOwner = Number(owner);
  if (!Number.isSafeInteger(normalizedOwner) || normalizedOwner < 0) return [];
  const player = (checkpoint?.players || []).find((entry) =>
    Number(entry?.id ?? entry?.index ?? entry?.player) === normalizedOwner
  );
  return normalizeShuffleOrder(player?.library);
}

function projectShuffleOrderToCurrentLibrary(order, currentLibrary) {
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

function sameShuffleOrder(left, right) {
  const normalizedLeft = normalizeShuffleOrder(left);
  const normalizedRight = normalizeShuffleOrder(right);
  if (normalizedLeft.length !== normalizedRight.length) return false;
  return normalizedLeft.every((entry, index) => entry === normalizedRight[index]);
}

function shuffleOrderIdMap(proofBeforeOrder, localBeforeOrder) {
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

function localizeShuffleOrder(order, idMap) {
  const normalized = normalizeShuffleOrder(order);
  if (!idMap) return normalized;
  const localized = [];
  for (const entry of normalized) {
    if (!idMap.has(entry)) return null;
    localized.push(idMap.get(entry));
  }
  return localized;
}

function shuffleProofWithRequirementOrder(proof, requirement) {
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

function alignShuffleProofsWithRequirements(shuffleProofs = [], requirements = []) {
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

function wasmObjectIdArg(objectId) {
  const normalized = Number(objectId);
  if (!Number.isSafeInteger(normalized) || normalized < 0) {
    throw new Error(`Invalid object id: ${objectId}`);
  }
  return BigInt(normalized);
}

function actionRefObjectId(actionRef) {
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

function actionRefWithObjectId(actionRef, objectId) {
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

function hiddenOpeningMatchesExport(opening, exported) {
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

function hiddenCardMetadataForObjectFromCheckpoint(checkpoint, objectId) {
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

function hiddenMetadataMatchesZifflePosition(metadata, position, positionCommitment = "") {
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

function hiddenObjectIdForOpeningFromCheckpoint(checkpoint, opening) {
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

function hiddenObjectIdForHiddenRefFromCheckpoint(checkpoint, hiddenRef) {
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

function checkpointObjectForId(checkpoint, objectId) {
  const normalized = Number(objectId);
  if (!Number.isSafeInteger(normalized) || normalized <= 0) return null;
  return (checkpoint?.objects || []).find((entry) => Number(entry?.id) === normalized) || null;
}

function checkpointObjectHiddenCard(object) {
  return object?.hiddenCard || object?.hidden_card || null;
}

function checkpointObjectName(object) {
  return String(object?.name || object?.identity?.name || "").trim();
}

function checkpointObjectIsRedactedHidden(object) {
  const name = checkpointObjectName(object);
  return !name || name === "Hidden Card";
}

function knownCheckpointObjectMatchesOpening(object, opening) {
  if (!object || checkpointObjectHiddenCard(object)) return false;
  const objectName = checkpointObjectName(object);
  const openingCard = String(opening?.card || "").trim();
  return Boolean(objectName) && (!openingCard || objectName === openingCard);
}

function canonicalMultiplayerPayload(value) {
  return canonicalJson(cloneMultiplayerPayload(value));
}

function peerSyncPerfNow() {
  return globalThis.performance && typeof globalThis.performance.now === "function"
    ? globalThis.performance.now()
    : Date.now();
}

function payloadSizeBytes(value) {
  try {
    return new TextEncoder().encode(canonicalMultiplayerPayload(value)).length;
  } catch {
    return null;
  }
}

function summarizePeerCommand(command) {
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

function summarizeCryptoRequirementsForPerf(requirements = []) {
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

function summarizeCryptoMaterialForPerf(material = {}) {
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

function summarizeSequencedActionForPerf(message = {}) {
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

function recordPeerSyncPerf(label, payload = {}) {
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

async function timePeerSyncPhase(label, payload, task) {
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

function signedActionIntentPayload(intent = {}) {
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

function actionIntentKey(intent = {}) {
  const payload = signedActionIntentPayload(intent);
  return [
    payload.matchId,
    payload.seq,
    payload.actorIndex,
  ].join(":");
}

function actionIntentFingerprint(intent = {}) {
  return canonicalMultiplayerPayload(signedActionIntentPayload(intent));
}

function reconnectProofPayload(proof = {}) {
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

function isProtocolResponseWaitTimeout(err) {
  return String(err?.message || err || "").startsWith("Timed out waiting for ");
}

function protocolResponseTimeoutClaimFromError(err) {
  return err?.protocolResponseTimeoutClaim || null;
}

function enqueueAsync(queueRef, task) {
  const next = queueRef.current.catch(() => undefined).then(task);
  queueRef.current = next.catch(() => undefined);
  return next;
}

function sanitizeCardList(cards) {
  if (!Array.isArray(cards)) return [];
  return cards
    .map((card) => String(card || "").trim())
    .filter(Boolean);
}

function sanitizeDeckSlotOpenings(openings) {
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

function deckSlotOpeningsForManifest(manifest) {
  return sanitizeDeckSlotOpenings(manifest?.slotSecrets);
}

function openDecklistPlayerFields({
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

function parseDeckSubmission(format, deckText, commanderText = "") {
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

function withDeckState(player, format, deck, commanders = [], sideboard = []) {
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

function buildRematchStateFromPayload(payload, localPeerId, readyOverrides = new Map()) {
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

function rematchPlayersReady(players) {
  const entries = Array.isArray(players) ? players : [];
  return entries.length > 0 && entries.every((player) => (
    player.connected !== false && player.ready
  ));
}

function reindexPlayers(players) {
  return players.map((player, index) => ({ ...player, index }));
}

function normalizePlayerIndex(value) {
  if (value === null || value === undefined || value === "") return null;
  const parsed = Number(value);
  if (!Number.isInteger(parsed) || parsed < 0 || parsed > 3) return null;
  return parsed;
}

function resolveLocalPlayerIndex(session) {
  const explicitIndex = normalizePlayerIndex(session?.localPlayerIndex);
  if (explicitIndex != null) return explicitIndex;

  const localPeerId = String(session?.localPeerId || "").trim();
  if (!localPeerId) return null;
  const localPlayer = (session?.players || []).find(
    (player) => String(player?.peerId || "") === localPeerId
  );
  return normalizePlayerIndex(localPlayer?.index);
}

function resolveLocalPlayerIndexFromPeer(session, players = null) {
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

function findLocalMatchPlayer(players, session, auditPublicKey = "", auditEncryptionPublicKey = "") {
  const entries = Array.isArray(players) ? players : [];
  const localPeerId = String(session?.localPeerId || "").trim();
  return entries.find((player) => String(player?.peerId || "").trim() === localPeerId)
    || entries.find((player) => String(player?.currentPeerId || "").trim() === localPeerId)
    || entries.find((player) =>
      playerMatchesPresentedAuditIdentity(player, auditPublicKey, auditEncryptionPublicKey)
    )
    || null;
}

function getPeerSessionStorage() {
  if (typeof window === "undefined") return null;
  return window.localStorage || window.sessionStorage || null;
}

function readStoredPlayerIndex(lobbyId) {
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

function readStoredAuditIdentity() {
  const storage = getPeerSessionStorage();
  if (!storage) return null;
  try {
    const raw = String(storage.getItem(AUDIT_IDENTITY_STORAGE_KEY) || "").trim();
    return raw ? JSON.parse(raw) : null;
  } catch {
    return null;
  }
}

function writeStoredAuditIdentity(identity) {
  const storage = getPeerSessionStorage();
  if (!storage || !identity) return;
  try {
    storage.setItem(AUDIT_IDENTITY_STORAGE_KEY, JSON.stringify(identity));
  } catch {
    // Ignore localStorage failures.
  }
}

function clearStoredAuditIdentity() {
  const storage = getPeerSessionStorage();
  if (!storage) return;
  try {
    storage.removeItem(AUDIT_IDENTITY_STORAGE_KEY);
  } catch {
    // Ignore localStorage failures.
  }
}

function actionQuorumVoteStorageKey(matchId, seq, voter) {
  return [
    ACTION_QUORUM_VOTE_STORAGE_PREFIX,
    String(matchId || ""),
    Number(seq || 0),
    Number(voter),
  ].join(":");
}

function readStoredActionQuorumVote(matchId, seq, voter) {
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

function writeStoredActionQuorumVote(vote) {
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

function privateDeckManifestStorageKey(matchId, owner) {
  return `${AUDIT_DECK_MANIFEST_STORAGE_PREFIX}:${String(matchId || "")}:${Number(owner)}`;
}

function readStoredPrivateDeckManifest(matchId, owner) {
  const storage = getPeerSessionStorage();
  if (!storage) return null;
  try {
    const raw = String(storage.getItem(privateDeckManifestStorageKey(matchId, owner)) || "").trim();
    return raw ? JSON.parse(raw) : null;
  } catch {
    return null;
  }
}

function writeStoredPrivateDeckManifest(manifest) {
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

function privateDeckManifestCardNames(manifest) {
  return [...new Set((manifest?.slotSecrets || [])
    .map((entry) => String(entry?.card || "").trim())
    .filter(Boolean))];
}

function preloadPrivateDeckManifestArt(manifest) {
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

function revealedOpeningStorageKey(indexKey) {
  return `${AUDIT_REVEALED_OPENING_STORAGE_PREFIX}:${String(indexKey || "")}`;
}

function readStoredRevealedOpening(indexKey) {
  const storage = getPeerSessionStorage();
  if (!storage) return null;
  try {
    const raw = String(storage.getItem(revealedOpeningStorageKey(indexKey)) || "").trim();
    return raw ? JSON.parse(raw) : null;
  } catch {
    return null;
  }
}

function writeStoredRevealedOpening(indexKey, opening) {
  const storage = getPeerSessionStorage();
  if (!storage || !indexKey || !opening) return;
  try {
    storage.setItem(revealedOpeningStorageKey(indexKey), JSON.stringify(opening));
  } catch {
    // Ignore localStorage failures.
  }
}

function removeStoredRevealedOpening(indexKey) {
  const storage = getPeerSessionStorage();
  if (!storage || !indexKey) return;
  try {
    storage.removeItem(revealedOpeningStorageKey(indexKey));
  } catch {
    // Ignore localStorage failures.
  }
}

function clearStoredRevealedOpeningsForMatch(matchId) {
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

function clearStoredRevealedOpeningsForMatchOwner(matchId, owner) {
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

function ziffleIdentityStorageKey(context) {
  return `${ZIFFLE_IDENTITY_STORAGE_PREFIX}:${String(context || "")}`;
}

function readStoredZiffleIdentity(context) {
  const storage = getPeerSessionStorage();
  if (!storage) return null;
  try {
    const raw = String(storage.getItem(ziffleIdentityStorageKey(context)) || "").trim();
    return raw ? JSON.parse(raw) : null;
  } catch {
    return null;
  }
}

function writeStoredZiffleIdentity(context, keyPair) {
  const storage = getPeerSessionStorage();
  if (!storage || !context || !keyPair) return;
  try {
    storage.setItem(ziffleIdentityStorageKey(context), JSON.stringify(keyPair));
  } catch {
    // Ignore localStorage failures.
  }
}

function resolveReconnectPlayerIndex(session, lobbyId) {
  const localIndex = resolveLocalPlayerIndex(session);
  if (localIndex != null) return localIndex;
  return readStoredPlayerIndex(lobbyId);
}

function writeStoredPlayerIndex(lobbyId, playerIndex) {
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

function clearStoredPlayerIndex(lobbyId = "") {
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

function createOfflinePeerId(lobbyId, playerIndex) {
  return `offline:${String(lobbyId || "lobby")}:${Number(playerIndex || 0)}`;
}

function isOfflinePeerId(peerId) {
  return String(peerId || "").trim().startsWith("offline:");
}

function firstOnlinePeerId(...peerIds) {
  for (const peerId of peerIds) {
    const normalized = String(peerId || "").trim();
    if (normalized && !isOfflinePeerId(normalized)) return normalized;
  }
  return "";
}

function markHostPeerDisconnected(players, hostPeerId, lobbyId, fallbackHostIndex = null) {
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

function findNextHostPlayer(players) {
  return [...(players || [])]
    .filter((player) => player.connected !== false)
    .sort((left, right) => Number(left.index || 0) - Number(right.index || 0))[0] || null;
}

function ensurePromotedLocalPlayer(players, session, lobbyId, localPlayerIndex) {
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

function toPublicPlayer(player) {
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

function toPublicPlayers(players) {
  return reindexPlayers(players).map(toPublicPlayer);
}

function toLobbyPlayer(player) {
  return {
    ...toPublicPlayer(player),
  };
}

function toLobbyPlayers(players) {
  return reindexPlayers(players).map(toLobbyPlayer);
}

function markPlayerConnectionState(players, peerId, connected, observedAtMs = Date.now()) {
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

function buildConnectionWarnings(session) {
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

function withConnectionWarnings(session) {
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

function ziffleRuntimeCommitment(deckHash, position) {
  return `ziffle:${String(deckHash || "")}:${Number(position)}`;
}

function ziffleDeckHashFromCommitment(commitment) {
  const normalized = String(commitment || "");
  if (!normalized.startsWith("ziffle:")) return "";
  const lastColon = normalized.lastIndexOf(":");
  return lastColon > "ziffle:".length
    ? normalized.slice("ziffle:".length, lastColon)
    : "";
}

function zifflePositionFromCommitment(commitment) {
  const normalized = String(commitment || "");
  if (!normalized.startsWith("ziffle:")) return null;
  const lastColon = normalized.lastIndexOf(":");
  if (lastColon <= "ziffle:".length) return null;
  const position = Number(normalized.slice(lastColon + 1));
  return Number.isSafeInteger(position) && position >= 0 ? position : null;
}

function ziffleContextFromOpening(opening) {
  if (!opening || typeof opening !== "object") return "";
  const proof = opening.ziffleReveal || opening.ziffleProof || opening.positionOpeningProof || {};
  return String(
    opening.ziffleContext
    || opening.ziffle_context
    || proof.context
    || ""
  );
}

function ziffleContextFromCeremony(ceremony) {
  return String(ceremony?.context || "");
}

function ziffleContextForCommitment(context, sourceCommitment, targetCommitment) {
  const normalizedContext = String(context || "");
  if (!normalizedContext) return "";
  const sourceDeckHash = ziffleDeckHashFromCommitment(sourceCommitment);
  const targetDeckHash = ziffleDeckHashFromCommitment(targetCommitment);
  if (sourceDeckHash && targetDeckHash && sourceDeckHash !== targetDeckHash) {
    return "";
  }
  return normalizedContext;
}

function ziffleIdentityPositionFromSources(...sources) {
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

function hiddenSourceZone(source) {
  return String(source?.zone ?? source?.hiddenZone ?? source?.hidden_zone ?? "").trim().toLowerCase();
}

function zifflePublicPositionFromSources(...sources) {
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

function withPinnedPublicZifflePosition(opening, publicPosition) {
  if (!opening || !publicPosition) return opening;
  return {
    ...opening,
    publicSlot: Number(publicPosition.publicSlot ?? publicPosition.position),
    publicCommitment: String(publicPosition.publicCommitment || publicPosition.positionCommitment || ""),
    position: Number(publicPosition.position),
    positionCommitment: String(publicPosition.positionCommitment || publicPosition.publicCommitment || ""),
  };
}

function openingHasZifflePosition(opening) {
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

function requirementHasZifflePosition(requirement) {
  if (!requirement || typeof requirement !== "object") return false;
  return Boolean(
    ziffleDeckHashFromCommitment(requirement.commitment)
    || ziffleDeckHashFromCommitment(requirement.positionCommitment)
    || ziffleDeckHashFromCommitment(requirement.position_commitment)
    || zifflePublicPositionFromSources(requirement)?.useAsPosition
  );
}

function exportedOpeningHasZifflePosition(exported) {
  if (!exported || typeof exported !== "object") return false;
  return Boolean(
    ziffleDeckHashFromCommitment(exported.commitment)
    || ziffleDeckHashFromCommitment(exported.positionCommitment)
    || ziffleDeckHashFromCommitment(exported.position_commitment)
    || ziffleDeckHashFromCommitment(exported.publicCommitment)
    || ziffleDeckHashFromCommitment(exported.public_commitment)
  );
}

function ziffleCeremonyForOpeningProof(proof, fallbackCeremony = null) {
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

function redactedMatchPayloadForPeer(payload, peerId, playerIndex = null) {
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

function validationDecksForMatchPayload(payload) {
  if (payload?.openDecklists && Array.isArray(payload.players)) {
    return payload.players.map((player) => sanitizeCardList(player.deck));
  }
  return payload?.decks;
}

function validationSideboardsForMatchPayload(payload) {
  if (payload?.openDecklists && Array.isArray(payload.players)) {
    return payload.players.map((player) => sanitizeCardList(player.sideboard));
  }
  return payload?.sideboards;
}

function validationCommandersForMatchPayload(payload) {
  if (payload?.openDecklists && Array.isArray(payload.players)) {
    return payload.players.map((player) => sanitizeCardList(player.commanders));
  }
  return payload?.commanders;
}

function canHostedMatchStart(session) {
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
  if (isTrustedMultiplayerSecurityMode(sessionSecurityMode(session))) return true;
  return session.players.every((player) =>
    player.ziffleKey && isSupportedZiffleDeckCount(player.deckCount)
  );
}

function playerCryptoSeatBindingReady(player) {
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

function playerMatchesPresentedAuditIdentity(player, auditPublicKey, auditEncryptionPublicKey) {
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

async function waitForCryptoSeatBindingsFromSession(sessionRef, timeoutMs = 10000) {
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

function compactMatchValidationError(error) {
  return String(error || "")
    .replace(/\s+/g, " ")
    .trim();
}

function summarizeMatchValidationIssues(issues) {
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

export function usePeerLobby({
  game,
  state,
  setState,
  setStatus,
  applySyncedCommand,
}) {
  const initialPeerOptions = buildPeerOptions();
  const initialHeartbeatConfig = buildPeerHeartbeatConfig();
  const initialMatchClockConfig = buildMatchClockConfig();
  const [multiplayer, setMultiplayer] = useState(() => createEmptyState());
  const peerRef = useRef(null);
  const hostConnectionRef = useRef(null);
  const clientConnectionsRef = useRef(new Map());
  const peerConnectionsRef = useRef(new Map());
  const connectionHeartbeatsRef = useRef(new Map());
  const matchStartPayloadRef = useRef(null);
  const actionHistoryRef = useRef([]);
  const gameRef = useRef(game);
  const stateRef = useRef(state);
  const multiplayerRef = useRef(multiplayer);
  const peerOptionsRef = useRef(initialPeerOptions);
  const peerHeartbeatConfigRef = useRef(initialHeartbeatConfig);
  const matchClockConfigRef = useRef(initialMatchClockConfig);
  const matchClockRef = useRef({
    policy: initialMatchClockConfig,
    playerCount: 0,
    baseRemainingMsByPlayer: [],
    activePlayerIndex: null,
    epochStartedAtMs: null,
    clockHash: INITIAL_MATCH_CLOCK_HASH,
    lastSequence: 0,
  });
  const timeoutClaimInFlightRef = useRef("");
  const disconnectForfeitInFlightRef = useRef(new Set());
  const localDisconnectObservationsRef = useRef(new Map());
  const peerServerLabelRef = useRef(describePeerServer(initialPeerOptions));
  const hostMessageQueueRef = useRef(Promise.resolve());
  const clientMessageQueueRef = useRef(Promise.resolve());
  const peerMessageQueueRef = useRef(Promise.resolve());
  const resyncingPeerIdsRef = useRef(new Set());
  const resyncWaitersRef = useRef([]);
  const submissionIdleWaitersRef = useRef([]);
  const actionSubmissionStartedAtMsRef = useRef(0);
  const awaitingStateResyncRef = useRef(false);
  const auditKeyPairRef = useRef(null);
  const auditEncryptionKeyPairRef = useRef(null);
  const auditPublicKeyRef = useRef("");
  const auditEncryptionPublicKeyRef = useRef("");
  const auditStateHashRef = useRef(INITIAL_AUDIT_STATE_HASH);
  const initialPublicCheckpointHashRef = useRef("");
  const auditVerifyKeyCacheRef = useRef(new Map());
  const liveAuditTranscriptRef = useRef(null);
  const privateDeckManifestsRef = useRef(new Map());
  const localRevealedOpeningsRef = useRef(new Map());
  const ziffleKeyPairsRef = useRef(new Map());
  const ziffleShuffleWaitersRef = useRef(new Map());
  const ziffleRevealWaitersRef = useRef(new Map());
  const ziffleRevealTokenCacheRef = useRef(new Map());
  const verifiedAuditOpeningsRef = useRef(new Set());
  const rngCommitWaitersRef = useRef(new Map());
  const rngRevealWaitersRef = useRef(new Map());
  const rngCommitNoncesRef = useRef(new Map());
  const signedRngCommitmentsRef = useRef(new Map());
  const rngRevealCommitSetLocksRef = useRef(new Map());
  const timeoutVoteWaitersRef = useRef(new Map());
  const actionQuorumVoteWaitersRef = useRef(new Map());
  const signedActionQuorumVotesRef = useRef(new Map());
  const cryptoMaterialWaitersRef = useRef(new Map());
  const outboundCryptoMaterialRequestsRef = useRef(new Map());
  const pendingActionIntentsRef = useRef(new Map());
  const pendingActionIntentTimeoutsRef = useRef(new Map());
  const ignoredActionIntentKeysRef = useRef(new Map());
  const actionIntentOpeningPreviewKeysRef = useRef(new Map());
  const reconnectChallengesRef = useRef(new Map());
  const privateViewDisclosuresRef = useRef(new Map());
  const liveZiffleCeremoniesRef = useRef(new Map());
  const localZiffleCeremonyLookupRef = useRef(new Map());
  const ziffleOpeningPositionsRef = useRef(new Map());
  const ziffleHandRevealKeyRef = useRef("");
  const ziffleHandRevealQuickKeyRef = useRef("");
  const localZiffleRevealInFlightRef = useRef(null);
  const verifiedShuffleProofsRef = useRef(new Set());
  const ziffleShufflePerfRef = useRef([]);
  const relayedActionIdsRef = useRef(new Set());
  const applyingSequencedActionsRef = useRef(new Map());
  const pendingSequencedActionsRef = useRef(new Map());
  const drainingPendingSequencedActionsRef = useRef(false);
  const actionCryptoRequirementsRef = useRef(new Map());
  const matchClockObservationExemptSequenceRef = useRef(0);
  const ensureDirectPeerConnectionsRef = useRef(() => {});

  useEffect(() => {
    gameRef.current = game;
  }, [game]);

  useEffect(() => {
    stateRef.current = state;
  }, [state]);

  // updateMultiplayer owns multiplayerRef so async message handlers can observe
  // state changes before React commits the next render. Mirroring rendered state
  // back into the ref here can regress the ref to an older render during fast
  // peer message sequences.

  const ensureDirectPeerConnections = useCallback((players) => {
    ensureDirectPeerConnectionsRef.current(players);
  }, []);

  const clearConnectionHeartbeat = useCallback((key) => {
    const heartbeat = connectionHeartbeatsRef.current.get(key);
    if (!heartbeat) return;
    window.clearInterval(heartbeat.timer);
    connectionHeartbeatsRef.current.delete(key);
  }, []);

  const clearAllConnectionHeartbeats = useCallback(() => {
    for (const heartbeat of connectionHeartbeatsRef.current.values()) {
      window.clearInterval(heartbeat.timer);
    }
    connectionHeartbeatsRef.current.clear();
  }, []);

  const markConnectionAlive = useCallback((key) => {
    const heartbeat = connectionHeartbeatsRef.current.get(key);
    if (heartbeat) {
      heartbeat.lastSeen = Date.now();
    }
  }, []);

  function pendingActionIntentSuppressesHeartbeatStale(nowMs = Date.now()) {
    if (
      multiplayerRef.current.submittingAction
      && actionSubmissionStartedAtMsRef.current > 0
      && nowMs < actionSubmissionStartedAtMsRef.current + MAX_PENDING_ACTION_INTENT_MS + MATCH_CLOCK_CLAIM_SKEW_MS
    ) {
      return true;
    }
    for (const record of pendingActionIntentsRef.current.values()) {
      if (nowMs < pendingActionIntentDueAtMs(record)) {
        return true;
      }
    }
    return false;
  }

  const startConnectionHeartbeat = useCallback((key, conn, onStale) => {
    clearConnectionHeartbeat(key);
    const { intervalMs, timeoutMs } = peerHeartbeatConfigRef.current;
    if (!intervalMs || !timeoutMs) return;

    const heartbeat = {
      lastSeen: Date.now(),
      timer: window.setInterval(() => {
        if (!conn || conn.open === false) {
          clearConnectionHeartbeat(key);
          onStale?.("Connection closed");
          return;
        }

        const nowMs = Date.now();
        if (
          nowMs - heartbeat.lastSeen > timeoutMs
          && !pendingActionIntentSuppressesHeartbeatStale(nowMs)
        ) {
          clearConnectionHeartbeat(key);
          try {
            conn.close();
          } catch {
            // Best effort; the stale callback handles local state cleanup.
          }
          onStale?.("Peer heartbeat timed out");
          return;
        }

        safeSend(conn, {
          type: "peer_heartbeat",
          protocolVersion: PROTOCOL_VERSION,
          at: nowMs,
        });
      }, intervalMs),
    };
    connectionHeartbeatsRef.current.set(key, heartbeat);
  }, [clearConnectionHeartbeat]);

  const handleConnectionHeartbeatMessage = useCallback((conn, message) => {
    if (message?.type === "peer_heartbeat_ack") {
      return message.protocolVersion === PROTOCOL_VERSION;
    }
    if (message?.type !== "peer_heartbeat") return false;
    if (message.protocolVersion !== PROTOCOL_VERSION) return false;
    safeSend(conn, {
      type: "peer_heartbeat_ack",
      protocolVersion: PROTOCOL_VERSION,
      at: message.at ?? Date.now(),
    });
    return true;
  }, []);

  const resolveSubmissionIdleWaiters = useCallback(() => {
    if (multiplayerRef.current.submittingAction) return;
    const waiters = submissionIdleWaitersRef.current;
    submissionIdleWaitersRef.current = [];
    for (const waiter of waiters) {
      if (waiter.timeoutId) {
        globalThis.clearTimeout(waiter.timeoutId);
      }
      waiter.resolve(true);
    }
  }, []);

  const waitForSubmissionIdle = useCallback((timeoutMs = ACTION_SUBMISSION_IDLE_WAIT_MS) => {
    if (!multiplayerRef.current.submittingAction) {
      return Promise.resolve(true);
    }
    return new Promise((resolve) => {
      const waiter = {
        resolve,
        timeoutId: null,
      };
      waiter.timeoutId = globalThis.setTimeout(() => {
        submissionIdleWaitersRef.current = submissionIdleWaitersRef.current.filter(
          (entry) => entry !== waiter
        );
        resolve(false);
      }, Math.max(0, Number(timeoutMs || 0)));
      submissionIdleWaitersRef.current.push(waiter);
    });
  }, []);

  const updateMultiplayer = useCallback((updater) => {
    const previous = multiplayerRef.current;
    const next =
      typeof updater === "function" ? updater(previous) : updater;
    if (next === previous) {
      return previous;
    }
    const normalized = withConnectionWarnings(next);
    if (normalized.submittingAction && !actionSubmissionStartedAtMsRef.current) {
      actionSubmissionStartedAtMsRef.current = Date.now();
    }
    if (!normalized.submittingAction) {
      actionSubmissionStartedAtMsRef.current = 0;
    }
    multiplayerRef.current = normalized;
    setMultiplayer(normalized);
    if (!normalized.submittingAction) {
      resolveSubmissionIdleWaiters();
    }
    return normalized;
  }, [resolveSubmissionIdleWaiters, setMultiplayer]);

  const beginPeerWait = useCallback((wait = {}) => {
    const requestId = String(
      wait.requestId || `peer-wait:${Date.now().toString(36)}:${randomAuditHex(8)}`
    );
    const peerWait = {
      kind: String(wait.kind || "peer_response"),
      requestId,
      title: String(wait.title || "Waiting for peers"),
      description: String(wait.description || ""),
      peerIndex: wait.peerIndex == null ? null : Number(wait.peerIndex),
      peerName: wait.peerName == null ? "" : String(wait.peerName),
      peers: Array.isArray(wait.peers) ? cloneMultiplayerPayload(wait.peers) : [],
      detail: wait.detail == null ? "" : String(wait.detail),
      operation: wait.operation == null ? "" : String(wait.operation),
      phase: wait.phase == null ? "" : String(wait.phase),
      cardName: wait.cardName == null ? "" : String(wait.cardName),
      zone: wait.zone == null ? "" : String(wait.zone),
      actionIntentKey: wait.actionIntentKey == null ? "" : String(wait.actionIntentKey),
      progressCurrent: Number.isFinite(Number(wait.progressCurrent))
        ? Number(wait.progressCurrent)
        : null,
      progressTotal: Number.isFinite(Number(wait.progressTotal))
        ? Number(wait.progressTotal)
        : null,
      responseTimeoutMs: Number.isFinite(Number(wait.responseTimeoutMs))
        ? Math.max(1, Math.floor(Number(wait.responseTimeoutMs)))
        : null,
      openingPreviews: mergeActionOpeningPreviews(
        [],
        [
          ...(Array.isArray(wait.openingPreviews) ? wait.openingPreviews : []),
          wait.openingPreview || wait.opening_preview,
        ]
      ),
      local: Boolean(wait.local),
      startedAtMs: Date.now(),
    };
    updateMultiplayer((prev) => ({ ...prev, peerWait }));
    return requestId;
  }, [updateMultiplayer]);

  const updatePeerWait = useCallback((requestId = null, patch = {}) => {
    updateMultiplayer((prev) => {
      if (!prev.peerWait) return prev;
      if (requestId && String(prev.peerWait.requestId || "") !== String(requestId)) {
        return prev;
      }
      const nextPatch = { ...patch };
      if (Object.prototype.hasOwnProperty.call(nextPatch, "progressCurrent")) {
        nextPatch.progressCurrent = Number.isFinite(Number(nextPatch.progressCurrent))
          ? Number(nextPatch.progressCurrent)
          : null;
      }
      if (Object.prototype.hasOwnProperty.call(nextPatch, "progressTotal")) {
        nextPatch.progressTotal = Number.isFinite(Number(nextPatch.progressTotal))
          ? Number(nextPatch.progressTotal)
          : null;
      }
      if (Object.prototype.hasOwnProperty.call(nextPatch, "responseTimeoutMs")) {
        nextPatch.responseTimeoutMs = Number.isFinite(Number(nextPatch.responseTimeoutMs))
          ? Math.max(1, Math.floor(Number(nextPatch.responseTimeoutMs)))
          : null;
      }
      const nextOpeningPreview = normalizeActionOpeningPreview(
        nextPatch.openingPreview || nextPatch.opening_preview
      );
      if (nextOpeningPreview) {
        nextPatch.openingPreviews = mergeActionOpeningPreviews(
          prev.peerWait.openingPreviews || [],
          [nextOpeningPreview]
        );
      } else if (Object.prototype.hasOwnProperty.call(nextPatch, "openingPreviews")) {
        nextPatch.openingPreviews = mergeActionOpeningPreviews([], nextPatch.openingPreviews || []);
      }
      delete nextPatch.opening_preview;
      return {
        ...prev,
        peerWait: {
          ...prev.peerWait,
          ...nextPatch,
        },
      };
    });
  }, [updateMultiplayer]);

  const clearPeerWait = useCallback((requestId = null) => {
    updateMultiplayer((prev) => {
      if (!prev.peerWait) return prev;
      if (requestId && String(prev.peerWait.requestId || "") !== String(requestId)) {
        return prev;
      }
      return { ...prev, peerWait: null };
    });
  }, [updateMultiplayer]);

  function clearPeerWaitForActionIntent(actionIntentKeyValue) {
    const normalizedKey = String(actionIntentKeyValue || "");
    if (!normalizedKey) return;
    updateMultiplayer((prev) => {
      if (String(prev.peerWait?.actionIntentKey || "") !== normalizedKey) return prev;
      return { ...prev, peerWait: null };
    });
  }

  function updatePeerWaitForActionIntent(actionIntentKeyValue, patch = {}) {
    const normalizedKey = String(actionIntentKeyValue || "");
    if (!normalizedKey) return false;
    let updated = false;
    updateMultiplayer((prev) => {
      if (String(prev.peerWait?.actionIntentKey || "") !== normalizedKey) return prev;
      const nextPatch = { ...patch };
      const nextOpeningPreview = normalizeActionOpeningPreview(
        nextPatch.openingPreview || nextPatch.opening_preview
      );
      if (nextOpeningPreview) {
        nextPatch.openingPreviews = mergeActionOpeningPreviews(
          prev.peerWait.openingPreviews || [],
          [nextOpeningPreview]
        );
      } else if (Object.prototype.hasOwnProperty.call(nextPatch, "openingPreviews")) {
        nextPatch.openingPreviews = mergeActionOpeningPreviews([], nextPatch.openingPreviews || []);
      }
      delete nextPatch.opening_preview;
      updated = true;
      return {
        ...prev,
        peerWait: {
          ...prev.peerWait,
          ...nextPatch,
        },
      };
    });
    return updated;
  }

  const ensureAuditIdentity = useCallback(async () => {
    let storedIdentity = null;
    if (!auditKeyPairRef.current) {
      storedIdentity = readStoredAuditIdentity();
      if (storedIdentity) {
        try {
          auditKeyPairRef.current = await importAuditKeyPair(storedIdentity);
        } catch (err) {
          void err;
          clearStoredAuditIdentity();
        }
      }
    }
    if (!auditEncryptionKeyPairRef.current) {
      storedIdentity = storedIdentity || readStoredAuditIdentity();
      if (storedIdentity) {
        try {
          auditEncryptionKeyPairRef.current = await importAuditEncryptionKeyPair(storedIdentity);
        } catch (err) {
          void err;
        }
      }
    }
    if (!auditKeyPairRef.current) {
      auditKeyPairRef.current = await createAuditSessionKey();
    }
    if (!auditEncryptionKeyPairRef.current) {
      auditEncryptionKeyPairRef.current = await createAuditEncryptionKey();
    }
    writeStoredAuditIdentity({
      ...(await exportAuditKeyPair(auditKeyPairRef.current)),
      ...(await exportAuditEncryptionKeyPair(auditEncryptionKeyPairRef.current)),
    });
    if (!auditPublicKeyRef.current) {
      auditPublicKeyRef.current = await exportAuditPublicKey(auditKeyPairRef.current);
    }
    if (!auditEncryptionPublicKeyRef.current) {
      auditEncryptionPublicKeyRef.current = await exportAuditEncryptionPublicKey(
        auditEncryptionKeyPairRef.current
      );
    }
    return {
      keyPair: auditKeyPairRef.current,
      encryptionKeyPair: auditEncryptionKeyPairRef.current,
      publicKey: auditPublicKeyRef.current,
      encryptionPublicKey: auditEncryptionPublicKeyRef.current,
    };
  }, []);

  const signPlayerGenesis = useCallback(async ({ matchId, player }) => {
    const { keyPair } = await ensureAuditIdentity();
    return buildSignedPlayerGenesis({
      keyPair,
      matchId,
      protocolVersion: PROTOCOL_VERSION,
      timeoutMs: matchClockConfigRef.current.initialMs,
      player,
    });
  }, [ensureAuditIdentity]);

	  const ensureZiffleIdentity = useCallback(async ({ context, deckCount = 60 }) => {
	    const normalizedContext = String(context || "").trim() || "match";
	    if (ziffleKeyPairsRef.current.has(normalizedContext)) {
	      return ziffleKeyPairsRef.current.get(normalizedContext);
	    }
	    const stored = readStoredZiffleIdentity(normalizedContext);
	    if (stored?.publicKeyHex && stored?.secretKeyHex) {
	      ziffleKeyPairsRef.current.set(normalizedContext, stored);
	      return stored;
	    }
	    const currentGame = gameRef.current;
	    if (!currentGame || typeof currentGame.ziffleKeygen !== "function") {
	      throw new Error("Ziffle mental-poker backend is not available in the game engine");
    }
    const keyPair = await currentGame.ziffleKeygen({
      deckCount: Number(deckCount || 60),
      context: normalizedContext,
      entropyHex: randomAuditHex(32),
	    });
	    ziffleKeyPairsRef.current.set(normalizedContext, keyPair);
	    writeStoredZiffleIdentity(normalizedContext, keyPair);
	    return keyPair;
	  }, []);

  const publicZiffleKey = useCallback((keyPair, playerIndex) => {
    if (!keyPair) return null;
    return {
      player: Number(playerIndex || 0),
      publicKeyHex: String(keyPair.publicKeyHex || ""),
      ownershipProofHex: String(keyPair.ownershipProofHex || ""),
    };
  }, []);

  const zifflePublicKeysForPlayers = useCallback((players = multiplayerRef.current.players) => {
    return reindexPlayers(players).map((player) => {
      const key = player.ziffleKey || {};
      return {
        player: Number(player.index || 0),
        publicKeyHex: String(key.publicKeyHex || ""),
        ownershipProofHex: String(key.ownershipProofHex || ""),
      };
    });
  }, []);

  const runtimeManifestForZiffleCeremony = useCallback((manifest, ceremony) => {
    const deckCount = Number(ceremony?.deckCount || manifest?.deckCount || 0);
    const baseManifest = publicDeckManifest(manifest) || {};
    return {
      ...baseManifest,
      deckCount,
      commitmentRoot: `ziffle:${String(ceremony?.deckHash || "")}`,
      slotCommitments: Array.from({ length: deckCount }, (_, position) => ({
        slot: position,
        commitment: ziffleRuntimeCommitment(ceremony.deckHash, position),
      })),
    };
  }, []);

  const makeZiffleRequestId = useCallback((prefix) => (
    `${prefix}:${Date.now().toString(36)}:${randomAuditHex(8)}`
  ), []);

  const waitForZiffleShuffleStep = useCallback((requestId, timeoutMs = 60000, wait = {}) => (
    new Promise((resolve, reject) => {
      beginPeerWait({
        kind: "ziffle_shuffle",
        requestId,
        title: "Waiting for shuffle material",
        description: "A peer is producing verifiable shuffle material before the game can continue.",
        ...wait,
      });
      const timer = window.setTimeout(() => {
        ziffleShuffleWaitersRef.current.delete(requestId);
        clearPeerWait(requestId);
        reject(new Error("Timed out waiting for ziffle shuffle step"));
      }, timeoutMs);
      ziffleShuffleWaitersRef.current.set(requestId, {
        resolve: (value) => {
          window.clearTimeout(timer);
          clearPeerWait(requestId);
          resolve(value);
        },
        reject: (err) => {
          window.clearTimeout(timer);
          clearPeerWait(requestId);
          reject(err);
        },
      });
    })
  ), [beginPeerWait, clearPeerWait]);

  const waitForZiffleRevealToken = useCallback((requestId, timeoutMs = 60000, metadata = null, wait = {}) => (
    new Promise((resolve, reject) => {
      const normalizedTimeoutMs = Math.max(1, Math.floor(Number(timeoutMs || 1)));
      const startedAtMs = Date.now();
      const hardDueAtMs = startedAtMs + Math.max(normalizedTimeoutMs, MAX_PENDING_ACTION_INTENT_MS);
      let dueAtMs = startedAtMs + normalizedTimeoutMs;
      let timer = null;
      const scheduleTimeout = () => {
        if (timer) window.clearTimeout(timer);
        timer = window.setTimeout(() => {
          ziffleRevealWaitersRef.current.delete(requestId);
          clearPeerWait(requestId);
          const err = new Error(
            metadata
              ? `Timed out waiting for ziffle reveal token: ${compactZiffleDiagnosticsJson(metadata)}`
              : "Timed out waiting for ziffle reveal token"
          );
          err.protocolResponseTimeoutTiming = {
            responseTimeoutMs: normalizedTimeoutMs,
            requestedAtMs: Math.max(1, dueAtMs - normalizedTimeoutMs),
          };
          reject(err);
        }, Math.max(1, Math.ceil(dueAtMs - Date.now())));
      };
      beginPeerWait({
        kind: "ziffle_reveal",
        requestId,
        title: "Waiting for reveal material",
        description: "A peer is sending cryptographic reveal material before this hidden card can open locally.",
        actionIntentKey: metadata?.actionIntentKey || "",
        ...wait,
      });
      scheduleTimeout();
      ziffleRevealWaitersRef.current.set(requestId, {
        metadata,
        actionIntentKey: String(metadata?.actionIntentKey || ""),
        extendTimeout: (additionalMs = normalizedTimeoutMs) => {
          const requestedExtension = Math.max(1, Math.floor(Number(additionalMs || normalizedTimeoutMs)));
          const nextDueAtMs = Math.min(Date.now() + requestedExtension, hardDueAtMs);
          if (nextDueAtMs <= dueAtMs) return false;
          dueAtMs = nextDueAtMs;
          scheduleTimeout();
          return true;
        },
        resolve: (value) => {
          if (timer) window.clearTimeout(timer);
          clearPeerWait(requestId);
          resolve(value);
        },
        reject: (err) => {
          if (timer) window.clearTimeout(timer);
          clearPeerWait(requestId);
          reject(err);
        },
      });
    })
  ), [beginPeerWait, clearPeerWait]);

  const waitForRngCommit = useCallback((requestId, timeoutMs = 60000, wait = {}) => (
    new Promise((resolve, reject) => {
      beginPeerWait({
        kind: "fair_random_commit",
        requestId,
        title: "Waiting for random commitment",
        description: "A peer must commit to their random contribution before the shared random value can be revealed.",
        ...wait,
      });
      const timer = window.setTimeout(() => {
        rngCommitWaitersRef.current.delete(requestId);
        clearPeerWait(requestId);
        reject(new Error("Timed out waiting for random commitment"));
      }, timeoutMs);
      rngCommitWaitersRef.current.set(requestId, {
        resolve: (value) => {
          window.clearTimeout(timer);
          clearPeerWait(requestId);
          resolve(value);
        },
        reject: (err) => {
          window.clearTimeout(timer);
          clearPeerWait(requestId);
          reject(err);
        },
      });
    })
  ), [beginPeerWait, clearPeerWait]);

  const waitForRngReveal = useCallback((requestId, timeoutMs = 60000, wait = {}) => (
    new Promise((resolve, reject) => {
      beginPeerWait({
        kind: "fair_random_reveal",
        requestId,
        title: "Waiting for random reveal",
        description: "A peer must reveal their committed random contribution before the shared random value can be used.",
        ...wait,
      });
      const timer = window.setTimeout(() => {
        rngRevealWaitersRef.current.delete(requestId);
        clearPeerWait(requestId);
        reject(new Error("Timed out waiting for random reveal"));
      }, timeoutMs);
      rngRevealWaitersRef.current.set(requestId, {
        resolve: (value) => {
          window.clearTimeout(timer);
          clearPeerWait(requestId);
          resolve(value);
        },
        reject: (err) => {
          window.clearTimeout(timer);
          clearPeerWait(requestId);
          reject(err);
        },
      });
    })
  ), [beginPeerWait, clearPeerWait]);

  const waitForTimeoutVote = useCallback((requestId, timeoutMs = 15000, wait = {}) => (
    new Promise((resolve, reject) => {
      beginPeerWait({
        kind: "timeout_vote",
        requestId,
        title: "Waiting for peer vote",
        description: "A peer must sign the timeout vote before this claim can be submitted.",
        ...wait,
      });
      const timer = window.setTimeout(() => {
        timeoutVoteWaitersRef.current.delete(requestId);
        clearPeerWait(requestId);
        reject(new Error("Timed out waiting for timeout vote"));
      }, timeoutMs);
      timeoutVoteWaitersRef.current.set(requestId, {
        resolve: (value) => {
          window.clearTimeout(timer);
          clearPeerWait(requestId);
          resolve(value);
        },
        reject: (err) => {
          window.clearTimeout(timer);
          clearPeerWait(requestId);
          reject(err);
        },
      });
    })
  ), [beginPeerWait, clearPeerWait]);

  const waitForActionQuorumVote = useCallback((requestId, timeoutMs = 30000, wait = {}) => (
    new Promise((resolve, reject) => {
      beginPeerWait({
        kind: "action_quorum",
        requestId,
        title: "Waiting for action quorum",
        description: "Peers are validating and signing this action before it can be accepted.",
        ...wait,
      });
      const timer = window.setTimeout(() => {
        actionQuorumVoteWaitersRef.current.delete(requestId);
        clearPeerWait(requestId);
        reject(new Error("Timed out waiting for action quorum vote"));
      }, timeoutMs);
      actionQuorumVoteWaitersRef.current.set(requestId, {
        resolve: (value) => {
          window.clearTimeout(timer);
          clearPeerWait(requestId);
          resolve(value);
        },
        reject: (err) => {
          window.clearTimeout(timer);
          clearPeerWait(requestId);
          reject(err);
        },
      });
    })
  ), [beginPeerWait, clearPeerWait]);

  const waitForCryptoMaterial = useCallback((requestId, timeoutMs = 60000, wait = {}) => (
    new Promise((resolve, reject) => {
      beginPeerWait({
        kind: "crypto_material",
        requestId,
        title: "Waiting for cryptographic material",
        description: "A peer must send hidden-card opening material before this action can advance.",
        ...wait,
      });
      const timer = window.setTimeout(() => {
        cryptoMaterialWaitersRef.current.delete(requestId);
        clearPeerWait(requestId);
        reject(new Error("Timed out waiting for cryptographic opening material"));
      }, timeoutMs);
      cryptoMaterialWaitersRef.current.set(requestId, {
        resolve: (value) => {
          window.clearTimeout(timer);
          clearPeerWait(requestId);
          resolve(value);
        },
        reject: (err) => {
          window.clearTimeout(timer);
          clearPeerWait(requestId);
          reject(err);
        },
      });
    })
  ), [beginPeerWait, clearPeerWait]);

  async function makeProtocolResponseTimeoutError(cause, claim = {}) {
    if (!multiplayerRef.current.matchStarted) return cause;
    const targetPlayerIndex = normalizePlayerIndex(claim.targetPlayerIndex);
    if (targetPlayerIndex == null) return cause;
    const players = reindexPlayers(matchStartPayloadRef.current?.players || multiplayerRef.current.players || []);
    const target = players.find((player) => Number(player.index) === Number(targetPlayerIndex));
    const responseTimeoutMs = Math.max(
      1,
      Math.floor(Number(claim.responseTimeoutMs || PROTOCOL_RESPONSE_TIMEOUT_MS))
    );
    const requestedAtMs = Math.max(
      1,
      Math.floor(Number(claim.requestedAtMs || Date.now() - responseTimeoutMs))
    );
    const requestPayload = cloneMultiplayerPayload(claim.requestPayload || {});
    const requestPayloadHash = String(
      claim.requestPayloadHash
      || await sha256Hex(canonicalMultiplayerPayload(requestPayload))
    );
    const targetLabel = target?.name || `Player ${targetPlayerIndex + 1}`;
    const err = new Error(
      `${targetLabel} did not respond to ${String(claim.requestType || "protocol request")} `
      + `within ${Math.ceil(responseTimeoutMs / 1000)}s`
    );
    err.cause = cause;
    err.protocolResponseTimeoutClaim = {
      matchId: currentAuditMatchId(),
      basisSequence: Number(claim.basisSequence ?? multiplayerRef.current.lastAppliedSequence ?? 0),
      targetPlayerIndex,
      targetPeerId: String(claim.targetPeerId || target?.peerId || ""),
      targetName: targetLabel,
      requesterIndex: normalizePlayerIndex(claim.requesterIndex)
        ?? resolveLocalPlayerIndex(multiplayerRef.current),
      requestType: String(claim.requestType || ""),
      requestId: String(claim.requestId || ""),
      requestPayloadHash,
      requestPayload,
      responseTimeoutMs,
      requestedAtMs,
      eligibleAtMs: requestedAtMs + responseTimeoutMs,
    };
    return err;
  }

  async function waitForProtocolResponse(waiter, claim) {
    try {
      return await waiter;
    } catch (err) {
      if (!isProtocolResponseWaitTimeout(err)) throw err;
      const timing = err?.protocolResponseTimeoutTiming || {};
      throw await makeProtocolResponseTimeoutError(err, {
        ...claim,
        ...(timing.responseTimeoutMs != null ? { responseTimeoutMs: timing.responseTimeoutMs } : {}),
        ...(timing.requestedAtMs != null ? { requestedAtMs: timing.requestedAtMs } : {}),
      });
    }
  }

  const resolveZiffleShuffleStep = useCallback((message) => {
    const requestId = String(message?.requestId || "");
    const waiter = ziffleShuffleWaitersRef.current.get(requestId);
    if (!waiter) return false;
    ziffleShuffleWaitersRef.current.delete(requestId);
    if (message.error) {
      waiter.reject(new Error(String(message.error)));
    } else {
      waiter.resolve(message.step);
    }
    return true;
  }, []);

  const resolveZiffleRevealToken = useCallback((message) => {
    const requestId = String(message?.requestId || "");
    const waiter = ziffleRevealWaitersRef.current.get(requestId);
    if (!waiter) return false;
    ziffleRevealWaitersRef.current.delete(requestId);
    if (message.error) {
      const diagnostics = {
        requester: waiter.metadata || null,
        responder: message.diagnostics || null,
      };
      const error = new Error(String(message.error));
      error.ziffleDiagnostics = diagnostics;
      waiter.reject(error);
    } else {
      waiter.resolve(message.tokens || message.token);
    }
    return true;
  }, []);

  const resolveRngCommit = useCallback((message) => {
    const requestId = String(message?.requestId || "");
    const waiter = rngCommitWaitersRef.current.get(requestId);
    if (!waiter) return false;
    rngCommitWaitersRef.current.delete(requestId);
    if (message.error) {
      waiter.reject(new Error(String(message.error)));
    } else {
      waiter.resolve(message.commitment);
    }
    return true;
  }, []);

  const resolveRngReveal = useCallback((message) => {
    const requestId = String(message?.requestId || "");
    const waiter = rngRevealWaitersRef.current.get(requestId);
    if (!waiter) return false;
    rngRevealWaitersRef.current.delete(requestId);
    if (message.error) {
      waiter.reject(new Error(String(message.error)));
    } else {
      waiter.resolve(message.reveal);
    }
    return true;
  }, []);

  const resolveTimeoutVote = useCallback((message) => {
    const requestId = String(message?.requestId || "");
    const waiter = timeoutVoteWaitersRef.current.get(requestId);
    if (!waiter) return false;
    timeoutVoteWaitersRef.current.delete(requestId);
    if (message.error) {
      waiter.reject(new Error(String(message.error)));
    } else {
      waiter.resolve(message.vote);
    }
    return true;
  }, []);

  const resolveActionQuorumVote = useCallback((message) => {
    const requestId = String(message?.requestId || "");
    const waiter = actionQuorumVoteWaitersRef.current.get(requestId);
    if (!waiter) return false;
    actionQuorumVoteWaitersRef.current.delete(requestId);
    if (message.error) {
      waiter.reject(new Error(String(message.error)));
    } else {
      waiter.resolve(message.vote);
    }
    return true;
  }, []);

  const resolveCryptoMaterial = useCallback((message) => {
    const requestId = String(message?.requestId || "");
    const waiter = cryptoMaterialWaitersRef.current.get(requestId);
    if (!waiter) return false;
    cryptoMaterialWaitersRef.current.delete(requestId);
    if (message.error) {
      waiter.reject(new Error(String(message.error)));
    } else {
      waiter.resolve({
        openings: message.openings || [],
        privateViewProofs: message.privateViewProofs || [],
      });
    }
    return true;
  }, []);

  const currentAuditMatchId = useCallback(() => {
    const matchPayload = matchStartPayloadRef.current;
    const session = multiplayerRef.current;
    return String(
      matchPayload?.auditMatchId
        || session.lobbyId
        || session.hostPeerId
        || "match"
    );
  }, []);

  const rememberPrivateViewDisclosure = useCallback((disclosure) => {
    if (!disclosure || typeof disclosure !== "object") return;
    const payload = disclosure.payload || disclosure;
    const matchId = String(disclosure.matchId || payload?.matchId || currentAuditMatchId());
    const plaintextHash = String(disclosure.plaintextHash || "");
    const requirementId = String(disclosure.requirementId || payload?.requirementId || "");
    const key = [
      matchId,
      Number(disclosure.seq ?? payload?.seq ?? 0),
      requirementId,
      Number(disclosure.owner ?? payload?.owner ?? -1),
      Number(disclosure.viewer ?? payload?.viewer ?? -1),
      Number(disclosure.objectId ?? payload?.objectId ?? -1),
      plaintextHash,
    ].join(":");
    privateViewDisclosuresRef.current.set(key, cloneMultiplayerPayload({
      ...disclosure,
      matchId,
      type: String(disclosure.type || "private_view_opening_disclosure"),
      payload: cloneMultiplayerPayload(payload),
    }));
  }, [currentAuditMatchId]);

  const resolveLocalCryptoPlayerIndex = useCallback((payload = matchStartPayloadRef.current) => {
    const session = multiplayerRef.current;
    return (
      resolveLocalPlayerIndexFromPeer(session, payload?.players)
      ?? resolveLocalPlayerIndex(session)
    );
  }, []);

	  const rememberPrivateDeckManifest = useCallback((manifest) => {
	    if (!manifest || !Array.isArray(manifest.slotSecrets)) return;
	    const key = `${manifest.matchId}:${Number(manifest.owner)}`;
	    privateDeckManifestsRef.current.set(key, manifest);
	    writeStoredPrivateDeckManifest(manifest);
      preloadPrivateDeckManifestArt(manifest);
	  }, []);

	  const privateDeckManifestForOwner = useCallback((owner, matchId = currentAuditMatchId()) => {
	    const key = `${matchId}:${Number(owner)}`;
	    const normalizedOwner = Number(owner);
	    // The match-start payload's public manifest is the source of truth for
	    // which deck sits at a seat. A locally stored private manifest can claim
	    // the wrong owner (e.g. a guest's provisional join-time manifest built
	    // before seat assignment), so reject any local copy that does not match
	    // the published commitments before trusting its slot secrets.
	    const payload = matchStartPayloadRef.current;
	    const payloadMatches = payload && String(payload.auditMatchId || "") === String(matchId || "");
	    const payloadPlayer = payloadMatches
	      ? reindexPlayers(payload.players || []).find(
	        (entry) => Number(entry.index) === normalizedOwner
	      )
	      : null;
	    const payloadManifests = payloadMatches && Array.isArray(payload.deckAuditManifests)
	      ? payload.deckAuditManifests
	      : [];
	    const sharedManifest = payloadMatches
	      ? publicDeckManifest(
	        payloadManifests.find((entry) => Number(entry?.owner) === normalizedOwner)
	          || payloadPlayer?.deckAuditManifest
	      )
	      : null;
	    const matchesPublishedManifest = (candidate) =>
	      !sharedManifest
	      || (
	        String(candidate?.commitmentRoot || "") === String(sharedManifest.commitmentRoot || "")
	        && String(candidate?.decklistCommitment || "") === String(sharedManifest.decklistCommitment || "")
	      );
	    const cached = privateDeckManifestsRef.current.get(key);
	    if (cached) {
	      if (matchesPublishedManifest(cached)) return cached;
	      privateDeckManifestsRef.current.delete(key);
	    }
	    const stored = readStoredPrivateDeckManifest(matchId, owner);
	    if (stored?.slotSecrets) {
	      if (matchesPublishedManifest(stored)) {
	        privateDeckManifestsRef.current.set(key, stored);
	        preloadPrivateDeckManifestArt(stored);
	        return stored;
	      }
	      // Self-heal: drop the mislabeled manifest so future reads go straight
	      // to the published payload reconstruction.
	      try {
	        getPeerSessionStorage()?.removeItem(privateDeckManifestStorageKey(matchId, owner));
	      } catch {
	        // Ignore storage failures.
	      }
	    }
	    // Open-decklist matches publish every player's slot openings in the
	    // match-start payload, so any seat can reconstruct any owner's manifest.
	    if (payloadMatches) {
	      const slotSecrets = sanitizeDeckSlotOpenings(payloadPlayer?.deckSlotOpenings);
	      if (
	        sharedManifest
	        && Number(sharedManifest.owner) === normalizedOwner
	        && slotSecrets.length > 0
	        && slotSecrets.length === Number(sharedManifest.deckCount || 0)
	      ) {
	        const shared = { ...sharedManifest, slotSecrets };
	        privateDeckManifestsRef.current.set(key, shared);
	        return shared;
	      }
	    }
	    return null;
	  }, [currentAuditMatchId]);

  const rememberZiffleOpeningPosition = useCallback((owner, originalSlot, position) => {
    const normalizedOwner = Number(owner);
    const normalizedSlot = Number(originalSlot);
    const normalizedPosition = Number(position);
    if (
      !Number.isSafeInteger(normalizedOwner)
      || normalizedOwner < 0
      || !Number.isSafeInteger(normalizedSlot)
      || normalizedSlot < 0
      || !Number.isSafeInteger(normalizedPosition)
      || normalizedPosition < 0
    ) {
      return;
    }
    ziffleOpeningPositionsRef.current.set(
      `${normalizedOwner}:${normalizedSlot}`,
      normalizedPosition
    );
  }, []);

  const ziffleOpeningPositionForSlot = useCallback((owner, originalSlot) => {
    const normalizedOwner = Number(owner);
    const normalizedSlot = Number(originalSlot);
    if (
      !Number.isSafeInteger(normalizedOwner)
      || normalizedOwner < 0
      || !Number.isSafeInteger(normalizedSlot)
      || normalizedSlot < 0
    ) {
      return null;
    }
    const position = ziffleOpeningPositionsRef.current.get(`${normalizedOwner}:${normalizedSlot}`);
    return Number.isSafeInteger(position) && position >= 0 ? position : null;
  }, []);

	  const clearOwnerZiffleOpeningCache = useCallback((owner, matchId = currentAuditMatchId()) => {
	    const normalizedOwner = Number(owner);
	    if (!Number.isSafeInteger(normalizedOwner)) return;
	    if (normalizedOwner === Number(resolveLocalPlayerIndex(multiplayerRef.current))) {
	      ziffleHandRevealKeyRef.current = "";
	      ziffleHandRevealQuickKeyRef.current = "";
	    }
	    for (const key of [...ziffleOpeningPositionsRef.current.keys()]) {
	      if (key.startsWith(`${normalizedOwner}:`)) {
	        ziffleOpeningPositionsRef.current.delete(key);
      }
    }
    for (const key of [...ziffleRevealTokenCacheRef.current.keys()]) {
      if (key.startsWith(`${normalizedOwner}:`)) {
        ziffleRevealTokenCacheRef.current.delete(key);
      }
	    }
	    const normalizedMatchId = String(matchId || "");
	    for (const [key, opening] of [...localRevealedOpeningsRef.current.entries()]) {
	      if (
	        key.startsWith(`${normalizedMatchId}:`)
	        && Number(opening?.owner) === normalizedOwner
	      ) {
	        if (
	          key.startsWith(`${normalizedMatchId}:object:`)
	          || key.startsWith(`${normalizedMatchId}:owner:${normalizedOwner}:position:`)
	        ) {
	          localRevealedOpeningsRef.current.delete(key);
	          removeStoredRevealedOpening(key);
	        } else {
	          const stripped = stripTransientZifflePositionOpeningFields(opening);
	          localRevealedOpeningsRef.current.set(key, stripped);
	          writeStoredRevealedOpening(key, stripped);
	        }
	      }
	    }
	  }, [currentAuditMatchId]);

  const rememberLocalRevealedOpening = useCallback((opening, details = {}) => {
    if (!opening || opening.owner == null || opening.slot == null || !opening.card) return;
    const matchId = String(details.matchId || currentAuditMatchId());
    const writeEntry = (indexKey, entry) => {
      localRevealedOpeningsRef.current.set(indexKey, entry);
      writeStoredRevealedOpening(indexKey, entry);
    };
    const entryPositionCommitment = String(
      details.positionCommitment
      || details.publicCommitment
      || details.public_commitment
      || opening.positionCommitment
      || opening.position_commitment
      || opening.publicCommitment
      || opening.public_commitment
      || ""
    );
    const entryPosition = zifflePositionFromCommitment(entryPositionCommitment);
    const entryPublicSlot = Number(opening.publicSlot ?? opening.public_slot);
    const entry = {
      ...cloneMultiplayerPayload(opening),
      matchId,
      objectId:
        details.objectId != null
          ? Number(details.objectId)
          : opening.objectId != null
            ? Number(opening.objectId)
            : null,
      position:
        entryPosition != null
          ? entryPosition
          : details.position != null
            ? Number(details.position)
            : opening.position != null
              ? Number(opening.position)
              : Number.isSafeInteger(entryPublicSlot) && entryPublicSlot >= 0
                ? entryPublicSlot
                : null,
      positionCommitment: entryPositionCommitment,
      ziffleContext: String(details.ziffleContext || ziffleContextFromOpening(opening) || ""),
    };
    if (entry.objectId != null) {
      writeEntry(`${matchId}:object:${entry.objectId}`, entry);
    }
    writeEntry(
      `${matchId}:owner:${Number(entry.owner)}:slot:${Number(entry.slot)}`,
      entry
    );
    if (entry.commitment) {
      writeEntry(
        `${matchId}:owner:${Number(entry.owner)}:commitment:${entry.commitment}`,
        entry
      );
    }
    if (entry.positionCommitment) {
      writeEntry(
        `${matchId}:owner:${Number(entry.owner)}:position:${entry.positionCommitment}`,
        entry
      );
      if (entry.ziffleContext) {
        writeEntry(
          `${matchId}:owner:${Number(entry.owner)}:position:${entry.positionCommitment}:context:${entry.ziffleContext}`,
          entry
        );
      }
    }
  }, [currentAuditMatchId]);

  const localRevealedOpeningForExport = useCallback((exported) => {
    if (!exported || exported.owner == null) return null;
    const matchId = currentAuditMatchId();
    const objectId = exported.object_id ?? exported.objectId;
    const owner = Number(exported.owner);
    const commitment = String(exported.commitment || "");
    const readEntry = (indexKey) => {
      const cached = localRevealedOpeningsRef.current.get(indexKey);
      if (cached) return cached;
      const stored = readStoredRevealedOpening(indexKey);
      if (stored) {
        localRevealedOpeningsRef.current.set(indexKey, stored);
      }
      return stored;
    };
    const candidates = [];
    if (objectId != null) {
      candidates.push(readEntry(`${matchId}:object:${Number(objectId)}`));
    }
    if (commitment) {
      candidates.push(
        readEntry(`${matchId}:owner:${owner}:commitment:${commitment}`),
        readEntry(`${matchId}:owner:${owner}:position:${commitment}`)
      );
    }
    for (const candidate of candidates) {
      if (!candidate) continue;
      if (Number(candidate.owner) !== owner) continue;
      if (exported.card && String(candidate.card || "") !== String(exported.card || "")) continue;
      if (
        commitment
        && String(candidate.commitment || "") !== commitment
        && String(candidate.positionCommitment || "") !== commitment
      ) {
        continue;
      }
      return cloneMultiplayerPayload(candidate);
    }
    return null;
  }, [currentAuditMatchId]);

  const localRevealedOpeningForRequirement = useCallback((requirement) => {
    if (!requirement || requirement.owner == null) return null;
    const matchId = currentAuditMatchId();
    const objectId = requirement.objectId ?? requirement.object_id;
    const requirementObjectId = Number(objectId);
    const owner = Number(requirement.owner);
    const slot = requirement.slot == null ? null : Number(requirement.slot);
    const commitment = String(requirement.commitment || "");
    const positionCommitment = String(
      requirement.positionCommitment
      || requirement.position_commitment
      || requirement.publicCommitment
      || requirement.public_commitment
      || ""
    );
    const slotIsZifflePosition = Boolean(
      ziffleDeckHashFromCommitment(commitment)
      || ziffleDeckHashFromCommitment(positionCommitment)
    );
    const expectedPositionCommitment =
      positionCommitment
      || (ziffleDeckHashFromCommitment(commitment) ? commitment : "");
    const candidateMatchesPosition = (candidate) => {
      if (expectedPositionCommitment) {
        const candidateObjectId = Number(candidate?.objectId ?? candidate?.object_id);
        const objectIdMatches =
          Number.isSafeInteger(requirementObjectId)
          && requirementObjectId > 0
          && Number.isSafeInteger(candidateObjectId)
          && candidateObjectId === requirementObjectId;
        const commitmentMatches =
          commitment
          && String(candidate?.commitment || "") === commitment;
        if (objectIdMatches && commitmentMatches) return true;
        return String(candidate?.positionCommitment || "") === expectedPositionCommitment;
      }
      return !candidate?.positionCommitment;
    };
    const readEntry = (indexKey) => {
      const cached = localRevealedOpeningsRef.current.get(indexKey);
      if (cached) return cached;
      const stored = readStoredRevealedOpening(indexKey);
      if (stored) {
        localRevealedOpeningsRef.current.set(indexKey, stored);
      }
      return stored;
    };
    const candidates = [];
    if (commitment) {
      candidates.push(
        readEntry(`${matchId}:owner:${owner}:commitment:${commitment}`),
        readEntry(`${matchId}:owner:${owner}:position:${commitment}`)
      );
    }
    if (positionCommitment) {
      candidates.push(
        readEntry(`${matchId}:owner:${owner}:position:${positionCommitment}`)
      );
    }
    if (objectId != null) {
      candidates.push(readEntry(`${matchId}:object:${Number(objectId)}`));
    }
    if (slot != null && !slotIsZifflePosition) {
      candidates.push(readEntry(`${matchId}:owner:${owner}:slot:${slot}`));
    }
    for (const candidate of candidates) {
      if (!candidate) continue;
      if (Number(candidate.owner) !== owner) continue;
      if (slot != null && !slotIsZifflePosition && Number(candidate.slot) !== slot) continue;
      if (requirement.card && String(candidate.card || "") !== String(requirement.card || "")) {
        continue;
      }
      if (
        commitment
        && String(candidate.commitment || "") !== commitment
        && String(candidate.positionCommitment || "") !== commitment
      ) {
        continue;
      }
      if (!candidateMatchesPosition(candidate)) continue;
      return cloneMultiplayerPayload(candidate);
    }
    if (slot != null && slotIsZifflePosition) {
      for (const candidate of localRevealedOpeningsRef.current.values()) {
        if (!candidate) continue;
        if (Number(candidate.owner) !== owner) continue;
        if (Number(candidate.position) !== slot) continue;
        if (requirement.card && String(candidate.card || "") !== String(requirement.card || "")) {
          continue;
        }
        if (
          commitment
          && String(candidate.commitment || "") !== commitment
          && String(candidate.positionCommitment || "") !== commitment
        ) {
          continue;
        }
        if (!candidateMatchesPosition(candidate)) continue;
        return cloneMultiplayerPayload(candidate);
      }
    }
    return null;
  }, [currentAuditMatchId]);

		  const localRevealedOpeningForZiffleReveal = useCallback(({
		    owner,
		    ceremony,
	    shuffleOriginalSlot,
	    position,
	    card = "",
	    objectId = null,
	  } = {}) => {
	    const normalizedOwner = Number(owner);
	    const expectedCard = String(card || "");
	    const beforeOrder = normalizeShuffleOrder(ceremony?.beforeOrder ?? ceremony?.before_order);
	    const afterOrder = normalizeShuffleOrder(ceremony?.afterOrder ?? ceremony?.after_order);
		    const objectIds = [
		      objectId,
		      beforeOrder[Number(shuffleOriginalSlot)],
		      afterOrder[Number(position)],
	    ]
	      .map((entry) => Number(entry))
	      .filter((entry, index, list) =>
	        Number.isSafeInteger(entry)
	        && entry >= 0
	        && list.indexOf(entry) === index
		      );
		    const matchId = currentAuditMatchId();
		    const normalizedPosition = Number(position);
        const parsedShuffleOriginalSlot = Number(shuffleOriginalSlot);
        const normalizedShuffleOriginalSlot =
          Number.isSafeInteger(parsedShuffleOriginalSlot) && parsedShuffleOriginalSlot >= 0
            ? parsedShuffleOriginalSlot
            : null;
		    const expectedPositionCommitment =
		      ceremony?.deckHash && Number.isSafeInteger(normalizedPosition) && normalizedPosition >= 0
		        ? ziffleRuntimeCommitment(ceremony.deckHash, normalizedPosition)
		        : "";
		    const expectedZiffleContext = ziffleContextFromCeremony(ceremony);
			    const candidateMatchesCurrentPosition = (candidate) => {
			      if (!candidate) return false;
			      const candidateZiffleContext = ziffleContextFromOpening(candidate);
			      if (
			        expectedZiffleContext
			        && candidateZiffleContext
			        && candidateZiffleContext !== expectedZiffleContext
			      ) {
			        return false;
			      }
			      const candidatePositionCommitment = String(candidate.positionCommitment || "");
		      if (
		        candidatePositionCommitment
		        && expectedPositionCommitment
		        && candidatePositionCommitment !== expectedPositionCommitment
		      ) {
		        return false;
		      }
			      if (
			        candidate.position != null
			        && Number(candidate.position) !== normalizedPosition
			      ) {
			        return false;
			      }
			      if (ziffleCeremonyHasObjectOrder(ceremony)) {
			        const hasObjectIdentity = [
			          candidate.shuffleObjectId,
			          candidate.shuffle_object_id,
			          candidate.objectId,
			          candidate.object_id,
			        ].some((value) => {
			          const id = Number(value);
			          return Number.isSafeInteger(id) && id >= 0;
			        });
			        if (
			          ziffleObjectOrderLinksOpening(
			            ceremony,
			            shuffleOriginalSlot,
			            position,
			            candidate
			          )
			        ) {
			          return true;
			        }
			        return !hasObjectIdentity && Boolean(candidatePositionCommitment || candidate.position != null);
			      }
			      if (candidatePositionCommitment || candidate.position != null) return true;
			      return ziffleObjectOrderLinksOpening(
			        ceremony,
			        shuffleOriginalSlot,
		        position,
		        candidate
		      );
		    };
		    const candidates = [];
      const readOpeningEntry = (indexKey) => {
        const cached = localRevealedOpeningsRef.current.get(indexKey);
        if (cached) candidates.push(cached);
        const stored = readStoredRevealedOpening(indexKey);
        if (stored) {
          localRevealedOpeningsRef.current.set(indexKey, stored);
          candidates.push(stored);
        }
      };
	    for (const objectId of objectIds) {
        readOpeningEntry(`${matchId}:object:${objectId}`);
	    }
      if (expectedPositionCommitment) {
        if (expectedZiffleContext) {
          readOpeningEntry(
            `${matchId}:owner:${normalizedOwner}:position:${expectedPositionCommitment}:context:${expectedZiffleContext}`
          );
        }
        readOpeningEntry(`${matchId}:owner:${normalizedOwner}:position:${expectedPositionCommitment}`);
      }
      if (normalizedShuffleOriginalSlot != null) {
        readOpeningEntry(`${matchId}:owner:${normalizedOwner}:slot:${normalizedShuffleOriginalSlot}`);
      }
	    for (const cached of localRevealedOpeningsRef.current.values()) {
	      if (
          objectIds.includes(Number(cached?.objectId))
          || (
            expectedPositionCommitment
            && String(cached?.positionCommitment || "") === expectedPositionCommitment
          )
        ) {
	        candidates.push(cached);
	      }
	    }
		    for (const candidate of candidates) {
		      if (!candidate) continue;
		      if (Number(candidate.owner) !== normalizedOwner) continue;
		      if (expectedCard && String(candidate.card || "") !== expectedCard) continue;
		      if (candidate.slot == null || !candidate.card) continue;
		      if (!candidateMatchesCurrentPosition(candidate)) continue;
		      return cloneMultiplayerPayload(candidate);
		    }
	    return null;
	  }, [currentAuditMatchId]);

		  const publicDeckManifestForOwner = useCallback((owner) => {
    const normalized = Number(owner);
    return publicDeckManifest(
      multiplayerRef.current.players.find(
        (player) => Number(player.index) === normalized
      )?.deckAuditManifest
    );
  }, []);

  const publicKeyForAuditSigner = useCallback((signerIndex) => {
    const normalized = normalizePlayerIndex(signerIndex);
    if (normalized == null) return "";
    const player = multiplayerRef.current.players.find(
      (entry) => Number(entry.index) === normalized
    );
    return String(player?.auditPublicKey || "");
  }, []);

  const auditEncryptionPublicKeyForPlayer = useCallback((playerIndex) => {
    const normalized = normalizePlayerIndex(playerIndex);
    if (normalized == null) return "";
    const player = (matchStartPayloadRef.current?.players || multiplayerRef.current.players || []).find(
      (entry) => Number(entry.index) === normalized
    );
    return String(player?.auditEncryptionPublicKey || "");
  }, []);

  const signedZiffleKeysForPayload = useCallback((matchPayload = null) => {
    const payload = matchPayload || matchStartPayloadRef.current;
    if (Array.isArray(payload?.ziffleKeys) && payload.ziffleKeys.length > 0) {
      return cloneMultiplayerPayload(payload.ziffleKeys);
    }
    return zifflePublicKeysForPlayers(
      reindexPlayers(payload?.players || multiplayerRef.current.players || [])
    );
  }, [zifflePublicKeysForPlayers]);

  const matchPayloadCeremoniesForLookup = useCallback((options = {}) => {
    const payloads = [
      options.payload,
      options.payload === matchStartPayloadRef.current ? null : matchStartPayloadRef.current,
    ].filter(Boolean);
    return payloads.flatMap((payload) =>
      Array.isArray(payload?.ziffleCeremonies) ? payload.ziffleCeremonies : []
    );
  }, []);

  const hydrateZiffleCeremonyForLookup = useCallback((ceremony, options = {}) => {
    if (!ceremony || typeof ceremony !== "object") return ceremony;
	    const owner = Number(ceremony.owner);
	    const deckHash = String(ceremony.deckHash || "");
	    const context = String(ceremony.context || "");
	    const explicitFallback = [
	      ...(Array.isArray(options.ziffleCeremonies) ? options.ziffleCeremonies : []),
	      ...(Array.isArray(options.shuffleProofs) ? options.shuffleProofs : []),
	    ].find((entry) =>
	      Number(entry?.owner) === owner
	      && String(entry?.deckHash || "") === deckHash
	      && String(entry?.context || "") === context
	    );
	    const payloadFallback = matchPayloadCeremoniesForLookup(options).find((entry) =>
	      Number(entry?.owner) === owner
	      && String(entry?.deckHash || "") === deckHash
	      && String(entry?.context || "") === context
	    );
	    const keys = Array.isArray(ceremony.keys) && ceremony.keys.length > 0
	      ? ceremony.keys
	      : (
	        Array.isArray(explicitFallback?.keys) && explicitFallback.keys.length > 0
	          ? explicitFallback.keys
	          : (
	            Array.isArray(payloadFallback?.keys) && payloadFallback.keys.length > 0
	              ? payloadFallback.keys
	              : signedZiffleKeysForPayload(options.payload)
	          )
	      );
	    const steps = Array.isArray(ceremony.steps) && ceremony.steps.length > 0
	      ? ceremony.steps
	      : (
	        Array.isArray(explicitFallback?.steps) && explicitFallback.steps.length > 0
	          ? explicitFallback.steps
	          : (Array.isArray(payloadFallback?.steps) ? payloadFallback.steps : [])
	      );
    return {
      ...ceremony,
      keys: cloneMultiplayerPayload(keys || []),
      steps: cloneMultiplayerPayload(steps || []),
    };
  }, [matchPayloadCeremoniesForLookup, signedZiffleKeysForPayload]);

	  const rememberLocalZiffleCeremonyForLookup = useCallback((ceremony) => {
	    if (!ceremony || typeof ceremony !== "object") return;
	    const owner = Number(ceremony.owner);
	    const deckHash = String(ceremony.deckHash || "");
	    const context = String(ceremony.context || "");
	    if (!Number.isInteger(owner) || !deckHash) return;
	    const key = `${owner}:${deckHash}:${context}`;
	    if (localZiffleCeremonyLookupRef.current.has(key)) {
	      localZiffleCeremonyLookupRef.current.delete(key);
	    }
	    localZiffleCeremonyLookupRef.current.set(key, cloneMultiplayerPayload(ceremony));
	  }, []);

	  const ziffleCeremonyCandidatesForOwner = useCallback((owner, options = {}) => {
	    const normalizedOwner = Number(owner);
	    const deckHash = String(options.deckHash || ziffleDeckHashFromCommitment(options.commitment) || "");
	    const context = String(options.context || options.ziffleContext || options.ziffle_context || "");
	    const candidates = [];
	    const seen = new Set();
	    const addCandidate = (entry) => {
	      if (!entry || typeof entry !== "object") return;
	      if (Number(entry.owner) !== normalizedOwner) return;
	      if (deckHash && String(entry.deckHash || "") !== deckHash) return;
	      if (context && String(entry.context || "") !== context) return;
	      const key = [
	        Number(entry.owner),
	        String(entry.deckHash || ""),
	        String(entry.context || ""),
	        normalizeShuffleOrder(entry.beforeOrder ?? entry.before_order).join(","),
	        normalizeShuffleOrder(entry.afterOrder ?? entry.after_order).join(","),
	      ].join(":");
	      if (seen.has(key)) return;
	      seen.add(key);
	      candidates.push(hydrateZiffleCeremonyForLookup(entry, options));
	    };
	    for (const entry of [
	      ...(Array.isArray(options.ziffleCeremonies) ? options.ziffleCeremonies : []),
	      ...(Array.isArray(options.shuffleProofs) ? options.shuffleProofs : []),
	    ]) {
	      addCandidate(entry);
	    }
	    addCandidate(liveZiffleCeremoniesRef.current.get(normalizedOwner));
	    [...localZiffleCeremonyLookupRef.current.values()].reverse().forEach(addCandidate);
	    for (const entry of matchPayloadCeremoniesForLookup(options)) {
	      addCandidate(entry);
	    }
	    return candidates;
	  }, [hydrateZiffleCeremonyForLookup, matchPayloadCeremoniesForLookup]);

  const ziffleCeremonyForOwner = useCallback((owner, options = {}) => {
	    const candidates = ziffleCeremonyCandidatesForOwner(owner, options);
	    if (candidates.length === 0) return null;
	    const withSteps = candidates.find((entry) =>
	      Array.isArray(entry?.steps) && entry.steps.length > 0
	    );
	    const withKeys = candidates.find((entry) =>
	      Array.isArray(entry?.keys) && entry.keys.length > 0
	    );
	    const withObjectOrder = candidates.find((entry) => ziffleCeremonyHasObjectOrder(entry));
	    if (withObjectOrder) {
	      return {
	        ...withObjectOrder,
	        keys: cloneMultiplayerPayload(
	          Array.isArray(withObjectOrder.keys) && withObjectOrder.keys.length > 0
	            ? withObjectOrder.keys
	            : withKeys?.keys || []
	        ),
	        steps: cloneMultiplayerPayload(
	          Array.isArray(withObjectOrder.steps) && withObjectOrder.steps.length > 0
	            ? withObjectOrder.steps
	            : withSteps?.steps || []
	        ),
	      };
	    }
	    return withSteps || candidates[0];
	  }, [ziffleCeremonyCandidatesForOwner]);

  function zifflePositionForObjectId(owner, objectId, options = {}) {
    const normalizedObjectId = Number(objectId);
    if (!Number.isSafeInteger(normalizedObjectId) || normalizedObjectId < 0) return null;
    for (const ceremony of ziffleCeremonyCandidatesForOwner(owner, options)) {
      if (!ceremony?.deckHash) continue;
      const afterOrder = normalizeShuffleOrder(ceremony.afterOrder ?? ceremony.after_order);
      const position = afterOrder.findIndex((entry) => Number(entry) === normalizedObjectId);
      if (position < 0) continue;
      return {
        ceremony,
        position,
        positionCommitment: ziffleRuntimeCommitment(ceremony.deckHash, position),
        ziffleContext: ziffleContextFromCeremony(ceremony),
      };
    }
    return null;
  }

	  function zifflePositionForOriginalSlot(owner, originalSlot, options = {}) {
	    const normalizedSlot = Number(originalSlot);
	    if (!Number.isSafeInteger(normalizedSlot) || normalizedSlot < 0) return null;
	    const matches = [];
	    for (const ceremony of ziffleCeremonyCandidatesForOwner(owner, options)) {
	      if (!ceremony?.deckHash) continue;
	      const beforeOrder = normalizeShuffleOrder(ceremony.beforeOrder ?? ceremony.before_order);
	      const afterOrder = normalizeShuffleOrder(ceremony.afterOrder ?? ceremony.after_order);
        const shuffleObjectId = Number(beforeOrder[normalizedSlot]);
        if (!Number.isSafeInteger(shuffleObjectId) || shuffleObjectId < 0) continue;
        const position = afterOrder.findIndex((entry) => Number(entry) === shuffleObjectId);
        if (position < 0) continue;
        matches.push({
          ceremony,
          position,
          shuffleObjectId,
          positionCommitment: ziffleRuntimeCommitment(ceremony.deckHash, position),
          ziffleContext: ziffleContextFromCeremony(ceremony),
        });
      }
      if (matches.length === 0) return null;
      const scoped = Boolean(
        options.context
        || options.ziffleContext
        || options.ziffle_context
        || options.deckHash
        || ziffleDeckHashFromCommitment(options.commitment)
      );
      if (scoped || matches.length === 1 || options.allowAmbiguousOriginalSlot === true) {
        return matches[0];
      }
      return null;
  }

  function ziffleTokensForPosition(tokens = [], position = null) {
    return (Array.isArray(tokens) ? tokens : [])
      .filter((token) =>
        position == null
        || token?.cardPosition == null
        || Number(token.cardPosition) === Number(position)
      )
      .map((token) => ({
        player: Number(token.player),
        publicKeyHex: String(token.publicKeyHex || ""),
        tokenHex: String(token.tokenHex || ""),
        proofHex: String(token.proofHex || ""),
      }));
  }

  function ziffleRevealTokenCacheKey(ceremony, player, position) {
    return [
      Number(ceremony?.owner ?? -1),
      ziffleKeyContextForCeremony(ceremony),
      String(ceremony?.context || ""),
      String(ceremony?.deckHash || ""),
      Number(player),
      Number(position),
    ].join(":");
  }

  function normalizeZiffleRevealToken(token, fallbackPosition = null) {
    if (!token || typeof token !== "object") return null;
    const position = Number(token.cardPosition ?? fallbackPosition);
    const player = Number(token.player);
    if (!Number.isSafeInteger(position) || position < 0 || !Number.isSafeInteger(player)) {
      return null;
    }
    return {
      player,
      publicKeyHex: String(token.publicKeyHex || ""),
      tokenHex: String(token.tokenHex || ""),
      proofHex: String(token.proofHex || ""),
      cardPosition: position,
    };
  }

  function rememberZiffleRevealTokens(ceremony, tokens = [], fallbackPositions = []) {
    const tokenList = Array.isArray(tokens) ? tokens : [tokens];
    const fallbackPosition = fallbackPositions.length === 1 ? fallbackPositions[0] : null;
    for (const token of tokenList) {
      const normalized = normalizeZiffleRevealToken(token, fallbackPosition);
      if (!normalized) continue;
      ziffleRevealTokenCacheRef.current.set(
        ziffleRevealTokenCacheKey(ceremony, normalized.player, normalized.cardPosition),
        normalized
      );
    }
  }

  function cachedZiffleRevealTokens(ceremony, player, positions = []) {
    const tokens = [];
    for (const position of positions) {
      const token = ziffleRevealTokenCacheRef.current.get(
        ziffleRevealTokenCacheKey(ceremony, player, position)
      );
      if (!token) return null;
      tokens.push({ ...token });
    }
    return tokens;
  }

	  function openingNeedsZiffleProof(opening) {
	    if (!opening) return false;
	    const proof = opening.ziffleReveal || opening.ziffleProof || opening.positionOpeningProof;
	    const positionCommitment = String(opening.positionCommitment || proof?.positionCommitment || "");
	    if (!ziffleDeckHashFromCommitment(positionCommitment)) return false;
	    if (proof) return true;
    const ceremony = ziffleCeremonyForOwner(opening.owner, {
      commitment: positionCommitment,
      context: ziffleContextFromOpening(opening),
    });
    if (ziffleCeremonyHasObjectOrder(ceremony)) {
      const position = Number(
        zifflePositionFromCommitment(positionCommitment) ?? opening.position
      );
      return !ziffleObjectOrderLinksOpening(ceremony, opening.slot, position, opening);
    }
	    return true;
	  }

  function ziffleCeremonyHasObjectOrder(ceremony) {
    return normalizeShuffleOrder(ceremony?.beforeOrder ?? ceremony?.before_order).length > 0
      || normalizeShuffleOrder(ceremony?.afterOrder ?? ceremony?.after_order).length > 0;
  }

  function ziffleShuffleObjectIdForPosition(ceremony, position) {
    const normalizedPosition = Number(position);
    if (!Number.isSafeInteger(normalizedPosition) || normalizedPosition < 0) return null;
    const afterOrder = normalizeShuffleOrder(ceremony?.afterOrder ?? ceremony?.after_order);
    const objectId = Number(afterOrder[normalizedPosition]);
    return Number.isSafeInteger(objectId) && objectId >= 0 ? objectId : null;
  }

  function ziffleShuffleOriginalSlotForPosition(ceremony, position, objectId = null) {
    const normalizedPosition = Number(position);
    if (!Number.isSafeInteger(normalizedPosition) || normalizedPosition < 0) return null;
    const beforeOrder = normalizeShuffleOrder(ceremony?.beforeOrder ?? ceremony?.before_order);
    const afterOrder = normalizeShuffleOrder(ceremony?.afterOrder ?? ceremony?.after_order);
    const positionObjectId = Number(afterOrder[normalizedPosition]);
    const explicitObjectId = Number(objectId);
    const targetObjectId =
      Number.isSafeInteger(positionObjectId) && positionObjectId >= 0
        ? positionObjectId
        : Number.isSafeInteger(explicitObjectId) && explicitObjectId >= 0
          ? explicitObjectId
          : null;
    if (targetObjectId == null) return null;
    if (beforeOrder.length === 0) {
      return afterOrder.length > 0 ? normalizedPosition : null;
    }
    const beforeIndex = beforeOrder.findIndex((entry) => Number(entry) === targetObjectId);
    return beforeIndex >= 0 ? beforeIndex : null;
  }

  function ziffleObjectOrderLinksOpening(ceremony, shuffleOriginalSlot, position, opening) {
    const proof = opening?.ziffleReveal || opening?.ziffleProof || opening?.positionOpeningProof || {};
    const normalizedShuffleOriginalSlot = Number(shuffleOriginalSlot);
    const hasShuffleOriginalSlot =
      Number.isSafeInteger(normalizedShuffleOriginalSlot) && normalizedShuffleOriginalSlot >= 0;
    const normalizedPosition = Number(position);
    const hasPosition = Number.isSafeInteger(normalizedPosition) && normalizedPosition >= 0;
    const beforeOrder = normalizeShuffleOrder(ceremony?.beforeOrder ?? ceremony?.before_order);
    const afterOrder = normalizeShuffleOrder(ceremony?.afterOrder ?? ceremony?.after_order);
    if (beforeOrder.length === 0 && afterOrder.length === 0) return false;
    const beforeObjectId = hasShuffleOriginalSlot ? Number(beforeOrder[normalizedShuffleOriginalSlot]) : NaN;
    const afterObjectId = hasPosition ? Number(afterOrder[normalizedPosition]) : NaN;
    if (
      hasShuffleOriginalSlot
      && hasPosition
      && Number.isSafeInteger(beforeObjectId)
      && beforeObjectId >= 0
      && Number.isSafeInteger(afterObjectId)
      && afterObjectId >= 0
      && beforeObjectId === afterObjectId
    ) {
      return true;
    }
    const normalizedId = (value) => {
      const id = Number(value);
      return Number.isSafeInteger(id) && id >= 0 ? id : null;
    };
    const shuffleObjectId = normalizedId(
      proof?.shuffleObjectId
      ?? proof?.shuffle_object_id
      ?? opening?.shuffleObjectId
      ?? opening?.shuffle_object_id
    );
    const objectId = normalizedId(
      proof?.objectId
      ?? proof?.object_id
      ?? opening?.objectId
      ?? opening?.object_id
    );
    const beforeExpectedObjectId = shuffleObjectId ?? objectId;
    const afterExpectedObjectId = objectId ?? shuffleObjectId;
    if (beforeExpectedObjectId == null || afterExpectedObjectId == null) return false;
    const beforeMatches =
      beforeOrder.length === 0
      || (
        hasShuffleOriginalSlot
        && beforeObjectId === beforeExpectedObjectId
      );
    const afterMatches =
      afterOrder.length === 0
      || (
        hasPosition
        && afterObjectId === afterExpectedObjectId
      );
    return beforeMatches && afterMatches;
  }

  function ziffleRevealMatchesOpening(ceremony, revealOriginalSlot, position, opening) {
    if (Number(revealOriginalSlot) === Number(opening?.slot)) {
      return true;
    }
    const beforeOrder = normalizeShuffleOrder(ceremony?.beforeOrder ?? ceremony?.before_order);
    const afterOrder = normalizeShuffleOrder(ceremony?.afterOrder ?? ceremony?.after_order);
    if (
      beforeOrder.length === 0
      && afterOrder.length === 0
      && Number(revealOriginalSlot) === Number(opening?.slot)
    ) {
      return true;
    }
    return ziffleObjectOrderLinksOpening(ceremony, revealOriginalSlot, position, opening);
  }

  function ziffleOpeningProofHasAuthenticatedObjectOrder(ceremony, opening, position) {
    return ceremony?.authenticatedOrder === true
      && ziffleCeremonyHasObjectOrder(ceremony)
      && ziffleObjectOrderLinksOpening(ceremony, opening.slot, position, opening);
  }

  async function verifyZiffleOpeningProofForOpening(opening, options = {}) {
    if (!openingNeedsZiffleProof(opening)) return;
    const proof = opening.ziffleReveal || opening.ziffleProof || opening.positionOpeningProof;
    if (!proof || typeof proof !== "object") {
      throw new Error("Ziffle card opening is missing its reveal proof");
    }
    if (String(proof.type || "") !== "ziffle_position_opening_v1") {
      throw new Error("Ziffle card opening proof type is unsupported");
    }
    const position = Number(
      zifflePositionFromCommitment(opening.positionCommitment)
      ?? opening.position
      ?? proof.position
    );
    if (!Number.isSafeInteger(position) || position < 0) {
      throw new Error("Ziffle card opening is missing a valid shuffled position");
    }
    if (Number(proof.owner) !== Number(opening.owner)) {
      throw new Error("Ziffle card opening proof owner mismatch");
    }
    if (Number(proof.position) !== position) {
      throw new Error("Ziffle card opening proof position mismatch");
    }
    if (Number(proof.originalSlot) !== Number(opening.slot)) {
      throw new Error("Ziffle card opening proof slot mismatch");
    }
	    const storedCeremony = ziffleCeremonyForOwner(opening.owner, {
	      payload: options.payload || matchStartPayloadRef.current,
	      shuffleProofs: options.shuffleProofs || [],
	      ziffleCeremonies: options.ziffleCeremonies || [],
	      commitment: opening.positionCommitment || proof.positionCommitment,
	      deckHash: proof.deckHash,
	      context: ziffleContextFromOpening(opening) || proof.context,
	    });
	    const ceremony = hydrateZiffleCeremonyForLookup(
	      ziffleCeremonyForOpeningProof(proof, storedCeremony),
	      {
	        payload: options.payload || matchStartPayloadRef.current,
	        shuffleProofs: options.shuffleProofs || [],
	        ziffleCeremonies: options.ziffleCeremonies || [],
	      }
	    );
	    if (!ceremony) {
	      throw new Error(`Missing ziffle ceremony for opening player ${Number(opening.owner) + 1}`);
	    }
    if (ziffleCeremonyHasObjectOrder(ceremony)) {
      if (ziffleOpeningProofHasAuthenticatedObjectOrder(ceremony, opening, position)) {
        return;
      }
      if (!proof && ziffleObjectOrderLinksOpening(ceremony, opening.slot, position, opening)) {
        return;
      }
      if (!proof) {
        const beforeOrder = normalizeShuffleOrder(ceremony.beforeOrder ?? ceremony.before_order);
        const afterOrder = normalizeShuffleOrder(ceremony.afterOrder ?? ceremony.after_order);
        const slotObjectId = Number(beforeOrder[Number(opening.slot)]);
        const positionObjectId = Number(afterOrder[position]);
        const openingObjectId = Number(
          opening.shuffleObjectId
          ?? opening.shuffle_object_id
          ?? opening.objectId
          ?? opening.object_id
        );
        if (
          !Number.isSafeInteger(slotObjectId)
          || !Number.isSafeInteger(positionObjectId)
          || !Number.isSafeInteger(openingObjectId)
        ) {
          throw new Error("Ziffle card opening object order does not match reveal");
        }
        throw new Error("Ziffle card opening object order does not match reveal");
      }
    }
    const positionCommitment =
      String(opening.positionCommitment || proof.positionCommitment || "")
      || ziffleRuntimeCommitment(ceremony.deckHash, position);
    if (positionCommitment !== ziffleRuntimeCommitment(ceremony.deckHash, position)) {
      throw new Error("Ziffle card opening position commitment mismatch");
    }
    if (String(proof.positionCommitment || "") !== positionCommitment) {
      throw new Error("Ziffle card opening proof commitment mismatch");
    }
    if (String(proof.context || "") !== String(ceremony.context || "")) {
      throw new Error("Ziffle card opening proof context mismatch");
    }
    if (String(proof.keyContext || proof.context || "") !== ziffleKeyContextForCeremony(ceremony)) {
      throw new Error("Ziffle card opening proof key context mismatch");
    }
    if (String(proof.deckHash || "") !== String(ceremony.deckHash || "")) {
      throw new Error("Ziffle card opening proof deck hash mismatch");
    }
    if (Number(proof.deckCount) !== Number(ceremony.deckCount)) {
      throw new Error("Ziffle card opening proof deck count mismatch");
    }
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.ziffleRevealCard !== "function") {
      throw new Error("Ziffle opening reveal backend is not available");
    }
    const tokens = ziffleTokensForPosition(proof.tokens || [], position);
    if (tokens.length === 0) {
      throw new Error("Ziffle card opening proof is missing reveal tokens");
    }
    const reveal = await currentGame.ziffleRevealCard({
      deckCount: Number(ceremony.deckCount),
      context: String(ceremony.context || ""),
      keyContext: ziffleKeyContextForCeremony(ceremony),
      keys: cloneMultiplayerPayload(ceremony.keys || []),
      steps: cloneMultiplayerPayload(ceremony.steps || []),
      cardPosition: position,
      tokens,
    });
    const revealOriginalSlot = Number(reveal.originalSlot);
    const proofShuffleOriginalSlot = Number(proof.shuffleOriginalSlot ?? proof.originalSlot);
    if (revealOriginalSlot !== proofShuffleOriginalSlot) {
      throw new Error(
        `Ziffle card opening proof reveals a different shuffle slot `
        + `(owner ${Number(opening.owner)}, position ${position}, proof slot ${proofShuffleOriginalSlot}, `
        + `revealed slot ${revealOriginalSlot}, card ${String(opening.card || "")})`
      );
    }
	    if (!ziffleRevealMatchesOpening(ceremony, revealOriginalSlot, position, opening)) {
	      throw new Error(
	        `Ziffle card opening proof reveals a different committed slot `
	        + `(owner ${Number(opening.owner)}, position ${position}, opening slot ${Number(opening.slot)}, `
	        + `revealed slot ${Number(reveal.originalSlot)}, card ${String(opening.card || "")})`
	      );
	    }
  }

	  async function ensureZiffleOpeningProof(opening, options = {}) {
	    const openingPositionCommitment = String(
	      opening?.positionCommitment || opening?.position_commitment || ""
	    );
	    const openingPosition = Number(
	      zifflePositionFromCommitment(openingPositionCommitment) ?? opening?.position
	    );
	    const openingHasZiffleIdentity = Boolean(
	      ziffleDeckHashFromCommitment(openingPositionCommitment)
	      && Number.isSafeInteger(openingPosition)
	      && openingPosition >= 0
	    );
	    if (!options.forceZiffleOpeningProof && !openingNeedsZiffleProof(opening)) return opening;
	    if (options.forceZiffleOpeningProof && !openingHasZiffleIdentity) return opening;
	    const currentGame = gameRef.current;
	    if (!currentGame || typeof currentGame.ziffleRevealCard !== "function") {
	      throw new Error("Ziffle opening reveal backend is not available");
	    }
	    const existingProof = opening.ziffleReveal || opening.ziffleProof || opening.positionOpeningProof;
	    if (existingProof) {
	      if (options.skipFreshZiffleOpeningProofVerification && !options.forceZiffleOpeningProof) {
	        return opening;
	      }
	      try {
	        await verifyZiffleOpeningProofForOpening(opening);
	        return opening;
	      } catch (err) {
	        const message = String(err?.message || err || "");
	        const canRebuildStaleProof =
	          !options._rebuiltStaleZiffleProof
	          && (
	            message.includes("reveals a different committed slot")
	            || message.includes("reveals a different shuffle slot")
	            || message.includes("object order does not match reveal")
	          );
	        if (!canRebuildStaleProof) {
	          throw err;
	        }
	        const rebuiltOpening = { ...opening };
	        delete rebuiltOpening.ziffleReveal;
	        delete rebuiltOpening.ziffleProof;
	        delete rebuiltOpening.positionOpeningProof;
	        return ensureZiffleOpeningProof(rebuiltOpening, {
	          ...options,
	          _rebuiltStaleZiffleProof: true,
	          forceZiffleOpeningProof: true,
	        });
	      }
	    }
	    const position = openingPosition;
    if (!Number.isSafeInteger(position) || position < 0) {
      throw new Error("Ziffle card opening is missing a valid shuffled position");
    }
    const ceremony = ziffleCeremonyForOwner(opening.owner, {
      commitment: opening.positionCommitment,
      context: ziffleContextFromOpening(opening),
    });
    if (!ceremony) {
      throw new Error(`Missing ziffle ceremony for opening player ${Number(opening.owner) + 1}`);
    }
    const tokens = await collectZiffleRevealTokens(ceremony, position, options);
    const reveal = await currentGame.ziffleRevealCard({
      deckCount: Number(ceremony.deckCount),
      context: String(ceremony.context || ""),
      keyContext: ziffleKeyContextForCeremony(ceremony),
      keys: cloneMultiplayerPayload(ceremony.keys || []),
      steps: cloneMultiplayerPayload(ceremony.steps || []),
      cardPosition: position,
      tokens,
    });
    const revealOriginalSlot = Number(reveal.originalSlot);
	    const positionCommitment =
	      String(opening.positionCommitment || "")
	      || ziffleRuntimeCommitment(ceremony.deckHash, position);
	    let proofOpening = opening;
	    let proofOriginalSlot = Number(opening.slot);
	    const manifest = privateDeckManifestForOwner(opening.owner);
	    if (
	      Number(opening.slot) !== Number(revealOriginalSlot)
	      || !ziffleRevealMatchesOpening(ceremony, revealOriginalSlot, position, opening)
	    ) {
	      const beforeOrder = normalizeShuffleOrder(ceremony.beforeOrder ?? ceremony.before_order);
	      const afterOrder = normalizeShuffleOrder(ceremony.afterOrder ?? ceremony.after_order);
	      const shuffleObjectId = Number(beforeOrder[revealOriginalSlot]);
	      const positionObjectId = Number(afterOrder[position]);
	      const orderLinkedOpening = {
        ...opening,
        ...(Number.isSafeInteger(shuffleObjectId) && shuffleObjectId >= 0
          ? { shuffleObjectId }
          : {}),
      };
	      if (
	        Number(opening.slot) === Number(revealOriginalSlot)
	        && Number.isSafeInteger(shuffleObjectId)
	        && shuffleObjectId >= 0
	        && Number(positionObjectId) === Number(shuffleObjectId)
	        && ziffleRevealMatchesOpening(ceremony, revealOriginalSlot, position, orderLinkedOpening)
	      ) {
	        proofOpening = orderLinkedOpening;
	      } else {
	      const resolvedRevealSlot = await resolveCommittedZiffleRevealSlot({
	        owner: opening.owner,
	        ceremony,
	        shuffleOriginalSlot: revealOriginalSlot,
	        shuffleOriginalSlotIsVerified: true,
        position,
        card: opening.card || "",
        objectId: opening.objectId,
        manifest,
        options,
      });
      if (!resolvedRevealSlot) {
        throw new Error(
          `Ziffle card opening proof reveals a different committed slot `
          + `(owner ${Number(opening.owner)}, position ${position}, opening slot ${Number(opening.slot)}, `
          + `revealed slot ${Number(reveal.originalSlot)}, card ${String(opening.card || "")})`
        );
      }
      proofOriginalSlot = Number(resolvedRevealSlot.slot);
      const rebuiltOpening = await buildDeckSlotOpening({
        manifest,
        slot: proofOriginalSlot,
        card: resolvedRevealSlot.card || opening.card,
      });
      proofOpening = {
        ...opening,
        ...rebuiltOpening,
        ...(resolvedRevealSlot.objectId != null ? { objectId: Number(resolvedRevealSlot.objectId) } : {}),
        ...(resolvedRevealSlot.shuffleObjectId != null || resolvedRevealSlot.objectId != null
          ? { shuffleObjectId: Number(resolvedRevealSlot.shuffleObjectId ?? resolvedRevealSlot.objectId) }
          : {}),
        reportedSlot: Number(opening.slot),
      };
      }
    }
    return {
      ...proofOpening,
      position,
      positionCommitment,
      ziffleContext: ziffleContextFromCeremony(ceremony),
      ziffleReveal: buildZiffleOpeningProof({
        opening: {
          ...proofOpening,
          position,
          positionCommitment,
          ziffleContext: ziffleContextFromCeremony(ceremony),
        },
        ceremony,
        position,
        originalSlot: proofOriginalSlot,
        shuffleOriginalSlot: revealOriginalSlot,
        positionCommitment,
        tokens,
        compact: true,
      }),
    };
  }

  const localZiffleDiagnostics = useCallback((label = "local") => {
    const session = multiplayerRef.current;
    return {
      label,
      localPeerId: String(session.localPeerId || ""),
      role: String(session.role || ""),
      mode: String(session.mode || ""),
      matchStarted: Boolean(session.matchStarted),
      localPlayerIndex:
        session.localPlayerIndex == null ? null : Number(session.localPlayerIndex),
      lobbyId: String(session.lobbyId || ""),
      hostPeerId: String(session.hostPeerId || ""),
      auditMatchId: String(matchStartPayloadRef.current?.auditMatchId || ""),
      matchStartPayloadPresent: Boolean(matchStartPayloadRef.current),
      payloadCeremonies: (matchStartPayloadRef.current?.ziffleCeremonies || [])
        .map(compactZiffleCeremonyForDiagnostics)
        .filter(Boolean),
      liveCeremonies: [...liveZiffleCeremoniesRef.current.values()]
        .map(compactZiffleCeremonyForDiagnostics)
        .filter(Boolean),
      players: (session.players || []).map((player) => ({
        index: Number(player.index),
        name: String(player.name || ""),
        peerId: String(player.peerId || ""),
        connected: player.connected !== false,
        hasZiffleKey: Boolean(player.ziffleKey),
      })),
    };
  }, []);

  const emitZiffleDiagnosticNotice = useCallback((title, err, diagnostics = null) => {
    const message = toErrorMessage(err, "Unknown ziffle ceremony");
    const mergedDiagnostics = {
      message,
      local: localZiffleDiagnostics(title),
      ...(err?.ziffleDiagnostics && typeof err.ziffleDiagnostics === "object"
        ? { error: err.ziffleDiagnostics }
        : {}),
      ...(diagnostics && typeof diagnostics === "object" ? diagnostics : {}),
    };
    const json = compactZiffleDiagnosticsJson(mergedDiagnostics);
    emitSyncFailureNotice(title, {
      body: ziffleDiagnosticNoticeBody(message, mergedDiagnostics),
      copyText: json,
      copyStatusMessage: "Copied Ziffle diagnostics",
      actions: [
        {
          label: "Copy diagnostics",
          copyText: json,
          copyStatusMessage: "Copied Ziffle diagnostics",
        },
      ],
    });
    console.warn("Ironsmith Ziffle diagnostics", mergedDiagnostics);
  }, [localZiffleDiagnostics]);

  const importCachedAuditPublicKey = useCallback(async (rawHex) => {
    const normalized = String(rawHex || "").trim();
    if (!normalized) {
      throw new Error("Missing audit public key");
    }
    if (!auditVerifyKeyCacheRef.current.has(normalized)) {
      auditVerifyKeyCacheRef.current.set(
        normalized,
        await importAuditPublicKey(normalized)
      );
    }
    return auditVerifyKeyCacheRef.current.get(normalized);
  }, []);

  async function signActionIntentForCommand({
    seq,
    actorIndex,
    command,
    prevStateHash,
    preActionPublicCheckpointHash,
  }) {
    const { keyPair } = await ensureAuditIdentity();
    const payload = signedActionIntentPayload({
      matchId: currentAuditMatchId(),
      seq,
      actorIndex,
      prevStateHash,
      preActionPublicCheckpointHash,
      command,
    });
    return {
      ...payload,
      signatureAlgorithm: "ecdsa-p256-sha256",
      signature: await signAuditPayload(keyPair, payload),
    };
  }

  async function verifySignedActionIntent(intent, expected = {}) {
    if (!intent || typeof intent !== "object") {
      throw new Error("Cryptographic material request is missing a signed action intent");
    }
    const payload = signedActionIntentPayload(intent);
    const expectedPayload = signedActionIntentPayload({
      matchId: expected.matchId ?? currentAuditMatchId(),
      seq: expected.seq ?? payload.seq,
      actorIndex: expected.actorIndex ?? payload.actorIndex,
      prevStateHash: expected.prevStateHash ?? payload.prevStateHash,
      preActionPublicCheckpointHash:
        expected.preActionPublicCheckpointHash
        ?? expected.publicCheckpointHash
        ?? payload.preActionPublicCheckpointHash,
      command: expected.command ?? payload.command,
    });
    if (
      payload.domain !== ACTION_INTENT_DOMAIN
      || payload.matchId !== expectedPayload.matchId
      || Number(payload.seq) !== Number(expectedPayload.seq)
      || Number(payload.actorIndex) !== Number(expectedPayload.actorIndex)
      || payload.prevStateHash !== expectedPayload.prevStateHash
      || payload.preActionPublicCheckpointHash !== expectedPayload.preActionPublicCheckpointHash
      || canonicalMultiplayerPayload(payload.command) !== canonicalMultiplayerPayload(expectedPayload.command)
    ) {
      throw new Error("Signed action intent does not match the requested action");
    }
    const publicKey = await importCachedAuditPublicKey(publicKeyForAuditSigner(payload.actorIndex));
    const valid = await verifyAuditPayload(publicKey, payload, intent.signature || "");
    if (!valid) {
      throw new Error("Signed action intent signature is invalid");
    }
    return {
      ...payload,
      signatureAlgorithm: "ecdsa-p256-sha256",
      signature: String(intent.signature || ""),
    };
  }

  const IGNORED_ACTION_INTENT_TTL_MS = 10 * 60 * 1000;
  const MAX_IGNORED_ACTION_INTENTS = 256;

  function pruneIgnoredActionIntents(nowMs = Date.now()) {
    for (const [key, record] of ignoredActionIntentKeysRef.current.entries()) {
      if (nowMs - Number(record?.at || 0) > IGNORED_ACTION_INTENT_TTL_MS) {
        ignoredActionIntentKeysRef.current.delete(key);
      }
    }
    while (ignoredActionIntentKeysRef.current.size > MAX_IGNORED_ACTION_INTENTS) {
      const oldestKey = ignoredActionIntentKeysRef.current.keys().next().value;
      if (!oldestKey) break;
      ignoredActionIntentKeysRef.current.delete(oldestKey);
    }
  }

  function rememberIgnoredActionIntentKey(actionIntentKeyValue, reason = "") {
    const key = String(actionIntentKeyValue || "");
    if (!key) return false;
    const nowMs = Date.now();
    ignoredActionIntentKeysRef.current.set(key, {
      reason: String(reason || ""),
      at: nowMs,
    });
    pruneIgnoredActionIntents(nowMs);
    clearPeerWaitForActionIntent(key);
    return true;
  }

  function ignoredActionIntentReason(actionIntentKeyValue) {
    const key = String(actionIntentKeyValue || "");
    if (!key) return "";
    pruneIgnoredActionIntents();
    return String(ignoredActionIntentKeysRef.current.get(key)?.reason || "");
  }

  function actionIntentKeyFromProtocolPayload(payload = {}) {
    try {
      const intent =
        payload?.actionIntent
        || payload?.actionAuthorization?.actionIntent
        || payload?.action_authorization?.actionIntent
        || payload?.action_authorization?.action_intent
        || null;
      return intent ? actionIntentKey(intent) : "";
    } catch {
      return "";
    }
  }

  function actionIntentKeyFromProtocolClaim(claim = {}) {
    return actionIntentKeyFromProtocolPayload(claim.requestPayload || claim.request_payload || claim);
  }

  function protocolActionIntentInactiveReason(actionIntentKeyValue = "") {
    const ignoredReason = ignoredActionIntentReason(actionIntentKeyValue);
    if (ignoredReason) return ignoredReason;
    const session = multiplayerRef.current;
    if (session.mode === "disputed") return "match_disputed";
    if (!session.matchStarted) return "match_not_started";
    return "";
  }

  function isDirectProtocolMessage(message = {}) {
    return [
      "ziffle_shuffle_step_request",
      "ziffle_shuffle_step_response",
      "ziffle_reveal_token_request",
      "ziffle_reveal_token_response",
      "rng_commit_request",
      "rng_commit_response",
      "rng_reveal_request",
      "rng_reveal_response",
      "timeout_vote_request",
      "timeout_vote_response",
      "disconnect_forfeit_vote_request",
      "disconnect_forfeit_vote_response",
      "protocol_timeout_vote_request",
      "protocol_timeout_vote_response",
      "action_quorum_vote_request",
      "action_quorum_vote_response",
      "crypto_material_request",
      "crypto_material_response",
      "action_intent_progress",
      "action_intent_cancel",
    ].includes(String(message?.type || ""));
  }

  function shouldSuppressProtocolMessageError(err, message = {}) {
    if (!isDirectProtocolMessage(message)) return false;
    const key = actionIntentKeyFromProtocolPayload(message);
    const inactiveReason = protocolActionIntentInactiveReason(key);
    const messageText = toErrorMessage(err);
    if (inactiveReason) {
      recordPeerSyncPerf("protocol_message_error:suppressed", {
        message_type: String(message?.type || ""),
        request_id: String(message?.requestId || ""),
        action_intent_key: key,
        reason: inactiveReason,
        error: messageText,
      });
      return true;
    }
    if (
      key
      && messageText.includes("Match clock hash chain does not match local transcript")
      && ignoredActionIntentReason(key)
    ) {
      recordPeerSyncPerf("protocol_message_error:suppressed", {
        message_type: String(message?.type || ""),
        request_id: String(message?.requestId || ""),
        action_intent_key: key,
        reason: "ignored_action_intent_clock_hash",
        error: messageText,
      });
      return true;
    }
    return false;
  }

  function clearPendingActionIntent(intentOrKey) {
    const key = typeof intentOrKey === "string" ? intentOrKey : actionIntentKey(intentOrKey);
    if (!key) return;
    const timeoutId = pendingActionIntentTimeoutsRef.current.get(key);
    if (timeoutId) {
      window.clearTimeout(timeoutId);
      pendingActionIntentTimeoutsRef.current.delete(key);
    }
    pendingActionIntentsRef.current.delete(key);
    actionIntentOpeningPreviewKeysRef.current.delete(key);
    clearPeerWaitForActionIntent(key);
  }

  function clearAllPendingActionIntents() {
    for (const timeoutId of pendingActionIntentTimeoutsRef.current.values()) {
      window.clearTimeout(timeoutId);
    }
    pendingActionIntentTimeoutsRef.current.clear();
    pendingActionIntentsRef.current.clear();
    actionIntentOpeningPreviewKeysRef.current.clear();
  }

  function ignoreAndClearAllPendingActionIntents(reason = "") {
    for (const key of pendingActionIntentsRef.current.keys()) {
      rememberIgnoredActionIntentKey(key, reason);
    }
    clearAllPendingActionIntents();
  }

  function pendingActionIntentEvidenceTimeoutMs(evidence = {}) {
    return Math.max(1, Number(evidence.responseTimeoutMs || PROTOCOL_RESPONSE_TIMEOUT_MS));
  }

  function pendingActionIntentEvidenceRequestedAtMs(evidence = {}) {
    return Math.max(
      1,
      Number(evidence.requestedAtMs || Date.now() - pendingActionIntentEvidenceTimeoutMs(evidence))
    );
  }

  function pendingActionIntentFirstObservedAtMs(record = {}) {
    return Math.max(
      1,
      Number(
        record.firstObservedAtMs
        || record.evidence?.requestedAtMs
        || Date.now()
      )
    );
  }

  function pendingActionIntentEvidenceDueAtMs(evidence = {}) {
    if (!evidence?.requestPayload) return Infinity;
    return (
      pendingActionIntentEvidenceRequestedAtMs(evidence)
      + pendingActionIntentEvidenceTimeoutMs(evidence)
      + MATCH_CLOCK_CLAIM_SKEW_MS
    );
  }

  function pendingActionIntentHardDueAtMs(record = {}) {
    return (
      pendingActionIntentFirstObservedAtMs(record)
      + MAX_PENDING_ACTION_INTENT_MS
      + MATCH_CLOCK_CLAIM_SKEW_MS
    );
  }

  function pendingActionIntentDueAtMs(record = {}) {
    if (!record?.intent) return Infinity;
    return Math.min(
      pendingActionIntentEvidenceDueAtMs(record.evidence || {}),
      pendingActionIntentHardDueAtMs(record)
    );
  }

  function shouldReplacePendingActionIntentEvidence(record = {}, evidence = {}) {
    if (!evidence?.requestPayload) return false;
    if (!record.evidence?.requestPayload) return true;
    const evidenceRequestedAtMs = Number(evidence.requestedAtMs || Date.now());
    const previousRequestedAtMs = Number(record.evidence.requestedAtMs || 0);
    if (evidenceRequestedAtMs < previousRequestedAtMs) return false;
    const evidenceDueAtMs = pendingActionIntentEvidenceDueAtMs(evidence);
    const previousDueAtMs = pendingActionIntentEvidenceDueAtMs(record.evidence);
    return evidenceDueAtMs >= previousDueAtMs || Date.now() >= previousDueAtMs;
  }

  async function pendingActionIntentHardTimeoutEvidence(key, record = {}) {
    const requestId = String(key || actionIntentKey(record.intent) || "");
    const requestPayload = {
      type: "pending_action_intent",
      protocolVersion: PROTOCOL_VERSION,
      requestId,
      actionIntent: cloneMultiplayerPayload(record.intent || {}),
      firstObservedAtMs: pendingActionIntentFirstObservedAtMs(record),
    };
    return {
      requestType: "pending_action_intent",
      requestId,
      requestPayload,
      requestPayloadHash: await sha256Hex(canonicalMultiplayerPayload(requestPayload)),
      responseTimeoutMs: MAX_PENDING_ACTION_INTENT_MS,
      requestedAtMs: pendingActionIntentFirstObservedAtMs(record),
    };
  }

  function schedulePendingActionIntentTimeout(key, record) {
    if (!key || !record?.intent) return;
    const existingTimeoutId = pendingActionIntentTimeoutsRef.current.get(key);
    if (existingTimeoutId) {
      window.clearTimeout(existingTimeoutId);
      pendingActionIntentTimeoutsRef.current.delete(key);
    }
    const dueAtMs = pendingActionIntentDueAtMs(record);
    if (!Number.isFinite(dueAtMs)) return;
    const delayMs = Math.max(1, Math.ceil(dueAtMs - Date.now()));
    const timeoutId = window.setTimeout(() => {
      pendingActionIntentTimeoutsRef.current.delete(key);
      void handlePendingActionIntentTimeout(key).catch((err) => {
        emitSyncFailureNotice(
          "Action intent timeout failed",
          err instanceof Error ? err.message : String(err)
        );
        setStatus(`Action intent timeout failed: ${toErrorMessage(err)}`, true);
      });
    }, delayMs);
    pendingActionIntentTimeoutsRef.current.set(key, timeoutId);
  }

  function matchingAppliedActionForIntent(intent) {
    const payload = signedActionIntentPayload(intent);
    const applied = actionHistoryEntryForSequence(payload.seq);
    if (!applied) return null;
    if (Number(applied.actorIndex) !== Number(payload.actorIndex)) return null;
    if (canonicalMultiplayerPayload(applied.command) !== canonicalMultiplayerPayload(payload.command)) {
      return null;
    }
    if (String(applied.audit?.prevStateHash || "") !== payload.prevStateHash) return null;
    return applied;
  }

  async function observedMatchClockElapsedForIntent(intent) {
    const payload = signedActionIntentPayload(intent);
    const liveState = gameRef.current && typeof gameRef.current.uiState === "function"
      ? await gameRef.current.uiState()
      : stateRef.current;
    const snapshot = updateMatchClockForState(liveState);
    if (!snapshot.enabled || Number(snapshot.activePlayerIndex) !== Number(payload.actorIndex)) {
      return null;
    }
    if (snapshot.startedAtMs == null) return 0;
    return Math.max(0, Math.floor(nowMonotonicMs() - Number(snapshot.startedAtMs)));
  }

  function pendingActionIntentHeldForProtocolWork(record = {}) {
    const requestType = String(record?.evidence?.requestType || "");
    if ([
      "action_quorum_vote_request",
      "crypto_material_request",
      "rng_reveal_request",
      "ziffle_reveal_token_request",
      "ziffle_shuffle_step_request",
    ].includes(requestType)) {
      return true;
    }
    if (requestType !== "action_intent_progress") return false;
    const phase = String(record?.evidence?.requestPayload?.phase || "");
    return [
      "payload_generation",
      "engine_work",
      "crypto_material",
      "opening_generation",
      "opening_preview",
      "payload_signing",
      "action_broadcast",
    ].includes(phase);
  }

  function actionBroadcastResponseTimeoutMs(actionPayload) {
    const openingCount = Array.isArray(actionPayload?.audit?.openings)
      ? actionPayload.audit.openings.length
      : 0;
    if (openingCount <= 0) return PROTOCOL_RESPONSE_TIMEOUT_MS;
    return Math.max(
      PROTOCOL_RESPONSE_TIMEOUT_MS,
      ziffleRevealTokenTimeoutMs(openingCount)
    );
  }

  async function rememberPendingActionIntent(intent, evidence = {}) {
    const verifiedIntent = await verifySignedActionIntent(intent);
    const key = actionIntentKey(verifiedIntent);
    const inactiveReason = protocolActionIntentInactiveReason(key);
    if (inactiveReason) {
      recordPeerSyncPerf("action_intent:ignored", {
        key,
        reason: inactiveReason,
        request_type: String(evidence?.requestType || ""),
        request_id: String(evidence?.requestId || ""),
      });
      return verifiedIntent;
    }
    const fingerprint = actionIntentFingerprint(verifiedIntent);
    const existing = pendingActionIntentsRef.current.get(key);
    if (existing && existing.fingerprint !== fingerprint) {
      throw new Error("Refusing conflicting signed action intent for this sequence");
    }
    if (matchingAppliedActionForIntent(verifiedIntent)) {
      return verifiedIntent;
    }
    const record = existing || {
      intent: cloneMultiplayerPayload(verifiedIntent),
      fingerprint,
      evidence: null,
      firstObservedAtMs: Date.now(),
      observedElapsedAtIntentMs: null,
    };
    if (!record.firstObservedAtMs) {
      record.firstObservedAtMs = Date.now();
    }
    const observedElapsed = await observedMatchClockElapsedForIntent(verifiedIntent);
    if (observedElapsed != null) {
      record.observedElapsedAtIntentMs = Math.max(
        Number(record.observedElapsedAtIntentMs || 0),
        Number(observedElapsed || 0)
      );
    }
    if (evidence?.requestPayload) {
      const evidenceRequestedAtMs = Number(evidence.requestedAtMs || Date.now());
      const nextEvidence = cloneMultiplayerPayload({
        ...evidence,
        requestedAtMs: evidenceRequestedAtMs,
      });
      if (shouldReplacePendingActionIntentEvidence(record, nextEvidence)) {
        record.evidence = nextEvidence;
      }
    }
    pendingActionIntentsRef.current.set(key, record);
    schedulePendingActionIntentTimeout(key, record);
    return verifiedIntent;
  }

  async function refreshPendingActionIntentEvidenceForAction(message, evidence = {}) {
    const audit = message?.audit || {};
    const key = actionIntentKey({
      matchId: audit.matchId || currentAuditMatchId(),
      seq: audit.seq ?? message?.seq,
      actorIndex: audit.actor ?? message?.actorIndex,
    });
    const record = pendingActionIntentsRef.current.get(key);
    if (!record || !evidence?.requestPayload) return false;
    if (!record.firstObservedAtMs) {
      record.firstObservedAtMs = Date.now();
    }
    const evidenceRequestedAtMs = Number(evidence.requestedAtMs || Date.now());
    const nextEvidence = cloneMultiplayerPayload({
      ...evidence,
      requestedAtMs: evidenceRequestedAtMs,
    });
    if (shouldReplacePendingActionIntentEvidence(record, nextEvidence)) {
      record.evidence = nextEvidence;
      pendingActionIntentsRef.current.set(key, record);
      schedulePendingActionIntentTimeout(key, record);
    }
    return true;
  }

  function extendZiffleRevealTokenWaitersForActionIntent(intent, timeoutMs) {
    const key = actionIntentKey(intent);
    if (!key) return false;
    let extended = false;
    for (const waiter of ziffleRevealWaitersRef.current.values()) {
      if (!waiter || String(waiter.actionIntentKey || "") !== key) continue;
      if (typeof waiter.extendTimeout === "function") {
        extended = waiter.extendTimeout(timeoutMs) || extended;
      }
    }
    return extended;
  }

  function actionIntentProgressOperation(phase) {
    switch (String(phase || "")) {
      case "payload_generation":
        return "Generating action payload";
      case "engine_work":
        return "Applying engine command";
      case "crypto_material":
        return "Collecting hidden-card material";
      case "opening_generation":
        return "Building reveal proofs";
      case "opening_preview":
        return "Opening revealed card";
      case "payload_signing":
        return "Signing audit payload";
      case "action_broadcast":
        return "Broadcasting verified action";
      default:
        return "Working on action sync";
    }
  }

  function actionIntentProgressExtraFromMessage(message = {}) {
    const extra = {};
    for (const key of ["operation", "detail", "cardName", "card_name", "zone", "title", "description"]) {
      if (message[key] == null) continue;
      const normalizedKey = key === "card_name" ? "cardName" : key;
      extra[normalizedKey] = String(message[key] || "");
    }
    const progressCurrent = Number(message.progressCurrent ?? message.progress_current);
    const progressTotal = Number(message.progressTotal ?? message.progress_total);
    if (Number.isFinite(progressCurrent)) extra.progressCurrent = progressCurrent;
    if (Number.isFinite(progressTotal)) extra.progressTotal = progressTotal;
    const openingPreview = normalizeActionOpeningPreview(
      message.openingPreview || message.opening_preview
    );
    if (openingPreview) {
      extra.openingPreview = openingPreview;
      extra.cardName = extra.cardName || openingPreview.card;
      extra.zone = extra.zone || openingPreview.zone;
    }
    return extra;
  }

  function previewActionIntentOpeningInInspector(actionIntentKeyValue, preview, progress = {}) {
    const normalizedPreview = normalizeActionOpeningPreview(preview);
    const key = String(actionIntentKeyValue || "");
    if (!key || !normalizedPreview) return;
    const previewKey = [
      normalizedPreview.owner,
      normalizedPreview.slot ?? "",
      normalizedPreview.objectId ?? "",
      normalizedPreview.stableId ?? "",
      normalizedPreview.position ?? "",
      normalizedPreview.zone || "",
      normalizedPreview.card || "",
    ].join(":");
    let seen = actionIntentOpeningPreviewKeysRef.current.get(key);
    if (!seen) {
      seen = new Set();
      actionIntentOpeningPreviewKeysRef.current.set(key, seen);
    }
    if (seen.has(previewKey)) return;
    seen.add(previewKey);
    previewAuditOpeningInInspector(normalizedPreview, stateRef.current, {
      previewIndex: progress.progressCurrent == null
        ? undefined
        : Math.max(0, Number(progress.progressCurrent) - 1),
      previewTotal: progress.progressTotal,
      previewZone: normalizedPreview.zone,
    });
  }

  function showActionIntentProgressWait(intent, phase, responseTimeoutMs, extra = {}) {
    if (!intent) return;
    const payload = signedActionIntentPayload(intent);
    const key = actionIntentKey(payload);
    const actorName = playerNameForIndex(multiplayerRef.current.players, payload.actorIndex);
    const operation = actionIntentProgressOperation(phase);
    const requestId = `action-progress:${key}`;
    const patch = {
      kind: "action_progress",
      requestId,
      actionIntentKey: key,
      peerIndex: Number(payload.actorIndex),
      peerName: actorName,
      title: `${actorName} is syncing an action`,
      description:
        `${actorName}'s browser is ${operation.toLowerCase()} for action ${Number(payload.seq)}. `
        + "The game will continue after that payload is verified.",
      phase: String(phase || ""),
      operation,
      responseTimeoutMs,
      ...extra,
    };
    if (updatePeerWaitForActionIntent(key, patch)) return;
    beginPeerWait(patch);
  }

  async function handleActionIntentProgressMessage(message) {
    if (!message?.actionIntent) return;
    const messageIntentKey = actionIntentKeyFromProtocolPayload(message);
    const inactiveReason = protocolActionIntentInactiveReason(messageIntentKey);
    if (inactiveReason) {
      recordPeerSyncPerf("action_intent_progress:ignored", {
        request_id: String(message.requestId || ""),
        phase: String(message.phase || ""),
        reason: inactiveReason,
        action_intent_key: messageIntentKey,
      });
      return;
    }
    const requestPayload = cloneMultiplayerPayload(message);
    const phase = String(message.phase || "");
    const actionPayload = message.action || message.applyAction || message.apply_action || null;
    const phaseDefaultTimeoutMs = phase === "action_broadcast"
      ? actionBroadcastResponseTimeoutMs(actionPayload)
      : ZIFFLE_REVEAL_TOKEN_TIMEOUT_MS_PER_CARD;
    const advertisedTimeoutMs = Number(message.responseTimeoutMs ?? message.response_timeout_ms);
    const responseTimeoutMs = Number.isFinite(advertisedTimeoutMs) && advertisedTimeoutMs > 0
      ? Math.max(Math.floor(advertisedTimeoutMs), phaseDefaultTimeoutMs)
      : phaseDefaultTimeoutMs;
    const verifiedIntent = await rememberPendingActionIntent(message.actionIntent, {
      requestType: "action_intent_progress",
      requestId: String(message.requestId || ""),
      requestPayload,
      requestPayloadHash: await sha256Hex(canonicalMultiplayerPayload(requestPayload)),
      responseTimeoutMs,
      requestedAtMs: Date.now(),
    });
    if (
      message.senderIndex != null
      && Number(message.senderIndex) !== Number(verifiedIntent.actorIndex)
    ) {
      recordPeerSyncPerf("action_intent_progress:ignored", {
        request_id: String(message.requestId || ""),
        phase,
        reason: "sender_actor_mismatch",
        sender: Number(message.senderIndex),
        actor: Number(verifiedIntent.actorIndex),
      });
      return;
    }
    const extendedRevealWaiters = extendZiffleRevealTokenWaitersForActionIntent(
      message.actionIntent,
      responseTimeoutMs
    );
    const progressExtra = actionIntentProgressExtraFromMessage(message);
    showActionIntentProgressWait(message.actionIntent, phase, responseTimeoutMs, progressExtra);
    if (progressExtra.openingPreview) {
      previewActionIntentOpeningInInspector(messageIntentKey, progressExtra.openingPreview, progressExtra);
    }
    recordPeerSyncPerf("action_intent_progress:received", {
      request_id: String(message.requestId || ""),
      phase,
      sender: message.senderIndex == null ? null : Number(message.senderIndex),
      response_timeout_ms: responseTimeoutMs,
      bytes: payloadSizeBytes(message),
      extended_reveal_waiters: extendedRevealWaiters,
    });
    if (
      phase === "action_broadcast"
      && actionPayload
      && String(actionPayload.type || "") === "apply_action"
    ) {
      await applySequencedActionMessage(cloneMultiplayerPayload(actionPayload));
    }
  }

  async function handleActionIntentCancelMessage(message) {
    if (!message?.actionIntent) return;
    const verifiedIntent = await verifySignedActionIntent(message.actionIntent);
    const key = actionIntentKey(verifiedIntent);
    rememberIgnoredActionIntentKey(key, String(message.reason || "action_intent_cancel"));
    const hadPending = pendingActionIntentsRef.current.has(key);
    if (hadPending || matchingAppliedActionForIntent(verifiedIntent)) {
      clearPendingActionIntent(key);
    }
    recordPeerSyncPerf("action_intent_cancel:received", {
      request_id: String(message.requestId || ""),
      sender: message.senderIndex == null ? null : Number(message.senderIndex),
      seq: Number(verifiedIntent.seq || 0),
      actor: Number(verifiedIntent.actorIndex ?? -1),
      cleared: hadPending,
      reason: String(message.reason || ""),
    });
  }

  function broadcastActionIntentProgress(
    actionIntent,
    phase = "payload_generation",
    responseTimeoutMs = null,
    extraPayload = null
  ) {
    if (!actionIntent) return false;
    const session = multiplayerRef.current;
    const advertisedTimeoutMs = Number(responseTimeoutMs);
    const normalizedPhase = String(phase || "payload_generation");
    const localExtensionMs = Number.isFinite(advertisedTimeoutMs) && advertisedTimeoutMs > 0
      ? Math.floor(advertisedTimeoutMs)
      : normalizedPhase === "action_broadcast"
        ? actionBroadcastResponseTimeoutMs(extraPayload?.action || extraPayload?.applyAction)
        : ZIFFLE_REVEAL_TOKEN_TIMEOUT_MS_PER_CARD;
    const extendedLocalRevealWaiters = extendZiffleRevealTokenWaitersForActionIntent(
      actionIntent,
      localExtensionMs
    );
    const payload = {
      type: "action_intent_progress",
      protocolVersion: PROTOCOL_VERSION,
      requestId: makeZiffleRequestId("action-progress"),
      senderPeerId: String(session.localPeerId || ""),
      senderIndex: resolveLocalPlayerIndex(session),
      phase: normalizedPhase,
      actionIntent: cloneMultiplayerPayload(actionIntent),
      at: Date.now(),
    };
    if (extraPayload && typeof extraPayload === "object") {
      Object.assign(payload, cloneMultiplayerPayload(extraPayload));
    }
    if (Number.isFinite(advertisedTimeoutMs) && advertisedTimeoutMs > 0) {
      payload.responseTimeoutMs = Math.floor(advertisedTimeoutMs);
    }
    let sent = false;
    for (const player of session.players || []) {
      const peerId = routePeerIdForPlayer(player);
      if (!peerId || peerId === session.localPeerId) continue;
      sent = sendDirectPeerMessage(peerId, payload) || sent;
    }
    recordPeerSyncPerf("action_intent_progress:sent", {
      request_id: payload.requestId,
      phase: payload.phase,
      sent,
      extended_reveal_waiters: extendedLocalRevealWaiters,
    });
    return sent;
  }

  function broadcastActionIntentCancel(actionIntent, reason = "") {
    if (!actionIntent) return false;
    const session = multiplayerRef.current;
    const payload = {
      type: "action_intent_cancel",
      protocolVersion: PROTOCOL_VERSION,
      requestId: makeZiffleRequestId("action-cancel"),
      senderPeerId: String(session.localPeerId || ""),
      senderIndex: resolveLocalPlayerIndex(session),
      actionIntent: cloneMultiplayerPayload(actionIntent),
      reason: String(reason || ""),
      at: Date.now(),
    };
    let sent = false;
    for (const player of session.players || []) {
      const peerId = routePeerIdForPlayer(player);
      if (!peerId || peerId === session.localPeerId) continue;
      sent = sendDirectPeerMessage(peerId, payload) || sent;
    }
    recordPeerSyncPerf("action_intent_cancel:sent", {
      request_id: payload.requestId,
      sent,
      reason: payload.reason,
    });
    return sent;
  }

  function startActionIntentProgressBroadcast(
    actionIntent,
    phase = "payload_generation",
    responseTimeoutMs = null,
    extraPayload = null
  ) {
    if (!actionIntent) return null;
    let currentPhase = phase;
    let currentResponseTimeoutMs = responseTimeoutMs;
    let currentExtraPayload = extraPayload;
    const sendCurrent = () => {
      broadcastActionIntentProgress(
        actionIntent,
        currentPhase,
        currentResponseTimeoutMs,
        currentExtraPayload
      );
    };
    sendCurrent();
    const timerId = window.setInterval(() => {
      sendCurrent();
    }, Math.max(250, Math.floor(ZIFFLE_REVEAL_TOKEN_TIMEOUT_MS_PER_CARD / 2)));
    const stop = () => window.clearInterval(timerId);
    stop.update = (
      nextPhase = currentPhase,
      nextResponseTimeoutMs = currentResponseTimeoutMs,
      nextExtraPayload = currentExtraPayload
    ) => {
      currentPhase = nextPhase;
      currentResponseTimeoutMs = nextResponseTimeoutMs;
      currentExtraPayload = nextExtraPayload;
      sendCurrent();
    };
    return stop;
  }

  function pendingActionIntentRecordForSequence(seq) {
    const matchId = currentAuditMatchId();
    const targetSeq = Number(seq);
    if (!Number.isSafeInteger(targetSeq) || targetSeq <= 0) return null;
    for (const [key, record] of pendingActionIntentsRef.current.entries()) {
      const intent = record?.intent;
      if (!intent) continue;
      const payload = signedActionIntentPayload(intent);
      if (
        payload.matchId === matchId
        && Number(payload.seq) === targetSeq
      ) {
        return { key, record, payload };
      }
    }
    return null;
  }

  async function waitForPendingActionIntentBeforeLocalSubmit(seq) {
    const targetSeq = Number(seq);
    if (!Number.isSafeInteger(targetSeq) || targetSeq <= 0) return true;
    const startedAtMs = Date.now();
    while (Date.now() - startedAtMs < MAX_PENDING_ACTION_INTENT_MS + MATCH_CLOCK_CLAIM_SKEW_MS) {
      const currentSequence = Number(multiplayerRef.current.lastAppliedSequence || 0);
      if (currentSequence >= targetSeq) return false;
      const pending = pendingActionIntentRecordForSequence(targetSeq);
      if (!pending) return true;
      if (matchingAppliedActionForIntent(pending.record.intent)) {
        clearPendingActionIntent(pending.key);
        return false;
      }
      const dueAtMs = pendingActionIntentDueAtMs(pending.record);
      const actorName = playerNameForIndex(
        multiplayerRef.current.players,
        pending.payload.actorIndex
      );
      setStatus(`Waiting for ${actorName}'s action payload`);
      if (Date.now() >= dueAtMs) {
        await handlePendingActionIntentTimeout(pending.key);
        return false;
      }
      await sleep(Math.min(250, Math.max(1, Math.ceil(dueAtMs - Date.now()))));
    }
    return false;
  }

  async function handlePendingActionIntentTimeout(key) {
    const record = pendingActionIntentsRef.current.get(key);
    if (!record) return;
    const intent = record.intent || {};
    if (matchingAppliedActionForIntent(intent)) {
      clearPendingActionIntent(key);
      return;
    }
    const seq = Number(intent.seq || 0);
    const currentSequence = Number(multiplayerRef.current.lastAppliedSequence || 0);
    if (seq <= currentSequence) {
      clearPendingActionIntent(key);
      return;
    }
    if (seq !== currentSequence + 1) {
      return;
    }
    const dueAtMs = pendingActionIntentDueAtMs(record);
    if (Date.now() < dueAtMs) {
      schedulePendingActionIntentTimeout(key, record);
      return;
    }
    const hardDueAtMs = pendingActionIntentHardDueAtMs(record);
    const evidenceDueAtMs = pendingActionIntentEvidenceDueAtMs(record.evidence || {});
    const evidence = hardDueAtMs <= evidenceDueAtMs
      ? await pendingActionIntentHardTimeoutEvidence(key, record)
      : (record.evidence || {});
    const timeoutMs = pendingActionIntentEvidenceTimeoutMs(evidence);
    const requestedAtMs = pendingActionIntentEvidenceRequestedAtMs(evidence);
    const targetPlayerIndex = normalizePlayerIndex(intent.actorIndex);
    if (targetPlayerIndex == null) return;
    const target = playerForProtocolResponseTimeout(targetPlayerIndex);
    const requestPayload = cloneMultiplayerPayload(evidence.requestPayload || {});
    const requestPayloadHash = String(
      evidence.requestPayloadHash
      || await sha256Hex(canonicalMultiplayerPayload(requestPayload))
    );
    await submitProtocolResponseTimeoutClaim({
      matchId: currentAuditMatchId(),
      basisSequence: currentSequence,
      targetPlayerIndex,
      targetPeerId: String(target?.peerId || evidence.actorPeerId || ""),
      targetName: target?.name || `Player ${targetPlayerIndex + 1}`,
      requesterIndex: resolveLocalPlayerIndex(multiplayerRef.current),
      requestType: String(evidence.requestType || requestPayload.type || "action_intent"),
      requestId: String(evidence.requestId || requestPayload.requestId || ""),
      requestPayloadHash,
      requestPayload,
      responseTimeoutMs: timeoutMs,
      requestedAtMs,
    });
  }

  async function verifyActionMatchesPendingIntent(message) {
    const audit = message?.audit || {};
    const key = actionIntentKey({
      matchId: audit.matchId || currentAuditMatchId(),
      seq: audit.seq ?? message?.seq,
      actorIndex: audit.actor ?? message?.actorIndex,
    });
    const record = pendingActionIntentsRef.current.get(key);
    if (!record) return;
    const intent = record.intent || {};
    const expectedPreActionPublicCheckpointHash =
      String(intent.preActionPublicCheckpointHash || "");
    if (expectedPreActionPublicCheckpointHash) {
      await verifyCurrentPublicCheckpointHash(
        expectedPreActionPublicCheckpointHash,
        "Signed action intent public checkpoint does not match local state"
      );
    }
    const expected = signedActionIntentPayload(intent);
    if (
      String(audit.matchId || "") !== expected.matchId
      || Number(audit.seq) !== Number(expected.seq)
      || Number(audit.actor) !== Number(expected.actorIndex)
      || String(audit.prevStateHash || "") !== expected.prevStateHash
      || canonicalMultiplayerPayload(message?.command) !== canonicalMultiplayerPayload(expected.command)
      || canonicalMultiplayerPayload(audit.command) !== canonicalMultiplayerPayload(expected.command)
    ) {
      throw new Error("Sequenced action conflicts with an earlier signed action intent");
	    }
	    const observedElapsed = Number(record.observedElapsedAtIntentMs || 0);
	    const clockElapsed = Number(audit.clock?.elapsedMs || 0);
    const intentWasHeldForCryptoMaterial = pendingActionIntentHeldForProtocolWork(record);
	    if (
	      !intentWasHeldForCryptoMaterial
	      && observedElapsed > 0
	      && clockElapsed + MATCH_CLOCK_CLAIM_SKEW_MS < observedElapsed
	    ) {
      throw new Error("Sequenced action match clock is below its signed action intent observation");
    }
    clearPendingActionIntent(key);
    return {
      intentWasHeldForCryptoMaterial,
      observedElapsedAtIntentMs: observedElapsed,
    };
	  }

  async function signReconnectProofForChallenge(challenge) {
    const { keyPair } = await ensureAuditIdentity();
    const payload = reconnectProofPayload({
      matchId: challenge.matchId,
      challengeId: challenge.requestId || challenge.challengeId,
      nonce: challenge.nonce,
      playerIndex: challenge.playerIndex,
      peerId: challenge.peerId || multiplayerRef.current.localPeerId,
      hostPeerId: challenge.hostPeerId,
      transcriptHash: challenge.transcriptHash,
    });
    return {
      ...payload,
      signatureAlgorithm: "ecdsa-p256-sha256",
      signature: await signAuditPayload(keyPair, payload),
    };
  }

  async function verifyReconnectProofForChallenge(proof, challenge, auditPublicKey) {
    if (!proof || typeof proof !== "object") {
      throw new Error("Reconnect response is missing audit-key proof");
    }
    const payload = reconnectProofPayload(proof);
    const expected = reconnectProofPayload({
      matchId: challenge.matchId,
      challengeId: challenge.requestId,
      nonce: challenge.nonce,
      playerIndex: challenge.playerIndex,
      peerId: challenge.peerId,
      hostPeerId: challenge.hostPeerId,
      transcriptHash: challenge.transcriptHash,
    });
    if (canonicalMultiplayerPayload(payload) !== canonicalMultiplayerPayload(expected)) {
      throw new Error("Reconnect proof does not match the host challenge");
    }
    const publicKey = await importCachedAuditPublicKey(auditPublicKey);
    const valid = await verifyAuditPayload(publicKey, payload, proof.signature || "");
    if (!valid) {
      throw new Error("Reconnect proof signature is invalid");
    }
  }

  function verifiedAuditOpeningKey(opening) {
    return canonicalMultiplayerPayload({
      matchId: currentAuditMatchId(),
      opening,
    });
  }

  const verifyAuditOpeningsAgainstManifests = useCallback(async (openings = [], options = {}) => {
    for (const opening of openings || []) {
      if (!opening || opening.owner == null || opening.slot == null) continue;
      const verificationKey = verifiedAuditOpeningKey(opening);
      if (verifiedAuditOpeningsRef.current.has(verificationKey)) continue;
      const manifest = publicDeckManifestForOwner(opening.owner);
      if (!manifest) {
        throw new Error(`Missing deck audit manifest for player ${Number(opening.owner) + 1}`);
      }
      const valid = await verifyCardOpeningAgainstManifest({
        manifest,
        slot: opening.slot,
        card: opening.card,
        salt: opening.salt,
      });
      if (!valid) {
        throw new Error(
          `Card opening for player ${Number(opening.owner) + 1}, slot ${Number(opening.slot)} does not match its deck commitment`
        );
      }
      await verifyZiffleOpeningProofForOpening(opening, options);
      verifiedAuditOpeningsRef.current.add(verificationKey);
    }
  }, [currentAuditMatchId, publicDeckManifestForOwner]);

  const buildDeckSlotOpeningForExport = useCallback(async ({
    manifest,
    preferredSlot,
    card,
    exportedCommitment = "",
    label = "private deck opening",
  }) => {
    const normalizedCommitment = String(exportedCommitment || "");
    let preferredOpening = null;
    try {
      preferredOpening = await buildDeckSlotOpening({
        manifest,
        slot: preferredSlot,
        card,
      });
    } catch (err) {
      if (!normalizedCommitment) throw err;
    }
    if (
      preferredOpening
      && (!normalizedCommitment || preferredOpening.commitment === normalizedCommitment)
    ) {
      return {
        opening: preferredOpening,
        remappedFromSlot: null,
      };
    }

    if (normalizedCommitment) {
      const committedSecret = (manifest?.slotSecrets || []).find(
        (secret) => String(secret?.commitment || "") === normalizedCommitment
      );
      if (committedSecret) {
        const committedSlot = Number(committedSecret.slot);
        return {
          opening: await buildDeckSlotOpening({
            manifest,
            slot: committedSlot,
            card: committedSecret.card,
          }),
          remappedFromSlot:
            Number(preferredSlot) === committedSlot ? null : Number(preferredSlot),
        };
      }
    }

    for (const secret of manifest?.slotSecrets || []) {
      const candidateSlot = Number(secret?.slot);
      if (!Number.isInteger(candidateSlot) || candidateSlot === Number(preferredSlot)) {
        continue;
      }
      try {
        const candidate = await buildDeckSlotOpening({
          manifest,
          slot: candidateSlot,
          card,
        });
        if (candidate.commitment === normalizedCommitment) {
          return {
            opening: candidate,
            remappedFromSlot: Number(preferredSlot),
          };
        }
      } catch {
        // Keep searching; a different card name cannot satisfy this slot commitment.
      }
    }

    throw new Error(
      `${label} does not match slot ${Number(preferredSlot)}`
      + ` (owner ${Number(manifest?.owner) + 1}, match ${String(manifest?.matchId || "")},`
      + ` exported ${normalizedCommitment.slice(0, 12) || "none"},`
      + ` local ${preferredOpening?.commitment?.slice(0, 12) || "none"})`
    );
  }, []);

	  const currentHiddenCardMetadataForObject = useCallback(async (objectId) => {
	    const normalized = Number(objectId);
	    if (!Number.isSafeInteger(normalized) || normalized < 0) return null;
	    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.exportSyncCheckpoint !== "function") {
      return null;
    }
	    let checkpoint = null;
	    try {
	      checkpoint = await currentGame.exportSyncCheckpoint();
	    } catch {
	      return null;
	    }
	    return hiddenCardMetadataForObjectFromCheckpoint(checkpoint, normalized);
	  }, []);

		  const sanitizeObjectBoundOpening = useCallback(async (opening) => {
		    if (!opening || typeof opening !== "object") return opening;
		    const normalizedObjectId = Number(opening.objectId ?? opening.object_id);
			    const ziffleCommitment = String(
			      opening.positionCommitment
		      || opening.position_commitment
		      || opening.publicCommitment
		      || opening.public_commitment
		      || (ziffleDeckHashFromCommitment(opening.commitment) ? opening.commitment : "")
			      || ""
			    );
		    const zifflePosition =
		      zifflePositionFromCommitment(ziffleCommitment)
		      ?? (opening.position != null ? Number(opening.position) : null);
		    if (
		      Number.isSafeInteger(zifflePosition)
		      && zifflePosition >= 0
		      && ziffleDeckHashFromCommitment(ziffleCommitment)
	    ) {
	      const candidates = ziffleCeremonyCandidatesForOwner(opening.owner, {
	        commitment: ziffleCommitment,
	        context: ziffleContextFromOpening(opening),
	      });
      const linkedCeremony = candidates.find((entry) =>
        ziffleObjectOrderLinksOpening(entry, opening.slot, zifflePosition, opening)
      );
      const ceremony = linkedCeremony
        || candidates.find((entry) => ziffleCeremonyHasObjectOrder(entry))
        || candidates[0];
      const existingShuffleObjectId = openingShuffleSourceId(opening);
      const proof = opening?.ziffleReveal || opening?.ziffleProof || opening?.positionOpeningProof || {};
      const proofShuffleObjectId = openingShuffleSourceId(proof);
      const beforeOrder = normalizeShuffleOrder(ceremony?.beforeOrder ?? ceremony?.before_order);
      const proofShuffleOriginalSlot = Number(
        proof?.shuffleOriginalSlot
        ?? proof?.shuffle_original_slot
        ?? opening?.shuffleOriginalSlot
        ?? opening?.shuffle_original_slot
        ?? opening?._debugShuffleOriginalSlot
        ?? proof?.originalSlot
      );
      const orderedSourceObjectId =
        Number.isSafeInteger(proofShuffleOriginalSlot) && proofShuffleOriginalSlot >= 0
          ? Number(beforeOrder[proofShuffleOriginalSlot])
          : NaN;
      const normalizedOrderedSourceObjectId =
        Number.isSafeInteger(orderedSourceObjectId) && orderedSourceObjectId >= 0
          ? orderedSourceObjectId
          : null;
      const orderedShuffleObjectId = ziffleShuffleObjectIdForPosition(ceremony, zifflePosition);
      const objectIdMatchesOrderedPosition = Boolean(
        Number.isSafeInteger(normalizedObjectId)
        && normalizedObjectId >= 0
        && orderedShuffleObjectId != null
        && normalizedObjectId === orderedShuffleObjectId
      );
      const ceremonyHasCommittedObjectOrder = Boolean(
        ziffleDeckHashFromCommitment(ziffleCommitment)
        && ziffleCeremonyHasObjectOrder(ceremony)
      );
	      const canInferShuffleObjectId = Boolean(
	        orderedShuffleObjectId != null
	        && (
	          linkedCeremony
	          || objectIdMatchesOrderedPosition
          || ceremonyHasCommittedObjectOrder
	          || ziffleContextFromOpening(opening)
	          || candidates.length <= 1
	        )
	      );
      const shuffleObjectId =
        proofShuffleObjectId
        ?? normalizedOrderedSourceObjectId
        ?? existingShuffleObjectId
        ?? (canInferShuffleObjectId ? orderedShuffleObjectId : null);
	      return {
	        ...opening,
	        position: zifflePosition,
	        positionCommitment: ziffleCommitment,
	        ...(Number.isSafeInteger(normalizedObjectId) && normalizedObjectId >= 0
	          ? { objectId: normalizedObjectId }
	          : orderedShuffleObjectId != null
	            ? { objectId: orderedShuffleObjectId }
            : {}),
        ...(shuffleObjectId != null ? { shuffleObjectId } : {}),
        ...(ceremony?.context ? { ziffleContext: ziffleContextFromCeremony(ceremony) } : {}),
      };
	    }
	    if (!Number.isSafeInteger(normalizedObjectId) || normalizedObjectId < 0) {
	      return opening;
	    }

	    const metadata = await currentHiddenCardMetadataForObject(normalizedObjectId);
	    const openingCommitment = String(opening.commitment || "");
	    const metadataCommitment = String(metadata?.commitment || "");
	    const metadataPublicCommitment = String(metadata?.publicCommitment || "");
	    const hiddenObjectStillMatches = Boolean(
	      metadata
	      && Number(metadata.owner) === Number(opening.owner)
	      && Number(metadata.slot) === Number(opening.slot)
	      && openingCommitment
	      && (
	        metadataCommitment === openingCommitment
	        || metadataPublicCommitment === openingCommitment
	      )
	    );
		    if (hiddenObjectStillMatches) {
		      return {
		        ...opening,
		        objectId: normalizedObjectId,
		      };
		    }
		    const hiddenObjectCanInferZifflePosition = Boolean(
		      metadata
		      && Number(metadata.owner) === Number(opening.owner)
		      && openingCommitment
		      && !ziffleDeckHashFromCommitment(openingCommitment)
		      && ziffleDeckHashFromCommitment(metadataCommitment)
		      && metadata.slot != null
		    );
		    if (hiddenObjectCanInferZifflePosition) {
		      return {
		        ...opening,
		        objectId: normalizedObjectId,
		      };
		    }

		    const currentGame = gameRef.current;
	    if (currentGame && typeof currentGame.exportSyncCheckpoint === "function") {
	      try {
	        const checkpoint = await currentGame.exportSyncCheckpoint();
	        const explicitObject = checkpointObjectForId(checkpoint, normalizedObjectId);
	        if (knownCheckpointObjectMatchesOpening(explicitObject, opening)) {
	          return {
	            ...opening,
	            objectId: normalizedObjectId,
	          };
	        }
	      } catch {
	        // Fall through to the safe slot/commitment opening below.
	      }
	    }

	    const objectCacheKey = `${currentAuditMatchId()}:object:${normalizedObjectId}`;
	    localRevealedOpeningsRef.current.delete(objectCacheKey);
	    removeStoredRevealedOpening(objectCacheKey);

	    const sanitized = { ...opening };
	    delete sanitized.objectId;
	    delete sanitized.object_id;
	    delete sanitized.shuffleObjectId;
	    delete sanitized.shuffle_object_id;
	    return sanitized;
	  }, [currentAuditMatchId, currentHiddenCardMetadataForObject, ziffleCeremonyCandidatesForOwner]);

	  const resolveCommittedZiffleRevealSlot = useCallback(async ({
    owner,
    ceremony,
    shuffleOriginalSlot,
    shuffleOriginalSlotIsVerified = false,
    position,
    card = "",
    objectId = null,
    manifest: providedManifest = null,
    payload = null,
    options = {},
  } = {}) => {
    const normalizedOwner = Number(owner);
    const normalizedPosition = Number(position);
    const parsedShuffleOriginalSlot = Number(shuffleOriginalSlot);
    const normalizedShuffleOriginalSlot =
      Number.isSafeInteger(parsedShuffleOriginalSlot) && parsedShuffleOriginalSlot >= 0
        ? parsedShuffleOriginalSlot
        : null;
    const expectedCard = String(card || "");
    const manifest = providedManifest || privateDeckManifestForOwner(
      normalizedOwner,
      payload?.auditMatchId,
    );
    const beforeOrder = normalizeShuffleOrder(ceremony?.beforeOrder ?? ceremony?.before_order);
    const afterOrder = normalizeShuffleOrder(ceremony?.afterOrder ?? ceremony?.after_order);
    const shuffleObjectId =
      normalizedShuffleOriginalSlot == null ? NaN : Number(beforeOrder[normalizedShuffleOriginalSlot]);
    const positionObjectId = Number(afterOrder[normalizedPosition]);
    const objectIds = [
      objectId,
      shuffleObjectId,
      positionObjectId,
    ].filter((entry, index, list) =>
      Number.isSafeInteger(Number(entry))
      && Number(entry) >= 0
      && list.findIndex((candidate) => Number(candidate) === Number(entry)) === index
    ).map((entry) => Number(entry));
    const expectedPositionCommitment =
      ceremony?.deckHash && Number.isSafeInteger(normalizedPosition) && normalizedPosition >= 0
        ? ziffleRuntimeCommitment(ceremony.deckHash, normalizedPosition)
        : "";
    const slotSecret = (slot) => (manifest?.slotSecrets || []).find(
      (secret) => Number(secret?.slot) === Number(slot)
    ) || null;
    const cardMatches = (secret, candidateCard = expectedCard) =>
      Boolean(secret)
      && (!candidateCard || String(secret.card || "") === String(candidateCard || ""));
    const fromSlot = (slot, source, details = {}) => {
      const normalizedSlot = Number(slot);
      if (!Number.isSafeInteger(normalizedSlot) || normalizedSlot < 0) return null;
      const secret = slotSecret(normalizedSlot);
      if (!cardMatches(secret)) return null;
      const resolvedObjectId = details.objectId == null ? null : Number(details.objectId);
      const resolvedShuffleObjectId = Number(details.shuffleObjectId ?? shuffleObjectId);
      return {
        owner: normalizedOwner,
        slot: normalizedSlot,
        card: String(secret.card || expectedCard || ""),
        commitment: String(secret.commitment || ""),
        objectId: resolvedObjectId != null && Number.isSafeInteger(resolvedObjectId) && resolvedObjectId >= 0
          ? resolvedObjectId
          : null,
        shuffleObjectId:
          Number.isSafeInteger(resolvedShuffleObjectId) && resolvedShuffleObjectId >= 0
            ? resolvedShuffleObjectId
            : null,
        position: Number.isSafeInteger(normalizedPosition) ? normalizedPosition : null,
        positionCommitment: expectedPositionCommitment,
        shuffleOriginalSlot: Number.isSafeInteger(normalizedShuffleOriginalSlot)
          ? normalizedShuffleOriginalSlot
          : null,
        positionLinked: details.positionLinked === true,
        source,
      };
    };
    const ceremonyHasObjectOrder = beforeOrder.length > 0 || afterOrder.length > 0;
    const revealLinksSlot = (candidate) =>
      Boolean(candidate)
      && (
        (
          candidate.positionLinked === true
          && expectedPositionCommitment
          && String(candidate.positionCommitment || "") === expectedPositionCommitment
          && Number(candidate.position) === normalizedPosition
        )
        || (
          !ceremonyHasObjectOrder
          && normalizedShuffleOriginalSlot != null
          && Number(candidate.slot) === normalizedShuffleOriginalSlot
        )
        || (
          normalizedShuffleOriginalSlot != null
          && ziffleObjectOrderLinksOpening(
            ceremony,
            normalizedShuffleOriginalSlot,
            normalizedPosition,
            candidate
          )
        )
      );
    const metadataExplicitlyLinksCurrentPosition = (metadata) => {
      if (!metadata) return false;
      const publicSlotRaw = metadata.publicSlot ?? metadata.public_slot ?? null;
      const publicSlot = publicSlotRaw == null ? null : Number(publicSlotRaw);
      const publicCommitment = String(
        metadata.publicCommitment ?? metadata.public_commitment ?? ""
      );
      const publicDeckHash = ziffleDeckHashFromCommitment(publicCommitment);
      if (publicSlot == null && !publicDeckHash) return false;
      if (publicSlot != null && publicSlot !== normalizedPosition) return false;
      if (publicDeckHash) {
        const expectedDeckHash = ziffleDeckHashFromCommitment(expectedPositionCommitment);
        if (expectedDeckHash && publicDeckHash !== expectedDeckHash) return false;
        if (expectedPositionCommitment && publicCommitment !== expectedPositionCommitment) return false;
        const committedPosition = zifflePositionFromCommitment(publicCommitment);
        if (committedPosition != null && committedPosition !== normalizedPosition) return false;
      }
      return true;
    };
    const verifiedRevealSlot = (() => {
      if (!shuffleOriginalSlotIsVerified || normalizedShuffleOriginalSlot == null) {
        return null;
      }
      const strict = fromSlot(normalizedShuffleOriginalSlot, "verified_ziffle_reveal_slot", {
        shuffleObjectId,
        positionLinked: true,
      });
      if (strict) return strict;
      const secret = slotSecret(normalizedShuffleOriginalSlot);
      if (!secret) return null;
      return {
        owner: normalizedOwner,
        slot: normalizedShuffleOriginalSlot,
        card: String(secret.card || ""),
        commitment: String(secret.commitment || ""),
        objectId: null,
        shuffleObjectId: Number.isSafeInteger(shuffleObjectId) && shuffleObjectId >= 0
          ? shuffleObjectId
          : null,
        position: Number.isSafeInteger(normalizedPosition) ? normalizedPosition : null,
        positionCommitment: expectedPositionCommitment,
        shuffleOriginalSlot: normalizedShuffleOriginalSlot,
        positionLinked: true,
        source: "verified_ziffle_reveal_slot_card_mismatch",
      };
    })();
    if (verifiedRevealSlot) {
      return verifiedRevealSlot;
    }
    const currentGame = gameRef.current;
	    const resolveMetadataSlot = async (
	      metadata,
	      source,
	      details = {},
      depth = 0,
      visited = new Set()
	    ) => {
	      if (!metadata || Number(metadata.owner) !== normalizedOwner) return null;
	      if (!hiddenMetadataMatchesZifflePosition(
	        metadata,
	        normalizedPosition,
	        expectedPositionCommitment
	      )) {
	        return null;
	      }
	      const metadataSlot = Number(metadata.slot);
	      if (!Number.isSafeInteger(metadataSlot) || metadataSlot < 0) return null;
      const positionLinked =
        details.positionLinked === true
        || metadataExplicitlyLinksCurrentPosition(metadata);
      const metadataCommitment = String(metadata.commitment || "");
      const metadataIsZifflePosition = Boolean(ziffleDeckHashFromCommitment(metadataCommitment));
      const metadataObjectId = Number(details.objectId ?? metadata.objectId);
      const metadataShuffleObjectId = Number(details.shuffleObjectId ?? shuffleObjectId);

      if (!metadataIsZifflePosition) {
        const resolved = fromSlot(metadataSlot, source, {
          objectId: Number.isSafeInteger(metadataObjectId) && metadataObjectId >= 0
            ? metadataObjectId
            : null,
          shuffleObjectId: Number.isSafeInteger(metadataShuffleObjectId) && metadataShuffleObjectId >= 0
            ? metadataShuffleObjectId
            : null,
          positionLinked,
        });
        if (resolved && revealLinksSlot(resolved)) {
          return {
            ...resolved,
            hiddenMetadata: metadata,
          };
        }
        return null;
      }

      if (depth >= 8 || !currentGame || typeof currentGame.ziffleRevealCard !== "function") {
        return null;
      }
      const nestedCeremony = ziffleCeremonyForOwner(normalizedOwner, {
        commitment: metadataCommitment,
        payload,
      });
      if (!nestedCeremony?.deckHash) return null;
      const nestedPosition = metadataSlot;
      const visitKey = [
        String(nestedCeremony.context || ""),
        String(nestedCeremony.deckHash || ""),
        Number(nestedPosition),
      ].join(":");
      if (visited.has(visitKey)) return null;
      visited.add(visitKey);

      const tokens = await collectZiffleRevealTokens(nestedCeremony, nestedPosition, options);
      const nestedReveal = await currentGame.ziffleRevealCard({
        deckCount: Number(nestedCeremony.deckCount),
        context: String(nestedCeremony.context || ""),
        keyContext: ziffleKeyContextForCeremony(nestedCeremony),
        keys: cloneMultiplayerPayload(nestedCeremony.keys || []),
        steps: cloneMultiplayerPayload(nestedCeremony.steps || []),
        cardPosition: nestedPosition,
        tokens,
      });
	      const nestedShuffleOriginalSlot = Number(nestedReveal.originalSlot);
	      if (!Number.isSafeInteger(nestedShuffleOriginalSlot) || nestedShuffleOriginalSlot < 0) {
	        return null;
	      }
	      const directNested = fromSlot(nestedShuffleOriginalSlot, `${source}:nested_ziffle`, {
	        objectId: Number.isSafeInteger(metadataObjectId) && metadataObjectId >= 0
	          ? metadataObjectId
	          : null,
	        shuffleObjectId: Number.isSafeInteger(metadataShuffleObjectId) && metadataShuffleObjectId >= 0
	          ? metadataShuffleObjectId
	          : null,
          positionLinked,
	      });
	      if (directNested && revealLinksSlot(directNested)) {
	        return {
	          ...directNested,
	          hiddenMetadata: metadata,
	        };
	      }
	      const nestedBeforeOrder = normalizeShuffleOrder(
	        nestedCeremony.beforeOrder ?? nestedCeremony.before_order
	      );
      const nestedAfterOrder = normalizeShuffleOrder(
        nestedCeremony.afterOrder ?? nestedCeremony.after_order
      );
      if (nestedBeforeOrder.length === 0 && nestedAfterOrder.length === 0) {
        const resolved = fromSlot(nestedShuffleOriginalSlot, source, {
          objectId: Number.isSafeInteger(metadataObjectId) && metadataObjectId >= 0
            ? metadataObjectId
            : null,
          shuffleObjectId: Number.isSafeInteger(metadataShuffleObjectId) && metadataShuffleObjectId >= 0
            ? metadataShuffleObjectId
            : null,
          positionLinked,
        });
        if (resolved && revealLinksSlot(resolved)) {
          return {
            ...resolved,
            hiddenMetadata: metadata,
          };
        }
        return null;
      }

      const nestedShuffleObjectId = Number(nestedBeforeOrder[nestedShuffleOriginalSlot]);
      const nestedPositionObjectId = Number(nestedAfterOrder[nestedPosition]);
      const nestedObjectIds = [
        metadataObjectId,
        nestedShuffleObjectId,
        nestedPositionObjectId,
      ].filter((entry, index, list) =>
        Number.isSafeInteger(Number(entry))
        && Number(entry) >= 0
        && list.findIndex((candidate) => Number(candidate) === Number(entry)) === index
      ).map((entry) => Number(entry));
      for (const nestedObjectId of nestedObjectIds) {
        const nestedMetadata = await currentHiddenCardMetadataForObject(nestedObjectId);
        const resolved = await resolveMetadataSlot(
          nestedMetadata,
          source,
          {
            objectId: nestedObjectId,
            shuffleObjectId: Number.isSafeInteger(metadataShuffleObjectId) && metadataShuffleObjectId >= 0
              ? metadataShuffleObjectId
              : nestedObjectId,
            positionLinked,
          },
          depth + 1,
          visited
        );
        if (resolved) return resolved;
      }
      return null;
    };
    const linkedOpening = localRevealedOpeningForZiffleReveal({
      owner: normalizedOwner,
      ceremony,
      shuffleOriginalSlot: normalizedShuffleOriginalSlot,
      position: normalizedPosition,
      card: expectedCard,
      objectId,
    });
    if (linkedOpening) {
      const linked = fromSlot(linkedOpening.slot, "cached_opening", {
        objectId: linkedOpening.objectId ?? objectId,
        shuffleObjectId: linkedOpening.shuffleObjectId
          ?? linkedOpening.shuffle_object_id
          ?? linkedOpening.objectId
          ?? shuffleObjectId,
      });
      if (linked && revealLinksSlot(linked)) {
        return {
          ...linked,
          card: String(linkedOpening.card || linked.card || ""),
          commitment: String(linkedOpening.commitment || linked.commitment || ""),
          positionCommitment: String(linkedOpening.positionCommitment || linked.positionCommitment || ""),
          linkedOpening,
        };
      }
    }
    for (const candidateObjectId of objectIds) {
      const metadata = await currentHiddenCardMetadataForObject(candidateObjectId);
      if (!metadata || Number(metadata.owner) !== normalizedOwner) continue;
      const metadataSlot = Number(metadata.slot);
      const metadataCommitment = String(metadata.commitment || "");
      const metadataCommitmentIsZifflePosition = Boolean(
        ziffleDeckHashFromCommitment(metadataCommitment)
      );
      const secret = metadataCommitmentIsZifflePosition ? null : slotSecret(metadataSlot);
      if (!metadataCommitmentIsZifflePosition && !cardMatches(secret)) continue;
      const metadataPublicSlot = metadata.publicSlot == null
        ? null
        : Number(metadata.publicSlot);
      const metadataPublicCommitment = String(metadata.publicCommitment || "");
      const metadataHasPublicPosition =
        metadataPublicSlot != null
        || Boolean(metadataPublicCommitment);
      const metadataCommitmentIsOriginal =
        Boolean(secret?.commitment)
        && metadataCommitment === String(secret.commitment || "");
      const metadataLinksPosition =
        metadataPublicSlot === normalizedPosition
        || metadataPublicCommitment === expectedPositionCommitment
        || (
          !metadataHasPublicPosition
          && (
            candidateObjectId === positionObjectId
            || candidateObjectId === shuffleObjectId
          )
        );
      if (
        metadataCommitmentIsOriginal
        || metadataLinksPosition
        || !ziffleDeckHashFromCommitment(metadataCommitment)
      ) {
        const metadataResolved = await resolveMetadataSlot(metadata, "hidden_metadata", {
          objectId: candidateObjectId,
          shuffleObjectId,
        });
        if (metadataResolved) return metadataResolved;
      }
    }
    if (currentGame && typeof currentGame.exportSyncCheckpoint === "function") {
      let checkpoint = null;
      try {
        checkpoint = await currentGame.exportSyncCheckpoint();
      } catch {
        checkpoint = null;
      }
      const checkpointObjectId = hiddenObjectIdForOpeningFromCheckpoint(checkpoint, {
        owner: normalizedOwner,
        position: normalizedPosition,
        positionCommitment: expectedPositionCommitment,
      });
      const checkpointMetadata = checkpointObjectId == null
        ? null
        : hiddenCardMetadataForObjectFromCheckpoint(checkpoint, checkpointObjectId);
      if (checkpointMetadata && Number(checkpointMetadata.owner) === normalizedOwner) {
        const checkpointResolved = await resolveMetadataSlot(
          checkpointMetadata,
          "checkpoint_position_metadata",
          {
            objectId: checkpointObjectId,
            shuffleObjectId,
          }
        );
        if (checkpointResolved) return checkpointResolved;
      }
    }
    const directSlot = normalizedShuffleOriginalSlot == null
      ? null
      : fromSlot(normalizedShuffleOriginalSlot, "shuffle_original_slot", {
        objectId:
          Number.isSafeInteger(positionObjectId)
          && positionObjectId >= 0
          && Number(positionObjectId) === Number(shuffleObjectId)
            ? positionObjectId
            : null,
        shuffleObjectId,
      });
    if (!ceremonyHasObjectOrder && directSlot && revealLinksSlot(directSlot)) return directSlot;
    if (
      !ceremonyHasObjectOrder
      && expectedCard
      && Number.isSafeInteger(shuffleObjectId)
      && shuffleObjectId >= 0
      && Number(positionObjectId) === shuffleObjectId
    ) {
      const firstMatchingSecret = (manifest?.slotSecrets || []).find((secret) =>
        String(secret?.card || "") === expectedCard
      );
      if (firstMatchingSecret) {
        const fallback = fromSlot(firstMatchingSecret.slot, "card_name_fallback", {
          objectId:
            Number.isSafeInteger(positionObjectId)
            && positionObjectId >= 0
            && Number(positionObjectId) === Number(shuffleObjectId)
              ? positionObjectId
              : null,
          shuffleObjectId,
        });
        if (revealLinksSlot(fallback)) {
          return fallback;
        }
      }
    }
    return null;
  }, [
	    currentHiddenCardMetadataForObject,
	    localRevealedOpeningForZiffleReveal,
	    privateDeckManifestForOwner,
      ziffleCeremonyForOwner,
	  ]);

  async function resolveCommittedSlotForZifflePosition({
    owner,
    ceremony,
    position,
    objectId = null,
    card = "",
    manifest = null,
    payload = null,
    options = {},
  } = {}) {
    const normalizedPosition = Number(position);
    const normalizedObjectId = Number(objectId);
    const shuffleOriginalSlot = ziffleShuffleOriginalSlotForPosition(
      ceremony,
      normalizedPosition,
      normalizedObjectId
    );
    const resolvedShuffleOriginalSlot =
      Number.isSafeInteger(shuffleOriginalSlot) && shuffleOriginalSlot >= 0
        ? shuffleOriginalSlot
        : null;
    const tryResolveForShuffleSlot = async (
      candidateShuffleOriginalSlot,
      shuffleOriginalSlotIsVerified = false
    ) => {
      const baseArgs = {
        owner,
        ceremony,
        shuffleOriginalSlot: candidateShuffleOriginalSlot,
        shuffleOriginalSlotIsVerified,
        position: normalizedPosition,
        objectId:
          Number.isSafeInteger(normalizedObjectId) && normalizedObjectId >= 0
            ? normalizedObjectId
            : null,
        manifest,
        payload,
        options,
      };
      let resolved = await resolveCommittedZiffleRevealSlot({
        ...baseArgs,
        card,
      });
      if (!resolved && card) {
        resolved = await resolveCommittedZiffleRevealSlot({
          ...baseArgs,
          card: "",
        });
      }
      return resolved;
    };
    const objectOrderResolvedRevealSlot = await tryResolveForShuffleSlot(
      resolvedShuffleOriginalSlot,
      false,
    );
    let cryptographicShuffleOriginalSlot = null;
    let cryptographicResolvedRevealSlot = null;
    const currentGame = gameRef.current;
    if (
      currentGame
      && typeof currentGame.ziffleRevealCard === "function"
      && Number.isSafeInteger(normalizedPosition)
      && normalizedPosition >= 0
      && ceremony?.deckHash
    ) {
      const tokens = await collectZiffleRevealTokens(ceremony, normalizedPosition, options);
      const reveal = await currentGame.ziffleRevealCard({
        deckCount: Number(ceremony.deckCount),
        context: String(ceremony.context || ""),
        keyContext: ziffleKeyContextForCeremony(ceremony),
        keys: cloneMultiplayerPayload(ceremony.keys || []),
        steps: cloneMultiplayerPayload(ceremony.steps || []),
        cardPosition: normalizedPosition,
        tokens,
      });
      const revealedSlot = Number(reveal?.originalSlot);
      if (Number.isSafeInteger(revealedSlot) && revealedSlot >= 0) {
        cryptographicShuffleOriginalSlot = revealedSlot;
        cryptographicResolvedRevealSlot = await tryResolveForShuffleSlot(revealedSlot, true);
      }
    }
    const ceremonyHasObjectOrder = ziffleCeremonyHasObjectOrder(ceremony);
    const cryptographicSlotMatchesObjectOrder =
      !ceremonyHasObjectOrder
      || cryptographicShuffleOriginalSlot == null
      || resolvedShuffleOriginalSlot == null
      || cryptographicShuffleOriginalSlot === resolvedShuffleOriginalSlot;
    const resolvedRevealSlot =
      (ceremonyHasObjectOrder && objectOrderResolvedRevealSlot)
        ? objectOrderResolvedRevealSlot
        : cryptographicSlotMatchesObjectOrder
          ? (cryptographicResolvedRevealSlot || objectOrderResolvedRevealSlot)
          : null;
    return {
      resolvedRevealSlot,
      shuffleOriginalSlot:
        resolvedRevealSlot === objectOrderResolvedRevealSlot
          ? resolvedShuffleOriginalSlot
          : cryptographicShuffleOriginalSlot ?? resolvedShuffleOriginalSlot,
    };
  }

  async function buildOpeningFromResolvedCommittedSlot({
    manifest,
    resolvedRevealSlot,
    fallbackObjectId = null,
    position,
    positionCommitment,
    ceremony = null,
    timing = null,
  } = {}) {
    const originalSlot = Number(resolvedRevealSlot?.slot);
    if (!Number.isSafeInteger(originalSlot) || originalSlot < 0) {
      throw new Error(`Missing private deck opening for ziffle position ${Number(position)}`);
    }
    const secret = (manifest?.slotSecrets || []).find(
      (candidate) => Number(candidate.slot) === originalSlot
    );
    if (!secret) {
      throw new Error(`Missing private deck opening for ziffle slot ${originalSlot}`);
    }
    const opening = await buildDeckSlotOpening({
      manifest,
      slot: originalSlot,
      card: resolvedRevealSlot?.card || secret.card,
    });
    const resolvedObjectId = resolvedRevealSlot?.objectId == null
      ? null
      : Number(resolvedRevealSlot.objectId);
    const normalizedFallbackObjectId = fallbackObjectId == null
      ? null
      : Number(fallbackObjectId);
    const openingObjectId =
      Number.isSafeInteger(resolvedObjectId) && resolvedObjectId >= 0
        ? resolvedObjectId
        : Number.isSafeInteger(normalizedFallbackObjectId) && normalizedFallbackObjectId >= 0
          ? normalizedFallbackObjectId
          : null;
    const resolvedShuffleObjectId = Number(
      resolvedRevealSlot?.shuffleObjectId
      ?? resolvedRevealSlot?.shuffle_object_id
      ?? resolvedRevealSlot?.objectId
    );
    const openingWithPosition = {
      ...opening,
      ...(openingObjectId != null ? { objectId: openingObjectId } : {}),
      ...(Number.isSafeInteger(resolvedShuffleObjectId) && resolvedShuffleObjectId >= 0
        ? { shuffleObjectId: resolvedShuffleObjectId }
        : {}),
      ...(timing ? { timing } : {}),
      position: Number(position),
      positionCommitment: String(positionCommitment || ""),
      ...(ceremony?.context ? { ziffleContext: ziffleContextFromCeremony(ceremony) } : {}),
    };
    return {
      opening,
      openingWithPosition,
      originalSlot,
      openingObjectId,
      secret,
    };
  }

	  async function addResolvedSelectObjectCommandIds(output, command, uiState = null) {
	    if (!output || command?.type !== "select_objects" || !Array.isArray(command.object_ids)) {
	      return output;
	    }
    const decision = uiState?.decision || null;
    const selectedIds = command.object_ids.map((objectId) => Number(objectId));
    const stableIds = commandObjectStableIds(command);
    const hiddenRefs = commandObjectHiddenRefs(command);
    for (const [index, selectedId] of selectedIds.entries()) {
      const candidate = selectObjectCandidateForId(decision, selectedId);
      if (selectObjectCandidateRevealPolicy(decision, candidate) !== "public") {
        continue;
      }
      const stableId = stableIds[index];
      if (stableId != null) {
        const localObjectId = await currentObjectIdForStableId(stableId);
        if (localObjectId != null) {
          output.add(Number(localObjectId));
        }
      }
      const hiddenRef = hiddenRefs[index];
      if (hiddenRef != null) {
        const localObjectId = await currentObjectIdForHiddenRef(hiddenRef);
        if (localObjectId != null) {
          output.add(Number(localObjectId));
        }
      }
	    }
	    return output;
	  }

	  async function localizeSelectObjectOpeningIds(output, command) {
	    if (!output || command?.type !== "select_objects" || !Array.isArray(command.object_ids)) {
	      return output;
	    }
	    const selectedIds = command.object_ids.map((objectId) => Number(objectId));
	    const stableIds = commandObjectStableIds(command);
	    const hiddenRefs = commandObjectHiddenRefs(command);
	    for (const [index, selectedId] of selectedIds.entries()) {
	      let localObjectId = null;
	      const hiddenRef = hiddenRefs[index];
	      if (hiddenRef != null) {
	        const cachedOpening = localRevealedOpeningForRequirement({
	          owner: hiddenRef.owner,
	          zone: hiddenRef.zone,
	          objectId: selectedId,
	          publicSlot: hiddenRef.publicSlot ?? hiddenRef.public_slot,
	          publicCommitment: hiddenRef.publicCommitment ?? hiddenRef.public_commitment,
	          position: hiddenRef.publicSlot ?? hiddenRef.public_slot,
	          positionCommitment: hiddenRef.publicCommitment ?? hiddenRef.public_commitment,
	          commitment: hiddenRef.publicCommitment ?? hiddenRef.public_commitment,
	        });
	        if (cachedOpening?.objectId != null) {
	          localObjectId = Number(cachedOpening.objectId);
	        }
	        if (localObjectId == null) {
	          localObjectId = await currentObjectIdForHiddenRef(hiddenRef);
	        }
	      }
	      const stableId = stableIds[index];
	      if (localObjectId == null && stableId != null) {
	        localObjectId = await currentObjectIdForStableId(stableId);
	      }
	      if (localObjectId == null) continue;
	      if (Number.isSafeInteger(selectedId) && selectedId > 0) {
	        output.delete(selectedId);
	      }
	      output.add(Number(localObjectId));
	    }
	    return output;
	  }

			  const buildLocalOpeningsForCommand = useCallback(async (command, cryptoRequirements = [], options = {}) => {
	    const currentGame = gameRef.current;
	    if (!currentGame || typeof currentGame.exportHiddenCardOpening !== "function") {
	      return [];
    }
    const commandObjectIds = collectCommandObjectIds(command, new Set(), options.uiState || stateRef.current);
    await addResolvedSelectObjectCommandIds(commandObjectIds, command, options.uiState || stateRef.current);
    const objectIds = new Set(commandObjectIds);
    const publicOpenRequirementByObjectId = new Map();
    for (const requirement of cryptoRequirements || []) {
      if (
        String(requirement?.type || "") === "public_open"
        && requirement.objectId != null
        && !requirementHasZifflePosition(requirement)
      ) {
        const objectId = Number(requirement.objectId);
        objectIds.add(objectId);
	        publicOpenRequirementByObjectId.set(objectId, requirement);
	      }
	    }
	    await localizeSelectObjectOpeningIds(objectIds, command);
	    await localizeSelectObjectOpeningIds(commandObjectIds, command);
	    const openings = [];
    const seen = new Set();
    const localSeat = resolveLocalCryptoPlayerIndex();
    for (const objectId of objectIds) {
      let exported = null;
      try {
        exported = await currentGame.exportHiddenCardOpening(wasmObjectIdArg(objectId));
      } catch (err) {
        void err;
        const requirement = publicOpenRequirementByObjectId.get(Number(objectId));
        if (
          requirement
          && Number(requirement.owner) === Number(localSeat)
          && requirement.slot != null
          && requirement.card
        ) {
          let opening = localRevealedOpeningForRequirement(requirement);
          const manifest = privateDeckManifestForOwner(requirement.owner);
          if (!manifest && !opening) {
            throw new Error(`Missing private deck opening for slot ${Number(requirement.slot)}`);
          }
          let remappedFromSlot = null;
          if (!opening) {
            const built = await buildDeckSlotOpeningForExport({
              manifest,
              preferredSlot: requirement.slot,
              card: requirement.card,
              exportedCommitment: requirement.commitment,
              label: "Local hidden card opening",
            });
            opening = built.opening;
            remappedFromSlot = built.remappedFromSlot;
          } else {
            opening = cloneMultiplayerPayload(opening);
          }
	          const key = `${Number(opening.owner)}:${Number(opening.slot)}`;
	          if (seen.has(key)) continue;
	          opening.objectId = Number(requirement.objectId);
	          opening.timing = options.timing || (commandObjectIds.has(Number(objectId)) ? "pre" : "post");
	          const hiddenMetadata = await currentHiddenCardMetadataForObject(
	            requirement.objectId ?? requirement.object_id ?? objectId
	          );
	          const publicPositionIdentity = zifflePublicPositionFromSources(
	            hiddenMetadata,
	            requirement
	          );
          if (opening && publicPositionIdentity?.useAsPosition === false) {
            opening = cloneMultiplayerPayload(opening);
            delete opening.position;
            delete opening.positionCommitment;
            delete opening.ziffleContext;
            delete opening.ziffleReveal;
            delete opening.ziffleProof;
            delete opening.positionOpeningProof;
          }
	          let positionCommitment = String(opening.positionCommitment || "");
	          let zifflePosition = opening.position ?? null;
	          let ziffleContext = ziffleContextFromOpening(opening);
	          if (publicPositionIdentity) {
	            opening = withPinnedPublicZifflePosition(opening, publicPositionIdentity);
	            if (publicPositionIdentity.useAsPosition !== false) {
	              positionCommitment = opening.positionCommitment;
	              zifflePosition = opening.position;
	              ziffleContext = "";
	            }
	          }
		          if (publicPositionIdentity?.useAsPosition !== true && zifflePosition == null) {
				            const objectOrderedPosition = zifflePositionForObjectId(
			              requirement.owner,
			              requirement.objectId ?? requirement.object_id ?? objectId
			            );
		            const originalSlotOrderedPosition = zifflePositionForOriginalSlot(
		              requirement.owner,
		              requirement.slot
		            );
			            const orderedPosition = objectOrderedPosition || originalSlotOrderedPosition;
            ziffleContext = ziffleContext || orderedPosition?.ziffleContext || "";
	            const publicPositionCommitment = String(
	              hiddenMetadata?.publicCommitment
	              || hiddenMetadata?.public_commitment
	              || orderedPosition?.positionCommitment
	              || ""
	            );
	            const hiddenPositionCommitment =
	              hiddenMetadata?.commitment && ziffleDeckHashFromCommitment(hiddenMetadata.commitment)
	                ? String(hiddenMetadata.commitment)
	                : "";
            if (
              publicPositionIdentity?.useAsPosition !== false
              && publicPositionCommitment
	              && ziffleDeckHashFromCommitment(publicPositionCommitment)
	              && (hiddenMetadata?.publicSlot != null || orderedPosition?.position != null)
	            ) {
	              zifflePosition = Number(hiddenMetadata?.publicSlot ?? orderedPosition.position);
	              positionCommitment = positionCommitment || publicPositionCommitment;
	            } else if (hiddenPositionCommitment && hiddenMetadata?.slot != null) {
              zifflePosition = Number(hiddenMetadata.slot);
              positionCommitment = positionCommitment || hiddenPositionCommitment;
            }
          }
          if (zifflePosition != null) {
            opening.position = Number(zifflePosition);
            const ceremony = ziffleCeremonyForOwner(opening.owner, {
              commitment: positionCommitment,
              context: ziffleContext,
            });
            if (ceremony?.deckHash && !positionCommitment) {
              positionCommitment = ziffleRuntimeCommitment(ceremony.deckHash, zifflePosition);
            }
            if (positionCommitment) {
              opening.positionCommitment = positionCommitment;
            }
            if (ceremony?.context) {
              opening.ziffleContext = ziffleContextFromCeremony(ceremony);
              ziffleContext = opening.ziffleContext;
            }
          }
          if (
            zifflePosition != null
            && opening.positionCommitment
            && ziffleDeckHashFromCommitment(opening.positionCommitment)
            && !opening.ziffleReveal
            && !opening.ziffleProof
            && !opening.positionOpeningProof
          ) {
            const ceremony = ziffleCeremonyForOwner(opening.owner, {
              commitment: opening.positionCommitment,
              context: ziffleContext,
            });
            if (ceremony?.deckHash && typeof currentGame.ziffleRevealCard === "function") {
              const tokens = await collectZiffleRevealTokens(ceremony, Number(zifflePosition), options);
              const reveal = await currentGame.ziffleRevealCard({
                deckCount: Number(ceremony.deckCount),
                context: String(ceremony.context || ""),
                keyContext: ziffleKeyContextForCeremony(ceremony),
                keys: cloneMultiplayerPayload(ceremony.keys || []),
                steps: cloneMultiplayerPayload(ceremony.steps || []),
                cardPosition: Number(zifflePosition),
                tokens,
              });
              const shuffleOriginalSlot = Number(reveal.originalSlot);
              const resolvedRevealSlot = await resolveCommittedZiffleRevealSlot({
                owner: opening.owner,
                ceremony,
                shuffleOriginalSlot,
                shuffleOriginalSlotIsVerified: true,
                position: Number(zifflePosition),
                card: opening.card || requirement.card || "",
                objectId: requirement.objectId ?? requirement.object_id ?? objectId,
                manifest,
                options,
              });
              if (resolvedRevealSlot) {
                if (Number(resolvedRevealSlot.slot) !== Number(opening.slot)) {
                  const rebuilt = await buildDeckSlotOpeningForExport({
                    manifest,
                    preferredSlot: resolvedRevealSlot.slot,
                    card: resolvedRevealSlot.card || opening.card,
                    exportedCommitment: "",
                    label: "Local hidden card opening",
                  });
                  opening = rebuilt.opening;
                }
                opening.position = Number(zifflePosition);
                opening.positionCommitment = opening.positionCommitment
                  || ziffleRuntimeCommitment(ceremony.deckHash, zifflePosition);
                opening.ziffleContext = ziffleContextFromCeremony(ceremony);
                ziffleContext = opening.ziffleContext;
                if (resolvedRevealSlot.shuffleObjectId != null || resolvedRevealSlot.objectId != null) {
                  opening.shuffleObjectId = Number(
                    resolvedRevealSlot.shuffleObjectId ?? resolvedRevealSlot.objectId
                  );
                }
                rememberZiffleOpeningPosition(opening.owner, opening.slot, Number(zifflePosition));
	                if (!ziffleCeremonyHasObjectOrder(ceremony)) {
	                  opening.ziffleReveal = buildZiffleOpeningProof({
                    opening,
                    ceremony,
                    position: Number(zifflePosition),
                    originalSlot: Number(opening.slot),
                    shuffleOriginalSlot,
                    positionCommitment: opening.positionCommitment,
                    tokens,
	                    compact: true,
	                  });
	                }
	              } else {
	                throw new Error(
	                  `Ziffle opening could not resolve committed slot `
	                  + `(owner ${Number(opening.owner) + 1}, position ${Number(zifflePosition)}, `
	                  + `shuffle slot ${shuffleOriginalSlot}, card ${String(opening.card || requirement.card || "")})`
	                );
	              }
	            }
	          }
	          if (remappedFromSlot != null) {
	            opening.reportedSlot = Number(remappedFromSlot);
	          }
	          opening = await ensureZiffleOpeningProof(opening, options);
	          opening = await sanitizeObjectBoundOpening(opening);
	          rememberLocalRevealedOpening(opening, {
	            objectId: opening.objectId,
	            position: opening.position,
	            positionCommitment: opening.positionCommitment,
	            ziffleContext,
	          });
          openings.push(opening);
          notifyOpeningBuilt(options, opening, {
            source: "command_public_open",
            index: openings.length,
            total: objectIds.size,
          });
          seen.add(key);
        }
        continue;
      }
      if (!exported || exported.owner == null || exported.slot == null || !exported.card) {
        continue;
      }
      const key = `${Number(exported.owner)}:${Number(exported.slot)}`;
      if (seen.has(key)) continue;
      if (Number(exported.owner) !== Number(localSeat)) {
        continue;
      }
      let cachedOpening = localRevealedOpeningForExport(exported);
      const manifest = privateDeckManifestForOwner(exported.owner);
      if (!manifest && !cachedOpening) {
        throw new Error(`Missing private deck opening for slot ${Number(exported.slot)}`);
      }
	      const hiddenMetadata = await currentHiddenCardMetadataForObject(
	        exported.object_id ?? exported.objectId ?? objectId
	      );
	      const exportedCommitment = String(exported.commitment || "");
	      const exportedCommitmentIsZiffle = Boolean(
	        ziffleDeckHashFromCommitment(exportedCommitment)
	      );
		      const objectOrderedPosition = zifflePositionForObjectId(
	        exported.owner,
	        exported.object_id ?? exported.objectId ?? objectId
	      );
	      const originalSlotOrderedPosition = zifflePositionForOriginalSlot(exported.owner, exported.slot);
	      const orderedPosition = objectOrderedPosition || originalSlotOrderedPosition;
	      let ziffleContext = orderedPosition?.ziffleContext || "";
	      let ziffleContextCommitment = orderedPosition?.positionCommitment || "";
		      const exportedPublicSlot = exported?.publicSlot ?? exported?.public_slot ?? null;
		      const exportedPublicCommitment = String(
		        exported?.publicCommitment || exported?.public_commitment || ""
		      );
		      const publicPositionIdentity = zifflePublicPositionFromSources(
		        hiddenMetadata,
		        {
		          publicSlot: exportedPublicSlot,
		          publicCommitment: exportedPublicCommitment,
		        }
		      );
      if (cachedOpening && publicPositionIdentity?.useAsPosition === false) {
        cachedOpening = cloneMultiplayerPayload(cachedOpening);
        delete cachedOpening.position;
        delete cachedOpening.positionCommitment;
        delete cachedOpening.ziffleContext;
        delete cachedOpening.ziffleReveal;
        delete cachedOpening.ziffleProof;
        delete cachedOpening.positionOpeningProof;
      }
		      const publicPosition = publicPositionIdentity?.useAsPosition === false
		        ? null
		        : publicPositionIdentity?.position ?? orderedPosition?.position ?? null;
	      const publicPositionCommitment =
	        publicPositionIdentity?.useAsPosition === false
	          ? ""
	          : publicPositionIdentity?.useAsPosition !== false
	          ? publicPositionIdentity?.positionCommitment
	            || (publicPosition != null ? orderedPosition?.positionCommitment || "" : "")
	          : "";
      const hiddenPositionCommitment =
        hiddenMetadata?.commitment && ziffleDeckHashFromCommitment(hiddenMetadata.commitment)
          ? String(hiddenMetadata.commitment)
          : "";
      const identityPosition = ziffleIdentityPositionFromSources(
        hiddenMetadata,
        exported
      );
      const currentZifflePosition =
        publicPosition
        ?? identityPosition?.position
        ?? (hiddenPositionCommitment ? Number(hiddenMetadata?.slot) : null);
      const currentZifflePositionCommitment =
        publicPositionCommitment || identityPosition?.positionCommitment || hiddenPositionCommitment;
      const mustUseZiffleOpening = Boolean(
        currentZifflePosition != null
        && ziffleDeckHashFromCommitment(currentZifflePositionCommitment)
      );
		      if (
		        cachedOpening
		        && currentZifflePositionCommitment
		        && !cachedOpeningMatchesZifflePosition(
		          cachedOpening,
		          currentZifflePosition,
		          currentZifflePositionCommitment
		        )
		      ) {
		        cachedOpening = null;
		      }
      if (
        cachedOpening
        && !currentZifflePositionCommitment
        && !exportedCommitmentIsZiffle
        && !openingHasZifflePosition(cachedOpening)
      ) {
        cachedOpening = cloneMultiplayerPayload(cachedOpening);
        delete cachedOpening.position;
        delete cachedOpening.positionCommitment;
        delete cachedOpening.ziffleReveal;
        delete cachedOpening.ziffleProof;
        delete cachedOpening.positionOpeningProof;
      }
	      if (
	        cachedOpening
	        && currentZifflePosition != null
        && cachedOpening.position != null
        && Number(cachedOpening.position) !== Number(currentZifflePosition)
      ) {
        cachedOpening = null;
	      }
	      let remappedFromSlot = null;
	      let opening = cachedOpening;
      if (!opening) {
        let preferredSlot = Number(exported.slot);
        let openingCard = String(exported.card || "");
        let position = null;
        let positionCommitment = "";
	      let ziffleProofCeremony = null;
	      let ziffleProofTokens = null;
		      let ziffleProofShuffleOriginalSlot = null;
		      let ziffleProofShuffleObjectId = null;
	        const exportedCommitment = String(exported.commitment || "");
		        let ziffleCommitment = currentZifflePositionCommitment || exportedCommitment;
		        let ziffleDeckHash = ziffleDeckHashFromCommitment(ziffleCommitment);
		        if (!exportedCommitment || ziffleDeckHash) {
		          if (!ziffleDeckHash) {
		            ziffleCommitment = currentZifflePositionCommitment;
	            ziffleDeckHash = ziffleDeckHashFromCommitment(ziffleCommitment);
	          }
	        } else {
	          if (currentZifflePosition != null && currentZifflePositionCommitment) {
	            position = currentZifflePosition;
	            positionCommitment = currentZifflePositionCommitment;
	            rememberZiffleOpeningPosition(exported.owner, preferredSlot, position);
	          }
          ziffleCommitment = "";
          ziffleDeckHash = "";
        }
        if (!exportedCommitment && !ziffleDeckHash) {
          const hiddenCommitments = [
	            publicPositionCommitment,
	            hiddenPositionCommitment,
	          ];
          ziffleCommitment = hiddenCommitments.find((commitment) =>
            ziffleDeckHashFromCommitment(commitment)
          ) || "";
          ziffleDeckHash = ziffleDeckHashFromCommitment(ziffleCommitment);
        }
	        if (ziffleDeckHash) {
		          position = Number(
		            currentZifflePosition
		            ?? zifflePositionFromCommitment(ziffleCommitment)
		            ?? exported.slot
		          );
	          const ceremony = ziffleCeremonyForOwner(exported.owner, {
	            commitment: ziffleCommitment,
	            context: ziffleContextForCommitment(
	              ziffleContext,
	              ziffleContextCommitment,
	              ziffleCommitment
	            ),
	          });
	          if (!ceremony) {
	            throw new Error(`Missing ziffle ceremony for opening player ${Number(exported.owner) + 1}`);
	          }
	          ziffleContext = ziffleContextFromCeremony(ceremony);
	          ziffleContextCommitment = ziffleRuntimeCommitment(ceremony.deckHash, position);
	          if (ziffleCeremonyHasObjectOrder(ceremony)) {
	            positionCommitment = ziffleCommitment || ziffleRuntimeCommitment(ceremony.deckHash, position);
	            const openingObjectId = exported.object_id ?? exported.objectId ?? objectId;
	            const {
	              resolvedRevealSlot,
	              shuffleOriginalSlot,
	            } = await resolveCommittedSlotForZifflePosition({
	              owner: exported.owner,
	              ceremony,
	              position,
	              card: openingCard,
	              objectId: openingObjectId,
	              manifest,
	              options,
	            });
	            if (!resolvedRevealSlot) {
	              throw new Error(
	                `Ziffle opening could not resolve committed slot `
	                + `(owner ${Number(exported.owner) + 1}, position ${position}, `
	                + `shuffle slot ${shuffleOriginalSlot ?? "none"}, card ${String(openingCard || "")})`
	              );
	            }
	            preferredSlot = Number(resolvedRevealSlot.slot);
	            openingCard = String(resolvedRevealSlot.card || openingCard || "");
	            ziffleProofShuffleObjectId =
	              resolvedRevealSlot.shuffleObjectId
	              ?? resolvedRevealSlot.objectId
	              ?? ziffleShuffleObjectIdForPosition(ceremony, position)
	              ?? Number(openingObjectId);
	            if (!Number.isSafeInteger(Number(ziffleProofShuffleObjectId)) || Number(ziffleProofShuffleObjectId) < 0) {
	              ziffleProofShuffleObjectId = null;
	            }
	            rememberZiffleOpeningPosition(exported.owner, preferredSlot, position);
	          } else {
	            if (typeof currentGame.ziffleRevealCard !== "function") {
	              throw new Error("Ziffle opening reveal backend is not available");
	            }
	            const tokens = await collectZiffleRevealTokens(ceremony, position, options);
	            const reveal = await currentGame.ziffleRevealCard({
	              deckCount: Number(ceremony.deckCount),
	              context: String(ceremony.context || ""),
	              keyContext: ziffleKeyContextForCeremony(ceremony),
	              keys: cloneMultiplayerPayload(ceremony.keys || []),
	              steps: cloneMultiplayerPayload(ceremony.steps || []),
	              cardPosition: position,
	              tokens,
		            });
		            ziffleProofShuffleOriginalSlot = Number(reveal.originalSlot);
		            let resolvedRevealSlot = await resolveCommittedZiffleRevealSlot({
		              owner: exported.owner,
		              ceremony,
		              shuffleOriginalSlot: ziffleProofShuffleOriginalSlot,
		              shuffleOriginalSlotIsVerified: true,
		              position,
	              card: openingCard,
	              objectId: exported.object_id ?? exported.objectId ?? objectId,
	              manifest,
	              options,
	            });
		            if (!resolvedRevealSlot && openingCard) {
		              resolvedRevealSlot = await resolveCommittedZiffleRevealSlot({
		                owner: exported.owner,
		                ceremony,
		                shuffleOriginalSlot: ziffleProofShuffleOriginalSlot,
		                shuffleOriginalSlotIsVerified: true,
		                position,
	                card: "",
	                objectId: exported.object_id ?? exported.objectId ?? objectId,
	                manifest,
	                options,
	              });
		            }
			            if (!resolvedRevealSlot) {
			              if (
			                exportedCommitment
			                && !exportedCommitmentIsZiffle
			                && !mustUseZiffleOpening
			              ) {
			                ziffleCommitment = "";
			                ziffleDeckHash = "";
			                position = null;
			                positionCommitment = "";
			                ziffleProofShuffleOriginalSlot = null;
			                ziffleProofShuffleObjectId = null;
			              } else {
			                throw new Error(
			                  `Ziffle opening could not resolve committed slot `
			                  + `(owner ${Number(exported.owner) + 1}, position ${position}, `
			                  + `shuffle slot ${ziffleProofShuffleOriginalSlot}, card ${String(exported.card || "")})`
			                );
			              }
			            }
			            if (resolvedRevealSlot) {
			              preferredSlot = Number(resolvedRevealSlot.slot);
			              openingCard = String(resolvedRevealSlot.card || openingCard || "");
			              ziffleProofShuffleObjectId =
			                resolvedRevealSlot.shuffleObjectId ?? resolvedRevealSlot.objectId ?? null;
			              positionCommitment = ziffleCommitment || ziffleRuntimeCommitment(ceremony.deckHash, position);
		              ziffleProofCeremony = ceremony;
		              ziffleProofTokens = tokens;
		              rememberZiffleOpeningPosition(exported.owner, preferredSlot, position);
			            }
	          }
	        }
	        const built = await buildDeckSlotOpeningForExport({
	          manifest,
	          preferredSlot,
	          card: openingCard,
	          exportedCommitment: ziffleDeckHash || exportedCommitmentIsZiffle
	            ? ""
	            : exportedCommitment,
	          label: "Local hidden card opening",
	        });
        opening = built.opening;
        remappedFromSlot = built.remappedFromSlot;
	        if (position != null) {
	          opening.position = Number(position);
	          opening.positionCommitment = positionCommitment;
	          if (ziffleContext) {
	            opening.ziffleContext = ziffleContext;
	          }
	          if (ziffleProofShuffleObjectId != null) {
	            opening.shuffleObjectId = Number(ziffleProofShuffleObjectId);
	          }
		          if (ziffleProofCeremony && !ziffleCeremonyHasObjectOrder(ziffleProofCeremony)) {
	            opening.ziffleReveal = buildZiffleOpeningProof({
	              opening,
	              ceremony: ziffleProofCeremony,
              position,
              originalSlot: Number(opening.slot),
              shuffleOriginalSlot: ziffleProofShuffleOriginalSlot ?? Number(opening.slot),
              positionCommitment,
              tokens: ziffleProofTokens || [],
              compact: true,
            });
          }
	        }
	      }
	      if (publicPositionIdentity) {
	        opening = withPinnedPublicZifflePosition(opening, publicPositionIdentity);
	        ziffleContext = "";
	      }
	      opening.objectId = Number(exported.object_id ?? exported.objectId ?? objectId);
      opening.timing = options.timing || (commandObjectIds.has(Number(objectId)) ? "pre" : "post");
      const shouldUseRememberedZifflePosition = Boolean(
        currentZifflePositionCommitment
        || ziffleDeckHashFromCommitment(opening.positionCommitment)
      );
      const zifflePosition = shouldUseRememberedZifflePosition
        ? ziffleOpeningPositionForSlot(opening.owner, opening.slot)
        : null;
      if (zifflePosition != null) {
        opening.position = Number(zifflePosition);
        const ceremony = ziffleCeremonyForOwner(opening.owner, {
          commitment: opening.positionCommitment,
          context: ziffleContext,
        });
        if (ceremony?.deckHash && !opening.positionCommitment) {
          opening.positionCommitment = ziffleRuntimeCommitment(ceremony.deckHash, zifflePosition);
        }
        if (ceremony?.context) {
          opening.ziffleContext = ziffleContextFromCeremony(ceremony);
          ziffleContext = opening.ziffleContext;
        }
      }
	      if (remappedFromSlot != null) {
	        opening.reportedSlot = Number(remappedFromSlot);
	      }
	      opening = await ensureZiffleOpeningProof(opening, options);
	      opening = await sanitizeObjectBoundOpening(opening);
	      rememberLocalRevealedOpening(opening, {
	        objectId: opening.objectId,
	        positionCommitment: opening.positionCommitment,
	        ziffleContext,
	      });
      openings.push(opening);
      notifyOpeningBuilt(options, opening, {
        source: "command_public_open",
        index: openings.length,
        total: objectIds.size,
      });
      seen.add(key);
    }
	    return openings;
	  }, [
      buildDeckSlotOpeningForExport,
      collectZiffleRevealTokens,
	      currentHiddenCardMetadataForObject,
	      localRevealedOpeningForRequirement,
	      localRevealedOpeningForExport,
	      privateDeckManifestForOwner,
      rememberLocalRevealedOpening,
      sanitizeObjectBoundOpening,
      rememberZiffleOpeningPosition,
      resolveCommittedSlotForZifflePosition,
      resolveCommittedZiffleRevealSlot,
      resolveLocalCryptoPlayerIndex,
      ziffleCeremonyForOwner,
      ziffleOpeningPositionForSlot,
    ]);

	  const buildLocalOpeningFromRequirement = useCallback(async (requirement, exported = null, options = {}) => {
	    const owner = Number(exported?.owner ?? requirement?.owner);
	    if (!Number.isInteger(owner)) {
	      throw new Error("Crypto opening requirement is missing an owner");
		    }
	    const manifest = privateDeckManifestForOwner(owner);
	      let cachedOpeningForRequirement = exported
        ? localRevealedOpeningForExport(exported)
        : localRevealedOpeningForRequirement(requirement);
		    if (!manifest && !cachedOpeningForRequirement) {
		      throw new Error(`Missing private deck opening material for player ${owner + 1}`);
		    }

	    let originalSlot = Number(exported?.slot ?? requirement?.slot);
	    if (!Number.isInteger(originalSlot) || originalSlot < 0) {
	      throw new Error("Crypto opening requirement is missing a committed slot");
	    }
	    let position = null;
	    let positionCommitment = "";
	      let ziffleProofCeremony = null;
	      let ziffleProofTokens = null;
	      let ziffleProofShuffleOriginalSlot = null;
	      let ziffleProofShuffleObjectId = null;
	      const hiddenMetadata = await currentHiddenCardMetadataForObject(
	        exported?.object_id ?? exported?.objectId ?? requirement?.objectId
	      );
		    const exportedCommitment = String(exported?.commitment || requirement?.commitment || "");
		    const exportedCommitmentIsZiffle = Boolean(
		      ziffleDeckHashFromCommitment(exportedCommitment)
		    );
		    const directSlotCard = String(exported?.card || requirement?.card || "");
		    const directSlotSecret = (manifest?.slotSecrets || []).find(
		      (secret) => Number(secret?.slot) === Number(originalSlot)
		    );
		    const directSlotMatchesCard = Boolean(
		      directSlotSecret
		      && (!directSlotCard || String(directSlotSecret.card || "") === directSlotCard)
		    );
	      const objectOrderedPosition = zifflePositionForObjectId(
	        owner,
	        exported?.object_id ?? exported?.objectId ?? requirement?.objectId
	      );
	      const originalSlotOrderedPosition = zifflePositionForOriginalSlot(
	        owner,
	        exported?.slot ?? requirement?.slot
	      );
	      const orderedPosition = objectOrderedPosition || originalSlotOrderedPosition;
	      let ziffleContext = orderedPosition?.ziffleContext || "";
	      let ziffleContextCommitment = orderedPosition?.positionCommitment || "";
	      const exportedPublicSlot = exported?.publicSlot ?? exported?.public_slot ?? null;
	      const requirementPublicSlot = requirement?.publicSlot ?? requirement?.public_slot ?? null;
		      const exportedPublicCommitment = String(
		        exported?.publicCommitment || exported?.public_commitment || ""
		      );
		      const requirementPublicCommitment = String(
		        requirement?.publicCommitment || requirement?.public_commitment || ""
		      );
		      const publicPositionIdentity = zifflePublicPositionFromSources(
		        hiddenMetadata,
		        {
		          publicSlot: exportedPublicSlot,
		          publicCommitment: exportedPublicCommitment,
		        },
		        {
		          publicSlot: requirementPublicSlot,
		          publicCommitment: requirementPublicCommitment,
		        }
		      );
      if (cachedOpeningForRequirement && publicPositionIdentity?.useAsPosition === false) {
        cachedOpeningForRequirement = cloneMultiplayerPayload(cachedOpeningForRequirement);
        delete cachedOpeningForRequirement.position;
        delete cachedOpeningForRequirement.positionCommitment;
        delete cachedOpeningForRequirement.ziffleContext;
        delete cachedOpeningForRequirement.ziffleReveal;
        delete cachedOpeningForRequirement.ziffleProof;
        delete cachedOpeningForRequirement.positionOpeningProof;
      }
		      const publicPosition = publicPositionIdentity?.useAsPosition === false
		        ? null
		        : publicPositionIdentity?.position ?? orderedPosition?.position ?? null;
		      const publicPositionCommitment =
		        publicPositionIdentity?.useAsPosition === false
		          ? ""
		          : publicPositionIdentity?.useAsPosition !== false
		          ? publicPositionIdentity?.positionCommitment
		            || (publicPosition != null ? orderedPosition?.positionCommitment || "" : "")
		          : "";
      const hiddenPositionCommitment =
        hiddenMetadata?.commitment && ziffleDeckHashFromCommitment(hiddenMetadata.commitment)
          ? String(hiddenMetadata.commitment)
          : "";
      const identityPosition = ziffleIdentityPositionFromSources(
        hiddenMetadata,
        exported,
        requirement
      );
      const currentZifflePosition =
        publicPosition
        ?? identityPosition?.position
        ?? (hiddenPositionCommitment ? Number(hiddenMetadata?.slot) : null);
      const currentZifflePositionCommitment =
        publicPositionCommitment || identityPosition?.positionCommitment || hiddenPositionCommitment;
      const mustUseZiffleOpening = Boolean(
        currentZifflePosition != null
        && ziffleDeckHashFromCommitment(currentZifflePositionCommitment)
      );
	      const preferDirectDeckOpening = Boolean(
	        Number.isSafeInteger(originalSlot)
	        && originalSlot >= 0
	        && (exported?.card || requirement?.card)
	        && directSlotMatchesCard
	      );
      const directOpeningHasLiveHiddenPosition = Boolean(
        preferDirectDeckOpening
        && currentZifflePosition != null
        && currentZifflePositionCommitment
        && (
          hiddenMetadata
          || orderedPosition?.position != null
          || requirementPublicCommitment
          || exportedPublicCommitment
        )
      );
		      if (
		        cachedOpeningForRequirement
		        && currentZifflePositionCommitment
		        && !cachedOpeningMatchesZifflePosition(
		          cachedOpeningForRequirement,
		          currentZifflePosition,
		          currentZifflePositionCommitment
		        )
		      ) {
		        cachedOpeningForRequirement = null;
		      }
      if (
        cachedOpeningForRequirement
        && currentZifflePosition != null
        && cachedOpeningForRequirement.position != null
        && Number(cachedOpeningForRequirement.position) !== Number(currentZifflePosition)
      ) {
        cachedOpeningForRequirement = null;
      }
      if (
        cachedOpeningForRequirement
        && !currentZifflePositionCommitment
        && !exportedCommitmentIsZiffle
        && !openingHasZifflePosition(cachedOpeningForRequirement)
      ) {
        cachedOpeningForRequirement = cloneMultiplayerPayload(cachedOpeningForRequirement);
        delete cachedOpeningForRequirement.position;
        delete cachedOpeningForRequirement.positionCommitment;
        delete cachedOpeningForRequirement.ziffleReveal;
        delete cachedOpeningForRequirement.ziffleProof;
        delete cachedOpeningForRequirement.positionOpeningProof;
	      }
					      let ziffleCommitment =
	                (!preferDirectDeckOpening || directOpeningHasLiveHiddenPosition || mustUseZiffleOpening)
	                  ? currentZifflePositionCommitment || (preferDirectDeckOpening ? "" : exportedCommitment)
	                  : "";
				    let ziffleDeckHash = ziffleDeckHashFromCommitment(ziffleCommitment);
			      if (!exportedCommitment || ziffleDeckHash) {
			        if (!ziffleDeckHash) {
			          ziffleCommitment = currentZifflePositionCommitment;
		          ziffleDeckHash = ziffleDeckHashFromCommitment(ziffleCommitment);
		        }
		      } else {
			        if (
			          (!preferDirectDeckOpening || directOpeningHasLiveHiddenPosition)
			          && currentZifflePosition != null
			          && currentZifflePositionCommitment
			        ) {
		          position = currentZifflePosition;
		          positionCommitment = currentZifflePositionCommitment;
		          rememberZiffleOpeningPosition(owner, originalSlot, position);
		        }
        ziffleCommitment = "";
        ziffleDeckHash = "";
      }
      if (!exportedCommitment && !ziffleDeckHash) {
        const hiddenCommitments = [
	          publicPositionCommitment,
	          hiddenPositionCommitment,
	        ];
        ziffleCommitment = hiddenCommitments.find((commitment) =>
          ziffleDeckHashFromCommitment(commitment)
        ) || "";
        ziffleDeckHash = ziffleDeckHashFromCommitment(ziffleCommitment);
      }
		    let ziffleResolvedCard = "";
		    if (ziffleDeckHash) {
		      const currentGame = gameRef.current;
			      position = Number(
	            currentZifflePosition
	            ?? zifflePositionFromCommitment(ziffleCommitment)
	            ?? exported?.slot
	            ?? requirement?.slot
	          );
		      const ceremony = ziffleCeremonyForOwner(owner, {
		        commitment: ziffleCommitment,
		        context: ziffleContextForCommitment(
		          ziffleContext,
		          ziffleContextCommitment,
		          ziffleCommitment
		        ),
		      });
		      if (!ceremony) {
		        throw new Error(`Missing ziffle ceremony for opening player ${owner + 1}`);
		      }
		      ziffleContext = ziffleContextFromCeremony(ceremony);
		      ziffleContextCommitment = ziffleRuntimeCommitment(ceremony.deckHash, position);
		      if (ziffleCeremonyHasObjectOrder(ceremony)) {
		        positionCommitment = ziffleCommitment || ziffleRuntimeCommitment(ceremony.deckHash, position);
		        const openingObjectId =
		          exported?.object_id ?? exported?.objectId ?? requirement?.objectId ?? requirement?.object_id;
		        const {
		          resolvedRevealSlot: resolvedZiffleRevealSlot,
		          shuffleOriginalSlot,
		        } = await resolveCommittedSlotForZifflePosition({
		          owner,
		          ceremony,
		          position,
		          card: exported?.card || requirement?.card || "",
		          objectId: openingObjectId,
		          manifest,
		          options,
		        });
		        let resolvedRevealSlot = resolvedZiffleRevealSlot;
		        if (!resolvedRevealSlot) {
		          const directSlot = Number(exported?.slot ?? requirement?.slot);
		          const directSecret = (manifest?.slotSecrets || []).find(
		            (secret) => Number(secret?.slot) === directSlot
		          );
		          const expectedCard = String(exported?.card || requirement?.card || "");
		          const linkedShuffleObjectId =
		            ziffleShuffleObjectIdForPosition(ceremony, position)
		            ?? Number(openingObjectId);
			          const candidate = directSecret
			            ? {
		              owner,
		              slot: directSlot,
		              card: String(directSecret.card || expectedCard || ""),
		              commitment: String(directSecret.commitment || ""),
		              objectId: Number.isSafeInteger(Number(openingObjectId)) && Number(openingObjectId) >= 0
		                ? Number(openingObjectId)
		                : null,
			              shuffleObjectId: Number.isSafeInteger(Number(linkedShuffleObjectId))
			                && Number(linkedShuffleObjectId) >= 0
			                  ? Number(linkedShuffleObjectId)
			                  : null,
			              position,
			              positionCommitment,
			              ziffleContext,
			              shuffleOriginalSlot,
				              source: "direct_requirement_slot",
				            }
			            : null;
		          const directRequirementPositionOpening = Boolean(
		            candidate
		            && ziffleCeremonyHasObjectOrder(ceremony)
			            && exportedCommitment
			            && !ziffleDeckHashFromCommitment(exportedCommitment)
			            && String(candidate.commitment || "") === exportedCommitment
			            && ziffleDeckHashFromCommitment(positionCommitment)
			            && (
			              shuffleOriginalSlot == null
			              || Number(candidate.slot) === Number(shuffleOriginalSlot)
			            )
			          );
			          if (
			            candidate
			            && (!expectedCard || String(candidate.card || "") === expectedCard)
			            && (
			              ziffleObjectOrderLinksOpening(ceremony, shuffleOriginalSlot, position, candidate)
			              || directRequirementPositionOpening
			            )
			          ) {
			            resolvedRevealSlot = candidate;
			          }
		        }
			        if (!resolvedRevealSlot) {
			          if (preferDirectDeckOpening && !mustUseZiffleOpening) {
			            ziffleCommitment = "";
			            ziffleDeckHash = "";
			            position = null;
			            positionCommitment = "";
			            ziffleProofShuffleOriginalSlot = null;
			            ziffleProofShuffleObjectId = null;
			          } else {
			            throw new Error(
			              `Ziffle opening could not resolve committed slot `
			              + `(owner ${owner + 1}, position ${position}, `
			              + `shuffle slot ${shuffleOriginalSlot ?? "none"}, `
			              + `card ${String(exported?.card || requirement?.card || "")})`
			            );
			          }
			        }
			        if (resolvedRevealSlot) {
			          originalSlot = Number(resolvedRevealSlot.slot);
			          ziffleResolvedCard = String(resolvedRevealSlot.card || "");
			          ziffleProofShuffleOriginalSlot = shuffleOriginalSlot;
			          ziffleProofShuffleObjectId =
			            resolvedRevealSlot.shuffleObjectId
			            ?? resolvedRevealSlot.objectId
			            ?? ziffleShuffleObjectIdForPosition(ceremony, position)
			            ?? Number(openingObjectId);
			          if (!Number.isSafeInteger(Number(ziffleProofShuffleObjectId)) || Number(ziffleProofShuffleObjectId) < 0) {
			            ziffleProofShuffleObjectId = null;
			          }
			          rememberZiffleOpeningPosition(owner, originalSlot, position);
			        }
		      } else {
		        if (!currentGame || typeof currentGame.ziffleRevealCard !== "function") {
		          throw new Error("Ziffle opening reveal backend is not available");
		        }
		        const tokens = await collectZiffleRevealTokens(ceremony, position, options);
		        const reveal = await currentGame.ziffleRevealCard({
		          deckCount: Number(ceremony.deckCount),
		          context: String(ceremony.context || ""),
		          keyContext: ziffleKeyContextForCeremony(ceremony),
		          keys: cloneMultiplayerPayload(ceremony.keys || []),
		          steps: cloneMultiplayerPayload(ceremony.steps || []),
		          cardPosition: position,
		          tokens,
		        });
			      ziffleProofShuffleOriginalSlot = Number(reveal.originalSlot);
			      let resolvedRevealSlot = await resolveCommittedZiffleRevealSlot({
			        owner,
			        ceremony,
			        shuffleOriginalSlot: ziffleProofShuffleOriginalSlot,
			        shuffleOriginalSlotIsVerified: true,
			        position,
				        card: exported?.card || requirement?.card || "",
				        objectId: exported?.object_id ?? exported?.objectId ?? requirement?.objectId,
				        manifest,
	              options,
				      });
			      if (!resolvedRevealSlot && (exported?.card || requirement?.card)) {
			        resolvedRevealSlot = await resolveCommittedZiffleRevealSlot({
			          owner,
			          ceremony,
			          shuffleOriginalSlot: ziffleProofShuffleOriginalSlot,
			          shuffleOriginalSlotIsVerified: true,
			          position,
				          card: "",
				          objectId: exported?.object_id ?? exported?.objectId ?? requirement?.objectId,
				          manifest,
	                options,
				        });
			      }
				      if (!resolvedRevealSlot) {
				        if (preferDirectDeckOpening && !mustUseZiffleOpening) {
				          ziffleCommitment = "";
				          ziffleDeckHash = "";
				          position = null;
				          positionCommitment = "";
				          ziffleProofShuffleOriginalSlot = null;
				          ziffleProofShuffleObjectId = null;
				        } else {
				          throw new Error(
				            `Ziffle opening could not resolve committed slot `
				            + `(owner ${owner + 1}, position ${position}, `
				            + `shuffle slot ${ziffleProofShuffleOriginalSlot}, `
				            + `card ${String(exported?.card || requirement?.card || "")})`
				          );
				        }
				      }
				      if (resolvedRevealSlot) {
				        originalSlot = Number(resolvedRevealSlot.slot);
				        ziffleResolvedCard = String(resolvedRevealSlot.card || "");
				        ziffleProofShuffleObjectId =
				          resolvedRevealSlot.shuffleObjectId ?? resolvedRevealSlot.objectId ?? null;
				        positionCommitment = ziffleCommitment || ziffleRuntimeCommitment(ceremony.deckHash, position);
		            ziffleContext = ziffleContextFromCeremony(ceremony);
		            ziffleProofCeremony = ceremony;
		            ziffleProofTokens = tokens;
			          rememberZiffleOpeningPosition(owner, originalSlot, position);
				      }
			      }
			    }
      if (
        position == null
        && cachedOpeningForRequirement?.position != null
	        && !preferDirectDeckOpening
	        && (
	          currentZifflePositionCommitment
	          || ziffleDeckHashFromCommitment(cachedOpeningForRequirement.positionCommitment)
	        )
      ) {
        position = Number(cachedOpeningForRequirement.position);
        positionCommitment = String(cachedOpeningForRequirement.positionCommitment || positionCommitment || "");
      }
	      if (
	        position == null
	        && !preferDirectDeckOpening
	        && (currentZifflePositionCommitment || ziffleDeckHashFromCommitment(positionCommitment))
	      ) {
	        const rememberedPosition = ziffleOpeningPositionForSlot(owner, originalSlot);
	        if (rememberedPosition != null) {
	          position = Number(rememberedPosition);
	          const ceremony = ziffleCeremonyForOwner(owner, {
	            commitment: positionCommitment || currentZifflePositionCommitment,
	            context: ziffleContextForCommitment(
	              ziffleContext,
	              ziffleContextCommitment,
	              positionCommitment || currentZifflePositionCommitment
	            ),
	          });
	          if (ceremony?.deckHash && !positionCommitment) {
	            positionCommitment = ziffleRuntimeCommitment(ceremony.deckHash, position);
	          }
	          if (ceremony?.context) {
	            ziffleContext = ziffleContextFromCeremony(ceremony);
	            ziffleContextCommitment = ziffleRuntimeCommitment(ceremony.deckHash, position);
	          }
	        }
	      }

		    const secret = (manifest?.slotSecrets || []).find(
	      (entry) => Number(entry.slot) === Number(originalSlot)
	    );
	    if (!secret && !cachedOpeningForRequirement && !exported?.card && !requirement?.card) {
	      throw new Error(`Missing private deck opening for slot ${Number(originalSlot)}`);
	    }
	    const card = String(ziffleResolvedCard || exported?.card || requirement?.card || secret?.card || cachedOpeningForRequirement?.card || "");
      let cachedOpening = cachedOpeningForRequirement;
      let remappedFromSlot = null;
	      let opening = cachedOpening;
	      if (!opening) {
	        let built = null;
	        try {
		          built = await buildDeckSlotOpeningForExport({
		            manifest,
		            preferredSlot: originalSlot,
		            card,
		            exportedCommitment: ziffleDeckHash || exportedCommitmentIsZiffle
		              ? ""
		              : exportedCommitment,
		            label: "Private deck opening",
		          });
	        } catch (err) {
	          const currentGame = gameRef.current;
	          const exportedCommitmentBlocksPositionFallback = Boolean(
	            exportedCommitment
	            && !ziffleDeckHashFromCommitment(exportedCommitment)
	            && !ziffleDeckHashFromCommitment(positionCommitment)
	            && !ziffleDeckHashFromCommitment(currentZifflePositionCommitment)
	          );
	          const candidatePositions = [
	            position,
	            zifflePositionFromCommitment(positionCommitment),
	            zifflePositionFromCommitment(currentZifflePositionCommitment),
	            zifflePositionFromCommitment(requirement?.positionCommitment),
	            zifflePositionFromCommitment(requirement?.position_commitment),
	            zifflePositionFromCommitment(requirement?.commitment),
	            zifflePositionFromCommitment(exported?.positionCommitment),
	            zifflePositionFromCommitment(exported?.position_commitment),
	            zifflePositionFromCommitment(exported?.commitment),
	            requirement?.position,
	            exported?.position,
	            requirement?.slot,
	            exported?.slot,
	          ]
	            .map((entry) => Number(entry))
	            .filter((entry, index, list) =>
	              Number.isSafeInteger(entry)
	              && entry >= 0
	              && list.indexOf(entry) === index
	            );
	          if (
	            !exportedCommitmentBlocksPositionFallback
	            && candidatePositions.length > 0
	            && typeof currentGame?.ziffleRevealCard === "function"
	          ) {
	            const candidateCommitments = [
	              positionCommitment,
	              currentZifflePositionCommitment,
	              requirement?.positionCommitment,
	              requirement?.position_commitment,
	              requirement?.commitment,
	              exported?.positionCommitment,
	              exported?.position_commitment,
	              exported?.commitment,
	            ].map((entry) => String(entry || "")).filter(Boolean);
	            for (const candidatePosition of candidatePositions) {
	              const matchingPositionCommitment = candidateCommitments.find((commitment) =>
	                zifflePositionFromCommitment(commitment) === candidatePosition
	              ) || "";
		              const ceremony = ziffleCeremonyForOwner(owner, {
		                commitment: matchingPositionCommitment,
		                context: ziffleContextForCommitment(
		                  ziffleContext,
		                  ziffleContextCommitment,
		                  matchingPositionCommitment
		                ),
		              });
		              if (!ceremony?.deckHash) continue;
		              if (ziffleCeremonyHasObjectOrder(ceremony)) continue;
		              ziffleContext = ziffleContextFromCeremony(ceremony);
		              ziffleContextCommitment = ziffleRuntimeCommitment(ceremony.deckHash, candidatePosition);
		              const expectedPositionCommitment = ziffleRuntimeCommitment(
		                ceremony.deckHash,
		                candidatePosition
	              );
	              const normalizedPositionCommitment = matchingPositionCommitment || expectedPositionCommitment;
	              if (normalizedPositionCommitment !== expectedPositionCommitment) {
	                continue;
	              }
	              try {
	                const tokens = await collectZiffleRevealTokens(ceremony, candidatePosition, options);
	                const reveal = await currentGame.ziffleRevealCard({
	                  deckCount: Number(ceremony.deckCount),
	                  context: String(ceremony.context || ""),
	                  keyContext: ziffleKeyContextForCeremony(ceremony),
	                  keys: cloneMultiplayerPayload(ceremony.keys || []),
	                  steps: cloneMultiplayerPayload(ceremony.steps || []),
	                  cardPosition: candidatePosition,
	                  tokens,
	                });
	                const revealShuffleOriginalSlot = Number(reveal.originalSlot);
	                let resolvedRevealSlot = await resolveCommittedZiffleRevealSlot({
	                  owner,
	                  ceremony,
	                  shuffleOriginalSlot: revealShuffleOriginalSlot,
	                  shuffleOriginalSlotIsVerified: true,
	                  position: candidatePosition,
		                  card,
		                  objectId: exported?.object_id ?? exported?.objectId ?? requirement?.objectId,
		                  manifest,
                  options,
		                });
	                if (!resolvedRevealSlot && card) {
	                  resolvedRevealSlot = await resolveCommittedZiffleRevealSlot({
	                    owner,
	                    ceremony,
	                    shuffleOriginalSlot: revealShuffleOriginalSlot,
	                    shuffleOriginalSlotIsVerified: true,
	                    position: candidatePosition,
		                    card: "",
		                    objectId: exported?.object_id ?? exported?.objectId ?? requirement?.objectId,
		                    manifest,
                    options,
		                  });
	                }
	                const candidateOriginalSlot = Number(resolvedRevealSlot?.slot);
	                if (!Number.isSafeInteger(candidateOriginalSlot) || candidateOriginalSlot < 0) {
	                  continue;
	                }
	                const candidateBuilt = await buildDeckSlotOpeningForExport({
	                  manifest,
	                  preferredSlot: candidateOriginalSlot,
	                  card: resolvedRevealSlot?.card || card,
	                  exportedCommitment: "",
	                  label: "Private deck opening",
	                });
	                position = candidatePosition;
		                originalSlot = candidateOriginalSlot;
		                positionCommitment = expectedPositionCommitment;
		                ziffleContext = ziffleContextFromCeremony(ceremony);
		                ziffleContextCommitment = expectedPositionCommitment;
		                ziffleProofCeremony = ceremony;
	                ziffleProofTokens = tokens;
	                ziffleProofShuffleOriginalSlot = revealShuffleOriginalSlot;
	                ziffleProofShuffleObjectId =
	                  resolvedRevealSlot?.shuffleObjectId ?? resolvedRevealSlot?.objectId ?? null;
	                rememberZiffleOpeningPosition(owner, originalSlot, position);
	                built = candidateBuilt;
	                break;
	              } catch {
	                // Another candidate may be the runtime position for this requirement.
	              }
	            }
	          }
	          if (!built) throw err;
	        }
	        opening = built.opening;
	        remappedFromSlot = built.remappedFromSlot;
	      }
      if (remappedFromSlot != null) {
        originalSlot = Number(opening.slot);
      }
		      if (position != null) {
				        opening = {
				          ...opening,
				          position,
			          ...(positionCommitment ? { positionCommitment } : {}),
		          ...(ziffleContext ? { ziffleContext } : {}),
		          ...(ziffleProofShuffleObjectId != null
		            ? { shuffleObjectId: Number(ziffleProofShuffleObjectId) }
	            : {}),
	        };
        if (ziffleProofCeremony && !ziffleCeremonyHasObjectOrder(ziffleProofCeremony)) {
          opening.ziffleReveal = buildZiffleOpeningProof({
            opening,
            ceremony: ziffleProofCeremony,
            position,
            originalSlot: Number(opening.slot),
            shuffleOriginalSlot: ziffleProofShuffleOriginalSlot ?? Number(opening.slot),
            positionCommitment,
            tokens: ziffleProofTokens || [],
            compact: true,
          });
		        }
		      }
		      if (publicPositionIdentity) {
		        opening = withPinnedPublicZifflePosition(opening, publicPositionIdentity);
		        ziffleContext = "";
		      }
		      opening = await ensureZiffleOpeningProof(opening, options);
	      let finalOpening = {
	        ...opening,
	        ...(requirement?.objectId != null
	          ? { objectId: Number(requirement.objectId) }
	          : exported?.object_id != null || exported?.objectId != null
	            ? { objectId: Number(exported.object_id ?? exported.objectId) }
	            : {}),
	        timing: "post",
	        ...(remappedFromSlot != null ? { reportedSlot: Number(remappedFromSlot) } : {}),
	      };
	      finalOpening = await sanitizeObjectBoundOpening(finalOpening);
	      finalOpening = await ensureZiffleOpeningProof(finalOpening, options);
	      finalOpening = await sanitizeObjectBoundOpening(finalOpening);
	      if (exported) {
		        rememberLocalRevealedOpening(finalOpening, {
		          objectId: finalOpening.objectId,
		          position: finalOpening.position ?? position,
		          positionCommitment: finalOpening.positionCommitment ?? positionCommitment,
		          ziffleContext,
		        });
	      }
		    return {
		      opening: finalOpening,
		      owner,
		      originalSlot,
		      position: finalOpening.position ?? position,
		      positionCommitment: finalOpening.positionCommitment ?? positionCommitment,
		    };
		  }, [
		    collectZiffleRevealTokens,
      buildDeckSlotOpeningForExport,
		      currentHiddenCardMetadataForObject,
	      localRevealedOpeningForExport,
	      localRevealedOpeningForRequirement,
		    privateDeckManifestForOwner,
	    rememberLocalRevealedOpening,
	    rememberZiffleOpeningPosition,
	    resolveCommittedZiffleRevealSlot,
	    sanitizeObjectBoundOpening,
	    ziffleCeremonyForOwner,
      ziffleOpeningPositionForSlot,
	  ]);

	  const prefetchZiffleRevealTokensForPublicOpenRequirements = useCallback(async (
	    requirements = [],
	    options = {}
		  ) => {
		    const localSeat = resolveLocalCryptoPlayerIndex();
		    const manifest = privateDeckManifestForOwner(localSeat);
		    const groups = new Map();
	    for (const requirement of requirements || []) {
	      if (String(requirement?.type || "") !== "public_open") continue;
	      if (Number(requirement.owner) !== Number(localSeat)) continue;
	      const objectId = requirement?.objectId ?? requirement?.object_id ?? null;
	      const hiddenMetadata = await currentHiddenCardMetadataForObject(objectId);
	      const objectOrderedPosition = zifflePositionForObjectId(
		        localSeat,
		        objectId
		      );
		      const originalSlotOrderedPosition = zifflePositionForOriginalSlot(localSeat, requirement?.slot);
		      const orderedPosition = objectOrderedPosition || originalSlotOrderedPosition;
		      const ziffleContext = orderedPosition?.ziffleContext || "";
	      const requirementPublicSlot = requirement?.publicSlot ?? requirement?.public_slot ?? null;
		      const requirementPublicCommitment = String(
		        requirement?.publicCommitment || requirement?.public_commitment || ""
		      );
		      const publicPositionIdentity = zifflePublicPositionFromSources(
		        hiddenMetadata,
		        {
		          publicSlot: requirementPublicSlot,
		          publicCommitment: requirementPublicCommitment,
		        }
		      );
		      const publicPosition = publicPositionIdentity?.useAsPosition === false
		        ? null
		        : publicPositionIdentity?.position ?? orderedPosition?.position ?? null;
	      const publicPositionCommitment =
	        publicPositionIdentity?.useAsPosition === false
	          ? ""
	          : publicPositionIdentity?.useAsPosition !== false
	          ? publicPositionIdentity?.positionCommitment || ""
	          : "";
		      const hiddenPositionCommitment =
		        hiddenMetadata?.commitment && ziffleDeckHashFromCommitment(hiddenMetadata.commitment)
		          ? String(hiddenMetadata.commitment)
		          : "";
		      const identityPosition = ziffleIdentityPositionFromSources(
		        hiddenMetadata,
		        {
		          slot: requirement?.slot,
		          commitment: requirement?.commitment,
		        }
		      );
			      let ziffleCommitment = String(
			        requirement?.positionCommitment
			        || requirement?.position_commitment
			        || publicPositionCommitment
			        || identityPosition?.positionCommitment
			        || requirement?.commitment
			        || ""
			      );
		      let ziffleDeckHash = ziffleDeckHashFromCommitment(ziffleCommitment);
		      if (!ziffleDeckHash && publicPosition != null && publicPositionCommitment) {
		        ziffleCommitment = publicPositionCommitment;
		        ziffleDeckHash = ziffleDeckHashFromCommitment(ziffleCommitment);
		      }
	      if (!ziffleDeckHash) {
	        ziffleCommitment = hiddenPositionCommitment || orderedPosition?.positionCommitment || "";
	        ziffleDeckHash = ziffleDeckHashFromCommitment(ziffleCommitment);
	      }
	      if (!ziffleDeckHash) continue;
		      const position = Number(
		        publicPosition
		        ?? zifflePositionFromCommitment(ziffleCommitment)
		        ?? identityPosition?.position
		        ?? (hiddenPositionCommitment ? Number(hiddenMetadata?.slot) : null)
		        ?? requirement?.slot
		      );
	      if (!Number.isSafeInteger(position) || position < 0) continue;
	      const ceremony = ziffleCeremonyForOwner(localSeat, {
	        commitment: ziffleCommitment,
	        context: ziffleContext,
	      });
	      if (!ceremony) continue;
	      const groupKey = [
	        Number(localSeat),
	        String(ceremony.context || ""),
	        String(ceremony.deckHash || ziffleDeckHash),
	      ].join(":");
	      if (!groups.has(groupKey)) {
	        groups.set(groupKey, { ceremony, positions: new Set() });
	      }
	      groups.get(groupKey).positions.add(position);
	    }
	    for (const group of groups.values()) {
	      const positions = [...group.positions];
	      if (positions.length === 0) continue;
	      await collectZiffleRevealTokensBatch(group.ceremony, positions, {
	        ...options,
	        requirements: options.requirements || requirements,
	      });
	    }
	  }, [
		    collectZiffleRevealTokensBatch,
		    currentHiddenCardMetadataForObject,
		    privateDeckManifestForOwner,
		    resolveLocalCryptoPlayerIndex,
	    ziffleCeremonyForOwner,
	    zifflePositionForObjectId,
	    zifflePositionForOriginalSlot,
	  ]);

  const batchedOwnerPublicZiffleOpeningsForRequirements = useCallback(async (
    requirements = [],
    options = {}
	  ) => {
	    const currentGame = gameRef.current;
	    const hasBatchReveal = typeof currentGame?.ziffleRevealCards === "function";
	    const hasSingleReveal = typeof currentGame?.ziffleRevealCard === "function";
	    if (!currentGame || (!hasBatchReveal && !hasSingleReveal)) {
	      return { openings: [], handledRequirements: new Set() };
	    }
	    const localSeat = resolveLocalCryptoPlayerIndex();
	    const manifest = privateDeckManifestForOwner(localSeat);
	    if (!manifest) {
	      return { openings: [], handledRequirements: new Set() };
	    }
	    const progressiveOpeningPreviews =
	      hasSingleReveal && typeof options.onOpeningBuilt === "function";

    const groups = new Map();
	    for (const requirement of requirements || []) {
	      if (String(requirement?.type || "") !== "public_open") continue;
	      if (Number(requirement.owner) !== Number(localSeat)) continue;
	      const objectId = requirement?.objectId ?? requirement?.object_id ?? null;
	      const hiddenMetadata = await currentHiddenCardMetadataForObject(objectId);
	      const objectOrderedPosition = zifflePositionForObjectId(
	        localSeat,
	        objectId
	      );
		      const originalSlotOrderedPosition = zifflePositionForOriginalSlot(localSeat, requirement?.slot);
      const orderedPosition = objectOrderedPosition || originalSlotOrderedPosition;
      const ziffleContext = orderedPosition?.ziffleContext || "";
      const requirementPublicSlot = requirement?.publicSlot ?? requirement?.public_slot ?? null;
	      const requirementPublicCommitment = String(
	        requirement?.publicCommitment || requirement?.public_commitment || ""
	      );
	      const publicPositionIdentity = zifflePublicPositionFromSources(
	        hiddenMetadata,
	        {
	          publicSlot: requirementPublicSlot,
	          publicCommitment: requirementPublicCommitment,
	        }
	      );
	      const publicPosition = publicPositionIdentity?.useAsPosition === false
	        ? null
	        : publicPositionIdentity?.position ?? orderedPosition?.position ?? null;
	      const publicPositionCommitment =
	        publicPositionIdentity?.useAsPosition === false
	          ? ""
	          : publicPositionIdentity?.useAsPosition !== false
	          ? publicPositionIdentity?.positionCommitment || ""
	          : "";
	      const hiddenPositionCommitment =
	        hiddenMetadata?.commitment && ziffleDeckHashFromCommitment(hiddenMetadata.commitment)
	          ? String(hiddenMetadata.commitment)
	          : "";
	      const identityPosition = ziffleIdentityPositionFromSources(
	        hiddenMetadata,
	        {
	          slot: requirement?.slot,
	          commitment: requirement?.commitment,
	        }
	      );
	      const exportedCommitment = String(requirement?.commitment || "");
	      let ziffleCommitment =
	        publicPositionCommitment
	        || identityPosition?.positionCommitment
	        || orderedPosition?.positionCommitment
	        || exportedCommitment;
	      let ziffleDeckHash = ziffleDeckHashFromCommitment(ziffleCommitment);
	      if (!ziffleDeckHash && publicPosition != null && publicPositionCommitment) {
	        ziffleCommitment = publicPositionCommitment;
	        ziffleDeckHash = ziffleDeckHashFromCommitment(ziffleCommitment);
	      }
	      if (!exportedCommitment || ziffleDeckHash) {
	        if (!ziffleDeckHash) {
	          ziffleCommitment = publicPositionCommitment
	            || hiddenPositionCommitment
	            || orderedPosition?.positionCommitment
	            || "";
          ziffleDeckHash = ziffleDeckHashFromCommitment(ziffleCommitment);
        }
      } else {
        ziffleCommitment = "";
        ziffleDeckHash = "";
      }
      if (!ziffleDeckHash) continue;

	      const position = Number(
	        publicPosition
	        ?? zifflePositionFromCommitment(ziffleCommitment)
	        ?? identityPosition?.position
	        ?? (hiddenPositionCommitment ? Number(hiddenMetadata?.slot) : null)
	        ?? requirement?.slot
	      );
      if (!Number.isSafeInteger(position) || position < 0) continue;
      const ceremony = ziffleCeremonyForOwner(localSeat, {
        commitment: ziffleCommitment,
        context: ziffleContext,
      });
      if (!ceremony) continue;
      const positionCommitment = ziffleCommitment || ziffleRuntimeCommitment(ceremony.deckHash, position);
      const groupKey = [
        Number(localSeat),
        String(ceremony.context || ""),
        String(ceremony.deckHash || ziffleDeckHash),
      ].join(":");
      if (!groups.has(groupKey)) {
        groups.set(groupKey, { ceremony, entries: [] });
      }
	      groups.get(groupKey).entries.push({
	        requirement,
	        position,
	        positionCommitment,
	        publicPositionIdentity,
	      });
    }

	    const openings = [];
	    const handledRequirements = new Set();
	    const expectedOpeningCount = [...groups.values()].reduce(
	      (total, group) => total + (Array.isArray(group.entries) ? group.entries.length : 0),
	      0
	    );
	    const seen = new Set();
	    const addRevealedPublicOpeningForEntry = async ({
	      ceremony,
	      entry,
	      shuffleOriginalSlot,
	      tokens = [],
	    }) => {
	      if (!Number.isSafeInteger(shuffleOriginalSlot) || shuffleOriginalSlot < 0) {
	        throw new Error(`Missing ziffle reveal for position ${Number(entry.position)}`);
	      }
	      let resolvedRevealSlot = await resolveCommittedZiffleRevealSlot({
	        owner: localSeat,
	        ceremony,
	        shuffleOriginalSlot,
	        shuffleOriginalSlotIsVerified: true,
	        position: entry.position,
	        card: entry.requirement?.card || "",
	        objectId: entry.requirement?.objectId ?? entry.requirement?.object_id,
	        manifest,
	        options,
	      });
	      if (!resolvedRevealSlot && entry.requirement?.card) {
	        resolvedRevealSlot = await resolveCommittedZiffleRevealSlot({
	          owner: localSeat,
	          ceremony,
	          shuffleOriginalSlot,
	          shuffleOriginalSlotIsVerified: true,
	          position: entry.position,
	          card: "",
	          objectId: entry.requirement?.objectId ?? entry.requirement?.object_id,
	          manifest,
	          options,
	        });
	      }
	      if (!resolvedRevealSlot) {
	        const beforeOrder = normalizeShuffleOrder(ceremony.beforeOrder ?? ceremony.before_order);
	        const afterOrder = normalizeShuffleOrder(ceremony.afterOrder ?? ceremony.after_order);
	        const interestingIds = [
	          entry.requirement?.objectId ?? entry.requirement?.object_id,
	          beforeOrder[Number(shuffleOriginalSlot)],
	          afterOrder[Number(entry.position)],
	        ].map((id) => Number(id)).filter((id, index, list) =>
	          Number.isSafeInteger(id) && id >= 0 && list.indexOf(id) === index
	        );
	        let candidateDebug = [];
	        try {
	          const checkpoint = await currentGame.exportSyncCheckpoint?.();
	          const objectsById = new Map((checkpoint?.objects || []).map((object) => [
	            Number(object.id),
	            object,
	          ]));
	          candidateDebug = interestingIds.map((id) => {
	            const object = objectsById.get(id) || {};
	            const hidden = object.hiddenCard || object.hidden_card || {};
	            return {
	              id,
	              name: object.name || object.identity?.name || null,
	              zone: object.zone || null,
	              stableId: object.stableId ?? object.stable_id ?? null,
	              hiddenOwner: hidden.owner ?? null,
	              hiddenSlot: hidden.slot ?? null,
	              hiddenCommitment: String(hidden.commitment || "").slice(0, 32),
	              publicSlot: hidden.publicSlot ?? hidden.public_slot ?? null,
	              publicCommitment: String(hidden.publicCommitment || hidden.public_commitment || "").slice(0, 32),
	            };
	          });
	        } catch {
	          candidateDebug = [];
	        }
	        throw new Error(
	          `Ziffle opening could not resolve committed slot `
	          + `(owner ${Number(localSeat) + 1}, position ${Number(entry.position)}, `
	          + `shuffle slot ${shuffleOriginalSlot}, card ${String(entry.requirement?.card || "")}, `
	          + `requirement ${JSON.stringify(entry.requirement)}, `
	          + `ids ${JSON.stringify(interestingIds)}, candidates ${JSON.stringify(candidateDebug)})`
	        );
	      }
	      const originalSlot = Number(resolvedRevealSlot.slot);
	      const secret = (manifest.slotSecrets || []).find(
	        (candidate) => Number(candidate.slot) === Number(originalSlot)
	      );
	      if (!secret) {
	        throw new Error(`Missing private deck opening for ziffle slot ${Number(originalSlot)}`);
	      }
	      const opening = await buildDeckSlotOpening({
	        manifest,
	        slot: originalSlot,
	        card: resolvedRevealSlot?.card || secret.card,
	      });
	      const requirementObjectId = Number(entry.requirement?.objectId ?? entry.requirement?.object_id);
	      const resolvedObjectId = resolvedRevealSlot?.objectId == null
	        ? null
	        : Number(resolvedRevealSlot.objectId);
	      const openingObjectId =
	        resolvedObjectId != null && Number.isSafeInteger(resolvedObjectId) && resolvedObjectId >= 0
	          ? resolvedObjectId
	          : null;
	      let openingWithPosition = {
	        ...opening,
	        ...(openingObjectId != null ? { objectId: openingObjectId } : {}),
	        ...(resolvedRevealSlot?.shuffleObjectId != null || resolvedRevealSlot?.objectId != null
	          ? { shuffleObjectId: Number(resolvedRevealSlot.shuffleObjectId ?? resolvedRevealSlot.objectId) }
	          : {}),
	        _debugResolveSource: String(resolvedRevealSlot?.source || ""),
	        _debugShuffleOriginalSlot: Number(shuffleOriginalSlot),
	        _debugResolvedSlot: Number(resolvedRevealSlot?.slot),
	        _debugResolvedObjectId: resolvedRevealSlot?.objectId == null ? null : Number(resolvedRevealSlot.objectId),
	        timing: "post",
	        position: Number(entry.position),
		        positionCommitment: entry.positionCommitment,
		        ziffleContext: ziffleContextFromCeremony(ceremony),
		      };
		      if (entry.publicPositionIdentity) {
		        openingWithPosition = withPinnedPublicZifflePosition(
		          openingWithPosition,
		          entry.publicPositionIdentity
		        );
		      }
		      if (!ziffleCeremonyHasObjectOrder(ceremony)) {
		        openingWithPosition.ziffleReveal = buildZiffleOpeningProof({
	          opening: openingWithPosition,
	          ceremony,
	          position: Number(entry.position),
	          originalSlot,
	          shuffleOriginalSlot,
	          positionCommitment: entry.positionCommitment,
	          tokens: ziffleTokensForPosition(tokens, entry.position),
	          compact: true,
	        });
	      }
	      openingWithPosition = await sanitizeObjectBoundOpening(openingWithPosition);
	      openingWithPosition = await ensureZiffleOpeningProof(openingWithPosition, {
	        ...options,
	        requirements: options.requirements || requirements,
	        skipFreshZiffleOpeningProofVerification: Boolean(openingWithPosition.ziffleReveal),
	      });
	      openingWithPosition = await sanitizeObjectBoundOpening(openingWithPosition);
	      const key = [
	        Number(openingWithPosition.owner),
	        Number(openingWithPosition.slot),
	        openingWithPosition.objectId ?? openingObjectId ?? -1,
	      ].join(":");
	      handledRequirements.add(entry.requirement);
	      if (seen.has(key)) return;
	      rememberLocalRevealedOpening(openingWithPosition, {
	        objectId: openingWithPosition.objectId ?? (
	          Number.isSafeInteger(requirementObjectId) ? requirementObjectId : null
	        ),
	        position: openingWithPosition.position,
	        positionCommitment: openingWithPosition.positionCommitment,
	        ziffleContext: openingWithPosition.ziffleContext,
	      });
	      rememberZiffleOpeningPosition(localSeat, Number(openingWithPosition.slot ?? originalSlot), entry.position);
	      openings.push(openingWithPosition);
	      notifyOpeningBuilt(options, openingWithPosition, {
	        source: "requirement_public_open",
	        requirement: entry.requirement,
	        index: openings.length,
	        total: expectedOpeningCount || requirements.length,
	      });
	      seen.add(key);
	    };
	    for (const { ceremony, entries } of groups.values()) {
	      if (ziffleCeremonyHasObjectOrder(ceremony)) {
	        const positions = [...new Set(entries.map((entry) => Number(entry.position)))].filter(
	          (position) => Number.isSafeInteger(position) && position >= 0
	        );
	        if (positions.length > 0 && !progressiveOpeningPreviews) {
	          await collectZiffleRevealTokensBatch(ceremony, positions, {
	            ...options,
	            requirements: options.requirements || requirements,
	          });
	        }
	        for (const entry of entries) {
	          const requirementObjectId = Number(entry.requirement?.objectId ?? entry.requirement?.object_id);
	          const {
	            resolvedRevealSlot: resolvedZiffleRevealSlot,
	            shuffleOriginalSlot,
	          } = await resolveCommittedSlotForZifflePosition({
	            owner: localSeat,
	            ceremony,
	            position: entry.position,
	            card: entry.requirement?.card || "",
	            objectId: requirementObjectId,
	            manifest,
	            options,
	          });
	          let resolvedRevealSlot = resolvedZiffleRevealSlot;
	          if (!resolvedRevealSlot) {
	            const directSlot = Number(entry.requirement?.slot);
	            const directSecret = (manifest?.slotSecrets || []).find(
	              (secret) => Number(secret?.slot) === directSlot
	            );
	            const expectedCard = String(entry.requirement?.card || "");
	            const linkedShuffleObjectId =
	              ziffleShuffleObjectIdForPosition(ceremony, entry.position)
	              ?? requirementObjectId;
	            const candidate = directSecret
	              ? {
	                owner: localSeat,
	                slot: directSlot,
	                card: String(directSecret.card || expectedCard || ""),
	                commitment: String(directSecret.commitment || ""),
	                objectId:
	                  Number.isSafeInteger(requirementObjectId) && requirementObjectId >= 0
	                    ? requirementObjectId
	                    : null,
	                shuffleObjectId:
	                  Number.isSafeInteger(Number(linkedShuffleObjectId))
	                  && Number(linkedShuffleObjectId) >= 0
	                    ? Number(linkedShuffleObjectId)
	                    : null,
		                position: entry.position,
		                positionCommitment: entry.positionCommitment,
		                ziffleContext: ziffleContextFromCeremony(ceremony),
		                shuffleOriginalSlot,
		                source: "direct_requirement_slot",
	              }
	              : null;
	            const expectedCommitment = String(entry.requirement?.commitment || "");
	            const directRequirementPositionOpening = Boolean(
	              candidate
		              && expectedCommitment
		              && !ziffleDeckHashFromCommitment(expectedCommitment)
		              && String(candidate.commitment || "") === expectedCommitment
		              && ziffleDeckHashFromCommitment(entry.positionCommitment)
		              && (
		                shuffleOriginalSlot == null
		                || Number(candidate.slot) === Number(shuffleOriginalSlot)
		              )
		            );
	            if (
	              candidate
	              && (!expectedCard || String(candidate.card || "") === expectedCard)
	              && (
	                ziffleObjectOrderLinksOpening(ceremony, shuffleOriginalSlot, entry.position, candidate)
	                || directRequirementPositionOpening
	              )
	            ) {
	              resolvedRevealSlot = candidate;
	            }
	          }
	          if (!resolvedRevealSlot) {
	            throw new Error(
	              `Ziffle opening could not resolve committed slot `
	              + `(owner ${Number(localSeat) + 1}, position ${Number(entry.position)}, `
	              + `shuffle slot ${shuffleOriginalSlot ?? "none"}, `
	              + `card ${String(entry.requirement?.card || "")}, `
	              + `requirement ${JSON.stringify(entry.requirement)})`
	            );
	          }
		          const {
		            openingWithPosition,
		            originalSlot,
		            openingObjectId,
		          } = await buildOpeningFromResolvedCommittedSlot({
	            manifest,
	            resolvedRevealSlot,
	            fallbackObjectId:
	              Number.isSafeInteger(requirementObjectId) && requirementObjectId >= 0
	                ? requirementObjectId
	                : null,
	            position: entry.position,
	            positionCommitment: entry.positionCommitment,
		            ceremony,
		            timing: "post",
		          });
			          let finalOpening = entry.publicPositionIdentity
			            ? withPinnedPublicZifflePosition(openingWithPosition, entry.publicPositionIdentity)
			            : openingWithPosition;
			          finalOpening = await sanitizeObjectBoundOpening(finalOpening);
		          finalOpening = await ensureZiffleOpeningProof(finalOpening, {
		            ...options,
		            requirements: options.requirements || requirements,
		          });
		          finalOpening = await sanitizeObjectBoundOpening(finalOpening);
		          const key = [
		            Number(finalOpening.owner),
		            Number(finalOpening.slot),
		            finalOpening.objectId ?? openingObjectId ?? -1,
		          ].join(":");
		          handledRequirements.add(entry.requirement);
		          if (seen.has(key)) continue;
		          rememberLocalRevealedOpening(finalOpening, {
		            objectId: finalOpening.objectId ?? (
		              Number.isSafeInteger(requirementObjectId) ? requirementObjectId : null
		            ),
			            position: finalOpening.position,
			            positionCommitment: finalOpening.positionCommitment,
			            ziffleContext: finalOpening.ziffleContext,
			          });
		          rememberZiffleOpeningPosition(localSeat, Number(finalOpening.slot ?? originalSlot), entry.position);
		          openings.push(finalOpening);
		          notifyOpeningBuilt(options, finalOpening, {
		            source: "requirement_public_open",
		            requirement: entry.requirement,
		            index: openings.length,
		            total: expectedOpeningCount || requirements.length,
		          });
		          seen.add(key);
	        }
	        continue;
	      }
	      const positions = [...new Set(entries.map((entry) => Number(entry.position)))];
	      if (positions.length === 0) continue;
	      const revealInPreviewBatches = progressiveOpeningPreviews && hasBatchReveal;
	      if (revealInPreviewBatches) {
	        for (const chunkEntries of chunkList(entries, ZIFFLE_OPENING_PREVIEW_BATCH_SIZE)) {
	          const chunkPositions = [...new Set(chunkEntries.map((entry) => Number(entry.position)))].filter(
	            (position) => Number.isSafeInteger(position) && position >= 0
	          );
	          if (chunkPositions.length === 0) continue;
	          const tokens = await collectZiffleRevealTokensBatch(ceremony, chunkPositions, {
	            ...options,
	            requirements: options.requirements || requirements,
	          });
	          const reveals = await currentGame.ziffleRevealCards({
	            deckCount: Number(ceremony.deckCount),
	            context: String(ceremony.context || ""),
	            keyContext: ziffleKeyContextForCeremony(ceremony),
	            keys: cloneMultiplayerPayload(ceremony.keys || []),
	            steps: cloneMultiplayerPayload(ceremony.steps || []),
	            cardPositions: chunkPositions,
	            tokens,
	          });
	          const revealByPosition = new Map(
	            (Array.isArray(reveals) ? reveals : []).map((reveal) => [
	              Number(reveal.cardPosition),
	              Number(reveal.originalSlot),
	            ])
	          );
	          for (const entry of chunkEntries) {
	            const shuffleOriginalSlot = revealByPosition.get(Number(entry.position));
	            await addRevealedPublicOpeningForEntry({
	              ceremony,
	              entry,
	              shuffleOriginalSlot,
	              tokens,
	            });
	          }
	        }
	        continue;
	      }
	      const revealIndividually = !hasBatchReveal && hasSingleReveal;
	      if (revealIndividually) {
	        for (const entry of entries) {
	          const position = Number(entry.position);
	          if (!Number.isSafeInteger(position) || position < 0) continue;
	          const tokens = await collectZiffleRevealTokens(ceremony, position, {
	            ...options,
	            requirements: options.requirements || requirements,
	          });
	          const reveal = await currentGame.ziffleRevealCard({
	            deckCount: Number(ceremony.deckCount),
	            context: String(ceremony.context || ""),
	            keyContext: ziffleKeyContextForCeremony(ceremony),
	            keys: cloneMultiplayerPayload(ceremony.keys || []),
	            steps: cloneMultiplayerPayload(ceremony.steps || []),
	            cardPosition: position,
	            tokens,
	          });
	          await addRevealedPublicOpeningForEntry({
	            ceremony,
	            entry,
	            shuffleOriginalSlot: Number(reveal.originalSlot),
	            tokens,
	          });
	        }
	        continue;
	      }
	      const tokens = await collectZiffleRevealTokensBatch(ceremony, positions, {
	        ...options,
	        requirements: options.requirements || requirements,
	      });
      const reveals = await currentGame.ziffleRevealCards({
        deckCount: Number(ceremony.deckCount),
        context: String(ceremony.context || ""),
        keyContext: ziffleKeyContextForCeremony(ceremony),
        keys: cloneMultiplayerPayload(ceremony.keys || []),
        steps: cloneMultiplayerPayload(ceremony.steps || []),
        cardPositions: positions,
        tokens,
      });
      const revealByPosition = new Map(
        (Array.isArray(reveals) ? reveals : []).map((reveal) => [
          Number(reveal.cardPosition),
          Number(reveal.originalSlot),
        ])
	      );
	      for (const entry of entries) {
	        const shuffleOriginalSlot = revealByPosition.get(Number(entry.position));
	        await addRevealedPublicOpeningForEntry({
	          ceremony,
	          entry,
	          shuffleOriginalSlot,
	          tokens,
	        });
	      }
	    }
    return { openings, handledRequirements };
  }, [
    collectZiffleRevealTokensBatch,
    currentHiddenCardMetadataForObject,
    privateDeckManifestForOwner,
	    rememberLocalRevealedOpening,
	    rememberZiffleOpeningPosition,
	    resolveCommittedZiffleRevealSlot,
	    resolveLocalCryptoPlayerIndex,
	    sanitizeObjectBoundOpening,
	    ziffleCeremonyForOwner,
    zifflePositionForObjectId,
    zifflePositionForOriginalSlot,
  ]);

	  const buildLocalRequirementOpeningsForRequirements = useCallback(async (requirements = [], options = {}) => {
      const {
        openings,
        handledRequirements,
      } = await batchedOwnerPublicZiffleOpeningsForRequirements(requirements, options);
	    const remainingRequirements = (requirements || []).filter(
        (requirement) => !handledRequirements.has(requirement)
      );
	    await prefetchZiffleRevealTokensForPublicOpenRequirements(remainingRequirements, options);
	    for (const requirement of requirements || []) {
        if (handledRequirements.has(requirement)) continue;
	      if (String(requirement?.type || "") !== "public_open") continue;
		      const localSeat = resolveLocalCryptoPlayerIndex();
	      if (Number(requirement.owner) !== Number(localSeat)) continue;
	      if (openings.some((opening) => openingMatchesRequirement(opening, requirement))) {
	        handledRequirements.add(requirement);
	        continue;
	      }
	      let opening = (await buildLocalOpeningFromRequirement(requirement, null, options)).opening;
	      opening = await sanitizeObjectBoundOpening(opening);
	      opening = await ensureZiffleOpeningProof(opening, options);
	      opening = await sanitizeObjectBoundOpening(opening);
		      openings.push(opening);
		      notifyOpeningBuilt(options, opening, {
		        source: "requirement_public_open",
		        requirement,
		        index: openings.length,
		        total: requirements.length,
		      });
		    }
	    return openings;
	  }, [
      batchedOwnerPublicZiffleOpeningsForRequirements,
	    buildLocalOpeningFromRequirement,
	    ensureZiffleOpeningProof,
	    prefetchZiffleRevealTokensForPublicOpenRequirements,
	    resolveLocalCryptoPlayerIndex,
	    sanitizeObjectBoundOpening,
	  ]);

	  const buildLocalDeckAuditManifest = useCallback(
    async ({ matchId, owner, deck, sideboard, commanders, persist = true }) => {
      const normalizedMatchId = String(matchId || "");
      const normalizedOwner = Number(owner);
      const normalizedDeck = sanitizeCardList(deck);
      const normalizedSideboard = sanitizeCardList(sideboard);
      const normalizedCommanders = sanitizeCardList(commanders);
      const expectedDecklistHash = await decklistHashForCards({
        matchId: normalizedMatchId,
        owner: normalizedOwner,
        deck: normalizedDeck,
        sideboard: normalizedSideboard,
        commanders: normalizedCommanders,
      });
      const existing = privateDeckManifestForOwner(normalizedOwner, normalizedMatchId);
      if (
        existing?.decklistHash === expectedDecklistHash
        && existing?.decklistCommitment
        && Number(existing.deckCount) === normalizedDeck.length
        && Number(existing.sideboardCount || 0) === normalizedSideboard.length
        && Number(existing.commanderCount || 0) === normalizedCommanders.length
        && Array.isArray(existing.slotSecrets)
        && existing.slotSecrets.length === normalizedDeck.length
      ) {
        if (persist) rememberPrivateDeckManifest(existing);
        return existing;
      }
      const manifest = await buildPrivateDeckManifest({
        matchId: normalizedMatchId,
        owner: normalizedOwner,
        deck: normalizedDeck,
        sideboard: normalizedSideboard,
        commanders: normalizedCommanders,
      });
      if (persist) rememberPrivateDeckManifest(manifest);
      return manifest;
    },
    [privateDeckManifestForOwner, rememberPrivateDeckManifest]
  );

  const currentPublicAuditCheckpointHash = useCallback(async () => {
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.exportPublicAuditCheckpoint !== "function") {
      throw new Error("Game engine cannot export a public audit checkpoint");
    }
    return publicCheckpointHash(await currentGame.exportPublicAuditCheckpoint());
  }, []);

  function currentKnownPublicAuditCheckpointHash() {
    const lastSequence = Number(multiplayerRef.current.lastAppliedSequence || 0);
    if (lastSequence > 0) {
      return String(actionHistoryEntryForSequence(lastSequence)?.audit?.publicCheckpointHash || "");
    }
    return String(initialPublicCheckpointHashRef.current || "");
  }

  const buildSequencedActionAudit = useCallback(async ({
    seq,
    actorIndex,
    command,
    clock = null,
    openings = [],
    rngReveals = [],
    shuffleProofs = [],
    privateViewProofs = [],
    publicCheckpointHash: providedPublicCheckpointHash = null,
  }) => {
    const { keyPair } = await ensureAuditIdentity();
    const signer = resolveLocalPlayerIndex(multiplayerRef.current);
    const resolvedPublicCheckpointHash = providedPublicCheckpointHash
      || await currentPublicAuditCheckpointHash();
    if (!resolvedPublicCheckpointHash) {
      throw new Error("Game engine cannot export a public audit checkpoint");
    }
    return buildSignedActionEnvelope({
      keyPair,
      matchId: currentAuditMatchId(),
      seq: Number(seq),
      actor: Number(actorIndex),
      signer: Number(signer ?? actorIndex),
      prevStateHash: auditStateHashRef.current || INITIAL_AUDIT_STATE_HASH,
      command: cloneMultiplayerPayload(command),
      clock: cloneMultiplayerPayload(clock),
      openings: cloneMultiplayerPayload(openings),
      rngReveals: cloneMultiplayerPayload(rngReveals),
      shuffleProofs: cloneMultiplayerPayload(shuffleProofs),
      privateViewProofs: cloneMultiplayerPayload(privateViewProofs),
      publicCheckpointHash: resolvedPublicCheckpointHash,
    });
  }, [currentAuditMatchId, currentPublicAuditCheckpointHash, ensureAuditIdentity]);

  const verifyCurrentPublicCheckpointHash = useCallback(async (expectedHash, reason = "Public checkpoint hash mismatch") => {
    const expected = String(expectedHash || "");
    if (!expected) {
      throw new Error("Sequenced audit is missing public checkpoint hash");
    }
    const actual = await currentPublicAuditCheckpointHash();
    if (actual !== expected) {
      throw new Error(reason);
    }
    return actual;
  }, [currentPublicAuditCheckpointHash]);

  const verifySequencedActionAudit = useCallback(
    async ({ audit, seq, actorIndex, command, expectedPrevStateHash = null }) => {
      if (!audit || typeof audit !== "object") {
        throw new Error("Sequenced action is missing its audit signature");
      }
      if (audit.matchId !== currentAuditMatchId()) {
        throw new Error("Sequenced audit action belongs to a different match");
      }
      if (Number(audit.seq) !== Number(seq) || Number(audit.actor) !== Number(actorIndex)) {
        throw new Error("Sequenced audit action does not match broadcast action");
      }
      const expectedPrev = expectedPrevStateHash ?? auditStateHashRef.current;
      if (audit.prevStateHash !== expectedPrev) {
        throw new Error("Sequenced audit hash chain does not match local transcript");
      }
      if (canonicalMultiplayerPayload(audit.command) !== canonicalMultiplayerPayload(command)) {
        throw new Error("Sequenced audit command does not match broadcast action");
      }
      if (!audit.publicCheckpointHash) {
        throw new Error("Sequenced audit is missing public checkpoint hash");
      }
      const computedHash = await auditStateHash({
        matchId: audit.matchId,
        seq: Number(audit.seq),
        prevStateHash: audit.prevStateHash,
        command: audit.command,
        clock: audit.clock,
        openings: audit.openings || [],
        rngReveals: audit.rngReveals || [],
        shuffleProofs: audit.shuffleProofs || [],
        privateViewProofs: audit.privateViewProofs || [],
        publicCheckpointHash: audit.publicCheckpointHash,
      });
      if (computedHash !== String(audit.nextStateHash || "")) {
        throw new Error("Sequenced audit next state hash is invalid");
      }
      await verifyAuditOpeningsAgainstManifests(audit.openings || [], {
        payload: matchStartPayloadRef.current,
        shuffleProofs: audit.shuffleProofs || [],
      });
      const signer = Number(audit.signer ?? audit.actor);
      if (signer !== Number(audit.actor)) {
        throw new Error("Sequenced action must be signed by the acting player");
      }
      const publicKey = await importCachedAuditPublicKey(publicKeyForAuditSigner(signer));
      const payload = {
        matchId: audit.matchId,
        seq: Number(audit.seq),
        actor: Number(audit.actor),
        signer,
        prevStateHash: audit.prevStateHash,
        command: audit.command,
        clock: audit.clock,
        openings: audit.openings || [],
        rngReveals: audit.rngReveals || [],
        shuffleProofs: audit.shuffleProofs || [],
        privateViewProofs: audit.privateViewProofs || [],
        publicCheckpointHash: audit.publicCheckpointHash,
        nextStateHash: audit.nextStateHash,
      };
      const valid = await verifyAuditPayload(publicKey, payload, audit.signature || "");
      if (!valid) {
        throw new Error("Sequenced audit action signature is invalid");
      }
    },
    [
      currentAuditMatchId,
      importCachedAuditPublicKey,
      publicKeyForAuditSigner,
      verifyAuditOpeningsAgainstManifests,
    ]
  );

  const verifyAuditSatisfiesCryptoRequirements = useCallback(async ({ requirements = [], audit = {} }) => {
    for (const requirement of requirements || []) {
      const type = String(requirement?.type || "");
      if (!type || type === "hidden_move" || type === "hidden_order_update") continue;
      if (type === "public_open") {
        const match =
          (audit.openings || []).find((opening) =>
            openingMatchesRequirement(opening, requirement)
          )
          || localRevealedOpeningForRequirement(requirement);
        if (!match) {
          throw new Error(
            `Missing ${type} audit opening for player ${Number(requirement.owner) + 1}: `
            + JSON.stringify({
              requirement: {
                owner: requirement.owner,
                slot: requirement.slot,
                objectId: requirement.objectId ?? requirement.object_id ?? null,
                card: requirement.card,
                commitment: String(requirement.commitment || "").slice(0, 48),
                positionCommitment: String(
                  requirement.positionCommitment || requirement.position_commitment || ""
                ).slice(0, 64),
                publicSlot: requirement.publicSlot ?? requirement.public_slot ?? null,
                publicCommitment: String(
                  requirement.publicCommitment || requirement.public_commitment || ""
                ).slice(0, 64),
              },
              openings: (audit.openings || [])
                .filter((opening) => Number(opening?.owner) === Number(requirement.owner))
                .slice(0, 80)
                .map((opening) => ({
                  slot: opening.slot,
                  objectId: opening.objectId ?? opening.object_id ?? null,
                  shuffleObjectId: opening.shuffleObjectId ?? opening.shuffle_object_id ?? null,
                  card: opening.card,
                  commitment: String(opening.commitment || "").slice(0, 48),
                  position: opening.position ?? null,
                  positionCommitment: String(
                    opening.positionCommitment || opening.position_commitment || ""
                  ).slice(0, 64),
                  publicSlot: opening.publicSlot ?? opening.public_slot ?? null,
                  publicCommitment: String(
                    opening.publicCommitment || opening.public_commitment || ""
                  ).slice(0, 64),
                })),
            })
          );
        }
        continue;
      }
      if (type === "private_open") {
        if (isOwnerPrivateViewRequirement(requirement)) {
          continue;
        }
        const proof = (audit.privateViewProofs || []).find((entry) =>
          String(entry?.requirementId || "") === String(requirement.id || "")
          || (
            Number(entry?.owner) === Number(requirement.owner)
            && Number(entry?.viewer) === Number(requirement.viewer)
            && Number(entry?.objectId) === Number(requirement.objectId)
          )
        );
        if (!proof?.encryptedOpening?.ciphertextHex || !proof?.encryptedOpening?.plaintextHash) {
          throw new Error(
            `Missing encrypted private opening for player ${Number(requirement.owner) + 1}`
          );
        }
        const recipientKey = auditEncryptionPublicKeyForPlayer(requirement.viewer);
        if (
          recipientKey
          && String(proof.encryptedOpening.recipientPublicKey || "") !== recipientKey
        ) {
          throw new Error("Encrypted private opening targets the wrong viewer key");
        }
        if (
          requirement.commitment
          && (proof.positionCommitment || proof.commitment)
          && ![proof.positionCommitment, proof.commitment]
            .filter(Boolean)
            .some((commitment) => String(commitment) === String(requirement.commitment))
        ) {
          throw new Error("Encrypted private opening commitment mismatch");
        }
        continue;
      }
      if (type === "private_view_window" || type === "public_view_window") {
        if (isOwnerPrivateViewRequirement(requirement)) {
          continue;
        }
        const proofList = type === "private_view_window"
          ? (audit.privateViewProofs || []).filter(
              (entry) => String(entry?.type || "") === "encrypted_private_opening"
            )
          : (audit.openings || []);
        const matchingProofs = proofList.filter((entry) =>
          Number(entry.owner) === Number(requirement.owner)
          && (type !== "private_view_window" || Number(entry.viewer) === Number(requirement.viewer))
        );
        let materialCount = matchingProofs.length;
        if (type === "public_view_window") {
          const known = new Map();
          const addKnownOpening = (entry) => {
            if (!entry || Number(entry.owner) !== Number(requirement.owner)) return;
            const key = [
              Number(entry.owner),
              Number(entry.slot ?? -1),
              Number(entry.objectId ?? -1),
              String(entry.commitment || entry.positionCommitment || ""),
              String(entry.card || ""),
            ].join(":");
            known.set(key, entry);
          };
          matchingProofs.forEach(addKnownOpening);
          for (const opening of localRevealedOpeningsRef.current.values()) {
            addKnownOpening(opening);
          }
          materialCount = known.size;
        }
        if (materialCount < Number(requirement.count || 0)) {
          throw new Error(
            `Missing audit material for ${type} of player ${Number(requirement.owner) + 1}`
          );
        }
        continue;
      }
      if (type === "verifiable_shuffle") {
        const proof = (audit.shuffleProofs || []).find((entry) =>
          shuffleProofMatchesRequirement(entry, requirement)
        );
        if (!proof) {
          throw new Error(
            `Missing verifiable shuffle proof for player ${Number(requirement.owner) + 1}`
          );
        }
        continue;
      }
      if (type === "fair_random") {
        const reveal = (audit.rngReveals || []).find(
          (entry) => String(entry?.requirementId || "") === String(requirement.id || "")
        );
        if (!reveal) {
          throw new Error("Missing transcripted fair-random reveal");
        }
        const expectedPlayers = reindexPlayers(
          matchStartPayloadRef.current?.players || multiplayerRef.current.players || []
        ).map((player) => Number(player.index)).sort((left, right) => left - right);
        const commits = Array.isArray(reveal.commits) ? reveal.commits : [];
        const reveals = Array.isArray(reveal.reveals) ? reveal.reveals : [];
        if (commits.length !== expectedPlayers.length || reveals.length !== expectedPlayers.length) {
          throw new Error("Transcripted fair-random reveal must include every player");
        }
        for (let index = 0; index < expectedPlayers.length; index += 1) {
          if (
            Number(commits[index]?.player) !== expectedPlayers[index]
            || Number(reveals[index]?.player) !== expectedPlayers[index]
          ) {
            throw new Error("Transcripted fair-random material must be sorted and complete");
          }
        }
        const seenPlayers = new Set();
        for (const commit of commits) {
          const player = Number(commit?.player);
          if (seenPlayers.has(player)) {
            throw new Error("Transcripted fair-random material contains a duplicate player");
          }
          seenPlayers.add(player);
          await verifyRngCommitmentEntry(commit, {
            matchId: currentAuditMatchId(),
            seq: Number(audit.seq),
            requirementId: String(requirement.id || ""),
            requester: Number(commit?.requester),
            player,
          });
        }
        for (const entry of reveal.reveals || []) {
          const expected = (reveal.commits || []).find(
            (commit) => Number(commit.player) === Number(entry.player)
          );
          const actual = await rngCommitmentForNonce(entry.nonceHex);
          if (
            !expected
            || actual !== String(expected.commitmentHex || "")
            || (entry.commitmentHex && actual !== String(entry.commitmentHex || ""))
          ) {
            throw new Error("Transcripted fair-random reveal does not match commitment");
          }
          await verifyRngRevealEntry(entry, {
            matchId: currentAuditMatchId(),
            seq: Number(audit.seq),
            requirementId: String(requirement.id || ""),
            requester: Number(entry?.requester),
            player: Number(entry?.player),
          });
        }
        const combinedSeedHex = await fairRandomCombinedSeedHex({
          matchId: currentAuditMatchId(),
          seq: Number(audit.seq),
          requirementId: String(requirement.id || ""),
          commits: reveal.commits || [],
          reveals: reveal.reveals || [],
        });
        if (combinedSeedHex !== String(reveal.combinedSeedHex || "")) {
          throw new Error("Transcripted fair-random combined seed is invalid");
        }
      }
    }
  }, [auditEncryptionPublicKeyForPlayer, currentAuditMatchId, localRevealedOpeningForRequirement]);

  function previewAuditOpeningInInspector(opening, latestState = null, options = {}) {
    if (options.previewInspector === false) return;
    if (!opening || !opening.card) return;
    const viewedCards =
      latestState?.viewed_cards
      || latestState?.active_viewed_cards
      || null;
    const fallbackObjectId = Number(opening.objectId ?? opening.object_id);
    const fallbackStableId = Number(opening.stableId ?? opening.stable_id);
    const fallbackCard = {
      id: Number.isSafeInteger(fallbackObjectId) && fallbackObjectId > 0
        ? fallbackObjectId
        : (Number.isSafeInteger(fallbackStableId) ? fallbackStableId : undefined),
      stable_id: Number.isSafeInteger(fallbackStableId) ? fallbackStableId : undefined,
      name: String(opening.card || ""),
      owner: Number(opening.owner),
      controller: Number(opening.owner),
      zone: String(opening.zone || opening.toZone || options.previewZone || "exile"),
      slot: Number.isSafeInteger(Number(opening.slot)) ? Number(opening.slot) : undefined,
    };
    const nextViewedCards = {
      ...(viewedCards || {}),
      visibility: viewedCards?.visibility || "public",
      subject: viewedCards?.subject ?? Number(opening.owner),
      zone: viewedCards?.zone || fallbackCard.zone,
      description: `Revealing ${String(opening.card || "")}`,
      cards: [fallbackCard],
      card_ids: [],
      inspector_only: true,
      transient_inspector_preview: true,
    };
    const previewIndex = Number(options.previewIndex);
    const previewTotal = Number(options.previewTotal);
    updateMultiplayer((prev) => {
      if (!prev.peerWait) return prev;
      return {
        ...prev,
        peerWait: {
          ...prev.peerWait,
          operation: "Opening revealed card",
          cardName: String(opening.card || ""),
          zone: String(nextViewedCards.zone || fallbackCard.zone || ""),
          openingPreviews: mergeActionOpeningPreviews(
            prev.peerWait.openingPreviews || [],
            [fallbackCard]
          ),
          progressCurrent:
            Number.isSafeInteger(previewIndex) && Number.isSafeInteger(previewTotal) && previewTotal > 0
              ? Math.min(previewTotal, previewIndex + 1)
              : prev.peerWait.progressCurrent,
          progressTotal:
            Number.isSafeInteger(previewTotal) && previewTotal > 0
              ? previewTotal
              : prev.peerWait.progressTotal,
        },
      };
    });
    const base = stateRef.current || latestState || {};
    setState({
      ...base,
      viewed_cards: nextViewedCards,
    });
  }

  const revealAuditOpenings = useCallback(async (openings = [], options = {}) => {
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.revealHiddenSlot !== "function") return;
    const timing = options.timing || null;
    const commandObjectIds = timing === "pre"
      ? collectCommandObjectIds(options.command, new Set(), options.uiState || stateRef.current)
      : new Set();
    if (timing === "pre") {
      await addResolvedSelectObjectCommandIds(
        commandObjectIds,
        options.command,
        options.uiState || stateRef.current
      );
    }
    if (timing === "post" && Array.isArray(openings) && openings.length > 0) {
      try {
        const checkpoint = await currentGame.exportSyncCheckpoint?.();
        const inconsistent = (checkpoint?.objects || []).filter((object) => {
          const hidden = object?.hiddenCard || object?.hidden_card || null;
          if (!hidden) return false;
          const commitment = String(hidden.commitment || "");
          const position = zifflePositionFromCommitment(commitment);
          return position != null
            && Number(hidden.slot) !== Number(position)
            && (hidden.publicSlot ?? hidden.public_slot) == null;
        }).slice(0, 8).map((object) => {
          const hidden = object.hiddenCard || object.hidden_card || {};
          return {
            id: object.id,
            name: object.name,
            zone: object.zone,
            slot: hidden.slot,
            commitment: hidden.commitment,
            publicSlot: hidden.publicSlot ?? hidden.public_slot ?? null,
          };
        });
        if (inconsistent.length > 0) {
          throw new Error(`pre-post-open hidden ziffle inconsistency ${JSON.stringify(inconsistent)}`);
        }
      } catch (err) {
        if (String(err?.message || err || "").includes("pre-post-open hidden ziffle inconsistency")) {
          throw err;
        }
      }
    }
    let changed = false;
    let latestState = null;
    const debugProcessedPostOpenings = [];
    const openingList = Array.isArray(openings) ? openings : [];
	    for (const [openingIndex, rawOpening] of openingList.entries()) {
	      let opening = await sanitizeObjectBoundOpening(rawOpening);
	      if (!opening || opening.owner == null || opening.slot == null || !opening.card) {
	        continue;
	      }
	      if (Number(opening.owner) === Number(resolveLocalCryptoPlayerIndex())) {
	        opening = await ensureZiffleOpeningProof(opening, options);
	        opening = await sanitizeObjectBoundOpening(opening);
	      }
      const opensCommandObject =
        opening.objectId != null && commandObjectIds.has(Number(opening.objectId));
      const recomputeDecision = Boolean(timing === "pre" && opensCommandObject);
      if (timing && String(opening.timing || "pre") !== timing && !opensCommandObject) {
        continue;
      }
	      let localHiddenMetadata = null;
	      let debugOpeningEntry = null;
	      try {
	        await verifyAuditOpeningsAgainstManifests([opening], options);
	        const openingObjectId = opening.objectId == null ? null : Number(opening.objectId);
	        let localRevealObjectId =
	          Number.isSafeInteger(openingObjectId) && openingObjectId > 0
	            ? openingObjectId
	            : null;
		        localHiddenMetadata = opening.objectId != null
		          ? await currentHiddenCardMetadataForObject(opening.objectId)
		          : null;
		        const openingPositionCommitment = String(opening.positionCommitment || "");
		        if (
		          localHiddenMetadata
		          && opening.position != null
		          && ziffleDeckHashFromCommitment(openingPositionCommitment)
		          && !hiddenMetadataMatchesZifflePosition(
		            localHiddenMetadata,
		            opening.position,
		            openingPositionCommitment
		          )
		        ) {
		          localRevealObjectId = null;
		          localHiddenMetadata = null;
		        }
		        if (
		          localHiddenMetadata
		          && opening.position != null
		          && ziffleDeckHashFromCommitment(openingPositionCommitment)
		          && Number(localHiddenMetadata.slot) !== Number(opening.slot)
		        ) {
		          localRevealObjectId = null;
		          localHiddenMetadata = null;
		        }
		        let checkpoint = null;
		        let ignoredExplicitObjectIdentity = false;
		        if (
		          localHiddenMetadata
		          && openingObjectId != null
		          && opening.card
		          && typeof currentGame.exportSyncCheckpoint === "function"
		        ) {
		          try {
		            checkpoint = await currentGame.exportSyncCheckpoint();
		          } catch {
		            checkpoint = null;
		          }
		          const explicitObject = checkpointObjectForId(checkpoint, openingObjectId);
		          if (
		            explicitObject
		            && !checkpointObjectIsRedactedHidden(explicitObject)
		            && checkpointObjectName(explicitObject)
		            && checkpointObjectName(explicitObject) !== String(opening.card || "").trim()
		          ) {
		            ignoredExplicitObjectIdentity = true;
		            localRevealObjectId = null;
		            localHiddenMetadata = null;
		          }
		        }
		        debugOpeningEntry = {
	          slot: Number(opening.slot),
	          objectId: opening.objectId == null ? null : Number(opening.objectId),
	          card: String(opening.card || ""),
	          position: opening.position == null ? null : Number(opening.position),
	          positionCommitment: String(opening.positionCommitment || "").slice(0, 32),
	          source: String(opening._debugResolveSource || ""),
	          hiddenSlot: localHiddenMetadata?.slot == null ? null : Number(localHiddenMetadata.slot),
	          hiddenCommitment: String(localHiddenMetadata?.commitment || "").slice(0, 32),
	          hiddenPublicSlot: localHiddenMetadata?.publicSlot == null ? null : Number(localHiddenMetadata.publicSlot),
	        };
		        let explicitObjectPresent = false;
		        let explicitObjectExistsWithoutHidden = false;
	        if (!localHiddenMetadata && typeof currentGame.exportSyncCheckpoint === "function") {
	          if (!checkpoint) {
	            try {
	              checkpoint = await currentGame.exportSyncCheckpoint();
	            } catch {
	              checkpoint = null;
	            }
	          }
		          const explicitObject = checkpointObjectForId(checkpoint, openingObjectId);
		          explicitObjectPresent = Boolean(explicitObject);
		          explicitObjectExistsWithoutHidden = Boolean(
		            explicitObject && !checkpointObjectHiddenCard(explicitObject)
		          );
	          if (knownCheckpointObjectMatchesOpening(explicitObject, opening)) {
	            rememberLocalRevealedOpening(opening, {
	              objectId: openingObjectId,
	              position: opening.position,
	              positionCommitment: opening.positionCommitment,
	            });
	            continue;
	          }
	          if (
	            explicitObject
	            && !checkpointObjectHiddenCard(explicitObject)
	            && !ignoredExplicitObjectIdentity
	            && opening.card
	            && checkpointObjectName(explicitObject) !== String(opening.card || "").trim()
	          ) {
	            throw new Error(
	              `opened object identity does not match reveal `
	              + `(object ${Number(openingObjectId)} is ${checkpointObjectName(explicitObject) || "unknown"}, `
	              + `opening is ${String(opening.card || "")})`
	            );
	          }
	          const resolvedObjectId = hiddenObjectIdForOpeningFromCheckpoint(checkpoint, opening);
	          if (resolvedObjectId != null) {
	            const resolvedMetadata = hiddenCardMetadataForObjectFromCheckpoint(
	              checkpoint,
	              resolvedObjectId
	            );
	            if (resolvedMetadata) {
	              localRevealObjectId = resolvedObjectId;
	              localHiddenMetadata = resolvedMetadata;
		            }
			          }
			        }
			        const localOwnerAlreadyKnowsObjectOpening = Boolean(
			          timing === "post"
			          && Number(opening.owner) === Number(resolveLocalCryptoPlayerIndex())
			          && opening.objectId != null
			          && localHiddenMetadata
			          && !openingHasZifflePosition(opening)
			          && Number(localHiddenMetadata.owner) === Number(opening.owner)
			          && Number(localHiddenMetadata.slot) === Number(opening.slot)
			          && opening.commitment
			          && !ziffleDeckHashFromCommitment(opening.commitment)
			          && (
			            String(localHiddenMetadata.commitment || "") === String(opening.commitment || "")
			            || String(localHiddenMetadata.publicCommitment || "") === String(opening.commitment || "")
			          )
			        );
			        if (localOwnerAlreadyKnowsObjectOpening) {
			          rememberLocalRevealedOpening(opening, {
			            objectId: localRevealObjectId ?? opening.objectId,
			          });
			          previewAuditOpeningInInspector(opening, latestState, {
			            ...options,
			            previewIndex: openingIndex,
			            previewTotal: openingList.length,
			          });
			          if (debugOpeningEntry) {
			            debugProcessedPostOpenings.push({ ...debugOpeningEntry, status: "ok" });
			            if (debugProcessedPostOpenings.length > 10) {
			              debugProcessedPostOpenings.shift();
			            }
			          }
			          continue;
			        }
			        if (
			          !localHiddenMetadata
			          && opening.position != null
			          && ziffleDeckHashFromCommitment(openingPositionCommitment)
			        ) {
		          localRevealObjectId = null;
		        }
			        const localHiddenZiffleCommitment =
			          localHiddenMetadata?.commitment
			          && ziffleDeckHashFromCommitment(localHiddenMetadata.commitment)
	            ? String(localHiddenMetadata.commitment)
	            : "";
	        const openingPublicZiffleCommitment = String(
	          opening.publicCommitment || opening.public_commitment || ""
	        );
	        const openingPublicZifflePosition =
	          zifflePositionFromCommitment(openingPublicZiffleCommitment)
	          ?? (
	            ziffleDeckHashFromCommitment(openingPublicZiffleCommitment)
	            && opening.publicSlot != null
	              ? Number(opening.publicSlot)
	              : opening.public_slot != null
	                ? Number(opening.public_slot)
	                : null
	          );
	        const revealPosition =
	          opening.position != null
	            ? Number(opening.position)
	            : localHiddenZiffleCommitment && localHiddenMetadata?.slot != null
	              ? Number(localHiddenMetadata.slot)
	              : openingPublicZifflePosition;
		        const revealPositionCommitment =
		          String(opening.positionCommitment || "")
		          || localHiddenZiffleCommitment
		          || openingPublicZiffleCommitment;
	        const isZifflePositionReveal = Boolean(
	          revealPosition != null
	          && ziffleDeckHashFromCommitment(revealPositionCommitment)
	        );
	        if (isZifflePositionReveal && localRevealObjectId != null) {
	          const metadataMatchesReveal =
	            localHiddenMetadata
	            && hiddenMetadataMatchesZifflePosition(
	              localHiddenMetadata,
	              revealPosition,
	              revealPositionCommitment
	            );
	          const metadataMatchesOriginalSlot =
	            localHiddenMetadata
	            && localHiddenMetadata.slot != null
	            && Number(localHiddenMetadata.slot) === Number(opening.slot);
	          if (!metadataMatchesReveal || !metadataMatchesOriginalSlot) {
	            localRevealObjectId = null;
	            localHiddenMetadata = null;
	          }
	        }
	        const revealByObjectMetadata = async () => {
	          if (
	            localRevealObjectId == null
	            || !localHiddenMetadata
	            || typeof currentGame.revealHiddenObject !== "function"
	          ) {
            return null;
	          }
	          const metadataSlot = Number(localHiddenMetadata.slot);
	          const metadataCommitment = String(localHiddenMetadata.commitment || "");
	          const metadataPublicSlot = localHiddenMetadata.publicSlot == null
	            ? null
	            : Number(localHiddenMetadata.publicSlot);
	          const metadataPublicCommitment = String(localHiddenMetadata.publicCommitment || "");
	          const matchesOriginal =
	            Number.isSafeInteger(metadataSlot)
	            && metadataSlot === Number(opening.slot)
	            && (
	              !opening.commitment
	              || metadataCommitment === String(opening.commitment || "")
	              || metadataPublicCommitment === String(opening.commitment || "")
	            );
	          const matchesPosition =
	            revealPosition != null
	            && (
	              metadataSlot === Number(revealPosition)
	              || metadataPublicSlot === Number(revealPosition)
	            )
	            && (
	              !revealPositionCommitment
	              || metadataCommitment === revealPositionCommitment
	              || metadataPublicCommitment === revealPositionCommitment
	            );
          if (!matchesOriginal && !matchesPosition) {
            return null;
	          }
	          return currentGame.revealHiddenObject({
	            objectId: Number(localRevealObjectId),
	            slot: metadataSlot,
	            cardName: String(opening.card),
	            commitment: metadataCommitment || undefined,
            recomputeDecision,
          });
        };
        const revealByCommittedSlot = () => currentGame.revealHiddenSlot({
          owner: Number(opening.owner),
          slot: Number(opening.slot),
          cardName: String(opening.card),
          commitment: opening.commitment || undefined,
          recomputeDecision,
        });
        if (
          revealPosition != null
          && typeof currentGame.revealHiddenPosition === "function"
        ) {
          const ceremony = ziffleCeremonyForOwner(opening.owner, {
            commitment: revealPositionCommitment,
            context: ziffleContextFromOpening(opening),
          });
          try {
            latestState = await currentGame.revealHiddenPosition({
              owner: Number(opening.owner),
              ...(localRevealObjectId != null ? { objectId: Number(localRevealObjectId) } : {}),
              position: Number(revealPosition),
              originalSlot: Number(opening.slot),
              cardName: String(opening.card),
              positionCommitment: revealPositionCommitment
                || (ceremony
                  ? ziffleRuntimeCommitment(ceremony.deckHash, revealPosition)
                  : undefined),
              commitment: opening.commitment || undefined,
              recomputeDecision,
            });
          } catch (err) {
            const message = String(err?.message || err || "");
            if (!message.includes("not present") && !message.includes("not a hidden")) {
              throw err;
            }
            latestState = await revealByObjectMetadata();
            if (!latestState) {
              const metadataSlot = localHiddenMetadata?.slot == null
                ? null
                : Number(localHiddenMetadata.slot);
              const metadataCommitment = String(localHiddenMetadata?.commitment || "");
              const canRevealByCommittedSlot =
                (
                  metadataSlot === Number(opening.slot)
                  && (!opening.commitment || metadataCommitment === String(opening.commitment || ""))
                )
	                || (
	                  !localHiddenMetadata
	                  && !explicitObjectExistsWithoutHidden
	                  && !isZifflePositionReveal
	                  && Boolean(opening.commitment || opening.positionCommitment)
	                );
              if (
                canRevealByCommittedSlot
              ) {
                latestState = await revealByCommittedSlot();
              } else {
                throw err;
              }
            }
          }
        } else {
	          latestState = await revealByObjectMetadata();
		          const hasSpecificOpeningObject = opening.objectId !== null && opening.objectId !== undefined;
		          const explicitObjectMissing =
		            hasSpecificOpeningObject
		            && checkpoint
		            && !explicitObjectPresent;
		          const mayRevealCommittedSlot =
		            !explicitObjectExistsWithoutHidden
		            && (!hasSpecificOpeningObject || localHiddenMetadata || explicitObjectMissing)
		            && (
		              opening.objectId == null
		              || localHiddenMetadata
		              || explicitObjectMissing
		              || (!hasSpecificOpeningObject && (opening.commitment || opening.positionCommitment))
		            );
          if (!latestState && mayRevealCommittedSlot) {
            latestState = await revealByCommittedSlot();
          }
	        }
	        rememberLocalRevealedOpening(opening, {
	          objectId: localRevealObjectId ?? opening.objectId,
	          position: revealPosition,
	          positionCommitment: revealPositionCommitment,
	        });
        previewAuditOpeningInInspector(opening, latestState, {
          ...options,
          previewIndex: openingIndex,
          previewTotal: openingList.length,
        });
	        if (debugOpeningEntry) {
	          debugProcessedPostOpenings.push({ ...debugOpeningEntry, status: "ok" });
	          if (debugProcessedPostOpenings.length > 10) {
	            debugProcessedPostOpenings.shift();
	          }
	        }
        changed = true;
      } catch (err) {
        const message = String(err?.message || err || "");
        if (
          opensCommandObject
          || (!message.includes("not present") && !message.includes("not a hidden"))
        ) {
          throw new Error(
            `${message}; opening owner ${Number(opening.owner)} slot ${Number(opening.slot)}`
            + ` object ${opening.objectId == null ? "none" : Number(opening.objectId)}`
            + ` card ${String(opening.card || "")}`
            + ` resolveSource ${String(opening._debugResolveSource || "none")}`
            + ` shuffleOriginalSlot ${opening._debugShuffleOriginalSlot == null ? "none" : Number(opening._debugShuffleOriginalSlot)}`
            + ` resolvedSlot ${opening._debugResolvedSlot == null ? "none" : Number(opening._debugResolvedSlot)}`
            + ` resolvedObject ${opening._debugResolvedObjectId == null ? "none" : Number(opening._debugResolvedObjectId)}`
            + ` commitment ${String(opening.commitment || "").slice(0, 24) || "none"}`
            + ` position ${opening.position == null ? "none" : Number(opening.position)}`
            + ` positionCommitment ${String(opening.positionCommitment || "").slice(0, 32) || "none"}`
            + ` hiddenSlot ${localHiddenMetadata?.slot == null ? "none" : Number(localHiddenMetadata.slot)}`
            + ` hiddenCommitment ${String(localHiddenMetadata?.commitment || "").slice(0, 32) || "none"}`
            + ` hiddenPublicSlot ${localHiddenMetadata?.publicSlot == null ? "none" : Number(localHiddenMetadata.publicSlot)}`
            + ` hiddenPublicCommitment ${String(localHiddenMetadata?.publicCommitment || "").slice(0, 32) || "none"}`
            + ` processed ${JSON.stringify(debugProcessedPostOpenings)}`
          );
        }
      }
    }
    if (changed && options.updateState !== false) {
      const nextState = latestState || await currentGame.uiState();
      stateRef.current = nextState;
      setState(nextState);
    }
    return latestState;
  }, [
	    currentHiddenCardMetadataForObject,
	    rememberLocalRevealedOpening,
	    sanitizeObjectBoundOpening,
	    setState,
    verifyAuditOpeningsAgainstManifests,
    ziffleCeremonyForOwner,
  ]);

  async function previewRequirementsForCommand(command) {
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.previewCryptoRequirements !== "function") {
      return [];
    }
    // cancel_decision / forfeit_player are not engine decision commands: routing
    // them through previewCryptoRequirements (which dispatches the command) would
    // throw "invalid command payload: unknown variant ...". They never produce
    // hidden-card material, so they have no crypto requirements.
    if (isNonDispatchSyncCommand(command)) {
      return [];
    }
    const requirements = await currentGame.previewCryptoRequirements(command);
    return Array.isArray(requirements) ? requirements : [];
  }

  async function currentHiddenObjectIdForOpening(opening) {
    if (!opening || opening.owner == null) return null;
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.exportSyncCheckpoint !== "function") {
      return null;
    }
    let checkpoint = null;
    try {
      checkpoint = await currentGame.exportSyncCheckpoint();
    } catch {
      return null;
    }
    return hiddenObjectIdForOpeningFromCheckpoint(checkpoint, opening);
  }

  function commandObjectStableIds(command) {
    const stableIds = Array.isArray(command?.object_stable_ids)
      ? command.object_stable_ids
      : Array.isArray(command?.objectStableIds)
        ? command.objectStableIds
        : [];
    return stableIds.map((stableId) => {
      const normalized = Number(stableId);
      return Number.isSafeInteger(normalized) && normalized > 0 ? normalized : null;
    });
  }

  function commandObjectHiddenRefs(command) {
    const hiddenRefs = Array.isArray(command?.object_hidden_refs)
      ? command.object_hidden_refs
      : Array.isArray(command?.objectHiddenRefs)
        ? command.objectHiddenRefs
        : [];
    return hiddenRefs.map(normalizeSelectObjectHiddenRef);
  }

  function openingMatchesCommandHiddenRef(opening, hiddenRef) {
    const ref = normalizeSelectObjectHiddenRef(hiddenRef);
    if (!opening || !ref) return false;
    if (ref.owner != null && Number(opening.owner) !== Number(ref.owner)) {
      return false;
    }
    const openingCommitments = [
      opening.publicCommitment,
      opening.public_commitment,
      opening.positionCommitment,
      opening.position_commitment,
      opening.commitment,
    ].map((entry) => String(entry || "")).filter(Boolean);
    const refCommitments = [
      ref.public_commitment,
      ref.commitment,
    ].map((entry) => String(entry || "")).filter(Boolean);
    if (refCommitments.length > 0) {
      const openingPositionCommitment = String(
        opening.positionCommitment || opening.position_commitment || opening.publicCommitment || opening.public_commitment || ""
      );
      const openingDeckHash = ziffleDeckHashFromCommitment(openingPositionCommitment);
      const openingPosition =
        zifflePositionFromCommitment(openingPositionCommitment)
        ?? (opening.position == null ? null : Number(opening.position))
        ?? (opening.publicSlot == null ? null : Number(opening.publicSlot));
      for (const refCommitment of refCommitments) {
        if (openingCommitments.includes(refCommitment)) return true;
        const refDeckHash = ziffleDeckHashFromCommitment(refCommitment);
        const refPosition =
          zifflePositionFromCommitment(refCommitment)
          ?? (ref.public_slot == null ? null : Number(ref.public_slot));
        if (
          refDeckHash
          && openingDeckHash === refDeckHash
          && openingPosition != null
          && refPosition != null
          && Number(openingPosition) === Number(refPosition)
        ) {
          return true;
        }
      }
      return false;
    }
    if (ref.public_slot != null) {
      const openingPosition = opening.position ?? opening.publicSlot ?? opening.public_slot;
      return Number(openingPosition) === Number(ref.public_slot);
    }
    if (ref.slot != null) {
      return Number(opening.slot) === Number(ref.slot);
    }
    return false;
  }

  function filterOpeningsForCommandHiddenRefs(openings = [], command = null) {
    if (command?.type !== "select_objects") return openings;
    const hiddenRefs = commandObjectHiddenRefs(command).filter(Boolean);
    if (hiddenRefs.length === 0) return openings;
    return (openings || []).filter((opening) =>
      hiddenRefs.some((hiddenRef) => openingMatchesCommandHiddenRef(opening, hiddenRef))
    );
  }

  async function currentObjectIdForStableId(stableId) {
    const normalized = Number(stableId);
    if (!Number.isSafeInteger(normalized) || normalized <= 0) return null;
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.exportSyncCheckpoint !== "function") {
      return null;
    }
    let checkpoint = null;
    try {
      checkpoint = await currentGame.exportSyncCheckpoint();
    } catch {
      return null;
    }
    const object = (checkpoint?.objects || []).find((entry) =>
      Number(entry?.stableId ?? entry?.stable_id) === normalized
    );
    const objectId = Number(object?.id);
    return Number.isSafeInteger(objectId) && objectId > 0 ? objectId : null;
  }

  async function currentStableIdForObjectId(objectId) {
    const normalized = Number(objectId);
    if (!Number.isSafeInteger(normalized) || normalized <= 0) return null;
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.exportSyncCheckpoint !== "function") {
      return null;
    }
    let checkpoint = null;
    try {
      checkpoint = await currentGame.exportSyncCheckpoint();
    } catch {
      return null;
    }
    const object = (checkpoint?.objects || []).find((entry) =>
      Number(entry?.id) === normalized
    );
    const stableId = Number(object?.stableId ?? object?.stable_id);
    return Number.isSafeInteger(stableId) && stableId > 0 ? stableId : null;
  }

  async function currentHiddenRefForObjectId(objectId) {
    const hidden = await currentHiddenCardMetadataForObject(objectId);
    let exported = null;
    const currentGame = gameRef.current;
    if (currentGame && typeof currentGame.exportHiddenCardOpening === "function") {
      try {
        exported = await currentGame.exportHiddenCardOpening(wasmObjectIdArg(objectId));
      } catch {
        exported = null;
      }
    }
    return normalizeSelectObjectHiddenRef({
      owner: hidden?.owner ?? exported?.owner,
      zone: hidden?.zone,
      slot: hidden?.slot ?? exported?.slot,
      commitment: hidden?.commitment ?? exported?.commitment,
      public_slot:
        hidden?.publicSlot
        ?? hidden?.public_slot
        ?? exported?.publicSlot
        ?? exported?.public_slot,
      public_commitment:
        hidden?.publicCommitment
        ?? hidden?.public_commitment
        ?? exported?.publicCommitment
        ?? exported?.public_commitment,
    });
  }

  async function currentObjectIdForHiddenRef(hiddenRef) {
    if (!normalizeSelectObjectHiddenRef(hiddenRef)) return null;
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.exportSyncCheckpoint !== "function") {
      return null;
    }
    let checkpoint = null;
    try {
      checkpoint = await currentGame.exportSyncCheckpoint();
    } catch {
      return null;
    }
    return hiddenObjectIdForHiddenRefFromCheckpoint(checkpoint, hiddenRef);
  }

  async function remapPriorityCommandForLocalHiddenOpening(command, openings = []) {
    if (!command || command.type !== "priority_action" || !command.action_ref) {
      return command;
    }
    const sourceObjectId = actionRefObjectId(command.action_ref);
    if (sourceObjectId == null) return command;

    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.uiState !== "function") return command;

    const liveState = await currentGame.uiState();
    const actions = Array.isArray(liveState?.decision?.actions)
      ? liveState.decision.actions
      : [];
    const sameKindActions = actions.filter(
      (action) => String(action?.action_ref?.kind || "") === String(command.action_ref.kind || "")
    );
    if (sameKindActions.some((action) =>
      Number(actionRefObjectId(action?.action_ref)) === Number(sourceObjectId)
    )) {
      return command;
    }

    const stableId = Number(command.object_stable_id ?? command.objectStableId);
    if (Number.isSafeInteger(stableId) && stableId > 0) {
      const stableObjectId = await currentObjectIdForStableId(stableId);
      const stableAction = sameKindActions.find((action) =>
        Number(actionRefObjectId(action?.action_ref)) === Number(stableObjectId)
      );
      if (stableAction) {
        const localObjectId = actionRefObjectId(stableAction.action_ref);
        const remapped = cloneMultiplayerPayload(command);
        remapped.action_ref = actionRefWithObjectId(command.action_ref, localObjectId);
        remapped.object_id = Number(localObjectId);
        return remapped;
      }
    }

    const hiddenRef = normalizeSelectObjectHiddenRef(
      command.object_hidden_ref ?? command.objectHiddenRef
    );
    if (hiddenRef) {
      const hiddenObjectId = await currentObjectIdForHiddenRef(hiddenRef);
      const hiddenAction = sameKindActions.find((action) =>
        Number(actionRefObjectId(action?.action_ref)) === Number(hiddenObjectId)
      );
      if (hiddenAction) {
        const localObjectId = actionRefObjectId(hiddenAction.action_ref);
        const remapped = cloneMultiplayerPayload(command);
        remapped.action_ref = actionRefWithObjectId(command.action_ref, localObjectId);
        remapped.object_id = Number(localObjectId);
        return remapped;
      }
    }

    const candidateOpenings = (openings || []).filter(
      (opening) => opening?.owner != null && opening?.slot != null && opening?.card
    );
    if (candidateOpenings.length === 0) return command;

    for (const action of sameKindActions) {
      const localObjectId = actionRefObjectId(action?.action_ref);
      if (localObjectId == null || typeof currentGame.exportHiddenCardOpening !== "function") {
        continue;
      }
      let exported = null;
      try {
        exported = await currentGame.exportHiddenCardOpening(wasmObjectIdArg(localObjectId));
      } catch {
        continue;
      }
      const opening = candidateOpenings.find((entry) =>
        hiddenOpeningMatchesExport(entry, exported)
      );
      if (!opening) continue;
      const remapped = cloneMultiplayerPayload(command);
      remapped.action_ref = actionRefWithObjectId(command.action_ref, localObjectId);
      remapped.object_id = Number(localObjectId);
      return remapped;
    }

    return command;
  }

  async function remapSelectObjectsCommandForLocalHiddenOpening(command, openings = [], actorIndex = null) {
    if (!command || command.type !== "select_objects" || !Array.isArray(command.object_ids)) {
      return command;
    }

    const selectedIds = command.object_ids.map((objectId) => Number(objectId));
    if (selectedIds.length === 0) return command;

    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.uiState !== "function") return command;

    const liveState = await currentGame.uiState();
    if (String(liveState?.decision?.kind || "") !== "select_objects") return command;
    const visibleCandidateIds = new Set(
      (liveState.decision.candidates || [])
        .map((candidate) => Number(candidate?.id))
        .filter((objectId) => Number.isSafeInteger(objectId) && objectId > 0)
    );
    const selectedStableIds = commandObjectStableIds(command);
    const hasStableIds = selectedStableIds.some((stableId) => stableId != null);
    const selectedHiddenRefs = commandObjectHiddenRefs(command);
    const hasHiddenRefs = selectedHiddenRefs.some((hiddenRef) => hiddenRef != null);
    if (
      !hasStableIds
      && !hasHiddenRefs
      && selectedIds.every((objectId) => visibleCandidateIds.has(objectId))
    ) {
      return command;
    }

    let candidateOpenings = (openings || []).filter(
      (opening) => opening?.owner != null && opening?.slot != null && opening?.card
    );
    const actorOwnedOpenings = Number.isInteger(Number(actorIndex))
      ? candidateOpenings.filter((opening) => Number(opening.owner) === Number(actorIndex))
      : [];
    if (actorOwnedOpenings.length > 0) {
      candidateOpenings = actorOwnedOpenings;
    }

    const usedOpenings = new Set();
    let changed = false;
    const objectIds = [];
    for (const [index, selectedId] of selectedIds.entries()) {
      const stableId = selectedStableIds[index];
      const localStableObjectId = stableId == null
        ? null
        : await currentObjectIdForStableId(stableId);
      if (localStableObjectId != null && visibleCandidateIds.has(localStableObjectId)) {
        objectIds.push(localStableObjectId);
        if (Number(localStableObjectId) !== Number(selectedId)) changed = true;
        continue;
      }

      const hiddenRef = selectedHiddenRefs[index];
      const localHiddenRefObjectId = hiddenRef == null
        ? null
        : await currentObjectIdForHiddenRef(hiddenRef);
      if (localHiddenRefObjectId != null && visibleCandidateIds.has(localHiddenRefObjectId)) {
        objectIds.push(localHiddenRefObjectId);
        if (Number(localHiddenRefObjectId) !== Number(selectedId)) changed = true;
        continue;
      }

      if (visibleCandidateIds.has(selectedId)) {
        objectIds.push(selectedId);
        continue;
      }

      if (candidateOpenings.length === 0) {
        objectIds.push(selectedId);
        continue;
      }

      const orderedOpenings = [
        ...candidateOpenings.filter((opening) => Number(opening.objectId ?? opening.object_id) === selectedId),
        ...candidateOpenings.filter((opening) => Number(opening.objectId ?? opening.object_id) !== selectedId),
      ];
      let localObjectId = null;
      for (const opening of orderedOpenings) {
        if (usedOpenings.has(opening)) continue;
        localObjectId = await currentHiddenObjectIdForOpening(opening);
        if (localObjectId == null) continue;
        usedOpenings.add(opening);
        break;
      }
      if (localObjectId != null) {
        objectIds.push(localObjectId);
        changed = true;
      } else {
        objectIds.push(selectedId);
      }
    }

    if (!changed) return command;
    return {
      ...cloneMultiplayerPayload(command),
      object_ids: objectIds,
    };
  }

  async function remapCommandForLocalHiddenOpening(command, openings = [], actorIndex = null) {
    const priorityCommand = await remapPriorityCommandForLocalHiddenOpening(command, openings);
    return remapSelectObjectsCommandForLocalHiddenOpening(priorityCommand, openings, actorIndex);
  }

  async function privateOpeningFromEncryptedProof(proof, requirement = {}, options = {}) {
    const localSeat = resolveLocalCryptoPlayerIndex();
    const viewer = Number(requirement?.viewer ?? proof?.viewer);
    if (!Number.isInteger(viewer) || viewer !== Number(localSeat)) return null;
    if (!proof?.encryptedOpening?.ciphertextHex) return null;
    const { encryptionKeyPair } = await ensureAuditIdentity();
    const payload = await decryptPrivateAuditPayload({
      keyPair: encryptionKeyPair,
      encrypted: proof.encryptedOpening,
    });
    if (
      payload?.matchId
      && String(payload.matchId || "") !== String(currentAuditMatchId())
    ) {
      throw new Error("Private-view opening belongs to a different match");
    }
    const opening = payload?.opening || null;
    if (!opening || opening.owner == null || opening.slot == null || !opening.card) {
      throw new Error("Private-view opening payload is incomplete");
    }
    if (options.persistDisclosure !== false) {
      rememberPrivateViewDisclosure({
        type: "private_view_opening_disclosure",
        matchId: currentAuditMatchId(),
        seq: Number(options.seq ?? requirement?.seq ?? proof?.seq ?? 0),
        requirementId: String(proof.requirementId || payload.requirementId || ""),
        owner: Number(proof.owner ?? payload.owner ?? opening.owner),
        viewer,
        zone: String(proof.zone || payload.zone || ""),
        objectId: Number(proof.objectId ?? payload.objectId ?? requirement.objectId ?? -1),
        plaintextHash: String(proof.encryptedOpening.plaintextHash || ""),
        payload,
      });
    }
	    return sanitizeObjectBoundOpening({
	      ...opening,
	      owner: Number(opening.owner),
	      slot: Number(opening.slot),
	      card: String(opening.card),
      objectId: Number(requirement.objectId ?? opening.objectId ?? proof.objectId),
      timing: "private",
      ...(proof.position != null && opening.position == null
        ? { position: Number(proof.position) }
        : {}),
	      ...(proof.positionCommitment && !opening.positionCommitment
	        ? { positionCommitment: String(proof.positionCommitment) }
	        : {}),
	    });
	  }

  async function privateOpeningFromProof(requirement, audit = {}, options = {}) {
    const proof = (audit.privateViewProofs || []).find((entry) =>
      String(entry?.type || "") === "encrypted_private_opening"
      && (
        String(entry?.requirementId || "") === String(requirement.id || "")
        || (
          Number(entry?.owner) === Number(requirement.owner)
          && Number(entry?.viewer) === Number(requirement.viewer)
          && Number(entry?.objectId) === Number(requirement.objectId)
        )
      )
    );
    return privateOpeningFromEncryptedProof(proof, requirement, {
      ...options,
      seq: options.seq ?? audit.seq,
    });
  }

  async function revealPrivateAuditProofsForLocalViewer(audit = {}, options = {}) {
    const localSeat = resolveLocalCryptoPlayerIndex();
    const openings = [];
    const seen = new Set();
    for (const proof of audit.privateViewProofs || []) {
      if (String(proof?.type || "") !== "encrypted_private_opening") continue;
      if (Number(proof.viewer) !== Number(localSeat)) continue;
      let opening = await privateOpeningFromEncryptedProof(proof, {
        owner: proof.owner,
        viewer: proof.viewer,
        objectId: proof.objectId,
        seq: audit.seq,
      }, {
        seq: options.seq ?? audit.seq,
        persistDisclosure: options.persistDisclosure,
      });
      if (!opening) continue;
      opening = await sanitizeObjectBoundOpening(opening);
      opening = await ensureZiffleOpeningProof(opening, options);
      opening = await sanitizeObjectBoundOpening(opening);
      const key = [
        Number(opening.owner),
        Number(opening.slot),
        Number(opening.objectId ?? proof.objectId ?? -1),
      ].join(":");
      if (seen.has(key)) continue;
      seen.add(key);
      openings.push(opening);
    }
    if (openings.length > 0) {
      await revealAuditOpenings(openings, options);
    }
  }

  async function batchedOwnerPrivateZiffleOpeningsForLocalViewer(requirements = [], options = {}) {
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.ziffleRevealCards !== "function") {
      return { openings: [], handledRequirements: new Set() };
    }
    const localSeat = resolveLocalCryptoPlayerIndex();
    const manifest = privateDeckManifestForOwner(localSeat);
    if (!manifest) {
      return { openings: [], handledRequirements: new Set() };
    }

    const groups = new Map();
    const handledRequirements = new Set();
    for (const requirement of requirements || []) {
      if (String(requirement?.type || "") !== "private_open") continue;
      if (Number(requirement.viewer) !== Number(localSeat)) continue;
      if (Number(requirement.owner) !== Number(localSeat)) continue;
      const positionCommitment = String(requirement.commitment || "");
      const deckHash = ziffleDeckHashFromCommitment(positionCommitment);
      if (!deckHash) {
        continue;
      }
	      const position = zifflePositionFromCommitment(positionCommitment) ?? Number(requirement.slot);
	      if (!Number.isSafeInteger(position) || position < 0) continue;
	      const objectOrderedPosition = zifflePositionForObjectId(
	        localSeat,
	        requirement?.objectId ?? requirement?.object_id,
	        { commitment: positionCommitment }
	      );
	      const ziffleContext = objectOrderedPosition?.ziffleContext || "";
	      const ceremony = ziffleCeremonyForOwner(localSeat, {
	        commitment: positionCommitment,
	        context: ziffleContext,
	      });
      if (!ceremony) {
        continue;
      }
      const key = [
        Number(localSeat),
        String(ceremony.context || ""),
        String(ceremony.deckHash || deckHash),
      ].join(":");
      if (!groups.has(key)) {
        groups.set(key, {
          ceremony,
          entries: [],
        });
      }
      groups.get(key).entries.push({
        requirement,
        position,
        positionCommitment,
      });
      handledRequirements.add(requirement);
    }

	    const openings = [];
	    const seen = new Set();
	    for (const { ceremony, entries } of groups.values()) {
	      if (ziffleCeremonyHasObjectOrder(ceremony)) {
	        for (const entry of entries) {
	          const objectId = Number(entry.requirement.objectId ?? entry.requirement.object_id);
	          const {
	            resolvedRevealSlot,
	            shuffleOriginalSlot,
	          } = await resolveCommittedSlotForZifflePosition({
	            owner: localSeat,
	            ceremony,
	            position: entry.position,
	            card: entry.requirement?.card || "",
	            objectId,
	            manifest,
	            options,
	          });
	          if (!resolvedRevealSlot) {
              handledRequirements.delete(entry.requirement);
              continue;
	          }
	          const builtOpening = await buildOpeningFromResolvedCommittedSlot({
	            manifest,
	            resolvedRevealSlot,
	            fallbackObjectId: Number.isSafeInteger(objectId) && objectId >= 0 ? objectId : null,
	            position: entry.position,
	            positionCommitment: entry.positionCommitment,
	            ceremony,
	            timing: "post",
	          });
	          let openingWithPosition = await sanitizeObjectBoundOpening(builtOpening.openingWithPosition);
	          openingWithPosition = await ensureZiffleOpeningProof(openingWithPosition, options);
	          openingWithPosition = await sanitizeObjectBoundOpening(openingWithPosition);
	          const originalSlot = builtOpening.originalSlot;
	          const key = [
	            Number(openingWithPosition.owner),
	            Number(openingWithPosition.slot),
	            Number.isSafeInteger(objectId) ? objectId : -1,
	          ].join(":");
	          if (seen.has(key)) continue;
	          rememberLocalRevealedOpening(openingWithPosition, {
		            objectId: openingWithPosition.objectId,
		            position: openingWithPosition.position,
		            positionCommitment: openingWithPosition.positionCommitment,
		            ziffleContext: openingWithPosition.ziffleContext,
		          });
	          rememberZiffleOpeningPosition(localSeat, originalSlot, entry.position);
	          openings.push(openingWithPosition);
	          seen.add(key);
	        }
	        continue;
      }
	      const positions = [...new Set(entries.map((entry) => entry.position))];
	      const tokens = await collectZiffleRevealTokensBatch(ceremony, positions, {
        ...options,
        requirements,
      });
      const reveals = await currentGame.ziffleRevealCards({
        deckCount: Number(ceremony.deckCount),
        context: String(ceremony.context || ""),
        keyContext: ziffleKeyContextForCeremony(ceremony),
        keys: cloneMultiplayerPayload(ceremony.keys || []),
        steps: cloneMultiplayerPayload(ceremony.steps || []),
        cardPositions: positions,
        tokens,
      });
      const revealByPosition = new Map(
        (Array.isArray(reveals) ? reveals : []).map((reveal) => [
          Number(reveal.cardPosition),
          Number(reveal.originalSlot),
        ])
      );
      for (const entry of entries) {
	        const shuffleOriginalSlot = revealByPosition.get(Number(entry.position));
	        if (!Number.isSafeInteger(shuffleOriginalSlot) || shuffleOriginalSlot < 0) {
	          throw new Error(`Missing ziffle reveal for position ${Number(entry.position)}`);
	        }
	        let resolvedRevealSlot = await resolveCommittedZiffleRevealSlot({
	          owner: localSeat,
	          ceremony,
	          shuffleOriginalSlot,
	          shuffleOriginalSlotIsVerified: true,
	          position: entry.position,
		          card: entry.requirement?.card || "",
		          objectId: entry.requirement?.objectId,
		          manifest,
            options,
		        });
	        if (!resolvedRevealSlot && entry.requirement?.card) {
	          resolvedRevealSlot = await resolveCommittedZiffleRevealSlot({
	            owner: localSeat,
	            ceremony,
	            shuffleOriginalSlot,
	            shuffleOriginalSlotIsVerified: true,
	            position: entry.position,
		            card: "",
		            objectId: entry.requirement?.objectId,
		            manifest,
              options,
		          });
	        }
	        if (!resolvedRevealSlot) {
	          const beforeOrder = normalizeShuffleOrder(ceremony.beforeOrder ?? ceremony.before_order);
	          const afterOrder = normalizeShuffleOrder(ceremony.afterOrder ?? ceremony.after_order);
	          const interestingIds = [
	            entry.requirement?.objectId ?? entry.requirement?.object_id,
	            beforeOrder[Number(shuffleOriginalSlot)],
	            afterOrder[Number(entry.position)],
	          ].map((id) => Number(id)).filter((id, index, list) =>
	            Number.isSafeInteger(id) && id >= 0 && list.indexOf(id) === index
	          );
	          let candidateDebug = [];
	          try {
	            const checkpoint = await currentGame.exportSyncCheckpoint?.();
	            const objectsById = new Map((checkpoint?.objects || []).map((object) => [
	              Number(object.id),
	              object,
	            ]));
	            candidateDebug = interestingIds.map((id) => {
	              const object = objectsById.get(id) || {};
	              const hidden = object.hiddenCard || object.hidden_card || {};
	              return {
	                id,
	                name: object.name || object.identity?.name || null,
	                zone: object.zone || null,
	                stableId: object.stableId ?? object.stable_id ?? null,
	                hiddenOwner: hidden.owner ?? null,
	                hiddenSlot: hidden.slot ?? null,
	                hiddenCommitment: String(hidden.commitment || "").slice(0, 32),
	                publicSlot: hidden.publicSlot ?? hidden.public_slot ?? null,
	                publicCommitment: String(hidden.publicCommitment || hidden.public_commitment || "").slice(0, 32),
	              };
	            });
	          } catch {
	            candidateDebug = [];
	          }
	          throw new Error(
	            `Ziffle opening could not resolve committed slot `
	            + `(owner ${Number(localSeat) + 1}, position ${Number(entry.position)}, `
	            + `shuffle slot ${shuffleOriginalSlot}, card ${String(entry.requirement?.card || "")}, `
	            + `ids ${JSON.stringify(interestingIds)}, candidates ${JSON.stringify(candidateDebug)})`
	          );
	        }
	        const originalSlot = Number(resolvedRevealSlot.slot);
	        const secret = (manifest.slotSecrets || []).find(
	          (candidate) => Number(candidate.slot) === Number(originalSlot)
	        );
        if (!secret) {
          throw new Error(`Missing private deck opening for ziffle slot ${Number(originalSlot)}`);
        }
        const opening = await buildDeckSlotOpening({
          manifest,
          slot: originalSlot,
          card: secret.card,
        });
        const objectId = Number(entry.requirement.objectId);
        const key = [
          Number(opening.owner),
          Number(opening.slot),
          Number.isSafeInteger(objectId) ? objectId : -1,
        ].join(":");
        if (seen.has(key)) continue;
        let openingWithPosition = {
	          ...opening,
	          ...(Number.isSafeInteger(objectId) ? { objectId } : {}),
	          ...(resolvedRevealSlot?.shuffleObjectId != null || resolvedRevealSlot?.objectId != null
	            ? { shuffleObjectId: Number(resolvedRevealSlot.shuffleObjectId ?? resolvedRevealSlot.objectId) }
	            : {}),
		          timing: "post",
		          position: Number(entry.position),
		          positionCommitment: entry.positionCommitment,
		          ziffleContext: ziffleContextFromCeremony(ceremony),
	        };
        if (!ziffleCeremonyHasObjectOrder(ceremony)) {
          openingWithPosition.ziffleReveal = buildZiffleOpeningProof({
            opening: openingWithPosition,
		          ceremony,
		          position: Number(entry.position),
		          originalSlot,
		          shuffleOriginalSlot,
		          positionCommitment: entry.positionCommitment,
		          tokens: ziffleTokensForPosition(tokens, entry.position),
		          compact: true,
		        });
        }
        openingWithPosition = await sanitizeObjectBoundOpening(openingWithPosition);
        openingWithPosition = await ensureZiffleOpeningProof(openingWithPosition, options);
        openingWithPosition = await sanitizeObjectBoundOpening(openingWithPosition);
        rememberLocalRevealedOpening(openingWithPosition, {
	          objectId: openingWithPosition.objectId,
	          position: openingWithPosition.position,
	          positionCommitment: openingWithPosition.positionCommitment,
	          ziffleContext: openingWithPosition.ziffleContext,
	        });
        rememberZiffleOpeningPosition(localSeat, originalSlot, entry.position);
        openings.push(openingWithPosition);
        seen.add(key);
      }
    }
    return { openings, handledRequirements };
  }

  async function privateOpeningsForLocalViewer(requirements = [], audit = {}, options = {}) {
    const localSeat = resolveLocalCryptoPlayerIndex();
    const {
      openings,
      handledRequirements,
    } = await batchedOwnerPrivateZiffleOpeningsForLocalViewer(requirements, options);
    const seen = new Set();
    for (const opening of openings) {
      seen.add([
        Number(opening.owner),
        Number(opening.slot),
        Number(opening.objectId ?? -1),
      ].join(":"));
    }
    for (const requirement of requirements || []) {
      if (handledRequirements.has(requirement)) continue;
      if (String(requirement?.type || "") !== "private_open") continue;
      if (Number(requirement.viewer) !== Number(localSeat)) continue;
      let opening = await privateOpeningFromProof(requirement, audit, options);
      if (!opening && Number(requirement.owner) === Number(localSeat)) {
        opening = (await buildLocalOpeningFromRequirement(requirement, null, options)).opening;
      }
      if (!opening) continue;
      opening = await sanitizeObjectBoundOpening(opening);
      opening = await ensureZiffleOpeningProof(opening, options);
      opening = await sanitizeObjectBoundOpening(opening);
      const key = [
        Number(opening.owner),
        Number(opening.slot),
        Number(opening.objectId ?? requirement.objectId ?? -1),
      ].join(":");
      if (seen.has(key)) continue;
      seen.add(key);
      openings.push(opening);
    }
    return openings;
  }

	  function hiddenPositionBatchRevealFromOpening(opening) {
	    if (!opening || opening.owner == null || opening.slot == null || !opening.card) return null;
	    const owner = Number(opening.owner);
	    const originalSlot = Number(opening.slot);
	    const cardName = String(opening.card || "").trim();
	    const positionCommitment = String(
	      opening.positionCommitment || opening.position_commitment || ""
	    );
	    const position = Number(
	      zifflePositionFromCommitment(positionCommitment) ?? opening.position
	    );
    if (
      !Number.isSafeInteger(owner)
      || owner < 0
      || !Number.isSafeInteger(position)
      || position < 0
      || position > 65535
      || !Number.isSafeInteger(originalSlot)
      || originalSlot < 0
      || originalSlot > 65535
      || !cardName
      || !ziffleDeckHashFromCommitment(positionCommitment)
    ) {
      return null;
    }
    const commitment = String(opening.commitment || "");
    return {
      owner,
      position,
      originalSlot,
      cardName,
      positionCommitment,
      ...(commitment ? { commitment } : {}),
      recomputeDecision: false,
    };
  }

  async function revealPrivateOpeningsForInjection(privateOpenings = [], options = {}) {
    const currentGame = gameRef.current;
    const openingList = Array.isArray(privateOpenings) ? privateOpenings : [];
    if (openingList.length === 0) return;
    if (typeof currentGame?.revealHiddenPositions !== "function") {
      await revealAuditOpenings(openingList, options);
      return;
    }
    const batchItems = [];
    const fallbackOpenings = [];
    for (const opening of openingList) {
      const reveal = hiddenPositionBatchRevealFromOpening(opening);
      if (reveal) {
        batchItems.push({ opening, reveal });
      } else {
        fallbackOpenings.push(opening);
      }
    }
    if (batchItems.length === 0) {
      await revealAuditOpenings(openingList, options);
      return;
    }
    try {
      await currentGame.revealHiddenPositions({
        reveals: batchItems.map((entry) => entry.reveal),
        recomputeDecision: false,
      });
      for (const { opening } of batchItems) {
        rememberLocalRevealedOpening(opening, {
          objectId: opening.objectId,
          position: opening.position,
          positionCommitment: opening.positionCommitment,
          ziffleContext: opening.ziffleContext,
        });
        rememberZiffleOpeningPosition(opening.owner, opening.slot, opening.position);
      }
    } catch {
      await revealAuditOpenings(openingList, options);
      return;
    }
    if (fallbackOpenings.length > 0) {
      await revealAuditOpenings(fallbackOpenings, options);
    }
  }

  async function injectCryptoMaterialForRequirements(requirements = [], audit = {}, options = {}) {
    const currentGame = gameRef.current;
    if (!currentGame) return;
    const seeds = [];
    const libraryShuffles = [];
    for (const requirement of requirements || []) {
      const type = String(requirement?.type || "");
      if (type === "verifiable_shuffle") {
        const proof = (audit.shuffleProofs || []).find((entry) =>
          shuffleProofMatchesRequirement(entry, requirement)
        );
        if (proof?.deckHash) seeds.push(String(proof.deckHash));
        const beforeOrder = normalizeShuffleOrder(proof?.beforeOrder ?? proof?.before_order);
        const afterOrder = normalizeShuffleOrder(proof?.afterOrder ?? proof?.after_order);
        if (beforeOrder.length > 0 && beforeOrder.length === afterOrder.length) {
          libraryShuffles.push({
            owner: Number(proof.owner ?? requirement.owner),
            beforeOrder,
            afterOrder,
          });
        }
      } else if (type === "fair_random") {
        const reveal = (audit.rngReveals || []).find(
          (entry) => String(entry?.requirementId || "") === String(requirement.id || "")
        );
        if (reveal?.combinedSeedHex) seeds.push(String(reveal.combinedSeedHex));
      }
    }
    if (
      (seeds.length > 0 || libraryShuffles.length > 0)
      && typeof currentGame.injectTranscriptRandomSeeds === "function"
    ) {
      await currentGame.injectTranscriptRandomSeeds({
        seeds,
        libraryShuffles,
      });
    }
    const privateOpenings = await privateOpeningsForLocalViewer(requirements, audit, options);
    if (privateOpenings.length > 0) {
      await revealPrivateOpeningsForInjection(privateOpenings, options);
    }
  }

	  const buildLocalPrivateViewProofsForRequirements = useCallback(async (requirements = [], options = {}) => {
	    const currentGame = gameRef.current;
	    const proofs = [];
	    const localSeat = resolveLocalCryptoPlayerIndex();
	    // The viewer (not the deck owner) is responsible for non-owner private
	    // views: it aggregates the other players' reveal tokens locally, so the
	    // owner never decrypts a card it is not entitled to see.
	    const privateOpenRequirements = (requirements || []).filter((requirement) =>
	      String(requirement?.type || "") === "private_open"
	      && !isOwnerPrivateViewRequirement(requirement)
	      && cryptoMaterialResponsibleSeat(requirement) === Number(localSeat)
	    );

	    let liveState = options.liveState || options.uiState || stateRef.current || null;
	    if (!options.liveState && !options.uiState && currentGame && typeof currentGame.uiState === "function") {
	      try {
	        liveState = await currentGame.uiState();
	      } catch {
	        // Use the last React state snapshot if the live engine snapshot is unavailable.
	      }
	    }
	    const viewedCards = liveState?.viewed_cards || liveState?.active_viewed_cards || null;
	    const existingPrivateOpenKeys = new Set(
	      privateOpenRequirements
	        .map((requirement) => {
	          const objectId = Number(requirement?.objectId ?? requirement?.object_id);
	          return Number.isSafeInteger(objectId) && objectId > 0
	            ? `${Number(requirement.owner)}:${Number(requirement.viewer)}:${objectId}`
	            : null;
	        })
	        .filter(Boolean)
	    );
	    const syntheticPrivateOpenRequirements = [];
	    for (const requirement of requirements || []) {
	      if (String(requirement?.type || "") !== "private_view_window") continue;
	      if (isOwnerPrivateViewRequirement(requirement)) continue;
	      if (cryptoMaterialResponsibleSeat(requirement) !== Number(localSeat)) continue;
	      if (!viewedCards || String(viewedCards.visibility || "") !== "private") continue;
	      if (Number(viewedCards.viewer) !== Number(requirement.viewer)) continue;
	      if (Number(viewedCards.subject) !== Number(requirement.owner)) continue;
	      if (String(viewedCards.zone || "").toLowerCase() !== String(requirement.zone || "").toLowerCase()) {
	        continue;
	      }
	      const count = Math.max(0, Number(requirement.count || 0));
	      const cards = Array.isArray(viewedCards.cards) ? viewedCards.cards : [];
	      const cardIds = Array.isArray(viewedCards.card_ids) ? viewedCards.card_ids : [];
	      for (let index = 0; index < cards.length && syntheticPrivateOpenRequirements.length < count; index += 1) {
	        const card = cards[index] || {};
	        const objectId = Number(cardIds[index] ?? card.id ?? card.objectId ?? card.object_id);
	        if (!Number.isSafeInteger(objectId) || objectId <= 0) continue;
	        const key = `${Number(requirement.owner)}:${Number(requirement.viewer)}:${objectId}`;
	        if (existingPrivateOpenKeys.has(key)) continue;
	        let exported = null;
	        if (currentGame && typeof currentGame.exportHiddenCardOpening === "function") {
	          try {
	            exported = await currentGame.exportHiddenCardOpening(wasmObjectIdArg(objectId));
	          } catch {
	            exported = null;
	          }
	        }
	        const metadata = exported ? null : await currentHiddenCardMetadataForObject(objectId);
	        const slot = Number(exported?.slot ?? metadata?.slot);
	        const commitment = String(exported?.commitment || metadata?.commitment || "");
	        if (!Number.isSafeInteger(slot) || slot < 0 || !commitment) continue;
	        syntheticPrivateOpenRequirements.push({
	          ...cloneMultiplayerPayload(requirement),
	          type: "private_open",
	          id: `${String(requirement.id || "private_view_window")}:object:${objectId}`,
	          objectId,
	          slot,
	          commitment,
	          publicSlot: exported?.publicSlot ?? exported?.public_slot ?? metadata?.publicSlot ?? null,
	          publicCommitment:
	            exported?.publicCommitment
	            || exported?.public_commitment
	            || metadata?.publicCommitment
	            || "",
	          card: String(exported?.card || card.name || ""),
	        });
	        existingPrivateOpenKeys.add(key);
	      }
	    }
	    const allPrivateOpenRequirements = [
	      ...privateOpenRequirements,
	      ...syntheticPrivateOpenRequirements,
	    ];

	    const privateOpeningProofs = (await Promise.all(allPrivateOpenRequirements.map(async (requirement) => {
	      const viewer = Number(requirement.viewer);
	      const recipientPublicKey = auditEncryptionPublicKeyForPlayer(viewer);
	      if (!recipientPublicKey) {
	        throw new Error(`Player ${viewer + 1} is missing a private-view encryption key`);
	      }
	      let exported = null;
	      if (currentGame && typeof currentGame.exportHiddenCardOpening === "function" && requirement.objectId != null) {
	        try {
	          exported = await currentGame.exportHiddenCardOpening(wasmObjectIdArg(requirement.objectId));
	        } catch {
	          exported = null;
	        }
	      }
	      const {
	        opening,
	        owner,
	        position,
	        positionCommitment,
	      } = await buildLocalOpeningFromRequirement(requirement, exported, options);
	      const privateOpening = await sanitizeObjectBoundOpening({
	        ...opening,
	        objectId: Number(requirement.objectId),
	        timing: "private",
	      });
	      const openingPayload = {
	        type: "private_view_opening",
	        matchId: currentAuditMatchId(),
	        requirementId: String(requirement.id || ""),
	        owner,
	        viewer,
	        zone: String(requirement.zone || ""),
	        objectId: Number(requirement.objectId),
	        opening: privateOpening,
	      };
	      const encryptedOpening = await encryptPrivateAuditPayload({
	        recipientPublicKey,
	        payload: openingPayload,
	      });
	      rememberPrivateViewDisclosure({
	        type: "private_view_opening_disclosure",
	        matchId: currentAuditMatchId(),
	        seq: Number(options.seq ?? 0),
	        requirementId: String(requirement.id || ""),
	        owner,
	        viewer,
	        zone: String(requirement.zone || ""),
	        objectId: Number(requirement.objectId),
	        plaintextHash: String(encryptedOpening.plaintextHash || ""),
	        payload: openingPayload,
	      });
	      const proof = {
	        type: "encrypted_private_opening",
	        requirementId: String(requirement.id || ""),
	        owner,
	        viewer,
	        zone: String(requirement.zone || ""),
	        objectId: Number(requirement.objectId),
	        slot: Number(opening.slot),
	        commitment: opening.commitment,
	        disclosurePolicy: "postgame_or_dispute",
	        encryptedOpening,
	      };
	      if (position != null) {
	        proof.position = position;
	        proof.positionCommitment = positionCommitment;
	      }
	      return proof;
	    }))).filter(Boolean);
	    proofs.push(...privateOpeningProofs);
	    for (const requirement of requirements || []) {
	      if (String(requirement?.type || "") !== "private_view_window") continue;
	      if (isOwnerPrivateViewRequirement(requirement)) continue;
	      if (cryptoMaterialResponsibleSeat(requirement) !== Number(localSeat)) continue;
	      const openingHashes = privateOpeningProofs
	        .filter((entry) =>
	          Number(entry.owner) === Number(requirement.owner)
	          && Number(entry.viewer) === Number(requirement.viewer)
	          && String(entry.zone || "") === String(requirement.zone || "")
	        )
	        .map((entry) => entry.encryptedOpening.plaintextHash);
	      const proof = {
	        type: "encrypted_private_view",
	        requirementId: String(requirement.id || ""),
	        owner: Number(requirement.owner),
	        viewer: Number(requirement.viewer),
	        zone: String(requirement.zone || ""),
	        count: Number(requirement.count || 0),
	        reason: String(requirement.reason || ""),
	        openingHashes,
	        disclosurePolicy: "postgame_or_dispute",
	      };
	      proof.materialHash = await sha256Hex(canonicalMultiplayerPayload(proof));
	      proofs.push(proof);
	    }
	    return proofs;
	  }, [
	    auditEncryptionPublicKeyForPlayer,
	    buildLocalOpeningFromRequirement,
	    currentAuditMatchId,
	    currentHiddenCardMetadataForObject,
	    sanitizeObjectBoundOpening,
	    rememberPrivateViewDisclosure,
	    resolveLocalCryptoPlayerIndex,
	  ]);

	  const buildLocalCryptoMaterialForRequirements = useCallback(async (requirements = [], options = {}) => {
	    const openings = mergeAuditOpenings(
	      await buildLocalOpeningsForCommand({}, requirements, options),
	      await buildLocalRequirementOpeningsForRequirements(requirements, options)
	    );
	    const privateViewProofs = await buildLocalPrivateViewProofsForRequirements(requirements, options);
	    return { openings, privateViewProofs };
	  }, [
	    buildLocalOpeningsForCommand,
	    buildLocalPrivateViewProofsForRequirements,
	    buildLocalRequirementOpeningsForRequirements,
	  ]);

  const derivePostApplyCryptoRequirementsForRequest = useCallback(async ({
    command,
    seq,
    actorIndex,
    liveState,
  }) => {
    const currentGame = gameRef.current;
    if (
      !currentGame
      || typeof currentGame.exportSyncCheckpoint !== "function"
      || typeof currentGame.importSyncCheckpoint !== "function"
      || typeof applySyncedCommand !== "function"
    ) {
      throw new Error("Cannot authorize post-apply hidden-card material without sandbox replay");
    }
    const checkpoint = await currentGame.exportSyncCheckpoint();
    const previousState = cloneMultiplayerPayload(stateRef.current);
    const localPlayer = resolveLocalPlayerIndex(multiplayerRef.current);
    try {
      const appliedState = await applySyncedCommand(command, "", {
        actorIndex,
        sequence: seq,
        publishState: false,
        validationOnly: true,
      });
      return filterCryptoRequirementsForCommand(
        command,
        liveState,
        freshCryptoRequirementsForSequence(
          seq,
          cryptoRequirementsFromState(appliedState)
        )
      );
    } finally {
      await currentGame.importSyncCheckpoint(
        checkpoint,
        localPlayer ?? multiplayerRef.current.localPlayerIndex ?? 0
      );
      const restoredState = typeof currentGame.uiState === "function"
        ? await currentGame.uiState()
        : previousState;
      stateRef.current = restoredState;
      setState(restoredState);
    }
  }, [
    applySyncedCommand,
    freshCryptoRequirementsForSequence,
    setState,
  ]);

  const authorizedCryptoMaterialRequirementsForRequest = useCallback(async (conn, message) => {
    const session = multiplayerRef.current;
    if (!session.matchStarted) {
      throw new Error("Cryptographic material request received before match start");
    }
    if (String(message?.matchId || "") !== currentAuditMatchId()) {
      throw new Error("Cryptographic material request belongs to a different match");
    }

    const requester = playerIndexForPeerId(conn?.peer);
    if (requester == null) {
      throw new Error("Cryptographic material requester is not a match player");
    }
    if (normalizePlayerIndex(message?.requesterIndex) !== requester) {
      throw new Error("Cryptographic material requester index does not match the peer");
    }

    const actorIndex = normalizePlayerIndex(message?.actorIndex);
    if (actorIndex == null || actorIndex !== requester) {
      throw new Error("Cryptographic material requester is not the acting player");
    }

    const seq = Number(message?.seq);
    const expectedSeq = Number(session.lastAppliedSequence || 0) + 1;
    if (!Number.isSafeInteger(seq) || seq !== expectedSeq) {
      throw new Error("Cryptographic material request has an invalid action sequence");
    }
    if (String(message?.prevStateHash || "") !== String(auditStateHashRef.current || INITIAL_AUDIT_STATE_HASH)) {
      throw new Error("Cryptographic material request is not based on the local transcript head");
    }
    if (!String(message?.publicCheckpointHash || "")) {
      throw new Error("Cryptographic material request is missing the public checkpoint hash");
    }

    const command = message?.command;
    if (!command || typeof command !== "object") {
      throw new Error("Cryptographic material request is missing the signed command preview");
    }

    const liveState = gameRef.current && typeof gameRef.current.uiState === "function"
      ? await gameRef.current.uiState()
      : stateRef.current;
    const decision = liveState?.decision || null;
    if (
      decision?.player !== null
      && decision?.player !== undefined
      && Number(decision.player) !== Number(actorIndex)
    ) {
      throw new Error("Cryptographic material requester is not the active decision player");
    }
    if (!isDecisionCommandCompatible(decision, command)) {
      throw new Error("Cryptographic material request command is not available locally");
    }
    await verifyCurrentPublicCheckpointHash(
      message.publicCheckpointHash,
      "Cryptographic material request public checkpoint does not match local state"
    );

	    const localSeat = resolveLocalCryptoPlayerIndex();
	    const previewedRequirements = filterCryptoRequirementsForCommand(
	      command,
	      liveState,
	      freshCryptoRequirementsForSequence(
	        seq,
	        await previewRequirementsForCommand(command)
	      )
	    );
	    const locallyKnownRequestedPublicOpenRequirements = (
	      Array.isArray(message.requirements) ? message.requirements : []
	    ).filter((requirement) =>
	      String(requirement?.type || "") === "public_open"
	      && Number(requirement.owner) === Number(localSeat)
	      && localRevealedOpeningForRequirement(requirement)
	    );
	    try {
	      const authorizedRequirements = authorizeCryptoMaterialRequestRequirements({
	        localSeat,
	        requestedRequirements: message.requirements,
	        previewedRequirements: [
	          ...previewedRequirements,
	          ...locallyKnownRequestedPublicOpenRequirements,
	        ],
	      });
      const actionIntent = await verifySignedActionIntent(message.actionIntent, {
        matchId: currentAuditMatchId(),
        seq,
        actorIndex,
        prevStateHash: message.prevStateHash,
        preActionPublicCheckpointHash: message.publicCheckpointHash,
        command,
      });
      return { requirements: authorizedRequirements, actionIntent };
    } catch (err) {
      const postApplyRequirements = await derivePostApplyCryptoRequirementsForRequest({
        command,
        seq,
        actorIndex,
        liveState,
      });
      if (postApplyRequirements.length === 0) {
        throw err;
      }
	      const authorizedRequirements = authorizeCryptoMaterialRequestRequirements({
	        localSeat,
	        requestedRequirements: message.requirements,
	        previewedRequirements: [
	          ...previewedRequirements,
	          ...postApplyRequirements,
	          ...locallyKnownRequestedPublicOpenRequirements,
	        ],
	      });
      const actionIntent = await verifySignedActionIntent(message.actionIntent, {
        matchId: currentAuditMatchId(),
        seq,
        actorIndex,
        prevStateHash: message.prevStateHash,
        preActionPublicCheckpointHash: message.publicCheckpointHash,
        command,
      });
      return { requirements: authorizedRequirements, actionIntent };
    }
		  }, [
			    currentAuditMatchId,
	    derivePostApplyCryptoRequirementsForRequest,
		    freshCryptoRequirementsForSequence,
	    localRevealedOpeningForRequirement,
		    resolveLocalCryptoPlayerIndex,
    verifyCurrentPublicCheckpointHash,
	  ]);

  const answerCryptoMaterialRequest = useCallback(async (conn, message) => {
    const requestPerf = {
      request_id: String(message?.requestId || ""),
      requester: message?.requesterIndex == null ? null : Number(message.requesterIndex),
      local_seat: resolveLocalCryptoPlayerIndex(),
      command: summarizePeerCommand(message?.command),
      requested_requirements: summarizeCryptoRequirementsForPerf(message?.requirements || []),
      request_bytes: payloadSizeBytes(message),
    };
    recordPeerSyncPerf("crypto_material_request:received", requestPerf);
    try {
      const { requirements, actionIntent } = await timePeerSyncPhase(
        "crypto_material_request:authorize",
        requestPerf,
        () => authorizedCryptoMaterialRequirementsForRequest(conn, message)
      );
      const authorizedPerf = {
        ...requestPerf,
        authorized_requirements: summarizeCryptoRequirementsForPerf(requirements),
      };
      const requestPayload = cloneMultiplayerPayload(message);
      if (requirements.length > 0) {
        await timePeerSyncPhase("crypto_material_request:remember_intent", authorizedPerf, async () =>
          rememberPendingActionIntent(actionIntent, {
          requestType: "crypto_material_request",
          requestId: String(message.requestId || ""),
          requestPayload,
          requestPayloadHash: await sha256Hex(canonicalMultiplayerPayload(requestPayload)),
          responseTimeoutMs: PROTOCOL_RESPONSE_TIMEOUT_MS,
          requestedAtMs: Date.now(),
          })
        );
      }
      setStatus("Generating hidden-card opening payloads for peer action");
      const material = await timePeerSyncPhase(
        "crypto_material_request:build_local_material",
        authorizedPerf,
        () => buildLocalCryptoMaterialForRequirements(requirements, {
          cryptoMaterialRequestId: message.requestId,
          command: message.command || null,
          seq: message.seq,
          actorIndex: message.actorIndex,
          requesterIndex: message.requesterIndex,
          actionIntent,
        })
      );
      recordPeerSyncPerf("crypto_material_request:send_response", {
        ...authorizedPerf,
        material: summarizeCryptoMaterialForPerf(material),
      });
      safeSend(conn, {
        type: "crypto_material_response",
        protocolVersion: PROTOCOL_VERSION,
        requestId: message.requestId,
        openings: material.openings,
        privateViewProofs: material.privateViewProofs,
      });
    } catch (err) {
      recordPeerSyncPerf("crypto_material_request:error_response", {
        ...requestPerf,
        error: toErrorMessage(err),
      });
      safeSend(conn, {
        type: "crypto_material_response",
        protocolVersion: PROTOCOL_VERSION,
        requestId: message.requestId,
        error: toErrorMessage(err),
      });
    }
  }, [
    authorizedCryptoMaterialRequirementsForRequest,
    buildLocalCryptoMaterialForRequirements,
  ]);

  const collectRemoteCryptoMaterialForRequirements = useCallback(async (requirements = [], options = {}) => {
	    const localSeat = resolveLocalCryptoPlayerIndex();
    const materialByOwner = new Map();
    for (const requirement of requirements || []) {
      const type = String(requirement?.type || "");
      if (!["public_open", "private_open", "private_view_window"].includes(type)) continue;
      if (isOwnerPrivateViewRequirement(requirement)) continue;
      // Private views addressed to another player are produced by the viewer
      // (mental-poker flow); everything else by the deck owner.
      const seat = cryptoMaterialResponsibleSeat(requirement);
      if (!Number.isInteger(seat) || seat === Number(localSeat)) continue;
      if (!materialByOwner.has(seat)) materialByOwner.set(seat, []);
      materialByOwner.get(seat).push(requirement);
    }
    const command = options.command || null;
    const seq = options.seq;
    const actorIndex = options.actorIndex;
    const requestPreview = Boolean(options.requestPreview);
    const players = reindexPlayers(matchStartPayloadRef.current?.players || multiplayerRef.current.players || []);
    if (command && requestPreview) {
      for (const player of players) {
        const owner = Number(player?.index);
        if (!Number.isInteger(owner) || owner === Number(localSeat)) continue;
        if (!materialByOwner.has(owner)) materialByOwner.set(owner, []);
      }
    }
    if (materialByOwner.size === 0) {
      recordPeerSyncPerf("collect_remote_crypto_material:skip", {
        command: summarizePeerCommand(options.command),
        requirements: summarizeCryptoRequirementsForPerf(requirements),
        reason: "no_remote_material_required",
      });
      return { openings: [], privateViewProofs: [] };
    }
    const prevStateHash = String(
      options.prevStateHash ?? auditStateHashRef.current ?? INITIAL_AUDIT_STATE_HASH
    );
    const publicCheckpointHash = String(
      options.publicCheckpointHash || await currentPublicAuditCheckpointHash()
    );
    const actionIntent = command
      ? (
          options.actionIntent
          || await signActionIntentForCommand({
            seq,
            actorIndex,
            command,
            prevStateHash,
            preActionPublicCheckpointHash: publicCheckpointHash,
          })
        )
      : null;

    const responses = [];
    recordPeerSyncPerf("collect_remote_crypto_material:start", {
      command: summarizePeerCommand(command),
      seq: seq == null ? null : Number(seq),
      actor: actorIndex == null ? null : Number(actorIndex),
      owners: [...materialByOwner.keys()].map(Number),
      requirements: summarizeCryptoRequirementsForPerf(requirements),
      request_preview: requestPreview,
    });
    // Fan out one request per responsible seat concurrently: a hidden action in
    // a 3-4 player game needs material from multiple peers, and awaiting each in
    // series made the actor's latency scale linearly with player count. The
    // reveal-token collector already uses this pattern; the waiter infra is
    // keyed by requestId so concurrent waiters are safe.
    if (materialByOwner.size > 0) {
      setStatus("Waiting for players to generate hidden-card opening payloads");
    }
    const ownerResponses = await Promise.all(
      [...materialByOwner].map(async ([owner, ownerRequirements]) => {
        const player = players.find((entry) => Number(entry.index) === owner);
        const routePeerId = routePeerIdForPlayer(player);
        if (!routePeerId) {
          throw new Error(`Missing peer route for cryptographic material from player ${owner + 1}`);
        }
        const requestId = makeZiffleRequestId("crypto-material");
        const playerLabel = player.name || `Player ${owner + 1}`;
        const requestedAtMs = Date.now();
        const waiter = waitForCryptoMaterial(requestId, PROTOCOL_RESPONSE_TIMEOUT_MS, {
          peerIndex: owner,
          peerName: playerLabel,
          description:
            `${playerLabel} is generating hidden-card opening payloads for this action.`,
        });
        outboundCryptoMaterialRequestsRef.current.set(requestId, {
          owner,
          targetSeat: owner,
          peerId: routePeerId,
          requirements: cloneMultiplayerPayload(ownerRequirements),
          command: command ? cloneMultiplayerPayload(command) : null,
          seq,
          actorIndex,
          prevStateHash,
          publicCheckpointHash,
          actionIntent: actionIntent ? cloneMultiplayerPayload(actionIntent) : null,
          createdAt: requestedAtMs,
        });
        const requestPayload = {
          type: "crypto_material_request",
          protocolVersion: PROTOCOL_VERSION,
          requestId,
          matchId: currentAuditMatchId(),
          requesterIndex: localSeat,
          ...(seq !== null && seq !== undefined ? { seq: Number(seq) } : {}),
          ...(actorIndex !== null && actorIndex !== undefined ? { actorIndex: Number(actorIndex) } : {}),
          prevStateHash,
          publicCheckpointHash,
          requirements: ownerRequirements,
          ...(command ? { command: cloneMultiplayerPayload(command) } : {}),
          ...(actionIntent ? { actionIntent: cloneMultiplayerPayload(actionIntent) } : {}),
        };
        const responsePerf = {
          owner,
          peer_id: routePeerId,
          request_id: requestId,
          command: summarizePeerCommand(command),
          seq: seq == null ? null : Number(seq),
          actor: actorIndex == null ? null : Number(actorIndex),
          requirements: summarizeCryptoRequirementsForPerf(ownerRequirements),
          request_preview: requestPreview,
          request_bytes: payloadSizeBytes(requestPayload),
        };
        recordPeerSyncPerf("collect_remote_crypto_material:send_request", responsePerf);
        await sendDirectProtocolMessage(routePeerId, requestPayload);
        try {
          const response = await timePeerSyncPhase(
            "collect_remote_crypto_material:wait_response",
            responsePerf,
            () => waitForProtocolResponse(waiter, {
              basisSequence: Number(multiplayerRef.current.lastAppliedSequence || 0),
              targetPlayerIndex: owner,
              targetPeerId: player.peerId,
              requesterIndex: localSeat,
              requestType: requestPayload.type,
              requestId,
              requestPayload,
              responseTimeoutMs: PROTOCOL_RESPONSE_TIMEOUT_MS,
              requestedAtMs,
            })
          );
          recordPeerSyncPerf("collect_remote_crypto_material:received_response", {
            ...responsePerf,
            material: summarizeCryptoMaterialForPerf(response),
          });
          return response;
        } finally {
          outboundCryptoMaterialRequestsRef.current.delete(requestId);
        }
      })
    );
    responses.push(...ownerResponses);

    const merged = {
      openings: mergeAuditOpenings(...responses.map((response) => response.openings || [])),
      privateViewProofs: mergePrivateViewProofs(
        ...responses.map((response) => response.privateViewProofs || [])
      ),
    };
    recordPeerSyncPerf("collect_remote_crypto_material:done", {
      command: summarizePeerCommand(command),
      seq: seq == null ? null : Number(seq),
      actor: actorIndex == null ? null : Number(actorIndex),
      owners: [...materialByOwner.keys()].map(Number),
      material: summarizeCryptoMaterialForPerf(merged),
    });
    return merged;
  }, [
    currentAuditMatchId,
    currentPublicAuditCheckpointHash,
    makeZiffleRequestId,
    resolveLocalCryptoPlayerIndex,
    setStatus,
    waitForCryptoMaterial,
    waitForZiffleRoute,
  ]);

  const resolvePeerResyncWaitersIfIdle = useCallback(() => {
    if (resyncingPeerIdsRef.current.size > 0) return;
    const waiters = resyncWaitersRef.current;
    resyncWaitersRef.current = [];
    for (const resolve of waiters) {
      resolve();
    }
  }, []);

  const finishPeerResync = useCallback(
    (peerId) => {
      if (peerId) {
        resyncingPeerIdsRef.current.delete(peerId);
      }
      resolvePeerResyncWaitersIfIdle();
    },
    [resolvePeerResyncWaitersIfIdle]
  );

  const clearAllPeerResyncs = useCallback(() => {
    resyncingPeerIdsRef.current.clear();
    resolvePeerResyncWaitersIfIdle();
  }, [resolvePeerResyncWaitersIfIdle]);

  const waitForPeerResyncs = useCallback(() => {
    if (resyncingPeerIdsRef.current.size === 0) {
      return Promise.resolve();
    }
    return new Promise((resolve) => {
      resyncWaitersRef.current.push(resolve);
    });
  }, []);

  const teardownPeer = useCallback(() => {
    clearAllConnectionHeartbeats();
    clearAllPeerResyncs();
    clearAllPendingActionIntents();
    ignoredActionIntentKeysRef.current.clear();
    for (const challenge of reconnectChallengesRef.current.values()) {
      if (challenge?.timeoutId) {
        window.clearTimeout(challenge.timeoutId);
      }
    }
    reconnectChallengesRef.current.clear();
    privateViewDisclosuresRef.current.clear();
    awaitingStateResyncRef.current = false;
    relayedActionIdsRef.current.clear();
    applyingSequencedActionsRef.current.clear();
    pendingSequencedActionsRef.current.clear();
    drainingPendingSequencedActionsRef.current = false;
    localZiffleRevealInFlightRef.current = null;

    const hostConn = hostConnectionRef.current;
    hostConnectionRef.current = null;
    if (hostConn) {
      try {
        hostConn.close();
      } catch (err) {
        void err;
      }
    }

    for (const conn of clientConnectionsRef.current.values()) {
      try {
        conn.close();
      } catch (err) {
        void err;
      }
    }
    clientConnectionsRef.current.clear();

    for (const conn of peerConnectionsRef.current.values()) {
      try {
        conn.close();
      } catch (err) {
        void err;
      }
    }
    peerConnectionsRef.current.clear();

    const peer = peerRef.current;
    peerRef.current = null;
    if (peer) {
      try {
        peer.destroy();
      } catch (err) {
        void err;
      }
    }
  }, [clearAllConnectionHeartbeats, clearAllPeerResyncs]);

  const leaveLobby = useCallback(
    (message = "Left lobby", options = {}) => {
      if (options.clearStoredPlayer !== false) {
        clearStoredPlayerIndex(multiplayerRef.current.lobbyId || multiplayerRef.current.hostPeerId);
      }
      teardownPeer();
      matchStartPayloadRef.current = null;
      liveZiffleCeremoniesRef.current.clear();
      localZiffleCeremonyLookupRef.current.clear();
      ziffleRevealTokenCacheRef.current.clear();
      verifiedAuditOpeningsRef.current.clear();
      verifiedShuffleProofsRef.current.clear();
      ziffleOpeningPositionsRef.current.clear();
      localRevealedOpeningsRef.current.clear();
      localDisconnectObservationsRef.current.clear();
      clearAllPendingActionIntents();
      ignoredActionIntentKeysRef.current.clear();
      reconnectChallengesRef.current.clear();
      actionHistoryRef.current = [];
      actionCryptoRequirementsRef.current.clear();
      matchClockObservationExemptSequenceRef.current = 0;
      initialPublicCheckpointHashRef.current = "";
      updateMultiplayer(createEmptyState());
      if (message) {
        setStatus(message, Boolean(options.isError));
      }
    },
    [setStatus, teardownPeer, updateMultiplayer]
  );

  const broadcastToClients = useCallback((payload) => {
    for (const conn of clientConnectionsRef.current.values()) {
      safeSend(conn, payload);
    }
  }, []);

  const broadcastMatchPresence = useCallback((peerId, connected, details = {}) => {
    const player = (multiplayerRef.current.players || []).find((entry) =>
      String(entry?.peerId || "") === String(peerId || "")
    );
    broadcastToClients({
      type: "player_presence",
      protocolVersion: PROTOCOL_VERSION,
      peerId,
      playerIndex: player?.index == null ? undefined : Number(player.index),
      connected: Boolean(connected),
      disconnectedAtMs: details.disconnectedAtMs == null
        ? undefined
        : Number(details.disconnectedAtMs),
      autoForfeitAtMs: details.autoForfeitAtMs == null
        ? undefined
        : Number(details.autoForfeitAtMs),
    });
  }, [broadcastToClients]);

  const sendMatchStartToClients = useCallback((payload) => {
    for (const conn of clientConnectionsRef.current.values()) {
      safeSend(conn, redactedMatchPayloadForPeer(payload, conn.peer));
    }
  }, []);

  const buildHostedResyncPayload = useCallback(() => {
    const session = multiplayerRef.current;
    const basePayload = matchStartPayloadRef.current;
    if (session.role !== "host" || !session.matchStarted || !basePayload) {
      return null;
    }

    const clockRuntime = matchClockRef.current;
    const clockState = stateRef.current;
    const clockPolicy = normalizeMatchClockPolicy(
      clockRuntime.policy || matchClockConfigRef.current
    );
    const clockPlayerCount = Math.max(
      0,
      Number(
        (session.players || []).length
        || clockState?.players?.length
        || clockRuntime.playerCount
        || 0
      )
    );
    const activePlayerIndex = matchClockActivePlayerFromState(clockState);
    const clockNowMs = nowMonotonicMs();
    const activeChanged = Number(clockRuntime.activePlayerIndex) !== Number(activePlayerIndex);
    matchClockRef.current = {
      policy: clockPolicy,
      playerCount: clockPlayerCount,
      baseRemainingMsByPlayer: normalizeMatchClockRemaining(
        clockRuntime.baseRemainingMsByPlayer,
        clockPlayerCount,
        clockPolicy.initialMs
      ),
      activePlayerIndex,
      epochStartedAtMs: activePlayerIndex == null
        ? null
        : (activeChanged ? clockNowMs : clockRuntime.epochStartedAtMs ?? clockNowMs),
      clockHash: String(clockRuntime.clockHash || INITIAL_MATCH_CLOCK_HASH),
      lastSequence: Number(clockRuntime.lastSequence || 0),
    };
    const currentMatchClock = createMatchClockSnapshot({
      policy: matchClockRef.current.policy,
      playerCount: matchClockRef.current.playerCount,
      baseRemainingMsByPlayer: matchClockRef.current.baseRemainingMsByPlayer,
      activePlayerIndex: matchClockRef.current.activePlayerIndex,
      epochStartedAtMs: matchClockRef.current.epochStartedAtMs,
      clockHash: matchClockRef.current.clockHash,
      lastSequence: matchClockRef.current.lastSequence,
      nowMs: clockNowMs,
    });

    const currentHostPeerId = session.localPeerId || session.hostPeerId || basePayload.hostPeerId || "";
    const currentHostPlayerIndex = resolveLocalPlayerIndex(session);
    const currentPlayers = toPublicPlayers(session.players).map((player) =>
      currentHostPlayerIndex != null && Number(player.index) === Number(currentHostPlayerIndex)
        ? {
            ...player,
            currentPeerId: currentHostPeerId || player.currentPeerId || player.peerId,
            connected: true,
          }
        : player
    );

    return {
      ...cloneMultiplayerPayload(basePayload),
      protocolVersion: PROTOCOL_VERSION,
      lobbyId: basePayload.lobbyId || session.lobbyId || "",
      hostPeerId: basePayload.hostPeerId || "",
      currentHostPeerId,
      currentHostPlayerIndex:
        currentHostPlayerIndex == null ? undefined : Number(currentHostPlayerIndex),
      currentPlayers,
      currentMatchClock,
      format: normalizeMatchFormat(basePayload.format || session.format),
      startingLife: Number(basePayload.startingLife || session.startingLife || 20),
      securityMode: sessionSecurityMode(session, matchPayloadSecurityMode(basePayload)),
      players: cloneMultiplayerPayload(basePayload.players || []),
    };
  }, []);

  const sendHostedStateMessage = useCallback(
    async (conn, payload) => {
      const session = multiplayerRef.current;
      const peerPlayer = session.players.find((entry) =>
        String(entry.peerId || "") === String(conn.peer || "")
        || String(entry.currentPeerId || "") === String(conn.peer || "")
      );
      if (!peerPlayer) {
        throw new Error("Cannot resync an unknown peer");
      }
      const peerIndex = normalizePlayerIndex(peerPlayer?.index);
      const currentGame = gameRef.current;
	      if (!currentGame || typeof currentGame.exportSyncCheckpoint !== "function") {
	        throw new Error("Game engine cannot export a resync checkpoint");
	      }
	      const securityMode = sessionSecurityMode(
	        session,
	        matchPayloadSecurityMode(payload.match, MULTIPLAYER_SECURITY_VERIFIED)
	      );
	      const checkpoint =
	        isVerifiedMultiplayerSecurityMode(securityMode)
	        && peerIndex != null
	        && typeof currentGame.exportRedactedSyncCheckpoint === "function"
	          ? await currentGame.exportRedactedSyncCheckpoint(peerIndex)
	          : await currentGame.exportSyncCheckpoint();
	      const serializedCheckpoint = cloneMultiplayerPayload(checkpoint);
	      const actions = (actionHistoryRef.current || [])
	        .map((entry) => cloneMultiplayerPayload(entry));
	      const lastSequence = Number(actions.at(-1)?.seq ?? 0);
	      const resyncEnvelope = isVerifiedMultiplayerSecurityMode(securityMode)
        ? await buildSignedResyncEnvelope({
            keyPair: auditKeyPairRef.current,
            matchId: payload.match?.auditMatchId || currentAuditMatchId(),
            signer: resolveLocalPlayerIndex(session) ?? 0,
            lastSequence,
            finalStateHash: auditStateHashRef.current || INITIAL_AUDIT_STATE_HASH,
            checkpoint: serializedCheckpoint,
            actions,
          })
        : null;
      safeSend(conn, {
        ...payload,
        lastSequence,
        match: redactedMatchPayloadForPeer(
          {
            ...payload.match,
            securityMode,
          },
          conn.peer,
          peerIndex
        ),
        checkpoint: serializedCheckpoint,
        actions,
        ...(resyncEnvelope ? { resyncEnvelope } : {}),
      });
    },
    [currentAuditMatchId]
  );

  function sequencedActionRelayKey(message) {
    if (isTrustedMultiplayerSecurityMode(message?.securityMode)) {
      return [
        MULTIPLAYER_SECURITY_TRUSTED,
        Number(message?.seq || 0),
        Number(message?.actorIndex ?? -1),
        canonicalMultiplayerPayload(message?.command || {}),
      ].join(":");
    }
    return [
      String(message?.audit?.matchId || currentAuditMatchId()),
      Number(message?.seq || 0),
      String(message?.audit?.signature || ""),
    ].join(":");
  }

  function relaySequencedAction(message) {
    if (!message || message.type !== "apply_action") return;
    const relayKey = sequencedActionRelayKey(message);
    if (relayedActionIdsRef.current.has(relayKey)) return;
    relayedActionIdsRef.current.add(relayKey);

    const session = multiplayerRef.current;
    ensureDirectPeerConnections(session.players || []);
    for (const player of session.players || []) {
      const peerId = routePeerIdForPlayer(player);
      if (!peerId || peerId === session.localPeerId) continue;
      const sent = sendDirectPeerMessage(peerId, {
        ...cloneMultiplayerPayload(message),
        relayedBy: session.localPeerId || "",
      });
      recordPeerSyncPerf("apply_action:relay_send", {
        seq: Number(message.seq || 0),
        actor: Number(message.actorIndex ?? message.audit?.actor ?? -1),
        target_player_index: player?.index == null ? null : Number(player.index),
        target_peer_id: peerId,
        local_peer_id: String(session.localPeerId || ""),
        role: String(session.role || ""),
        sent,
      });
    }
  }

  function playerCountForClock(uiState) {
    const sessionCount = multiplayerRef.current?.players?.length || 0;
    const stateCount = uiState?.players?.length || 0;
    return Math.max(0, Number(sessionCount || stateCount || matchClockRef.current.playerCount || 0));
  }

  function runtimeMatchClockSnapshot(nowMs = nowMonotonicMs()) {
    const runtime = matchClockRef.current;
    return createMatchClockSnapshot({
      policy: runtime.policy || matchClockConfigRef.current,
      playerCount: runtime.playerCount,
      baseRemainingMsByPlayer: runtime.baseRemainingMsByPlayer,
      activePlayerIndex: runtime.activePlayerIndex,
      epochStartedAtMs: runtime.epochStartedAtMs,
      clockHash: runtime.clockHash || INITIAL_MATCH_CLOCK_HASH,
      lastSequence: runtime.lastSequence || 0,
      nowMs,
    });
  }

  function publishMatchClockSnapshot(snapshot) {
    updateMultiplayer((prev) => ({
      ...prev,
      matchClock: snapshot,
      actionTimer: actionTimerSnapshotFromMatchClock(snapshot),
    }));
    return snapshot;
  }

  function resetMatchClockForMatch(payload, uiState) {
    const policy = matchClockPolicyFromPayload(payload, matchClockConfigRef.current);
    matchClockConfigRef.current = policy;
    const playerCount = Math.max(
      payload?.players?.length || 0,
      playerCountForClock(uiState)
    );
    matchClockRef.current = {
      policy,
      playerCount,
      baseRemainingMsByPlayer: normalizeMatchClockRemaining([], playerCount, policy.initialMs),
      activePlayerIndex: null,
      epochStartedAtMs: null,
      clockHash: INITIAL_MATCH_CLOCK_HASH,
      lastSequence: 0,
    };
    return updateMatchClockForState(uiState, { reset: true, policy });
  }

  function updateMatchClockForState(uiState, options = {}) {
    const policy = normalizeMatchClockPolicy(options.policy || matchClockConfigRef.current);
    matchClockConfigRef.current = policy;
    const playerCount = playerCountForClock(uiState);
    const activePlayerIndex = matchClockActivePlayerFromState(uiState);
    const current = matchClockRef.current;
    const reset = Boolean(options.reset);
    const baseRemainingMsByPlayer = reset
      ? normalizeMatchClockRemaining([], playerCount, policy.initialMs)
      : normalizeMatchClockRemaining(
          current.baseRemainingMsByPlayer,
          playerCount,
          policy.initialMs
        );
    const nowMs = nowMonotonicMs();
    const activeChanged = Number(current.activePlayerIndex) !== Number(activePlayerIndex);
    matchClockRef.current = {
      policy,
      playerCount,
      baseRemainingMsByPlayer,
      activePlayerIndex,
      epochStartedAtMs: activePlayerIndex == null
        ? null
        : (reset || activeChanged ? nowMs : current.epochStartedAtMs ?? nowMs),
      clockHash: reset
        ? INITIAL_MATCH_CLOCK_HASH
        : String(current.clockHash || INITIAL_MATCH_CLOCK_HASH),
      lastSequence: reset ? 0 : Number(current.lastSequence || 0),
    };
    return publishMatchClockSnapshot(runtimeMatchClockSnapshot(nowMs));
  }

  function currentMatchClockSnapshot() {
    return runtimeMatchClockSnapshot();
  }

  async function buildMatchClockAuditForCommand({
    command,
    seq,
    actorIndex,
    uiState,
  }) {
    const snapshot = updateMatchClockForState(uiState);
    if (!snapshot.enabled) return null;
    const runtime = matchClockRef.current;
    const activePlayer = snapshot.activePlayerIndex;
    const isTimeoutForfeit = isActionTimeoutForfeitCommand(command);
    const isDisconnectForfeit = isDisconnectTimeoutForfeitCommand(command);
    const isProtocolTimeoutForfeit = isProtocolResponseTimeoutForfeitCommand(command);
    const baseRemaining = normalizeMatchClockRemaining(
      runtime.baseRemainingMsByPlayer,
      runtime.playerCount,
      runtime.policy.initialMs
    );
    let elapsedMs = 0;
    if (activePlayer != null) {
      const observedElapsed = Math.max(
        0,
        Math.floor(nowMonotonicMs() - Number(runtime.epochStartedAtMs ?? nowMonotonicMs()))
      );
      elapsedMs = isTimeoutForfeit
        ? Number(baseRemaining[activePlayer] || 0)
        : (isDisconnectForfeit || isProtocolTimeoutForfeit)
          ? 0
          : Math.min(Number(baseRemaining[activePlayer] || 0), observedElapsed);
    }
    const clock = {
      type: MATCH_CLOCK_AUDIT_TYPE,
      version: 1,
      matchId: currentAuditMatchId(),
      seq: Number(seq),
      actor: Number(actorIndex),
      reason: isTimeoutForfeit
        ? "timeout_claim"
        : isDisconnectForfeit
          ? "disconnect_timeout_claim"
          : isProtocolTimeoutForfeit
            ? "protocol_response_timeout_claim"
            : "action",
      policy: matchClockPolicyPayload(runtime.policy),
      activePlayer: activePlayer == null ? null : Number(activePlayer),
      elapsedMs,
      remainingMsByPlayer: debitMatchClockRemaining(baseRemaining, activePlayer, elapsedMs),
      previousClockHash: String(runtime.clockHash || INITIAL_MATCH_CLOCK_HASH),
      basisSequence: Number(multiplayerRef.current.lastAppliedSequence || 0),
    };
    clock.clockHash = await matchClockAuditHash(clock);
    return clock;
  }

	  async function verifyMatchClockAuditForAction({
	    clock,
	    command,
	    seq,
	    actorIndex,
	    uiState,
	    skewMs = 0,
	    enforceObservationBounds = true,
	    enforceUnderreportBounds = enforceObservationBounds,
	    skipTimeoutCertificate = false,
	  }) {
    const snapshot = updateMatchClockForState(uiState);
    if (!snapshot.enabled) {
      if (clock) throw new Error("Sequenced audit unexpectedly included match clock data");
      return;
    }
    if (!clock || typeof clock !== "object") {
      throw new Error("Sequenced audit is missing match clock data");
    }
    if (String(clock.type || "") !== MATCH_CLOCK_AUDIT_TYPE) {
      throw new Error("Sequenced audit has unsupported match clock data");
    }
    if (String(clock.matchId || "") !== currentAuditMatchId()) {
      throw new Error("Match clock audit belongs to a different match");
    }
    if (Number(clock.seq) !== Number(seq) || Number(clock.actor) !== Number(actorIndex)) {
      throw new Error("Match clock audit does not match broadcast action");
    }
    const runtime = matchClockRef.current;
    if (String(clock.previousClockHash || "") !== String(runtime.clockHash || INITIAL_MATCH_CLOCK_HASH)) {
      throw new Error("Match clock hash chain does not match local transcript");
    }
    const expectedHash = await matchClockAuditHash(clock);
    if (expectedHash !== String(clock.clockHash || "")) {
      throw new Error("Match clock audit hash is invalid");
    }
    const policy = normalizeMatchClockPolicy(clock.policy || {});
    if (
      Number(policy.initialMs) !== Number(runtime.policy.initialMs)
      || Number(policy.graceMs) !== Number(runtime.policy.graceMs)
    ) {
      throw new Error("Match clock policy does not match the match genesis");
    }
    const activePlayer = snapshot.activePlayerIndex;
    const clockActivePlayer = clock.activePlayer == null ? null : Number(clock.activePlayer);
    if (clockActivePlayer !== activePlayer) {
      throw new Error("Match clock active player does not match the current decision");
    }
    const baseRemaining = normalizeMatchClockRemaining(
      runtime.baseRemainingMsByPlayer,
      runtime.playerCount,
      runtime.policy.initialMs
    );
    const rawElapsedMs = Number(clock.elapsedMs ?? 0);
    if (!Number.isFinite(rawElapsedMs) || rawElapsedMs < 0) {
      throw new Error("Match clock elapsed time is invalid");
    }
    const elapsedMs = Math.floor(rawElapsedMs);
    if (activePlayer == null && elapsedMs !== 0) {
      throw new Error("Match clock elapsed time requires an active decision");
    }
    const expectedRemaining = debitMatchClockRemaining(baseRemaining, activePlayer, elapsedMs);
    const actualRemaining = normalizeMatchClockRemaining(
      clock.remainingMsByPlayer,
      runtime.playerCount,
      runtime.policy.initialMs
    );
    if (activePlayer != null) {
      const activeRemaining = Number(baseRemaining[activePlayer] || 0);
      if (elapsedMs > activeRemaining) {
        throw new Error("Match clock elapsed time exceeds remaining time");
      }
      const observedElapsedMs = snapshot.startedAtMs == null
        ? 0
        : Math.max(0, Math.floor(nowMonotonicMs() - Number(snapshot.startedAtMs)));
      const signedElapsedIsSelfDisadvantageous = isDisadvantageousActivePlayerClockAdvance({
        actorIndex,
        activePlayerIndex: activePlayer,
        elapsedMs,
        observedElapsedMs,
        previousRemainingMsByPlayer: baseRemaining,
        submittedRemainingMsByPlayer: actualRemaining,
        isTimeoutForfeit: isActionTimeoutForfeitCommand(command),
        skewMs,
      });
      if (
        enforceObservationBounds
        && elapsedMs > observedElapsedMs + Number(skewMs || 0)
        && !signedElapsedIsSelfDisadvantageous
      ) {
        throw new Error("Match clock elapsed time exceeds local observation");
      }
      const underreportSkewMs = Math.max(
        Number(skewMs || 0),
        MATCH_CLOCK_ELAPSED_UNDERREPORT_SKEW_MS
      );
	      if (
	        enforceUnderreportBounds
	        && elapsedMs + underreportSkewMs < observedElapsedMs
	      ) {
        throw new Error("Match clock elapsed time is below local observation");
      }
    }
    if (canonicalJson(expectedRemaining) !== canonicalJson(actualRemaining)) {
      throw new Error("Match clock remaining time does not match elapsed time");
    }
    if (!isActionTimeoutForfeitCommand(command)) return;
    const forfeitedPlayer = Number(command.player);
    if (activePlayer == null || forfeitedPlayer !== activePlayer) {
      throw new Error("Timeout forfeit does not match the active match clock");
    }
    if (actualRemaining[activePlayer] !== 0) {
      throw new Error("Timeout forfeit did not exhaust the player's match clock");
    }
    if (!skipTimeoutCertificate) {
      await verifyTimeoutCertificate(command, uiState);
    }
    const liveRemaining = Number(snapshot.remainingMs ?? runtime.policy.initialMs);
    if (liveRemaining > Number(runtime.policy.graceMs || 0) + Number(skewMs || 0)) {
      throw new Error("Match clock has not expired");
    }
  }

  function commitMatchClockAudit(clock, uiState) {
    if (!clock || typeof clock !== "object") {
      return updateMatchClockForState(uiState);
    }
    const policy = normalizeMatchClockPolicy(clock.policy || matchClockConfigRef.current);
    const playerCount = playerCountForClock(uiState);
    matchClockConfigRef.current = policy;
    matchClockRef.current = {
      policy,
      playerCount,
      baseRemainingMsByPlayer: normalizeMatchClockRemaining(
        clock.remainingMsByPlayer,
        playerCount,
        policy.initialMs
      ),
      activePlayerIndex: null,
      epochStartedAtMs: null,
      clockHash: String(clock.clockHash || INITIAL_MATCH_CLOCK_HASH),
      lastSequence: Number(clock.seq || 0),
    };
    return updateMatchClockForState(uiState, { policy });
  }

  function stageLocalMatchClockAudit(clock) {
    if (!clock || typeof clock !== "object") return null;
    const previous = {
      ...matchClockRef.current,
      baseRemainingMsByPlayer: [...(matchClockRef.current.baseRemainingMsByPlayer || [])],
    };
    const policy = normalizeMatchClockPolicy(clock.policy || matchClockConfigRef.current);
    const playerCount = Math.max(
      previous.playerCount || 0,
      Array.isArray(clock.remainingMsByPlayer) ? clock.remainingMsByPlayer.length : 0
    );
    matchClockConfigRef.current = policy;
    matchClockRef.current = {
      policy,
      playerCount,
      baseRemainingMsByPlayer: normalizeMatchClockRemaining(
        clock.remainingMsByPlayer,
        playerCount,
        policy.initialMs
      ),
      activePlayerIndex: null,
      epochStartedAtMs: null,
      clockHash: String(clock.clockHash || previous.clockHash || INITIAL_MATCH_CLOCK_HASH),
      lastSequence: Number(clock.seq || previous.lastSequence || 0),
    };
    publishMatchClockSnapshot(runtimeMatchClockSnapshot());
    return previous;
  }

  function restoreMatchClockRuntime(runtime, uiState) {
    if (!runtime) return null;
    matchClockRef.current = {
      ...runtime,
      baseRemainingMsByPlayer: [...(runtime.baseRemainingMsByPlayer || [])],
    };
    return updateMatchClockForState(uiState || stateRef.current, {
      policy: runtime.policy || matchClockConfigRef.current,
    });
  }

  function latestMatchClockAuditFromActions(actions = []) {
    for (let index = (actions || []).length - 1; index >= 0; index -= 1) {
      const entry = actions[index] || {};
      const clock = entry.clock || entry.audit?.clock || null;
      if (clock && typeof clock === "object") return clock;
    }
    return null;
  }

  function restoreMatchClockRuntimeFromActionTranscript(actions = [], uiState, matchPayload = null) {
    const clock = latestMatchClockAuditFromActions(actions);
    const finalSequence = Number(actions.at(-1)?.seq || 0);
    if (!clock) {
      const policy = matchClockPolicyFromPayload(matchPayload, matchClockConfigRef.current);
      const playerCount = Math.max(
        Array.isArray(matchPayload?.players) ? matchPayload.players.length : 0,
        playerCountForClock(uiState)
      );
      matchClockConfigRef.current = policy;
      matchClockRef.current = {
        policy,
        playerCount,
        baseRemainingMsByPlayer: normalizeMatchClockRemaining([], playerCount, policy.initialMs),
        activePlayerIndex: null,
        epochStartedAtMs: null,
        clockHash: INITIAL_MATCH_CLOCK_HASH,
        lastSequence: finalSequence,
      };
      return updateMatchClockForState(uiState, { policy });
    }

    const policy = normalizeMatchClockPolicy(clock.policy || matchClockConfigRef.current);
    const playerCount = Math.max(
      playerCountForClock(uiState),
      Array.isArray(clock.remainingMsByPlayer) ? clock.remainingMsByPlayer.length : 0
    );
    matchClockConfigRef.current = policy;
    matchClockRef.current = {
      policy,
      playerCount,
      baseRemainingMsByPlayer: normalizeMatchClockRemaining(
        clock.remainingMsByPlayer,
        playerCount,
        policy.initialMs
      ),
      activePlayerIndex: null,
      epochStartedAtMs: null,
      clockHash: String(clock.clockHash || INITIAL_MATCH_CLOCK_HASH),
      lastSequence: Number(clock.seq || finalSequence),
    };
    return updateMatchClockForState(uiState, { policy });
  }

  function alignMatchClockObservationFromHostSnapshot(hostSnapshot, uiState) {
    if (!hostSnapshot || typeof hostSnapshot !== "object") {
      return updateMatchClockForState(uiState);
    }
    const runtime = matchClockRef.current;
    const policy = normalizeMatchClockPolicy(runtime.policy || matchClockConfigRef.current);
    const playerCount = playerCountForClock(uiState);
    const activePlayerIndex = matchClockActivePlayerFromState(uiState);
    const hostClockHash = String(hostSnapshot.clockHash || "");
    const hostSequence = Number(hostSnapshot.lastSequence || 0);
    if (
      hostClockHash
      && hostClockHash !== String(runtime.clockHash || INITIAL_MATCH_CLOCK_HASH)
    ) {
      return updateMatchClockForState(uiState);
    }
    if (hostSequence && hostSequence !== Number(runtime.lastSequence || 0)) {
      return updateMatchClockForState(uiState);
    }
    const baseRemaining = normalizeMatchClockRemaining(
      runtime.baseRemainingMsByPlayer,
      playerCount,
      policy.initialMs
    );
    const hostRemaining = normalizeMatchClockRemaining(
      hostSnapshot.remainingMsByPlayer,
      playerCount,
      policy.initialMs
    );
    const nowMs = nowMonotonicMs();
    const activeElapsed = activePlayerIndex == null
      ? 0
      : Math.max(
          0,
          Number(baseRemaining[activePlayerIndex] || 0)
            - Number(hostRemaining[activePlayerIndex] || 0)
        );
    matchClockConfigRef.current = policy;
    matchClockRef.current = {
      policy,
      playerCount,
      baseRemainingMsByPlayer: baseRemaining,
      activePlayerIndex,
      epochStartedAtMs: activePlayerIndex == null ? null : nowMs - activeElapsed,
      clockHash: String(runtime.clockHash || INITIAL_MATCH_CLOCK_HASH),
      lastSequence: Number(runtime.lastSequence || 0),
    };
    return publishMatchClockSnapshot(runtimeMatchClockSnapshot(nowMs));
  }

	  async function signTimeoutVoteForSnapshot({
    basisSequence,
    forfeitedPlayer,
    activePlayer,
    clockHash,
    remainingMs,
  }) {
    const { keyPair } = await ensureAuditIdentity();
    const voter = resolveLocalPlayerIndex(multiplayerRef.current);
    if (voter == null || Number(voter) === Number(forfeitedPlayer)) {
      throw new Error("Timed-out player cannot sign their own timeout certificate");
    }
    const payload = timeoutVotePayload({
      matchId: currentAuditMatchId(),
      basisSequence,
      forfeitedPlayer,
      activePlayer,
      clockHash,
      remainingMs,
      voter,
    });
    return {
      ...payload,
      signatureAlgorithm: "ecdsa-p256-sha256",
      signature: await signAuditPayload(keyPair, payload),
    };
  }

  async function verifyTimeoutVote(vote, expected) {
    const voter = normalizePlayerIndex(vote?.voter);
    if (voter == null) {
      throw new Error("Timeout certificate contains an invalid voter");
    }
    const payload = timeoutVotePayload({
      ...expected,
      voter,
    });
    if (
      String(vote?.domain || "") !== TIMEOUT_VOTE_DOMAIN
      || String(vote?.matchId || "") !== payload.matchId
      || Number(vote?.basisSequence) !== payload.basisSequence
      || Number(vote?.forfeitedPlayer) !== payload.forfeitedPlayer
      || Number(vote?.activePlayer) !== payload.activePlayer
      || String(vote?.clockHash || "") !== payload.clockHash
      || Math.max(0, Math.floor(Number(vote?.remainingMs || 0))) !== payload.remainingMs
    ) {
      throw new Error("Timeout certificate vote does not match the timeout claim");
    }
    const publicKey = await importCachedAuditPublicKey(publicKeyForAuditSigner(voter));
    const valid = await verifyAuditPayload(publicKey, payload, vote.signature || "");
    if (!valid) {
      throw new Error("Timeout certificate vote signature is invalid");
    }
    return voter;
  }

  async function verifyTimeoutCertificate(command, uiState) {
    if (!isActionTimeoutForfeitCommand(command)) return;
    const playerCount = playerCountForClock(uiState);
    if (playerCount < 3) return;
    const forfeitedPlayer = Number(command.player);
    const certificate = timeoutCertificateFromCommand(command);
    if (!certificate || String(certificate.type || "") !== "match_clock_timeout_quorum_v1") {
      throw new Error("Multiplayer timeout forfeit is missing its quorum certificate");
    }
    const expected = {
      matchId: currentAuditMatchId(),
      basisSequence: Number(command.basis_sequence ?? certificate.basisSequence ?? 0),
      forfeitedPlayer,
      activePlayer: forfeitedPlayer,
      clockHash: String(command.match_clock_hash || certificate.clockHash || ""),
      remainingMs: Number(command.remaining_ms ?? certificate.remainingMs ?? 0),
    };
    const requiredVoters = expectedTimeoutVoters(
      matchStartPayloadRef.current?.players || multiplayerRef.current.players || [],
      forfeitedPlayer
    );
    const votes = Array.isArray(certificate.votes) ? certificate.votes : [];
    if (votes.length !== requiredVoters.length) {
      throw new Error("Timeout certificate must include every non-timed-out player");
    }
    const seen = new Set();
    for (const vote of votes) {
      const voter = await verifyTimeoutVote(vote, expected);
      if (!requiredVoters.includes(voter)) {
        throw new Error("Timeout certificate contains an ineligible voter");
      }
      if (seen.has(voter)) {
        throw new Error("Timeout certificate contains a duplicate voter");
      }
      seen.add(voter);
    }
    for (const voter of requiredVoters) {
      if (!seen.has(voter)) {
        throw new Error("Timeout certificate is missing a required voter");
      }
    }
  }

  async function validateTimeoutForfeitCommand(command, uiState, options = {}) {
    if (!isActionTimeoutForfeitCommand(command)) return;
    const forfeitedPlayer = Number(command.player);
    const activePlayer = matchClockActivePlayerFromState(uiState);
    if (activePlayer == null) {
      throw new Error("No active decision can be forfeited by timeout");
    }
    if (activePlayer !== forfeitedPlayer) {
      throw new Error("Timeout forfeit does not match the current decision player");
    }

    const timer = currentMatchClockSnapshot();
    if (timer.activePlayerIndex == null || Number(timer.activePlayerIndex) !== forfeitedPlayer) {
      throw new Error("Timeout forfeit does not match the active match clock");
    }

    const skewMs = Number(options.skewMs ?? 0);
    if (Number(timer.remainingMs ?? 0) > Number(timer.graceMs || 0) + skewMs) {
      throw new Error("Match clock has not expired");
    }
    if (!options.skipCertificate) {
      await verifyTimeoutCertificate(command, uiState);
    }
  }

  async function answerTimeoutVoteRequest(conn, message) {
    try {
      const requester = playerIndexForPeerId(conn?.peer);
      const localVoter = resolveLocalPlayerIndex(multiplayerRef.current);
      const forfeitedPlayer = Number(message?.forfeitedPlayer);
      const basisSequence = Number(message?.basisSequence);
      if (requester == null) {
        throw new Error("Timeout vote requester is not a match player");
      }
      if (Number(requester) === forfeitedPlayer) {
        throw new Error("Timed-out player cannot request their own timeout certificate");
      }
      if (localVoter == null || Number(localVoter) === forfeitedPlayer) {
        throw new Error("Timed-out player cannot vote on their own timeout");
      }
      if (String(message?.matchId || "") !== currentAuditMatchId()) {
        throw new Error("Timeout vote request belongs to a different match");
      }
      if (basisSequence !== Number(multiplayerRef.current.lastAppliedSequence || 0)) {
        throw new Error("Timeout vote request is not based on the local transcript head");
      }
      const liveState = gameRef.current ? await gameRef.current.uiState() : stateRef.current;
      const activePlayer = matchClockActivePlayerFromState(liveState);
      updateMatchClockForState(liveState);
      const timer = currentMatchClockSnapshot();
      if (activePlayer == null || Number(activePlayer) !== forfeitedPlayer) {
        throw new Error("Timeout vote does not match the active decision player");
      }
      if (Number(timer.activePlayerIndex) !== forfeitedPlayer) {
        throw new Error("Timeout vote does not match the active match clock");
      }
      if (String(message?.clockHash || "") !== String(timer.clockHash || "")) {
        throw new Error("Timeout vote clock hash does not match the local clock head");
      }
      if (Number(timer.remainingMs ?? 0) > Number(timer.graceMs || 0) + MATCH_CLOCK_CLAIM_SKEW_MS) {
        throw new Error("Match clock has not expired");
      }
      const vote = await signTimeoutVoteForSnapshot({
        basisSequence,
        forfeitedPlayer,
        activePlayer,
        clockHash: String(timer.clockHash || ""),
        remainingMs: Number(timer.remainingMs || 0),
      });
      safeSend(conn, {
        type: "timeout_vote_response",
        protocolVersion: PROTOCOL_VERSION,
        requestId: message.requestId,
        vote,
      });
    } catch (err) {
      safeSend(conn, {
        type: "timeout_vote_response",
        protocolVersion: PROTOCOL_VERSION,
        requestId: message.requestId,
        error: toErrorMessage(err),
      });
    }
  }

  async function answerDisconnectForfeitVoteRequest(conn, message) {
    try {
      const requester = playerIndexForPeerId(conn?.peer);
      const localVoter = resolveLocalPlayerIndex(multiplayerRef.current);
      const forfeitedPlayer = normalizePlayerIndex(message?.forfeitedPlayer);
      const basisSequence = Number(message?.basisSequence);
      if (requester == null) {
        throw new Error("Disconnect forfeit requester is not a match player");
      }
      if (normalizePlayerIndex(message?.requesterIndex) !== requester) {
        throw new Error("Disconnect forfeit requester index does not match the peer");
      }
      if (forfeitedPlayer == null) {
        throw new Error("Disconnect forfeit request targets an invalid player");
      }
      if (localVoter == null || Number(localVoter) === Number(forfeitedPlayer)) {
        throw new Error("Disconnected player cannot vote on their own disconnect forfeit");
      }
      if (String(message?.matchId || "") !== currentAuditMatchId()) {
        throw new Error("Disconnect forfeit vote request belongs to a different match");
      }
      if (basisSequence !== Number(multiplayerRef.current.lastAppliedSequence || 0)) {
        throw new Error("Disconnect forfeit vote request is not based on the local transcript head");
      }
      const vote = await signDisconnectForfeitVoteForCommand({
        type: "forfeit_player",
        player: forfeitedPlayer,
        reason: DISCONNECT_FORFEIT_REASON,
        basis_sequence: basisSequence,
        disconnected_peer_id: String(message?.forfeitedPeerId || ""),
        disconnect_timeout_ms: Number(message?.disconnectTimeoutMs || DISCONNECT_AUTO_FORFEIT_MS),
      });
      safeSend(conn, {
        type: "disconnect_forfeit_vote_response",
        protocolVersion: PROTOCOL_VERSION,
        requestId: message.requestId,
        vote,
      });
    } catch (err) {
      safeSend(conn, {
        type: "disconnect_forfeit_vote_response",
        protocolVersion: PROTOCOL_VERSION,
        requestId: message.requestId,
        error: toErrorMessage(err),
      });
    }
  }

  async function collectTimeoutCertificateForCommand(command) {
    if (!isActionTimeoutForfeitCommand(command)) return null;
    const players = reindexPlayers(matchStartPayloadRef.current?.players || multiplayerRef.current.players || []);
    if (players.length < 3) return null;
    if (!isCurrentAuditPlayerCount(players.length)) {
      throw new Error("Current protocol requires 2, 3, or 4 players");
    }
    const forfeitedPlayer = Number(command.player);
    const basisSequence = Number(command.basis_sequence || multiplayerRef.current.lastAppliedSequence || 0);
    const expected = {
      matchId: currentAuditMatchId(),
      basisSequence,
      forfeitedPlayer,
      activePlayer: forfeitedPlayer,
      clockHash: String(command.match_clock_hash || ""),
      remainingMs: Number(command.remaining_ms || 0),
    };
    const requiredVoters = expectedTimeoutVoters(players, forfeitedPlayer);
    const localPlayer = resolveLocalPlayerIndex(multiplayerRef.current);
    const votes = [];
    for (const voter of requiredVoters) {
      if (Number(voter) === Number(localPlayer)) {
        votes.push(await signTimeoutVoteForSnapshot(expected));
        continue;
      }
      const player = players.find((entry) => Number(entry.index) === Number(voter));
      const routePeerId = routePeerIdForPlayer(player);
      if (!routePeerId) {
        throw new Error(`Missing peer route for timeout voter ${voter + 1}`);
      }
      const conn = await waitForZiffleRoute(routePeerId);
      const requestId = makeZiffleRequestId("timeout-vote");
      const voterLabel = player.name || `Player ${Number(voter) + 1}`;
      const waiter = waitForTimeoutVote(requestId, 15000, {
        peerIndex: voter,
        peerName: voterLabel,
        description:
          `${voterLabel} must sign the timeout vote before this forfeit claim can be submitted.`,
      });
      safeSend(conn, {
        type: "timeout_vote_request",
        protocolVersion: PROTOCOL_VERSION,
        requestId,
        ...expected,
        requesterIndex: localPlayer,
      });
      const vote = await waiter;
      await verifyTimeoutVote(vote, expected);
      votes.push(vote);
    }
    votes.sort((left, right) => Number(left.voter) - Number(right.voter));
    return {
      type: "match_clock_timeout_quorum_v1",
      matchId: expected.matchId,
      basisSequence,
      forfeitedPlayer,
      clockHash: expected.clockHash,
      remainingMs: expected.remainingMs,
      voters: requiredVoters,
      votes,
    };
  }

  function forfeitedPlayersForQuorum(pendingAction = null) {
    const forfeited = new Set();
    for (const entry of actionHistoryRef.current || []) {
      if (isForfeitCommand(entry?.command)) {
        forfeited.add(Number(entry.command.player));
      }
    }
    if (pendingAction && isForfeitCommand(pendingAction.command)) {
      forfeited.add(Number(pendingAction.command.player));
    }
    return forfeited;
  }

  function actionQuorumRoster(action = null) {
    const forfeited = forfeitedPlayersForQuorum(action);
    return reindexPlayers(matchStartPayloadRef.current?.players || multiplayerRef.current.players || [])
      .filter((player) => !forfeited.has(Number(player.index)));
  }

  function disconnectForfeitRoster(forfeitedPlayer) {
    const target = Number(forfeitedPlayer);
    return actionQuorumRoster()
      .filter((player) => Number(player.index) !== target);
  }

  function rememberLocalDisconnectObservation(peerId, details = {}) {
    const normalizedPeerId = String(peerId || "").trim();
    if (!normalizedPeerId) return null;
    const player = reindexPlayers(matchStartPayloadRef.current?.players || multiplayerRef.current.players || [])
      .find((entry) => String(entry?.peerId || "").trim() === normalizedPeerId);
    const playerIndex = normalizePlayerIndex(details.playerIndex ?? player?.index);
    const observedAtMs = Math.max(
      0,
      Math.floor(Number(details.disconnectedAtMs ?? Date.now()))
    );
    if (playerIndex == null || !observedAtMs) return null;
    const observation = {
      playerIndex,
      peerId: normalizedPeerId,
      disconnectedAtMs: observedAtMs,
      source: String(details.source || "direct_peer"),
    };
    localDisconnectObservationsRef.current.set(`player:${playerIndex}`, observation);
    localDisconnectObservationsRef.current.set(`peer:${normalizedPeerId}`, observation);
    return observation;
  }

  function clearLocalDisconnectObservation(peerId, playerIndex = null) {
    const normalizedPeerId = String(peerId || "").trim();
    const normalizedPlayerIndex = normalizePlayerIndex(playerIndex);
    if (normalizedPeerId) {
      localDisconnectObservationsRef.current.delete(`peer:${normalizedPeerId}`);
    }
    if (normalizedPlayerIndex != null) {
      localDisconnectObservationsRef.current.delete(`player:${normalizedPlayerIndex}`);
    }
  }

  function localDisconnectObservationForPlayer(forfeitedPlayer, peerId = "") {
    const target = normalizePlayerIndex(forfeitedPlayer);
    const normalizedPeerId = String(peerId || "").trim();
    const byPlayer = target == null
      ? null
      : localDisconnectObservationsRef.current.get(`player:${target}`);
    const byPeer = normalizedPeerId
      ? localDisconnectObservationsRef.current.get(`peer:${normalizedPeerId}`)
      : null;
    const observation = byPlayer || byPeer || null;
    if (!observation) return null;
    if (target != null && Number(observation.playerIndex) !== target) return null;
    if (normalizedPeerId && String(observation.peerId || "") !== normalizedPeerId) return null;
    return observation;
  }

  function playerForDisconnectForfeit(forfeitedPlayer) {
    const target = Number(forfeitedPlayer);
    return reindexPlayers(multiplayerRef.current.players || [])
      .find((player) => Number(player.index) === target)
      || reindexPlayers(matchStartPayloadRef.current?.players || [])
        .find((player) => Number(player.index) === target)
      || null;
  }

  async function signDisconnectForfeitVoteForCommand(command) {
    const { keyPair } = await ensureAuditIdentity();
    const voter = resolveLocalPlayerIndex(multiplayerRef.current);
    const forfeitedPlayer = normalizePlayerIndex(command?.player);
    if (voter == null || forfeitedPlayer == null || Number(voter) === Number(forfeitedPlayer)) {
      throw new Error("Disconnected player cannot sign their own disconnect forfeit");
    }
    const target = playerForDisconnectForfeit(forfeitedPlayer);
    const forfeitedPeerId = String(command?.disconnected_peer_id || target?.peerId || "").trim();
    const disconnectTimeoutMs = Math.max(
      0,
      Math.floor(Number(command?.disconnect_timeout_ms || DISCONNECT_AUTO_FORFEIT_MS))
    );
    if (!forfeitedPeerId) {
      throw new Error("Disconnect forfeit is missing the disconnected peer id");
    }
    if (!target || String(target.peerId || "").trim() !== forfeitedPeerId) {
      throw new Error("Disconnect forfeit peer id does not match the local player record");
    }
    const observation = localDisconnectObservationForPlayer(forfeitedPlayer, forfeitedPeerId);
    if (!observation) {
      throw new Error("Disconnect forfeit requires an independently observed disconnect");
    }
    const disconnectedAtMs = Math.max(
      0,
      Math.floor(Number(observation.disconnectedAtMs || 0))
    );
    if (!disconnectedAtMs) {
      throw new Error("Disconnect forfeit is missing a disconnect observation timestamp");
    }
    const eligibleAtMs = disconnectedAtMs + disconnectTimeoutMs;
    const signedAtMs = Date.now();
    if (signedAtMs + MATCH_CLOCK_CLAIM_SKEW_MS < eligibleAtMs) {
      throw new Error("Disconnect timeout has not elapsed");
    }
    return buildSignedDisconnectForfeitVote({
      keyPair,
      matchId: currentAuditMatchId(),
      basisSequence: Number(command?.basis_sequence ?? multiplayerRef.current.lastAppliedSequence ?? 0),
      forfeitedPlayer,
      forfeitedPeerId,
      disconnectTimeoutMs,
      disconnectedAtMs,
      eligibleAtMs,
      signedAtMs,
      voter,
    });
  }

  async function validateDisconnectForfeitCommand(command, options = {}) {
    if (!isDisconnectTimeoutForfeitCommand(command)) return;
    const forfeitedPlayer = normalizePlayerIndex(command.player);
    const actorIndex = normalizePlayerIndex(options.actorIndex);
    if (forfeitedPlayer == null) {
      throw new Error("Disconnect forfeit targets an invalid player");
    }
    if (actorIndex != null && actorIndex === forfeitedPlayer) {
      throw new Error("Disconnected player cannot claim their own disconnect forfeit");
    }
    const expectedBasisSequence = Number(multiplayerRef.current.lastAppliedSequence || 0);
    if (Number(command.basis_sequence ?? expectedBasisSequence) !== expectedBasisSequence) {
      throw new Error("Disconnect forfeit is not based on the local transcript head");
    }
    const certificate = disconnectCertificateFromCommand(command);
    const target = playerForDisconnectForfeit(forfeitedPlayer);
    const forfeitedPeerId = String(
      command.disconnected_peer_id
      || certificate?.forfeitedPeerId
      || target?.peerId
      || ""
    ).trim();
    const disconnectTimeoutMs = Math.max(
      0,
      Math.floor(Number(command.disconnect_timeout_ms ?? certificate?.disconnectTimeoutMs ?? DISCONNECT_AUTO_FORFEIT_MS))
    );
    const claimedDisconnectedAtMs = Math.max(
      0,
      Math.floor(Number(command.disconnected_at_ms ?? certificate?.disconnectedAtMs ?? 0))
    );
    if (!target) {
      throw new Error("Disconnect forfeit target is missing from the local player record");
    }
    if (!forfeitedPeerId || String(target.peerId || "").trim() !== forfeitedPeerId) {
      throw new Error("Disconnect forfeit peer id does not match the local player record");
    }
    const observation = localDisconnectObservationForPlayer(forfeitedPlayer, forfeitedPeerId);
    const observedDisconnectedAtMs = Math.max(
      0,
      Math.floor(Number(observation?.disconnectedAtMs || 0))
    );
    if (!observedDisconnectedAtMs) {
      throw new Error("Disconnect forfeit is missing a local disconnect observation timestamp");
    }
    if (
      claimedDisconnectedAtMs
      && Math.abs(claimedDisconnectedAtMs - observedDisconnectedAtMs) > DISCONNECT_AUTO_FORFEIT_MS
    ) {
      throw new Error("Disconnect forfeit does not match the local disconnect observation");
    }
    if (Date.now() + MATCH_CLOCK_CLAIM_SKEW_MS < observedDisconnectedAtMs + disconnectTimeoutMs) {
      throw new Error("Disconnect timeout has not elapsed");
    }
    const roster = disconnectForfeitRoster(forfeitedPlayer);
    if (options.skipCertificate) return;
    await verifyDisconnectForfeitCertificate({
      certificate,
      command: {
        ...command,
        disconnected_peer_id: forfeitedPeerId,
        disconnect_timeout_ms: disconnectTimeoutMs,
        matchId: currentAuditMatchId(),
        nowMs: Date.now(),
        maxFutureSkewMs: MATCH_CLOCK_CLAIM_SKEW_MS,
      },
      players: roster,
      threshold: disconnectForfeitVoteThreshold(roster.length),
    });
  }

  async function collectDisconnectForfeitCertificateForCommand(command) {
    if (!isDisconnectTimeoutForfeitCommand(command)) return null;
    const forfeitedPlayer = normalizePlayerIndex(command.player);
    if (forfeitedPlayer == null) {
      throw new Error("Disconnect forfeit targets an invalid player");
    }
    const roster = disconnectForfeitRoster(forfeitedPlayer);
    const threshold = disconnectForfeitVoteThreshold(roster.length);
    if (threshold <= 0) return null;

    const target = playerForDisconnectForfeit(forfeitedPlayer);
    const expected = {
      matchId: currentAuditMatchId(),
      basisSequence: Number(command.basis_sequence ?? multiplayerRef.current.lastAppliedSequence ?? 0),
      forfeitedPlayer,
      forfeitedPeerId: String(command.disconnected_peer_id || target?.peerId || ""),
      disconnectTimeoutMs: Math.max(
        0,
        Math.floor(Number(command.disconnect_timeout_ms || DISCONNECT_AUTO_FORFEIT_MS))
      ),
      disconnectedAtMs: null,
      nowMs: Date.now(),
      maxFutureSkewMs: MATCH_CLOCK_CLAIM_SKEW_MS,
    };
    const votes = [];
    const seen = new Set();
    const addVote = async (vote) => {
      const voter = await verifyDisconnectForfeitVote({
        vote,
        expected,
        players: roster,
      });
      if (seen.has(voter)) return;
      seen.add(voter);
      votes.push(cloneMultiplayerPayload(vote));
    };

    const localPlayer = resolveLocalPlayerIndex(multiplayerRef.current);
    if (roster.some((player) => Number(player.index) === Number(localPlayer))) {
      await addVote(await signDisconnectForfeitVoteForCommand({
        ...command,
        disconnected_peer_id: expected.forfeitedPeerId,
        disconnect_timeout_ms: expected.disconnectTimeoutMs,
      }));
    }

    if (votes.length < threshold) {
      ensureDirectPeerConnections(roster);
      const pendingWaitRequestIds = [];
      let collectingVotes = true;
      const pending = roster
        .filter((player) => Number(player.index) !== Number(localPlayer))
        .map(async (player) => {
          const routePeerId = routePeerIdForPlayer(player);
          if (!routePeerId) {
            throw new Error(`Missing peer route for disconnect voter ${Number(player.index) + 1}`);
          }
          const conn = await waitForZiffleRoute(routePeerId);
          if (!collectingVotes) {
            throw new Error("Disconnect-forfeit vote collection is complete");
          }
          const requestId = makeZiffleRequestId("disconnect-forfeit-vote");
          pendingWaitRequestIds.push(requestId);
          const voterLabel = player.name || `Player ${Number(player.index) + 1}`;
          const waiter = waitForTimeoutVote(requestId, 15000, {
            peerIndex: Number(player.index),
            peerName: voterLabel,
            description:
              `${voterLabel} must sign the disconnect-forfeit vote before this claim can be submitted.`,
          });
          safeSend(conn, {
            type: "disconnect_forfeit_vote_request",
            protocolVersion: PROTOCOL_VERSION,
            requestId,
            ...expected,
            requesterIndex: localPlayer,
          });
          return waiter;
        });
      const unsettled = new Set(pending);
      while (votes.length < threshold && unsettled.size > 0) {
        const settled = await Promise.race([...unsettled].map((promise) =>
          promise.then(
            (vote) => ({ promise, vote }),
            (err) => ({ promise, err })
          )
        ));
        unsettled.delete(settled.promise);
        if (settled.err) {
          if (protocolResponseTimeoutClaimFromError(settled.err)) throw settled.err;
          continue;
        }
        await addVote(settled.vote);
      }
      collectingVotes = false;
      for (const promise of unsettled) {
        promise.catch(() => {});
      }
      for (const requestId of pendingWaitRequestIds) {
        clearPeerWait(requestId);
      }
    }

    if (votes.length < threshold) {
      throw new Error(
        `Disconnect forfeit certificate has ${votes.length} vote(s), expected at least ${threshold}`
      );
    }
    votes.sort((left, right) => Number(left.voter) - Number(right.voter));
    return {
      type: "ironsmith-disconnect-forfeit-v1",
      matchId: expected.matchId,
      basisSequence: expected.basisSequence,
      forfeitedPlayer: expected.forfeitedPlayer,
      forfeitedPeerId: expected.forfeitedPeerId,
      disconnectTimeoutMs: expected.disconnectTimeoutMs,
      threshold,
      voters: votes.map((vote) => Number(vote.voter)),
      votes,
    };
  }

  function protocolResponseTimeoutRoster(forfeitedPlayer) {
    const target = Number(forfeitedPlayer);
    return actionQuorumRoster()
      .filter((player) => Number(player.index) !== target);
  }

  function playerForProtocolResponseTimeout(forfeitedPlayer) {
    const target = Number(forfeitedPlayer);
    return reindexPlayers(multiplayerRef.current.players || [])
      .find((player) => Number(player.index) === target)
      || reindexPlayers(matchStartPayloadRef.current?.players || [])
        .find((player) => Number(player.index) === target)
      || null;
  }

  async function signProtocolResponseTimeoutVoteForCommand(command) {
    const { keyPair } = await ensureAuditIdentity();
    const voter = resolveLocalPlayerIndex(multiplayerRef.current);
    const forfeitedPlayer = normalizePlayerIndex(command?.player);
    if (voter == null || forfeitedPlayer == null || Number(voter) === Number(forfeitedPlayer)) {
      throw new Error("Timed-out protocol responder cannot sign their own response-timeout forfeit");
    }
    const target = playerForProtocolResponseTimeout(forfeitedPlayer);
    const forfeitedPeerId = String(
      command?.timed_out_peer_id
      || command?.forfeited_peer_id
      || target?.peerId
      || ""
    ).trim();
    const responseTimeoutMs = Math.max(
      1,
      Math.floor(Number(command?.response_timeout_ms || PROTOCOL_RESPONSE_TIMEOUT_MS))
    );
    const requestedAtMs = Math.max(0, Math.floor(Number(command?.requested_at_ms || 0)));
    const eligibleAtMs = requestedAtMs + responseTimeoutMs;
    const signedAtMs = Date.now();
    if (!forfeitedPeerId) {
      throw new Error("Protocol response timeout is missing the timed-out peer id");
    }
    if (!target || String(target.peerId || "").trim() !== forfeitedPeerId) {
      throw new Error("Protocol response timeout peer id does not match the local player record");
    }
    if (!String(command?.request_type || "") || !String(command?.request_id || "")) {
      throw new Error("Protocol response timeout is missing request identity");
    }
    if (!String(command?.request_payload_hash || "")) {
      throw new Error("Protocol response timeout is missing request evidence");
    }
    if (signedAtMs + MATCH_CLOCK_CLAIM_SKEW_MS < eligibleAtMs) {
      throw new Error("Protocol response timeout has not elapsed");
    }
    return buildSignedProtocolResponseTimeoutVote({
      keyPair,
      matchId: currentAuditMatchId(),
      basisSequence: Number(command?.basis_sequence ?? multiplayerRef.current.lastAppliedSequence ?? 0),
      forfeitedPlayer,
      forfeitedPeerId,
      requestType: String(command.request_type || ""),
      requestId: String(command.request_id || ""),
      requestPayloadHash: String(command.request_payload_hash || ""),
      responseTimeoutMs,
      requestedAtMs,
      eligibleAtMs,
      signedAtMs,
      voter,
    });
  }

  async function validateProtocolResponseTimeoutCommand(command, options = {}) {
    if (!isProtocolResponseTimeoutForfeitCommand(command)) return;
    const forfeitedPlayer = normalizePlayerIndex(command.player);
    const actorIndex = normalizePlayerIndex(options.actorIndex);
    if (forfeitedPlayer == null) {
      throw new Error("Protocol response timeout targets an invalid player");
    }
    if (actorIndex != null && actorIndex === forfeitedPlayer) {
      throw new Error("Timed-out protocol responder cannot claim their own response-timeout forfeit");
    }
    const expectedBasisSequence = Number(multiplayerRef.current.lastAppliedSequence || 0);
    if (Number(command.basis_sequence ?? expectedBasisSequence) !== expectedBasisSequence) {
      throw new Error("Protocol response timeout is not based on the local transcript head");
    }
    const certificate = command.protocol_timeout_certificate || command.protocolTimeoutCertificate;
    const target = playerForProtocolResponseTimeout(forfeitedPlayer);
    const forfeitedPeerId = String(
      command.timed_out_peer_id
      || command.forfeited_peer_id
      || certificate?.forfeitedPeerId
      || target?.peerId
      || ""
    ).trim();
    const responseTimeoutMs = Math.max(
      1,
      Math.floor(Number(command.response_timeout_ms ?? certificate?.responseTimeoutMs ?? PROTOCOL_RESPONSE_TIMEOUT_MS))
    );
    const requestedAtMs = Math.max(
      0,
      Math.floor(Number(command.requested_at_ms ?? certificate?.requestedAtMs ?? 0))
    );
    const eligibleAtMs = requestedAtMs + responseTimeoutMs;
    if (!target) {
      throw new Error("Protocol response timeout target is missing from the local player record");
    }
    if (!forfeitedPeerId || String(target.peerId || "").trim() !== forfeitedPeerId) {
      throw new Error("Protocol response timeout peer id does not match the local player record");
    }
    if (!String(command.request_type || certificate?.requestType || "")) {
      throw new Error("Protocol response timeout is missing request type");
    }
    if (!String(command.request_id || certificate?.requestId || "")) {
      throw new Error("Protocol response timeout is missing request id");
    }
    if (!String(command.request_payload_hash || certificate?.requestPayloadHash || "")) {
      throw new Error("Protocol response timeout is missing request evidence");
    }
    if (Date.now() + MATCH_CLOCK_CLAIM_SKEW_MS < eligibleAtMs) {
      throw new Error("Protocol response timeout has not elapsed");
    }
    const roster = protocolResponseTimeoutRoster(forfeitedPlayer);
    if (options.skipCertificate) return;
    await verifyProtocolResponseTimeoutCertificate({
      certificate,
      command: {
        ...command,
        timed_out_peer_id: forfeitedPeerId,
        response_timeout_ms: responseTimeoutMs,
        matchId: currentAuditMatchId(),
        nowMs: Date.now(),
        maxFutureSkewMs: MATCH_CLOCK_CLAIM_SKEW_MS,
      },
      players: roster,
      threshold: protocolResponseTimeoutVoteThreshold(roster.length),
    });
  }

  async function validateTrustedSequencedAction({
    command,
    actorIndex,
    seq,
    clock,
    uiState,
    enforceMatchClockObservationBounds = true,
  }) {
    const normalizedActor = normalizePlayerIndex(actorIndex);
    if (normalizedActor == null) {
      throw new Error("Trusted action actor is not a match player");
    }
    const liveState = uiState || stateRef.current;
    if (isUnauthorizedAddCardCommand(command)) {
      const actorName = playerNameForIndex(multiplayerRef.current.players, normalizedActor);
      throw new Error(`Unauthorized add-card action from ${actorName}`);
    }
    const expectedActor = liveState?.decision?.player;
    const isTimeoutForfeit = isActionTimeoutForfeitCommand(command);
    const isDisconnectForfeit = isDisconnectTimeoutForfeitCommand(command);
    const isProtocolTimeoutForfeit = isProtocolResponseTimeoutForfeitCommand(command);
    const isSelfForfeit = isSelfForfeitCommand(command, normalizedActor);
    if (isSelfForfeit && !isSorcerySpeedForfeitState(liveState, normalizedActor)) {
      throw new Error("Surrender is only available at sorcery speed");
    }
    if (
      isForfeitCommand(command)
      && !isTimeoutForfeit
      && !isDisconnectForfeit
      && !isProtocolTimeoutForfeit
    ) {
      if (Number(command.player) !== Number(normalizedActor)) {
        throw new Error("A player can only forfeit themselves");
      }
    }
    if (isTimeoutForfeit) {
      await validateTimeoutForfeitCommand(command, liveState, {
        skewMs: MATCH_CLOCK_CLAIM_SKEW_MS,
        skipCertificate: true,
      });
    } else if (isDisconnectForfeit) {
      await validateDisconnectForfeitCommand(command, {
        actorIndex: normalizedActor,
        skipCertificate: true,
      });
    } else if (isProtocolTimeoutForfeit) {
      await validateProtocolResponseTimeoutCommand(command, {
        actorIndex: normalizedActor,
        skipCertificate: true,
      });
    } else {
      if (!isDecisionCommandCompatible(liveState?.decision, command)) {
        throw new Error("Trusted action is no longer available");
      }
      if (
        expectedActor !== null
        && expectedActor !== undefined
        && Number(expectedActor) !== Number(normalizedActor)
      ) {
        throw new Error("Sequenced action actor is not the current decision player");
      }
    }

    const skipMatchClockObservationBounds =
      Number(seq) === Number(matchClockObservationExemptSequenceRef.current || 0);
    await verifyMatchClockAuditForAction({
      clock,
      command,
      seq,
      actorIndex: normalizedActor,
      uiState: liveState,
      skewMs: Math.max(
        MATCH_CLOCK_CLAIM_SKEW_MS,
        MATCH_CLOCK_ELAPSED_UNDERREPORT_SKEW_MS
      ),
      enforceObservationBounds:
        enforceMatchClockObservationBounds
        && !skipMatchClockObservationBounds,
      enforceUnderreportBounds:
        enforceMatchClockObservationBounds
        && !skipMatchClockObservationBounds,
      skipTimeoutCertificate: true,
    });
  }

  async function collectProtocolResponseTimeoutCertificateForCommand(command) {
    if (!isProtocolResponseTimeoutForfeitCommand(command)) return null;
    const forfeitedPlayer = normalizePlayerIndex(command.player);
    if (forfeitedPlayer == null) {
      throw new Error("Protocol response timeout targets an invalid player");
    }
    const roster = protocolResponseTimeoutRoster(forfeitedPlayer);
    const threshold = protocolResponseTimeoutVoteThreshold(roster.length);
    if (threshold <= 0) return null;

    const target = playerForProtocolResponseTimeout(forfeitedPlayer);
    const expected = {
      matchId: currentAuditMatchId(),
      basisSequence: Number(command.basis_sequence ?? multiplayerRef.current.lastAppliedSequence ?? 0),
      forfeitedPlayer,
      forfeitedPeerId: String(command.timed_out_peer_id || command.forfeited_peer_id || target?.peerId || ""),
      requestType: String(command.request_type || ""),
      requestId: String(command.request_id || ""),
      requestPayloadHash: String(command.request_payload_hash || ""),
      responseTimeoutMs: Math.max(
        1,
        Math.floor(Number(command.response_timeout_ms || PROTOCOL_RESPONSE_TIMEOUT_MS))
      ),
      requestedAtMs: Math.max(0, Math.floor(Number(command.requested_at_ms || 0))),
      nowMs: Date.now(),
      maxFutureSkewMs: MATCH_CLOCK_CLAIM_SKEW_MS,
    };
    const votes = [];
    const seen = new Set();
    const addVote = async (vote) => {
      const voter = await verifyProtocolResponseTimeoutVote({
        vote,
        expected: {
          ...expected,
          nowMs: Date.now(),
        },
        players: roster,
      });
      if (seen.has(voter)) return;
      seen.add(voter);
      votes.push(cloneMultiplayerPayload(vote));
    };

    const localPlayer = resolveLocalPlayerIndex(multiplayerRef.current);
    if (roster.some((player) => Number(player.index) === Number(localPlayer))) {
      await addVote(await signProtocolResponseTimeoutVoteForCommand({
        ...command,
        timed_out_peer_id: expected.forfeitedPeerId,
        response_timeout_ms: expected.responseTimeoutMs,
      }));
    }

    if (votes.length < threshold) {
      ensureDirectPeerConnections(roster);
      const pendingWaitRequestIds = [];
      let collectingVotes = true;
      const pending = roster
        .filter((player) => Number(player.index) !== Number(localPlayer))
        .map(async (player) => {
          const routePeerId = routePeerIdForPlayer(player);
          if (!routePeerId) {
            throw new Error(`Missing peer route for protocol-timeout voter ${Number(player.index) + 1}`);
          }
          const conn = await waitForZiffleRoute(routePeerId);
          if (!collectingVotes) {
            throw new Error("Protocol-timeout vote collection is complete");
          }
          const requestId = makeZiffleRequestId("protocol-timeout-vote");
          pendingWaitRequestIds.push(requestId);
          const voterLabel = player.name || `Player ${Number(player.index) + 1}`;
          const waiter = waitForTimeoutVote(requestId, PROTOCOL_RESPONSE_TIMEOUT_VOTE_WAIT_MS, {
            peerIndex: Number(player.index),
            peerName: voterLabel,
            description:
              `${voterLabel} must sign the protocol-timeout vote before this claim can be submitted.`,
          });
          const { requestId: timedOutRequestId, ...voteRequestExpected } = expected;
          safeSend(conn, {
            type: "protocol_timeout_vote_request",
            protocolVersion: PROTOCOL_VERSION,
            requestId,
            ...voteRequestExpected,
            timedOutRequestId,
            requesterIndex: localPlayer,
          });
          return waiter;
        });
      const unsettled = new Set(pending);
      while (votes.length < threshold && unsettled.size > 0) {
        const settled = await Promise.race([...unsettled].map((promise) =>
          promise.then(
            (vote) => ({ promise, vote }),
            (err) => ({ promise, err })
          )
        ));
        unsettled.delete(settled.promise);
        if (settled.err) {
          if (protocolResponseTimeoutClaimFromError(settled.err)) throw settled.err;
          continue;
        }
        await addVote(settled.vote);
      }
      collectingVotes = false;
      for (const promise of unsettled) {
        promise.catch(() => {});
      }
      for (const requestId of pendingWaitRequestIds) {
        clearPeerWait(requestId);
      }
    }

    if (votes.length < threshold) {
      throw new Error(
        `Protocol response timeout certificate has ${votes.length} vote(s), expected at least ${threshold}`
      );
    }
    votes.sort((left, right) => Number(left.voter) - Number(right.voter));
    return {
      type: "ironsmith-protocol-response-timeout-v1",
      matchId: expected.matchId,
      basisSequence: expected.basisSequence,
      forfeitedPlayer: expected.forfeitedPlayer,
      forfeitedPeerId: expected.forfeitedPeerId,
      requestType: expected.requestType,
      requestId: expected.requestId,
      requestPayloadHash: expected.requestPayloadHash,
      responseTimeoutMs: expected.responseTimeoutMs,
      requestedAtMs: expected.requestedAtMs,
      eligibleAtMs: expected.requestedAtMs + expected.responseTimeoutMs,
      threshold,
      voters: votes.map((vote) => Number(vote.voter)),
      votes,
    };
  }

  async function answerProtocolResponseTimeoutVoteRequest(conn, message) {
    try {
      const requester = playerIndexForPeerId(conn?.peer);
      const localVoter = resolveLocalPlayerIndex(multiplayerRef.current);
      const forfeitedPlayer = normalizePlayerIndex(message?.forfeitedPlayer);
      const basisSequence = Number(message?.basisSequence);
      if (requester == null) {
        throw new Error("Protocol-timeout vote requester is not a match player");
      }
      if (normalizePlayerIndex(message?.requesterIndex) !== requester) {
        throw new Error("Protocol-timeout requester index does not match the peer");
      }
      if (forfeitedPlayer == null) {
        throw new Error("Protocol-timeout vote request targets an invalid player");
      }
      if (localVoter == null || Number(localVoter) === Number(forfeitedPlayer)) {
        throw new Error("Timed-out protocol responder cannot vote on their own forfeit");
      }
      if (String(message?.matchId || "") !== currentAuditMatchId()) {
        throw new Error("Protocol-timeout vote request belongs to a different match");
      }
      if (basisSequence !== Number(multiplayerRef.current.lastAppliedSequence || 0)) {
        throw new Error("Protocol-timeout vote request is not based on the local transcript head");
      }
      const vote = await signProtocolResponseTimeoutVoteForCommand({
        type: "forfeit_player",
        player: forfeitedPlayer,
        reason: PROTOCOL_RESPONSE_TIMEOUT_REASON,
        basis_sequence: basisSequence,
        timed_out_peer_id: String(message?.forfeitedPeerId || ""),
        request_type: String(message?.requestType || ""),
        request_id: String(message?.timedOutRequestId || message?.originalRequestId || ""),
        request_payload_hash: String(message?.requestPayloadHash || ""),
        response_timeout_ms: Number(message?.responseTimeoutMs || PROTOCOL_RESPONSE_TIMEOUT_MS),
        requested_at_ms: Number(message?.requestedAtMs || 0),
      });
      safeSend(conn, {
        type: "protocol_timeout_vote_response",
        protocolVersion: PROTOCOL_VERSION,
        requestId: message.requestId,
        vote,
      });
    } catch (err) {
      safeSend(conn, {
        type: "protocol_timeout_vote_response",
        protocolVersion: PROTOCOL_VERSION,
        requestId: message.requestId,
        error: toErrorMessage(err),
      });
    }
  }

  function actionQuorumThresholdForMessage(message, players = actionQuorumRoster(message)) {
    const forfeited = forfeitedPlayersForQuorum();
    const activePlayerCount = reindexPlayers(
      matchStartPayloadRef.current?.players || multiplayerRef.current.players || []
    )
      .filter((player) => !forfeited.has(Number(player.index)))
      .length;
    if (activePlayerCount < 3) return 0;
    if (isForfeitCommand(message?.command)) {
      const target = Number(message.command.player);
      if (Number(message?.actorIndex) === target) return 0;
      if (isDisconnectTimeoutForfeitCommand(message.command)) return 0;
      if (isProtocolResponseTimeoutForfeitCommand(message.command)) return 0;
      return players.length;
    }
    return actionQuorumThreshold(players.length);
  }

  function actionQuorumVoteCacheKey(vote) {
    return [
      String(vote?.matchId || ""),
      Number(vote?.seq || 0),
      Number(vote?.voter),
    ].join(":");
  }

  function actionQuorumVoteConflict(left, right) {
    if (!left || !right) return false;
    return (
      String(left.prevStateHash || "") !== String(right.prevStateHash || "")
      || String(left.nextStateHash || "") !== String(right.nextStateHash || "")
      || String(left.publicCheckpointHash || "") !== String(right.publicCheckpointHash || "")
      || String(left.actionSignature || "") !== String(right.actionSignature || "")
      || Number(left.actor) !== Number(right.actor)
    );
  }

  function rememberSignedActionQuorumVote(vote) {
    const key = actionQuorumVoteCacheKey(vote);
    const existing =
      signedActionQuorumVotesRef.current.get(key)
      || readStoredActionQuorumVote(vote?.matchId, vote?.seq, vote?.voter);
    if (existing && actionQuorumVoteConflict(existing, vote)) {
      throw new Error("Refusing to sign conflicting action quorum votes for the same sequence");
    }
    signedActionQuorumVotesRef.current.set(key, cloneMultiplayerPayload(vote));
    writeStoredActionQuorumVote(vote);
    return vote;
  }

  async function signActionQuorumVoteForMessage(message) {
    const localVoter = resolveLocalPlayerIndex(multiplayerRef.current);
    if (localVoter == null) {
      throw new Error("Local player cannot sign an action quorum vote");
    }
    const stored =
      signedActionQuorumVotesRef.current.get([
        String(message?.audit?.matchId || currentAuditMatchId()),
        Number(message?.audit?.seq || message?.seq || 0),
        Number(localVoter),
      ].join(":"))
      || readStoredActionQuorumVote(
        message?.audit?.matchId || currentAuditMatchId(),
        message?.audit?.seq || message?.seq,
        localVoter
      );
    const expectedVote = {
      matchId: String(message?.audit?.matchId || ""),
      seq: Number(message?.audit?.seq || message?.seq || 0),
      actor: Number(message?.audit?.actor ?? message?.actorIndex ?? 0),
      voter: Number(localVoter),
      prevStateHash: String(message?.audit?.prevStateHash || ""),
      nextStateHash: String(message?.audit?.nextStateHash || ""),
      publicCheckpointHash: String(message?.audit?.publicCheckpointHash || ""),
      actionSignature: String(message?.audit?.signature || ""),
    };
    if (stored) {
      if (actionQuorumVoteConflict(stored, expectedVote)) {
        throw new Error("Refusing to sign conflicting action quorum votes for the same sequence");
      }
      return stored;
    }
    const { keyPair } = await ensureAuditIdentity();
    return rememberSignedActionQuorumVote(await buildSignedActionQuorumVote({
      keyPair,
      action: message,
      voter: localVoter,
    }));
  }

  async function verifyActionQuorumForMessage(message) {
    const players = actionQuorumRoster(message);
    const threshold = actionQuorumThresholdForMessage(message, players);
    if (threshold <= 0) return;
    await verifyActionQuorumCertificate({
      certificate: message?.audit?.quorumCertificate || message?.quorumCertificate,
      action: message,
      players,
      threshold,
    });
  }

  async function verifyActionQuorumVoteForMessage(vote, message) {
    return verifyActionQuorumVote({
      vote,
      action: message,
      players: actionQuorumRoster(message),
    });
  }

  async function collectActionQuorumCertificate(message) {
    const players = actionQuorumRoster(message);
    const threshold = actionQuorumThresholdForMessage(message, players);
    if (threshold <= 0) return null;

    const votes = [];
    const seen = new Set();
    const addVote = async (vote) => {
      const voter = await verifyActionQuorumVoteForMessage(vote, message);
      if (seen.has(voter)) return;
      seen.add(voter);
      votes.push(cloneMultiplayerPayload(vote));
    };

    await addVote(await signActionQuorumVoteForMessage(message));
    if (votes.length < threshold) {
      ensureDirectPeerConnections(players);
      const localPlayer = resolveLocalPlayerIndex(multiplayerRef.current);
      const pendingWaitRequestIds = [];
      let collectingVotes = true;
      const pending = players
        .filter((player) => Number(player.index) !== Number(localPlayer))
        .map(async (player) => {
          const routePeerId = routePeerIdForPlayer(player);
          if (!routePeerId) {
            throw new Error(`Missing peer route for quorum voter ${Number(player.index) + 1}`);
          }
          const conn = await waitForZiffleRoute(routePeerId);
          if (!collectingVotes) {
            throw new Error("Action quorum vote collection is complete");
          }
          const requestId = makeZiffleRequestId("action-quorum");
          const requestedAtMs = Date.now();
          pendingWaitRequestIds.push(requestId);
          const voterLabel = player.name || `Player ${Number(player.index) + 1}`;
          const waiter = waitForActionQuorumVote(requestId, PROTOCOL_RESPONSE_TIMEOUT_MS, {
            peerIndex: Number(player.index),
            peerName: voterLabel,
            description:
              `${voterLabel} is verifying and signing the action payload.`,
          });
          const requestPayload = {
            type: "action_quorum_vote_request",
            protocolVersion: PROTOCOL_VERSION,
            requestId,
            requesterIndex: localPlayer,
            action: cloneMultiplayerPayload(message),
          };
          const quorumPerf = {
            request_id: requestId,
            voter: Number(player.index),
            requester: localPlayer,
            action: summarizeSequencedActionForPerf(message),
            request_bytes: payloadSizeBytes(requestPayload),
          };
          recordPeerSyncPerf("action_quorum:send_vote_request", quorumPerf);
          safeSend(conn, requestPayload);
          return timePeerSyncPhase(
            "action_quorum:wait_vote_response",
            quorumPerf,
            () => waitForProtocolResponse(waiter, {
              basisSequence: Number(multiplayerRef.current.lastAppliedSequence || 0),
              targetPlayerIndex: Number(player.index),
              targetPeerId: player.peerId,
              requesterIndex: localPlayer,
              requestType: requestPayload.type,
              requestId,
              requestPayload,
              responseTimeoutMs: PROTOCOL_RESPONSE_TIMEOUT_MS,
              requestedAtMs,
            })
          );
        });
      const unsettled = new Set(pending);
      while (votes.length < threshold && unsettled.size > 0) {
        const settled = await Promise.race([...unsettled].map((promise) =>
          promise.then(
            (vote) => ({ promise, vote }),
            (err) => ({ promise, err })
          )
        ));
        unsettled.delete(settled.promise);
        if (settled.err) {
          if (protocolResponseTimeoutClaimFromError(settled.err)) throw settled.err;
          continue;
        }
        await addVote(settled.vote);
      }
      collectingVotes = false;
      for (const promise of unsettled) {
        promise.catch(() => {});
      }
      for (const requestId of pendingWaitRequestIds) {
        clearPeerWait(requestId);
      }
    }

    if (votes.length < threshold) {
      throw new Error(
        `Action quorum certificate has ${votes.length} vote(s), expected at least ${threshold}`
      );
    }
    votes.sort((left, right) => Number(left.voter) - Number(right.voter));
    const audit = message.audit || {};
    return {
      type: "ironsmith-action-quorum-v1",
      matchId: String(audit.matchId || ""),
      seq: Number(audit.seq || message.seq || 0),
      actor: Number(audit.actor ?? message.actorIndex ?? 0),
      prevStateHash: String(audit.prevStateHash || ""),
      nextStateHash: String(audit.nextStateHash || ""),
      publicCheckpointHash: String(audit.publicCheckpointHash || ""),
      actionSignature: String(audit.signature || ""),
      threshold,
      voters: votes.map((vote) => Number(vote.voter)),
      votes,
    };
  }

  async function answerActionQuorumVoteRequest(conn, message) {
    const quorumPerf = {
      request_id: String(message?.requestId || ""),
      requester: message?.requesterIndex == null ? null : Number(message.requesterIndex),
      action: summarizeSequencedActionForPerf(message?.action),
      request_bytes: payloadSizeBytes(message),
    };
    recordPeerSyncPerf("action_quorum:received_vote_request", quorumPerf);
    try {
      const requester = playerIndexForPeerId(conn?.peer);
      if (requester == null) {
        throw new Error("Action quorum requester is not a match player");
      }
      if (normalizePlayerIndex(message?.requesterIndex) !== requester) {
        throw new Error("Action quorum requester index does not match the peer");
      }
      const action = message?.action;
      if (!action || action.type !== "apply_action") {
        throw new Error("Action quorum request is missing an action");
      }
      await timePeerSyncPhase(
        "action_quorum:dry_run_apply_action",
        quorumPerf,
        () => applySequencedActionMessage(action, {
          relay: false,
          dryRun: true,
          skipQuorumCertificate: true,
          throwOnOrderMismatch: true,
        })
      );
      const requestPayload = cloneMultiplayerPayload(message);
      await refreshPendingActionIntentEvidenceForAction(action, {
        requestType: "action_quorum_vote_request",
        requestId: String(message.requestId || ""),
        requestPayload,
        requestPayloadHash: await sha256Hex(canonicalMultiplayerPayload(requestPayload)),
        responseTimeoutMs: PROTOCOL_RESPONSE_TIMEOUT_MS,
        requestedAtMs: Date.now(),
      });
      const vote = await timePeerSyncPhase(
        "action_quorum:sign_vote",
        quorumPerf,
        () => signActionQuorumVoteForMessage(action)
      );
      recordPeerSyncPerf("action_quorum:send_vote_response", quorumPerf);
      safeSend(conn, {
        type: "action_quorum_vote_response",
        protocolVersion: PROTOCOL_VERSION,
        requestId: message.requestId,
        vote,
      });
    } catch (err) {
      const failureReason = toErrorMessage(err);
      recordPeerSyncPerf("action_quorum:error_response", {
        ...quorumPerf,
        error: failureReason,
      });
      if (message?.action && isRejectedActionCheatReason(failureReason)) {
        const actorName = playerNameForIndex(
          multiplayerRef.current.players,
          message.action.actorIndex
        );
        const status = `Cheat detected from ${actorName}: ${failureReason}`;
        emitSyncFailureNotice("Cheat detected", status);
        setStatus(status, true);
      }
      safeSend(conn, {
        type: "action_quorum_vote_response",
        protocolVersion: PROTOCOL_VERSION,
        requestId: message.requestId,
        error: failureReason,
      });
    }
  }

  function actionHistoryEntryForSequence(seq) {
    return (actionHistoryRef.current || []).find(
      (entry) => Number(entry?.seq) === Number(seq)
    ) || null;
  }

  function rememberActionCryptoRequirements(seq, requirements = []) {
    const sequence = Number(seq);
    if (!Number.isSafeInteger(sequence) || sequence <= 0) return;
    const merged = new Map();
    for (const requirement of [
      ...(actionCryptoRequirementsRef.current.get(sequence) || []),
      ...(requirements || []),
    ]) {
      if (!requirement || typeof requirement !== "object") continue;
      const key = String(requirement.id || canonicalMultiplayerPayload(requirement));
      if (!merged.has(key)) {
        merged.set(key, cloneMultiplayerPayload(requirement));
      }
    }
    actionCryptoRequirementsRef.current.set(sequence, [...merged.values()]);
    const minRetainedSequence = Math.max(1, sequence - 64);
    for (const existingSeq of actionCryptoRequirementsRef.current.keys()) {
      if (Number(existingSeq) < minRetainedSequence) {
        actionCryptoRequirementsRef.current.delete(existingSeq);
      }
    }
  }

  function cryptoRequirementReplayKey(requirement) {
    if (!requirement || typeof requirement !== "object") return "";
    return canonicalMultiplayerPayload({
      id: requirement.id ?? null,
      type: requirement.type ?? requirement.requirement_type ?? null,
      owner: requirement.owner ?? null,
      viewer: requirement.viewer ?? null,
      zone: requirement.zone ?? null,
      slot: requirement.slot ?? null,
      objectId: requirement.objectId ?? requirement.object_id ?? null,
      commitment: requirement.commitment ?? null,
      count: requirement.count ?? null,
      from: requirement.from ?? null,
      to: requirement.to ?? null,
      beforeOrder: normalizeShuffleOrder(requirement.beforeOrder ?? requirement.before_order),
      afterOrder: normalizeShuffleOrder(requirement.afterOrder ?? requirement.after_order),
      randomCountBefore: requirement.randomCountBefore ?? requirement.random_count_before ?? null,
      randomCountAfter: requirement.randomCountAfter ?? requirement.random_count_after ?? null,
      visibility: requirement.visibility ?? null,
      reason: requirement.reason ?? null,
    });
  }

  function freshCryptoRequirementsForSequence(seq, requirements = []) {
    const sequence = Number(seq);
    if (!Number.isSafeInteger(sequence) || sequence <= 0) {
      return Array.isArray(requirements) ? requirements : [];
    }
    const previous = new Set();
    for (const [existingSeq, existingRequirements] of actionCryptoRequirementsRef.current.entries()) {
      if (Number(existingSeq) >= sequence) continue;
      for (const requirement of existingRequirements || []) {
        const key = cryptoRequirementReplayKey(requirement);
        if (key) previous.add(key);
      }
    }
    return (Array.isArray(requirements) ? requirements : []).filter((requirement) => {
      const key = cryptoRequirementReplayKey(requirement);
      return !key || !previous.has(key);
    });
  }

  function shuffleProofReplayKey(proof) {
    if (!proof || typeof proof !== "object") return "";
    return canonicalMultiplayerPayload({
      requirementId: proof.requirementId ?? proof.requirement_id ?? null,
      owner: proof.owner ?? null,
      zone: proof.zone ?? "library",
      deckHash: proof.deckHash ?? proof.deck_hash ?? null,
      context: proof.context ?? null,
      keyContext: proof.keyContext ?? proof.key_context ?? null,
      beforeOrder: normalizeShuffleOrder(proof.beforeOrder ?? proof.before_order),
      afterOrder: normalizeShuffleOrder(proof.afterOrder ?? proof.after_order),
    });
  }

  function shuffleProofAlreadyAppliedBefore(seq, proof) {
    const sequence = Number(seq);
    const key = shuffleProofReplayKey(proof);
    if (!key || !Number.isSafeInteger(sequence) || sequence <= 0) return false;
    return (actionHistoryRef.current || []).some((entry) =>
      Number(entry?.seq) < sequence
      && (entry?.audit?.shuffleProofs || []).some((existing) =>
        shuffleProofReplayKey(existing) === key
      )
    );
  }

  function shuffleProofRequirementAlreadyRecordedBefore(seq, proof) {
    const sequence = Number(seq);
    if (!Number.isSafeInteger(sequence) || sequence <= 0) return false;
    const proofAfterOrder = normalizeShuffleOrder(proof?.afterOrder ?? proof?.after_order);
    return [...actionCryptoRequirementsRef.current.entries()].some(([existingSeq, requirements]) =>
      Number(existingSeq) < sequence
      && (requirements || []).some((requirement) =>
        String(requirement?.type || requirement?.requirement_type || "") === "verifiable_shuffle"
        && shuffleProofMatchesRequirement(proof, requirement)
        && sameShuffleOrder(
          proofAfterOrder,
          requirement.afterOrder ?? requirement.after_order
        )
      )
    );
  }

  function actionCryptoRequirementsForSequence(seq) {
    const requirements = actionCryptoRequirementsRef.current.get(Number(seq));
    return Array.isArray(requirements) ? requirements : [];
  }

  function stateHashBeforeSequence(seq) {
    const previousSeq = Number(seq) - 1;
    if (previousSeq <= 0) return INITIAL_AUDIT_STATE_HASH;
    const previous = actionHistoryEntryForSequence(previousSeq);
    return String(previous?.audit?.nextStateHash || "");
  }

  function sequencedActionsEquivalent(left, right) {
    if (!left || !right) return false;
    return (
      Number(left.seq || 0) === Number(right.seq || 0)
      && Number(left.actorIndex) === Number(right.actorIndex)
      && canonicalMultiplayerPayload(left.command) === canonicalMultiplayerPayload(right.command)
      && String(left.audit?.prevStateHash || "") === String(right.audit?.prevStateHash || "")
      && String(left.audit?.nextStateHash || "") === String(right.audit?.nextStateHash || "")
    );
  }

  function markMatchDisputed(reason, evidence = {}) {
    const body = String(reason || "Match transcript fork detected");
    const dispute = evidence?.dispute || null;
    ignoreAndClearAllPendingActionIntents("match_disputed");
    if (dispute && liveAuditTranscriptRef.current) {
      const existingDisputes = Array.isArray(liveAuditTranscriptRef.current.disputes)
        ? liveAuditTranscriptRef.current.disputes
        : [];
      liveAuditTranscriptRef.current = {
        ...liveAuditTranscriptRef.current,
        disputes: [
          ...existingDisputes,
          cloneMultiplayerPayload(dispute),
        ],
      };
    }
    emitSyncFailureNotice("Match disputed", body);
    timeoutClaimInFlightRef.current = "";
    updateMultiplayer((prev) => ({
      ...prev,
      mode: "disputed",
      matchStarted: false,
      submittingAction: false,
      matchDisputed: {
        reason: body,
        evidence: cloneMultiplayerPayload(evidence),
        accusedPlayers: Array.isArray(dispute?.accusedPlayers)
          ? dispute.accusedPlayers.map(Number)
          : [],
        at: Date.now(),
      },
    }));
    setStatus(body, true);
  }

  async function handleHistoricalSequencedAction(message) {
    const seq = Number(message?.seq || 0);
    const existing = actionHistoryEntryForSequence(seq);
    if (!existing) return true;
    if (sequencedActionsEquivalent(existing, message)) return true;

    const expectedPrevStateHash = stateHashBeforeSequence(seq);
    if (!expectedPrevStateHash) return true;
    try {
      await verifySequencedActionAudit({
        audit: message.audit,
        seq,
        actorIndex: message.actorIndex,
        command: message.command,
        expectedPrevStateHash,
      });
    } catch {
      return true;
    }

    const reason = `Match disputed: two different valid actions were signed for sequence ${seq}.`;
    const dispute = buildActionForkDisputeEvidence({
      sequence: seq,
      reason,
      existingAction: existing,
      conflictingAction: message,
    });
    markMatchDisputed(
      reason,
      {
        sequence: seq,
        accusedPlayers: dispute.accusedPlayers,
        dispute,
      }
    );
    return true;
  }

  async function appendAppliedSequencedAction(message) {
    const nextSequence = Number(message.seq || 0);
    actionHistoryRef.current = [
      ...actionHistoryRef.current,
      {
        seq: nextSequence,
        actorIndex: Number(message.actorIndex),
        command: cloneMultiplayerPayload(message.command),
        label: String(message.label || ""),
        securityMode: normalizeMultiplayerSecurityMode(
          message.securityMode,
          message.audit ? MULTIPLAYER_SECURITY_VERIFIED : MULTIPLAYER_SECURITY_TRUSTED
        ),
        clock: cloneMultiplayerPayload(message.clock),
        audit: cloneMultiplayerPayload(message.audit),
      },
    ];
    if (liveAuditTranscriptRef.current) {
      liveAuditTranscriptRef.current = {
        ...liveAuditTranscriptRef.current,
        actions: actionHistoryRef.current.map((entry) =>
          cloneMultiplayerPayload(entry)
        ),
      };
    }
    clearPendingActionIntent({
      matchId: message.audit?.matchId || currentAuditMatchId(),
      seq: nextSequence,
      actorIndex: message.audit?.actor ?? message.actorIndex,
    });
    if (message.audit?.nextStateHash) {
      auditStateHashRef.current = message.audit.nextStateHash;
    }
    updateMultiplayer((prev) => ({
      ...prev,
      lastAppliedSequence: nextSequence,
      submittingAction: false,
    }));
  }

  async function publishCurrentRuntimeState(stateHint = null) {
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.uiState !== "function") return null;
    const localIndex = resolveLocalPlayerIndex(multiplayerRef.current);
    if (
      localIndex != null
      && typeof currentGame.setPerspective === "function"
    ) {
      try {
        await currentGame.setPerspective(localIndex);
      } catch {
        // Keep the state publish best-effort; the engine should already have
        // the correct perspective during normal multiplayer action application.
      }
    }
    const nextState = await preserveViewedCardsFromHint(
      await currentGame.uiState(),
      stateHint,
      currentGame,
    );
    stateRef.current = nextState;
    setState(nextState);
    return nextState;
  }

  async function createSequencedActionValidationSnapshot() {
    const currentGame = gameRef.current;
    if (
      !currentGame
      || typeof currentGame.exportSyncCheckpoint !== "function"
      || typeof currentGame.importSyncCheckpoint !== "function"
    ) {
      throw new Error("Game engine cannot sandbox action quorum validation");
    }
    return {
      checkpoint: await currentGame.exportSyncCheckpoint(),
      state: cloneMultiplayerPayload(stateRef.current),
      actionHistory: actionHistoryRef.current.map((entry) => cloneMultiplayerPayload(entry)),
      liveAuditTranscript: liveAuditTranscriptRef.current
        ? cloneMultiplayerPayload(liveAuditTranscriptRef.current)
        : null,
      matchStartPayload: matchStartPayloadRef.current
        ? cloneMultiplayerPayload(matchStartPayloadRef.current)
        : null,
      auditStateHash: auditStateHashRef.current,
      initialPublicCheckpointHash: initialPublicCheckpointHashRef.current,
      lastAppliedSequence: Number(multiplayerRef.current.lastAppliedSequence || 0),
      matchClock: cloneMultiplayerPayload(matchClockRef.current),
      matchClockConfig: cloneMultiplayerPayload(matchClockConfigRef.current),
      actionCryptoRequirements: new Map(
        [...actionCryptoRequirementsRef.current.entries()].map(([seq, requirements]) => [
          seq,
          cloneMultiplayerPayload(requirements),
        ])
      ),
	      relayedActionIds: [...relayedActionIdsRef.current],
	      ziffleHandRevealKey: ziffleHandRevealKeyRef.current,
	      ziffleHandRevealQuickKey: ziffleHandRevealQuickKeyRef.current,
	    };
  }

  async function restoreSequencedActionValidationSnapshot(snapshot) {
    if (!snapshot) return;
    const currentGame = gameRef.current;
    const localPlayer = resolveLocalPlayerIndex(multiplayerRef.current);
    if (
      currentGame
      && snapshot.checkpoint
      && typeof currentGame.importSyncCheckpoint === "function"
    ) {
      await currentGame.importSyncCheckpoint(
        snapshot.checkpoint,
        localPlayer ?? multiplayerRef.current.localPlayerIndex ?? 0
      );
    }
    actionHistoryRef.current = snapshot.actionHistory.map((entry) => cloneMultiplayerPayload(entry));
    liveAuditTranscriptRef.current = snapshot.liveAuditTranscript
      ? cloneMultiplayerPayload(snapshot.liveAuditTranscript)
      : null;
    matchStartPayloadRef.current = snapshot.matchStartPayload
      ? cloneMultiplayerPayload(snapshot.matchStartPayload)
      : null;
    auditStateHashRef.current = snapshot.auditStateHash;
    initialPublicCheckpointHashRef.current = snapshot.initialPublicCheckpointHash || "";
    matchClockConfigRef.current = cloneMultiplayerPayload(snapshot.matchClockConfig);
    matchClockRef.current = cloneMultiplayerPayload(snapshot.matchClock);
    actionCryptoRequirementsRef.current = new Map(
      [...snapshot.actionCryptoRequirements.entries()].map(([seq, requirements]) => [
        seq,
        cloneMultiplayerPayload(requirements),
      ])
    );
	    relayedActionIdsRef.current = new Set(snapshot.relayedActionIds || []);
	    ziffleHandRevealKeyRef.current = snapshot.ziffleHandRevealKey;
	    ziffleHandRevealQuickKeyRef.current = snapshot.ziffleHandRevealQuickKey || "";
    const restoredState = currentGame && typeof currentGame.uiState === "function"
      ? await currentGame.uiState()
      : cloneMultiplayerPayload(snapshot.state);
    stateRef.current = restoredState;
    setState(restoredState);
    publishMatchClockSnapshot(runtimeMatchClockSnapshot());
    updateMultiplayer((prev) => ({
      ...prev,
      lastAppliedSequence: snapshot.lastAppliedSequence,
      submittingAction: false,
    }));
  }

  function sequencedActionValidationSnapshotStillCurrent(snapshot) {
    if (!snapshot) return false;
    const snapshotSequence = Number(snapshot.lastAppliedSequence || 0);
    const currentSequence = Number(multiplayerRef.current.lastAppliedSequence || 0);
    const currentHistorySequence = Number(
      actionHistoryRef.current.at(-1)?.seq ?? currentSequence
    );
    return (
      currentSequence === snapshotSequence
      && currentHistorySequence === snapshotSequence
    );
  }

  async function restoreSequencedActionValidationSnapshotIfCurrent(snapshot) {
    if (!sequencedActionValidationSnapshotStillCurrent(snapshot)) {
      recordPeerSyncPerf("validation_snapshot:skip_restore", {
        snapshot_sequence: Number(snapshot?.lastAppliedSequence || 0),
        current_sequence: Number(multiplayerRef.current.lastAppliedSequence || 0),
        current_history_sequence: Number(
          actionHistoryRef.current.at(-1)?.seq ?? multiplayerRef.current.lastAppliedSequence ?? 0
        ),
      });
      return false;
    }
    await restoreSequencedActionValidationSnapshot(snapshot);
    return true;
  }

  function bufferedSequencedActionOptions(options = {}) {
    return {
      ...(options.relay === false ? { relay: false } : {}),
      ...(options.skipQuorumCertificate === true ? { skipQuorumCertificate: true } : {}),
      ...(options.enforceMatchClockObservationBounds === false
        ? { enforceMatchClockObservationBounds: false }
        : {}),
      ...(options.failureResyncReason ? { failureResyncReason: options.failureResyncReason } : {}),
      ...(options.resyncReason ? { resyncReason: options.resyncReason } : {}),
      ...(options.allowWhileAwaitingResync ? { allowWhileAwaitingResync: true } : {}),
    };
  }

  function bufferFutureSequencedAction(message, options = {}) {
    const seq = Number(message?.seq || 0);
    if (!Number.isSafeInteger(seq) || seq <= 0) return false;
    const existing = pendingSequencedActionsRef.current.get(seq);
    if (existing) {
      if (sequencedActionsEquivalent(existing.message, message)) {
        return true;
      }
      return false;
    }
    pendingSequencedActionsRef.current.set(seq, {
      message: cloneMultiplayerPayload(message),
      options: bufferedSequencedActionOptions(options),
      receivedAtMs: Date.now(),
    });
    recordPeerSyncPerf("apply_action:buffer_future", {
      seq,
      expected: Number(multiplayerRef.current.lastAppliedSequence || 0) + 1,
      actor: Number(message?.actorIndex ?? message?.audit?.actor ?? -1),
      pending: pendingSequencedActionsRef.current.size,
    });
    return true;
  }

  async function drainPendingSequencedActions() {
    if (drainingPendingSequencedActionsRef.current) return;
    drainingPendingSequencedActionsRef.current = true;
    try {
      while (!awaitingStateResyncRef.current) {
        const nextSequence = Number(multiplayerRef.current.lastAppliedSequence || 0) + 1;
        const pending = pendingSequencedActionsRef.current.get(nextSequence);
        if (!pending) break;
        pendingSequencedActionsRef.current.delete(nextSequence);
        recordPeerSyncPerf("apply_action:drain_future", {
          seq: nextSequence,
          pending: pendingSequencedActionsRef.current.size,
        });
        await applySequencedActionMessage(pending.message, {
          ...pending.options,
          fromBufferedSequencedAction: true,
        });
      }
    } finally {
      drainingPendingSequencedActionsRef.current = false;
    }
  }

  async function applySequencedActionMessage(message, options = {}) {
    const nextSequence = Number(message?.seq || 0);
    if (
      !options.dryRun
      && Number.isSafeInteger(nextSequence)
      && nextSequence > 0
    ) {
      const inFlight = applyingSequencedActionsRef.current.get(nextSequence);
      if (inFlight) {
        try {
          await inFlight.promise;
        } catch (err) {
          if (options.throwOnFailure) {
            throw err;
          }
          return { duplicate: true, coalesced: true, failed: true };
        }
        const existingAction = actionHistoryEntryForSequence(nextSequence);
        if (existingAction && sequencedActionsEquivalent(existingAction, message)) {
          if (options.relay !== false) {
            relaySequencedAction(message);
          }
          return { duplicate: true, coalesced: true };
        }
      }
      const promise = applySequencedActionMessageInner(message, options);
      applyingSequencedActionsRef.current.set(nextSequence, {
        message: cloneMultiplayerPayload(message),
        promise,
      });
      try {
        return await promise;
      } finally {
        const current = applyingSequencedActionsRef.current.get(nextSequence);
        if (current?.promise === promise) {
          applyingSequencedActionsRef.current.delete(nextSequence);
        }
      }
    }
    return applySequencedActionMessageInner(message, options);
  }

  async function applySequencedActionMessageInner(message, options = {}) {
    const nextSequence = Number(message?.seq || 0);
    const session = multiplayerRef.current;
    const dryRun = Boolean(options.dryRun);
    const throwOnFailure = Boolean(options.throwOnFailure);
    const existingAction = actionHistoryEntryForSequence(nextSequence);
    if (existingAction && sequencedActionsEquivalent(existingAction, message)) {
      if (options.relay !== false) {
        relaySequencedAction(message);
      }
      return { duplicate: true };
    }
    if (awaitingStateResyncRef.current && !options.allowWhileAwaitingResync) {
      if (dryRun || options.throwOnOrderMismatch || throwOnFailure) {
        throw new Error("Cannot validate action while awaiting state resync");
      }
      return;
    }
    if (nextSequence <= session.lastAppliedSequence) {
      if (dryRun || options.throwOnOrderMismatch || throwOnFailure) {
        throw new Error("Action quorum request is not ahead of the local transcript");
      }
      await handleHistoricalSequencedAction(message);
      return;
    }
    if (nextSequence !== session.lastAppliedSequence + 1) {
      if (
        !dryRun
        && !options.throwOnOrderMismatch
        && !throwOnFailure
        && nextSequence > session.lastAppliedSequence + 1
        && bufferFutureSequencedAction(message, options)
      ) {
        setStatus(`Waiting for action ${session.lastAppliedSequence + 1}`);
        return { buffered: true };
      }
      if (dryRun || options.throwOnOrderMismatch || throwOnFailure) {
        throw new Error(
          `Action order mismatch. Expected ${session.lastAppliedSequence + 1}, received ${nextSequence}.`
        );
      }
      reportSyncFailure(
        `Action order mismatch. Expected ${session.lastAppliedSequence + 1}, received ${nextSequence}.`,
        options.resyncReason || "Multiplayer action order mismatch. Resyncing with host...",
        "Multiplayer action order mismatch"
      );
      return;
    }

    const validationSnapshot = dryRun
      ? await createSequencedActionValidationSnapshot()
      : null;
    let snapshotRestored = false;
    const restoreValidationSnapshot = async () => {
      if (!validationSnapshot || snapshotRestored) return;
      snapshotRestored = await restoreSequencedActionValidationSnapshotIfCurrent(validationSnapshot);
      if (!snapshotRestored) {
        throw new Error("Action validation snapshot was superseded by a newer action");
      }
    };

    if (!dryRun) {
      updateMultiplayer((prev) => ({ ...prev, submittingAction: true }));
    }
    let applyPhase = "init";
    try {
      const localSecurityMode = sessionSecurityMode(
        session,
        matchPayloadSecurityMode(matchStartPayloadRef.current, MULTIPLAYER_SECURITY_VERIFIED)
      );
      const messageSecurityMode = sequencedActionSecurityMode(message, session);
      if (messageSecurityMode !== localSecurityMode) {
        throw new Error(
          `Sequenced action security mode mismatch. Expected ${localSecurityMode}, received ${messageSecurityMode}.`
        );
      }
      if (isTrustedMultiplayerSecurityMode(localSecurityMode)) {
        applyPhase = "pre_apply_checks";
        const liveStateForClock = gameRef.current
          ? await gameRef.current.uiState()
          : stateRef.current;
        await validateTrustedSequencedAction({
          command: message.command,
          actorIndex: message.actorIndex,
          seq: nextSequence,
          clock: message.clock,
          uiState: liveStateForClock,
          enforceMatchClockObservationBounds:
            options.enforceMatchClockObservationBounds !== false,
        });
        applyPhase = "apply_command";
        const appliedState = await applySyncedCommand(message.command, message.label || "", {
          actorIndex: message.actorIndex,
          sequence: nextSequence,
          publishState: false,
        });
        if (dryRun) {
          await restoreValidationSnapshot();
          return { trusted: true };
        }
        commitMatchClockAudit(message.clock, appliedState);
        await appendAppliedSequencedAction({
          ...message,
          securityMode: MULTIPLAYER_SECURITY_TRUSTED,
        });
        if (Number(nextSequence) === Number(matchClockObservationExemptSequenceRef.current || 0)) {
          matchClockObservationExemptSequenceRef.current = 0;
        }
        if (options.relay !== false) {
          relaySequencedAction({
            ...message,
            securityMode: MULTIPLAYER_SECURITY_TRUSTED,
          });
        }
        await publishCurrentRuntimeState(appliedState);
        await drainPendingSequencedActions();
        return { trusted: true };
      }
      applyPhase = "verify_audit";
      await verifySequencedActionAudit({
        audit: message.audit,
        seq: nextSequence,
        actorIndex: message.actorIndex,
        command: message.command,
      });
      applyPhase = "verify_pending_intent";
      const pendingIntentVerification = await verifyActionMatchesPendingIntent(message);
      if (!options.skipQuorumCertificate) {
        applyPhase = "verify_quorum";
        await verifyActionQuorumForMessage(message);
      }
      applyPhase = "pre_apply_checks";
      const liveStateForClock = gameRef.current ? await gameRef.current.uiState() : stateRef.current;
      const auditSigner = Number(message.audit?.signer ?? message.actorIndex);
      if (auditSigner !== Number(message.actorIndex)) {
        throw new Error("Sequenced action must be signed by the acting player");
      }
      if (isUnauthorizedAddCardCommand(message.command)) {
        const actorName = playerNameForIndex(session.players, message.actorIndex);
        throw new Error(`Unauthorized add-card action from ${actorName}`);
      }
      const expectedActor = gameRef.current
        ? (await gameRef.current.uiState())?.decision?.player
        : null;
      const isTimeoutForfeit = isActionTimeoutForfeitCommand(message.command);
      const isDisconnectForfeit = isDisconnectTimeoutForfeitCommand(message.command);
      const isProtocolTimeoutForfeit = isProtocolResponseTimeoutForfeitCommand(message.command);
      const isSelfForfeit = isSelfForfeitCommand(message.command, message.actorIndex);
      if (isSelfForfeit && !isSorcerySpeedForfeitState(liveStateForClock, message.actorIndex)) {
        throw new Error("Surrender is only available at sorcery speed");
      }
      if (
        isForfeitCommand(message.command)
        && !isTimeoutForfeit
        && !isDisconnectForfeit
        && !isProtocolTimeoutForfeit
      ) {
        if (Number(message.command.player) !== Number(message.actorIndex)) {
          throw new Error("A player can only forfeit themselves");
        }
      }
      if (isTimeoutForfeit) {
        await validateTimeoutForfeitCommand(message.command, liveStateForClock, {
          skewMs: MATCH_CLOCK_CLAIM_SKEW_MS,
        });
      } else if (isDisconnectForfeit) {
        await validateDisconnectForfeitCommand(message.command, {
          actorIndex: message.actorIndex,
        });
      } else if (isProtocolTimeoutForfeit) {
        await validateProtocolResponseTimeoutCommand(message.command, {
          actorIndex: message.actorIndex,
        });
      } else if (
        expectedActor !== null
        && expectedActor !== undefined
        && Number(expectedActor) !== Number(message.actorIndex)
      ) {
        throw new Error("Sequenced action actor is not the current decision player");
      }
	      const skipMatchClockObservationBounds =
	        Number(nextSequence) === Number(matchClockObservationExemptSequenceRef.current || 0);
	      const actionCarriesCryptoMaterial = Boolean(
	        (message.audit?.openings || []).length
	        || (message.audit?.privateViewProofs || []).length
	        || (message.audit?.shuffleProofs || []).length
	        || (message.audit?.rngReveals || []).length
	      );
	      const actionWasDelayedByProtocolWork = Boolean(
	        pendingIntentVerification?.intentWasHeldForCryptoMaterial
	      );
	      if (!isUnauthorizedAddCardCommand(message.command)) {
	        await verifyMatchClockAuditForAction({
	          clock: message.audit?.clock,
          command: message.command,
          seq: nextSequence,
          actorIndex: message.actorIndex,
          uiState: liveStateForClock,
	          skewMs: Math.max(
	            MATCH_CLOCK_CLAIM_SKEW_MS,
	            MATCH_CLOCK_ELAPSED_UNDERREPORT_SKEW_MS
	          ),
	          enforceObservationBounds:
	            options.enforceMatchClockObservationBounds !== false
	            && !skipMatchClockObservationBounds,
	          enforceUnderreportBounds:
	            options.enforceMatchClockObservationBounds !== false
	            && !skipMatchClockObservationBounds
	            && !actionCarriesCryptoMaterial
	            && !actionWasDelayedByProtocolWork,
	        });
	      }
      applyPhase = "reveal_pre_openings";
      await revealAuditOpenings(message.audit?.openings || [], {
        timing: "pre",
        command: message.command,
        shuffleProofs: message.audit?.shuffleProofs || [],
        uiState: liveStateForClock,
        updateState: false,
      });
      applyPhase = "reveal_private_proofs";
      await revealPrivateAuditProofsForLocalViewer(message.audit || {}, {
        updateState: false,
        persistDisclosure: !dryRun,
      });
      applyPhase = "remap_local_command";
      const localCommand = await remapCommandForLocalHiddenOpening(
        message.command,
        message.audit?.openings || [],
        message.actorIndex
      );
      applyPhase = "preview_requirements";
      const cryptoRequirements = filterCryptoRequirementsForCommand(
        localCommand,
        liveStateForClock,
        freshCryptoRequirementsForSequence(
          nextSequence,
          await previewRequirementsForCommand(localCommand)
        )
      );
      rememberActionCryptoRequirements(nextSequence, cryptoRequirements);
      applyPhase = "verify_shuffle_proofs";
      await verifyShuffleProofsForRequirements(
        cryptoRequirements,
        message.audit?.shuffleProofs || [],
        { allowAfterOrderMismatch: true }
      );
      applyPhase = "verify_crypto_requirements";
      await verifyAuditSatisfiesCryptoRequirements({
        requirements: cryptoRequirements,
        audit: message.audit,
      });
      applyPhase = "inject_crypto_material";
      await injectCryptoMaterialForRequirements(cryptoRequirements, message.audit || {}, {
        command: localCommand,
        seq: nextSequence,
        actorIndex: message.actorIndex,
        requirements: cryptoRequirements,
        updateState: false,
        persistDisclosure: !dryRun,
      });
      const liveStateBeforeApply = gameRef.current
        ? await gameRef.current.uiState()
        : liveStateForClock;
      const expectedActorBeforeApply = liveStateBeforeApply?.decision?.player;
      if (
        expectedActorBeforeApply !== null
        && expectedActorBeforeApply !== undefined
        && Number(expectedActorBeforeApply) !== Number(message.actorIndex)
      ) {
        throw new Error("Sequenced action actor is not the current decision player");
      }
      applyPhase = "apply_command";
      const publishAppliedStateImmediately = false;
      const appliedState = await applySyncedCommand(localCommand, message.label || "", {
        actorIndex: message.actorIndex,
        sequence: nextSequence,
        publishState: publishAppliedStateImmediately,
      });
      const remotePostOpeningState = await revealAuditOpenings(message.audit?.openings || [], {
        timing: "post",
        shuffleProofs: message.audit?.shuffleProofs || [],
        updateState: false,
      });
      const appliedCryptoRequirements = filterCryptoRequirementsForCommand(
        localCommand,
        liveStateForClock,
        freshCryptoRequirementsForSequence(
          nextSequence,
          cryptoRequirementsFromState(appliedState)
        )
      );
      rememberActionCryptoRequirements(nextSequence, appliedCryptoRequirements);
      await verifyShuffleProofsForRequirements(
        appliedCryptoRequirements,
        message.audit?.shuffleProofs || []
      );
      const actionCryptoRequirements = [...cryptoRequirements, ...appliedCryptoRequirements];
      const actionShuffleProofs = (message.audit?.shuffleProofs || []).filter((proof) =>
        !shuffleProofAlreadyAppliedBefore(nextSequence, proof)
        && !shuffleProofRequirementAlreadyRecordedBefore(nextSequence, proof)
        &&
        actionCryptoRequirements.some((requirement) =>
          shuffleProofMatchesRequirement(proof, requirement)
        )
      );
      const actionShuffleApplicationRequirements = [
        ...appliedCryptoRequirements,
        ...cryptoRequirements,
      ];
      const localizedActionShuffleProofs = alignShuffleProofsWithRequirements(
        actionShuffleProofs,
        actionShuffleApplicationRequirements
      );
      await applyVerifiedShuffleProofs(localizedActionShuffleProofs);
      await verifyAuditSatisfiesCryptoRequirements({
        requirements: appliedCryptoRequirements,
        audit: message.audit,
      });
      await revealPrivateAuditProofsForLocalViewer(message.audit || {}, {
        updateState: false,
        persistDisclosure: !dryRun,
      });
      await revealLocalZiffleHand(matchStartPayloadRef.current, {
        skipIfHandUnchanged: true,
        stateHint: viewedCardsStateHint(remotePostOpeningState, appliedState),
        command: localCommand,
        seq: nextSequence,
        actorIndex: message.actorIndex,
        actionAudit: message.audit,
        requirements: actionCryptoRequirements,
        updateState: false,
      });
      await verifyCurrentPublicCheckpointHash(
        message.audit?.publicCheckpointHash,
        "Sequenced action public checkpoint hash does not match local state"
      );
      if (dryRun) {
        await restoreValidationSnapshot();
        return {
          verified: true,
          publicCheckpointHash: String(message.audit?.publicCheckpointHash || ""),
          nextStateHash: String(message.audit?.nextStateHash || ""),
        };
      }
      commitMatchClockAudit(message.audit?.clock, appliedState);
      await appendAppliedSequencedAction(message);
      if (Number(nextSequence) === Number(matchClockObservationExemptSequenceRef.current || 0)) {
        matchClockObservationExemptSequenceRef.current = 0;
      }
      if (options.relay !== false) {
        relaySequencedAction(message);
      }
      await publishCurrentRuntimeState(
        viewedCardsStateHint(remotePostOpeningState, appliedState)
      );
      await drainPendingSequencedActions();
	    } catch (err) {
	      const failureReason = err instanceof Error ? err.message : String(err);
	      const rejectedActionCheat = isRejectedActionCheatReason(failureReason);
	      const unauthorizedAddCardCheat = isUnauthorizedAddCardCommand(message?.command);
	      if (!rejectedActionCheat && !unauthorizedAddCardCheat) {
	        console.error("[ironsmith] apply_action:failed", {
	          seq: nextSequence,
	          actor: Number(message?.actorIndex ?? -1),
	          phase: applyPhase,
	          command: summarizePeerCommand(message?.command),
	          error: failureReason,
	        });
	      }
      if (validationSnapshot) {
        try {
          await restoreValidationSnapshot();
        } catch {
          // Preserve the validation error as the actionable failure.
        }
      } else {
        updateMultiplayer((prev) => ({
          ...prev,
          submittingAction: false,
        }));
      }
      if (dryRun) {
        throw err;
      }
      const protocolTimeoutClaim = protocolResponseTimeoutClaimFromError(err);
      if (protocolTimeoutClaim) {
        if (throwOnFailure) {
          throw err;
        }
        await submitProtocolResponseTimeoutClaim(protocolTimeoutClaim);
        return;
      }
	      if (unauthorizedAddCardCheat) {
	        if (throwOnFailure) {
	          throw err;
	        }
	        const actorName = playerNameForIndex(multiplayerRef.current.players, message.actorIndex);
	        const status = isTrustedMultiplayerSecurityMode(message?.securityMode)
	          ? `Rejected add-card cheat from ${actorName}: ${failureReason}`
	          : `Rejected signed add-card cheat from ${actorName}: ${failureReason}`;
	        emitSyncFailureNotice("Cheat detected", status);
	        setStatus(status, true);
	        return;
	      }
	      if (rejectedActionCheat) {
	        if (throwOnFailure) {
	          throw err;
	        }
	        const actorName = playerNameForIndex(multiplayerRef.current.players, message.actorIndex);
	        if (isTrustedMultiplayerSecurityMode(sequencedActionSecurityMode(message, multiplayerRef.current))) {
	          const status = `Trusted action rejected from ${actorName}: ${failureReason}`;
	          const resynced = reportSyncFailure(
	            status,
	            options.failureResyncReason || "Trusted action mismatch. Resyncing with host...",
	            status
	          );
	          if (!resynced) {
	            setStatus(status, true);
	          }
	          return;
	        }
	        const status = `Cheat detected from ${actorName}: ${failureReason}`;
	        emitSyncFailureNotice("Cheat detected", status);
	        setStatus(status, true);
	        return;
	      }
      if (throwOnFailure) {
        throw err;
      }
      const resynced = reportSyncFailure(
        failureReason,
        options.failureResyncReason || "Failed to apply synced action. Resyncing with host..."
      );
      if (!resynced) {
        throw err;
      }
    }
  }

  async function buildLocalZiffleShuffleStep(request) {
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.ziffleBuildShuffleStep !== "function") {
      throw new Error("Ziffle shuffle backend is not available");
    }
    return currentGame.ziffleBuildShuffleStep({
      ...cloneMultiplayerPayload(request),
      entropyHex: randomAuditHex(32),
    });
  }

  async function answerZiffleShuffleStepRequest(conn, message) {
    try {
      const step = await buildLocalZiffleShuffleStep(message.request || {});
      safeSend(conn, {
        type: "ziffle_shuffle_step_response",
        protocolVersion: PROTOCOL_VERSION,
        requestId: message.requestId,
        step,
      });
    } catch (err) {
      safeSend(conn, {
        type: "ziffle_shuffle_step_response",
        protocolVersion: PROTOCOL_VERSION,
        requestId: message.requestId,
        error: toErrorMessage(err),
      });
    }
  }

  function recordZiffleShufflePerf(entry) {
    const normalized = {
      at: Date.now(),
      ...entry,
    };
    ziffleShufflePerfRef.current = [
      ...ziffleShufflePerfRef.current,
      normalized,
    ].slice(-32);
    console.debug?.("Ironsmith ziffle shuffle perf", normalized);
  }

  function normalizedZiffleShuffleCeremony(input, index, keys) {
    const deckCount = Number(input?.deckCount);
    if (!Number.isSafeInteger(deckCount) || deckCount <= 1) return null;
    if (!isSupportedZiffleDeckCount(deckCount)) {
      throw new Error(`Unsupported in-game ziffle library size ${deckCount}`);
    }
    const owner = Number(input?.owner);
    const zone = String(input?.zone || "library");
    const context = String(input?.context || "");
    const keyContext = String(input?.keyContext || context);
    return {
      id: String(input?.id || input?.requirementId || `ziffle:${owner}:${zone}:${index}`),
      requirement: input?.requirement || null,
      requirementId: String(input?.requirementId || input?.id || ""),
      owner,
      zone,
      deckCount,
      context,
      keyContext,
      manifest: input?.manifest || null,
      keys: cloneMultiplayerPayload(input?.keys || keys || []),
      beforeOrder: normalizeShuffleOrder(input?.beforeOrder ?? input?.before_order),
      afterOrder: normalizeShuffleOrder(input?.afterOrder ?? input?.after_order),
      steps: [],
      timings: {
        stepMs: [],
      },
    };
  }

  function ziffleShuffleRequestForCeremony(ceremony, shuffler) {
    return {
      deckCount: Number(ceremony.deckCount),
      context: String(ceremony.context || ""),
      keyContext: String(ceremony.keyContext || ceremony.context || ""),
      keys: cloneMultiplayerPayload(ceremony.keys || []),
      steps: cloneMultiplayerPayload(ceremony.steps || []),
      shuffler: Number(shuffler.index || 0),
    };
  }

  function appendZiffleShuffleStep(ceremony, step) {
    ceremony.steps.push({
      shuffler: Number(step.shuffler),
      deckHex: String(step.deckHex || ""),
      proofHex: String(step.proofHex || ""),
    });
  }

  async function runBatchedZiffleShuffleCeremonies(rawCeremonies, players, options = {}) {
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.ziffleVerifyShuffle !== "function") {
      throw new Error("Ziffle mental-poker backend is not available");
    }
    const orderedPlayers = reindexPlayers(players);
    const keys = cloneMultiplayerPayload(options.keys || zifflePublicKeysForPlayers(orderedPlayers));
    const ceremonies = (rawCeremonies || [])
      .map((ceremony, index) => normalizedZiffleShuffleCeremony(ceremony, index, keys))
      .filter(Boolean);
    if (ceremonies.length === 0) return [];

    const session = multiplayerRef.current;
    const localIndex = resolveLocalPlayerIndex(session);
    const localPeerId = String(session.localPeerId || "");
    const startedAt = nowMonotonicMs();
    const rounds = [];

    for (const shuffler of orderedPlayers) {
      const shufflerIndex = Number(shuffler.index || 0);
      const isLocalShuffler =
        shufflerIndex === Number(localIndex)
        || (localPeerId && String(shuffler.peerId || "") === localPeerId);
      const roundStartedAt = nowMonotonicMs();
      let results;
      if (isLocalShuffler) {
        results = await Promise.all(ceremonies.map(async (ceremony) => {
          const stepStartedAt = nowMonotonicMs();
          const step = await buildLocalZiffleShuffleStep(
            ziffleShuffleRequestForCeremony(ceremony, shuffler)
          );
          ceremony.timings.stepMs.push(nowMonotonicMs() - stepStartedAt);
          return { ceremony, step };
        }));
      } else {
        const routePeerId = routePeerIdForPlayer(shuffler);
        if (!routePeerId) {
          throw new Error(`Missing peer for ziffle shuffle player ${shufflerIndex + 1}`);
        }
        const label = shuffler.name || `Player ${shufflerIndex + 1}`;
        setStatus(
          `Waiting for cryptographic shuffle material from ${label} `
          + `(${ceremonies.length} shuffle${ceremonies.length === 1 ? "" : "s"})`
        );
        results = await Promise.all(ceremonies.map(async (ceremony) => {
          const requestId = makeZiffleRequestId("ziffle-live-shuffle");
          const requestedAtMs = Date.now();
          const waiter = waitForZiffleShuffleStep(requestId, PROTOCOL_RESPONSE_TIMEOUT_MS, {
            peerIndex: shufflerIndex,
            peerName: label,
            description:
              `${label} must provide verifiable shuffle material for ${ceremonies.length} `
              + `shuffle${ceremonies.length === 1 ? "" : "s"} before the game can advance.`,
          });
          const stepStartedAt = nowMonotonicMs();
          const requestPayload = {
            type: "ziffle_shuffle_step_request",
            protocolVersion: PROTOCOL_VERSION,
            requestId,
            request: ziffleShuffleRequestForCeremony(ceremony, shuffler),
          };
          await sendDirectProtocolMessage(routePeerId, requestPayload);
          const step = await waitForProtocolResponse(waiter, {
            basisSequence: Number(multiplayerRef.current.lastAppliedSequence || 0),
            targetPlayerIndex: shufflerIndex,
            targetPeerId: shuffler.peerId,
            requesterIndex: localIndex,
            requestType: requestPayload.type,
            requestId,
            requestPayload,
            responseTimeoutMs: PROTOCOL_RESPONSE_TIMEOUT_MS,
            requestedAtMs,
          });
          ceremony.timings.stepMs.push(nowMonotonicMs() - stepStartedAt);
          return { ceremony, step };
        }));
      }
      for (const { ceremony, step } of results) {
        appendZiffleShuffleStep(ceremony, step);
      }
      rounds.push({
        shuffler: shufflerIndex,
        local: Boolean(isLocalShuffler),
        ceremonyCount: ceremonies.length,
        ms: Math.round(nowMonotonicMs() - roundStartedAt),
      });
    }

    const verifyStartedAt = nowMonotonicMs();
    const verifiedCeremonies = await Promise.all(ceremonies.map(async (ceremony) => {
      const verified = await currentGame.ziffleVerifyShuffle({
        deckCount: Number(ceremony.deckCount),
        context: String(ceremony.context || ""),
        keyContext: String(ceremony.keyContext || ceremony.context || ""),
        keys: cloneMultiplayerPayload(ceremony.keys || []),
        steps: cloneMultiplayerPayload(ceremony.steps || []),
      });
      return { ceremony, verified };
    }));
    const verifyMs = nowMonotonicMs() - verifyStartedAt;
    for (const { ceremony, verified } of verifiedCeremonies) {
      ceremony.deckHash = String(verified.deckHash || "");
      ceremony.deckHex = String(verified.deckHex || "");
    }

    recordZiffleShufflePerf({
      kind: String(options.kind || "shuffle"),
      ceremonyCount: ceremonies.length,
      playerCount: orderedPlayers.length,
      deckCounts: ceremonies.map((ceremony) => ceremony.deckCount),
      totalMs: Math.round(nowMonotonicMs() - startedAt),
      verifyMs: Math.round(verifyMs),
      rounds,
    });

    return ceremonies;
  }

  async function buildLocalZiffleRevealToken(ceremony, cardPosition) {
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.ziffleBuildRevealToken !== "function") {
      throw new Error("Ziffle reveal-token backend is not available");
    }
    const localIndex = resolveLocalCryptoPlayerIndex();
    const keyContext = ziffleKeyContextForCeremony(ceremony);
    const keyPair = await ensureZiffleIdentity({
      context: keyContext,
      deckCount: ceremony.deckCount,
    });
    return currentGame.ziffleBuildRevealToken({
      deckCount: Number(ceremony.deckCount),
      context: String(ceremony.context || ""),
      keyContext,
      keys: cloneMultiplayerPayload(ceremony.keys || []),
      steps: cloneMultiplayerPayload(ceremony.steps || []),
      cardPosition: Number(cardPosition),
      publicKeyHex: String(keyPair.publicKeyHex || ""),
      secretKeyHex: String(keyPair.secretKeyHex || ""),
      entropyHex: randomAuditHex(32),
      player: Number(localIndex || 0),
    });
  }

  function zifflePositionsDetail(positions = []) {
    const normalized = (positions || []).map(Number).filter((position) => Number.isFinite(position));
    if (normalized.length === 0) return "";
    const sample = normalized.slice(0, 5).join(", ");
    return normalized.length > 5
      ? `Positions ${sample}, +${normalized.length - 5} more`
      : `Positions ${sample}`;
  }

  async function buildLocalZiffleRevealTokens(ceremony, cardPositions) {
    const positions = normalizeZiffleCardPositions(cardPositions, ceremony, {
      allowEmpty: true,
      label: "Local ziffle reveal-token build",
    });
    if (positions.length === 0) return [];
    const currentGame = gameRef.current;
    if (!currentGame) {
      throw new Error("Ziffle reveal-token backend is not available");
    }
    const waitId = beginPeerWait({
      kind: "local_ziffle_reveal",
      local: true,
      peerIndex: resolveLocalCryptoPlayerIndex(),
      peerName: "You",
      title: "Generating reveal proof",
      description:
        `Your browser is deriving cryptographic opening material for ${positions.length} hidden `
        + `card${positions.length === 1 ? "" : "s"}.`,
      operation: "Generating reveal proof",
      detail: zifflePositionsDetail(positions),
      progressCurrent: 0,
      progressTotal: positions.length,
      responseTimeoutMs: ziffleRevealTokenTimeoutMs(positions.length, ceremony),
    });
    try {
      if (typeof currentGame.ziffleBuildRevealTokens !== "function") {
        const tokens = [];
        for (const [index, position] of positions.entries()) {
          updatePeerWait(waitId, {
            detail: `Position ${Number(position)}`,
            progressCurrent: index,
            progressTotal: positions.length,
          });
          const token = await buildLocalZiffleRevealToken(ceremony, position);
          tokens.push({
            ...token,
            cardPosition: positions[index],
          });
          updatePeerWait(waitId, {
            progressCurrent: index + 1,
            progressTotal: positions.length,
          });
        }
        return tokens;
      }
      const localIndex = resolveLocalCryptoPlayerIndex();
      const keyContext = ziffleKeyContextForCeremony(ceremony);
      const keyPair = await ensureZiffleIdentity({
        context: keyContext,
        deckCount: ceremony.deckCount,
      });
      updatePeerWait(waitId, {
        operation: "Generating batched reveal proof",
        progressCurrent: 0,
        progressTotal: positions.length,
      });
      return await currentGame.ziffleBuildRevealTokens({
        deckCount: Number(ceremony.deckCount),
        context: String(ceremony.context || ""),
        keyContext,
        keys: cloneMultiplayerPayload(ceremony.keys || []),
        steps: cloneMultiplayerPayload(ceremony.steps || []),
        cardPositions: positions,
        publicKeyHex: String(keyPair.publicKeyHex || ""),
        secretKeyHex: String(keyPair.secretKeyHex || ""),
        entropyHex: randomAuditHex(32),
        player: Number(localIndex || 0),
      });
    } finally {
      clearPeerWait(waitId);
    }
  }

  async function waitForZiffleCeremony(owner, options = {}, timeoutMs = 10000) {
    const started = Date.now();
    while (Date.now() - started < timeoutMs) {
      const ceremony = ziffleCeremonyForOwner(owner, options);
      if (ceremony) return ceremony;
      await sleep(50);
    }
    return null;
  }

  async function authorizedZiffleRevealPositionsForOwner(owner, deckHash) {
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.exportSyncCheckpoint !== "function") {
      return new Set();
    }
    const checkpoint = await currentGame.exportSyncCheckpoint();
    const positions = new Set();
    const expectedDeckHash = String(deckHash || "");
    const addMetadataPosition = (metadata) => {
      if (!metadata || Number(metadata.owner) !== Number(owner)) return;
      const publicCommitment = String(
        metadata.publicCommitment
        || metadata.public_commitment
        || ""
      );
      const hiddenCommitment = String(metadata.commitment || "");
      const publicSlot = metadata.publicSlot ?? metadata.public_slot ?? null;
      const addCommittedPosition = (position, commitment) => {
        const commitmentDeckHash = ziffleDeckHashFromCommitment(commitment);
        if (!commitmentDeckHash) return;
        if (expectedDeckHash && commitmentDeckHash !== expectedDeckHash) return;
        const committedPosition = position ?? zifflePositionFromCommitment(commitment);
        const normalizedPosition = Number(committedPosition);
        if (Number.isSafeInteger(normalizedPosition) && normalizedPosition >= 0) {
          positions.add(normalizedPosition);
        }
      };
      addCommittedPosition(publicSlot, publicCommitment);
      addCommittedPosition(metadata.slot, hiddenCommitment);
    };
    const objectsById = new Map((checkpoint?.objects || []).map((object) => [
      Number(object?.id),
      object,
    ]));
    const collectZoneObjectIds = (player, zoneKey) => {
      for (const objectId of player?.[zoneKey] || []) {
        const object = objectsById.get(Number(objectId));
        const hidden = object?.hiddenCard || object?.hidden_card || null;
        if (!hidden) continue;
        addMetadataPosition({
          objectId: Number(objectId),
          owner: hidden.owner,
          slot: hidden.slot,
          commitment: hidden.commitment,
          publicSlot: hidden.publicSlot ?? hidden.public_slot ?? null,
          publicCommitment: hidden.publicCommitment ?? hidden.public_commitment ?? "",
        });
      }
    };
    for (const player of checkpoint?.players || []) {
      if (Number(player?.id ?? player?.index) !== Number(owner)) continue;
      collectZoneObjectIds(player, "hand");
      collectZoneObjectIds(player, "graveyard");
      collectZoneObjectIds(player, "commanders");
      collectZoneObjectIds(player, "sideboard");
    }
    for (const objectId of checkpoint?.exile || []) {
      const object = objectsById.get(Number(objectId));
      const hidden = object?.hiddenCard || object?.hidden_card || null;
      if (!hidden || Number(hidden.owner) !== Number(owner)) continue;
      addMetadataPosition({
        objectId: Number(objectId),
        owner: hidden.owner,
        slot: hidden.slot,
        commitment: hidden.commitment,
        publicSlot: hidden.publicSlot ?? hidden.public_slot ?? null,
        publicCommitment: hidden.publicCommitment ?? hidden.public_commitment ?? "",
      });
    }
    const blockedZones = new Set(["library", "outside_game"]);
    for (const object of checkpoint?.objects || []) {
      const hidden = object?.hiddenCard || object?.hidden_card || null;
      if (!hidden || Number(hidden.owner) !== Number(owner)) continue;
      const zone = String(object?.zone || hidden.zone || "");
      if (blockedZones.has(zone)) continue;
      addMetadataPosition({
        objectId: Number(object?.id),
        owner: hidden.owner,
        slot: hidden.slot,
        commitment: hidden.commitment,
        publicSlot: hidden.publicSlot ?? hidden.public_slot ?? null,
        publicCommitment: hidden.publicCommitment ?? hidden.public_commitment ?? "",
      });
    }
    return positions;
  }

  async function waitForAuthorizedZiffleRevealPositions(owner, deckHash, requestedPositions, timeoutMs = 10000) {
    const needed = new Set((requestedPositions || []).map((position) => Number(position)));
    const started = Date.now();
    let allowed = await authorizedZiffleRevealPositionsForOwner(owner, deckHash);
    while ([...needed].some((position) => !allowed.has(position)) && Date.now() - started < timeoutMs) {
      await sleep(50);
      allowed = await authorizedZiffleRevealPositionsForOwner(owner, deckHash);
    }
    return allowed;
  }

  function ziffleRevealAuthorizedByOutboundCryptoRequest(message, requester, owner, positions) {
    const requestId = String(message?.cryptoMaterialRequestId || "");
    if (!requestId) return false;
    const pending = outboundCryptoMaterialRequestsRef.current.get(requestId);
    if (!pending) return false;
    if (String(pending.peerId || "") !== String(message?.requesterPeerId || "")) return false;
    if (!pending.actionIntent) return false;
    // The peer we asked for material is the seat responsible for the
    // requirement: the deck owner, or the viewer of a non-owner private view
    // (who must collect our reveal token to aggregate the card locally).
    if (Number(pending.targetSeat ?? pending.owner) !== Number(requester)) return false;
    const requested = new Set((positions || []).map((position) => Number(position)));
    const allowed = new Set();
    for (const requirement of pending.requirements || []) {
      if (Number(requirement?.owner) !== Number(owner)) continue;
      const type = ziffleRequirementType(requirement);
      if (
        (type === "private_open" || type === "private_view_window")
        && ziffleRequirementViewer(requirement) !== Number(requester)
        && Number(requirement.owner) !== Number(requester)
      ) {
        continue;
      }
      const position = ziffleRevealPositionFromRequirement(requirement);
      if (Number.isSafeInteger(position) && position >= 0) {
        allowed.add(position);
      }
    }
    return [...requested].every((position) => allowed.has(position));
  }

  function ziffleRequirementType(requirement) {
    return String(requirement?.type || requirement?.requirement_type || "");
  }

  function ziffleRequirementZone(requirement) {
    return String(requirement?.zone || "");
  }

  function ziffleRequirementViewer(requirement) {
    if (requirement?.viewer === null || requirement?.viewer === undefined) return null;
    const viewer = Number(requirement.viewer);
    return Number.isInteger(viewer) ? viewer : null;
  }

  function zifflePositionFromRequirement(requirement) {
    const publicPosition = zifflePublicPositionFromSources(requirement);
    if (publicPosition && publicPosition.useAsPosition !== false) return publicPosition.position;
    const committedPosition =
      zifflePositionFromCommitment(requirement?.commitment)
      ?? zifflePositionFromCommitment(requirement?.positionCommitment)
      ?? zifflePositionFromCommitment(requirement?.position_commitment);
    if (committedPosition != null) return committedPosition;
    const explicitPosition = Number(requirement?.position);
    if (Number.isSafeInteger(explicitPosition) && explicitPosition >= 0) return explicitPosition;
    const publicSlot = Number(requirement?.publicSlot ?? requirement?.public_slot);
    if (Number.isSafeInteger(publicSlot) && publicSlot >= 0) return publicSlot;
    const slot = Number(requirement?.slot);
    return Number.isSafeInteger(slot) && slot >= 0 ? slot : null;
  }

  function ziffleRevealPositionFromRequirement(requirement) {
    const publicPosition = zifflePublicPositionFromSources(requirement);
    if (publicPosition && publicPosition.useAsPosition !== false) {
      return publicPosition.position;
    }
    const committedPosition =
      zifflePositionFromCommitment(requirement?.commitment)
      ?? zifflePositionFromCommitment(requirement?.positionCommitment)
      ?? zifflePositionFromCommitment(requirement?.position_commitment);
    if (committedPosition != null) return committedPosition;
    const zone = ziffleRequirementZone(requirement).toLowerCase();
    if (zone && zone !== "library") return null;
    const explicitPosition = Number(requirement?.position);
    if (Number.isSafeInteger(explicitPosition) && explicitPosition >= 0) return explicitPosition;
    const publicSlot = Number(requirement?.publicSlot ?? requirement?.public_slot);
    return Number.isSafeInteger(publicSlot) && publicSlot >= 0 ? publicSlot : null;
  }

  function ziffleRequirementsAuthorizeRevealPositions(requirements, requester, owner, positions, ceremony) {
    const requested = (positions || [])
      .map((position) => Number(position))
      .filter((position) => Number.isSafeInteger(position) && position >= 0);
    if (requested.length === 0) return false;
    const exactPositions = new Set();
    const deckCount = Number(ceremony?.deckCount);
    let wholeLibraryWindow = false;
    for (const requirement of requirements || []) {
      if (Number(requirement?.owner) !== Number(owner)) continue;
      const type = ziffleRequirementType(requirement);
      const zone = ziffleRequirementZone(requirement);
      const viewer = ziffleRequirementViewer(requirement);
      if (type === "private_open" && viewer !== Number(requester)) continue;
      if (type === "private_view_window" && viewer !== Number(requester)) continue;
      if (type === "public_open" || type === "private_open") {
        const position = ziffleRevealPositionFromRequirement(requirement);
        if (position != null) exactPositions.add(position);
      } else if (type === "verifiable_shuffle" && zone === "library") {
        const afterOrder = normalizeShuffleOrder(requirement?.afterOrder ?? requirement?.after_order);
        const libraryPrefixCount = Number(requirement?.count);
        if (
          Number.isSafeInteger(deckCount)
          && deckCount > 0
          && afterOrder.length === deckCount
          && Number.isSafeInteger(libraryPrefixCount)
          && libraryPrefixCount >= 0
          && libraryPrefixCount <= afterOrder.length
        ) {
          for (let position = libraryPrefixCount; position < afterOrder.length; position += 1) {
            exactPositions.add(position);
          }
        }
      } else if (
        (type === "public_view_window" || type === "private_view_window")
        && zone === "library"
        && Number.isSafeInteger(deckCount)
        && deckCount > 0
        && Number(requirement?.count || 0) >= deckCount
      ) {
        wholeLibraryWindow = true;
      }
    }
    if (requested.every((position) => exactPositions.has(position))) {
      return true;
    }
    return Boolean(
      wholeLibraryWindow
      && Number.isSafeInteger(deckCount)
      && requested.length <= deckCount
    );
  }

  async function ziffleRequirementsAuthorizeRevealPositionsByMetadata(
    requirements,
    requester,
    owner,
    positions,
    ceremony
  ) {
    const requested = new Set((positions || [])
      .map((position) => Number(position))
      .filter((position) => Number.isSafeInteger(position) && position >= 0));
    if (requested.size === 0) return false;
    const deckHash = String(ceremony?.deckHash || "");
    const afterOrder = normalizeShuffleOrder(ceremony?.afterOrder ?? ceremony?.after_order);
    const currentGame = gameRef.current;
    let checkpoint = null;
    const metadataForObject = async (objectId) => {
      const normalized = Number(objectId);
      if (!Number.isSafeInteger(normalized) || normalized < 0) return null;
      if (checkpoint) return hiddenCardMetadataForObjectFromCheckpoint(checkpoint, normalized);
      return currentHiddenCardMetadataForObject(normalized);
    };
    const allowed = new Set();
    for (const requirement of requirements || []) {
      if (Number(requirement?.owner) !== Number(owner)) continue;
      const type = ziffleRequirementType(requirement);
      if (type !== "public_open" && type !== "private_open") continue;
      if (type === "private_open" && ziffleRequirementViewer(requirement) !== Number(requester)) {
        continue;
      }
      let objectId = requirement.objectId ?? requirement.object_id;
      if (objectId == null && currentGame && typeof currentGame.exportSyncCheckpoint === "function") {
        if (!checkpoint) {
          try {
            checkpoint = await currentGame.exportSyncCheckpoint();
          } catch {
            checkpoint = null;
          }
        }
        objectId = hiddenObjectIdForOpeningFromCheckpoint(checkpoint, {
          owner,
          slot: requirement.slot,
          position: ziffleRevealPositionFromRequirement(requirement),
          positionCommitment: requirement.publicCommitment
            || requirement.public_commitment
            || requirement.positionCommitment
            || requirement.position_commitment
            || requirement.commitment,
          commitment: requirement.commitment,
        });
      }
      const metadata = await metadataForObject(objectId);
      if (!metadata || Number(metadata.owner) !== Number(owner)) continue;
      const publicCommitment = String(metadata.publicCommitment || "");
      const hiddenCommitment = String(metadata.commitment || "");
      if (
        metadata.publicSlot != null
        && (!deckHash || ziffleDeckHashFromCommitment(publicCommitment) === deckHash)
      ) {
        allowed.add(Number(metadata.publicSlot));
      }
      if (
        metadata.slot != null
        && ziffleDeckHashFromCommitment(hiddenCommitment)
        && (!deckHash || ziffleDeckHashFromCommitment(hiddenCommitment) === deckHash)
      ) {
        allowed.add(Number(metadata.slot));
      }
      for (const requestedPosition of requested) {
        const positionObjectId = Number(afterOrder[requestedPosition]);
        if (!Number.isSafeInteger(positionObjectId) || positionObjectId < 0) continue;
        const positionMetadata = await metadataForObject(positionObjectId);
        if (!positionMetadata || Number(positionMetadata.owner) !== Number(owner)) continue;
        const positionPublicCommitment = String(positionMetadata.publicCommitment || "");
        const positionHiddenCommitment = String(positionMetadata.commitment || "");
        const positionMatchesRequirement =
          Number(positionMetadata.slot) === Number(requirement.slot)
          || Number(positionMetadata.publicSlot) === requestedPosition
          || ziffleRevealPositionFromRequirement(requirement) === Number(positionMetadata.slot);
        const positionUsesCeremony =
          (!deckHash || ziffleDeckHashFromCommitment(positionPublicCommitment) === deckHash)
          || (!deckHash || ziffleDeckHashFromCommitment(positionHiddenCommitment) === deckHash);
        if (positionMatchesRequirement && positionUsesCeremony) {
          allowed.add(requestedPosition);
        }
      }
    }
    return [...requested].every((position) => allowed.has(position));
  }

  function ziffleRequirementsAuthorizeRevealPositionCount(requirements, requester, owner, positions, ceremony) {
    const requested = [...new Set((positions || [])
      .map((position) => Number(position))
      .filter((position) => Number.isSafeInteger(position) && position >= 0))];
    const deckCount = Number(ceremony?.deckCount);
    if (
      requested.length === 0
      || !Number.isSafeInteger(deckCount)
      || requested.some((position) => position >= deckCount)
    ) {
      return false;
    }
    let openCount = 0;
    for (const requirement of requirements || []) {
      if (Number(requirement?.owner) !== Number(owner)) continue;
      const type = ziffleRequirementType(requirement);
      if (type === "public_open") {
        openCount += 1;
      } else if (
        type === "private_open"
        && ziffleRequirementViewer(requirement) === Number(requester)
      ) {
        openCount += 1;
      }
    }
    return openCount > 0 && requested.length <= openCount;
  }

  function compactCryptoRequirementForDiagnostics(requirement) {
    if (!requirement || typeof requirement !== "object") return null;
    const beforeOrder = normalizeShuffleOrder(requirement.beforeOrder ?? requirement.before_order);
    const afterOrder = normalizeShuffleOrder(requirement.afterOrder ?? requirement.after_order);
    return {
      type: ziffleRequirementType(requirement),
      owner: Number(requirement.owner),
      viewer: ziffleRequirementViewer(requirement),
      zone: ziffleRequirementZone(requirement),
      count: requirement.count == null ? null : Number(requirement.count),
      slot: requirement.slot == null ? null : Number(requirement.slot),
      position: zifflePositionFromRequirement(requirement),
      beforeOrderLength: beforeOrder.length,
      afterOrderLength: afterOrder.length,
      afterOrderFirst: afterOrder.length > 0 ? afterOrder[0] : null,
      afterOrderLast: afterOrder.length > 0 ? afterOrder[afterOrder.length - 1] : null,
    };
  }

  function compactActionAuthorizationForDiagnostics(auth) {
    if (!auth || typeof auth !== "object") return null;
    return {
      matchId: String(auth.matchId || ""),
      seq: Number(auth.seq || 0),
      requesterIndex: auth.requesterIndex == null ? null : Number(auth.requesterIndex),
      actorIndex: auth.actorIndex == null ? null : Number(auth.actorIndex),
      commandType: String(auth.command?.type || ""),
      actionKind: String(auth.command?.action_ref?.kind || ""),
      requirements: (Array.isArray(auth.requirements) ? auth.requirements : [])
        .map(compactCryptoRequirementForDiagnostics)
        .filter(Boolean),
    };
  }

  function actionAuthorizationRequirementMatchesPreview(previewed, attached) {
    if (!previewed || !attached) return false;
    const type = ziffleRequirementType(previewed);
    if (!type || type !== ziffleRequirementType(attached)) return false;
    if (Number(previewed.owner) !== Number(attached.owner)) return false;
    if (ziffleRequirementZone(previewed) !== ziffleRequirementZone(attached)) return false;
    if (type !== "verifiable_shuffle") return false;

    const previewBefore = normalizeShuffleOrder(previewed.beforeOrder ?? previewed.before_order);
    const attachedBefore = normalizeShuffleOrder(attached.beforeOrder ?? attached.before_order);
    if (previewBefore.length > 0 && attachedBefore.length > 0 && !sameShuffleOrder(previewBefore, attachedBefore)) {
      return false;
    }

    const previewAfter = normalizeShuffleOrder(previewed.afterOrder ?? previewed.after_order);
    const attachedAfter = normalizeShuffleOrder(attached.afterOrder ?? attached.after_order);
    if (previewAfter.length > 0 && attachedAfter.length > 0 && !sameShuffleOrder(previewAfter, attachedAfter)) {
      return false;
    }

    return true;
  }

  function enrichPreviewedRequirementsFromAuthorization(previewedRequirements = [], attachedRequirements = []) {
    return (previewedRequirements || []).map((previewed) => {
      const attached = (attachedRequirements || []).find((candidate) =>
        actionAuthorizationRequirementMatchesPreview(previewed, candidate)
      );
      if (!attached) return previewed;
      return {
        ...previewed,
        count: previewed.count ?? attached.count,
        beforeOrder: previewed.beforeOrder ?? attached.beforeOrder,
        before_order: previewed.before_order ?? attached.before_order,
        afterOrder: previewed.afterOrder ?? attached.afterOrder,
        after_order: previewed.after_order ?? attached.after_order,
      };
    });
  }

  function playerLibrarySizeFromState(stateLike, owner) {
    const player = (stateLike?.players || []).find((candidate) =>
      Number(candidate?.id ?? candidate?.index) === Number(owner)
    );
    if (!player) return null;
    const librarySize = Number(player.librarySize ?? player.library_size);
    if (Number.isSafeInteger(librarySize) && librarySize >= 0) return librarySize;
    if (Array.isArray(player.library)) return player.library.length;
    return null;
  }

  function attachedMulliganShuffleRequirements(auth, liveState, owner) {
    if (String(auth?.command?.action_ref?.kind || "") !== "take_mulligan") return [];
    const librarySize = playerLibrarySizeFromState(liveState, owner);
    if (!Number.isSafeInteger(librarySize) || librarySize < 0) return [];
    return (Array.isArray(auth.requirements) ? auth.requirements : []).filter((requirement) =>
      ziffleRequirementType(requirement) === "verifiable_shuffle"
      && Number(requirement?.owner) === Number(owner)
      && ziffleRequirementZone(requirement) === "library"
      && Number(requirement?.count) === librarySize
    );
  }

  async function waitForRevealAuthorizationSequence(sequence, debug = null, timeoutMs = 10000) {
    const target = Number(sequence);
    const started = Date.now();
    let currentSequence = Number(multiplayerRef.current.lastAppliedSequence || 0);
    while (
      Number.isSafeInteger(target)
      && target > currentSequence + 1
      && Date.now() - started < timeoutMs
    ) {
      await sleep(50);
      currentSequence = Number(multiplayerRef.current.lastAppliedSequence || 0);
    }
    if (debug) {
      debug.currentSequence = currentSequence;
      debug.expectedSeq = currentSequence + 1;
      debug.sequenceWaitMs = Date.now() - started;
    }
    return currentSequence;
  }

  async function ziffleRevealAuthorizedByAction(message, requester, owner, positions, ceremony, debug = null) {
    const auth = message?.actionAuthorization;
    const reject = (reason) => {
      if (debug) debug.reason = reason;
      return false;
    };
    if (!auth || typeof auth !== "object") return reject("missing_action_authorization");
    if (String(auth.matchId || "") !== String(currentAuditMatchId())) return reject("match_id_mismatch");
    if (Number(auth.requesterIndex) !== Number(requester)) return reject("requester_index_mismatch");
    // The requester may be the deck owner (its own reveals) or the viewer of a
    // non-owner private view (mental-poker flow); requirement checks below
    // only authorize private positions whose viewer is the requester.
    const sequence = Number(auth.seq);
    if (!Number.isSafeInteger(sequence) || sequence <= 0) return reject("invalid_sequence");
    if (!auth.command || typeof auth.command !== "object") return reject("missing_command");
    const currentSequence = await waitForRevealAuthorizationSequence(sequence, debug);
    const expectedSeq = currentSequence + 1;
    if (debug) {
      debug.sequence = sequence;
      debug.currentSequence = currentSequence;
      debug.expectedSeq = expectedSeq;
    }

    if (sequence <= currentSequence) {
      const applied = actionHistoryEntryForSequence(sequence);
      if (!applied) return reject("missing_applied_action");
      if (canonicalMultiplayerPayload(applied.command) !== canonicalMultiplayerPayload(auth.command)) {
        return reject("applied_command_mismatch");
      }
      if (
        auth.actorIndex !== null
        && auth.actorIndex !== undefined
        && Number(auth.actorIndex) !== Number(applied.actorIndex)
      ) {
        return reject("applied_actor_mismatch");
      }
      const storedRequirements = actionCryptoRequirementsForSequence(sequence);
      if (debug) {
        debug.storedRequirements = storedRequirements.map(compactCryptoRequirementForDiagnostics).filter(Boolean);
      }
      const authorized = ziffleRequirementsAuthorizeRevealPositions(
        actionCryptoRequirementsForSequence(sequence),
        requester,
        owner,
        positions,
        ceremony
      );
      if (authorized && debug) debug.reason = "authorized_by_stored_requirements";
      if (authorized) return true;
      const authorizedByMetadata = await ziffleRequirementsAuthorizeRevealPositionsByMetadata(
        storedRequirements,
        requester,
        owner,
        positions,
        ceremony
      );
      if (authorizedByMetadata && debug) debug.reason = "authorized_by_stored_requirement_metadata";
      if (authorizedByMetadata) return true;
      const authorizedByOpenCount = ziffleRequirementsAuthorizeRevealPositionCount(
        storedRequirements,
        requester,
        owner,
        positions,
        ceremony
      );
      if (authorizedByOpenCount && debug) debug.reason = "authorized_by_stored_open_count";
      if (authorizedByOpenCount) return true;
      // The visible-state fallback derives positions from zones the OWNER is
      // entitled to open; never extend it to other requesters.
      if (Number(requester) === Number(owner)) {
        const authorizedByVisibleState = await waitForAuthorizedZiffleRevealPositions(
          owner,
          String(ceremony?.deckHash || ""),
          positions,
          2000
        );
        if ([...positions].every((position) => authorizedByVisibleState.has(Number(position)))) {
          if (debug) debug.reason = "authorized_by_visible_current_hidden_zone_state";
          return true;
        }
      }
      return reject("stored_requirements_do_not_authorize_positions");
    }

    if (sequence !== expectedSeq) return reject("unexpected_sequence");

    const attachedRequirements = Array.isArray(auth.requirements) ? auth.requirements : [];
    const attachedOpenCountAuthorized =
      Number(auth.actorIndex) === Number(requester)
      && ziffleRequirementsAuthorizeRevealPositionCount(
        attachedRequirements,
        requester,
        owner,
        positions,
        ceremony
      );
    if (attachedOpenCountAuthorized && auth.actionIntent) {
      try {
        await verifySignedActionIntent(auth.actionIntent, {
          matchId: currentAuditMatchId(),
          seq: sequence,
          actorIndex: auth.actorIndex,
          prevStateHash: auth.prevStateHash || auth.actionIntent.prevStateHash,
          preActionPublicCheckpointHash:
            auth.preActionPublicCheckpointHash
            || auth.publicCheckpointHash
            || auth.actionIntent.preActionPublicCheckpointHash,
          command: auth.command,
        });
        if (debug) debug.reason = "authorized_by_signed_attached_open_count";
        return true;
      } catch (err) {
        if (debug) debug.signedAttachedIntentError = toErrorMessage(err);
      }
    }

    const liveState = gameRef.current ? await gameRef.current.uiState() : stateRef.current;
    const compatible = isDecisionCommandCompatible(liveState?.decision, auth.command);
    if (debug) {
      debug.liveDecisionKind = String(liveState?.decision?.kind || "");
      debug.liveDecisionPlayer = liveState?.decision?.player == null ? null : Number(liveState.decision.player);
      debug.commandCompatible = compatible;
    }
    if (!compatible) return reject("command_not_compatible_with_live_decision");
    if (
      auth.actorIndex !== null
      && auth.actorIndex !== undefined
      && liveState?.decision?.player !== null
      && liveState?.decision?.player !== undefined
      && Number(auth.actorIndex) !== Number(liveState.decision.player)
    ) {
      return reject("actor_index_mismatch");
    }
    if (auth.actionAudit) {
      try {
        await verifySequencedActionAudit({
          audit: auth.actionAudit,
          seq: sequence,
          actorIndex: auth.actorIndex,
          command: auth.command,
        });
      } catch (err) {
        if (debug) debug.signedActionAuditError = toErrorMessage(err);
        return reject("signed_action_audit_invalid");
      }
    } else {
      const preActionPublicCheckpointHash = await currentPublicAuditCheckpointHash();
      try {
        await verifySignedActionIntent(auth.actionIntent, {
          matchId: currentAuditMatchId(),
          seq: sequence,
          actorIndex: auth.actorIndex,
          prevStateHash: auth.prevStateHash || auditStateHashRef.current || INITIAL_AUDIT_STATE_HASH,
          preActionPublicCheckpointHash,
          command: auth.command,
        });
      } catch (err) {
        if (debug) debug.signedIntentError = toErrorMessage(err);
        return reject("signed_action_intent_invalid");
      }
    }

    const previewedRequirements = await previewRequirementsForCommand(auth.command);
    if (debug) {
      debug.previewedRequirements = previewedRequirements
        .map(compactCryptoRequirementForDiagnostics)
        .filter(Boolean);
      debug.attachedRequirements = (Array.isArray(auth.requirements) ? auth.requirements : [])
        .map(compactCryptoRequirementForDiagnostics)
        .filter(Boolean);
    }
    if (ziffleRequirementsAuthorizeRevealPositions(
      previewedRequirements,
      requester,
      owner,
      positions,
      ceremony
    )) {
      if (debug) debug.reason = "authorized_by_previewed_requirements";
      return true;
    }
    const enrichedRequirements = enrichPreviewedRequirementsFromAuthorization(
      previewedRequirements,
      Array.isArray(auth.requirements) ? auth.requirements : []
    );
    if (debug) {
      debug.enrichedRequirements = enrichedRequirements
        .map(compactCryptoRequirementForDiagnostics)
        .filter(Boolean);
    }
    const authorized = ziffleRequirementsAuthorizeRevealPositions(
      enrichedRequirements,
      requester,
      owner,
      positions,
      ceremony
    );
    if (authorized && debug) debug.reason = "authorized_by_enriched_requirements";
    if (authorized) return true;

    const authorizedByMetadata = await ziffleRequirementsAuthorizeRevealPositionsByMetadata(
      enrichedRequirements,
      requester,
      owner,
      positions,
      ceremony
    );
    if (authorizedByMetadata && debug) debug.reason = "authorized_by_requirement_metadata";
    if (authorizedByMetadata) return true;

    const authorizedByOpenCount = ziffleRequirementsAuthorizeRevealPositionCount(
      enrichedRequirements,
      requester,
      owner,
      positions,
      ceremony
    );
    if (authorizedByOpenCount && debug) debug.reason = "authorized_by_open_requirement_count";
    if (authorizedByOpenCount) return true;

    const mulliganShuffleRequirements = attachedMulliganShuffleRequirements(auth, liveState, owner);
    if (debug) {
      debug.mulliganShuffleRequirements = mulliganShuffleRequirements
        .map(compactCryptoRequirementForDiagnostics)
        .filter(Boolean);
    }
    const authorizedByMulliganShuffle = ziffleRequirementsAuthorizeRevealPositions(
      mulliganShuffleRequirements,
      requester,
      owner,
      positions,
      ceremony
    );
    if (authorizedByMulliganShuffle && debug) debug.reason = "authorized_by_attached_mulligan_shuffle";
    if (authorizedByMulliganShuffle) return true;

    if (Number(requester) === Number(owner)) {
      const authorizedByVisibleState = await waitForAuthorizedZiffleRevealPositions(
        owner,
        String(ceremony?.deckHash || ""),
        positions,
        2000
      );
      if ([...positions].every((position) => authorizedByVisibleState.has(Number(position)))) {
        if (debug) debug.reason = "authorized_by_visible_current_hidden_zone_state";
        return true;
      }
    }
    return reject("requirements_do_not_authorize_positions");
  }

  async function answerZiffleRevealTokenRequest(conn, message) {
    let diagnostics = null;
    try {
      const requester = playerIndexForPeerId(conn?.peer);
      const requestedOwner = Number(message.ceremonyOwner);
      if (requester == null) {
        throw new Error("Ziffle reveal tokens can only be requested by a match player");
      }
      const lookup = {
        deckHash: message.deckHash,
        context: message.ceremonyContext,
      };
      const requestCeremony = message.ceremony;
      const requestCeremonySummary = compactZiffleCeremonyForDiagnostics(requestCeremony);
      let requestCeremonyRejectReason = "";
      const attachedCeremony =
        requestCeremony && typeof requestCeremony === "object"
          ? (() => {
            if (Number(requestCeremony.owner) !== Number(message.ceremonyOwner)) {
              requestCeremonyRejectReason = "attached ceremony owner does not match request owner";
              return null;
            }
            if (
              message.deckHash
              && String(requestCeremony.deckHash || "") !== String(message.deckHash || "")
            ) {
              requestCeremonyRejectReason = "attached ceremony deckHash does not match request deckHash";
              return null;
            }
            if (
              message.ceremonyContext
              && String(requestCeremony.context || "") !== String(message.ceremonyContext || "")
            ) {
              requestCeremonyRejectReason = "attached ceremony context does not match request context";
              return null;
            }
            return cloneMultiplayerPayload(requestCeremony);
          })()
          : null;
      const liveCeremonies = [...liveZiffleCeremoniesRef.current.values()]
        .map(compactZiffleCeremonyForDiagnostics)
        .filter(Boolean);
      const payloadCeremonies = (matchStartPayloadRef.current?.ziffleCeremonies || [])
        .map(compactZiffleCeremonyForDiagnostics)
        .filter(Boolean);
      const currentSession = multiplayerRef.current;
      const actionAuthorizationDebug = {};
      diagnostics = {
        requestId: String(message.requestId || ""),
        requesterPeerId: String(message.requesterPeerId || ""),
        responderPeerId: String(currentSession.localPeerId || ""),
        connectionPeerId: String(conn?.peer || ""),
        responderRole: String(currentSession.role || ""),
        responderMode: String(currentSession.mode || ""),
        responderMatchStarted: Boolean(currentSession.matchStarted),
        responderLocalPlayerIndex:
          currentSession.localPlayerIndex == null ? null : Number(currentSession.localPlayerIndex),
        requestedOwner: Number(message.ceremonyOwner),
        requestedDeckHash: String(message.deckHash || ""),
        requestedContext: String(message.ceremonyContext || ""),
        requestedCardPosition: Number(message.cardPosition),
        actionAuthorizationPresent: Boolean(message.actionAuthorization),
        actionAuthorization: compactActionAuthorizationForDiagnostics(message.actionAuthorization),
        actionAuthorizationDebug,
        attachedCeremonyPresent: Boolean(requestCeremony),
        attachedCeremony: requestCeremonySummary,
        attachedCeremonyRejectReason: requestCeremonyRejectReason,
        matchStartPayloadPresent: Boolean(matchStartPayloadRef.current),
        matchStartAuditMatchId: String(matchStartPayloadRef.current?.auditMatchId || ""),
        liveCeremonies,
        payloadCeremonies,
        sessionPlayers: (currentSession.players || []).map((player) => ({
          index: Number(player.index),
          name: String(player.name || ""),
          peerId: String(player.peerId || ""),
          connected: player.connected !== false,
          hasZiffleKey: Boolean(player.ziffleKey),
        })),
      };
      const ceremony = ziffleCeremonyForOwner(message.ceremonyOwner, lookup)
        || attachedCeremony
        || await waitForZiffleCeremony(message.ceremonyOwner, lookup);
      if (!ceremony) {
        const error = new Error("Unknown ziffle ceremony");
        error.ziffleDiagnostics = diagnostics;
        throw error;
      }
      const rawCardPositions = Array.isArray(message.cardPositions) && message.cardPositions.length > 0
        ? message.cardPositions
        : [message.cardPosition];
      const cardPositions = normalizeZiffleCardPositions(rawCardPositions, ceremony, {
        label: "Ziffle reveal-token request",
      });
      const revealTokenPerf = {
        request_id: String(message.requestId || ""),
        requester,
        owner: requestedOwner,
        positions: cardPositions.length,
        sample_positions: cardPositions.slice(0, 12).map(Number),
        request_bytes: payloadSizeBytes(message),
      };
      recordPeerSyncPerf("ziffle_reveal_token_request:received", revealTokenPerf);
      const authorizedByCryptoRequest = ziffleRevealAuthorizedByOutboundCryptoRequest(
        message,
        requester,
        requestedOwner,
        cardPositions
      );
      const authorizedByAction = authorizedByCryptoRequest
        ? false
        : await ziffleRevealAuthorizedByAction(
          message,
          requester,
          requestedOwner,
          cardPositions,
          ceremony,
          actionAuthorizationDebug
        );
      // The visible-state fallback only applies to the deck owner re-opening
      // positions it is already entitled to; other requesters must carry an
      // explicit authorization.
      const allowedPositions =
        (authorizedByCryptoRequest || authorizedByAction || Number(requester) !== requestedOwner)
          ? new Set()
          : await waitForAuthorizedZiffleRevealPositions(
            requestedOwner,
            String(ceremony.deckHash || message.deckHash || ""),
            cardPositions
          );
      for (const position of cardPositions) {
        if (
          !allowedPositions.has(Number(position))
          && !authorizedByCryptoRequest
          && !authorizedByAction
        ) {
          throw new Error("Ziffle reveal-token request is not authorized by the visible hidden-zone state");
        }
      }
      if (authorizedByAction && message.actionAuthorization?.actionIntent) {
        const requestPayload = cloneMultiplayerPayload(message);
        await timePeerSyncPhase(
          "ziffle_reveal_token_request:remember_intent",
          revealTokenPerf,
          async () => rememberPendingActionIntent(message.actionAuthorization.actionIntent, {
            requestType: "ziffle_reveal_token_request",
            requestId: String(message.requestId || ""),
            requestPayload,
            requestPayloadHash: await sha256Hex(canonicalMultiplayerPayload(requestPayload)),
            responseTimeoutMs: ziffleRevealTokenTimeoutMs(cardPositions.length, ceremony),
            requestedAtMs: Date.now(),
          })
        );
      }
      setStatus(`Generating hidden-card reveal payloads for ${cardPositions.length} card${cardPositions.length === 1 ? "" : "s"}`);
      const tokens = await timePeerSyncPhase(
        "ziffle_reveal_token_request:build_local_tokens",
        revealTokenPerf,
        () => buildLocalZiffleRevealTokens(ceremony, cardPositions)
      );
      recordPeerSyncPerf("ziffle_reveal_token_request:send_response", {
        ...revealTokenPerf,
        tokens: Array.isArray(tokens) ? tokens.length : 0,
        response_bytes: payloadSizeBytes({ tokens }),
      });
      safeSend(conn, {
        type: "ziffle_reveal_token_response",
        protocolVersion: PROTOCOL_VERSION,
        requestId: message.requestId,
        token: tokens[0] || null,
        tokens,
      });
    } catch (err) {
      const ziffleDiagnostics = err?.ziffleDiagnostics || diagnostics;
      if (ziffleDiagnostics) {
        console.warn("Ironsmith ziffle reveal-token request failed", ziffleDiagnostics);
      }
      safeSend(conn, {
        type: "ziffle_reveal_token_response",
        protocolVersion: PROTOCOL_VERSION,
        requestId: message.requestId,
        error: toErrorMessage(err),
        diagnostics: ziffleDiagnostics || null,
      });
    }
  }

  function ziffleRoutePeerCandidates(peerId) {
    const target = String(peerId || "").trim();
    if (!target) return [];
    const players = routingPlayers();
    const candidates = [target];
    const hostRoute = currentHostRouteInfo();
    for (const player of players || []) {
      const stablePeerId = String(player?.peerId || "").trim();
      const currentPeerId = String(player?.currentPeerId || "").trim();
      if (stablePeerId !== target && currentPeerId !== target) continue;
      if (currentPeerId) candidates.push(currentPeerId);
      if (stablePeerId) candidates.push(stablePeerId);
      if (
        hostRoute.peerId
        && hostRoute.index != null
        && Number(player?.index) === Number(hostRoute.index)
      ) {
        candidates.push(hostRoute.peerId);
      }
    }
    return [...new Set(candidates.filter(Boolean))];
  }

  function openConnectionForPeerCandidates(connections, candidates) {
    for (const candidate of candidates) {
      const conn = connections.get(candidate);
      if (conn && conn.open !== false) return conn;
    }
    for (const conn of connections.values()) {
      if (!conn || conn.open === false) continue;
      if (candidates.includes(String(conn.peer || "").trim())) return conn;
    }
    return null;
  }

  function openZiffleRoute(peerId) {
    const target = String(peerId || "").trim();
    const session = multiplayerRef.current;
    if (!target || target === session.localPeerId) return null;
    const candidates = ziffleRoutePeerCandidates(target);
    if (session.role === "host") {
      return (
        openConnectionForPeerCandidates(clientConnectionsRef.current, candidates)
        || openConnectionForPeerCandidates(peerConnectionsRef.current, candidates)
      );
    }
    const hostRoute = currentHostRouteInfo();
    if (
      candidates.includes(String(session.hostPeerId || "").trim())
      || (hostRoute.peerId && candidates.includes(hostRoute.peerId))
    ) {
      const conn = hostConnectionRef.current;
      if (conn && conn.open !== false) return conn;
      return openConnectionForPeerCandidates(peerConnectionsRef.current, candidates);
    }
    return openConnectionForPeerCandidates(peerConnectionsRef.current, candidates);
  }

  async function waitForZiffleRoute(peerId, timeoutMs = Math.min(PROTOCOL_RESPONSE_TIMEOUT_MS, 30000)) {
    const target = String(peerId || "").trim();
    const started = Date.now();
    const nextDirectAttemptAt = new Map();
    while (Date.now() - started < timeoutMs) {
      const conn = openZiffleRoute(target);
      if (conn) return conn;
      const session = multiplayerRef.current;
      const now = Date.now();
      for (const candidate of ziffleRoutePeerCandidates(target)) {
        if (
          !candidate
          || candidate === String(session.localPeerId || "").trim()
          || Number(nextDirectAttemptAt.get(candidate) || 0) > now
        ) {
          continue;
        }
        nextDirectAttemptAt.set(candidate, now + 3000);
        connectDirectPeer(candidate);
      }
      await sleep(50);
    }
    const session = multiplayerRef.current;
    const error = new Error(`No direct ziffle route to peer ${target || peerId}`);
    error.ziffleDiagnostics = {
      localPeerId: String(session.localPeerId || ""),
      role: String(session.role || ""),
      hostPeerId: String(session.hostPeerId || ""),
      hostRoute: currentHostRouteInfo(),
      hostConnectionPeer: String(hostConnectionRef.current?.peer || ""),
      hostConnectionOpen: Boolean(hostConnectionRef.current && hostConnectionRef.current.open !== false),
      peerConnectionPeers: [...peerConnectionsRef.current.entries()].map(([key, conn]) => ({
        key: String(key || ""),
        peer: String(conn?.peer || ""),
        open: Boolean(conn && conn.open !== false),
      })),
      clientConnectionPeers: [...clientConnectionsRef.current.entries()].map(([key, conn]) => ({
        key: String(key || ""),
        peer: String(conn?.peer || ""),
        open: Boolean(conn && conn.open !== false),
      })),
      candidates: ziffleRoutePeerCandidates(target),
      players: routingPlayers().map((player) => ({
        index: player?.index == null ? null : Number(player.index),
        peerId: String(player?.peerId || ""),
        currentPeerId: String(player?.currentPeerId || ""),
        connected: player?.connected !== false,
      })),
    };
    throw error;
  }

  async function sendDirectProtocolMessage(peerId, payload, options = {}) {
    const target = String(peerId || "").trim();
    if (!target) {
      throw new Error(`Missing peer route for ${String(payload?.type || "protocol message")}`);
    }
    const session = multiplayerRef.current;
    if (target === String(session.localPeerId || "").trim()) {
      throw new Error(`Cannot request ${String(payload?.type || "protocol message")} from the local peer`);
    }
    const attempts = Math.max(1, Number(options.attempts || 3));
    const routeTimeoutMs = Math.max(250, Number(options.routeTimeoutMs || 5000));
    const retryDelayMs = Math.max(50, Number(options.retryDelayMs || 500));
    let lastError = null;
    for (let attempt = 1; attempt <= attempts; attempt += 1) {
      try {
        const conn = await waitForZiffleRoute(target, routeTimeoutMs);
        safeSend(conn, payload);
        return true;
      } catch (err) {
        lastError = err;
      }
      if (sendDirectPeerMessage(target, payload)) {
        return true;
      }
      if (attempt < attempts) {
        await sleep(retryDelayMs);
      }
    }
    if (lastError) throw lastError;
    throw new Error(`No direct ziffle route to peer ${target}`);
  }

  async function collectZiffleRevealTokens(ceremony, cardPosition, options = {}) {
    const tokens = await collectZiffleRevealTokensBatch(ceremony, [cardPosition], options);
    return tokens
      .filter((token) => Number(token.cardPosition ?? cardPosition) === Number(cardPosition))
      .map((token) => ({
        player: token.player,
        publicKeyHex: token.publicKeyHex,
        tokenHex: token.tokenHex,
        proofHex: token.proofHex,
      }));
  }

  async function collectZiffleRevealTokensBatch(ceremony, cardPositions, options = {}) {
    const session = multiplayerRef.current;
    const localIndex = resolveLocalPlayerIndex(session);
    const players = reindexPlayers(
      (session.players || []).length > 0
        ? session.players
        : (
          matchStartPayloadRef.current?.currentPlayers
          || matchStartPayloadRef.current?.players
          || []
        )
    );
    const positions = normalizeZiffleCardPositions(cardPositions, ceremony, {
      allowEmpty: true,
      label: "Ziffle reveal token batch",
    });
    if (positions.length === 0) return [];
    const batchPerf = {
      owner: ceremony?.owner == null ? null : Number(ceremony.owner),
      positions: positions.length,
      sample_positions: positions.slice(0, 12),
      key_players: Array.isArray(ceremony?.keys)
        ? ceremony.keys.map((key) => Number(key?.player)).filter((player) => Number.isFinite(player))
        : [],
      command: summarizePeerCommand(options.command),
      seq: options.seq == null ? null : Number(options.seq),
      actor: options.actorIndex == null ? null : Number(options.actorIndex),
    };
    recordPeerSyncPerf("ziffle_reveal_token_batch:start", batchPerf);
    const actionAuthorization = options.command
      ? {
        matchId: currentAuditMatchId(),
        seq: Number(options.seq ?? Number(session.lastAppliedSequence || 0) + 1),
        requesterIndex: Number(localIndex),
        ...(options.actorIndex !== null && options.actorIndex !== undefined
          ? { actorIndex: Number(options.actorIndex) }
          : {}),
        prevStateHash: String(options.prevStateHash || auditStateHashRef.current || INITIAL_AUDIT_STATE_HASH),
        preActionPublicCheckpointHash: String(
          options.preActionPublicCheckpointHash
          || options.publicCheckpointHash
          || ""
        ),
        command: cloneMultiplayerPayload(options.command),
        requirements: cloneMultiplayerPayload(options.requirements || []),
        ...(options.actionIntent ? { actionIntent: cloneMultiplayerPayload(options.actionIntent) } : {}),
        ...(options.actionAudit ? { actionAudit: cloneMultiplayerPayload(options.actionAudit) } : {}),
      }
      : null;
    const ceremonyKeys = Array.isArray(ceremony.keys) ? ceremony.keys : [];
    const tokenGroups = new Array(ceremonyKeys.length).fill(null);
    for (let keyIndex = 0; keyIndex < ceremonyKeys.length; keyIndex += 1) {
      const key = ceremonyKeys[keyIndex];
      const tokenPlayer = Number(key.player);
      const cached = cachedZiffleRevealTokens(ceremony, tokenPlayer, positions);
      if (cached) {
        recordPeerSyncPerf("ziffle_reveal_token_batch:cache_hit", {
          ...batchPerf,
          token_player: tokenPlayer,
          tokens: Array.isArray(cached) ? cached.length : 0,
        });
        tokenGroups[keyIndex] = cached;
        continue;
      }
      if (tokenPlayer !== Number(localIndex)) continue;
      const localTokens = await timePeerSyncPhase(
        "ziffle_reveal_token_batch:build_local_tokens",
        {
          ...batchPerf,
          token_player: tokenPlayer,
        },
        () => buildLocalZiffleRevealTokens(ceremony, positions)
      );
      rememberZiffleRevealTokens(ceremony, localTokens, positions);
      tokenGroups[keyIndex] = cachedZiffleRevealTokens(ceremony, tokenPlayer, positions) || localTokens;
    }
    const remoteTokenTasks = ceremonyKeys.map(async (key, keyIndex) => {
      if (tokenGroups[keyIndex]) return;
      const tokenPlayer = Number(key.player);
      const cached = cachedZiffleRevealTokens(ceremony, tokenPlayer, positions);
      if (cached) {
        recordPeerSyncPerf("ziffle_reveal_token_batch:cache_hit", {
          ...batchPerf,
          token_player: tokenPlayer,
          tokens: Array.isArray(cached) ? cached.length : 0,
        });
        tokenGroups[keyIndex] = cached;
        return;
      }
      const peer = players.find((player) => Number(player.index) === tokenPlayer) || {
        index: tokenPlayer,
      };
      const routePeerId = routePeerIdForPlayer(peer);
      if (!routePeerId) {
        throw new Error(`Missing peer for ziffle reveal token player ${key.player}`);
      }
      const peerLabel = peer.name || `Player ${Number(key.player) + 1}`;
      setStatus(`Waiting for ${peerLabel} to generate hidden-card reveal payloads`);
      const requestId = makeZiffleRequestId("ziffle-reveal");
      const revealTokenTimeoutMs = ziffleRevealTokenTimeoutMs(positions.length, ceremony);
      const responseTimeoutMs = actionAuthorization
        ? revealTokenTimeoutMs
        : Math.max(revealTokenTimeoutMs, PROTOCOL_RESPONSE_TIMEOUT_MS);
      const requestDiagnostics = {
        requestId,
        localPeerId: String(session.localPeerId || ""),
        localRole: String(session.role || ""),
        localMode: String(session.mode || ""),
        targetPeerId: routePeerId,
        targetStablePeerId: String(peer.peerId || ""),
        targetPlayerIndex: tokenPlayer,
        ceremony: compactZiffleCeremonyForDiagnostics(ceremony),
        cardPosition: positions.length === 1 ? positions[0] : null,
        cardPositions: positions,
        matchStartPayloadPresent: Boolean(matchStartPayloadRef.current),
        matchStartAuditMatchId: String(matchStartPayloadRef.current?.auditMatchId || ""),
        actionIntentKey: actionAuthorization?.actionIntent
          ? actionIntentKey(actionAuthorization.actionIntent)
          : "",
      };
      const requestedAtMs = Date.now();
      const waiter = waitForZiffleRevealToken(requestId, responseTimeoutMs, requestDiagnostics, {
        peerIndex: tokenPlayer,
        peerName: peerLabel,
        description:
          `${peerLabel} is generating reveal proof material for ${positions.length} hidden `
          + `card${positions.length === 1 ? "" : "s"}.`,
        operation: "Waiting for reveal proof",
        detail: zifflePositionsDetail(positions),
        progressCurrent: 0,
        progressTotal: positions.length,
        responseTimeoutMs,
      });
      const includeCeremonyInRevealRequest =
        Boolean(options.includeCeremonyInRevealRequest)
        || Boolean(actionAuthorization);
      const requestPayload = {
        type: "ziffle_reveal_token_request",
        protocolVersion: PROTOCOL_VERSION,
        requestId,
        ceremonyOwner: Number(ceremony.owner),
        deckHash: String(ceremony.deckHash || ""),
        ceremonyContext: String(ceremony.context || ""),
        ...(includeCeremonyInRevealRequest ? { ceremony: cloneMultiplayerPayload(ceremony) } : {}),
        cardPosition: positions[0],
        cardPositions: positions,
        requesterPeerId: session.localPeerId || "",
        requesterIndex: localIndex,
        cryptoMaterialRequestId: options.cryptoMaterialRequestId || "",
        ...(actionAuthorization ? { actionAuthorization } : {}),
      };
      const requestPerf = {
        ...batchPerf,
        token_player: tokenPlayer,
        request_id: requestId,
        peer_id: routePeerId,
        request_bytes: payloadSizeBytes(requestPayload),
      };
      recordPeerSyncPerf("ziffle_reveal_token_batch:send_request", requestPerf);
      await sendDirectProtocolMessage(routePeerId, requestPayload);
      const remoteTokens = await timePeerSyncPhase(
        "ziffle_reveal_token_batch:wait_response",
        requestPerf,
        () => waitForProtocolResponse(waiter, {
          basisSequence: Number(multiplayerRef.current.lastAppliedSequence || 0),
          targetPlayerIndex: tokenPlayer,
          targetPeerId: routePeerId,
          requesterIndex: localIndex,
          requestType: requestPayload.type,
          requestId,
          requestPayload,
          responseTimeoutMs,
          requestedAtMs,
        })
      );
      recordPeerSyncPerf("ziffle_reveal_token_batch:received_response", {
        ...requestPerf,
        tokens: Array.isArray(remoteTokens) ? remoteTokens.length : 0,
        response_bytes: payloadSizeBytes(remoteTokens),
      });
      rememberZiffleRevealTokens(ceremony, remoteTokens, positions);
      tokenGroups[keyIndex] = cachedZiffleRevealTokens(ceremony, tokenPlayer, positions) || remoteTokens;
    });
    await Promise.all(remoteTokenTasks);
    const tokens = tokenGroups
      .filter(Boolean)
      .flatMap((group) => Array.isArray(group) ? group : [group]);
    recordPeerSyncPerf("ziffle_reveal_token_batch:done", {
      ...batchPerf,
      tokens: tokens.length,
      bytes: payloadSizeBytes(tokens),
    });
    return tokens;
  }

  async function buildLiveZiffleShuffleProofs(requirements, seq) {
    const players = reindexPlayers(matchStartPayloadRef.current?.players || multiplayerRef.current.players || []);
    const keys = zifflePublicKeysForPlayers(players);
    const keyContext = currentAuditMatchId();
    const ceremonies = (requirements || []).map((requirement) => {
      const beforeOrder = normalizeShuffleOrder(requirement?.beforeOrder ?? requirement?.before_order);
      const afterOrder = normalizeShuffleOrder(requirement?.afterOrder ?? requirement?.after_order);
      if (beforeOrder.length > 0 && afterOrder.length > 0 && beforeOrder.length !== afterOrder.length) {
        throw new Error("Verifiable shuffle order length mismatch");
      }
      const deckCount = Number(afterOrder.length || beforeOrder.length || 0);
      if (deckCount <= 1) return null;
      return {
        id: String(requirement.id || ""),
        requirement,
        requirementId: String(requirement.id || ""),
        owner: Number(requirement.owner),
        zone: String(requirement.zone || "library"),
        deckCount,
        keyContext,
        context: [
          keyContext,
          "action",
          Number(seq),
          "shuffle",
          String(requirement.id || ""),
          Number(requirement.owner),
          String(requirement.zone || "library"),
        ].join(":"),
        keys,
        beforeOrder,
        afterOrder,
      };
    }).filter(Boolean);
    const completed = await runBatchedZiffleShuffleCeremonies(ceremonies, players, {
      keys,
      kind: "action",
    });
	    return completed.map((ceremony) => {
	      const proof = {
	        type: "ziffle_shuffle",
        requirementId: String(ceremony.requirementId || ""),
        owner: Number(ceremony.owner),
        zone: String(ceremony.zone || "library"),
        epoch: Number(seq),
        deckCount: Number(ceremony.deckCount),
        context: String(ceremony.context || ""),
        keyContext: String(ceremony.keyContext || ceremony.context || ""),
        keys: cloneMultiplayerPayload(ceremony.keys || []),
        steps: cloneMultiplayerPayload(ceremony.steps || []),
	        deckHash: String(ceremony.deckHash || ""),
		        beforeOrder: normalizeShuffleOrder(ceremony.beforeOrder),
		        afterOrder: normalizeShuffleOrder(ceremony.afterOrder),
		        authenticatedOrder: true,
		      };
	      verifiedShuffleProofsRef.current.add(proof);
	      rememberLocalZiffleCeremonyForLookup(proof);
	      return proof;
	    });
	  }

  async function buildLocalShuffleProofsForRequirements(cryptoRequirements = [], seq) {
    const requirements = (cryptoRequirements || []).filter(
      (requirement) => ziffleRequirementType(requirement) === "verifiable_shuffle"
    );
    return buildLiveZiffleShuffleProofs(requirements, seq);
  }

  const assertZiffleShuffleProofBoundToSignedMatch = useCallback((proof, requirement = null, options = {}) => {
    const matchPayload = options.matchPayload || matchStartPayloadRef.current;
    const players = reindexPlayers(matchPayload?.players || multiplayerRef.current.players || []);
    const playerSeats = new Set(players.map((player) => Number(player.index)));
    const owner = normalizePlayerIndex(proof?.owner);
    if (owner === null || !playerSeats.has(owner)) {
      throw new Error("Ziffle shuffle proof references an unknown player");
    }
    if (requirement && owner !== normalizePlayerIndex(requirement.owner)) {
      throw new Error(`Ziffle shuffle proof owner mismatch for player ${Number(requirement.owner) + 1}`);
    }
    const zone = String(proof?.zone || "library");
    const expectedZone = String(requirement?.zone || "library");
    if (zone !== expectedZone) {
      throw new Error(`Ziffle shuffle proof zone mismatch for player ${owner + 1}`);
    }
    const expectedKeys = signedZiffleKeysForPayload(matchPayload);
    if (!expectedKeys.length || canonicalMultiplayerPayload(proof?.keys || []) !== canonicalMultiplayerPayload(expectedKeys)) {
      throw new Error("Ziffle shuffle proof is not bound to the signed ziffle key roster");
    }
    const expectedMatchId = String(
      matchPayload?.auditMatchId
        || matchPayload?.matchId
        || currentAuditMatchId()
        || ""
    );
    const proofContext = String(proof?.context || "");
    const proofKeyContext = String(proof?.keyContext || proof?.context || "");
    if (
      !expectedMatchId
      || (proofContext !== expectedMatchId && !proofContext.startsWith(`${expectedMatchId}:`))
    ) {
      throw new Error("Ziffle shuffle proof is bound to a different match");
    }
    if (proofKeyContext !== expectedMatchId) {
      throw new Error("Ziffle shuffle proof uses a mismatched ziffle key context");
    }
  }, [currentAuditMatchId, signedZiffleKeysForPayload]);

  async function verifyShuffleProofsForRequirements(requirements = [], shuffleProofs = [], options = {}) {
    const currentGame = gameRef.current;
    const allowAfterOrderMismatch = Boolean(options.allowAfterOrderMismatch);
    let verifiedCount = 0;
    let skippedCount = 0;
    let verifyMs = 0;
    const pendingVerifications = [];
    for (const requirement of requirements || []) {
      if (ziffleRequirementType(requirement) !== "verifiable_shuffle") continue;
      const proof = (shuffleProofs || []).find((entry) =>
        shuffleProofMatchesRequirement(entry, requirement)
      );
      if (!proof) {
        throw new Error(`Missing verifiable shuffle proof for player ${Number(requirement.owner) + 1}`);
      }
      const requirementBefore = normalizeShuffleOrder(requirement.beforeOrder ?? requirement.before_order);
      const requirementAfter = normalizeShuffleOrder(requirement.afterOrder ?? requirement.after_order);
      const proofBefore = normalizeShuffleOrder(proof.beforeOrder ?? proof.before_order);
      const proofAfter = normalizeShuffleOrder(proof.afterOrder ?? proof.after_order);
      const beforeMap = shuffleOrderIdMap(proofBefore, requirementBefore);
      const afterMap = shuffleOrderIdMap(proofAfter, requirementAfter);
      const localizedProofBefore = beforeMap ? localizeShuffleOrder(proofBefore, beforeMap) : proofBefore;
      const localizedProofAfter = afterMap ? localizeShuffleOrder(proofAfter, afterMap) : proofAfter;
      if (
        requirementBefore.length > 0
        && !sameShuffleOrder(localizedProofBefore, requirementBefore)
      ) {
        throw new Error(`Verifiable shuffle before-order mismatch for player ${Number(requirement.owner) + 1}`);
      }
      if (
        requirementAfter.length > 0
        && !sameShuffleOrder(localizedProofAfter, requirementAfter)
      ) {
        if (!allowAfterOrderMismatch) {
          throw new Error(`Verifiable shuffle after-order mismatch for player ${Number(requirement.owner) + 1}`);
        }
      }
      if (verifiedShuffleProofsRef.current.has(proof)) {
        skippedCount += 1;
        continue;
      }
      if (!currentGame || typeof currentGame.ziffleVerifyShuffle !== "function") {
        throw new Error("Ziffle mental-poker backend is not available");
      }
      assertZiffleShuffleProofBoundToSignedMatch(proof, requirement);
      pendingVerifications.push({ proof });
    }
    if (pendingVerifications.length > 0) {
      const verifyStartedAt = nowMonotonicMs();
      const verifiedProofs = await Promise.all(pendingVerifications.map(async ({ proof }) => {
        const verified = await currentGame.ziffleVerifyShuffle({
          deckCount: Number(proof.deckCount),
          context: String(proof.context || ""),
          keyContext: String(proof.keyContext || proof.context || ""),
          keys: cloneMultiplayerPayload(proof.keys || []),
          steps: cloneMultiplayerPayload(proof.steps || []),
        });
        if (String(verified.deckHash || "") !== String(proof.deckHash || "")) {
          throw new Error(`Ziffle shuffle proof mismatch for player ${Number(proof.owner) + 1}`);
        }
        return proof;
      }));
      verifyMs = nowMonotonicMs() - verifyStartedAt;
      for (const proof of verifiedProofs) {
        verifiedShuffleProofsRef.current.add(proof);
      }
      verifiedCount = verifiedProofs.length;
    }
    if (verifiedCount > 0 || skippedCount > 0) {
      recordZiffleShufflePerf({
        kind: "verify",
        verifiedCount,
        skippedCount,
        verifyMs: Math.round(verifyMs),
      });
    }
  }

	  async function applyVerifiedShuffleProofs(shuffleProofs = []) {
	    const currentGame = gameRef.current;
	    if (!currentGame || typeof currentGame.applyVerifiedHiddenLibraryShuffle !== "function") return;
	    for (const proof of shuffleProofs || []) {
	      if (String(proof?.zone || "library") !== "library") continue;
	      let afterOrder = normalizeShuffleOrder(proof.afterOrder ?? proof.after_order);
	      if (typeof currentGame.exportSyncCheckpoint === "function") {
	        try {
	          const checkpoint = await currentGame.exportSyncCheckpoint();
	          const currentLibrary = playerLibraryOrderFromCheckpoint(checkpoint, proof.owner);
	          if (currentLibrary.length > 0) {
	            const projectedAfterOrder = projectShuffleOrderToCurrentLibrary(afterOrder, currentLibrary);
	            if (projectedAfterOrder) {
	              afterOrder = projectedAfterOrder;
	            } else if (
	              afterOrder.length >= currentLibrary.length
	              && String(proof.deckHash || "")
	            ) {
	              afterOrder = currentLibrary;
	            }
	          }
	        } catch {
	          // Fall back to the proof-carried order; the engine will validate coverage.
	        }
	      }
	      try {
	        await currentGame.applyVerifiedHiddenLibraryShuffle({
	          owner: Number(proof.owner),
	          deckHash: String(proof.deckHash || ""),
	          afterOrder,
        });
      } catch (err) {
        throw new Error(
          `${String(err?.message || err || "failed to apply verified shuffle")}; `
          + `shuffle proof owner ${Number(proof.owner)} requirement ${String(proof.requirementId || "")} `
          + `afterOrderLen ${afterOrder.length} firstAfterOrder ${afterOrder.slice(0, 12).join(",")}`
        );
	      }
		      clearOwnerZiffleOpeningCache(proof.owner);
			      const localProof = {
			        ...cloneMultiplayerPayload(proof),
			        afterOrder,
			        after_order: afterOrder,
			        authenticatedOrder: true,
			      };
		      liveZiffleCeremoniesRef.current.set(Number(proof.owner), localProof);
	      rememberLocalZiffleCeremonyForLookup(localProof);
	    }
	  }

  async function rngCommitmentForNonce(nonceHex) {
    return sha256Hex(canonicalJson({
      domain: "ironsmith-rng-commit-v1",
      nonceHex: String(nonceHex || ""),
    }));
  }

  async function signRngCommitmentEntry({
    matchId,
    seq,
    requirementId,
    requestId,
    requester,
    player,
    commitmentHex,
  }) {
    const { keyPair } = await ensureAuditIdentity();
    const payload = rngCommitmentPayload({
      matchId,
      seq,
      requirementId,
      requestId,
      requester,
      player,
      commitmentHex,
    });
    return {
      player: Number(player),
      requester: Number(requester),
      requestId: String(requestId || ""),
      commitmentHex: String(commitmentHex || ""),
      signature: await signAuditPayload(keyPair, payload),
    };
  }

  async function signRngRevealEntry({
    matchId,
    seq,
    requirementId,
    requestId,
    commitRequestId,
    requester,
    player,
    nonceHex,
    commitmentHex,
  }) {
    const { keyPair } = await ensureAuditIdentity();
    const payload = rngRevealPayload({
      matchId,
      seq,
      requirementId,
      requestId,
      commitRequestId,
      requester,
      player,
      nonceHex,
      commitmentHex,
    });
    return {
      player: Number(player),
      requester: Number(requester),
      requestId: String(requestId || ""),
      commitRequestId: String(commitRequestId || ""),
      nonceHex: String(nonceHex || ""),
      commitmentHex: String(commitmentHex || ""),
      signature: await signAuditPayload(keyPair, payload),
    };
  }

  async function verifyRngCommitmentEntry(entry, {
    matchId,
    seq,
    requirementId,
    requester,
    player,
  }) {
    if (!entry?.signature) {
      throw new Error("Random commitment response is missing its signature");
    }
    const signer = Number(player ?? entry.player);
    const publicKey = await importCachedAuditPublicKey(publicKeyForAuditSigner(signer));
    const valid = await verifyAuditPayload(publicKey, rngCommitmentPayload({
      matchId,
      seq,
      requirementId,
      requestId: entry.requestId,
      requester: requester ?? entry.requester,
      player: signer,
      commitmentHex: entry.commitmentHex,
    }), entry.signature || "");
    if (!valid) {
      throw new Error("Random commitment response signature is invalid");
    }
  }

  async function verifyRngRevealEntry(entry, {
    matchId,
    seq,
    requirementId,
    requester,
    player,
  }) {
    if (!entry?.signature) {
      throw new Error("Random reveal response is missing its signature");
    }
    const signer = Number(player ?? entry.player);
    const publicKey = await importCachedAuditPublicKey(publicKeyForAuditSigner(signer));
    const valid = await verifyAuditPayload(publicKey, rngRevealPayload({
      matchId,
      seq,
      requirementId,
      requestId: entry.requestId,
      commitRequestId: entry.commitRequestId,
      requester: requester ?? entry.requester,
      player: signer,
      nonceHex: entry.nonceHex,
      commitmentHex: entry.commitmentHex,
    }), entry.signature || "");
    if (!valid) {
      throw new Error("Random reveal response signature is invalid");
    }
  }

  function currentHostRouteInfo() {
    const session = multiplayerRef.current;
    const payload = matchStartPayloadRef.current || {};
    const hostPeerId = firstOnlinePeerId(
      payload.currentHostPeerId,
      payload.hostPeerId,
      session.hostPeerId
    );
    const players = [
      ...(Array.isArray(session.players) ? session.players : []),
      ...(Array.isArray(payload.currentPlayers) ? payload.currentPlayers : []),
      ...(Array.isArray(payload.players) ? payload.players : []),
    ];
    const matchedHost = hostPeerId
      ? players.find((entry) =>
        String(entry?.peerId || "").trim() === hostPeerId
        || String(entry?.currentPeerId || "").trim() === hostPeerId
      )
      : null;
    const sessionHostIndex = (
      session.role === "host"
      && hostPeerId
      && String(session.localPeerId || "").trim() === hostPeerId
    )
      ? normalizePlayerIndex(session.localPlayerIndex)
      : null;
    const hostIndex =
      sessionHostIndex
      ?? normalizePlayerIndex(payload.currentHostPlayerIndex)
      ?? normalizePlayerIndex(matchedHost?.index)
      ?? normalizePlayerIndex(payload.genesis?.hostSeat)
      ?? (hostPeerId ? 0 : null);
    return { peerId: hostPeerId, index: hostIndex };
  }

  function routingPlayers() {
    const session = multiplayerRef.current;
    const payload = matchStartPayloadRef.current || {};
    return [
      ...(Array.isArray(session.players) ? session.players : []),
      ...(Array.isArray(payload.currentPlayers) ? payload.currentPlayers : []),
      ...(Array.isArray(payload.players) ? payload.players : []),
    ];
  }

  function playerIndexForPeerId(peerId) {
    const peer = String(peerId || "");
    const players = routingPlayers();
    const player = players.find((entry) =>
      String(entry?.peerId || "") === peer
      || String(entry?.currentPeerId || "") === peer
    );
    if (player?.index != null) return Number(player.index);
    const hostRoute = currentHostRouteInfo();
    if (peer && hostRoute.peerId && peer === hostRoute.peerId) {
      return hostRoute.index == null ? null : Number(hostRoute.index);
    }
    return null;
  }

  function routePeerIdForPlayer(player) {
    const index = normalizePlayerIndex(player?.index);
    const session = multiplayerRef.current;
    const livePlayers = routingPlayers();
    const indexedPlayers = index == null
      ? []
      : livePlayers.filter((entry) => Number(entry?.index) === index);
    const hostPeerId = String(session.hostPeerId || "").trim();
    const liveCurrentPeerId = firstOnlinePeerId(
      ...indexedPlayers.map((entry) => entry?.currentPeerId)
    );
    const liveStablePeerId = firstOnlinePeerId(
      ...indexedPlayers.map((entry) => entry?.peerId)
    );
    const stablePeerId = String(player?.peerId || "").trim();
    const currentPeerId = String(player?.currentPeerId || "").trim();
    const hostRoute = currentHostRouteInfo();
    if (
      session.role === "client"
      && hostRoute.peerId
      && hostRoute.index != null
      && index === hostRoute.index
    ) {
      return hostRoute.peerId;
    }
    const matchesHostPeerId = Boolean(
      hostPeerId
      && (
        liveStablePeerId === hostPeerId
        || liveCurrentPeerId === hostPeerId
        || stablePeerId === hostPeerId
        || currentPeerId === hostPeerId
      )
    );
    if (
      matchesHostPeerId
      && (
        hostRoute.index == null
        || index == null
        || Number(index) === Number(hostRoute.index)
      )
    ) {
      return hostPeerId;
    }
    return firstOnlinePeerId(
      liveCurrentPeerId,
      liveStablePeerId,
      currentPeerId,
      stablePeerId
    );
  }

  function fairRandomRequirementId(requirement) {
    return String(
      requirement?.id
      || requirement?.requirementId
      || requirement?.requirement_id
      || ""
    );
  }

  async function fairRandomRequestContextKey({
    matchId,
    seq,
    requirement,
    requester,
    actorIndex,
    prevStateHash,
    publicCheckpointHash,
    command,
  }) {
    return sha256Hex(canonicalMultiplayerPayload({
      domain: "ironsmith-rng-request-context-v1",
      matchId: String(matchId || ""),
      seq: Number(seq),
      requester: Number(requester),
      actorIndex: Number(actorIndex),
      prevStateHash: String(prevStateHash || ""),
      publicCheckpointHash: String(publicCheckpointHash || ""),
      command: cloneMultiplayerPayload(command),
      requirement: cloneMultiplayerPayload(requirement),
    }));
  }

  async function fairRandomCommitSetHash(contextKey, commits = []) {
    return sha256Hex(canonicalMultiplayerPayload({
      domain: "ironsmith-rng-commit-set-lock-v1",
      contextKey: String(contextKey || ""),
      commits: (Array.isArray(commits) ? commits : [])
        .map((entry) => ({
          player: Number(entry?.player),
          commitmentHex: String(entry?.commitmentHex || ""),
        }))
        .sort((left, right) => Number(left.player) - Number(right.player)),
    }));
  }

  async function validateCompleteRngCommitSet(commits, request) {
    const entries = Array.isArray(commits) ? commits : [];
    const expectedPlayers = reindexPlayers(
      matchStartPayloadRef.current?.players || multiplayerRef.current.players || []
    ).map((player) => Number(player.index)).sort((left, right) => left - right);
    if (entries.length !== expectedPlayers.length) {
      throw new Error("Random reveal request must include every player commitment");
    }
    for (let index = 0; index < expectedPlayers.length; index += 1) {
      if (Number(entries[index]?.player) !== expectedPlayers[index]) {
        throw new Error("Random reveal request commitments must be sorted and complete");
      }
    }
    for (const entry of entries) {
      const player = Number(entry?.player);
      if (Number(entry?.requester) !== Number(request.requesterPlayer)) {
        throw new Error("Random commitment set requester mismatch");
      }
      if (!String(entry?.commitmentHex || "")) {
        throw new Error("Random commitment set is missing a commitment");
      }
      await verifyRngCommitmentEntry(entry, {
        matchId: request.matchId,
        seq: request.seq,
        requirementId: request.requirementId,
        requester: request.requesterPlayer,
        player,
      });
    }
    return cloneMultiplayerPayload(entries);
  }

  async function validateIncomingRngRequest(conn, message, label) {
    const session = multiplayerRef.current;
    if (!session.matchStarted) {
      throw new Error(`${label} received before match start`);
    }
    if (String(message?.matchId || "") !== currentAuditMatchId()) {
      throw new Error(`${label} belongs to a different match`);
    }
    const seq = Number(message?.seq);
    const expectedSeq = Number(session.lastAppliedSequence || 0) + 1;
    if (!Number.isInteger(seq) || seq !== expectedSeq) {
      throw new Error(`${label} has an invalid action sequence`);
    }
    if (!String(message?.requirementId || "")) {
      throw new Error(`${label} is missing a requirement id`);
    }
    const requester = playerIndexForPeerId(conn?.peer);
    if (requester == null) {
      throw new Error(`${label} requester is not a match player`);
    }
    if (normalizePlayerIndex(message?.requesterIndex) !== requester) {
      throw new Error(`${label} requester index does not match the peer`);
    }
    const actorIndex = normalizePlayerIndex(message?.actorIndex);
    if (actorIndex == null || actorIndex !== requester) {
      throw new Error(`${label} requester is not the acting player`);
    }
    const prevStateHash = String(message?.prevStateHash || "");
    if (prevStateHash !== String(auditStateHashRef.current || INITIAL_AUDIT_STATE_HASH)) {
      throw new Error(`${label} is not based on the local transcript head`);
    }
    const publicCheckpointHash = String(message?.publicCheckpointHash || "");
    if (!publicCheckpointHash) {
      throw new Error(`${label} is missing the public checkpoint hash`);
    }
    const command = message?.command;
    if (!command || typeof command !== "object") {
      throw new Error(`${label} is missing the signed command preview`);
    }
    const requestedRequirement = message?.requirement;
    if (
      !requestedRequirement
      || typeof requestedRequirement !== "object"
      || String(requestedRequirement?.type || requestedRequirement?.requirement_type || "") !== "fair_random"
    ) {
      throw new Error(`${label} is missing the fair-random requirement`);
    }
    const decision = gameRef.current
      ? (await gameRef.current.uiState())?.decision
      : stateRef.current?.decision;
    if (
      decision?.player == null
      || Number(decision.player) !== Number(requester)
    ) {
      throw new Error(`${label} was not sent by the active decision player`);
    }
    if (!isDecisionCommandCompatible(decision, command)) {
      throw new Error(`${label} command is not available locally`);
    }
    await verifyCurrentPublicCheckpointHash(
      publicCheckpointHash,
      `${label} public checkpoint does not match local state`
    );
    const actionIntent = await verifySignedActionIntent(message.actionIntent, {
      matchId: currentAuditMatchId(),
      seq,
      actorIndex,
      prevStateHash,
      preActionPublicCheckpointHash: publicCheckpointHash,
      command,
    });
    const liveState = gameRef.current && typeof gameRef.current.uiState === "function"
      ? await gameRef.current.uiState()
      : stateRef.current;
    const previewedRequirements = filterCryptoRequirementsForCommand(
      command,
      liveState,
      freshCryptoRequirementsForSequence(
        seq,
        await previewRequirementsForCommand(command)
      )
    );
    const requestedRequirementKey = cryptoRequirementReplayKey(requestedRequirement);
    const requirement = previewedRequirements.find((entry) =>
      String(entry?.type || entry?.requirement_type || "") === "fair_random"
      && fairRandomRequirementId(entry) === String(message.requirementId || "")
      && cryptoRequirementReplayKey(entry) === requestedRequirementKey
    );
    if (!requirement) {
      throw new Error(`${label} asks for unauthorized fair-random material`);
    }
    const contextKey = await fairRandomRequestContextKey({
      matchId: currentAuditMatchId(),
      seq,
      requirement,
      requester,
      actorIndex,
      prevStateHash,
      publicCheckpointHash,
      command,
    });
    return {
      matchId: currentAuditMatchId(),
      seq,
      requirementId: String(message.requirementId || ""),
      requirement,
      contextKey,
      command,
      actorIndex,
      prevStateHash,
      publicCheckpointHash,
      actionIntent,
      requesterPeerId: String(conn?.peer || ""),
      requesterPlayer: Number(requester),
    };
  }

  async function answerRngCommitRequest(conn, message) {
    try {
      const request = await validateIncomingRngRequest(conn, message, "Random commitment request");
      const localPlayer = resolveLocalPlayerIndex(multiplayerRef.current);
      if (localPlayer == null) {
        throw new Error("Local player seat is not assigned");
      }
      const contributionKey = `${request.contextKey}:${Number(localPlayer)}`;
      let contribution = signedRngCommitmentsRef.current.get(contributionKey);
      if (!contribution) {
        const nonceHex = randomAuditHex(32);
        contribution = {
          nonceHex,
          commitmentHex: await rngCommitmentForNonce(nonceHex),
        };
        signedRngCommitmentsRef.current.set(contributionKey, contribution);
      }
      rngCommitNoncesRef.current.set(String(message.requestId || ""), {
        ...request,
        contributionKey,
        localPlayer,
        nonceHex: contribution.nonceHex,
        commitmentHex: contribution.commitmentHex,
      });
      safeSend(conn, {
        type: "rng_commit_response",
        protocolVersion: PROTOCOL_VERSION,
        requestId: message.requestId,
        commitment: await signRngCommitmentEntry({
          matchId: request.matchId,
          seq: request.seq,
          requirementId: request.requirementId,
          requestId: message.requestId,
          requester: request.requesterPlayer,
          player: localPlayer,
          commitmentHex: contribution.commitmentHex,
        }),
      });
    } catch (err) {
      safeSend(conn, {
        type: "rng_commit_response",
        protocolVersion: PROTOCOL_VERSION,
        requestId: message.requestId,
        error: toErrorMessage(err),
      });
    }
  }

  async function answerRngRevealRequest(conn, message) {
    try {
      const request = await validateIncomingRngRequest(conn, message, "Random reveal request");
      const stored = rngCommitNoncesRef.current.get(String(message.commitRequestId || ""));
      if (!stored) {
        throw new Error("Unknown random commitment request");
      }
      if (
        request.matchId !== stored.matchId
        || Number(request.seq) !== Number(stored.seq)
        || request.requirementId !== stored.requirementId
        || request.requesterPeerId !== stored.requesterPeerId
        || request.contextKey !== stored.contextKey
      ) {
        throw new Error("Random reveal request does not match the commitment request");
      }
      const localPlayer = resolveLocalPlayerIndex(multiplayerRef.current);
      if (localPlayer == null) {
        throw new Error("Local player seat is not assigned");
      }
      const commits = await validateCompleteRngCommitSet(message.commits, request);
      const localCommit = commits.find((entry) => Number(entry?.player) === Number(localPlayer));
      if (!localCommit || String(localCommit.commitmentHex || "") !== String(stored.commitmentHex || "")) {
        throw new Error("Random reveal request does not include the locked local commitment");
      }
      const commitSetHash = await fairRandomCommitSetHash(request.contextKey, commits);
      const lockKey = `${request.contextKey}:${Number(localPlayer)}`;
      const existingLock = rngRevealCommitSetLocksRef.current.get(lockKey);
      if (existingLock && existingLock !== commitSetHash) {
        throw new Error("Random reveal request conflicts with the locked commitment set");
      }
      rngRevealCommitSetLocksRef.current.set(lockKey, commitSetHash);
      const requestPayload = cloneMultiplayerPayload(message);
      await rememberPendingActionIntent(request.actionIntent, {
        requestType: "rng_reveal_request",
        requestId: String(message.requestId || ""),
        requestPayload,
        requestPayloadHash: await sha256Hex(canonicalMultiplayerPayload(requestPayload)),
        responseTimeoutMs: PROTOCOL_RESPONSE_TIMEOUT_MS,
        requestedAtMs: Date.now(),
      });
      safeSend(conn, {
        type: "rng_reveal_response",
        protocolVersion: PROTOCOL_VERSION,
        requestId: message.requestId,
        reveal: await signRngRevealEntry({
          matchId: request.matchId,
          seq: request.seq,
          requirementId: request.requirementId,
          requestId: message.requestId,
          commitRequestId: message.commitRequestId,
          requester: request.requesterPlayer,
          player: localPlayer,
          nonceHex: stored.nonceHex,
          commitmentHex: stored.commitmentHex,
        }),
      });
    } catch (err) {
      safeSend(conn, {
        type: "rng_reveal_response",
        protocolVersion: PROTOCOL_VERSION,
        requestId: message.requestId,
        error: toErrorMessage(err),
      });
    }
  }

  async function collectFairRandomReveal(requirement, seq, options = {}) {
    const players = reindexPlayers(matchStartPayloadRef.current?.players || multiplayerRef.current.players || []);
    const localIndex = resolveLocalCryptoPlayerIndex();
    if (localIndex == null) {
      throw new Error("Local player seat is not assigned");
    }
    const matchId = currentAuditMatchId();
    const requirementId = fairRandomRequirementId(requirement);
    if (!requirementId) {
      throw new Error("Fair-random requirement is missing an id");
    }
    const command = options.command || null;
    if (!command || typeof command !== "object") {
      throw new Error("Fair-random request requires a command preview");
    }
    const actorIndex = normalizePlayerIndex(options.actorIndex ?? localIndex);
    if (actorIndex == null || actorIndex !== Number(localIndex)) {
      throw new Error("Fair-random request actor does not match the local player");
    }
    const prevStateHash = String(
      options.prevStateHash ?? auditStateHashRef.current ?? INITIAL_AUDIT_STATE_HASH
    );
    const publicCheckpointHash = String(
      options.publicCheckpointHash || await currentPublicAuditCheckpointHash()
    );
    const actionIntent = options.actionIntent || await signActionIntentForCommand({
      seq,
      actorIndex,
      command,
      prevStateHash,
      preActionPublicCheckpointHash: publicCheckpointHash,
    });
    const requestContext = {
      matchId,
      seq: Number(seq),
      requirementId,
      requesterIndex: Number(localIndex),
      actorIndex,
      prevStateHash,
      publicCheckpointHash,
      command: cloneMultiplayerPayload(command),
      requirement: cloneMultiplayerPayload(requirement),
      actionIntent: cloneMultiplayerPayload(actionIntent),
    };
    const contextKey = await fairRandomRequestContextKey({
      matchId,
      seq,
      requirement,
      requester: localIndex,
      actorIndex,
      prevStateHash,
      publicCheckpointHash,
      command,
    });
    const commits = [];
    const revealRequests = [];
    for (const player of players) {
      if (Number(player.index) === Number(localIndex)) {
        const contributionKey = `${contextKey}:${Number(player.index)}`;
        let contribution = signedRngCommitmentsRef.current.get(contributionKey);
        if (!contribution) {
          const nonceHex = randomAuditHex(32);
          contribution = {
            nonceHex,
            commitmentHex: await rngCommitmentForNonce(nonceHex),
          };
          signedRngCommitmentsRef.current.set(contributionKey, contribution);
        }
        const commitRequestId = makeZiffleRequestId("rng-local");
        commits.push(await signRngCommitmentEntry({
          matchId,
          seq: Number(seq),
          requirementId,
          requestId: commitRequestId,
          requester: localIndex,
          player: Number(player.index),
          commitmentHex: contribution.commitmentHex,
        }));
        revealRequests.push({ player, commitRequestId, local: contribution });
      } else {
        const routePeerId = routePeerIdForPlayer(player);
        const conn = await waitForZiffleRoute(routePeerId);
        const requestId = makeZiffleRequestId("rng-commit");
        const requestedAtMs = Date.now();
        const playerLabel = player.name || `Player ${Number(player.index) + 1}`;
        const waiter = waitForRngCommit(requestId, PROTOCOL_RESPONSE_TIMEOUT_MS, {
          peerIndex: Number(player.index),
          peerName: playerLabel,
          description:
            `${playerLabel} must sign a random commitment before the shared random value can be generated.`,
        });
        setStatus(`Waiting for random commitment from ${playerLabel}`);
        const requestPayload = {
          type: "rng_commit_request",
          protocolVersion: PROTOCOL_VERSION,
          requestId,
          ...requestContext,
        };
        safeSend(conn, requestPayload);
        const commitment = await waitForProtocolResponse(waiter, {
          basisSequence: Number(multiplayerRef.current.lastAppliedSequence || 0),
          targetPlayerIndex: Number(player.index),
          targetPeerId: routePeerId,
          requesterIndex: localIndex,
          requestType: requestPayload.type,
          requestId,
          requestPayload,
          responseTimeoutMs: PROTOCOL_RESPONSE_TIMEOUT_MS,
          requestedAtMs,
        });
        if (Number(commitment?.player) !== Number(player.index)) {
          throw new Error("Random commitment response came from the wrong player");
        }
        if (!String(commitment?.commitmentHex || "")) {
          throw new Error("Random commitment response is missing its commitment");
        }
        await verifyRngCommitmentEntry(commitment, {
          matchId,
          seq: Number(seq),
          requirementId,
          requester: localIndex,
          player: Number(player.index),
        });
        commits.push({
          player: Number(commitment.player),
          requester: Number(commitment.requester),
          requestId: String(commitment.requestId || requestId),
          commitmentHex: String(commitment.commitmentHex || ""),
          signature: String(commitment.signature || ""),
        });
        revealRequests.push({ player, commitRequestId: requestId });
      }
    }
    commits.sort((a, b) => Number(a.player) - Number(b.player));
    const commitSetHash = await fairRandomCommitSetHash(contextKey, commits);
    rngRevealCommitSetLocksRef.current.set(`${contextKey}:${Number(localIndex)}`, commitSetHash);
    const reveals = [];
    for (const request of revealRequests) {
      if (request.local) {
        reveals.push(await signRngRevealEntry({
          matchId,
          seq: Number(seq),
          requirementId,
          requestId: makeZiffleRequestId("rng-local-reveal"),
          commitRequestId: request.commitRequestId,
          requester: localIndex,
          player: Number(request.player.index),
          nonceHex: request.local.nonceHex,
          commitmentHex: request.local.commitmentHex,
        }));
      } else {
        const routePeerId = routePeerIdForPlayer(request.player);
        const conn = await waitForZiffleRoute(routePeerId);
        const requestId = makeZiffleRequestId("rng-reveal");
        const requestedAtMs = Date.now();
        const playerLabel = request.player.name || `Player ${Number(request.player.index) + 1}`;
        const waiter = waitForRngReveal(requestId, PROTOCOL_RESPONSE_TIMEOUT_MS, {
          peerIndex: Number(request.player.index),
          peerName: playerLabel,
          description:
            `${playerLabel} must reveal their committed random contribution before the game can use it.`,
        });
        setStatus(`Waiting for random reveal from ${playerLabel}`);
        const requestPayload = {
          type: "rng_reveal_request",
          protocolVersion: PROTOCOL_VERSION,
          requestId,
          commitRequestId: request.commitRequestId,
          ...requestContext,
          commits,
        };
        safeSend(conn, requestPayload);
        const reveal = await waitForProtocolResponse(waiter, {
          basisSequence: Number(multiplayerRef.current.lastAppliedSequence || 0),
          targetPlayerIndex: Number(request.player.index),
          targetPeerId: routePeerId,
          requesterIndex: localIndex,
          requestType: requestPayload.type,
          requestId,
          requestPayload,
          responseTimeoutMs: PROTOCOL_RESPONSE_TIMEOUT_MS,
          requestedAtMs,
        });
        if (Number(reveal?.player) !== Number(request.player.index)) {
          throw new Error("Random reveal response came from the wrong player");
        }
        await verifyRngRevealEntry(reveal, {
          matchId,
          seq: Number(seq),
          requirementId,
          requester: localIndex,
          player: Number(request.player.index),
        });
        reveals.push({
          player: Number(reveal.player),
          requester: Number(reveal.requester),
          requestId: String(reveal.requestId || requestId),
          commitRequestId: String(reveal.commitRequestId || request.commitRequestId),
          nonceHex: String(reveal.nonceHex || ""),
          commitmentHex: String(reveal.commitmentHex || ""),
          signature: String(reveal.signature || ""),
        });
      }
    }
    reveals.sort((a, b) => Number(a.player) - Number(b.player));
    for (const reveal of reveals) {
      const expected = commits.find((commit) => Number(commit.player) === Number(reveal.player));
      const actual = await rngCommitmentForNonce(reveal.nonceHex);
      if (!expected || actual !== expected.commitmentHex || actual !== reveal.commitmentHex) {
        throw new Error("Random reveal does not match its commitment");
      }
    }
    const combinedSeedHex = await fairRandomCombinedSeedHex({
      matchId,
      seq: Number(seq),
      requirementId,
      commits,
      reveals,
    });
    return {
      type: "commit_reveal_random",
      requirementId,
      count: Number(requirement.count || 1),
      commits,
      reveals,
      combinedSeedHex,
    };
  }

  const buildLocalRngRevealsForRequirements = useCallback(async (cryptoRequirements = [], seq = 0, options = {}) => {
    const fairRandomRequirements = (cryptoRequirements || []).filter(
      (requirement) => String(requirement?.type || requirement?.requirement_type || "") === "fair_random"
    );
    const reveals = [];
    for (const requirement of fairRandomRequirements) {
      reveals.push(await collectFairRandomReveal(requirement, seq, options));
    }
    return reveals;
  }, [
    currentAuditMatchId,
    currentPublicAuditCheckpointHash,
    makeZiffleRequestId,
    resolveLocalCryptoPlayerIndex,
    setStatus,
    waitForRngCommit,
    waitForRngReveal,
  ]);

  function handIdsForRevealKey(player) {
    if (!player || typeof player !== "object") return null;
    if (Array.isArray(player.hand)) {
      return player.hand.map((id) => Number(id)).filter(Number.isFinite);
    }
    if (Array.isArray(player.hand_cards)) {
      return player.hand_cards
        .map((card) => Number(card?.id ?? card?.object_id ?? card?.objectId))
        .filter(Number.isFinite);
    }
    return null;
  }

	  function localZiffleHandRevealKey(stateLike, localIndex, matchId) {
	    const players = Array.isArray(stateLike?.players) ? stateLike.players : [];
	    const player = players.find((entry) => Number(entry?.id) === Number(localIndex));
	    const ids = handIdsForRevealKey(player);
	    if (!ids) return "";
	    const objects = new Map(
	      (Array.isArray(stateLike?.objects) ? stateLike.objects : [])
	        .map((object) => [Number(object?.id), object])
	    );
	    const identities = ids.map((id) => {
	      const object = objects.get(Number(id));
	      const hidden = object?.hiddenCard || object?.hidden_card || null;
	      if (!hidden) return String(Number(id));
	      return [
	        Number(id),
	        hidden.owner == null ? "" : Number(hidden.owner),
	        hidden.slot == null ? "" : Number(hidden.slot),
	        String(hidden.commitment || ""),
	        hidden.publicSlot == null && hidden.public_slot == null
	          ? ""
	          : Number(hidden.publicSlot ?? hidden.public_slot),
	        String(hidden.publicCommitment || hidden.public_commitment || ""),
	      ].join(":");
	    });
	    identities.sort();
	    return [
	      String(matchId || currentAuditMatchId() || ""),
	      Number(localIndex),
	      identities.join(","),
	    ].join("|");
	  }

	  function localZiffleHandRevealObjectIdsKey(stateLike, localIndex, matchId) {
	    const players = Array.isArray(stateLike?.players) ? stateLike.players : [];
	    const player = players.find((entry) => Number(entry?.id) === Number(localIndex));
	    const ids = handIdsForRevealKey(player);
	    if (!ids) return "";
	    ids.sort((left, right) => left - right);
	    return [
	      String(matchId || currentAuditMatchId() || ""),
	      Number(localIndex),
	      ids.join(","),
	    ].join("|");
	  }

  function isInspectorOnlyViewedCards(viewedCards) {
    return Boolean(viewedCards?.inspector_only || viewedCards?.inspectorOnly);
  }

  function viewedCardsStateHint(...states) {
    for (const candidate of states) {
      if (candidate?.viewed_cards && !isInspectorOnlyViewedCards(candidate.viewed_cards)) {
        return candidate;
      }
    }
    return states.find(Boolean) || null;
  }

  function isHiddenViewedCardName(name) {
    return String(name || "").trim().toLowerCase() === "hidden card";
  }

  async function hydrateViewedCardsFromLiveObjects(viewedCards, currentGame) {
    if (
      !viewedCards
      || !Array.isArray(viewedCards.cards)
      || typeof currentGame?.objectDetails !== "function"
    ) {
      return viewedCards;
    }

    let changed = false;
    const cards = [];
    for (const card of viewedCards.cards) {
      const objectId = Number(card?.id);
      if (!Number.isSafeInteger(objectId) || objectId < 0) {
        cards.push(card);
        continue;
      }

      let details = null;
      try {
        details = await currentGame.objectDetails(wasmObjectIdArg(objectId));
      } catch {
        cards.push(card);
        continue;
      }

      const detailName = String(details?.name || "").trim();
      const cardName = String(card?.name || "").trim();
      let nextCard = card;
      if (
        detailName
        && !isHiddenViewedCardName(detailName)
        && (isHiddenViewedCardName(cardName) || cardName !== detailName)
      ) {
        nextCard = { ...nextCard, name: detailName };
        changed = true;
      }
      if (details?.oracle_text && details.oracle_text !== nextCard?.oracle_text) {
        nextCard = { ...nextCard, oracle_text: details.oracle_text };
        changed = true;
      }
      if (
        details?.stable_id != null
        && Number(details.stable_id) !== Number(nextCard?.stable_id)
      ) {
        nextCard = { ...nextCard, stable_id: details.stable_id };
        changed = true;
      }
      cards.push(nextCard);
    }

    return changed ? { ...viewedCards, cards } : viewedCards;
  }

  async function preserveViewedCardsFromHint(nextState, stateHint = null, currentGame = null) {
    if (!nextState) {
      return nextState;
    }
    if (nextState.viewed_cards) {
      const viewedCards = await hydrateViewedCardsFromLiveObjects(
        nextState.viewed_cards,
        currentGame
      );
      return viewedCards === nextState.viewed_cards
        ? nextState
        : { ...nextState, viewed_cards: viewedCards };
    }
    if (!stateHint?.viewed_cards || isInspectorOnlyViewedCards(stateHint.viewed_cards)) {
      return nextState;
    }
    const viewedCards = await hydrateViewedCardsFromLiveObjects(
      stateHint.viewed_cards,
      currentGame
    );
    return {
      ...nextState,
      viewed_cards: viewedCards,
    };
  }

  async function ziffleRevealTokenOptionsForLocalHandReveal({
    ceremony,
    positions,
    options = {},
    localIndex,
  } = {}) {
    const sanitized = { includeCeremonyInRevealRequest: true };
    const normalizedLocalIndex = Number(localIndex);
    if (
      !options?.command
      || !Number.isSafeInteger(normalizedLocalIndex)
      || Number(options.actorIndex) !== normalizedLocalIndex
    ) {
      return sanitized;
    }
    const requirements = Array.isArray(options.requirements) ? options.requirements : [];
    if (requirements.length === 0) return sanitized;
    const requester = normalizedLocalIndex;
    const owner = normalizedLocalIndex;
    if (
      ziffleRequirementsAuthorizeRevealPositions(
        requirements,
        requester,
        owner,
        positions,
        ceremony
      )
      || ziffleRequirementsAuthorizeRevealPositionCount(
        requirements,
        requester,
        owner,
        positions,
        ceremony
      )
    ) {
      return {
        ...options,
        includeCeremonyInRevealRequest: true,
      };
    }
    try {
      const authorizedByMetadata = await ziffleRequirementsAuthorizeRevealPositionsByMetadata(
        requirements,
        requester,
        owner,
        positions,
        ceremony
      );
      if (authorizedByMetadata) {
        return {
          ...options,
          includeCeremonyInRevealRequest: true,
        };
      }
    } catch {
      // Fall through to visible-state authorization.
    }
    return sanitized;
  }

  async function revealLocalZiffleHand(payload = matchStartPayloadRef.current, options = {}) {
    const previousReveal = localZiffleRevealInFlightRef.current;
    const revealPromise = (async () => {
      if (previousReveal) {
        await previousReveal;
      }
      return revealLocalZiffleHandInner(payload, options);
    })();
    localZiffleRevealInFlightRef.current = revealPromise;
    try {
      return await revealPromise;
    } finally {
      if (localZiffleRevealInFlightRef.current === revealPromise) {
        localZiffleRevealInFlightRef.current = null;
      }
    }
  }

	  async function revealLocalZiffleHandInner(payload = matchStartPayloadRef.current, options = {}) {
    if (!payload?.ziffleCeremonies?.length && liveZiffleCeremoniesRef.current.size === 0) return;
    const currentGame = gameRef.current;
    if (
      !currentGame
      || typeof currentGame.exportSyncCheckpoint !== "function"
      || typeof currentGame.exportHiddenCardOpening !== "function"
      || typeof currentGame.ziffleRevealCard !== "function"
      || typeof currentGame.ziffleRevealCards !== "function"
      || typeof currentGame.revealHiddenPosition !== "function"
    ) {
      return;
    }
    const localIndex = resolveLocalPlayerIndex(multiplayerRef.current);
    if (localIndex == null) return;
    const manifest = privateDeckManifestForOwner(localIndex, payload.auditMatchId);
	    if (!manifest) {
	      throw new Error("Missing private deck manifest for local ziffle hand reveal");
	    }

	    if (
	      options.skipIfHandUnchanged
	      && (options.requirements || []).length === 0
	      && options.command?.type === "priority_action"
	      && ["pass_priority", "test_priority_action"].includes(
	        String(options.command?.action_ref?.kind || options.command?.actionRef?.kind || "")
	      )
	    ) {
	      return;
	    }

	    if (options.skipIfHandUnchanged) {
	      const quickKey = localZiffleHandRevealObjectIdsKey(
	        options.stateHint || stateRef.current,
	        localIndex,
	        payload.auditMatchId,
	      );
	      if (quickKey && quickKey === ziffleHandRevealQuickKeyRef.current) {
	        return;
	      }
	    }

	    const checkpoint = await currentGame.exportSyncCheckpoint();
	    const localPlayer = (checkpoint.players || []).find(
	      (player) => Number(player.id) === Number(localIndex)
	    );
	    const checkpointKey = localZiffleHandRevealKey(checkpoint, localIndex, payload.auditMatchId);
	    const checkpointQuickKey = localZiffleHandRevealObjectIdsKey(
	      checkpoint,
	      localIndex,
	      payload.auditMatchId
	    );
	    if (
	      options.skipIfHandUnchanged
	      && checkpointKey
	      && checkpointKey === ziffleHandRevealKeyRef.current
	    ) {
	      if (checkpointQuickKey) ziffleHandRevealQuickKeyRef.current = checkpointQuickKey;
	      return;
	    }
    const handIds = new Set((localPlayer?.hand || []).map((id) => Number(id)));
	    if (handIds.size === 0) {
	      if (checkpointKey) ziffleHandRevealKeyRef.current = checkpointKey;
	      if (checkpointQuickKey) ziffleHandRevealQuickKeyRef.current = checkpointQuickKey;
	      return;
	    }
    const objects = new Map((checkpoint.objects || []).map((object) => [Number(object.id), object]));
    let changed = false;
    const ziffleGroups = new Map();
    for (const objectId of handIds) {
      const object = objects.get(objectId);
      const hidden = object?.hiddenCard;
      let exported = null;
      try {
        exported = await currentGame.exportHiddenCardOpening(wasmObjectIdArg(objectId));
      } catch {
        exported = null;
      }
      if (exported && Number(exported.owner) !== Number(localIndex)) {
        exported = null;
      }
      if (!hidden && !exported) continue;
      const exportedCommitment = String(exported?.commitment || "");
      const exportedZiffleDeckHash = ziffleDeckHashFromCommitment(exportedCommitment);
      const hiddenCommitment = String(hidden?.commitment || "");
      const hiddenZiffleDeckHash = ziffleDeckHashFromCommitment(hiddenCommitment);
	      const publicSlot =
	        hidden?.publicSlot
	        ?? hidden?.public_slot
	        ?? exported?.publicSlot
	        ?? exported?.public_slot
	        ?? null;
	      const publicCommitment = String(
	        hidden?.publicCommitment
	        || hidden?.public_commitment
	        || exported?.publicCommitment
	        || exported?.public_commitment
	        || ""
	      );
	      const publicZiffleDeckHash = ziffleDeckHashFromCommitment(publicCommitment);
	      const publicPositionFromCommitment = zifflePositionFromCommitment(publicCommitment);
		      const orderedPosition =
		        zifflePositionForObjectId(localIndex, objectId, { payload })
		        || zifflePositionForOriginalSlot(localIndex, exported?.slot, { payload });
		      const ziffleContext = orderedPosition?.ziffleContext || "";
		      const identityPosition = ziffleIdentityPositionFromSources(hidden, exported);
		      const knownPosition =
		        (publicZiffleDeckHash
		          ? (publicSlot != null ? Number(publicSlot) : publicPositionFromCommitment)
		          : hiddenZiffleDeckHash
		            ? Number(hidden?.slot)
		            : exportedZiffleDeckHash
		              ? Number(exported?.slot)
		              : orderedPosition?.position ?? null)
		        ?? identityPosition?.position;
		      const knownPositionCommitment =
		        (publicZiffleDeckHash
		          ? publicCommitment
		          : hiddenZiffleDeckHash
		            ? hiddenCommitment
		            : exportedZiffleDeckHash
		              ? exportedCommitment
		              : orderedPosition?.positionCommitment || "")
		        || identityPosition?.positionCommitment;
	      if (exported && !exportedZiffleDeckHash && !knownPositionCommitment) {
        let opening = localRevealedOpeningForExport(exported);
        if (opening && !knownPositionCommitment && !openingHasZifflePosition(opening)) {
          opening = cloneMultiplayerPayload(opening);
          delete opening.position;
          delete opening.positionCommitment;
          delete opening.ziffleReveal;
          delete opening.ziffleProof;
          delete opening.positionOpeningProof;
        }
        if (!opening) {
          const built = await buildDeckSlotOpeningForExport({
            manifest,
            preferredSlot: exported.slot,
            card: exported.card,
            exportedCommitment,
            label: "Local hidden card opening",
          });
          opening = built.opening;
        }
        rememberLocalRevealedOpening(opening, {
          objectId,
          position: opening.position ?? knownPosition,
          positionCommitment: opening.positionCommitment || knownPositionCommitment,
          matchId: payload.auditMatchId,
        });
        if (knownPosition != null) {
          rememberZiffleOpeningPosition(localIndex, opening.slot, knownPosition);
        }
        if (typeof currentGame.revealHiddenObject === "function") {
          try {
            await currentGame.revealHiddenObject({
              objectId,
              slot: Number(opening.slot),
              cardName: String(opening.card || exported.card || ""),
              commitment: String(opening.commitment || exportedCommitment || "") || undefined,
              recomputeDecision: false,
            });
            changed = true;
          } catch (err) {
            const message = String(err?.message || err || "");
            if (!message.includes("not present") && !message.includes("not a hidden")) {
              throw err;
            }
          }
        }
        continue;
      }
      if (!knownPositionCommitment) continue;
      const position = Number(knownPosition);
      const positionCommitment = knownPositionCommitment;
      const ceremony = ziffleCeremonyForOwner(localIndex, {
        payload,
        commitment: positionCommitment,
        context: ziffleContext,
      });
      if (!ceremony) continue;
      const groupKey = [
        Number(localIndex),
        String(ceremony.context || ""),
        String(ceremony.deckHash || ""),
      ].join(":");
      if (!ziffleGroups.has(groupKey)) {
        ziffleGroups.set(groupKey, {
          ceremony,
          entries: [],
        });
      }
      ziffleGroups.get(groupKey).entries.push({
        objectId,
        position,
        positionCommitment,
        exported,
      });
	    }

	    for (const { ceremony, entries } of ziffleGroups.values()) {
	      if (ziffleCeremonyHasObjectOrder(ceremony)) {
	        for (const entry of entries) {
	          const position = Number(entry.position);
	          const positionCommitment =
	            entry.positionCommitment || ziffleRuntimeCommitment(ceremony.deckHash, position);
	          const revealTokenOptions = await ziffleRevealTokenOptionsForLocalHandReveal({
	            ceremony,
	            positions: [position],
	            options,
	            localIndex,
	          });
	          const {
	            resolvedRevealSlot,
	            shuffleOriginalSlot,
	          } = await resolveCommittedSlotForZifflePosition({
	            owner: localIndex,
	            ceremony,
	            position,
	            card: entry.exported?.card || "",
	            objectId: entry.objectId,
	            manifest,
	            payload,
	            options: revealTokenOptions,
	          });
	          if (!resolvedRevealSlot) {
	            throw new Error(
	              `Ziffle hand reveal could not resolve committed slot `
	              + `(owner ${Number(localIndex) + 1}, position ${position}, `
	              + `shuffle slot ${shuffleOriginalSlot ?? "none"}, card ${String(entry.exported?.card || "")})`
	            );
	          }
	          const {
	            opening,
	            openingWithPosition,
	            originalSlot,
	          } = await buildOpeningFromResolvedCommittedSlot({
	            manifest,
	            resolvedRevealSlot,
	            fallbackObjectId: entry.objectId,
	            position,
	            positionCommitment,
	            ceremony,
	          });
	          const revealObjectId = Number(entry.objectId);
	          if (!Number.isSafeInteger(revealObjectId) || revealObjectId < 0) {
	            throw new Error("Local ziffle hand reveal is missing the current hand object id");
	          }
		          await currentGame.revealHiddenPosition({
		            owner: Number(localIndex),
		            objectId: revealObjectId,
		            position,
	            originalSlot,
	            cardName: opening.card,
	            positionCommitment,
		            commitment: opening.commitment,
		          }).catch((err) => {
		            if (!entry.exported) throw err;
		          });
		          const sanitizedOpeningWithPosition = await sanitizeObjectBoundOpening({
		            ...openingWithPosition,
		            objectId: revealObjectId,
		          });
		          rememberLocalRevealedOpening(
		            sanitizedOpeningWithPosition,
		            {
		              objectId: Number(revealObjectId),
		              position,
	              positionCommitment,
	              ziffleContext: sanitizedOpeningWithPosition.ziffleContext,
	              matchId: payload.auditMatchId,
	            }
	          );
	          rememberZiffleOpeningPosition(localIndex, originalSlot, position);
	          changed = true;
	        }
	        continue;
	      }
	      const positions = [...new Set(entries.map((entry) => Number(entry.position)))];
	      const revealTokenOptions = await ziffleRevealTokenOptionsForLocalHandReveal({
	        ceremony,
	        positions,
	        options,
	        localIndex,
	      });
	      const tokens = await collectZiffleRevealTokensBatch(ceremony, positions, revealTokenOptions);
      const reveals = await currentGame.ziffleRevealCards({
        deckCount: Number(ceremony.deckCount),
        context: String(ceremony.context || ""),
        keyContext: ziffleKeyContextForCeremony(ceremony),
        keys: cloneMultiplayerPayload(ceremony.keys || []),
        steps: cloneMultiplayerPayload(ceremony.steps || []),
        cardPositions: positions,
        tokens,
      });
      const revealByPosition = new Map(
        (Array.isArray(reveals) ? reveals : []).map((reveal) => [
          Number(reveal.cardPosition),
          Number(reveal.originalSlot),
        ])
      );
      for (const entry of entries) {
	        const position = Number(entry.position);
	        const shuffleOriginalSlot = revealByPosition.get(position);
	        if (!Number.isSafeInteger(shuffleOriginalSlot) || shuffleOriginalSlot < 0) {
	          throw new Error(`Missing ziffle reveal for position ${position}`);
	        }
	        const resolvedRevealSlot = await resolveCommittedZiffleRevealSlot({
	          owner: localIndex,
	          ceremony,
	          shuffleOriginalSlot,
	          shuffleOriginalSlotIsVerified: true,
	          position,
		          card: "",
		          objectId: entry.objectId,
		          manifest,
		          payload,
              options: revealTokenOptions,
		        });
		        if (!resolvedRevealSlot) {
		          let resolveDebug = null;
		          try {
		            const beforeOrder = normalizeShuffleOrder(ceremony.beforeOrder ?? ceremony.before_order);
		            const afterOrder = normalizeShuffleOrder(ceremony.afterOrder ?? ceremony.after_order);
		            const ids = [
		              entry.objectId,
		              beforeOrder[Number(shuffleOriginalSlot)],
		              afterOrder[Number(position)],
		            ].map((id) => Number(id)).filter((id, index, list) =>
		              Number.isSafeInteger(id) && id >= 0 && list.indexOf(id) === index
		            );
		            const checkpoint = await currentGame.exportSyncCheckpoint?.();
		            const objectsById = new Map((checkpoint?.objects || []).map((object) => [
		              Number(object.id),
		              object,
		            ]));
		            resolveDebug = {
		              beforeObjectId: beforeOrder[Number(shuffleOriginalSlot)] ?? null,
		              afterObjectId: afterOrder[Number(position)] ?? null,
		              entryObjectId: entry.objectId,
		              exported: entry.exported
		                ? {
		                  slot: entry.exported.slot,
		                  card: entry.exported.card,
		                  commitment: String(entry.exported.commitment || "").slice(0, 32),
		                  publicSlot: entry.exported.publicSlot ?? entry.exported.public_slot ?? null,
		                  publicCommitment: String(
		                    entry.exported.publicCommitment || entry.exported.public_commitment || ""
		                  ).slice(0, 48),
		                }
		                : null,
		              candidates: ids.map((id) => {
		                const object = objectsById.get(id) || {};
		                const hidden = object.hiddenCard || object.hidden_card || {};
		                return {
		                  id,
		                  name: object.name || null,
		                  zone: object.zone || null,
		                  hiddenSlot: hidden.slot ?? null,
		                  hiddenCommitment: String(hidden.commitment || "").slice(0, 48),
		                  publicSlot: hidden.publicSlot ?? hidden.public_slot ?? null,
		                  publicCommitment: String(
		                    hidden.publicCommitment || hidden.public_commitment || ""
		                  ).slice(0, 48),
		                };
		              }),
		            };
		          } catch {
		            resolveDebug = null;
		          }
		          throw new Error(
		            `Ziffle hand reveal could not resolve committed slot `
		            + `(owner ${Number(localIndex) + 1}, position ${position}, `
		            + `shuffle slot ${shuffleOriginalSlot}, card ${String(entry.exported?.card || "")}`
		            + `${resolveDebug ? `, debug ${JSON.stringify(resolveDebug)}` : ""})`
		          );
		        }
	        const originalSlot = Number(resolvedRevealSlot.slot);
	        const secret = (manifest.slotSecrets || []).find(
	          (candidate) => Number(candidate.slot) === originalSlot
	        );
	        if (!secret) {
	          throw new Error(`Missing private deck opening for ziffle slot ${originalSlot}`);
        }
        const opening = await buildDeckSlotOpening({
	          manifest,
	          slot: originalSlot,
	          card: resolvedRevealSlot?.card || secret.card,
	        });
        const positionCommitment =
          entry.positionCommitment || ziffleRuntimeCommitment(ceremony.deckHash, position);
        const revealObjectId = Number(entry.objectId);
        if (!Number.isSafeInteger(revealObjectId) || revealObjectId < 0) {
          throw new Error("Local ziffle hand reveal is missing the current hand object id");
        }
        await currentGame.revealHiddenPosition({
          owner: Number(localIndex),
          objectId: revealObjectId,
          position,
          originalSlot,
          cardName: opening.card,
          positionCommitment,
          commitment: opening.commitment,
        }).catch((err) => {
          if (!entry.exported) throw err;
        });
		        const openingWithPosition = {
		          ...opening,
		          ...(revealObjectId != null ? { objectId: Number(revealObjectId) } : {}),
		          ...(resolvedRevealSlot?.shuffleObjectId != null || resolvedRevealSlot?.objectId != null
		            ? { shuffleObjectId: Number(resolvedRevealSlot.shuffleObjectId ?? resolvedRevealSlot.objectId) }
		            : {}),
			          position,
			          positionCommitment,
			          ziffleContext: ziffleContextFromCeremony(ceremony),
		        };
        if (!ziffleCeremonyHasObjectOrder(ceremony)) {
          openingWithPosition.ziffleReveal = buildZiffleOpeningProof({
	            opening: {
	              ...opening,
	              position,
	              positionCommitment,
	              ziffleContext: ziffleContextFromCeremony(ceremony),
	            },
            ceremony,
            position,
            originalSlot,
            shuffleOriginalSlot,
            positionCommitment,
            tokens: ziffleTokensForPosition(tokens, position),
            compact: true,
          });
        }
	        const sanitizedOpeningWithPosition = await sanitizeObjectBoundOpening(openingWithPosition);
	        rememberLocalRevealedOpening(
	          sanitizedOpeningWithPosition,
	          {
	            objectId: revealObjectId,
	            position,
	            positionCommitment,
	            ziffleContext: sanitizedOpeningWithPosition.ziffleContext,
	            matchId: payload.auditMatchId,
	          }
        );
        rememberZiffleOpeningPosition(localIndex, originalSlot, position);
        changed = true;
      }
    }
    if (changed) {
      await currentGame.setPerspective(localIndex);
      if (options.updateState !== false) {
        const nextState = await preserveViewedCardsFromHint(
          await currentGame.uiState(),
          options.stateHint,
          currentGame,
        );
        stateRef.current = nextState;
        setState(nextState);
      }
    }
	    if (checkpointKey) {
	      ziffleHandRevealKeyRef.current = checkpointKey;
	    }
	    if (checkpointQuickKey) {
	      ziffleHandRevealQuickKeyRef.current = checkpointQuickKey;
	    }
	  }

  async function buildZiffleCeremoniesForPayload(payload, players) {
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.ziffleVerifyShuffle !== "function") {
      throw new Error("Ziffle mental-poker backend is not available");
    }
    const keys = zifflePublicKeysForPlayers(players);
    const ceremonies = [];
    const runtimeHiddenDeckManifests = [];
    const orderedPlayers = reindexPlayers(players);
    for (const ownerPlayer of orderedPlayers) {
      const manifest = publicDeckManifest(ownerPlayer.deckAuditManifest);
      const deckCount = Number(manifest?.deckCount || ownerPlayer.deckCount || 0);
      const context = String(payload.auditMatchId || payload.lobbyId || payload.hostPeerId || "");
      const keyContext = context;
      const steps = [];
      for (const shuffler of orderedPlayers) {
        const request = {
          deckCount,
          context,
          keyContext,
          keys,
          steps: cloneMultiplayerPayload(steps),
          shuffler: Number(shuffler.index || 0),
        };
        let step;
        if (shuffler.peerId === multiplayerRef.current.localPeerId) {
          step = await buildLocalZiffleShuffleStep(request);
        } else {
          const conn = clientConnectionsRef.current.get(shuffler.peerId);
          if (!conn || conn.open === false) {
            throw new Error(`Cannot request ziffle shuffle step from ${shuffler.name}`);
          }
          const requestId = makeZiffleRequestId("ziffle-shuffle");
          const shufflerLabel = shuffler.name || `Player ${Number(shuffler.index) + 1}`;
          const waiter = waitForZiffleShuffleStep(requestId, 60000, {
            peerIndex: Number(shuffler.index),
            peerName: shufflerLabel,
            description:
              `${shufflerLabel} must provide verifiable shuffle material before the match can start.`,
          });
          safeSend(conn, {
            type: "ziffle_shuffle_step_request",
            protocolVersion: PROTOCOL_VERSION,
            requestId,
            request,
          });
          step = await waiter;
        }
        steps.push({
          shuffler: Number(step.shuffler ?? shuffler.index ?? 0),
          deckHex: String(step.deckHex || ""),
          proofHex: String(step.proofHex || ""),
        });
      }
      const verified = await currentGame.ziffleVerifyShuffle({
        deckCount,
        context,
        keyContext,
        keys,
        steps,
      });
      const ceremony = {
        owner: Number(ownerPlayer.index || 0),
        deckCount,
        context,
        keyContext,
        keys,
        steps,
        deckHash: String(verified.deckHash || ""),
      };
      ceremonies.push(ceremony);
      runtimeHiddenDeckManifests.push(runtimeManifestForZiffleCeremony(manifest, ceremony));
    }
    payload.ziffleKeys = keys;
    payload.ziffleCeremonies = ceremonies;
    payload.runtimeHiddenDeckManifests = runtimeHiddenDeckManifests;
    payload.decks = orderedPlayers.map(() => []);
    return payload;
  }

  async function verifyZiffleCeremoniesForPayload(payload) {
    const ceremonies = Array.isArray(payload?.ziffleCeremonies) ? payload.ziffleCeremonies : [];
    const playerCount = Array.isArray(payload?.players) ? payload.players.length : 0;
    if (!isCurrentAuditPlayerCount(playerCount)) {
      throw new Error("Current protocol requires 2, 3, or 4 players");
    }
    if (ceremonies.length !== playerCount) {
      throw new Error("Current protocol requires one ziffle ceremony per player deck");
    }
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.ziffleVerifyShuffle !== "function") {
      throw new Error("Ziffle mental-poker backend is not available");
    }
    for (const ceremony of ceremonies) {
      const verified = await currentGame.ziffleVerifyShuffle({
        deckCount: Number(ceremony.deckCount),
        context: String(ceremony.context || ""),
        keyContext: ziffleKeyContextForCeremony(ceremony),
        keys: cloneMultiplayerPayload(ceremony.keys || []),
        steps: cloneMultiplayerPayload(ceremony.steps || []),
      });
      if (String(verified.deckHash || "") !== String(ceremony.deckHash || "")) {
        throw new Error(
          `Ziffle shuffle proof mismatch for player ${Number(ceremony.owner) + 1}`
        );
      }
    }
  }

	  const applyMatchStart = useCallback(
	    async (payload, options = {}) => {
	      const currentGame = gameRef.current;
	      if (!currentGame || typeof currentGame.startMatch !== "function") {
	        throw new Error("Game engine is not ready for multiplayer");
	      }
	      const securityMode = matchPayloadSecurityMode(payload, sessionSecurityMode(multiplayerRef.current));
	      const verifiedMode = isVerifiedMultiplayerSecurityMode(securityMode);

	      const currentSession = multiplayerRef.current;
	      const localAuditPublicKey = auditPublicKeyRef.current || "";
      const localEncryptionPublicKey = auditEncryptionPublicKeyRef.current || "";
      const localEntry = findLocalMatchPlayer(
        payload.players,
        currentSession,
        localAuditPublicKey,
        localEncryptionPublicKey
      );

	      if (!localEntry) {
	        throw new Error("Local player is missing from the match payload");
	      }
	      payload.securityMode = securityMode;
	      if (verifiedMode && !options.skipGenesisVerification) {
	        await verifySignedMatchGenesis(payload);
	      }
	      if (verifiedMode) {
	        await verifyZiffleCeremoniesForPayload(payload);
	      }
	      matchStartPayloadRef.current = cloneMultiplayerPayload(payload);
	      ensureDirectPeerConnections(payload.players || []);
	      if (
	        verifiedMode
	        && localAuditPublicKey
	        && String(localEntry.auditPublicKey || "") !== String(localAuditPublicKey)
	      ) {
	        throw new Error("Match genesis does not bind the local audit key");
	      }
	      if (
	        verifiedMode
	        && localEncryptionPublicKey
	        && String(localEntry.auditEncryptionPublicKey || "") !== String(localEncryptionPublicKey)
	      ) {
	        throw new Error("Match genesis does not bind the local private-view encryption key");
	      }

	      const startDecks = verifiedMode
	        ? payload.players.map(() => [])
	        : validationDecksForMatchPayload(payload);
	      const startSideboards = validationSideboardsForMatchPayload(payload);
	      const startCommanders = validationCommandersForMatchPayload(payload);

	      await currentGame.startMatch({
	        playerNames: payload.players.map((player) => player.name),
	        startingLife: payload.startingLife,
	        seed: payload.seed,
	        format: payload.format,
	        decks: startDecks,
	        sideboards: startSideboards,
	        commanders: startCommanders,
	        hiddenDeckManifests: verifiedMode ? payload.runtimeHiddenDeckManifests : undefined,
	        openingHandSize: payload.openingHandSize ?? DEFAULT_OPENING_HAND_SIZE,
	      });
	      await currentGame.setPerspective(localEntry.index);
	      liveZiffleCeremoniesRef.current.clear();
	      localZiffleCeremonyLookupRef.current.clear();
	      ziffleOpeningPositionsRef.current.clear();
	      ziffleRevealTokenCacheRef.current.clear();
	      verifiedAuditOpeningsRef.current.clear();
	      verifiedShuffleProofsRef.current.clear();
	      localRevealedOpeningsRef.current.clear();
	      privateViewDisclosuresRef.current.clear();
	      localZiffleRevealInFlightRef.current = null;
	      if (verifiedMode) {
	        clearStoredRevealedOpeningsForMatch(payload.auditMatchId || currentAuditMatchId());
	      }
	      matchStartPayloadRef.current = cloneMultiplayerPayload(payload);
	      ensureDirectPeerConnections(payload.players || []);
	      await currentGame.setPerspective(localEntry.index);

	      const nextState = await currentGame.uiState();
	      setState(nextState);
	      const matchClock = resetMatchClockForMatch(payload, nextState);
	      matchStartPayloadRef.current = cloneMultiplayerPayload(payload);
	      actionHistoryRef.current = [];
	      actionCryptoRequirementsRef.current.clear();
	      relayedActionIdsRef.current.clear();
	      applyingSequencedActionsRef.current.clear();
	      pendingSequencedActionsRef.current.clear();
	      drainingPendingSequencedActionsRef.current = false;
	      localZiffleRevealInFlightRef.current = null;
	      auditStateHashRef.current = INITIAL_AUDIT_STATE_HASH;
	      const initialPublicCheckpointHash = await publicCheckpointHash(
	        await currentGame.exportPublicAuditCheckpoint()
	      );
	      if (
	        verifiedMode
	        && payload.initialPublicCheckpointHash
	        && String(payload.initialPublicCheckpointHash) !== initialPublicCheckpointHash
	      ) {
	        throw new Error("Initial public checkpoint does not match signed match genesis");
	      }
	      initialPublicCheckpointHashRef.current = initialPublicCheckpointHash;
	      payload.initialPublicCheckpointHash = initialPublicCheckpointHash;
	      matchStartPayloadRef.current = cloneMultiplayerPayload(payload);
	      liveAuditTranscriptRef.current = {
	        version: 1,
	        kind: verifiedMode
	          ? "ironsmith-live-browser-audit-v1"
	          : "ironsmith-live-browser-trusted-log-v1",
	        securityMode,
	        match: cloneMultiplayerPayload(payload),
	        matchId: payload.auditMatchId || currentAuditMatchId(),
	        lobbyId: payload.lobbyId || payload.hostPeerId || "",
	        protocolVersion: PROTOCOL_VERSION,
	        signatureAlgorithm: verifiedMode ? "ecdsa-p256-sha256" : "none",
	        genesis: verifiedMode ? cloneMultiplayerPayload(payload.genesis) : null,
	        initialStateHash: INITIAL_AUDIT_STATE_HASH,
	        initialPublicCheckpointHash,
	        actions: [],
	      };
	      writeStoredPlayerIndex(payload.lobbyId || payload.hostPeerId, localEntry.index);

	      updateMultiplayer((prev) => ({
	        ...prev,
	        role: payload.hostPeerId === prev.localPeerId ? "host" : prev.role,
	        mode: "in_match",
	        lobbyId: payload.lobbyId || prev.lobbyId,
	        hostPeerId: payload.hostPeerId || prev.hostPeerId,
	        localPlayerIndex: localEntry.index,
	        desiredPlayers: payload.players.length,
	        startingLife: payload.startingLife,
	        format: normalizeMatchFormat(payload.format),
	        securityMode,
	        localDeckCount:
	          payload.deckAuditManifests?.[localEntry.index]?.deckCount
	          ?? payload.players?.[localEntry.index]?.deckCount
	          ?? payload.decks?.[localEntry.index]?.length
	          ?? prev.localDeckCount,
	        localCommanderCount:
	          payload.commanders?.[localEntry.index]?.length
	          ?? payload.players?.[localEntry.index]?.commanderCount
	          ?? prev.localCommanderCount,
	        players: payload.players,
	        rematch: null,
	        matchStarted: true,
	        lastAppliedSequence: 0,
	        submittingAction: false,
	        matchClock,
	        actionTimer: actionTimerSnapshotFromMatchClock(matchClock),
	      }));

	      if (verifiedMode && !options.deferLocalZiffleReveal) {
	        await revealLocalZiffleHand(payload);
	        await currentGame.setPerspective(localEntry.index);
	        setState(await currentGame.uiState());
	      }

	      setStatus(
	        verifiedMode
	          ? `Verified multiplayer match started as ${localEntry.name}`
	          : `Trusted multiplayer match started as ${localEntry.name}`
	      );
	    },
    [currentAuditMatchId, setState, setStatus, updateMultiplayer]
  );

  const requestResync = useCallback((reason = "Resyncing with host...") => {
    const session = multiplayerRef.current;
    if (session.role !== "client" || !session.matchStarted) return false;
    const conn = hostConnectionRef.current;
    if (!conn || conn.open === false) return false;

    updateMultiplayer((prev) => ({
      ...prev,
      submittingAction: false,
    }));
    awaitingStateResyncRef.current = true;
    safeSend(conn, {
      type: "resync_request",
      protocolVersion: PROTOCOL_VERSION,
      lastSequence: session.lastAppliedSequence,
    });
    setStatus(reason, true);
    return true;
  }, [setStatus, updateMultiplayer]);

  const reportSyncFailure = useCallback(
    (body, resyncReason = "", fallbackStatus = body) => {
      emitSyncFailureNotice("Sync failed", body);
      if (resyncReason && requestResync(resyncReason)) {
        return true;
      }
      setStatus(fallbackStatus, true);
      return false;
    },
    [requestResync, setStatus]
  );

  const applyStateResync = useCallback(
    async (message) => {
      awaitingStateResyncRef.current = true;
      if (multiplayerRef.current.submittingAction) {
        setStatus("Waiting for local action to settle before resync");
        const idle = await waitForSubmissionIdle(PROTOCOL_RESPONSE_TIMEOUT_MS);
        if (!idle) {
          throw new Error("Timed out waiting for local action to settle before resync");
        }
      }
      const matchPayload = message?.match;
      if (!matchPayload || typeof matchPayload !== "object") {
        throw new Error("Resync payload is missing match state");
      }
      if (!message?.checkpoint || typeof message.checkpoint !== "object") {
        throw new Error("Resync payload is missing WASM checkpoint");
      }

      const currentGame = gameRef.current;
      if (!currentGame || typeof currentGame.startMatch !== "function") {
        throw new Error("Game engine cannot replay a resync transcript");
      }
      if (
        typeof currentGame.exportSyncCheckpoint !== "function"
        || typeof currentGame.importSyncCheckpoint !== "function"
      ) {
        throw new Error("Game engine cannot sandbox a resync replay");
      }

      const currentSession = multiplayerRef.current;
      const resyncPlayers = Array.isArray(matchPayload.currentPlayers)
        ? matchPayload.currentPlayers
        : matchPayload.players || [];
      const localEntry = findLocalMatchPlayer(
        resyncPlayers,
        currentSession,
        auditPublicKeyRef.current || "",
        auditEncryptionPublicKeyRef.current || ""
      );

      if (!localEntry) {
        throw new Error("Local player is missing from the resync payload");
      }

	      const actionEntries = Array.isArray(message?.actions)
	        ? [...message.actions].sort((left, right) => Number(left?.seq || 0) - Number(right?.seq || 0))
	        : [];
	      const remoteFinalSequence = Number(actionEntries.at(-1)?.seq ?? message?.lastSequence ?? 0);
	      const localLastSequence = Number(currentSession.lastAppliedSequence || 0);
	      if (
	        Number.isSafeInteger(remoteFinalSequence)
	        && Number.isSafeInteger(localLastSequence)
	        && remoteFinalSequence < localLastSequence
	      ) {
	        awaitingStateResyncRef.current = false;
	        safeSend(hostConnectionRef.current, {
	          type: "resync_ack",
	          protocolVersion: PROTOCOL_VERSION,
	          lastSequence: localLastSequence,
	        });
	        return;
	      }
	      const continuity = assertResyncActionsExtendLocalTranscript({
	        actionEntries,
	        localActions: actionHistoryRef.current,
	        localLastSequence: currentSession.lastAppliedSequence,
	      });
      const messageLastSequence = Number(message?.lastSequence ?? continuity.finalSequence);
      if (messageLastSequence !== continuity.finalSequence) {
        throw new Error("Resync message last sequence does not match action transcript");
      }
      const securityMode = matchPayloadSecurityMode(
        matchPayload,
        sessionSecurityMode(currentSession, MULTIPLAYER_SECURITY_VERIFIED)
      );
      if (isTrustedMultiplayerSecurityMode(securityMode)) {
        const payloadHostPeerId = String(
          matchPayload.currentHostPeerId
          || matchPayload.hostPeerId
          || ""
        ).trim();
        const expectedHostPeerId = String(currentSession.hostPeerId || "").trim();
        if (payloadHostPeerId && expectedHostPeerId && payloadHostPeerId !== expectedHostPeerId) {
          throw new Error("Resync payload was not sent by the current match host");
        }
        const currentHostPlayer = (resyncPlayers || []).find(
          (player) => String(player?.peerId || "").trim() === (payloadHostPeerId || expectedHostPeerId)
            || String(player?.currentPeerId || "").trim() === (payloadHostPeerId || expectedHostPeerId)
        );
        const expectedHostSeat = normalizePlayerIndex(matchPayload.currentHostPlayerIndex)
          ?? normalizePlayerIndex(currentHostPlayer?.index)
          ?? 0;
        await currentGame.importSyncCheckpoint(
          message.checkpoint,
          localEntry.index ?? currentSession.localPlayerIndex ?? 0
        );
        if (typeof currentGame.setPerspective === "function") {
          await currentGame.setPerspective(localEntry.index);
        }
        const nextState = typeof currentGame.uiState === "function"
          ? await currentGame.uiState()
          : stateRef.current;
        stateRef.current = nextState;
        setState(nextState);
        restoreMatchClockRuntimeFromActionTranscript(actionEntries, nextState, matchPayload);
        const matchClock = alignMatchClockObservationFromHostSnapshot(
          matchPayload.currentMatchClock,
          nextState
        );
        matchClockObservationExemptSequenceRef.current = messageLastSequence + 1;
        const acceptedMatchPayload = {
          ...cloneMultiplayerPayload(matchPayload),
          securityMode: MULTIPLAYER_SECURITY_TRUSTED,
        };
        const acceptedSessionPlayersBase = Array.isArray(matchPayload.currentPlayers)
          ? cloneMultiplayerPayload(matchPayload.currentPlayers)
          : acceptedMatchPayload.players || [];
        const acceptedSessionPlayers = acceptedSessionPlayersBase.map((player) =>
          expectedHostSeat != null && Number(player?.index) === Number(expectedHostSeat)
            ? {
                ...player,
                currentPeerId: payloadHostPeerId || expectedHostPeerId || player.currentPeerId,
                connected: true,
              }
            : player
        );
        acceptedMatchPayload.currentHostPeerId =
          payloadHostPeerId || expectedHostPeerId || acceptedMatchPayload.currentHostPeerId || "";
        acceptedMatchPayload.currentHostPlayerIndex = expectedHostSeat;
        acceptedMatchPayload.currentPlayers = cloneMultiplayerPayload(acceptedSessionPlayers);
        matchStartPayloadRef.current = cloneMultiplayerPayload(acceptedMatchPayload);
        actionHistoryRef.current = actionEntries.map((entry) => ({
          ...cloneMultiplayerPayload(entry),
          securityMode: normalizeMultiplayerSecurityMode(
            entry.securityMode,
            MULTIPLAYER_SECURITY_TRUSTED
          ),
        }));
        actionCryptoRequirementsRef.current.clear();
        relayedActionIdsRef.current.clear();
        applyingSequencedActionsRef.current.clear();
        pendingSequencedActionsRef.current.clear();
        drainingPendingSequencedActionsRef.current = false;
        localZiffleRevealInFlightRef.current = null;
        auditStateHashRef.current = INITIAL_AUDIT_STATE_HASH;
        initialPublicCheckpointHashRef.current =
          acceptedMatchPayload.initialPublicCheckpointHash
          || initialPublicCheckpointHashRef.current
          || "";
        liveAuditTranscriptRef.current = {
          version: 1,
          kind: "ironsmith-live-browser-trusted-log-v1",
          securityMode: MULTIPLAYER_SECURITY_TRUSTED,
          match: cloneMultiplayerPayload(acceptedMatchPayload),
          matchId: acceptedMatchPayload.auditMatchId || currentAuditMatchId(),
          lobbyId: acceptedMatchPayload.lobbyId || acceptedMatchPayload.hostPeerId || "",
          protocolVersion: PROTOCOL_VERSION,
          signatureAlgorithm: "none",
          genesis: null,
          initialStateHash: INITIAL_AUDIT_STATE_HASH,
          initialPublicCheckpointHash: initialPublicCheckpointHashRef.current,
          actions: actionHistoryRef.current.map((entry) => cloneMultiplayerPayload(entry)),
        };
        clearAllPendingActionIntents();
        ignoredActionIntentKeysRef.current.clear();
        writeStoredPlayerIndex(
          acceptedMatchPayload.lobbyId || acceptedMatchPayload.hostPeerId,
          localEntry.index
        );
        updateMultiplayer((prev) => ({
          ...prev,
          role: acceptedMatchPayload.hostPeerId === prev.localPeerId ? "host" : prev.role,
          lobbyId: acceptedMatchPayload.lobbyId || prev.lobbyId,
          hostPeerId:
            acceptedMatchPayload.currentHostPeerId
            || acceptedMatchPayload.hostPeerId
            || prev.hostPeerId,
          desiredPlayers: acceptedMatchPayload.players?.length ?? prev.desiredPlayers,
          startingLife: Number(acceptedMatchPayload.startingLife || prev.startingLife || 20),
          format: normalizeMatchFormat(acceptedMatchPayload.format || prev.format),
          securityMode: MULTIPLAYER_SECURITY_TRUSTED,
          localPlayerIndex: localEntry.index ?? prev.localPlayerIndex,
          players: acceptedSessionPlayers.length > 0 ? acceptedSessionPlayers : prev.players,
          lastAppliedSequence: messageLastSequence,
          submittingAction: false,
          matchStarted: true,
          mode: "in_match",
          matchClock,
          actionTimer: actionTimerSnapshotFromMatchClock(matchClock),
        }));
        ensureDirectPeerConnections(acceptedSessionPlayers);
        setStatus(
          actionEntries.length > 0
            ? `Resynced with trusted host at action ${messageLastSequence}`
            : "Resynced with trusted host",
        );
        awaitingStateResyncRef.current = false;
        safeSend(hostConnectionRef.current, {
          type: "resync_ack",
          protocolVersion: PROTOCOL_VERSION,
          lastSequence: messageLastSequence,
        });
        return;
      }
      await verifySignedMatchGenesis(matchPayload);
      const verifyTranscriptShuffleProof = async (proof) => {
        if (!currentGame || typeof currentGame.ziffleVerifyShuffle !== "function") {
          throw new Error("Ziffle mental-poker backend is not available");
        }
        assertZiffleShuffleProofBoundToSignedMatch(proof, null, { matchPayload });
        const verified = await currentGame.ziffleVerifyShuffle({
          deckCount: Number(proof.deckCount),
          context: String(proof.context || ""),
          keyContext: String(proof.keyContext || proof.context || ""),
          keys: cloneMultiplayerPayload(proof.keys || []),
          steps: cloneMultiplayerPayload(proof.steps || []),
        });
        if (String(verified.deckHash || "") !== String(proof.deckHash || "")) {
          throw new Error(`Ziffle shuffle proof mismatch for player ${Number(proof.owner) + 1}`);
        }
      };
      const verifyTranscriptZiffleOpening = async ({ proof, ceremony }) => {
        if (!currentGame || typeof currentGame.ziffleRevealCard !== "function") {
          throw new Error("Ziffle mental-poker backend is not available");
        }
        const reveal = await currentGame.ziffleRevealCard({
          deckCount: Number(ceremony.deckCount),
          context: String(ceremony.context || ""),
          keyContext: String(ceremony.keyContext || ceremony.context || ""),
          keys: cloneMultiplayerPayload(ceremony.keys || []),
          steps: cloneMultiplayerPayload(ceremony.steps || []),
          cardPosition: Number(proof.position),
          tokens: ziffleTokensForPosition(proof.tokens || [], proof.position),
        });
        return { originalSlot: Number(reveal.originalSlot) };
      };
      const transcriptReport = await verifyLiveAuditTranscript({
        version: 1,
        kind: "ironsmith-live-browser-audit-v1",
        match: cloneMultiplayerPayload(matchPayload),
        matchId: matchPayload.auditMatchId || currentAuditMatchId(),
        lobbyId: matchPayload.lobbyId || matchPayload.hostPeerId || "",
        protocolVersion: PROTOCOL_VERSION,
        signatureAlgorithm: "ecdsa-p256-sha256",
        genesis: cloneMultiplayerPayload(matchPayload.genesis),
        initialStateHash: INITIAL_AUDIT_STATE_HASH,
        initialPublicCheckpointHash: matchPayload.initialPublicCheckpointHash || "",
        actions: actionEntries.map((entry) => cloneMultiplayerPayload(entry)),
      }, globalThis.crypto, {
        verifyShuffleProof: verifyTranscriptShuffleProof,
        verifyZiffleOpening: verifyTranscriptZiffleOpening,
        requireEngineReplay: false,
        requirePrivateViewDisclosures: false,
      });
      const payloadHostPeerId = String(
        matchPayload.currentHostPeerId
        || matchPayload.hostPeerId
        || ""
      ).trim();
      const expectedHostPeerId = String(currentSession.hostPeerId || "").trim();
      if (payloadHostPeerId && expectedHostPeerId && payloadHostPeerId !== expectedHostPeerId) {
        throw new Error("Resync payload was not sent by the current match host");
      }
      const currentHostPlayer = (resyncPlayers || []).find(
        (player) => String(player?.peerId || "").trim() === (payloadHostPeerId || expectedHostPeerId)
          || String(player?.currentPeerId || "").trim() === (payloadHostPeerId || expectedHostPeerId)
      );
      const expectedHostSeat = normalizePlayerIndex(matchPayload.currentHostPlayerIndex)
        ?? normalizePlayerIndex(currentHostPlayer?.index)
        ?? normalizePlayerIndex(matchPayload.genesis?.hostSeat)
        ?? 0;
      const resyncSigner = normalizePlayerIndex(message?.resyncEnvelope?.signer) ?? expectedHostSeat;
      if (resyncSigner !== expectedHostSeat) {
        throw new Error("Resync envelope was not signed by the match host");
      }
      const signerPlayer = (matchPayload.players || []).find(
        (player) => Number(player.index) === resyncSigner
      );
      const signerKey = await importAuditPublicKey(String(signerPlayer?.auditPublicKey || ""));
      await verifySignedResyncEnvelope({
        envelope: message.resyncEnvelope,
        publicKey: signerKey,
        checkpoint: message.checkpoint,
        actions: actionEntries,
      });
      if (Number(message.resyncEnvelope?.lastSequence ?? continuity.finalSequence) !== continuity.finalSequence) {
        throw new Error("Resync envelope last sequence does not match action transcript");
      }
      if (
        message.resyncEnvelope
        && String(message.resyncEnvelope.finalStateHash || "") !== String(transcriptReport.finalStateHash || "")
      ) {
        throw new Error("Resync transcript final hash does not match signed envelope");
      }
      const finalAction = actionEntries.at(-1);
      const finalPublicCheckpointHash = finalAction?.audit?.publicCheckpointHash || "";
      if (actionEntries.length > 0 && !finalPublicCheckpointHash) {
        throw new Error("Resync transcript is missing final public checkpoint hash");
      }
      const expectedPublicCheckpointHash = actionEntries.length > 0
        ? finalPublicCheckpointHash
        : (
          matchPayload.initialPublicCheckpointHash
          || initialPublicCheckpointHashRef.current
          || ""
        );

      const validationSnapshot = await createSequencedActionValidationSnapshot();
      const multiplayerSnapshot = cloneMultiplayerPayload(currentSession);
      const replayMatchPayload = cloneMultiplayerPayload(matchPayload);
      let nextState;
      try {
        await applyMatchStart(replayMatchPayload, {
          deferLocalZiffleReveal: true,
        });
        for (const action of actionEntries) {
          await applySequencedActionMessage(cloneMultiplayerPayload(action), {
            relay: false,
            allowWhileAwaitingResync: true,
            throwOnOrderMismatch: true,
            throwOnFailure: true,
            failureResyncReason: "",
            enforceMatchClockObservationBounds: false,
          });
        }
        nextState = typeof currentGame.uiState === "function"
          ? await currentGame.uiState()
          : stateRef.current;
        if (expectedPublicCheckpointHash) {
          await verifyCurrentPublicCheckpointHash(
            expectedPublicCheckpointHash,
            "Resync replay public state does not match the signed action transcript"
          );
        }
      } catch (err) {
        try {
          await restoreSequencedActionValidationSnapshot(validationSnapshot);
          updateMultiplayer(() => ({
            ...multiplayerSnapshot,
            submittingAction: false,
          }));
        } catch {
          // Keep the resync verification error as the actionable failure.
        }
        awaitingStateResyncRef.current = false;
        throw err;
      }
      setState(nextState);
      const matchClock = alignMatchClockObservationFromHostSnapshot(
        matchPayload.currentMatchClock,
        nextState
      );

      const lastSequence = continuity.finalSequence;
      matchClockObservationExemptSequenceRef.current = lastSequence + 1;
      const acceptedMatchPayload = matchStartPayloadRef.current
        ? cloneMultiplayerPayload(matchStartPayloadRef.current)
        : cloneMultiplayerPayload(replayMatchPayload);
      const acceptedSessionPlayersBase = Array.isArray(matchPayload.currentPlayers)
        ? cloneMultiplayerPayload(matchPayload.currentPlayers)
        : acceptedMatchPayload.players || [];
      const acceptedSessionPlayers = acceptedSessionPlayersBase.map((player) =>
        expectedHostSeat != null && Number(player?.index) === Number(expectedHostSeat)
          ? {
              ...player,
              currentPeerId: payloadHostPeerId || expectedHostPeerId || player.currentPeerId,
              connected: true,
            }
          : player
      );
      matchStartPayloadRef.current = cloneMultiplayerPayload(acceptedMatchPayload);
      matchStartPayloadRef.current.currentHostPeerId =
        payloadHostPeerId || expectedHostPeerId || acceptedMatchPayload.currentHostPeerId || "";
      matchStartPayloadRef.current.currentHostPlayerIndex = expectedHostSeat;
      matchStartPayloadRef.current.currentPlayers = cloneMultiplayerPayload(acceptedSessionPlayers);
      actionHistoryRef.current = actionEntries.map((entry) => cloneMultiplayerPayload(entry));
      relayedActionIdsRef.current.clear();
      applyingSequencedActionsRef.current.clear();
      pendingSequencedActionsRef.current.clear();
      drainingPendingSequencedActionsRef.current = false;
      localZiffleRevealInFlightRef.current = null;
      auditStateHashRef.current =
        actionEntries.at(-1)?.audit?.nextStateHash || INITIAL_AUDIT_STATE_HASH;
      initialPublicCheckpointHashRef.current =
        acceptedMatchPayload.initialPublicCheckpointHash
        || transcriptReport.initialPublicCheckpointHash
        || initialPublicCheckpointHashRef.current;
      liveAuditTranscriptRef.current = {
        version: 1,
        kind: "ironsmith-live-browser-audit-v1",
        match: cloneMultiplayerPayload(acceptedMatchPayload),
        matchId: acceptedMatchPayload.auditMatchId || currentAuditMatchId(),
        lobbyId: acceptedMatchPayload.lobbyId || acceptedMatchPayload.hostPeerId || "",
        protocolVersion: PROTOCOL_VERSION,
        signatureAlgorithm: "ecdsa-p256-sha256",
        genesis: cloneMultiplayerPayload(acceptedMatchPayload.genesis),
        initialStateHash: INITIAL_AUDIT_STATE_HASH,
        initialPublicCheckpointHash: initialPublicCheckpointHashRef.current,
        actions: actionHistoryRef.current.map((entry) => cloneMultiplayerPayload(entry)),
      };
      clearAllPendingActionIntents();
      ignoredActionIntentKeysRef.current.clear();
      writeStoredPlayerIndex(
        acceptedMatchPayload.lobbyId || acceptedMatchPayload.hostPeerId,
        localEntry.index
      );
      updateMultiplayer((prev) => ({
        ...prev,
        role: acceptedMatchPayload.hostPeerId === prev.localPeerId ? "host" : prev.role,
        lobbyId: acceptedMatchPayload.lobbyId || prev.lobbyId,
        hostPeerId: matchPayload.currentHostPeerId || acceptedMatchPayload.hostPeerId || prev.hostPeerId,
        desiredPlayers: acceptedMatchPayload.players?.length ?? prev.desiredPlayers,
        startingLife: Number(acceptedMatchPayload.startingLife || prev.startingLife || 20),
        format: normalizeMatchFormat(acceptedMatchPayload.format || prev.format),
        securityMode,
        localPlayerIndex: localEntry.index ?? prev.localPlayerIndex,
        players: acceptedSessionPlayers.length > 0 ? acceptedSessionPlayers : prev.players,
        lastAppliedSequence: lastSequence,
        submittingAction: false,
        matchStarted: true,
        mode: "in_match",
        matchClock,
        actionTimer: actionTimerSnapshotFromMatchClock(matchClock),
      }));
      ensureDirectPeerConnections(acceptedSessionPlayers);
      setStatus(
        actionEntries.length > 0
          ? `Resynced with host at action ${lastSequence}`
          : "Resynced with host",
      );
      awaitingStateResyncRef.current = false;
      await revealLocalZiffleHand(acceptedMatchPayload);

      safeSend(hostConnectionRef.current, {
        type: "resync_ack",
        protocolVersion: PROTOCOL_VERSION,
        lastSequence,
      });
    },
    [
      assertZiffleShuffleProofBoundToSignedMatch,
      alignMatchClockObservationFromHostSnapshot,
      currentAuditMatchId,
      setState,
      setStatus,
      updateMultiplayer,
      waitForSubmissionIdle,
      verifyCurrentPublicCheckpointHash,
      applyMatchStart,
      ensureDirectPeerConnections,
    ]
  );

  const broadcastLobbyState = useCallback(() => {
    const session = multiplayerRef.current;
    if (session.role !== "host") return;
    broadcastToClients({
      type: "lobby_state",
      protocolVersion: PROTOCOL_VERSION,
      lobbyId: session.lobbyId,
      hostPeerId: session.localPeerId,
      desiredPlayers: session.desiredPlayers,
      startingLife: session.startingLife,
      format: session.format,
      securityMode: sessionSecurityMode(session),
      players: toLobbyPlayers(session.players),
      matchStarted: session.matchStarted,
    });
  }, [broadcastToClients]);

  const startTrustedMatchFromPlayers = useCallback(async (rawPlayers, options = {}) => {
    const session = multiplayerRef.current;
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.startMatch !== "function") {
      setStatus("Game engine is not ready for multiplayer", true);
      return;
    }
    const players = reindexPlayers(rawPlayers || session.players);
    if (!isCurrentAuditPlayerCount(players.length)) {
      setStatus("Trusted multiplayer requires 2, 3, or 4 players", true);
      return;
    }
    const format = normalizeMatchFormat(session.format);
    const sideboards = players.map((player) => sanitizeCardList(player.sideboard));
    const commanders =
      format === MATCH_FORMAT_COMMANDER
        ? players.map((player) => sanitizeCardList(player.commanders))
        : undefined;
    const payloadPlayers = players.map((player) => ({
      ...toPublicPlayer({
        ...player,
        auditPublicKey: "",
        auditEncryptionPublicKey: "",
        playerGenesisSignature: null,
        deckAuditManifest: null,
        deckSlotOpenings: [],
        ziffleKey: null,
      }),
      ready: false,
    }));
    const payload = {
      type: "match_start",
      protocolVersion: PROTOCOL_VERSION,
      securityMode: MULTIPLAYER_SECURITY_TRUSTED,
      lobbyId: session.lobbyId,
      hostPeerId: session.localPeerId,
      players: payloadPlayers,
      format,
      openDecklists: true,
      decks: players.map((player) => sanitizeCardList(player.deck)),
      sideboards,
      commanders,
      startingLife: session.startingLife,
      openingHandSize: DEFAULT_OPENING_HAND_SIZE,
      timeoutMs: matchClockConfigRef.current.initialMs,
      matchClockPolicy: matchClockPolicyPayload(matchClockConfigRef.current),
      auditMatchId: session.lobbyId || session.localPeerId,
    };
    payload.seed = createMatchSeed(payload);

    updateMultiplayer((prev) => ({
      ...prev,
      mode: "starting",
      ...(options.rematch
        ? {
            rematch: {
              ...(prev.rematch || {}),
              phase: "starting",
              players,
            },
          }
        : { players }),
    }));

    try {
      if (typeof currentGame.validateMatchConfig === "function") {
        const validation = await currentGame.validateMatchConfig({
          playerNames: payload.players.map((player) => player.name),
          startingLife: payload.startingLife,
          seed: payload.seed,
          format: payload.format,
          decks: validationDecksForMatchPayload(payload),
          sideboards: validationSideboardsForMatchPayload(payload),
          commanders: validationCommandersForMatchPayload(payload),
          openingHandSize: payload.openingHandSize ?? DEFAULT_OPENING_HAND_SIZE,
        });
        if (validation?.valid === false) {
          const summary = summarizeMatchValidationIssues(validation.issues);
          emitSyncFailureNotice(options.rematch ? "Trusted rematch blocked" : "Trusted match start blocked", summary.notice);
          updateMultiplayer((prev) => ({
            ...prev,
            mode: options.rematch ? "in_match" : "lobby",
            ...(options.rematch
              ? {
                  rematch: {
                    ...(prev.rematch || {}),
                    phase: "sideboarding",
                    players,
                  },
                }
              : {}),
          }));
          setStatus(summary.status, true);
          return;
        }
      }

      await applyMatchStart(payload, {
        skipGenesisVerification: true,
      });
      matchStartPayloadRef.current = cloneMultiplayerPayload(payload);
      sendMatchStartToClients(payload);
    } catch (err) {
      emitSyncFailureNotice(
        options.rematch ? "Trusted rematch start failed" : "Trusted match start failed",
        err instanceof Error ? err.message : String(err)
      );
      updateMultiplayer((prev) => ({
        ...prev,
        mode: options.rematch ? "in_match" : "lobby",
        ...(options.rematch
          ? {
              rematch: {
                ...(prev.rematch || {}),
                phase: "sideboarding",
                players,
              },
            }
          : {}),
      }));
      setStatus(`Trusted match start failed: ${toErrorMessage(err)}`, true);
    }
  }, [
    applyMatchStart,
    sendMatchStartToClients,
    setStatus,
    updateMultiplayer,
  ]);

  const startHostedMatch = useCallback(async () => {
    const session = multiplayerRef.current;
    if (!canHostedMatchStart(session)) return;
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.startMatch !== "function") {
      setStatus("Game engine is not ready for multiplayer", true);
      return;
    }
    if (isTrustedMultiplayerSecurityMode(sessionSecurityMode(session))) {
      await startTrustedMatchFromPlayers(session.players);
      return;
    }

    await ensureAuditIdentity();
    let players = reindexPlayers(session.players).map((player) => ({
      ...player,
      auditPublicKey:
        player.peerId === session.localPeerId
          ? auditPublicKeyRef.current
          : player.auditPublicKey || "",
      auditEncryptionPublicKey:
        player.peerId === session.localPeerId
          ? auditEncryptionPublicKeyRef.current
          : player.auditEncryptionPublicKey || "",
    })
    );
    if (!isCurrentAuditPlayerCount(players.length)) {
      setStatus("Current P2P anticheat protocol requires 2, 3, or 4 players", true);
      return;
    }

    const sideboards = players.map((player) => sanitizeCardList(player.sideboard));
    const format = normalizeMatchFormat(session.format);
    const commanders =
      format === MATCH_FORMAT_COMMANDER
        ? players.map((player) => sanitizeCardList(player.commanders))
        : null;
    const missingZiffle = players.find((player) => !player.ziffleKey);
    if (missingZiffle) {
      setStatus(`${missingZiffle.name || "A player"} is missing a ziffle mental-poker key`, true);
      return;
    }
    const unsupportedZiffle = players.find(
      (player) => !isSupportedZiffleDeckCount(player.deckCount)
    );
    if (unsupportedZiffle) {
      setStatus(
        `${unsupportedZiffle.name || "A player"} has unsupported ziffle library size ${unsupportedZiffle.deckCount}`,
        true
      );
      return;
    }
    players = await waitForCryptoSeatBindingsFromSession(multiplayerRef);
    const unboundCryptoPlayer = players.find((player) => !playerCryptoSeatBindingReady(player));
    if (unboundCryptoPlayer) {
      setStatus(
        `${unboundCryptoPlayer.name || "A player"} is still binding deck commitments to their seat`,
        true
      );
      return;
    }

    const payload = {
      type: "match_start",
      protocolVersion: PROTOCOL_VERSION,
      securityMode: MULTIPLAYER_SECURITY_VERIFIED,
      lobbyId: session.lobbyId,
      hostPeerId: session.localPeerId,
      players: players.map(toPublicPlayer),
      format,
      openDecklists: true,
      decks: players.map(() => []),
      sideboards,
      commanders: commanders || undefined,
      ziffleKeys: players.map((player) => player.ziffleKey),
      startingLife: session.startingLife,
      openingHandSize: DEFAULT_OPENING_HAND_SIZE,
      timeoutMs: matchClockConfigRef.current.initialMs,
      matchClockPolicy: matchClockPolicyPayload(matchClockConfigRef.current),
    };
    payload.seed = createMatchSeed(payload);
    payload.auditMatchId = payload.lobbyId || payload.hostPeerId;
    players = await Promise.all(
      players.map(async (player) => {
        let manifest = publicDeckManifest(player.deckAuditManifest);
        const hasLocalDeck = player.peerId === session.localPeerId;
        if (hasLocalDeck) {
          manifest = await buildLocalDeckAuditManifest({
            matchId: payload.auditMatchId,
            owner: Number(player.index || 0),
            deck: player.deck,
            sideboard: player.sideboard,
            commanders: player.commanders,
          });
        } else if (!manifest || manifest.matchId !== payload.auditMatchId) {
          throw new Error(`Missing committed deck manifest for ${player.name || "remote player"}`);
        }
        return {
          ...player,
          deckAuditManifest: publicDeckManifest(manifest),
          deckSlotOpenings: sanitizeDeckSlotOpenings(
            player.deckSlotOpenings?.length
              ? player.deckSlotOpenings
              : deckSlotOpeningsForManifest(manifest)
          ),
        };
      })
    );
    payload.players = players.map(toPublicPlayer);
    payload.deckAuditManifests = players.map((player) =>
      publicDeckManifest(player.deckAuditManifest)
    );
    try {
      await buildZiffleCeremoniesForPayload(payload, players);
    } catch (err) {
      emitSyncFailureNotice("Ziffle setup failed", toErrorMessage(err));
      updateMultiplayer((prev) => ({ ...prev, mode: "lobby" }));
      setStatus(`Ziffle setup failed: ${toErrorMessage(err)}`, true);
      return;
    }
    players = await Promise.all(players.map(async (player) => {
      if (player.peerId !== session.localPeerId) return player;
      const publicPlayer = toPublicPlayer(player);
      return {
        ...player,
        playerGenesisSignature: await signPlayerGenesis({
          matchId: payload.auditMatchId,
          player: publicPlayer,
        }),
      };
    }));
    payload.players = players.map(toPublicPlayer);
    payload.genesis = await buildSignedMatchGenesis({
      keyPair: auditKeyPairRef.current,
      match: payload,
      hostSeat: 0,
    });

    updateMultiplayer((prev) => ({ ...prev, mode: "starting", players }));

    try {
      if (typeof currentGame.validateMatchConfig === "function") {
        const validation = await currentGame.validateMatchConfig({
          playerNames: payload.players.map((player) => player.name),
          startingLife: payload.startingLife,
          seed: payload.seed,
          format: payload.format,
          decks: validationDecksForMatchPayload(payload),
          sideboards: validationSideboardsForMatchPayload(payload),
          commanders: validationCommandersForMatchPayload(payload),
          openingHandSize: payload.openingHandSize ?? DEFAULT_OPENING_HAND_SIZE,
          hiddenDeckManifests: payload.runtimeHiddenDeckManifests,
        });
        if (validation?.valid === false) {
          const summary = summarizeMatchValidationIssues(validation.issues);
          emitSyncFailureNotice("Match start blocked", summary.notice);
          updateMultiplayer((prev) => ({ ...prev, mode: "lobby" }));
          setStatus(summary.status, true);
          return;
        }
      }

      await applyMatchStart(payload, {
        deferLocalZiffleReveal: true,
        skipGenesisVerification: true,
      });
      payload.genesis = await buildSignedMatchGenesis({
        keyPair: auditKeyPairRef.current,
        match: payload,
        hostSeat: 0,
      });
      await verifySignedMatchGenesis(payload);
      matchStartPayloadRef.current = cloneMultiplayerPayload(payload);
      if (liveAuditTranscriptRef.current) {
        liveAuditTranscriptRef.current = {
          ...liveAuditTranscriptRef.current,
          match: cloneMultiplayerPayload(payload),
          genesis: cloneMultiplayerPayload(payload.genesis),
          initialPublicCheckpointHash: payload.initialPublicCheckpointHash || "",
        };
      }
      sendMatchStartToClients(payload);
      await revealLocalZiffleHand(payload);
    } catch (err) {
      if (/ziffle/i.test(toErrorMessage(err, ""))) {
        emitZiffleDiagnosticNotice("Match start failed", err, {
          phase: "host_start_match",
          payload: {
            auditMatchId: String(payload.auditMatchId || ""),
            lobbyId: String(payload.lobbyId || ""),
            hostPeerId: String(payload.hostPeerId || ""),
            ziffleCeremonies: (payload.ziffleCeremonies || [])
              .map(compactZiffleCeremonyForDiagnostics)
              .filter(Boolean),
          },
        });
      } else {
        emitSyncFailureNotice(
          "Match start failed",
          err instanceof Error ? err.message : String(err)
        );
      }
      updateMultiplayer((prev) => ({ ...prev, mode: "lobby" }));
      setStatus(`Match start failed: ${err}`, true);
    }
  }, [
    applyMatchStart,
    broadcastToClients,
    buildLocalDeckAuditManifest,
    ensureAuditIdentity,
    emitZiffleDiagnosticNotice,
    sendMatchStartToClients,
    setStatus,
    signPlayerGenesis,
    startTrustedMatchFromPlayers,
    updateMultiplayer,
  ]);

  const broadcastRematchState = useCallback((rematch) => {
    broadcastToClients({
      type: "rematch_state",
      protocolVersion: PROTOCOL_VERSION,
      rematch: {
        phase: rematch?.phase || "sideboarding",
        players: (rematch?.players || []).map((player) => ({
          ...toPublicPlayer(player),
          ready: Boolean(player.ready),
        })),
      },
    });
  }, [broadcastToClients]);

  const startRematchFromState = useCallback(async (rematch) => {
    const session = multiplayerRef.current;
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.startMatch !== "function") {
      setStatus("Game engine is not ready for multiplayer", true);
      return;
    }
    if (isTrustedMultiplayerSecurityMode(sessionSecurityMode(session))) {
      await startTrustedMatchFromPlayers(rematch?.players || [], { rematch: true });
      return;
    }

    await ensureAuditIdentity();
    let players = reindexPlayers(rematch?.players || []).map((player) => ({
      ...player,
      auditPublicKey:
        player.peerId === session.localPeerId
          ? auditPublicKeyRef.current
          : player.auditPublicKey || "",
      auditEncryptionPublicKey:
        player.peerId === session.localPeerId
          ? auditEncryptionPublicKeyRef.current
          : player.auditEncryptionPublicKey || "",
    })
    );
    if (!isCurrentAuditPlayerCount(players.length)) {
      setStatus("Current P2P anticheat protocol requires 2, 3, or 4 players", true);
      return;
    }
    const format = normalizeMatchFormat(session.format);
    const sideboards = players.map((player) => sanitizeCardList(player.sideboard));
    const commanders =
      format === MATCH_FORMAT_COMMANDER
        ? players.map((player) => sanitizeCardList(player.commanders))
        : null;
    const missingZiffle = players.find((player) => !player.ziffleKey);
    if (missingZiffle) {
      setStatus(`${missingZiffle.name || "A player"} is missing a ziffle mental-poker key`, true);
      return;
    }
    const unsupportedZiffle = players.find(
      (player) => !isSupportedZiffleDeckCount(player.deckCount)
    );
    if (unsupportedZiffle) {
      setStatus(
        `${unsupportedZiffle.name || "A player"} has unsupported ziffle library size ${unsupportedZiffle.deckCount}`,
        true
      );
      return;
    }
    players = await waitForCryptoSeatBindingsFromSession({ current: { players } });
    const unboundCryptoPlayer = players.find((player) => !playerCryptoSeatBindingReady(player));
    if (unboundCryptoPlayer) {
      setStatus(
        `${unboundCryptoPlayer.name || "A player"} is still binding deck commitments to their seat`,
        true
      );
      return;
    }
    const payload = {
      type: "match_start",
      protocolVersion: PROTOCOL_VERSION,
      securityMode: MULTIPLAYER_SECURITY_VERIFIED,
      lobbyId: session.lobbyId,
      hostPeerId: session.localPeerId,
      players: players.map((player) => ({
        ...toPublicPlayer(player),
        ready: false,
      })),
      format,
      openDecklists: true,
      decks: players.map(() => []),
      sideboards,
      commanders: commanders || undefined,
      ziffleKeys: players.map((player) => player.ziffleKey),
      startingLife: session.startingLife,
      openingHandSize: DEFAULT_OPENING_HAND_SIZE,
      timeoutMs: matchClockConfigRef.current.initialMs,
      matchClockPolicy: matchClockPolicyPayload(matchClockConfigRef.current),
    };
    payload.seed = createMatchSeed(payload);
    payload.auditMatchId = payload.lobbyId || payload.hostPeerId;
    players = await Promise.all(
      players.map(async (player) => {
        let manifest = publicDeckManifest(player.deckAuditManifest);
        const hasLocalDeck = player.peerId === session.localPeerId;
        if (hasLocalDeck) {
          manifest = await buildLocalDeckAuditManifest({
            matchId: payload.auditMatchId,
            owner: Number(player.index || 0),
            deck: player.deck,
            sideboard: player.sideboard,
            commanders: player.commanders,
          });
        } else if (!manifest || manifest.matchId !== payload.auditMatchId) {
          throw new Error(`Missing committed rematch deck manifest for ${player.name || "remote player"}`);
        }
        return {
          ...player,
          deckAuditManifest: publicDeckManifest(manifest),
          deckSlotOpenings: sanitizeDeckSlotOpenings(
            player.deckSlotOpenings?.length
              ? player.deckSlotOpenings
              : deckSlotOpeningsForManifest(manifest)
          ),
        };
      })
    );
    payload.players = players.map((player) => ({
      ...toPublicPlayer(player),
      ready: false,
    }));
    payload.deckAuditManifests = players.map((player) =>
      publicDeckManifest(player.deckAuditManifest)
    );
    try {
      await buildZiffleCeremoniesForPayload(payload, players);
    } catch (err) {
      emitSyncFailureNotice("Rematch ziffle setup failed", toErrorMessage(err));
      updateMultiplayer((prev) => ({
        ...prev,
        mode: "in_match",
        rematch: {
          ...(prev.rematch || {}),
          phase: "sideboarding",
          players,
        },
      }));
      setStatus(`Rematch ziffle setup failed: ${toErrorMessage(err)}`, true);
      return;
    }
    players = await Promise.all(players.map(async (player) => {
      if (player.peerId !== session.localPeerId) return player;
      const publicPlayer = toPublicPlayer(player);
      return {
        ...player,
        playerGenesisSignature: await signPlayerGenesis({
          matchId: payload.auditMatchId,
          player: publicPlayer,
        }),
      };
    }));
    payload.players = players.map((player) => ({
      ...toPublicPlayer(player),
      ready: false,
    }));
    payload.genesis = await buildSignedMatchGenesis({
      keyPair: auditKeyPairRef.current,
      match: payload,
      hostSeat: 0,
    });

    updateMultiplayer((prev) => ({
      ...prev,
      mode: "starting",
      rematch: {
        ...(prev.rematch || {}),
        phase: "starting",
        players,
      },
    }));

    try {
      if (typeof currentGame.validateMatchConfig === "function") {
        const validation = await currentGame.validateMatchConfig({
          playerNames: payload.players.map((player) => player.name),
          startingLife: payload.startingLife,
          seed: payload.seed,
          format: payload.format,
          decks: validationDecksForMatchPayload(payload),
          sideboards: validationSideboardsForMatchPayload(payload),
          commanders: validationCommandersForMatchPayload(payload),
          openingHandSize: payload.openingHandSize ?? DEFAULT_OPENING_HAND_SIZE,
          hiddenDeckManifests: payload.runtimeHiddenDeckManifests,
        });
        if (validation?.valid === false) {
          const summary = summarizeMatchValidationIssues(validation.issues);
          emitSyncFailureNotice("Rematch blocked", summary.notice);
          updateMultiplayer((prev) => ({
            ...prev,
            mode: "in_match",
            rematch: {
              ...(prev.rematch || {}),
              phase: "sideboarding",
              players,
            },
          }));
          setStatus(summary.status, true);
          return;
        }
      }

      await applyMatchStart(payload, {
        deferLocalZiffleReveal: true,
        skipGenesisVerification: true,
      });
      payload.genesis = await buildSignedMatchGenesis({
        keyPair: auditKeyPairRef.current,
        match: payload,
        hostSeat: 0,
      });
      await verifySignedMatchGenesis(payload);
      matchStartPayloadRef.current = cloneMultiplayerPayload(payload);
      if (liveAuditTranscriptRef.current) {
        liveAuditTranscriptRef.current = {
          ...liveAuditTranscriptRef.current,
          match: cloneMultiplayerPayload(payload),
          genesis: cloneMultiplayerPayload(payload.genesis),
          initialPublicCheckpointHash: payload.initialPublicCheckpointHash || "",
        };
      }
      sendMatchStartToClients(payload);
      await revealLocalZiffleHand(payload);
    } catch (err) {
      if (/ziffle/i.test(toErrorMessage(err, ""))) {
        emitZiffleDiagnosticNotice("Rematch start failed", err, {
          phase: "host_start_rematch",
          payload: {
            auditMatchId: String(payload.auditMatchId || ""),
            lobbyId: String(payload.lobbyId || ""),
            hostPeerId: String(payload.hostPeerId || ""),
            ziffleCeremonies: (payload.ziffleCeremonies || [])
              .map(compactZiffleCeremonyForDiagnostics)
              .filter(Boolean),
          },
        });
      } else {
        emitSyncFailureNotice(
          "Rematch start failed",
          err instanceof Error ? err.message : String(err)
        );
      }
      updateMultiplayer((prev) => ({
        ...prev,
        mode: "in_match",
        rematch: {
          ...(prev.rematch || {}),
          phase: "sideboarding",
          players,
        },
      }));
      setStatus(`Rematch start failed: ${err}`, true);
    }
  }, [
    applyMatchStart,
    broadcastToClients,
    buildLocalDeckAuditManifest,
    ensureAuditIdentity,
    emitZiffleDiagnosticNotice,
    sendMatchStartToClients,
    setStatus,
    signPlayerGenesis,
    startTrustedMatchFromPlayers,
    updateMultiplayer,
  ]);

  const startRematchSideboarding = useCallback((source = "local") => {
    const session = multiplayerRef.current;
    const payload = matchStartPayloadRef.current;
    if (!session.matchStarted || !payload) {
      setStatus("No completed multiplayer match is available to replay", true);
      return;
    }

    if (session.role !== "host") {
      const conn = hostConnectionRef.current;
      if (!conn || conn.open === false) {
        setStatus("Host connection is not available", true);
        return;
      }
      safeSend(conn, {
        type: "rematch_request",
        protocolVersion: PROTOCOL_VERSION,
      });
      setStatus("Waiting for host to open sideboarding");
      return;
    }

    const rematch = buildRematchStateFromPayload(payload, session.localPeerId);
    updateMultiplayer((prev) => ({
      ...prev,
      mode: "in_match",
      rematch,
    }));
    broadcastToClients({
      type: "rematch_start",
      protocolVersion: PROTOCOL_VERSION,
      match: payload,
    });
    setStatus(source === "remote" ? "Sideboarding opened for rematch" : "Sideboard for the next game");
  }, [broadcastToClients, setStatus, updateMultiplayer]);

  const updateRematchDecks = useCallback(({ deck, sideboard }) => {
    updateMultiplayer((prev) => {
      if (!prev.rematch || prev.rematch.phase !== "sideboarding") return prev;
      return {
        ...prev,
        rematch: {
          ...prev.rematch,
          localDeck: sanitizeCardList(deck ?? prev.rematch.localDeck),
          localSideboard: sanitizeCardList(sideboard ?? prev.rematch.localSideboard),
          localReady: false,
        },
      };
    });
  }, [updateMultiplayer]);

  const readyForRematch = useCallback(async () => {
    const session = multiplayerRef.current;
    const rematch = session.rematch;
    if (!rematch || rematch.phase !== "sideboarding") {
      setStatus("Sideboarding is not active", true);
      return;
    }
    const localIndex = resolveLocalPlayerIndex(session);
    if (localIndex == null) {
      setStatus("Local player seat is not assigned", true);
      return;
    }

    const localDeck = sanitizeCardList(rematch.localDeck);
    const localSideboard = sanitizeCardList(rematch.localSideboard);
    const localPlayer = reindexPlayers(rematch.players || []).find(
      (player) => Number(player.index) === Number(localIndex)
    );
    const localCommanders = sanitizeCardList(localPlayer?.commanders);
    if (isTrustedMultiplayerSecurityMode(sessionSecurityMode(session))) {
      if (session.role !== "host") {
        const conn = hostConnectionRef.current;
        if (!conn || conn.open === false) {
          setStatus("Host connection is not available", true);
          return;
        }
        updateMultiplayer((prev) => ({
          ...prev,
          rematch: prev.rematch
            ? { ...prev.rematch, localReady: true }
            : prev.rematch,
        }));
        safeSend(conn, {
          type: "rematch_ready",
          protocolVersion: PROTOCOL_VERSION,
          securityMode: MULTIPLAYER_SECURITY_TRUSTED,
          deck: localDeck,
          sideboard: localSideboard,
          commanders: localCommanders,
          deckAuditManifest: null,
          deckSlotOpenings: [],
          ziffleKey: null,
          playerGenesisSignature: null,
          deckCount: localDeck.length,
          sideboardCount: localSideboard.length,
          commanderCount: localCommanders.length,
        });
        setStatus("Ready for trusted rematch");
        return;
      }

      const nextSession = updateMultiplayer((prev) => {
        if (!prev.rematch) return prev;
        const players = reindexPlayers(prev.rematch.players || []).map((player) => (
          Number(player.index) === localIndex
            ? {
                ...player,
                deck: localDeck,
                sideboard: localSideboard,
                commanders: localCommanders,
                deckAuditManifest: null,
                deckSlotOpenings: [],
                ziffleKey: null,
                playerGenesisSignature: null,
                deckCount: localDeck.length,
                sideboardCount: localSideboard.length,
                commanderCount: localCommanders.length,
                ready: true,
              }
            : player
        ));
        return {
          ...prev,
          rematch: {
            ...prev.rematch,
            players,
            localDeck,
            localSideboard,
            localReady: true,
          },
        };
      });

      broadcastRematchState(nextSession.rematch);
      if (rematchPlayersReady(nextSession.rematch?.players)) {
        await startRematchFromState(nextSession.rematch);
      } else {
        setStatus("Ready for trusted rematch; waiting for other players");
      }
      return;
    }
    const deckAuditManifest = await buildLocalDeckAuditManifest({
      matchId: session.lobbyId || session.hostPeerId || "pending",
      owner: localIndex,
      deck: localDeck,
      sideboard: localSideboard,
      commanders: localCommanders,
    });
    const deckSlotOpenings = deckSlotOpeningsForManifest(deckAuditManifest);
    const {
      publicKey: auditPublicKey,
      encryptionPublicKey: auditEncryptionPublicKey,
    } = await ensureAuditIdentity();
    const localGenesisPlayer = {
      peerId: localPlayer?.peerId || session.localPeerId,
      name: localPlayer?.name || session.localName,
      index: localIndex,
      auditPublicKey,
      auditEncryptionPublicKey,
      deckAuditManifest: publicDeckManifest(deckAuditManifest),
      ziffleKey: localPlayer?.ziffleKey || null,
      ...openDecklistPlayerFields({
        deck: localDeck,
        sideboard: localSideboard,
        commanders: localCommanders,
        deckSlotOpenings,
      }),
      deckCount: localDeck.length,
      sideboardCount: localSideboard.length,
      commanderCount: localCommanders.length,
    };
    const playerGenesisSignature = await signPlayerGenesis({
      matchId: session.lobbyId || session.hostPeerId || "pending",
      player: localGenesisPlayer,
    });

    if (session.role !== "host") {
      const conn = hostConnectionRef.current;
      if (!conn || conn.open === false) {
        setStatus("Host connection is not available", true);
        return;
      }
      updateMultiplayer((prev) => ({
        ...prev,
        rematch: prev.rematch
          ? { ...prev.rematch, localReady: true }
          : prev.rematch,
      }));
      safeSend(conn, {
        type: "rematch_ready",
        protocolVersion: PROTOCOL_VERSION,
        auditPublicKey,
        auditEncryptionPublicKey,
        playerGenesisSignature,
        deckAuditManifest: publicDeckManifest(deckAuditManifest),
        deck: localDeck,
        sideboard: localSideboard,
        commanders: localCommanders,
        deckSlotOpenings,
        deckCount: localDeck.length,
        sideboardCount: localSideboard.length,
        commanderCount: localCommanders.length,
      });
      setStatus("Ready for rematch");
      return;
    }

    const nextSession = updateMultiplayer((prev) => {
      if (!prev.rematch) return prev;
      const players = reindexPlayers(prev.rematch.players || []).map((player) => (
        Number(player.index) === localIndex
	          ? {
	              ...player,
              auditPublicKey,
              auditEncryptionPublicKey,
              playerGenesisSignature,
	              deck: localDeck,
	              sideboard: localSideboard,
              commanders: localCommanders,
	              deckAuditManifest: publicDeckManifest(deckAuditManifest),
              deckSlotOpenings,
	              deckCount: localDeck.length,
	              sideboardCount: localSideboard.length,
              commanderCount: localCommanders.length,
	              ready: true,
	            }
          : player
      ));
      return {
        ...prev,
        rematch: {
          ...prev.rematch,
          players,
          localDeck,
          localSideboard,
          localReady: true,
        },
      };
    });

    broadcastRematchState(nextSession.rematch);
    if (rematchPlayersReady(nextSession.rematch?.players)) {
      await startRematchFromState(nextSession.rematch);
    } else {
      setStatus("Ready for rematch; waiting for other players");
    }
	  }, [
	    broadcastRematchState,
	    buildLocalDeckAuditManifest,
	    ensureAuditIdentity,
	    setStatus,
	    signPlayerGenesis,
	    startRematchFromState,
	    updateMultiplayer,
	  ]);

	  async function publishLocalDeckUpdateForAssignedSeat(sessionSnapshot = multiplayerRef.current) {
	    const session = sessionSnapshot || multiplayerRef.current;
	    if (session.role !== "client" || session.matchStarted || session.mode === "starting") return;
	    const localPlayerIndex = resolveLocalPlayerIndex(session);
	    if (localPlayerIndex == null) return;
	    const deckSubmission = parseDeckSubmission(
	      session.format,
	      session.localDeckText,
	      session.localCommanderText
	    );
	    if (isTrustedMultiplayerSecurityMode(sessionSecurityMode(session))) {
	      updateMultiplayer((prev) => ({
	        ...prev,
	        localDeckCount: deckSubmission.deckCount,
	        localCommanderCount: deckSubmission.commanderCount,
	        players: reindexPlayers(
	          prev.players.map((player) =>
	            player.peerId === prev.localPeerId
	              ? withDeckState(
	                  {
	                    ...player,
	                    deckAuditManifest: null,
	                    deckSlotOpenings: [],
	                    ziffleKey: null,
	                    playerGenesisSignature: null,
	                  },
	                  prev.format,
	                  deckSubmission.deck,
	                  deckSubmission.commanders,
	                  deckSubmission.sideboard
	                )
	              : player
	          )
	        ),
	      }));
	      const conn = hostConnectionRef.current;
	      if (!conn || conn.open === false) return;
	      safeSend(conn, {
	        type: "deck_update",
	        protocolVersion: PROTOCOL_VERSION,
	        securityMode: MULTIPLAYER_SECURITY_TRUSTED,
	        deckAuditManifest: null,
	        deck: deckSubmission.deck,
	        sideboard: deckSubmission.sideboard,
	        deckSlotOpenings: [],
	        ziffleKey: null,
	        deckCount: deckSubmission.deckCount,
	        sideboardCount: deckSubmission.sideboard.length,
	        commanders: deckSubmission.commanders,
	        commanderCount: deckSubmission.commanderCount,
	        ready: deckSubmission.ready,
	      });
	      return;
	    }
	    const matchId = session.lobbyId || session.hostPeerId || "pending";
    const expectedDecklistHash = await decklistHashForCards({
      matchId,
      owner: localPlayerIndex,
      deck: deckSubmission.deck,
      sideboard: deckSubmission.sideboard,
      commanders: deckSubmission.commanders,
    });
    const localEntry = (session.players || []).find(
      (player) => player.peerId === session.localPeerId
    );
    const currentManifest = publicDeckManifest(localEntry?.deckAuditManifest);
    const currentPrivateManifest = privateDeckManifestForOwner(localPlayerIndex, matchId);
    const currentZifflePlayer = normalizePlayerIndex(localEntry?.ziffleKey?.player);
    const currentSigner = normalizePlayerIndex(localEntry?.playerGenesisSignature?.signer);
    if (
      currentManifest?.owner === localPlayerIndex
      && currentManifest?.matchId === matchId
      && currentPrivateManifest?.decklistHash === expectedDecklistHash
      && currentManifest?.decklistCommitment === currentPrivateManifest?.decklistCommitment
      && currentManifest?.commitmentRoot === currentPrivateManifest?.commitmentRoot
      && currentZifflePlayer === localPlayerIndex
      && currentSigner === localPlayerIndex
    ) {
      return;
    }

    const {
      publicKey: auditPublicKey,
      encryptionPublicKey: auditEncryptionPublicKey,
    } = await ensureAuditIdentity();
    const deckAuditManifest = await buildLocalDeckAuditManifest({
      matchId,
      owner: localPlayerIndex,
      deck: deckSubmission.deck,
      sideboard: deckSubmission.sideboard,
      commanders: deckSubmission.commanders,
    });
    const deckSlotOpenings = deckSlotOpeningsForManifest(deckAuditManifest);
    const ziffleKeyPair = await ensureZiffleIdentity({
      context: matchId,
      deckCount: deckSubmission.deckCount || 60,
    });
    const localZiffleKey = publicZiffleKey(ziffleKeyPair, localPlayerIndex);
    const playerGenesisSignature = await signPlayerGenesis({
      matchId,
      player: {
        peerId: session.localPeerId,
        name: session.localName,
	        index: localPlayerIndex,
	        auditPublicKey,
	        auditEncryptionPublicKey,
	        deckAuditManifest: publicDeckManifest(deckAuditManifest),
        ...openDecklistPlayerFields({
          deck: deckSubmission.deck,
          sideboard: deckSubmission.sideboard,
          commanders: deckSubmission.commanders,
          deckSlotOpenings,
        }),
        ziffleKey: localZiffleKey,
        deckCount: deckSubmission.deckCount,
        sideboardCount: deckSubmission.sideboard.length,
        commanderCount: deckSubmission.commanderCount,
      },
    });

    updateMultiplayer((prev) => ({
      ...prev,
      localDeckCount: deckSubmission.deckCount,
      localCommanderCount: deckSubmission.commanderCount,
      players: reindexPlayers(
        prev.players.map((player) =>
          player.peerId === prev.localPeerId
            ? {
	                ...player,
	                auditPublicKey,
	                auditEncryptionPublicKey,
                playerGenesisSignature,
                deckAuditManifest: publicDeckManifest(deckAuditManifest),
                deck: deckSubmission.deck,
                sideboard: deckSubmission.sideboard,
                commanders: deckSubmission.commanders,
                deckSlotOpenings,
                ziffleKey: localZiffleKey,
                deckCount: deckSubmission.deckCount,
                sideboardCount: deckSubmission.sideboard.length,
                commanderCount: deckSubmission.commanderCount,
                ready: deckSubmission.ready,
              }
            : player
        )
      ),
    }));

    const conn = hostConnectionRef.current;
    if (!conn || conn.open === false) return;
    safeSend(conn, {
      type: "deck_update",
      protocolVersion: PROTOCOL_VERSION,
	      auditPublicKey,
	      auditEncryptionPublicKey,
	      playerGenesisSignature,
      deckAuditManifest: publicDeckManifest(deckAuditManifest),
      deck: deckSubmission.deck,
      sideboard: deckSubmission.sideboard,
      deckSlotOpenings,
      ziffleKey: localZiffleKey,
      deckCount: deckSubmission.deckCount,
	          sideboardCount: deckSubmission.sideboard.length,
	          commanders: deckSubmission.commanders,
	          commanderCount: deckSubmission.commanderCount,
	          ready: deckSubmission.ready,
	        });
  }

  const handleHostMessage = useCallback(
    async (message) => {
      if (!message || typeof message !== "object") return;
      if (message.protocolVersion !== PROTOCOL_VERSION) {
        setStatus("Lobby protocol version mismatch", true);
        return;
      }

      switch (message.type) {
        case "lobby_state": {
          const nextSession = updateMultiplayer((prev) => {
            const localEntry = (message.players || []).find(
              (player) => player.peerId === prev.localPeerId
            );
            if (localEntry) {
              writeStoredPlayerIndex(
                message.lobbyId || message.hostPeerId || prev.lobbyId,
                localEntry.index
              );
            }
            const nextFormat = normalizeMatchFormat(message.format || prev.format);
            const nextSecurityMode = normalizeMultiplayerSecurityMode(
              message.securityMode,
              prev.securityMode
            );
            const localDeckSubmission = parseDeckSubmission(
              nextFormat,
              prev.localDeckText,
              prev.localCommanderText
            );
            return {
              ...prev,
              mode: message.matchStarted ? "starting" : "lobby",
              lobbyId: message.lobbyId || prev.lobbyId,
              hostPeerId: message.hostPeerId || prev.hostPeerId,
              desiredPlayers: Number(message.desiredPlayers || prev.desiredPlayers || 0),
              startingLife: Number(message.startingLife || prev.startingLife || 20),
              format: nextFormat,
              securityMode: nextSecurityMode,
              localDeckCount: localDeckSubmission.deckCount,
              localCommanderCount: localDeckSubmission.commanderCount,
              players: message.players || [],
              localPlayerIndex: localEntry ? localEntry.index : prev.localPlayerIndex,
              matchStarted: Boolean(message.matchStarted),
              submittingAction: false,
            };
          });
          ensureDirectPeerConnections(message.players || []);
          void publishLocalDeckUpdateForAssignedSeat(nextSession).catch((err) => {
            emitSyncFailureNotice(
              "Deck commitment update failed",
              err instanceof Error ? err.message : String(err)
            );
            setStatus(`Deck commitment update failed: ${toErrorMessage(err)}`, true);
          });
          return;
        }
        case "reject":
          leaveLobby(message.reason || "Lobby join rejected", {
            clearStoredPlayer: message.reason !== "Player slot already filled",
            isError: true,
          });
          return;
        case "reconnect_challenge": {
          const conn = hostConnectionRef.current;
          if (!conn || conn.open === false) {
            setStatus("Reconnect challenge arrived without an open host connection", true);
            return;
          }
          const {
            publicKey: auditPublicKey,
            encryptionPublicKey: auditEncryptionPublicKey,
          } = await ensureAuditIdentity();
          const proof = await signReconnectProofForChallenge(message);
          safeSend(conn, {
            type: "join_request",
            protocolVersion: PROTOCOL_VERSION,
            name: multiplayerRef.current.localName || "Player",
            requestedPlayerIndex: Number(message.playerIndex),
            auditPublicKey,
            auditEncryptionPublicKey,
            reconnectChallengeId: message.requestId,
            reconnectProof: proof,
          });
          setStatus("Proving reconnect identity to host");
          return;
        }
        case "action_error":
          updateMultiplayer((prev) => ({ ...prev, submittingAction: false }));
          if (message.rollbackSynced) {
            const reason = message.reason || "Host rejected the local action";
            const rollbackSequence = Number(message.rollbackSequence || 0);
            emitSyncFailureNotice("Action reverted", reason);
            if (
              !requestResync(
                rollbackSequence > 0
                  ? `Host reverted invalid action at ${rollbackSequence}. Resyncing with host...`
                  : "Host reverted invalid action. Resyncing with host..."
              )
            ) {
              setStatus(reason, true);
            }
            return;
          }
          reportSyncFailure(
            message.reason || "Action rejected by multiplayer host",
            "Host rejected the local action. Resyncing with host...",
            message.reason || "Action rejected"
          );
          return;
        case "match_start":
          awaitingStateResyncRef.current = false;
          await applyMatchStart(message);
          return;
        case "ziffle_shuffle_step_request":
          await answerZiffleShuffleStepRequest(hostConnectionRef.current, message);
          return;
        case "ziffle_shuffle_step_response":
          resolveZiffleShuffleStep(message);
          return;
        case "rng_commit_request":
          await answerRngCommitRequest(hostConnectionRef.current, message);
          return;
        case "rng_reveal_request":
          await answerRngRevealRequest(hostConnectionRef.current, message);
          return;
        case "rng_commit_response":
          resolveRngCommit(message);
          return;
        case "rng_reveal_response":
          resolveRngReveal(message);
          return;
        case "timeout_vote_request":
          await answerTimeoutVoteRequest(hostConnectionRef.current, message);
          return;
        case "timeout_vote_response":
          resolveTimeoutVote(message);
          return;
        case "disconnect_forfeit_vote_request":
          await answerDisconnectForfeitVoteRequest(hostConnectionRef.current, message);
          return;
        case "disconnect_forfeit_vote_response":
          resolveTimeoutVote(message);
          return;
        case "protocol_timeout_vote_request":
          await answerProtocolResponseTimeoutVoteRequest(hostConnectionRef.current, message);
          return;
        case "protocol_timeout_vote_response":
          resolveTimeoutVote(message);
          return;
        case "action_quorum_vote_request":
          await answerActionQuorumVoteRequest(hostConnectionRef.current, message);
          return;
        case "action_quorum_vote_response":
          resolveActionQuorumVote(message);
          return;
        case "crypto_material_request":
          await answerCryptoMaterialRequest(hostConnectionRef.current, message);
          return;
        case "crypto_material_response":
          resolveCryptoMaterial(message);
          return;
        case "action_intent_progress":
          await handleActionIntentProgressMessage(message);
          return;
        case "action_intent_cancel":
          await handleActionIntentCancelMessage(message);
          return;
        case "ziffle_reveal_token_request":
          await answerZiffleRevealTokenRequest(hostConnectionRef.current, message);
          return;
        case "ziffle_reveal_token_response":
          resolveZiffleRevealToken(message);
          return;
        case "resync_ack":
          awaitingStateResyncRef.current = false;
          setStatus(`Already synced with host at action ${Number(message.lastSequence ?? 0)}`);
          return;
        case "rematch_start": {
          const rematch = buildRematchStateFromPayload(
            message.match,
            multiplayerRef.current.localPeerId
          );
          updateMultiplayer((prev) => ({
            ...prev,
            mode: "in_match",
            rematch,
          }));
          setStatus("Sideboard for the next game");
          return;
        }
        case "rematch_state": {
          const publicPlayersByPeer = new Map(
            (message.rematch?.players || []).map((player) => [player.peerId, player])
          );
          updateMultiplayer((prev) => {
            if (!prev.rematch) return prev;
            const players = (prev.rematch.players || []).map((player) => {
              const update = publicPlayersByPeer.get(player.peerId);
              if (!update) return player;
              return {
                ...player,
                ...update,
                deck: sanitizeCardList(update.deck),
                sideboard: sanitizeCardList(update.sideboard),
                commanders: sanitizeCardList(update.commanders),
                deckSlotOpenings: sanitizeDeckSlotOpenings(update.deckSlotOpenings),
                ready: Boolean(update.ready),
              };
            });
            const localPlayer = players.find((player) => player.peerId === prev.localPeerId);
            return {
              ...prev,
              rematch: {
                ...prev.rematch,
                phase: message.rematch?.phase || prev.rematch.phase,
                players,
                localReady: Boolean(localPlayer?.ready),
              },
            };
          });
          return;
        }
        case "state_resync":
          await applyStateResync(message);
          return;
        case "player_presence":
          updateMultiplayer((prev) => {
            const peerId = String(message.peerId || "").trim();
            const playerIndex = normalizePlayerIndex(message.playerIndex);
            const players = (prev.players || []).some((player) => player.peerId === peerId)
              ? prev.players
              : (prev.players || []).map((player) =>
                  playerIndex != null && Number(player.index) === playerIndex
                    ? { ...player, peerId }
                    : player
                );
            return {
              ...prev,
              players: markPlayerConnectionState(
                players,
                peerId,
                message.connected !== false,
                message.disconnectedAtMs == null ? Date.now() : Number(message.disconnectedAtMs)
              ),
            };
          });
          return;
        case "apply_action": {
          await applySequencedActionMessage(message);
          return;
        }
        default:
          return;
      }
    },
    [
      answerCryptoMaterialRequest,
      answerActionQuorumVoteRequest,
      answerTimeoutVoteRequest,
      answerDisconnectForfeitVoteRequest,
      answerProtocolResponseTimeoutVoteRequest,
      applyMatchStart,
      applyStateResync,
      applySyncedCommand,
      ensureDirectPeerConnections,
      leaveLobby,
      reportSyncFailure,
      requestResync,
      revealAuditOpenings,
      publishLocalDeckUpdateForAssignedSeat,
      resolveCryptoMaterial,
      resolveActionQuorumVote,
      resolveRngCommit,
      resolveRngReveal,
      resolveTimeoutVote,
      resolveZiffleRevealToken,
      resolveZiffleShuffleStep,
      setStatus,
      updateMultiplayer,
      verifySequencedActionAudit,
    ]
  );

  const handleClientDisconnect = useCallback(
    (peerId) => {
      clearConnectionHeartbeat(connectionHeartbeatKey("client", peerId));
      for (const [key, conn] of clientConnectionsRef.current.entries()) {
        if (String(key || "") === String(peerId || "") || String(conn?.peer || "") === String(peerId || "")) {
          clientConnectionsRef.current.delete(key);
        }
      }
      finishPeerResync(peerId);
      const departed = multiplayerRef.current.players.find(
        (player) => player.peerId === peerId
      );
      const disconnectedAtMs = Date.now();
      rememberLocalDisconnectObservation(peerId, {
        playerIndex: departed?.index,
        disconnectedAtMs,
        source: "host_client_connection",
      });
      updateMultiplayer((prev) => ({
        ...prev,
        players: markPlayerConnectionState(prev.players, peerId, false, disconnectedAtMs),
      }));
      if (departed) {
        setStatus(`${departed.name} disconnected; waiting 60s for timeout policy`);
      }
      if (multiplayerRef.current.matchStarted) {
        broadcastMatchPresence(peerId, false, {
          disconnectedAtMs,
          autoForfeitAtMs: disconnectedAtMs + DISCONNECT_AUTO_FORFEIT_MS,
        });
      } else {
        broadcastLobbyState();
      }
    },
    [
      broadcastLobbyState,
      broadcastMatchPresence,
      clearConnectionHeartbeat,
      finishPeerResync,
      setStatus,
      updateMultiplayer,
    ]
  );

  const handlePeerMessage = useCallback(async (conn, message) => {
    if (!message || typeof message !== "object") return;
    if (message.protocolVersion !== PROTOCOL_VERSION) {
      conn.close();
      return;
    }
    switch (message.type) {
      case "peer_ready":
        return;
      case "apply_action":
        await applySequencedActionMessage(message);
        return;
      case "ziffle_shuffle_step_request":
        await answerZiffleShuffleStepRequest(conn, message);
        return;
      case "ziffle_shuffle_step_response":
        resolveZiffleShuffleStep(message);
        return;
      case "ziffle_reveal_token_request":
        await answerZiffleRevealTokenRequest(conn, message);
        return;
      case "ziffle_reveal_token_response":
        resolveZiffleRevealToken(message);
        return;
      case "rng_commit_request":
        await answerRngCommitRequest(conn, message);
        return;
      case "rng_reveal_request":
        await answerRngRevealRequest(conn, message);
        return;
      case "rng_commit_response":
        resolveRngCommit(message);
        return;
      case "rng_reveal_response":
        resolveRngReveal(message);
        return;
      case "timeout_vote_request":
        await answerTimeoutVoteRequest(conn, message);
        return;
      case "timeout_vote_response":
        resolveTimeoutVote(message);
        return;
      case "disconnect_forfeit_vote_request":
        await answerDisconnectForfeitVoteRequest(conn, message);
        return;
      case "disconnect_forfeit_vote_response":
        resolveTimeoutVote(message);
        return;
      case "protocol_timeout_vote_request":
        await answerProtocolResponseTimeoutVoteRequest(conn, message);
        return;
      case "protocol_timeout_vote_response":
        resolveTimeoutVote(message);
        return;
      case "action_quorum_vote_request":
        await answerActionQuorumVoteRequest(conn, message);
        return;
      case "action_quorum_vote_response":
        resolveActionQuorumVote(message);
        return;
      case "crypto_material_request":
        await answerCryptoMaterialRequest(conn, message);
        return;
      case "crypto_material_response":
        resolveCryptoMaterial(message);
        return;
      case "action_intent_progress":
        await handleActionIntentProgressMessage(message);
        return;
      case "action_intent_cancel":
        await handleActionIntentCancelMessage(message);
        return;
      default:
        return;
    }
  }, [
    answerCryptoMaterialRequest,
    answerActionQuorumVoteRequest,
    answerTimeoutVoteRequest,
    answerDisconnectForfeitVoteRequest,
    answerProtocolResponseTimeoutVoteRequest,
    resolveActionQuorumVote,
    resolveCryptoMaterial,
    resolveRngCommit,
    resolveRngReveal,
    resolveTimeoutVote,
    resolveZiffleRevealToken,
    resolveZiffleShuffleStep,
  ]);

  const handlePeerDisconnect = useCallback(
    (peerId) => {
      clearConnectionHeartbeat(connectionHeartbeatKey("peer", peerId));
      if (peerConnectionsRef.current.get(peerId)?.peer === peerId) {
        peerConnectionsRef.current.delete(peerId);
      }
      if (multiplayerRef.current.matchStarted) {
        const departed = multiplayerRef.current.players.find(
          (player) => player.peerId === peerId
        );
        rememberLocalDisconnectObservation(peerId, {
          playerIndex: departed?.index,
          disconnectedAtMs: Date.now(),
          source: "direct_peer_connection",
        });
        if (departed) {
          setStatus(`${departed.name} disconnected from direct peer sync`, true);
        }
      }
    },
    [clearConnectionHeartbeat, setStatus, updateMultiplayer]
  );

  const configurePeerConnection = useCallback(
    (conn) => {
      const heartbeatKey = connectionHeartbeatKey("peer", conn.peer);
      const beginHeartbeat = () => {
        peerConnectionsRef.current.set(conn.peer, conn);
        const connectedPlayer = multiplayerRef.current.players.find(
          (player) => player.peerId === conn.peer
        );
        clearLocalDisconnectObservation(conn.peer, connectedPlayer?.index);
        if (multiplayerRef.current.matchStarted) {
          updateMultiplayer((prev) => ({
            ...prev,
            players: markPlayerConnectionState(prev.players, conn.peer, true),
          }));
        }
        startConnectionHeartbeat(heartbeatKey, conn, () => {
          if (peerConnectionsRef.current.get(conn.peer) !== conn) return;
          handlePeerDisconnect(conn.peer);
        });
        safeSend(conn, {
          type: "peer_ready",
          protocolVersion: PROTOCOL_VERSION,
          peerId: multiplayerRef.current.localPeerId || "",
        });
      };
      if (conn.open) {
        beginHeartbeat();
      } else {
        conn.on("open", beginHeartbeat);
      }
      conn.on("data", (message) => {
        markConnectionAlive(heartbeatKey);
        if (handleConnectionHeartbeatMessage(conn, message)) return;
        if (message?.type === "apply_action") {
          void enqueueAsync(peerMessageQueueRef, () =>
            handlePeerMessage(conn, message)
          ).catch((err) => {
            setStatus(`Peer message failed: ${toErrorMessage(err)}`, true);
          });
          return;
        }
        if (
          message?.type === "ziffle_shuffle_step_request"
          || message?.type === "ziffle_shuffle_step_response"
          || message?.type === "ziffle_reveal_token_request"
          || message?.type === "ziffle_reveal_token_response"
          || message?.type === "rng_commit_request"
          || message?.type === "rng_commit_response"
          || message?.type === "rng_reveal_request"
          || message?.type === "rng_reveal_response"
          || message?.type === "timeout_vote_request"
          || message?.type === "timeout_vote_response"
          || message?.type === "disconnect_forfeit_vote_request"
          || message?.type === "disconnect_forfeit_vote_response"
          || message?.type === "protocol_timeout_vote_request"
          || message?.type === "protocol_timeout_vote_response"
          || message?.type === "action_quorum_vote_request"
          || message?.type === "action_quorum_vote_response"
          || message?.type === "crypto_material_request"
          || message?.type === "crypto_material_response"
          || message?.type === "action_intent_progress"
          || message?.type === "action_intent_cancel"
        ) {
          void handlePeerMessage(conn, message).catch((err) => {
            if (shouldSuppressProtocolMessageError(err, message)) return;
            setStatus(`Peer message failed: ${toErrorMessage(err)}`, true);
          });
          return;
        }
        void enqueueAsync(peerMessageQueueRef, () =>
          handlePeerMessage(conn, message)
        ).catch((err) => {
          setStatus(`Peer message failed: ${toErrorMessage(err)}`, true);
        });
      });
      conn.on("close", () => {
        clearConnectionHeartbeat(heartbeatKey);
        if (peerConnectionsRef.current.get(conn.peer) === conn) {
          handlePeerDisconnect(conn.peer);
        }
      });
      conn.on("error", () => {
        clearConnectionHeartbeat(heartbeatKey);
        if (peerConnectionsRef.current.get(conn.peer) === conn) {
          handlePeerDisconnect(conn.peer);
        }
      });
    },
    [
      clearConnectionHeartbeat,
      handleConnectionHeartbeatMessage,
      handlePeerDisconnect,
      handlePeerMessage,
      markConnectionAlive,
      setStatus,
      startConnectionHeartbeat,
      updateMultiplayer,
    ]
  );

  const connectDirectPeer = useCallback(
    (peerId) => {
      const target = String(peerId || "").trim();
      const peer = peerRef.current;
      const session = multiplayerRef.current;
      if (!target || target === session.localPeerId || !peer || peer.destroyed) return null;
      const existing = peerConnectionsRef.current.get(target);
      if (existing?.open) return existing;
      if (existing) {
        try {
          existing.close();
        } catch (err) {
          void err;
        }
      }
	      const conn = peer.connect(target, {
	        reliable: true,
	        serialization: "binary",
	        metadata: {
          channel: "peer-direct",
          lobbyId: session.lobbyId || session.hostPeerId || "",
          from: session.localPeerId || "",
        },
      });
      peerConnectionsRef.current.set(target, conn);
      configurePeerConnection(conn);
      return conn;
    },
    [configurePeerConnection]
  );

  ensureDirectPeerConnectionsRef.current = (players = multiplayerRef.current.players) => {
    const session = multiplayerRef.current;
    const localPeerId = String(session.localPeerId || "").trim();
	    const hostPeerId = String(session.hostPeerId || "").trim();
	    if (!localPeerId || !peerRef.current || peerRef.current.destroyed) return;
	    for (const player of players || []) {
	      const peerId = routePeerIdForPlayer(player);
	      if (!peerId || peerId === localPeerId || peerId === hostPeerId) continue;
	      // Deterministic initiator prevents duplicate client-client channels.
	      if (localPeerId < peerId) {
	        connectDirectPeer(peerId);
      }
    }
  };

  const sendDirectPeerMessage = useCallback(
    (peerId, payload) => {
      const target = String(peerId || "").trim();
      if (!target) return false;
      const session = multiplayerRef.current;
      if (target === session.localPeerId) return true;
      const existingRoute = openZiffleRoute(target);
      if (existingRoute) {
        safeSend(existingRoute, payload);
        return true;
      }
      if (session.role === "host") {
        let conn = null;
        for (const candidate of ziffleRoutePeerCandidates(target)) {
          if (!candidate || candidate === session.localPeerId) continue;
          conn = connectDirectPeer(candidate);
          if (conn && conn.open !== false) break;
        }
        if (!conn || conn.open === false) return false;
        safeSend(conn, payload);
        return true;
      }
      if (target === session.hostPeerId) {
        let conn = hostConnectionRef.current;
        if (!conn || conn.open === false) {
          conn = peerConnectionsRef.current.get(target);
        }
        if (!conn || conn.open === false) {
          conn = connectDirectPeer(target);
        }
        if (!conn || conn.open === false) return false;
        safeSend(conn, payload);
        return true;
      }
      let conn = peerConnectionsRef.current.get(target);
      if (!conn || conn.open === false) {
        conn = connectDirectPeer(target);
      }
      if (!conn || conn.open === false) return false;
      safeSend(conn, payload);
      return true;
    },
    [connectDirectPeer]
  );

  function reconnectChallengeMapKey(peerId, requestId) {
    return [String(peerId || ""), String(requestId || "")].join(":");
  }

  function clearReconnectChallenge(peerId, requestId) {
    const key = reconnectChallengeMapKey(peerId, requestId);
    const challenge = reconnectChallengesRef.current.get(key);
    if (challenge?.timeoutId) {
      window.clearTimeout(challenge.timeoutId);
    }
    reconnectChallengesRef.current.delete(key);
  }

  function issueReconnectChallenge(conn, playerIndex) {
    const session = multiplayerRef.current;
    const requestId = makeZiffleRequestId("reconnect-proof");
    const challenge = {
      type: "reconnect_challenge",
      protocolVersion: PROTOCOL_VERSION,
      requestId,
      matchId: currentAuditMatchId(),
      playerIndex: Number(playerIndex),
      peerId: String(conn?.peer || ""),
      hostPeerId: String(session.localPeerId || session.hostPeerId || ""),
      transcriptHash: String(auditStateHashRef.current || INITIAL_AUDIT_STATE_HASH),
      nonce: randomAuditHex(32),
    };
    const key = reconnectChallengeMapKey(conn?.peer, requestId);
    const timeoutId = window.setTimeout(() => {
      reconnectChallengesRef.current.delete(key);
    }, PEER_CONNECT_TIMEOUT_MS);
    reconnectChallengesRef.current.set(key, {
      ...challenge,
      timeoutId,
    });
    safeSend(conn, challenge);
  }

  const handleClientMessage = useCallback(
    async (conn, message) => {
      if (!message || typeof message !== "object") return;
      if (message.protocolVersion !== PROTOCOL_VERSION) {
        safeSend(conn, {
          type: "reject",
          protocolVersion: PROTOCOL_VERSION,
          reason: "Protocol version mismatch",
        });
        conn.close();
        return;
      }

      switch (message.type) {
      case "ziffle_shuffle_step_request":
        await answerZiffleShuffleStepRequest(conn, message);
        return;
      case "ziffle_shuffle_step_response":
        resolveZiffleShuffleStep(message);
        return;
      case "rng_commit_response":
        resolveRngCommit(message);
        return;
      case "rng_reveal_response":
        resolveRngReveal(message);
        return;
      case "timeout_vote_response":
        resolveTimeoutVote(message);
        return;
      case "disconnect_forfeit_vote_response":
        resolveTimeoutVote(message);
        return;
      case "action_quorum_vote_response":
        resolveActionQuorumVote(message);
        return;
      case "crypto_material_response":
        resolveCryptoMaterial(message);
        return;
      case "apply_action":
        await applySequencedActionMessage(message);
        return;
      case "rng_commit_request":
        await answerRngCommitRequest(conn, message);
        return;
      case "rng_reveal_request":
        await answerRngRevealRequest(conn, message);
        return;
      case "timeout_vote_request":
        await answerTimeoutVoteRequest(conn, message);
        return;
      case "disconnect_forfeit_vote_request":
        await answerDisconnectForfeitVoteRequest(conn, message);
        return;
      case "protocol_timeout_vote_request":
        await answerProtocolResponseTimeoutVoteRequest(conn, message);
        return;
      case "protocol_timeout_vote_response":
        resolveTimeoutVote(message);
        return;
      case "action_quorum_vote_request":
        await answerActionQuorumVoteRequest(conn, message);
        return;
      case "crypto_material_request":
        await answerCryptoMaterialRequest(conn, message);
        return;
      case "action_intent_progress":
        await handleActionIntentProgressMessage(message);
        return;
      case "action_intent_cancel":
        await handleActionIntentCancelMessage(message);
        return;
      case "ziffle_reveal_token_request":
        await answerZiffleRevealTokenRequest(conn, message);
          return;
        case "ziffle_reveal_token_response":
          resolveZiffleRevealToken(message);
          return;
        case "join_request": {
          const session = multiplayerRef.current;
          const requestedPlayerIndex = normalizePlayerIndex(message.requestedPlayerIndex);
          const indexedPlayers = reindexPlayers(session.players);
          let targetPlayerIndex = null;

          if (requestedPlayerIndex != null) {
            const requestedSlot = indexedPlayers.find(
              (player) => Number(player.index) === requestedPlayerIndex
            );
            if (!requestedSlot || requestedPlayerIndex >= session.desiredPlayers) {
              safeSend(conn, {
                type: "reject",
                protocolVersion: PROTOCOL_VERSION,
                reason: "Player slot does not exist",
              });
              conn.close();
              return;
            }
            if (requestedSlot.connected !== false && requestedSlot.peerId !== conn.peer) {
              safeSend(conn, {
                type: "reject",
                protocolVersion: PROTOCOL_VERSION,
                reason: "Player slot already filled",
              });
              conn.close();
              return;
            }
            targetPlayerIndex = requestedPlayerIndex;
          } else {
            const disconnectedSlots = [...indexedPlayers]
              .sort((left, right) => Number(left.index || 0) - Number(right.index || 0))
              .filter((player) => player.connected === false);
            const matchingDisconnectedSlot = session.matchStarted
              ? disconnectedSlots.find((player) =>
                  playerMatchesPresentedAuditIdentity(
                    player,
                    message.auditPublicKey,
                    message.auditEncryptionPublicKey
                  )
                )
              : null;
            const disconnectedSlot = matchingDisconnectedSlot || disconnectedSlots[0] || null;
            if (disconnectedSlot) {
              targetPlayerIndex = Number(disconnectedSlot.index);
            } else if (!session.matchStarted && indexedPlayers.length < session.desiredPlayers) {
              targetPlayerIndex = indexedPlayers.length;
            }
          }

	          if (targetPlayerIndex == null) {
	            safeSend(conn, {
              type: "reject",
              protocolVersion: PROTOCOL_VERSION,
              reason: session.matchStarted
                ? "No disconnected player slots are available"
                : "Lobby is full",
            });
            conn.close();
	            return;
	          }

	          if (session.matchStarted) {
	            const existingSlot = indexedPlayers.find(
	              (player) => Number(player.index) === targetPlayerIndex
	            );
	            if (isVerifiedMultiplayerSecurityMode(sessionSecurityMode(session))) {
	              const expectedAuditKey = String(existingSlot?.auditPublicKey || "").trim();
	              const presentedAuditKey = String(message.auditPublicKey || "").trim();
	              const expectedEncryptionKey = String(existingSlot?.auditEncryptionPublicKey || "").trim();
	              const presentedEncryptionKey = String(message.auditEncryptionPublicKey || "").trim();
	              const proof = message.reconnectProof || null;
	              if (!expectedAuditKey || !expectedEncryptionKey) {
	                safeSend(conn, {
	                  type: "reject",
	                  protocolVersion: PROTOCOL_VERSION,
	                  reason: "Reconnect audit identity is missing from the player slot",
	                });
	                conn.close();
	                return;
	              }
	              const presentedIdentityMatches = Boolean(
	                presentedAuditKey
	                && presentedAuditKey === expectedAuditKey
	                && presentedEncryptionKey
	                && presentedEncryptionKey === expectedEncryptionKey
	              );
	              if (!presentedIdentityMatches && !proof) {
	                issueReconnectChallenge(conn, targetPlayerIndex);
	                return;
	              }
	              if (!presentedIdentityMatches) {
	                safeSend(conn, {
	                  type: "reject",
	                  protocolVersion: PROTOCOL_VERSION,
	                  reason: "Reconnect audit identity does not match the player slot",
	                });
	                conn.close();
	                return;
	              }
	              const challengeId = String(
	                proof?.challengeId
	                || proof?.requestId
                || message.reconnectChallengeId
                || ""
              );
              if (!challengeId) {
                issueReconnectChallenge(conn, targetPlayerIndex);
                return;
              }
              const challengeKey = reconnectChallengeMapKey(conn.peer, challengeId);
              const challenge = reconnectChallengesRef.current.get(challengeKey);
              if (!challenge) {
                safeSend(conn, {
                  type: "reject",
                  protocolVersion: PROTOCOL_VERSION,
                  reason: "Reconnect challenge expired; retry reconnect",
                });
                conn.close();
                return;
              }
              try {
                await verifyReconnectProofForChallenge(proof, challenge, expectedAuditKey);
              } catch (err) {
                clearReconnectChallenge(conn.peer, challengeId);
                safeSend(conn, {
                  type: "reject",
                  protocolVersion: PROTOCOL_VERSION,
                  reason: toErrorMessage(err, "Reconnect audit proof is invalid"),
                });
                conn.close();
                return;
              }
              clearReconnectChallenge(conn.peer, challengeId);
            }
		          }

	          clientConnectionsRef.current.set(conn.peer, conn);
          const name = sanitizePlayerName(
            message.name,
            `Player ${targetPlayerIndex + 1}`
          );
          const nextSession = updateMultiplayer((prev) => {
            const nextPlayers = [...reindexPlayers(prev.players)];
            const existingSlot = nextPlayers.find(
              (player) => Number(player.index) === targetPlayerIndex
            );
            const basePlayer = existingSlot || {
              name,
              index: targetPlayerIndex,
              ready: false,
              deck: [],
              commanders: [],
            };
	            const nextPlayer = prev.matchStarted
	              ? {
	                  ...basePlayer,
	                  peerId: basePlayer.peerId || conn.peer,
	                  currentPeerId: conn.peer,
	                  connected: true,
	                }
              : {
                  ...basePlayer,
	                  peerId: conn.peer,
	                  name,
	                  auditPublicKey: String(message.auditPublicKey || basePlayer.auditPublicKey || ""),
	                  auditEncryptionPublicKey: String(
	                    message.auditEncryptionPublicKey || basePlayer.auditEncryptionPublicKey || ""
	                  ),
	                  playerGenesisSignature:
                    message.playerGenesisSignature || basePlayer.playerGenesisSignature || null,
	                  deckAuditManifest:
	                    publicDeckManifest(message.deckAuditManifest)
	                    || publicDeckManifest(basePlayer.deckAuditManifest),
                  deck: sanitizeCardList(message.deck),
                  sideboard: sanitizeCardList(message.sideboard),
	                  ziffleKey: message.ziffleKey || basePlayer.ziffleKey || null,
	                  commanders: sanitizeCardList(message.commanders),
                  deckSlotOpenings: sanitizeDeckSlotOpenings(message.deckSlotOpenings),
	                  ready: Boolean(message.ready),
                  deckCount: Number(message.deckCount || 0),
                  sideboardCount: Number(message.sideboardCount || 0),
                  commanderCount: Number(message.commanderCount || 0),
                  connected: true,
                };
            const replaced = existingSlot
              ? nextPlayers.map((player) =>
                  Number(player.index) === targetPlayerIndex ? nextPlayer : player
                )
              : [...nextPlayers, nextPlayer];
            return {
              ...prev,
              mode: prev.matchStarted ? "in_match" : "lobby",
              players: reindexPlayers(replaced),
            };
          });
          const stablePeerId = nextSession.players.find(
            (player) => Number(player.index) === targetPlayerIndex
          )?.peerId;
          if (stablePeerId && stablePeerId !== conn.peer) {
            clientConnectionsRef.current.set(stablePeerId, conn);
          }

          clearLocalDisconnectObservation(conn.peer, targetPlayerIndex);

          if (nextSession.matchStarted) {
            const matchPayload = buildHostedResyncPayload();
            if (!matchPayload) {
              safeSend(conn, {
                type: "action_error",
                protocolVersion: PROTOCOL_VERSION,
                reason: "Host cannot rebuild the current match state",
              });
              return;
            }
            resyncingPeerIdsRef.current.add(conn.peer);
            try {
              await sendHostedStateMessage(conn, {
                type: "state_resync",
                protocolVersion: PROTOCOL_VERSION,
                match: matchPayload,
                lastSequence: nextSession.lastAppliedSequence,
              });
            } catch (err) {
              finishPeerResync(conn.peer);
              safeSend(conn, {
                type: "action_error",
                protocolVersion: PROTOCOL_VERSION,
                reason: toErrorMessage(err, "Host could not serialize current match state"),
              });
              return;
            }
            broadcastMatchPresence(conn.peer, true);
            setStatus(`${name} took over player ${targetPlayerIndex + 1}`);
            return;
          }

          setStatus(`${name} joined as player ${targetPlayerIndex + 1}`);
          broadcastLobbyState();
          return;
        }
        case "resync_ack": {
          const ackSequence = Number(message.lastSequence ?? 0);
          const wasPending = resyncingPeerIdsRef.current.has(conn.peer);
          finishPeerResync(conn.peer);
          if (!wasPending) return;
          const remaining = resyncingPeerIdsRef.current.size;
          setStatus(
            remaining > 0
              ? `Peer resynced; waiting for ${remaining} more`
              : `Peers resynced at action ${ackSequence}`
          );
          return;
        }
        case "resync_request": {
          const session = multiplayerRef.current;
          if (!session.matchStarted) {
            safeSend(conn, {
              type: "action_error",
              protocolVersion: PROTOCOL_VERSION,
              reason: "Match has not started",
            });
            return;
          }

          const existingPlayer = session.players.find((player) =>
            String(player.peerId || "") === String(conn.peer || "")
            || String(player.currentPeerId || "") === String(conn.peer || "")
          );
          if (!existingPlayer) {
            safeSend(conn, {
              type: "reject",
              protocolVersion: PROTOCOL_VERSION,
              reason: "This peer is not part of the active match",
            });
            conn.close();
            return;
          }

          const requesterSequence = Number(message.lastSequence ?? 0);
          if (
            existingPlayer.connected !== false
            && Number.isSafeInteger(requesterSequence)
            && requesterSequence >= Number(session.lastAppliedSequence || 0)
          ) {
            clientConnectionsRef.current.set(conn.peer, conn);
            clearLocalDisconnectObservation(conn.peer, existingPlayer?.index);
            updateMultiplayer((prev) => ({
              ...prev,
              players: markPlayerConnectionState(prev.players, conn.peer, true),
            }));
            safeSend(conn, {
              type: "resync_ack",
              protocolVersion: PROTOCOL_VERSION,
              lastSequence: session.lastAppliedSequence,
            });
            return;
          }

          if (isVerifiedMultiplayerSecurityMode(sessionSecurityMode(session))) {
            if (!message.reconnectProof) {
              issueReconnectChallenge(conn, existingPlayer.index);
              return;
            }
            const challengeId = String(
              message.reconnectProof?.challengeId
              || message.reconnectProof?.requestId
              || message.reconnectChallengeId
              || ""
            );
            const challenge = reconnectChallengesRef.current.get(
              reconnectChallengeMapKey(conn.peer, challengeId)
            );
            if (!challenge) {
              safeSend(conn, {
                type: "reject",
                protocolVersion: PROTOCOL_VERSION,
                reason: "Reconnect challenge expired; retry reconnect",
              });
              conn.close();
              return;
            }
            try {
              await verifyReconnectProofForChallenge(
                message.reconnectProof,
                challenge,
                String(existingPlayer.auditPublicKey || "")
              );
            } catch (err) {
              clearReconnectChallenge(conn.peer, challengeId);
              safeSend(conn, {
                type: "reject",
                protocolVersion: PROTOCOL_VERSION,
                reason: toErrorMessage(err, "Reconnect audit proof is invalid"),
              });
              conn.close();
              return;
            }
            clearReconnectChallenge(conn.peer, challengeId);
          }

          clientConnectionsRef.current.set(conn.peer, conn);
          clearLocalDisconnectObservation(conn.peer, existingPlayer?.index);
          const nextSession = updateMultiplayer((prev) => ({
            ...prev,
            players: markPlayerConnectionState(prev.players, conn.peer, true),
          }));
          const matchPayload = buildHostedResyncPayload();
          if (!matchPayload) {
            safeSend(conn, {
              type: "action_error",
              protocolVersion: PROTOCOL_VERSION,
              reason: "Host cannot rebuild the current match state",
            });
            return;
          }

          resyncingPeerIdsRef.current.add(conn.peer);
          try {
            await sendHostedStateMessage(conn, {
              type: "state_resync",
              protocolVersion: PROTOCOL_VERSION,
              match: matchPayload,
              lastSequence: nextSession.lastAppliedSequence,
            });
          } catch (err) {
            finishPeerResync(conn.peer);
            safeSend(conn, {
              type: "action_error",
              protocolVersion: PROTOCOL_VERSION,
              reason: toErrorMessage(err, "Host could not serialize current match state"),
            });
            return;
          }
          broadcastMatchPresence(conn.peer, true);
          setStatus(`${existingPlayer.name} is resyncing; host actions paused`);
          return;
        }
        case "deck_update": {
          const session = multiplayerRef.current;
          if (session.matchStarted) return;
          updateMultiplayer((prev) => ({
            ...prev,
            players: reindexPlayers(
              prev.players.map((player) =>
                player.peerId === conn.peer
                  ? {
	                      ...player,
	                      auditPublicKey: String(message.auditPublicKey || player.auditPublicKey || ""),
	                      auditEncryptionPublicKey: String(
	                        message.auditEncryptionPublicKey || player.auditEncryptionPublicKey || ""
	                      ),
	                      playerGenesisSignature:
                        message.playerGenesisSignature || player.playerGenesisSignature || null,
	                      deckAuditManifest:
	                        publicDeckManifest(message.deckAuditManifest)
	                        || publicDeckManifest(player.deckAuditManifest),
                      deck: sanitizeCardList(message.deck),
                      sideboard: sanitizeCardList(message.sideboard),
	                      ziffleKey: message.ziffleKey || player.ziffleKey || null,
	                      commanders: sanitizeCardList(message.commanders),
                      deckSlotOpenings: sanitizeDeckSlotOpenings(message.deckSlotOpenings),
	                      ready: Boolean(message.ready),
                      deckCount: Number(message.deckCount || 0),
                      sideboardCount: Number(message.sideboardCount || 0),
                      commanderCount: Number(message.commanderCount || 0),
                    }
                  : player
              )
            ),
          }));
          broadcastLobbyState();
          return;
        }
        case "rematch_request": {
          const session = multiplayerRef.current;
          if (!session.matchStarted) {
            safeSend(conn, {
              type: "action_error",
              protocolVersion: PROTOCOL_VERSION,
              reason: "Match has not started",
            });
            return;
          }
          const actor = session.players.find((player) => player.peerId === conn.peer);
          if (!actor) {
            safeSend(conn, {
              type: "action_error",
              protocolVersion: PROTOCOL_VERSION,
              reason: "This peer is not assigned to an active seat",
            });
            return;
          }
          startRematchSideboarding("remote");
          return;
        }
        case "rematch_ready": {
          const session = multiplayerRef.current;
          const rematch = session.rematch;
          if (!session.matchStarted || !rematch || rematch.phase !== "sideboarding") {
            safeSend(conn, {
              type: "action_error",
              protocolVersion: PROTOCOL_VERSION,
              reason: "Sideboarding is not active",
            });
            return;
          }
          const actor = rematch.players.find((player) => player.peerId === conn.peer);
          if (!actor) {
            safeSend(conn, {
              type: "action_error",
              protocolVersion: PROTOCOL_VERSION,
              reason: "This peer is not assigned to an active seat",
            });
            return;
          }

          const trustedRematch = isTrustedMultiplayerSecurityMode(sessionSecurityMode(session));
          const nextSession = updateMultiplayer((prev) => {
            if (!prev.rematch) return prev;
            const players = reindexPlayers(prev.rematch.players || []).map((player) => (
	              player.peerId === conn.peer
	                ? {
	                    ...player,
                    auditPublicKey: trustedRematch
                      ? ""
                      : String(message.auditPublicKey || player.auditPublicKey || ""),
                    auditEncryptionPublicKey: trustedRematch
                      ? ""
                      : String(message.auditEncryptionPublicKey || player.auditEncryptionPublicKey || ""),
                    playerGenesisSignature:
                      trustedRematch
                        ? null
                        : message.playerGenesisSignature || player.playerGenesisSignature || null,
                    deck: sanitizeCardList(message.deck),
                    sideboard: sanitizeCardList(message.sideboard),
                    commanders: sanitizeCardList(message.commanders || player.commanders),
	                    deckAuditManifest:
                      trustedRematch
                        ? null
                        : publicDeckManifest(message.deckAuditManifest)
                          || publicDeckManifest(player.deckAuditManifest),
                    deckSlotOpenings: trustedRematch
                      ? []
                      : sanitizeDeckSlotOpenings(message.deckSlotOpenings),
                    ziffleKey: trustedRematch ? null : message.ziffleKey || player.ziffleKey || null,
	                    deckCount: Number(message.deckCount || 0),
	                    sideboardCount: Number(message.sideboardCount || 0),
                    commanderCount: Number(message.commanderCount || player.commanderCount || 0),
	                    ready: true,
	                  }
                : player
            ));
            return {
              ...prev,
              rematch: {
                ...prev.rematch,
                players,
              },
            };
          });
          broadcastRematchState(nextSession.rematch);
          if (rematchPlayersReady(nextSession.rematch?.players)) {
            await startRematchFromState(nextSession.rematch);
          } else {
            setStatus(`${actor.name} is ready for rematch`);
          }
          return;
        }
        default:
          return;
      }
    },
    [
      answerCryptoMaterialRequest,
      answerActionQuorumVoteRequest,
      answerTimeoutVoteRequest,
      answerDisconnectForfeitVoteRequest,
      answerProtocolResponseTimeoutVoteRequest,
      buildHostedResyncPayload,
      broadcastMatchPresence,
      broadcastLobbyState,
      broadcastRematchState,
      finishPeerResync,
      sendHostedStateMessage,
      setStatus,
      startRematchFromState,
      startRematchSideboarding,
      updateMultiplayer,
      resolveCryptoMaterial,
      resolveActionQuorumVote,
      resolveRngCommit,
      resolveRngReveal,
      resolveTimeoutVote,
      resolveZiffleRevealToken,
      resolveZiffleShuffleStep,
    ]
  );

  const configureHostConnection = useCallback(
    (conn) => {
      const heartbeatKey = connectionHeartbeatKey("client", conn.peer);
      const beginHeartbeat = () => startConnectionHeartbeat(heartbeatKey, conn, () => {
        if (clientConnectionsRef.current.get(conn.peer) !== conn) return;
        handleClientDisconnect(conn.peer);
      });
      if (conn.open) {
        beginHeartbeat();
      } else {
        conn.on("open", beginHeartbeat);
      }
      conn.on("data", (message) => {
        markConnectionAlive(heartbeatKey);
        if (handleConnectionHeartbeatMessage(conn, message)) return;
        const handleError = (err) => {
          if (shouldSuppressProtocolMessageError(err, message)) return;
          safeSend(conn, {
            type: "action_error",
            protocolVersion: PROTOCOL_VERSION,
            reason: toErrorMessage(err),
            rollbackSynced: Boolean(err?.syncedRollbackBroadcast),
            rollbackSequence:
              err?.syncedRollbackSequence == null ? null : Number(err.syncedRollbackSequence),
          });
        };
        if (message?.type === "apply_action") {
          void enqueueAsync(clientMessageQueueRef, () =>
            handleClientMessage(conn, message)
          ).catch(handleError);
          return;
        }
        // Acks must not wait behind actions that are blocked on those same acks.
        if (
          message?.type === "resync_ack"
          || message?.type === "ziffle_shuffle_step_request"
          || message?.type === "ziffle_shuffle_step_response"
          || message?.type === "ziffle_reveal_token_request"
          || message?.type === "ziffle_reveal_token_response"
          || message?.type === "rng_commit_request"
          || message?.type === "rng_commit_response"
          || message?.type === "rng_reveal_request"
          || message?.type === "rng_reveal_response"
          || message?.type === "timeout_vote_request"
          || message?.type === "timeout_vote_response"
          || message?.type === "disconnect_forfeit_vote_request"
          || message?.type === "disconnect_forfeit_vote_response"
          || message?.type === "protocol_timeout_vote_request"
          || message?.type === "protocol_timeout_vote_response"
          || message?.type === "action_quorum_vote_request"
          || message?.type === "action_quorum_vote_response"
          || message?.type === "action_intent_progress"
          || message?.type === "action_intent_cancel"
        ) {
          void handleClientMessage(conn, message).catch(handleError);
          return;
        }
        void enqueueAsync(clientMessageQueueRef, () =>
          handleClientMessage(conn, message)
        ).catch(handleError);
      });
      conn.on("close", () => {
        clearConnectionHeartbeat(heartbeatKey);
        if (clientConnectionsRef.current.get(conn.peer) !== conn) return;
        handleClientDisconnect(conn.peer);
      });
      conn.on("error", () => {
        clearConnectionHeartbeat(heartbeatKey);
        if (clientConnectionsRef.current.get(conn.peer) !== conn) return;
        handleClientDisconnect(conn.peer);
      });
    },
    [
      clearConnectionHeartbeat,
      handleClientDisconnect,
      handleClientMessage,
      handleConnectionHeartbeatMessage,
      markConnectionAlive,
      startConnectionHeartbeat,
    ]
  );

  const configureIncomingConnection = useCallback(
    (conn) => {
      if (conn?.metadata?.channel === "peer-direct") {
        configurePeerConnection(conn);
        return;
      }
      configureHostConnection(conn);
    },
    [configureHostConnection, configurePeerConnection]
  );

  const promoteLocalPlayerToHost = useCallback(
    (reason = "Lobby host disconnected.") => {
      const session = multiplayerRef.current;
      const lobbyId = String(session.lobbyId || session.hostPeerId || "").trim();
      const localPlayerIndex = resolveReconnectPlayerIndex(session, lobbyId);
      if (session.role !== "client" || !lobbyId || localPlayerIndex == null) {
        return false;
      }

	      // Older clients did not preserve a stable hostPeerId in every payload.
	      // If the host id no longer matches any slot, treat player 1 as the
	      // disconnected original host so the remaining clients can still elect.
	      const reclaimingOriginalHost = localPlayerIndex === 0;
	      if (reclaimingOriginalHost) {
	        updateMultiplayer((prev) => ({
	          ...prev,
	          mode: prev.matchStarted ? "in_match" : "joining",
	          hostPeerId: lobbyId,
	          submittingAction: false,
	        }));
	        return false;
	      }
	      const fallbackHostIndex = 0;
	      const playersWithHostOffline = markHostPeerDisconnected(
	        session.players,
	        session.hostPeerId || lobbyId,
	        lobbyId,
	        fallbackHostIndex
	      );
      const nextHost = findNextHostPlayer(playersWithHostOffline);
      if (!nextHost || Number(nextHost.index) !== localPlayerIndex) {
        updateMultiplayer((prev) => ({
          ...prev,
          mode: prev.matchStarted ? "in_match" : "joining",
          hostPeerId: lobbyId,
          players: playersWithHostOffline,
          submittingAction: false,
        }));
        return false;
      }

      const previousPeer = peerRef.current;
      const previousHostConnection = hostConnectionRef.current;
      hostConnectionRef.current = null;
      if (previousHostConnection) {
        try {
          previousHostConnection.close();
        } catch (err) {
          void err;
        }
      }

      for (const conn of clientConnectionsRef.current.values()) {
        try {
          conn.close();
        } catch (err) {
          void err;
        }
      }
      clientConnectionsRef.current.clear();
      clearAllConnectionHeartbeats();
      clearAllPeerResyncs();
      awaitingStateResyncRef.current = false;

      if (previousPeer) {
        try {
          previousPeer.destroy();
        } catch (err) {
          void err;
        }
      }
      peerRef.current = null;

      updateMultiplayer((prev) => ({
        ...prev,
        role: "host",
        mode: "hosting",
        lobbyId,
        hostPeerId: lobbyId,
        localPlayerIndex,
        desiredPlayers: Math.max(
          Number(prev.desiredPlayers || 0),
          playersWithHostOffline.length,
          2
        ),
        players: reclaimingOriginalHost
          ? ensurePromotedLocalPlayer(prev.players, prev, lobbyId, localPlayerIndex)
          : markHostPeerDisconnected(
              prev.players,
              prev.hostPeerId || lobbyId,
              lobbyId,
              fallbackHostIndex
            ),
        submittingAction: false,
      }));

      let takeoverAttempts = 0;
      const startTakeoverPeer = () => {
        const latestSession = multiplayerRef.current;
        if (
          latestSession.role !== "host" ||
          latestSession.lobbyId !== lobbyId ||
          normalizePlayerIndex(latestSession.localPlayerIndex) !== localPlayerIndex
        ) {
          return;
        }

        takeoverAttempts += 1;
        const takeoverPeer = createPeer(lobbyId, peerOptionsRef.current);
        peerRef.current = takeoverPeer;
        let reconnectTimer = null;
        let reconnectAttempts = 0;
        const clearReconnect = () => {
          if (reconnectTimer) {
            clearTimeout(reconnectTimer);
            reconnectTimer = null;
          }
          reconnectAttempts = 0;
        };
        const scheduleReconnect = (statusReason) => {
          if (peerRef.current !== takeoverPeer || takeoverPeer.destroyed || reconnectTimer) return;
          reconnectAttempts += 1;
          const delay = Math.min(8000, 1000 * reconnectAttempts);
          setStatus(
            `${statusReason} Retrying signaling in ${Math.ceil(delay / 1000)}s...`,
            true
          );
          reconnectTimer = window.setTimeout(() => {
            reconnectTimer = null;
            if (peerRef.current !== takeoverPeer || takeoverPeer.destroyed) return;
            try {
              takeoverPeer.reconnect();
            } catch (err) {
              setStatus(formatPeerError(err, "Could not reconnect lobby signaling"), true);
              leaveLobby("");
            }
          }, delay);
        };
        const scheduleTakeoverRetry = (err) => {
          if (peerRef.current !== takeoverPeer) return;
          try {
            takeoverPeer.destroy();
          } catch (destroyErr) {
            void destroyErr;
          }
          peerRef.current = null;
          const delay = Math.min(8000, 1000 * takeoverAttempts);
          setStatus(
            `${formatPeerError(err, "Could not take over lobby host")} Retrying host takeover in ${Math.ceil(delay / 1000)}s...`,
            true
          );
          window.setTimeout(startTakeoverPeer, delay);
        };
        const openTimeout = window.setTimeout(() => {
          if (peerRef.current !== takeoverPeer || takeoverPeer.open) return;
          scheduleTakeoverRetry("Timed out while claiming the lobby host id");
        }, PEER_OPEN_TIMEOUT_MS);

        setStatus(
          takeoverAttempts === 1
            ? `${reason} Taking over lobby host...`
            : "Retrying lobby host takeover..."
        );

        takeoverPeer.on("open", (peerId) => {
          if (peerRef.current !== takeoverPeer) return;
          clearTimeout(openTimeout);
          clearReconnect();
          const current = multiplayerRef.current;
          const currentDeck = parseDeckSubmission(
            current.format,
            current.localDeckText,
            current.localCommanderText
          );
          const nextPlayers = reindexPlayers(
            current.players.map((player) => {
              if (Number(player.index) === localPlayerIndex) {
                const nextPlayer = {
                  ...player,
                  peerId,
                  currentPeerId: peerId,
                  auditPublicKey: current.matchStarted
                    ? player.auditPublicKey
                    : auditPublicKeyRef.current || player.auditPublicKey || "",
                  auditEncryptionPublicKey: current.matchStarted
                    ? player.auditEncryptionPublicKey
                    : auditEncryptionPublicKeyRef.current || player.auditEncryptionPublicKey || "",
                  connected: true,
                };
                return current.matchStarted
                  ? nextPlayer
                  : withDeckState(
                      nextPlayer,
                      current.format,
                      currentDeck.deck,
                      currentDeck.commanders,
                      currentDeck.sideboard
                    );
              }
              return {
                ...player,
                peerId:
                  player.peerId === peerId
                    ? createOfflinePeerId(lobbyId, player.index)
                    : player.peerId,
                connected: false,
                disconnectedAtMs: player.disconnectedAtMs || Date.now(),
                autoForfeitAtMs:
                  player.autoForfeitAtMs
                  || (Date.now() + DISCONNECT_AUTO_FORFEIT_MS),
              };
            })
          );
          writeStoredPlayerIndex(lobbyId, localPlayerIndex);
          const nextSession = updateMultiplayer((prev) => ({
            ...prev,
            role: "host",
            mode: prev.matchStarted ? "in_match" : "lobby",
            lobbyId,
            hostPeerId: peerId,
            localPeerId: peerId,
            localPlayerIndex,
            localDeckCount: currentDeck.deckCount,
            localCommanderCount: currentDeck.commanderCount,
            players: nextPlayers,
            submittingAction: false,
          }));

	          if (matchStartPayloadRef.current) {
	            matchStartPayloadRef.current = {
	              ...cloneMultiplayerPayload(matchStartPayloadRef.current),
	              currentLobbyId: lobbyId,
	              currentHostPeerId: peerId,
	              currentHostPlayerIndex: localPlayerIndex,
	              currentPlayers: toPublicPlayers(nextSession.players),
	            };
	          }

          setStatus(`You are now the lobby host: ${lobbyId}`);
        });
        takeoverPeer.on("connection", configureIncomingConnection);
        takeoverPeer.on("error", (err) => {
          if (peerRef.current !== takeoverPeer) return;
          clearTimeout(openTimeout);
          const type = String(err?.type || "").trim();
          if (type === "unavailable-id" || isRecoverablePeerError(err)) {
            scheduleTakeoverRetry(err);
            return;
          }
          setStatus(formatPeerError(err, "Lobby host takeover failed"), true);
          leaveLobby("");
        });
        takeoverPeer.on("disconnected", () => {
          if (peerRef.current !== takeoverPeer) return;
          clearTimeout(openTimeout);
          scheduleReconnect(
            `Disconnected from the PeerJS signaling server (${peerServerLabelRef.current}).`
          );
        });
        takeoverPeer.on("close", () => {
          clearTimeout(openTimeout);
          clearReconnect();
        });
      };

      startTakeoverPeer();
      return true;
    },
    [
      clearAllPeerResyncs,
      clearAllConnectionHeartbeats,
      configureIncomingConnection,
      leaveLobby,
      peerOptionsRef,
      setStatus,
      updateMultiplayer,
    ]
  );

  const createLobby = useCallback(
    async ({
      name,
      desiredPlayers,
      startingLife,
      format = MATCH_FORMAT_NORMAL,
      securityMode = MULTIPLAYER_SECURITY_TRUSTED,
      deckText = "",
      commanderText = "",
    }) => {
      teardownPeer();
      const normalizedSecurityMode = normalizeMultiplayerSecurityMode(securityMode);
      const auditIdentity = isVerifiedMultiplayerSecurityMode(normalizedSecurityMode)
        ? await ensureAuditIdentity()
        : { publicKey: "", encryptionPublicKey: "" };
      const {
        publicKey: auditPublicKey,
        encryptionPublicKey: auditEncryptionPublicKey,
      } = auditIdentity;
      const localName = sanitizePlayerName(name, "Host");
      const targetPlayers = Math.max(
        CURRENT_AUDIT_MIN_PLAYERS,
        Math.min(
          CURRENT_AUDIT_MAX_PLAYERS,
          Number(desiredPlayers) || CURRENT_AUDIT_MIN_PLAYERS
        )
      );
      const lifeTotal = Math.max(1, Number(startingLife) || 20);
      const normalizedFormat = normalizeMatchFormat(format);
      const deckSubmission = parseDeckSubmission(
        normalizedFormat,
        deckText,
        commanderText
      );
      const peer = createPeer("", peerOptionsRef.current);
      peerRef.current = peer;
      let reconnectTimer = null;
      let reconnectAttempts = 0;
      const clearReconnect = () => {
        if (reconnectTimer) {
          clearTimeout(reconnectTimer);
          reconnectTimer = null;
        }
        reconnectAttempts = 0;
      };
      const scheduleReconnect = (reason) => {
        if (peerRef.current !== peer || peer.destroyed || reconnectTimer) return;
        reconnectAttempts += 1;
        const delay = Math.min(8000, 1000 * reconnectAttempts);
        setStatus(
          `${reason} Retrying signaling in ${Math.ceil(delay / 1000)}s...`,
          true
        );
        reconnectTimer = window.setTimeout(() => {
          reconnectTimer = null;
          if (peerRef.current !== peer || peer.destroyed) return;
          try {
            peer.reconnect();
          } catch (err) {
            setStatus(formatPeerError(err, "Could not reconnect lobby signaling"), true);
            leaveLobby("");
          }
        }, delay);
      };
      const openTimeout = window.setTimeout(() => {
        if (peerRef.current !== peer || peer.open) return;
        setStatus(
          `Could not register the lobby with the PeerJS signaling server (${peerServerLabelRef.current}).`,
          true
        );
        leaveLobby("");
      }, PEER_OPEN_TIMEOUT_MS);

      updateMultiplayer({
        ...createEmptyState(),
        role: "host",
        mode: "hosting",
        localName,
        desiredPlayers: targetPlayers,
        startingLife: lifeTotal,
        format: normalizedFormat,
        securityMode: normalizedSecurityMode,
        signalingServer: peerServerLabelRef.current,
        localDeckText: String(deckText || ""),
        localCommanderText: String(commanderText || ""),
        localDeckCount: deckSubmission.deckCount,
        localCommanderCount: deckSubmission.commanderCount,
      });
      setStatus(`Registering lobby with PeerJS (${peerServerLabelRef.current})...`);

      peer.on("open", async (peerId) => {
        clearTimeout(openTimeout);
        clearReconnect();
        const session = multiplayerRef.current;
        const isReconnect =
          session.role === "host" && session.localPeerId && session.localPeerId === peerId;
        if (isReconnect) {
          updateMultiplayer((prev) => ({
            ...prev,
            mode: prev.matchStarted ? "in_match" : "lobby",
            lobbyId: prev.lobbyId || peerId,
            hostPeerId: peerId,
            localPeerId: peerId,
            players: prev.players.map((player) =>
              player.peerId === peerId
                ? { ...player, auditPublicKey, auditEncryptionPublicKey }
                : player
            ),
          }));
          if (!session.matchStarted) {
            broadcastLobbyState();
          }
          setStatus(`Lobby signaling reconnected: ${peerId}`);
          return;
        }
        const currentDeck = parseDeckSubmission(
          session.format,
          session.localDeckText,
          session.localCommanderText
        );
        let deckAuditManifest = null;
        let deckSlotOpenings = [];
        let ziffleKey = null;
        let playerGenesisSignature = null;
        if (isVerifiedMultiplayerSecurityMode(normalizedSecurityMode)) {
          deckAuditManifest = await buildLocalDeckAuditManifest({
            matchId: peerId,
            owner: 0,
            deck: currentDeck.deck,
            sideboard: currentDeck.sideboard,
            commanders: currentDeck.commanders,
          });
          deckSlotOpenings = deckSlotOpeningsForManifest(deckAuditManifest);
          const ziffleKeyPair = await ensureZiffleIdentity({
            context: peerId,
            deckCount: currentDeck.deckCount || 60,
          });
          ziffleKey = publicZiffleKey(ziffleKeyPair, 0);
          playerGenesisSignature = await signPlayerGenesis({
            matchId: peerId,
            player: {
              peerId,
              name: localName,
              index: 0,
              auditPublicKey,
              auditEncryptionPublicKey,
              deckAuditManifest: publicDeckManifest(deckAuditManifest),
              ziffleKey,
              ...openDecklistPlayerFields({
                deck: currentDeck.deck,
                sideboard: currentDeck.sideboard,
                commanders: currentDeck.commanders,
                deckSlotOpenings,
              }),
              deckCount: currentDeck.deckCount,
              sideboardCount: currentDeck.sideboard.length,
              commanderCount: currentDeck.commanderCount,
            },
          });
        }
        writeStoredPlayerIndex(peerId, 0);
        updateMultiplayer((prev) => ({
          ...prev,
          mode: "lobby",
          lobbyId: peerId,
          hostPeerId: peerId,
          localPeerId: peerId,
          localPlayerIndex: 0,
          localDeckCount: currentDeck.deckCount,
          localCommanderCount: currentDeck.commanderCount,
          players: [
            withDeckState(
              {
                peerId,
                name: localName,
                index: 0,
                auditPublicKey,
                auditEncryptionPublicKey,
                deckAuditManifest: publicDeckManifest(deckAuditManifest),
                deckSlotOpenings,
                ziffleKey,
                playerGenesisSignature,
                connected: true,
              },
              prev.format,
              currentDeck.deck,
              currentDeck.commanders,
              currentDeck.sideboard
            ),
          ],
        }));
        setStatus(`Lobby created: ${peerId}`);
      });
      peer.on("connection", configureIncomingConnection);
      peer.on("error", (err) => {
        clearTimeout(openTimeout);
        if (isRecoverablePeerError(err)) {
          scheduleReconnect(formatPeerError(err, "Lost lobby signaling"));
          return;
        }
        setStatus(formatPeerError(err, "Lobby error"), true);
        leaveLobby("");
      });
      peer.on("disconnected", () => {
        clearTimeout(openTimeout);
        scheduleReconnect(
          `Disconnected from the PeerJS signaling server (${peerServerLabelRef.current}).`
        );
      });
      peer.on("close", () => {
        clearTimeout(openTimeout);
        clearReconnect();
      });
    },
    [
      broadcastLobbyState,
      buildLocalDeckAuditManifest,
      configureIncomingConnection,
      ensureAuditIdentity,
      ensureZiffleIdentity,
      leaveLobby,
      peerOptionsRef,
      publicZiffleKey,
      setStatus,
      signPlayerGenesis,
      teardownPeer,
      updateMultiplayer,
    ]
  );

  const joinLobby = useCallback(
    async ({ name, lobbyId, deckText = "", commanderText = "" }) => {
      teardownPeer();
      const localName = sanitizePlayerName(name, "Guest");
      const targetLobby = String(lobbyId || "").trim();
      if (!targetLobby) {
        setStatus("Lobby code is required", true);
        return;
      }

      const deckSubmission = parseDeckSubmission(
        MATCH_FORMAT_NORMAL,
        deckText,
        commanderText
      );
      const peer = createPeer("", peerOptionsRef.current);
      peerRef.current = peer;
      let reconnectTimer = null;
      let reconnectAttempts = 0;
      let hostReconnectTimer = null;
      let hostReconnectAttempts = 0;
      const clearReconnect = () => {
        if (reconnectTimer) {
          clearTimeout(reconnectTimer);
          reconnectTimer = null;
        }
        reconnectAttempts = 0;
      };
      const clearHostReconnect = () => {
        if (hostReconnectTimer) {
          clearTimeout(hostReconnectTimer);
          hostReconnectTimer = null;
        }
        hostReconnectAttempts = 0;
      };
      const peerOpenTimeout = window.setTimeout(() => {
        if (peerRef.current !== peer || peer.open) return;
        setStatus(
          `Could not connect to the PeerJS signaling server (${peerServerLabelRef.current}).`,
          true
        );
        leaveLobby("");
      }, PEER_OPEN_TIMEOUT_MS);

      updateMultiplayer({
        ...createEmptyState(),
        role: "client",
        mode: "joining",
        lobbyId: targetLobby,
        hostPeerId: targetLobby,
        localName,
        signalingServer: peerServerLabelRef.current,
        localDeckText: String(deckText || ""),
        localCommanderText: String(commanderText || ""),
        localDeckCount: deckSubmission.deckCount,
        localCommanderCount: deckSubmission.commanderCount,
      });
      setStatus(`Connecting to the PeerJS signaling server (${peerServerLabelRef.current})...`);

      const scheduleHostReconnect = (reason) => {
        if (peerRef.current !== peer || peer.destroyed) {
          return;
        }
        const inMatch = multiplayerRef.current.matchStarted;

        updateMultiplayer((prev) => ({
          ...prev,
          mode: inMatch ? "in_match" : "joining",
          submittingAction: false,
        }));

        if (peer.disconnected || !peer.open) {
          setStatus(`${reason} Waiting for the signaling server to reconnect...`, true);
          return;
        }
        if (hostReconnectTimer) return;

        hostReconnectAttempts += 1;
        const delay = Math.min(8000, 1000 * hostReconnectAttempts);
        setStatus(
          `${reason} Retrying ${inMatch ? "match host" : "lobby host"} in ${Math.ceil(delay / 1000)}s...`,
          true
        );
        hostReconnectTimer = window.setTimeout(() => {
          hostReconnectTimer = null;
          if (peerRef.current !== peer || peer.destroyed) {
            return;
          }
          setStatus(inMatch ? "Reconnecting to match host..." : "Reconnecting to lobby host...");
          connectToHost();
        }, delay);
      };

      const connectToHost = () => {
        if (peerRef.current !== peer || peer.destroyed) return;

        const currentConn = hostConnectionRef.current;
        if (currentConn?.open) return;
        if (currentConn) {
          hostConnectionRef.current = null;
          clearConnectionHeartbeat(connectionHeartbeatKey("host", currentConn.peer));
          try {
            currentConn.close();
          } catch (err) {
            void err;
          }
        }

        const hostTarget =
          String(multiplayerRef.current.hostPeerId || targetLobby).trim() || targetLobby;
	        const conn = peer.connect(hostTarget, {
	          reliable: true,
	          serialization: "binary",
	        });
        hostConnectionRef.current = conn;
        const heartbeatKey = connectionHeartbeatKey("host", hostTarget);
        const connOpenTimeout = window.setTimeout(() => {
          if (hostConnectionRef.current !== conn || conn.open) return;
          hostConnectionRef.current = null;
          clearConnectionHeartbeat(heartbeatKey);
          try {
            conn.close();
          } catch (err) {
            void err;
          }
          scheduleHostReconnect(
            "Could not reach the lobby host. If the code is correct, this is usually a WebRTC connectivity issue between the two machines."
          );
        }, PEER_CONNECT_TIMEOUT_MS);
        const clearJoinTimeouts = () => {
          clearTimeout(peerOpenTimeout);
          clearTimeout(connOpenTimeout);
        };
        const handleHostConnectionLost = (reason) => {
          if (hostConnectionRef.current !== conn) return;
          clearJoinTimeouts();
          clearConnectionHeartbeat(heartbeatKey);
          hostConnectionRef.current = null;
          const hostPlayer = multiplayerRef.current.players.find(
            (player) => String(player?.peerId || "") === hostTarget
          );
          rememberLocalDisconnectObservation(hostTarget, {
            playerIndex: hostPlayer?.index,
            disconnectedAtMs: Date.now(),
            source: "host_connection",
          });
          updateMultiplayer((prev) => ({
            ...prev,
            players: markPlayerConnectionState(prev.players, hostTarget, false),
          }));
          if (promoteLocalPlayerToHost(reason)) return;
          scheduleHostReconnect(reason);
        };
        conn.on("open", async () => {
          if (hostConnectionRef.current !== conn) return;
          clearJoinTimeouts();
          clearHostReconnect();
          const hostPlayer = multiplayerRef.current.players.find(
            (player) => String(player?.peerId || "") === hostTarget
          );
          clearLocalDisconnectObservation(hostTarget, hostPlayer?.index);
          startConnectionHeartbeat(heartbeatKey, conn, () => {
            handleHostConnectionLost("Lost heartbeat from lobby host.");
          });
          updateMultiplayer((prev) => ({
            ...prev,
            players: markPlayerConnectionState(prev.players, hostTarget, true),
          }));
	          const session = multiplayerRef.current;
	          if (session.matchStarted) {
	            safeSend(conn, {
              type: "resync_request",
              protocolVersion: PROTOCOL_VERSION,
              lastSequence: session.lastAppliedSequence,
            });
            setStatus(`Reconnected to match host ${hostTarget}`);
            return;
          }

          const currentDeck = parseDeckSubmission(
            session.format,
            session.localDeckText,
            session.localCommanderText
          );
          const requestedPlayerIndex = resolveReconnectPlayerIndex(session, targetLobby);
          const joinRequest = {
            type: "join_request",
            protocolVersion: PROTOCOL_VERSION,
            name: localName,
            securityMode: sessionSecurityMode(session),
            deck: currentDeck.deck,
            sideboard: currentDeck.sideboard,
            deckSlotOpenings: [],
            deckAuditManifest: null,
            ziffleKey: null,
            deckCount: currentDeck.deckCount,
            sideboardCount: currentDeck.sideboard.length,
            commanders: currentDeck.commanders,
            commanderCount: currentDeck.commanderCount,
		            ready: currentDeck.ready,
          };
          if (requestedPlayerIndex != null) {
            joinRequest.requestedPlayerIndex = requestedPlayerIndex;
          }
          safeSend(conn, joinRequest);
          setStatus(`Joined lobby ${targetLobby}`);
        });
        conn.on("iceStateChanged", (state) => {
          if (hostConnectionRef.current !== conn) return;
          if (state === "checking") {
            setStatus("Negotiating direct peer connection...");
            return;
          }
          if (state === "failed") {
            handleHostConnectionLost(
              "Could not establish a direct peer connection to the lobby host. The two machines likely need TURN relay support."
            );
            return;
          }
          if (state === "disconnected") {
            setStatus(
              multiplayerRef.current.matchStarted
                ? "Peer connection interrupted. Attempting to reconnect to the host..."
                : "Peer connection interrupted while joining.",
              true
            );
          }
        });
        conn.on("data", (message) => {
          if (hostConnectionRef.current !== conn) return;
          markConnectionAlive(heartbeatKey);
          if (handleConnectionHeartbeatMessage(conn, message)) return;
          if (message?.type === "apply_action") {
            void enqueueAsync(hostMessageQueueRef, () => handleHostMessage(message)).catch((err) => {
              emitSyncFailureNotice(
                "Sync failed",
                err instanceof Error ? err.message : String(err)
              );
              setStatus(`Lobby message failed: ${err}`, true);
            });
            return;
          }
          if (
            message?.type === "ziffle_shuffle_step_request"
            || message?.type === "ziffle_shuffle_step_response"
            || message?.type === "ziffle_reveal_token_request"
            || message?.type === "ziffle_reveal_token_response"
            || message?.type === "rng_commit_request"
            || message?.type === "rng_commit_response"
            || message?.type === "rng_reveal_request"
            || message?.type === "rng_reveal_response"
            || message?.type === "timeout_vote_request"
            || message?.type === "timeout_vote_response"
            || message?.type === "disconnect_forfeit_vote_request"
            || message?.type === "disconnect_forfeit_vote_response"
            || message?.type === "protocol_timeout_vote_request"
            || message?.type === "protocol_timeout_vote_response"
            || message?.type === "action_quorum_vote_request"
            || message?.type === "action_quorum_vote_response"
            || message?.type === "action_intent_progress"
            || message?.type === "action_intent_cancel"
          ) {
            void handleHostMessage(message).catch((err) => {
              if (shouldSuppressProtocolMessageError(err, message)) return;
              emitZiffleDiagnosticNotice("Ziffle message failed", err, {
                phase: "host_ziffle_message",
                messageType: String(message?.type || ""),
                requestId: String(message?.requestId || ""),
              });
              setStatus(`Ziffle message failed: ${err}`, true);
            });
            return;
          }
          void enqueueAsync(hostMessageQueueRef, () => handleHostMessage(message)).catch((err) => {
            emitSyncFailureNotice(
              "Sync failed",
              err instanceof Error ? err.message : String(err)
            );
            setStatus(`Lobby message failed: ${err}`, true);
          });
        });
        conn.on("close", () => {
          if (hostConnectionRef.current !== conn) return;
          handleHostConnectionLost("Disconnected from lobby host.");
        });
        conn.on("error", (err) => {
          if (hostConnectionRef.current !== conn) return;
          const reason = formatPeerError(err, "Lobby connection failed");
          handleHostConnectionLost(reason);
        });
      };
      const scheduleReconnect = (reason) => {
        if (peerRef.current !== peer || peer.destroyed || reconnectTimer) return;
        reconnectAttempts += 1;
        const delay = Math.min(8000, 1000 * reconnectAttempts);
        setStatus(
          `${reason} Retrying signaling in ${Math.ceil(delay / 1000)}s...`,
          true
        );
        reconnectTimer = window.setTimeout(() => {
          reconnectTimer = null;
          if (peerRef.current !== peer || peer.destroyed) return;
          try {
            peer.reconnect();
          } catch (err) {
            setStatus(formatPeerError(err, "Could not reconnect lobby signaling"), true);
            leaveLobby("");
          }
        }, delay);
      };

      peer.on("open", (peerId) => {
        clearTimeout(peerOpenTimeout);
        clearReconnect();
        clearHostReconnect();
        const previousPeerId = multiplayerRef.current.localPeerId;
        updateMultiplayer((prev) => ({
          ...prev,
          localPeerId: peerId,
        }));
        if (previousPeerId === peerId && hostConnectionRef.current?.open) {
          setStatus(`Lobby signaling reconnected: ${peerId}`);
          return;
        }
        setStatus("Connecting to lobby host...");
        connectToHost();
      });
      peer.on("connection", (conn) => {
        if (conn?.metadata?.channel === "peer-direct") {
          configurePeerConnection(conn);
          return;
        }
        try {
          conn.close();
        } catch (err) {
          void err;
        }
      });
      peer.on("error", (err) => {
        clearTimeout(peerOpenTimeout);
        if (isRecoverablePeerError(err)) {
          scheduleReconnect(formatPeerError(err, "Lost lobby signaling"));
          return;
        }
        const errorMessage = formatPeerError(err, "Lobby error");
        if (String(err?.type || "").trim() === "peer-unavailable") {
          const reason = "Lobby host is not registered on the signaling server yet.";
          if (promoteLocalPlayerToHost(reason)) return;
          scheduleHostReconnect(reason);
          return;
        }
        setStatus(errorMessage, true);
        leaveLobby("");
      });
      peer.on("disconnected", () => {
        clearTimeout(peerOpenTimeout);
        scheduleReconnect(
          `Disconnected from the PeerJS signaling server (${peerServerLabelRef.current}).`
        );
      });
      peer.on("close", () => {
        clearTimeout(peerOpenTimeout);
        clearReconnect();
        clearHostReconnect();
      });
    },
    [
      clearConnectionHeartbeat,
      configurePeerConnection,
      emitZiffleDiagnosticNotice,
      handleHostMessage,
      handleConnectionHeartbeatMessage,
      leaveLobby,
      markConnectionAlive,
      peerOptionsRef,
      promoteLocalPlayerToHost,
      setStatus,
      startConnectionHeartbeat,
      teardownPeer,
      updateMultiplayer,
    ]
  );

  const updateLobbyDeck = useCallback(
    async (updates) => {
      const currentSession = multiplayerRef.current;
      if (currentSession.matchStarted || currentSession.mode === "starting") {
        return;
      }

      const nextDeckText =
        typeof updates === "string"
          ? String(updates)
          : Object.prototype.hasOwnProperty.call(updates || {}, "deckText")
            ? String(updates.deckText || "")
            : currentSession.localDeckText;
      const nextCommanderText =
        typeof updates === "string"
          ? currentSession.localCommanderText
          : Object.prototype.hasOwnProperty.call(updates || {}, "commanderText")
            ? String(updates.commanderText || "")
            : currentSession.localCommanderText;

	    const deckSubmission = parseDeckSubmission(
	        currentSession.format,
	        nextDeckText,
	        nextCommanderText
	      );
	      if (isTrustedMultiplayerSecurityMode(sessionSecurityMode(currentSession))) {
	        const nextSession = updateMultiplayer((prev) => ({
	          ...prev,
	          localDeckText: nextDeckText,
	          localCommanderText: nextCommanderText,
	          localDeckCount: deckSubmission.deckCount,
	          localCommanderCount: deckSubmission.commanderCount,
	          players:
	            prev.role === "host" && prev.localPeerId
	              ? reindexPlayers(
	                  prev.players.map((player) =>
	                    player.peerId === prev.localPeerId
	                      ? withDeckState(
	                          {
	                            ...player,
	                            deckAuditManifest: null,
	                            deckSlotOpenings: [],
	                            ziffleKey: null,
	                            playerGenesisSignature: null,
	                          },
	                          prev.format,
	                          deckSubmission.deck,
	                          deckSubmission.commanders,
	                          deckSubmission.sideboard
	                        )
	                      : player
	                  )
	                )
	              : prev.players,
	        }));
	        if (nextSession.role === "host") {
	          broadcastLobbyState();
	          return;
	        }
	        const conn = hostConnectionRef.current;
	        if (nextSession.role === "client" && conn && conn.open !== false) {
	          safeSend(conn, {
	            type: "deck_update",
	            protocolVersion: PROTOCOL_VERSION,
	            securityMode: MULTIPLAYER_SECURITY_TRUSTED,
	            deckAuditManifest: null,
	            deck: deckSubmission.deck,
	            sideboard: deckSubmission.sideboard,
	            deckSlotOpenings: [],
	            ziffleKey: null,
	            deckCount: deckSubmission.deckCount,
	            sideboardCount: deckSubmission.sideboard.length,
	            commanders: deckSubmission.commanders,
	            commanderCount: deckSubmission.commanderCount,
	            ready: deckSubmission.ready,
	          });
	        }
	        return;
	      }
	      const {
	        publicKey: auditPublicKey,
        encryptionPublicKey: auditEncryptionPublicKey,
      } = await ensureAuditIdentity();
      const resolvedLocalPlayerIndex = resolveLocalPlayerIndex(currentSession);
      const localPlayerIndex = resolvedLocalPlayerIndex ?? 0;
      const deckAuditManifest = await buildLocalDeckAuditManifest({
        matchId: currentSession.lobbyId || currentSession.hostPeerId || "pending",
        owner: localPlayerIndex,
        deck: deckSubmission.deck,
        sideboard: deckSubmission.sideboard,
        commanders: deckSubmission.commanders,
        // Without an assigned seat the owner is a guess; persisting it could
        // shadow the real seat-0 deck in private manifest lookups.
        persist: resolvedLocalPlayerIndex != null,
      });
      const deckSlotOpenings = deckSlotOpeningsForManifest(deckAuditManifest);
      const ziffleKeyPair = await ensureZiffleIdentity({
        context: currentSession.lobbyId || currentSession.hostPeerId || "pending",
        deckCount: deckSubmission.deckCount || 60,
      });
      const localZiffleKey = publicZiffleKey(ziffleKeyPair, localPlayerIndex);
      const localGenesisPlayer = {
        peerId: currentSession.localPeerId,
        name: currentSession.localName,
        index: localPlayerIndex,
        auditPublicKey,
        auditEncryptionPublicKey,
        deckAuditManifest: publicDeckManifest(deckAuditManifest),
        ziffleKey: localZiffleKey,
        ...openDecklistPlayerFields({
          deck: deckSubmission.deck,
          sideboard: deckSubmission.sideboard,
          commanders: deckSubmission.commanders,
          deckSlotOpenings,
        }),
        deckCount: deckSubmission.deckCount,
        sideboardCount: deckSubmission.sideboard.length,
        commanderCount: deckSubmission.commanderCount,
      };
      const playerGenesisSignature = await signPlayerGenesis({
        matchId: currentSession.lobbyId || currentSession.hostPeerId || "pending",
        player: localGenesisPlayer,
      });
      const nextSession = updateMultiplayer((prev) => ({
        ...prev,
        localDeckText: nextDeckText,
        localCommanderText: nextCommanderText,
        localDeckCount: deckSubmission.deckCount,
        localCommanderCount: deckSubmission.commanderCount,
        players:
          prev.role === "host" && prev.localPeerId
            ? reindexPlayers(
                prev.players.map((player) =>
                  player.peerId === prev.localPeerId
                    ? withDeckState(
                        {
	                          ...player,
	                          auditPublicKey,
		                          auditEncryptionPublicKey,
		                          deckAuditManifest: publicDeckManifest(deckAuditManifest),
                          deckSlotOpenings,
	                          ziffleKey: localZiffleKey,
	                          playerGenesisSignature,
                        },
                        prev.format,
                        deckSubmission.deck,
                        deckSubmission.commanders,
                        deckSubmission.sideboard
                      )
                    : player
                )
              )
            : prev.players,
      }));

      if (nextSession.role === "host") {
        broadcastLobbyState();
        return;
      }

      const conn = hostConnectionRef.current;
      if (nextSession.role === "client" && conn && conn.open !== false) {
        safeSend(conn, {
          type: "deck_update",
	          protocolVersion: PROTOCOL_VERSION,
	          auditPublicKey,
	          auditEncryptionPublicKey,
		          playerGenesisSignature,
          deckAuditManifest: publicDeckManifest(deckAuditManifest),
          deck: deckSubmission.deck,
          sideboard: deckSubmission.sideboard,
          deckSlotOpenings,
          ziffleKey: localZiffleKey,
          deckCount: deckSubmission.deckCount,
          sideboardCount: deckSubmission.sideboard.length,
          commanders: deckSubmission.commanders,
          commanderCount: deckSubmission.commanderCount,
	          ready: deckSubmission.ready,
        });
      }
    },
    [
      broadcastLobbyState,
      buildLocalDeckAuditManifest,
      ensureZiffleIdentity,
      ensureAuditIdentity,
      publicZiffleKey,
      signPlayerGenesis,
      updateMultiplayer,
    ]
  );

  async function submitProtocolResponseTimeoutClaim(claim) {
    if (!claim || typeof claim !== "object") return false;
    const targetPlayerIndex = normalizePlayerIndex(claim.targetPlayerIndex);
    if (targetPlayerIndex == null) return false;
    const timedOutActionIntentKey = actionIntentKeyFromProtocolClaim(claim);
    const players = reindexPlayers(matchStartPayloadRef.current?.players || multiplayerRef.current.players || []);
    const roster = protocolResponseTimeoutRoster(targetPlayerIndex);
    const threshold = protocolResponseTimeoutVoteThreshold(roster.length);
    const target = players.find((player) => Number(player.index) === Number(targetPlayerIndex));
    const targetName = String(claim.targetName || target?.name || `Player ${targetPlayerIndex + 1}`);
    if (timedOutActionIntentKey) {
      rememberIgnoredActionIntentKey(timedOutActionIntentKey, "protocol_response_timeout");
      clearPendingActionIntent(timedOutActionIntentKey);
    }
    if (threshold <= 0) {
      markMatchDisputed(
        `Protocol response timeout from ${targetName}; two-player matches require external arbitration.`,
        {
          type: "protocol_response_timeout",
          accusedPlayers: [targetPlayerIndex],
          claim: cloneMultiplayerPayload(claim),
        }
      );
      return true;
    }
    const responseTimeoutMs = Math.max(
      1,
      Math.floor(Number(claim.responseTimeoutMs || PROTOCOL_RESPONSE_TIMEOUT_MS))
    );
    const requestedAtMs = Math.max(1, Math.floor(Number(claim.requestedAtMs || Date.now() - responseTimeoutMs)));
    const command = {
      type: "forfeit_player",
      player: targetPlayerIndex,
      reason: PROTOCOL_RESPONSE_TIMEOUT_REASON,
      timed_out_peer_id: String(claim.targetPeerId || target?.peerId || ""),
      request_type: String(claim.requestType || ""),
      request_id: String(claim.requestId || ""),
      request_payload_hash: String(claim.requestPayloadHash || ""),
      response_timeout_ms: responseTimeoutMs,
      requested_at_ms: requestedAtMs,
      eligible_at_ms: requestedAtMs + responseTimeoutMs,
      claimed_at_ms: Date.now(),
      basis_sequence: Number(claim.basisSequence ?? multiplayerRef.current.lastAppliedSequence ?? 0),
    };
    try {
      await submitMultiplayerCommand(
        command,
        `${targetName} forfeited by protocol response timeout`
      );
    } catch (err) {
      const failureReason = toErrorMessage(err);
      if (
        failureReason.includes("Protocol response timeout certificate")
        || failureReason.includes("Protocol-timeout vote")
        || failureReason.includes("protocol-timeout voter")
      ) {
        markMatchDisputed(
          `Protocol response timeout from ${targetName}; certificate quorum could not be completed.`,
          {
            type: "protocol_response_timeout_certificate_failed",
            accusedPlayers: [targetPlayerIndex],
            claim: cloneMultiplayerPayload(claim),
            command: cloneMultiplayerPayload(command),
            error: failureReason,
          }
        );
        return true;
      }
      throw err;
    }
    return true;
  }

  const submitMultiplayerCommand = useCallback(
    async (command, label = "") => {
      let session = multiplayerRef.current;
      if (!session.matchStarted) {
        setStatus("Match has not started yet", true);
        return;
      }
      if (session.submittingAction) {
        setStatus("Waiting for the previous action to sync");
        await waitForSubmissionIdle();
        session = multiplayerRef.current;
        if (!session.matchStarted) {
          setStatus("Match has not started yet", true);
          return;
        }
        if (session.submittingAction) {
          setStatus("Waiting for the previous action to sync");
          return;
        }
      }
      if (session.localPlayerIndex == null) {
        setStatus("Local player seat is not assigned", true);
        return;
      }
      if (awaitingStateResyncRef.current) {
        setStatus("Waiting for resync to finish");
        return;
      }
      const trustedMode = isTrustedMultiplayerSecurityMode(sessionSecurityMode(session));

      let stagedMatchClockRuntime = null;
      let stopActionIntentProgress = null;
      let localActionWaitId = null;
      let localSubmissionSnapshot = null;
      let localSubmissionCommitted = false;
      let signedActionIntent = null;
      const clearLocalActionWait = () => {
        if (localActionWaitId) {
          clearPeerWait(localActionWaitId);
          localActionWaitId = null;
        }
      };
      const stopLocalActionIntentProgress = () => {
        if (stopActionIntentProgress) {
          stopActionIntentProgress();
          stopActionIntentProgress = null;
        }
      };
      const cancelLocalActionIntent = (reason) => {
        stopLocalActionIntentProgress();
        if (signedActionIntent && !localSubmissionCommitted) {
          rememberIgnoredActionIntentKey(actionIntentKey(signedActionIntent), reason || "local_action_cancelled");
          broadcastActionIntentCancel(signedActionIntent, reason);
        }
      };
      updateMultiplayer((prev) => ({ ...prev, submittingAction: true }));
      try {
        if (localZiffleRevealInFlightRef.current) {
          setStatus("Waiting for hidden-card reveal payloads to finish");
          await localZiffleRevealInFlightRef.current;
        }
        if (resyncingPeerIdsRef.current.size > 0) {
          const waitId = beginPeerWait({
            kind: "peer_resync",
            title: "Waiting for peer resync",
            description:
              "One or more peers are importing the latest checkpoint before another action can be submitted.",
          });
          setStatus("Waiting for peers to finish resyncing");
          try {
            await waitForPeerResyncs();
          } finally {
            clearPeerWait(waitId);
          }
        }
        let preSubmitState = gameRef.current ? await gameRef.current.uiState() : stateRef.current;
        if (!isDecisionCommandCompatible(preSubmitState?.decision, command)) {
          updateMultiplayer((prev) => ({ ...prev, submittingAction: false }));
          setStatus("That action is no longer available");
          return;
        }
        if (command?.type === "priority_action" && command.action_ref) {
          const objectId = Number(command.object_id ?? command.objectId ?? actionRefObjectId(command.action_ref));
          const hasStableId = command.object_stable_id != null || command.objectStableId != null;
          let priorityObjectMetadata = {};
          if (Number.isSafeInteger(objectId) && objectId > 0 && !hasStableId) {
            const stableId = await currentStableIdForObjectId(objectId);
            if (stableId != null) {
              priorityObjectMetadata.object_stable_id = stableId;
            }
          }
          const hasHiddenRef = command.object_hidden_ref != null || command.objectHiddenRef != null;
          if (Number.isSafeInteger(objectId) && objectId > 0 && !hasHiddenRef) {
            const hiddenRef = await currentHiddenRefForObjectId(objectId);
            if (hiddenRef) {
              priorityObjectMetadata.object_hidden_ref = hiddenRef;
            }
          }
          if (Number.isSafeInteger(objectId) && objectId > 0) {
            priorityObjectMetadata.object_id = objectId;
          }
          if (Object.keys(priorityObjectMetadata).length > 0) {
            command = {
              ...command,
              ...priorityObjectMetadata,
            };
          }
        }
        if (command?.type === "select_objects" && Array.isArray(command.object_ids)) {
          const objectIds = command.object_ids.map((objectId) => Number(objectId));
          const { stableIds, hiddenRefs } = selectObjectSyncMetadataForCommand(
            { ...command, object_ids: objectIds },
            preSubmitState
          );
          command = {
            ...command,
            object_ids: objectIds,
          };
          if (
            !Array.isArray(command.object_stable_ids)
            && !Array.isArray(command.objectStableIds)
            && stableIds.some((stableId) => stableId != null)
          ) {
            command.object_stable_ids = stableIds;
          }
          if (
            !Array.isArray(command.object_hidden_refs)
            && !Array.isArray(command.objectHiddenRefs)
            && hiddenRefs.some((hiddenRef) => hiddenRef != null)
          ) {
            command.object_hidden_refs = hiddenRefs;
          }
        }
        const expectedActor = preSubmitState?.decision?.player;
        const isTimeoutForfeit = isActionTimeoutForfeitCommand(command);
        const isDisconnectForfeit = isDisconnectTimeoutForfeitCommand(command);
        const isProtocolTimeoutForfeit = isProtocolResponseTimeoutForfeitCommand(command);
        const isSelfForfeit = isSelfForfeitCommand(command, session.localPlayerIndex);
        if (isSelfForfeit && !isSorcerySpeedForfeitState(preSubmitState, session.localPlayerIndex)) {
          throw new Error("Surrender is only available at sorcery speed");
        }
        if (
          isForfeitCommand(command)
          && !isTimeoutForfeit
          && !isDisconnectForfeit
          && !isProtocolTimeoutForfeit
        ) {
          if (Number(command.player) !== Number(session.localPlayerIndex)) {
            throw new Error("A player can only forfeit themselves");
          }
        }
        if (isTimeoutForfeit) {
          const liveState = gameRef.current ? await gameRef.current.uiState() : stateRef.current;
          updateMatchClockForState(liveState);
          if (!trustedMode && !timeoutCertificateFromCommand(command)) {
            const certificate = await collectTimeoutCertificateForCommand(command);
            if (certificate) {
              command = {
                ...command,
                timeout_certificate: certificate,
              };
            }
          }
          await validateTimeoutForfeitCommand(command, liveState, {
            skewMs: MATCH_CLOCK_CLAIM_SKEW_MS,
            skipCertificate: trustedMode,
          });
        } else if (isDisconnectForfeit) {
          if (!trustedMode && !disconnectCertificateFromCommand(command)) {
            const certificate = await collectDisconnectForfeitCertificateForCommand(command);
            if (certificate) {
              command = {
                ...command,
                disconnected_peer_id: command.disconnected_peer_id || certificate.forfeitedPeerId,
                disconnect_timeout_ms: certificate.disconnectTimeoutMs,
                disconnect_certificate: certificate,
              };
            }
          }
          await validateDisconnectForfeitCommand(command, {
            actorIndex: session.localPlayerIndex,
            skipCertificate: trustedMode,
          });
        } else if (isProtocolTimeoutForfeit) {
          if (
            !trustedMode
            && !command.protocol_timeout_certificate
            && !command.protocolTimeoutCertificate
          ) {
            const certificate = await collectProtocolResponseTimeoutCertificateForCommand(command);
            if (certificate) {
              command = {
                ...command,
                timed_out_peer_id: command.timed_out_peer_id || certificate.forfeitedPeerId,
                response_timeout_ms: certificate.responseTimeoutMs,
                requested_at_ms: certificate.requestedAtMs,
                protocol_timeout_certificate: certificate,
              };
            }
          }
          await validateProtocolResponseTimeoutCommand(command, {
            actorIndex: session.localPlayerIndex,
            skipCertificate: trustedMode,
          });
        } else if (
          expectedActor !== null
          && expectedActor !== undefined
          && Number(expectedActor) !== Number(session.localPlayerIndex)
        ) {
          updateMultiplayer((prev) => ({ ...prev, submittingAction: false }));
          setStatus("It is not your turn to act");
          return;
        }
        const nextSequence = Number(multiplayerRef.current.lastAppliedSequence || 0) + 1;
        if (trustedMode) {
          preSubmitState = gameRef.current ? await gameRef.current.uiState() : stateRef.current;
          const expectedPreviousSequence = nextSequence - 1;
          const latestAppliedSequenceBeforeApply = Number(multiplayerRef.current.lastAppliedSequence || 0);
          const latestHistorySequenceBeforeApply = Number(
            actionHistoryRef.current.at(-1)?.seq ?? latestAppliedSequenceBeforeApply
          );
          if (
            awaitingStateResyncRef.current
            || latestAppliedSequenceBeforeApply !== expectedPreviousSequence
            || latestHistorySequenceBeforeApply !== expectedPreviousSequence
          ) {
            updateMultiplayer((prev) => ({ ...prev, submittingAction: false }));
            setStatus("Another action was broadcast first");
            return;
          }
          const clock = await buildMatchClockAuditForCommand({
            command,
            seq: nextSequence,
            actorIndex: session.localPlayerIndex,
            uiState: preSubmitState,
          });
          await validateTrustedSequencedAction({
            command,
            actorIndex: session.localPlayerIndex,
            seq: nextSequence,
            clock,
            uiState: preSubmitState,
            enforceMatchClockObservationBounds: false,
          });
          localSubmissionSnapshot = await createSequencedActionValidationSnapshot();
          stagedMatchClockRuntime = stageLocalMatchClockAudit(clock);
          const appliedState = await applySyncedCommand(command, label || "", {
            actorIndex: session.localPlayerIndex,
            sequence: nextSequence,
            publishState: false,
          });
          const message = {
            type: "apply_action",
            protocolVersion: PROTOCOL_VERSION,
            securityMode: MULTIPLAYER_SECURITY_TRUSTED,
            seq: nextSequence,
            actorIndex: session.localPlayerIndex,
            command,
            label: label || "",
            clock,
          };
          commitMatchClockAudit(clock, appliedState);
          await appendAppliedSequencedAction(message);
          localSubmissionCommitted = true;
          relaySequencedAction(message);
          await publishCurrentRuntimeState(appliedState);
          await drainPendingSequencedActions();
          setStatus("Action broadcast to trusted peers");
          return;
        }
	        const showLocalActionWait = (patch = {}) => {
          const requestId = `local-action:${currentAuditMatchId()}:${nextSequence}`;
          const localName = playerNameForIndex(
            multiplayerRef.current.players,
            multiplayerRef.current.localPlayerIndex
          );
          const wait = {
            kind: "local_action",
            requestId,
            local: true,
            peerIndex: multiplayerRef.current.localPlayerIndex,
            peerName: localName || "You",
            title: "Working on your action",
            description:
              "Your browser is applying the action locally, producing hidden-card proofs, "
              + "and assembling the signed payload for peers.",
            operation: "Preparing action",
            ...patch,
          };
          if (localActionWaitId === requestId && multiplayerRef.current.peerWait?.requestId === requestId) {
            updatePeerWait(requestId, wait);
          } else {
            localActionWaitId = beginPeerWait(wait);
	          }
        };
        let localOpeningPreviewCount = 0;
        let localOpeningPreviewTotal = 0;
        const localOpeningPreviewKeys = new Set();
        const broadcastLocalActionProgress = (
          phase = "payload_generation",
          responseTimeoutMs = ZIFFLE_REVEAL_TOKEN_TIMEOUT_MS_PER_CARD,
          extraPayload = {}
        ) => {
          if (!signedActionIntent) return;
          if (typeof stopActionIntentProgress?.update === "function") {
            stopActionIntentProgress.update(phase, responseTimeoutMs, extraPayload);
            return;
          }
          broadcastActionIntentProgress(
            signedActionIntent,
            phase,
            responseTimeoutMs,
            extraPayload
          );
        };
        const updateLocalActionProgress = (
          waitPatch = {},
          phase = "payload_generation",
          responseTimeoutMs = ZIFFLE_REVEAL_TOKEN_TIMEOUT_MS_PER_CARD,
          extraPayload = null
        ) => {
          showLocalActionWait(waitPatch);
          broadcastLocalActionProgress(
            phase,
            responseTimeoutMs,
            extraPayload || {
              operation: waitPatch.operation,
              detail: waitPatch.detail,
              cardName: waitPatch.cardName,
              zone: waitPatch.zone,
              progressCurrent: waitPatch.progressCurrent,
              progressTotal: waitPatch.progressTotal,
              title: waitPatch.title,
              description: waitPatch.description,
            }
          );
        };
        const previewBuiltLocalOpening = (opening, metadata = {}) => {
          const preview = actionOpeningPreviewFromOpening(opening, {
            zone: metadata.zone || "exile",
          });
          if (!preview) return;
          const previewKey = [
            preview.owner,
            opening?.slot ?? "",
            preview.objectId ?? "",
            preview.stableId ?? "",
            preview.position ?? "",
            preview.zone || "",
            preview.card || "",
          ].join(":");
          if (localOpeningPreviewKeys.has(previewKey)) return;
          localOpeningPreviewKeys.add(previewKey);
          localOpeningPreviewCount += 1;
          const metadataTotal = Number(metadata.total);
          localOpeningPreviewTotal = Math.max(
            localOpeningPreviewTotal,
            Number.isFinite(metadataTotal) && metadataTotal > 0 ? metadataTotal : 0,
            localOpeningPreviewCount
          );
          const progress = {
            progressCurrent: localOpeningPreviewCount,
            progressTotal: localOpeningPreviewTotal,
          };
          previewAuditOpeningInInspector(preview, stateRef.current, {
            previewIndex: localOpeningPreviewCount - 1,
            previewTotal: localOpeningPreviewTotal,
            previewZone: preview.zone,
          });
          const progressPayload = {
            operation: "Opening revealed card",
            cardName: preview.card,
            zone: preview.zone,
            openingPreview: preview,
            ...progress,
          };
          updateLocalActionProgress(
            {
              kind: "local_ziffle_reveal",
              title: "Opening revealed card",
              operation: "Opening revealed card",
              cardName: preview.card,
              zone: preview.zone,
              ...progress,
            },
            "opening_preview",
            ZIFFLE_REVEAL_TOKEN_TIMEOUT_MS_PER_CARD,
            progressPayload
          );
        };
        const submitPerf = {
          seq: nextSequence,
          actor: session.localPlayerIndex == null ? null : Number(session.localPlayerIndex),
          command: summarizePeerCommand(command),
          decision_kind: String(preSubmitState?.decision?.kind || ""),
          decision_player: preSubmitState?.decision?.player == null
            ? null
            : Number(preSubmitState.decision.player),
          stack_size: Number(preSubmitState?.stack_size || 0),
        };
        recordPeerSyncPerf("submit_action:start", submitPerf);
        const preActionStateHash = String(auditStateHashRef.current || INITIAL_AUDIT_STATE_HASH);
        let preActionPublicCheckpointHash = "";
        const ensurePreActionPublicCheckpointHash = async () => {
          if (!preActionPublicCheckpointHash) {
            preActionPublicCheckpointHash =
              currentKnownPublicAuditCheckpointHash()
              || await currentPublicAuditCheckpointHash();
          }
          return preActionPublicCheckpointHash;
        };
        const ensureSignedActionIntent = async () => {
          if (!signedActionIntent) {
            signedActionIntent = await signActionIntentForCommand({
              seq: nextSequence,
              actorIndex: session.localPlayerIndex,
              command,
              prevStateHash: preActionStateHash,
              preActionPublicCheckpointHash: await ensurePreActionPublicCheckpointHash(),
            });
          }
          return signedActionIntent;
        };
        const pendingIntentCleared = await waitForPendingActionIntentBeforeLocalSubmit(nextSequence);
        if (!pendingIntentCleared) {
          updateMultiplayer((prev) => ({ ...prev, submittingAction: false }));
          setStatus("Another action was signed first");
          return;
        }
        preSubmitState = gameRef.current ? await gameRef.current.uiState() : stateRef.current;
        if (Number(multiplayerRef.current.lastAppliedSequence || 0) !== nextSequence - 1) {
          updateMultiplayer((prev) => ({ ...prev, submittingAction: false }));
          setStatus("Another action was signed first");
          return;
        }
        if (!isDecisionCommandCompatible(preSubmitState?.decision, command)) {
          updateMultiplayer((prev) => ({ ...prev, submittingAction: false }));
          setStatus("That action is no longer available");
          return;
        }
        const refreshedExpectedActor = preSubmitState?.decision?.player;
        if (
          !isForfeitCommand(command)
          && refreshedExpectedActor !== null
          && refreshedExpectedActor !== undefined
          && Number(refreshedExpectedActor) !== Number(session.localPlayerIndex)
        ) {
          updateMultiplayer((prev) => ({ ...prev, submittingAction: false }));
          setStatus("It is not your turn to act");
          return;
        }
        const clock = await buildMatchClockAuditForCommand({
          command,
          seq: nextSequence,
          actorIndex: session.localPlayerIndex,
          uiState: preSubmitState,
        });
	        await verifyMatchClockAuditForAction({
	          clock,
	          command,
	          seq: nextSequence,
	          actorIndex: session.localPlayerIndex,
	          uiState: preSubmitState,
	          skewMs: 0,
	          enforceObservationBounds: false,
	        });
        localSubmissionSnapshot = await createSequencedActionValidationSnapshot();
        stagedMatchClockRuntime = stageLocalMatchClockAudit(clock);
        let cryptoRequirements = await timePeerSyncPhase(
          "submit_action:preview_requirements",
          submitPerf,
          async () => filterCryptoRequirementsForCommand(
            command,
            preSubmitState,
            freshCryptoRequirementsForSequence(
              nextSequence,
              await previewRequirementsForCommand(command)
            )
          )
        );
        recordPeerSyncPerf("submit_action:preview_requirements:summary", {
          ...submitPerf,
          requirements: summarizeCryptoRequirementsForPerf(cryptoRequirements),
        });
        rememberActionCryptoRequirements(nextSequence, cryptoRequirements);
        let requestRemoteCryptoPreview = shouldRequestRemoteCryptoPreview(
          command,
          preSubmitState,
          cryptoRequirements
        );
        const actionMayNeedCryptoMaterial =
          cryptoRequirements.length > 0
          || commandMayProducePostApplyOpenings(command, preSubmitState, cryptoRequirements);
        await ensureSignedActionIntent();
        const initialActionProgress = {
          kind: "local_payload",
          title: "Preparing action payload",
          operation: actionMayNeedCryptoMaterial
            ? "Preparing hidden-card material"
            : "Preparing signed action",
          detail:
            cryptoRequirements.length > 0
              ? `${cryptoRequirements.length} crypto requirement${cryptoRequirements.length === 1 ? "" : "s"}`
              : "No hidden-card material required yet",
          actionIntentKey: actionIntentKey(signedActionIntent),
          progressCurrent: 0,
          progressTotal: Math.max(1, cryptoRequirements.length || 1),
        };
        showLocalActionWait(initialActionProgress);
        stopActionIntentProgress = startActionIntentProgressBroadcast(
          signedActionIntent,
          "payload_generation",
          actionMayNeedCryptoMaterial
            ? ZIFFLE_REVEAL_TOKEN_TIMEOUT_MS_PER_CARD
            : PROTOCOL_RESPONSE_TIMEOUT_MS,
          {
            operation: initialActionProgress.operation,
            detail: initialActionProgress.detail,
            progressCurrent: initialActionProgress.progressCurrent,
            progressTotal: initialActionProgress.progressTotal,
            title: initialActionProgress.title,
          }
        );
        if (cryptoRequirements.length > 0) {
          setStatus("Preparing cryptographic material for action");
        }
        updateLocalActionProgress({
          kind: "local_payload",
          operation: "Building local shuffle proofs",
          detail: `${cryptoRequirements.length} requirement${cryptoRequirements.length === 1 ? "" : "s"}`,
        }, "payload_generation");
        let shuffleProofs = await timePeerSyncPhase(
          "submit_action:build_local_shuffle_proofs",
          {
            ...submitPerf,
            requirements: summarizeCryptoRequirementsForPerf(cryptoRequirements),
          },
          () => buildLocalShuffleProofsForRequirements(
            cryptoRequirements,
            nextSequence
          )
        );
        recordPeerSyncPerf("submit_action:build_local_shuffle_proofs:summary", {
          ...submitPerf,
          shuffle_proofs: Array.isArray(shuffleProofs) ? shuffleProofs.length : 0,
          bytes: payloadSizeBytes(shuffleProofs),
        });
        updateLocalActionProgress({
          kind: "local_payload",
          operation: "Building random reveals",
          detail: `${cryptoRequirements.length} requirement${cryptoRequirements.length === 1 ? "" : "s"}`,
        }, "payload_generation");
        const rngReveals = await timePeerSyncPhase(
          "submit_action:build_local_rng_reveals",
          {
            ...submitPerf,
            requirements: summarizeCryptoRequirementsForPerf(cryptoRequirements),
          },
          () => buildLocalRngRevealsForRequirements(
            cryptoRequirements,
            nextSequence,
            {
              command,
              actorIndex: session.localPlayerIndex,
              prevStateHash: preActionStateHash,
              publicCheckpointHash: preActionPublicCheckpointHash,
              actionIntent: signedActionIntent,
            }
          )
        );
        recordPeerSyncPerf("submit_action:build_local_rng_reveals:summary", {
          ...submitPerf,
          rng_reveals: Array.isArray(rngReveals) ? rngReveals.length : 0,
          bytes: payloadSizeBytes(rngReveals),
        });
        let actionCryptoOptions = {
          command,
          seq: nextSequence,
          actorIndex: session.localPlayerIndex,
          prevStateHash: preActionStateHash,
          publicCheckpointHash: preActionPublicCheckpointHash,
          preActionPublicCheckpointHash,
          actionIntent: signedActionIntent,
          requirements: cryptoRequirements,
          uiState: preSubmitState,
          updateState: false,
        };
        await timePeerSyncPhase(
          "submit_action:inject_crypto_material",
          {
            ...submitPerf,
            requirements: summarizeCryptoRequirementsForPerf(cryptoRequirements),
            shuffle_proofs: Array.isArray(shuffleProofs) ? shuffleProofs.length : 0,
            rng_reveals: Array.isArray(rngReveals) ? rngReveals.length : 0,
          },
          () => injectCryptoMaterialForRequirements(cryptoRequirements, {
            shuffleProofs,
            rngReveals,
          }, actionCryptoOptions)
        );
        if (shuffleProofs.length > 0) {
          cryptoRequirements = await timePeerSyncPhase(
            "submit_action:refresh_requirements_after_shuffle",
            submitPerf,
            async () => filterCryptoRequirementsForCommand(
              command,
              preSubmitState,
              freshCryptoRequirementsForSequence(
                nextSequence,
                await previewRequirementsForCommand(command)
              )
            )
          );
          recordPeerSyncPerf("submit_action:refresh_requirements_after_shuffle:summary", {
            ...submitPerf,
            requirements: summarizeCryptoRequirementsForPerf(cryptoRequirements),
          });
          rememberActionCryptoRequirements(nextSequence, cryptoRequirements);
          shuffleProofs = alignShuffleProofsWithRequirements(shuffleProofs, cryptoRequirements);
          requestRemoteCryptoPreview = shouldRequestRemoteCryptoPreview(
            command,
            preSubmitState,
            cryptoRequirements
          );
          actionCryptoOptions = {
            ...actionCryptoOptions,
            requirements: cryptoRequirements,
          };
        }
        let remoteCryptoMaterial = await timePeerSyncPhase(
          "submit_action:collect_remote_crypto_material_pre",
          {
            ...submitPerf,
            requirements: summarizeCryptoRequirementsForPerf(cryptoRequirements),
            request_preview: requestRemoteCryptoPreview,
          },
          () => collectRemoteCryptoMaterialForRequirements(
            cryptoRequirements,
            {
              command,
              seq: nextSequence,
              actorIndex: session.localPlayerIndex,
              requestPreview: requestRemoteCryptoPreview,
              prevStateHash: preActionStateHash,
              publicCheckpointHash: preActionPublicCheckpointHash,
              actionIntent: signedActionIntent,
            }
          )
        );
        updateLocalActionProgress({
          kind: "local_payload",
          operation: "Opening remote hidden-card material",
          detail: `${Array.isArray(remoteCryptoMaterial.openings) ? remoteCryptoMaterial.openings.length : 0} opening${Array.isArray(remoteCryptoMaterial.openings) && remoteCryptoMaterial.openings.length === 1 ? "" : "s"}`,
        }, "crypto_material");
        recordPeerSyncPerf("submit_action:collect_remote_crypto_material_pre:summary", {
          ...submitPerf,
          material: summarizeCryptoMaterialForPerf(remoteCryptoMaterial),
        });
        await timePeerSyncPhase(
          "submit_action:reveal_remote_openings_pre",
          {
            ...submitPerf,
            openings: Array.isArray(remoteCryptoMaterial.openings) ? remoteCryptoMaterial.openings.length : 0,
          },
          () => revealAuditOpenings(remoteCryptoMaterial.openings || [], {
            timing: "pre",
            shuffleProofs,
            updateState: false,
          })
        );
        const preOpenings = await timePeerSyncPhase(
          "submit_action:build_local_openings_pre",
          {
            ...submitPerf,
            requirements: summarizeCryptoRequirementsForPerf(cryptoRequirements),
          },
          () => buildLocalOpeningsForCommand(command, cryptoRequirements, {
            ...actionCryptoOptions,
            onOpeningBuilt: previewBuiltLocalOpening,
          })
        );
        recordPeerSyncPerf("submit_action:build_local_openings_pre:summary", {
          ...submitPerf,
          openings: Array.isArray(preOpenings) ? preOpenings.length : 0,
          bytes: payloadSizeBytes(preOpenings),
        });
        const publishAppliedStateImmediately = false;
        const expectedPreviousSequence = nextSequence - 1;
        const latestAppliedSequenceBeforeApply = Number(multiplayerRef.current.lastAppliedSequence || 0);
        const latestHistorySequenceBeforeApply = Number(
          actionHistoryRef.current.at(-1)?.seq ?? latestAppliedSequenceBeforeApply
        );
        if (
          awaitingStateResyncRef.current
          || latestAppliedSequenceBeforeApply !== expectedPreviousSequence
          || latestHistorySequenceBeforeApply !== expectedPreviousSequence
        ) {
          cancelLocalActionIntent("Local action was superseded before local apply");
          if (localSubmissionSnapshot) {
            await restoreSequencedActionValidationSnapshotIfCurrent(localSubmissionSnapshot);
          } else if (stagedMatchClockRuntime) {
            restoreMatchClockRuntime(stagedMatchClockRuntime, stateRef.current);
            stagedMatchClockRuntime = null;
          }
          updateMultiplayer((prev) => ({ ...prev, submittingAction: false }));
          clearLocalActionWait();
          setStatus("Another action was signed first");
          return;
        }
        const liveStateBeforeApply = gameRef.current ? await gameRef.current.uiState() : stateRef.current;
        if (!isDecisionCommandCompatible(liveStateBeforeApply?.decision, command)) {
          cancelLocalActionIntent("Action is no longer available before local apply");
          if (localSubmissionSnapshot) {
            await restoreSequencedActionValidationSnapshotIfCurrent(localSubmissionSnapshot);
          } else if (stagedMatchClockRuntime) {
            restoreMatchClockRuntime(stagedMatchClockRuntime, stateRef.current);
            stagedMatchClockRuntime = null;
          }
          updateMultiplayer((prev) => ({ ...prev, submittingAction: false }));
          clearLocalActionWait();
          setStatus("That action is no longer available");
          return;
        }
        const expectedActorBeforeApply = liveStateBeforeApply?.decision?.player;
        if (
          expectedActorBeforeApply !== null
          && expectedActorBeforeApply !== undefined
          && Number(expectedActorBeforeApply) !== Number(session.localPlayerIndex)
        ) {
          cancelLocalActionIntent("Action actor no longer has priority before local apply");
          if (localSubmissionSnapshot) {
            await restoreSequencedActionValidationSnapshotIfCurrent(localSubmissionSnapshot);
          } else if (stagedMatchClockRuntime) {
            restoreMatchClockRuntime(stagedMatchClockRuntime, stateRef.current);
            stagedMatchClockRuntime = null;
          }
          updateMultiplayer((prev) => ({ ...prev, submittingAction: false }));
          clearLocalActionWait();
          setStatus("It is not your turn to act");
          return;
        }
        updateLocalActionProgress({
          kind: "engine_work",
          title: "Resolving action locally",
          operation: "Engine applying command",
          detail: label || summarizePeerCommand(command)?.type || String(command?.type || "action"),
        }, "engine_work", PROTOCOL_RESPONSE_TIMEOUT_MS);
        setStatus("Engine is applying the action locally");
        let appliedState = await timePeerSyncPhase(
          "submit_action:apply_synced_command",
          {
            ...submitPerf,
            publish_state: publishAppliedStateImmediately,
          },
          () => applySyncedCommand(command, label || "", {
            actorIndex: session.localPlayerIndex,
            sequence: nextSequence,
            publishState: publishAppliedStateImmediately,
          })
        );
        const remotePostOpeningState = await timePeerSyncPhase(
          "submit_action:reveal_remote_openings_post",
          {
            ...submitPerf,
            openings: Array.isArray(remoteCryptoMaterial.openings) ? remoteCryptoMaterial.openings.length : 0,
          },
          () => revealAuditOpenings(
            remoteCryptoMaterial.openings || [],
            {
              timing: "post",
              shuffleProofs,
              updateState: false,
            }
          )
        );
        if (remotePostOpeningState) {
          appliedState = remotePostOpeningState;
        }
        const appliedRequirements = await timePeerSyncPhase(
          "submit_action:applied_requirements_from_state",
          submitPerf,
          async () => filterCryptoRequirementsForCommand(
            command,
            preSubmitState,
            freshCryptoRequirementsForSequence(
              nextSequence,
              cryptoRequirementsFromState(appliedState)
            )
          )
        );
        recordPeerSyncPerf("submit_action:applied_requirements_from_state:summary", {
          ...submitPerf,
          requirements: summarizeCryptoRequirementsForPerf(appliedRequirements),
        });
        rememberActionCryptoRequirements(nextSequence, appliedRequirements);
        if (shuffleProofs.length > 0) {
          shuffleProofs = alignShuffleProofsWithRequirements(
            shuffleProofs,
            appliedRequirements.length > 0 ? appliedRequirements : cryptoRequirements
          );
        }
        const postShuffleRequirements = missingShuffleRequirements(
          appliedRequirements,
          shuffleProofs
        );
        if (postShuffleRequirements.length > 0) {
          const postShuffleProofs = await timePeerSyncPhase(
            "submit_action:build_local_post_shuffle_proofs",
            {
              ...submitPerf,
              requirements: summarizeCryptoRequirementsForPerf(postShuffleRequirements),
            },
            () => buildLocalShuffleProofsForRequirements(
              postShuffleRequirements,
              nextSequence
            )
          );
          shuffleProofs = mergeShuffleProofs(shuffleProofs, postShuffleProofs);
        }
        await timePeerSyncPhase(
          "submit_action:verify_shuffle_proofs",
          {
            ...submitPerf,
            requirements: summarizeCryptoRequirementsForPerf([...cryptoRequirements, ...appliedRequirements]),
            shuffle_proofs: Array.isArray(shuffleProofs) ? shuffleProofs.length : 0,
          },
          () => verifyShuffleProofsForRequirements(
            [...cryptoRequirements, ...appliedRequirements],
            shuffleProofs
          )
        );
        const shuffleApplicationRequirements = [
          ...appliedRequirements,
          ...cryptoRequirements,
        ];
        const localizedShuffleProofs = alignShuffleProofsWithRequirements(
          shuffleProofs,
          shuffleApplicationRequirements
        );
        await timePeerSyncPhase(
          "submit_action:apply_verified_shuffle_proofs",
          {
            ...submitPerf,
            shuffle_proofs: Array.isArray(localizedShuffleProofs) ? localizedShuffleProofs.length : 0,
          },
          () => applyVerifiedShuffleProofs(localizedShuffleProofs)
        );
        await timePeerSyncPhase(
          "submit_action:reveal_local_ziffle_hand",
          {
            ...submitPerf,
            requirements: summarizeCryptoRequirementsForPerf([...cryptoRequirements, ...appliedRequirements]),
          },
          () => revealLocalZiffleHand(matchStartPayloadRef.current, {
            skipIfHandUnchanged: true,
            stateHint: appliedState,
            command,
            seq: nextSequence,
            actorIndex: session.localPlayerIndex,
            actionIntent: signedActionIntent,
            requirements: [...cryptoRequirements, ...appliedRequirements],
            updateState: false,
          })
        );
        const openingRequirements = appliedRequirements.length > 0
          ? [...cryptoRequirements, ...appliedRequirements]
          : cryptoRequirements;
        const missingRemotePostOpenRequirements = missingRemotePublicOpenRequirements(
          openingRequirements,
          remoteCryptoMaterial,
          session.localPlayerIndex
        );
        recordPeerSyncPerf("submit_action:missing_remote_post_open_requirements", {
          ...submitPerf,
          requirements: summarizeCryptoRequirementsForPerf(missingRemotePostOpenRequirements),
        });
        if (missingRemotePostOpenRequirements.length > 0) {
          updateLocalActionProgress({
            kind: "crypto_material",
            title: "Requesting post-action openings",
            operation: "Waiting for peer opening material",
            detail:
              `${missingRemotePostOpenRequirements.length} requirement`
              + `${missingRemotePostOpenRequirements.length === 1 ? "" : "s"}`,
          }, "crypto_material");
          const postRemoteCryptoMaterial = await timePeerSyncPhase(
            "submit_action:collect_remote_crypto_material_post",
            {
              ...submitPerf,
              requirements: summarizeCryptoRequirementsForPerf(missingRemotePostOpenRequirements),
            },
            async () => collectRemoteCryptoMaterialForRequirements(
              missingRemotePostOpenRequirements,
              {
                command,
                seq: nextSequence,
                actorIndex: session.localPlayerIndex,
                prevStateHash: preActionStateHash,
                publicCheckpointHash: await ensurePreActionPublicCheckpointHash(),
                actionIntent: await ensureSignedActionIntent(),
              }
            )
          );
          remoteCryptoMaterial = {
            openings: mergeAuditOpenings(
              remoteCryptoMaterial.openings,
              postRemoteCryptoMaterial.openings
            ),
            privateViewProofs: mergePrivateViewProofs(
              remoteCryptoMaterial.privateViewProofs,
              postRemoteCryptoMaterial.privateViewProofs
            ),
          };
          const postRemoteOpeningState = await timePeerSyncPhase(
            "submit_action:reveal_remote_openings_post_missing",
            {
              ...submitPerf,
              openings: Array.isArray(postRemoteCryptoMaterial.openings)
                ? postRemoteCryptoMaterial.openings.length
                : 0,
            },
            () => revealAuditOpenings(postRemoteCryptoMaterial.openings || [], {
              timing: "post",
              shuffleProofs,
              updateState: false,
            })
          );
          if (postRemoteOpeningState) {
            appliedState = postRemoteOpeningState;
          }
        }
        const expectedOpeningPreviewTotal = expectedLocalPublicOpeningPreviewCount(
          openingRequirements,
          session.localPlayerIndex
        );
        localOpeningPreviewTotal = Math.max(
          localOpeningPreviewTotal,
          expectedOpeningPreviewTotal
        );
        updateLocalActionProgress({
          kind: "local_ziffle_reveal",
          title: "Generating reveal openings",
          operation: "Building local public openings",
          detail:
            `${openingRequirements.length} requirement`
            + `${openingRequirements.length === 1 ? "" : "s"}`,
          progressCurrent: 0,
          progressTotal: expectedOpeningPreviewTotal || null,
        }, "opening_generation");
        const postOpenings = await timePeerSyncPhase(
          "submit_action:build_local_openings_post",
          {
            ...submitPerf,
            requirements: summarizeCryptoRequirementsForPerf(openingRequirements),
          },
	          () => buildLocalOpeningsForCommand(command, openingRequirements, {
	            ...actionCryptoOptions,
	            requirements: openingRequirements,
	            timing: "post",
	            forceZiffleOpeningProof: true,
	            onOpeningBuilt: previewBuiltLocalOpening,
	          })
	        );
        recordPeerSyncPerf("submit_action:build_local_openings_post:summary", {
          ...submitPerf,
          openings: Array.isArray(postOpenings) ? postOpenings.length : 0,
          bytes: payloadSizeBytes(postOpenings),
        });
        const localRequirementOpenings = await timePeerSyncPhase(
          "submit_action:build_local_requirement_openings",
          {
            ...submitPerf,
            requirements: summarizeCryptoRequirementsForPerf(openingRequirements),
          },
	          () => buildLocalRequirementOpeningsForRequirements(
	            openingRequirements,
	            {
	              ...actionCryptoOptions,
	              requirements: openingRequirements,
	              forceZiffleOpeningProof: true,
	              onOpeningBuilt: previewBuiltLocalOpening,
	            }
	          )
        );
        recordPeerSyncPerf("submit_action:build_local_requirement_openings:summary", {
          ...submitPerf,
          openings: Array.isArray(localRequirementOpenings) ? localRequirementOpenings.length : 0,
          bytes: payloadSizeBytes(localRequirementOpenings),
        });
        const selectedPostOpenings = filterOpeningsForCommandHiddenRefs(postOpenings, command);
        const selectedLocalRequirementOpenings = filterOpeningsForCommandHiddenRefs(
          localRequirementOpenings,
          command,
        );
        const localPostOpeningState = await timePeerSyncPhase(
          "submit_action:reveal_local_openings_post",
          {
            ...submitPerf,
            openings: Array.isArray(selectedPostOpenings) ? selectedPostOpenings.length : 0,
            requirement_openings: Array.isArray(selectedLocalRequirementOpenings)
              ? selectedLocalRequirementOpenings.length
              : 0,
          },
          () => revealAuditOpenings(
            mergeAuditOpenings(selectedPostOpenings, selectedLocalRequirementOpenings),
            {
              timing: "post",
              shuffleProofs,
              updateState: false,
            }
          )
        );
        if (localPostOpeningState) {
          appliedState = localPostOpeningState;
        }
        const openings = mergeAuditOpenings(
          preOpenings,
          selectedPostOpenings,
          selectedLocalRequirementOpenings,
          remoteCryptoMaterial.openings
        );
        const localPrivateViewProofs = await timePeerSyncPhase(
          "submit_action:build_local_private_view_proofs",
          {
            ...submitPerf,
            requirements: summarizeCryptoRequirementsForPerf(
              appliedRequirements.length > 0 ? appliedRequirements : cryptoRequirements
            ),
          },
	          () => buildLocalPrivateViewProofsForRequirements(
	            appliedRequirements.length > 0 ? appliedRequirements : cryptoRequirements,
	            {
	              ...actionCryptoOptions,
	              liveState: appliedState,
	              uiState: appliedState,
	            }
	          )
	        );
        const privateViewProofs = mergePrivateViewProofs(
          localPrivateViewProofs,
          remoteCryptoMaterial.privateViewProofs
        );
        await timePeerSyncPhase(
          "submit_action:reveal_private_audit_proofs",
          {
            ...submitPerf,
            private_view_proofs: Array.isArray(privateViewProofs) ? privateViewProofs.length : 0,
          },
          () => revealPrivateAuditProofsForLocalViewer({ seq: nextSequence, privateViewProofs }, {
            seq: nextSequence,
            updateState: false,
          })
        );
        const localPublicCheckpointHash = await timePeerSyncPhase(
          "submit_action:current_public_checkpoint_hash",
          submitPerf,
          () => currentPublicAuditCheckpointHash()
        );
        updateLocalActionProgress({
          kind: "local_payload",
          title: "Signing action payload",
          operation: "Building audit payload",
          detail:
            `${Array.isArray(openings) ? openings.length : 0} opening`
            + `${Array.isArray(openings) && openings.length === 1 ? "" : "s"}`,
          progressCurrent: 1,
          progressTotal: 1,
        }, "payload_signing", PROTOCOL_RESPONSE_TIMEOUT_MS);
        setStatus("Generating action verification payload");
        const audit = await timePeerSyncPhase(
          "submit_action:build_sequenced_action_audit",
          {
            ...submitPerf,
            openings: Array.isArray(openings) ? openings.length : 0,
            private_view_proofs: Array.isArray(privateViewProofs) ? privateViewProofs.length : 0,
            shuffle_proofs: Array.isArray(shuffleProofs) ? shuffleProofs.length : 0,
            rng_reveals: Array.isArray(rngReveals) ? rngReveals.length : 0,
            openings_bytes: payloadSizeBytes(openings),
          },
          () => buildSequencedActionAudit({
            seq: nextSequence,
            actorIndex: session.localPlayerIndex,
            command,
            clock,
            openings,
            rngReveals,
            shuffleProofs,
            privateViewProofs,
            publicCheckpointHash: localPublicCheckpointHash,
          })
        );
        const message = {
          type: "apply_action",
          protocolVersion: PROTOCOL_VERSION,
          securityMode: MULTIPLAYER_SECURITY_VERIFIED,
          seq: nextSequence,
          actorIndex: session.localPlayerIndex,
          command,
          label: label || "",
          audit,
        };
        recordPeerSyncPerf("submit_action:message_summary", summarizeSequencedActionForPerf(message));
        await timePeerSyncPhase(
          "submit_action:verify_sequenced_action_audit",
          summarizeSequencedActionForPerf(message),
          () => verifySequencedActionAudit({
            audit,
            seq: nextSequence,
            actorIndex: session.localPlayerIndex,
            command,
          })
        );
        await timePeerSyncPhase(
          "submit_action:verify_audit_satisfies_crypto_requirements",
          {
            ...summarizeSequencedActionForPerf(message),
            requirements: summarizeCryptoRequirementsForPerf(
              appliedRequirements.length > 0 ? appliedRequirements : cryptoRequirements
            ),
          },
          () => verifyAuditSatisfiesCryptoRequirements({
            requirements: appliedRequirements.length > 0 ? appliedRequirements : cryptoRequirements,
            audit,
          })
        );
        if (String(audit.publicCheckpointHash || "") !== localPublicCheckpointHash) {
          throw new Error("Local public checkpoint hash does not match signed action");
        }
        clearLocalActionWait();
        setStatus("Waiting for peers to verify action payload");
        const quorumCertificate = await timePeerSyncPhase(
          "submit_action:collect_action_quorum_certificate",
          summarizeSequencedActionForPerf(message),
          () => collectActionQuorumCertificate(message)
        );
        if (quorumCertificate) {
          message.audit = {
            ...message.audit,
            quorumCertificate,
          };
          await verifyActionQuorumForMessage(message);
        }
        commitMatchClockAudit(clock, appliedState);
        await appendAppliedSequencedAction(message);
        localSubmissionCommitted = true;
        if (signedActionIntent) {
          broadcastActionIntentProgress(
            signedActionIntent,
            "action_broadcast",
            actionBroadcastResponseTimeoutMs(message),
            { action: message }
          );
        }
        relaySequencedAction(message);
        recordPeerSyncPerf("submit_action:done", summarizeSequencedActionForPerf(message));
        stopLocalActionIntentProgress();
        await publishCurrentRuntimeState(
          viewedCardsStateHint(localPostOpeningState, remotePostOpeningState, appliedState)
        );
        await drainPendingSequencedActions();
        clearLocalActionWait();
        setStatus("Action signed and broadcast to peers");
      } catch (err) {
        stopLocalActionIntentProgress();
        clearLocalActionWait();
        let restoredLocalSubmissionSnapshot = false;
        if (localSubmissionSnapshot && !localSubmissionCommitted) {
          try {
            restoredLocalSubmissionSnapshot =
              await restoreSequencedActionValidationSnapshotIfCurrent(localSubmissionSnapshot);
          } catch {
            // Preserve the original submit failure below.
          }
        }
        if (
          stagedMatchClockRuntime
          && !localSubmissionSnapshot
          && !restoredLocalSubmissionSnapshot
          && !localSubmissionCommitted
        ) {
          restoreMatchClockRuntime(stagedMatchClockRuntime, stateRef.current);
        }
        if (!restoredLocalSubmissionSnapshot) {
          updateMultiplayer((prev) => ({ ...prev, submittingAction: false }));
        }
        const failureReason = toErrorMessage(err);
        recordPeerSyncPerf("submit_action:error", {
          command: summarizePeerCommand(command),
          error: failureReason,
        });
        if (signedActionIntent && !localSubmissionCommitted) {
          broadcastActionIntentCancel(signedActionIntent, failureReason);
        }
        const protocolTimeoutClaim = protocolResponseTimeoutClaimFromError(err);
        if (protocolTimeoutClaim) {
          await submitProtocolResponseTimeoutClaim(protocolTimeoutClaim);
          return;
        }
        if (
          failureReason.includes("Action quorum certificate")
          || failureReason.includes("action quorum vote")
        ) {
          setStatus(`Action rejected by quorum: ${failureReason}`, true);
          return;
        }
        throw err;
      }
    },
    [
      buildLocalOpeningsForCommand,
      buildLocalPrivateViewProofsForRequirements,
      buildLocalRequirementOpeningsForRequirements,
      buildLocalRngRevealsForRequirements,
      collectRemoteCryptoMaterialForRequirements,
      buildSequencedActionAudit,
      currentPublicAuditCheckpointHash,
      revealAuditOpenings,
      verifySequencedActionAudit,
      verifyAuditSatisfiesCryptoRequirements,
      setStatus,
      updateMultiplayer,
      beginPeerWait,
      clearPeerWait,
      waitForPeerResyncs,
      waitForSubmissionIdle,
    ]
  );

  const submitMultiplayerAddCardCheat = useCallback(
    async ({ playerIndex, cardName, zone = "hand", skipTriggers = false } = {}) => {
      const session = multiplayerRef.current;
      if (!session.matchStarted) {
        setStatus("Match has not started yet", true);
        return;
      }
      if (session.localPlayerIndex == null) {
        setStatus("Local player seat is not assigned", true);
        return;
      }
      if (isTrustedMultiplayerSecurityMode(sessionSecurityMode(session))) {
        setStatus("Add-card audit cheat is only available in verified matches", true);
        return;
      }
      const name = String(cardName || "").trim();
      if (!name) {
        setStatus("Enter a card name to add", true);
        return;
      }
      const actorIndex = Number(session.localPlayerIndex);
      const nextSequence = Number(session.lastAppliedSequence || 0) + 1;
      const command = {
        type: "add_card_to_zone",
        player: Number(playerIndex ?? actorIndex),
        cardName: name,
        zone: String(zone || "hand"),
        skipTriggers: Boolean(skipTriggers),
        unauthorized: true,
      };
      const audit = await buildSequencedActionAudit({
        seq: nextSequence,
        actorIndex,
        command,
      });
      const message = {
        type: "apply_action",
        protocolVersion: PROTOCOL_VERSION,
        seq: nextSequence,
        actorIndex,
        command,
        label: `Unauthorized add ${name}`,
        audit,
      };
      await verifySequencedActionAudit({
        audit,
        seq: nextSequence,
        actorIndex,
        command,
      });
      relaySequencedAction(message);
      setStatus(`Local add-card cheat broadcast for audit: ${name}`);
    },
    [
      buildSequencedActionAudit,
      setStatus,
      verifySequencedActionAudit,
    ]
  );

  useEffect(() => {
    if (!multiplayer.matchStarted || !matchClockConfigRef.current.initialMs) {
      const idleSnapshot = createMatchClockSnapshot({
        policy: matchClockConfigRef.current,
      });
      matchClockRef.current = {
        policy: matchClockConfigRef.current,
        playerCount: 0,
        baseRemainingMsByPlayer: [],
        activePlayerIndex: null,
        epochStartedAtMs: null,
        clockHash: INITIAL_MATCH_CLOCK_HASH,
        lastSequence: 0,
      };
      timeoutClaimInFlightRef.current = "";
      updateMultiplayer((prev) => {
        const current = prev.matchClock || {};
        if (
          Boolean(current.enabled) === Boolean(idleSnapshot.enabled)
          && Number(current.initialMs || 0) === Number(idleSnapshot.initialMs || 0)
          && current.activePlayerIndex == null
          && current.startedAtMs == null
        ) {
          return prev;
        }
        return {
          ...prev,
          matchClock: idleSnapshot,
          actionTimer: actionTimerSnapshotFromMatchClock(idleSnapshot),
        };
      });
      return undefined;
    }

    let disposed = false;
    const tick = async () => {
      if (disposed) return;
      const session = multiplayerRef.current;
      if (!session.matchStarted || awaitingStateResyncRef.current) return;

      let liveState = stateRef.current;
      try {
        if (gameRef.current && typeof gameRef.current.uiState === "function") {
          liveState = await gameRef.current.uiState();
        }
      } catch {
        liveState = stateRef.current;
      }

      const timer = updateMatchClockForState(liveState);
      if (
        !timer.enabled
        || !timer.expired
        || timer.activePlayerIndex == null
        || session.submittingAction
        || session.localPlayerIndex == null
        || resyncingPeerIdsRef.current.size > 0
      ) {
        return;
      }

      const claimKey = [
        session.lastAppliedSequence,
        timer.activePlayerIndex,
        timer.clockHash,
      ].join(":");
      if (timeoutClaimInFlightRef.current === claimKey) return;
      if (Number(timer.remainingMs ?? 0) > Number(timer.graceMs || 0) + MATCH_CLOCK_CLAIM_SKEW_MS) return;

      timeoutClaimInFlightRef.current = claimKey;
      const playerName = playerNameForIndex(session.players, timer.activePlayerIndex);
      const command = {
        type: "forfeit_player",
        player: Number(timer.activePlayerIndex),
        reason: "peer_claimed_match_clock_timeout",
        timeout_ms: Number(timer.initialMs),
        match_clock_hash: String(timer.clockHash || ""),
        remaining_ms: Number(timer.remainingMs || 0),
        claimed_at_ms: Date.now(),
        basis_sequence: Number(session.lastAppliedSequence || 0),
      };
      try {
        await submitMultiplayerCommand(
          command,
          `${playerName} was peer-claimed inactive after their match clock expired`
        );
      } catch (err) {
        timeoutClaimInFlightRef.current = "";
        const message = err instanceof Error ? err.message : String(err);
        if (!message.includes("Match clock has not expired")) {
          setStatus(`Timeout forfeit failed: ${message}`, true);
          console.error(err);
        }
      }
    };

    void tick();
    const timerId = window.setInterval(() => {
      void tick();
    }, MATCH_CLOCK_TICK_MS);
    return () => {
      disposed = true;
      window.clearInterval(timerId);
    };
  }, [
    multiplayer.matchStarted,
    setStatus,
    submitMultiplayerCommand,
    updateMultiplayer,
  ]);

  useEffect(() => {
    if (!multiplayer.matchStarted) {
      disconnectForfeitInFlightRef.current.clear();
      return undefined;
    }

    let disposed = false;
    const tick = async () => {
      if (disposed) return;
      updateMultiplayer((prev) => {
        const hasDisconnectedPlayers = (prev.players || []).some((player) =>
          player.connected === false && player.disconnectedAtMs
        );
        if (!hasDisconnectedPlayers) return prev;
        const nowMs = Date.now();
        let changed = false;
        const players = (prev.players || []).map((player) => {
          if (player.connected !== false || !player.disconnectedAtMs) return player;
          const autoForfeitAtMs = Number(player.autoForfeitAtMs || 0)
            || Number(player.disconnectedAtMs) + DISCONNECT_AUTO_FORFEIT_MS;
          const disconnectRemainingMs = Math.max(0, autoForfeitAtMs - nowMs);
          if (
            Number(player.autoForfeitAtMs || 0) === autoForfeitAtMs
            && Math.abs(Number(player.disconnectRemainingMs ?? disconnectRemainingMs) - disconnectRemainingMs) < 250
          ) {
            return player;
          }
          changed = true;
          return {
            ...player,
            autoForfeitAtMs,
            disconnectRemainingMs,
          };
        });
        if (!changed) return prev;
        return {
          ...prev,
          players,
        };
      });

      const session = multiplayerRef.current;
      if (
        !session.matchStarted
        || session.submittingAction
        || session.localPlayerIndex == null
        || awaitingStateResyncRef.current
        || resyncingPeerIdsRef.current.size > 0
      ) {
        return;
      }

      const alreadyForfeited = forfeitedPlayersForQuorum();
      const expired = (session.connectionWarnings || [])
        .filter((warning) =>
          !warning.local
          && warning.expired
          && !alreadyForfeited.has(Number(warning.playerIndex))
        )
        .sort((left, right) => Number(left.playerIndex) - Number(right.playerIndex));
      if (expired.length === 0) return;

      const warning = expired[0];
      const claimKey = [
        session.lastAppliedSequence,
        warning.playerIndex,
        warning.disconnectedAtMs,
      ].join(":");
      if (disconnectForfeitInFlightRef.current.has(claimKey)) return;
      disconnectForfeitInFlightRef.current.add(claimKey);
      const playerName = playerNameForIndex(session.players, warning.playerIndex);
      const command = {
        type: "forfeit_player",
        player: Number(warning.playerIndex),
        reason: DISCONNECT_FORFEIT_REASON,
        disconnected_peer_id: String(warning.peerId || ""),
        disconnect_timeout_ms: DISCONNECT_AUTO_FORFEIT_MS,
        disconnected_at_ms: Number(warning.disconnectedAtMs || 0),
        auto_forfeit_at_ms: Number(warning.autoForfeitAtMs || 0),
        claimed_at_ms: Date.now(),
        basis_sequence: Number(session.lastAppliedSequence || 0),
      };
      try {
        await submitMultiplayerCommand(
          command,
          `${playerName} forfeited by unanimous disconnect timeout policy`
        );
      } catch (err) {
        disconnectForfeitInFlightRef.current.delete(claimKey);
        const message = toErrorMessage(err);
        if (!message.includes("Disconnect timeout has not elapsed")) {
          setStatus(`Disconnect timeout policy failed: ${message}`, true);
          console.error(err);
        }
      }
    };

    void tick();
    const timerId = window.setInterval(() => {
      void tick();
    }, 1000);
    return () => {
      disposed = true;
      window.clearInterval(timerId);
    };
  }, [
    multiplayer.matchStarted,
    setStatus,
    submitMultiplayerCommand,
    updateMultiplayer,
  ]);

  const exportAuditTranscript = useCallback(async ({ includeLiveCheckpoint = true } = {}) => {
    if (!liveAuditTranscriptRef.current) return null;
    const transcript = cloneMultiplayerPayload(liveAuditTranscriptRef.current);
    const currentGame = gameRef.current;
    const finalPublicCheckpoint = includeLiveCheckpoint && currentGame
      && typeof currentGame.exportPublicAuditCheckpoint === "function"
      ? await currentGame.exportPublicAuditCheckpoint()
      : null;
    const actions = Array.isArray(transcript.actions) ? transcript.actions : [];
    const finalPublicCheckpointHash = finalPublicCheckpoint
      ? await publicCheckpointHash(finalPublicCheckpoint)
      : String(actions.at(-1)?.audit?.publicCheckpointHash || transcript.initialPublicCheckpointHash || "");
    const finalStateHash = auditStateHashRef.current || INITIAL_AUDIT_STATE_HASH;
    const disputes = Array.isArray(transcript.disputes) ? transcript.disputes : [];
    const matchId = String(transcript.matchId || transcript.match?.auditMatchId || currentAuditMatchId());
	    const privateViewDisclosures = [...privateViewDisclosuresRef.current.values()]
	      .filter((disclosure) =>
	        String(disclosure?.matchId || disclosure?.payload?.matchId || "") === matchId
	      )
	      .map((disclosure) => cloneMultiplayerPayload(disclosure));
	    return {
      ...transcript,
      exportedAt: new Date().toISOString(),
      privateViewDisclosures,
      finalStateHash,
      finalPublicCheckpoint,
      finalPublicCheckpointHash,
      disputes,
      outcome: buildExportedMatchOutcome({
        uiState: stateRef.current,
        finalPublicCheckpoint,
        finalStateHash,
        finalPublicCheckpointHash,
        matchDisputed: multiplayerRef.current.matchDisputed || null,
        disputes,
      }),
    };
  }, [currentAuditMatchId]);

  useEffect(
    () => () => {
      teardownPeer();
    },
    [teardownPeer]
  );

  return {
    multiplayer,
    canStartHostedMatch: canHostedMatchStart(multiplayer),
    createLobby,
    joinLobby,
    leaveLobby,
    startHostedMatch,
    updateLobbyDeck,
    startRematchSideboarding,
    updateRematchDecks,
    readyForRematch,
    submitMultiplayerCommand,
    submitMultiplayerAddCardCheat,
    exportAuditTranscript,
    routePeerIdForPlayer,
  };
}
