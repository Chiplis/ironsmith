import { useCallback, useEffect, useRef, useState } from "react";
import Peer from "peerjs";
import {
  auditStateHash,
  actionQuorumThreshold,
  buildActionForkDisputeEvidence,
  buildSignedDisconnectForfeitVote,
  buildSignedActionEnvelope,
  buildSignedActionQuorumVote,
  buildDeckSlotOpening,
  buildPrivateDeckManifest,
  buildSignedMatchGenesis,
  buildSignedPlayerGenesis,
  buildSignedResyncEnvelope,
  canonicalJson,
  CURRENT_AUDIT_MAX_PLAYERS,
  CURRENT_AUDIT_MIN_PLAYERS,
  CURRENT_AUDIT_PROTOCOL_VERSION,
  DISCONNECT_AUTO_FORFEIT_MS,
  DISCONNECT_FORFEIT_REASON,
  createAuditEncryptionKey,
  createAuditSessionKey,
  decryptPrivateAuditPayload,
  encryptPrivateAuditPayload,
  decklistHashForCards,
  exportAuditEncryptionKeyPair,
  exportAuditEncryptionPublicKey,
  exportAuditKeyPair,
  exportAuditPublicKey,
  importAuditEncryptionKeyPair,
  importAuditKeyPair,
  importAuditPublicKey,
  isCurrentAuditPlayerCount,
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
  parseSideboardList,
} from "@/lib/decklists";
import { emitSyncFailureNotice } from "@/lib/ui-notices";
import { isDecisionCommandCompatible } from "@/lib/sync-commands";

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
const ACTION_SUBMISSION_IDLE_WAIT_MS = 5000;
const MATCH_CLOCK_AUDIT_TYPE = "match_clock_v1";
const MATCH_CLOCK_POLICY_TYPE = "per_player_match_clock_v1";
const MATCH_CLOCK_AUDIT_DOMAIN = "ironsmith-match-clock-audit-v1";
const TIMEOUT_VOTE_DOMAIN = "ironsmith-match-timeout-vote-v1";
const AUDIT_DECK_MANIFEST_STORAGE_PREFIX = "ironsmith.auditDeckManifest.v1";
const AUDIT_REVEALED_OPENING_STORAGE_PREFIX = "ironsmith.auditRevealedOpening.v1";
const ACTION_QUORUM_VOTE_STORAGE_PREFIX = "ironsmith.actionQuorumVote.v1";
const AUDIT_IDENTITY_STORAGE_KEY = "ironsmith.auditIdentity.v1";
const ZIFFLE_IDENTITY_STORAGE_PREFIX = "ironsmith.ziffleIdentity.v1";
const CURRENT_PLAYER_STORAGE_KEY = "currentPlayer";
const CURRENT_LOBBY_STORAGE_KEY = "currentLobby";
const MATCH_SEED_OFFSET = 0xcbf29ce484222325n;
const MATCH_SEED_PRIME = 0x100000001b3n;
const MATCH_SEED_MASK = 0xffffffffffffffffn;
const matchSeedEncoder = new TextEncoder();
const sleep = (ms) => new Promise((resolve) => globalThis.setTimeout(resolve, ms));

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
    && String(command?.reason || "") === DISCONNECT_FORFEIT_REASON;
}

function isSelfForfeitCommand(command, actorIndex) {
  return isForfeitCommand(command)
    && !isActionTimeoutForfeitCommand(command)
    && !isDisconnectTimeoutForfeitCommand(command)
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
  if (count <= 0) return 0;
  if (count === 1) return 1;
  return 2;
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

function collectCommandObjectIds(command, output = new Set()) {
  if (!command || typeof command !== "object") return output;
  if (command.type === "priority_action" && command.action_ref) {
    const objectId = actionRefObjectId(command.action_ref);
    const numeric = Number(objectId);
    if (Number.isSafeInteger(numeric) && numeric > 0) {
      output.add(numeric);
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
  if (
    requirement.objectId != null
    && opening.objectId != null
    && Number(opening.objectId) !== Number(requirement.objectId)
    && requirement.slot == null
    && !requirement.commitment
  ) {
    return false;
  }
  if (requirement.commitment) {
    const requiredCommitment = String(requirement.commitment);
    const commitmentMatches =
      String(opening.commitment || "") === requiredCommitment
      || String(opening.positionCommitment || "") === requiredCommitment;
    if (!commitmentMatches) return false;
    if (String(opening.commitment || "") === requiredCommitment) return true;
  }
  if (requirement.slot == null) return true;
  return Number(opening.slot) === Number(requirement.slot)
    || Number(opening.position) === Number(requirement.slot);
}

function mergeAuditOpenings(...openingLists) {
  const merged = new Map();
  for (const opening of openingLists.flat()) {
    if (!opening || opening.owner == null || opening.slot == null) continue;
    const key = `${Number(opening.owner)}:${Number(opening.slot)}:${Number(opening.objectId ?? -1)}`;
    const existing = merged.get(key);
    if (existing?.timing === "pre" && opening.timing !== "pre") continue;
    merged.set(key, opening);
  }
  return [...merged.values()];
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

function shuffleProofMatchesRequirement(proof, requirement) {
  if (!proof || !requirement) return false;
  return (
    String(proof?.requirementId || "") === String(requirement.id || "")
    || (
      Number(proof?.owner) === Number(requirement.owner)
      && String(proof?.zone || "library") === String(requirement.zone || "library")
    )
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

function sameShuffleOrder(left, right) {
  const normalizedLeft = normalizeShuffleOrder(left);
  const normalizedRight = normalizeShuffleOrder(right);
  if (normalizedLeft.length !== normalizedRight.length) return false;
  return normalizedLeft.every((entry, index) => entry === normalizedRight[index]);
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

function canonicalMultiplayerPayload(value) {
  return canonicalJson(cloneMultiplayerPayload(value));
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

function parseDeckSubmission(format, deckText, commanderText = "") {
  const deck = sanitizeCardList(parseDeckList(deckText));
  const sideboard = sanitizeCardList(parseSideboardList(deckText));
  const commanders = sanitizeCardList(parseCommanderList(commanderText));
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
      deck: sanitizeCardList(payload?.decks?.[index]),
      sideboard: sanitizeCardList(payload?.sideboards?.[index]),
      commanders: sanitizeCardList(payload?.commanders?.[index]),
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
    if (String(player?.peerId || "").trim() !== targetPeerId) return player;
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

function isSupportedZiffleDeckCount(count) {
  const normalized = Number(count);
  return Number.isInteger(normalized) && normalized >= 2 && normalized <= 100;
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

function redactedMatchPayloadForPeer(payload, peerId) {
  if (!payload || typeof payload !== "object") return payload;
  const target = (payload.players || []).find((player) => player.peerId === peerId);
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

function canHostedMatchStart(session) {
  const playerCount = session.players.length;
  return (
    session.role === "host" &&
    !session.matchStarted &&
    session.mode !== "starting" &&
    playerCount === session.desiredPlayers &&
    isCurrentAuditPlayerCount(playerCount) &&
    session.players.every((player) => player.connected !== false && player.ready)
  );
}

function playerCryptoSeatBindingReady(player) {
  if (!player) return false;
  const index = normalizePlayerIndex(player.index);
  if (index == null) return false;
  const manifest = publicDeckManifest(player.deckAuditManifest);
  const zifflePlayer = normalizePlayerIndex(player.ziffleKey?.player);
  const signer = normalizePlayerIndex(player.playerGenesisSignature?.signer);
  return (
    manifest?.owner === index
    && String(manifest.matchId || "")
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
  const peerServerLabelRef = useRef(describePeerServer(initialPeerOptions));
  const hostMessageQueueRef = useRef(Promise.resolve());
  const clientMessageQueueRef = useRef(Promise.resolve());
  const peerMessageQueueRef = useRef(Promise.resolve());
  const resyncingPeerIdsRef = useRef(new Set());
  const resyncWaitersRef = useRef([]);
  const submissionIdleWaitersRef = useRef([]);
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
  const rngCommitWaitersRef = useRef(new Map());
  const rngRevealWaitersRef = useRef(new Map());
  const rngCommitNoncesRef = useRef(new Map());
  const timeoutVoteWaitersRef = useRef(new Map());
  const actionQuorumVoteWaitersRef = useRef(new Map());
  const signedActionQuorumVotesRef = useRef(new Map());
  const cryptoMaterialWaitersRef = useRef(new Map());
  const outboundCryptoMaterialRequestsRef = useRef(new Map());
  const liveZiffleCeremoniesRef = useRef(new Map());
  const ziffleOpeningPositionsRef = useRef(new Map());
  const ziffleHandRevealKeyRef = useRef("");
  const relayedActionIdsRef = useRef(new Set());
  const actionCryptoRequirementsRef = useRef(new Map());
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

        if (Date.now() - heartbeat.lastSeen > timeoutMs) {
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
          at: Date.now(),
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
    multiplayerRef.current = normalized;
    setMultiplayer(normalized);
    if (!normalized.submittingAction) {
      resolveSubmissionIdleWaiters();
    }
    return normalized;
  }, [resolveSubmissionIdleWaiters, setMultiplayer]);

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

  const waitForZiffleShuffleStep = useCallback((requestId, timeoutMs = 60000) => (
    new Promise((resolve, reject) => {
      const timer = window.setTimeout(() => {
        ziffleShuffleWaitersRef.current.delete(requestId);
        reject(new Error("Timed out waiting for ziffle shuffle step"));
      }, timeoutMs);
      ziffleShuffleWaitersRef.current.set(requestId, {
        resolve: (value) => {
          window.clearTimeout(timer);
          resolve(value);
        },
        reject: (err) => {
          window.clearTimeout(timer);
          reject(err);
        },
      });
    })
  ), []);

  const waitForZiffleRevealToken = useCallback((requestId, timeoutMs = 60000, metadata = null) => (
    new Promise((resolve, reject) => {
      const timer = window.setTimeout(() => {
        ziffleRevealWaitersRef.current.delete(requestId);
        reject(new Error(
          metadata
            ? `Timed out waiting for ziffle reveal token: ${compactZiffleDiagnosticsJson(metadata)}`
            : "Timed out waiting for ziffle reveal token"
        ));
      }, timeoutMs);
      ziffleRevealWaitersRef.current.set(requestId, {
        metadata,
        resolve: (value) => {
          window.clearTimeout(timer);
          resolve(value);
        },
        reject: (err) => {
          window.clearTimeout(timer);
          reject(err);
        },
      });
    })
  ), []);

  const waitForRngCommit = useCallback((requestId, timeoutMs = 60000) => (
    new Promise((resolve, reject) => {
      const timer = window.setTimeout(() => {
        rngCommitWaitersRef.current.delete(requestId);
        reject(new Error("Timed out waiting for random commitment"));
      }, timeoutMs);
      rngCommitWaitersRef.current.set(requestId, {
        resolve: (value) => {
          window.clearTimeout(timer);
          resolve(value);
        },
        reject: (err) => {
          window.clearTimeout(timer);
          reject(err);
        },
      });
    })
  ), []);

  const waitForRngReveal = useCallback((requestId, timeoutMs = 60000) => (
    new Promise((resolve, reject) => {
      const timer = window.setTimeout(() => {
        rngRevealWaitersRef.current.delete(requestId);
        reject(new Error("Timed out waiting for random reveal"));
      }, timeoutMs);
      rngRevealWaitersRef.current.set(requestId, {
        resolve: (value) => {
          window.clearTimeout(timer);
          resolve(value);
        },
        reject: (err) => {
          window.clearTimeout(timer);
          reject(err);
        },
      });
    })
  ), []);

  const waitForTimeoutVote = useCallback((requestId, timeoutMs = 15000) => (
    new Promise((resolve, reject) => {
      const timer = window.setTimeout(() => {
        timeoutVoteWaitersRef.current.delete(requestId);
        reject(new Error("Timed out waiting for timeout vote"));
      }, timeoutMs);
      timeoutVoteWaitersRef.current.set(requestId, {
        resolve: (value) => {
          window.clearTimeout(timer);
          resolve(value);
        },
        reject: (err) => {
          window.clearTimeout(timer);
          reject(err);
        },
      });
    })
  ), []);

  const waitForActionQuorumVote = useCallback((requestId, timeoutMs = 30000) => (
    new Promise((resolve, reject) => {
      const timer = window.setTimeout(() => {
        actionQuorumVoteWaitersRef.current.delete(requestId);
        reject(new Error("Timed out waiting for action quorum vote"));
      }, timeoutMs);
      actionQuorumVoteWaitersRef.current.set(requestId, {
        resolve: (value) => {
          window.clearTimeout(timer);
          resolve(value);
        },
        reject: (err) => {
          window.clearTimeout(timer);
          reject(err);
        },
      });
    })
  ), []);

  const waitForCryptoMaterial = useCallback((requestId, timeoutMs = 60000) => (
    new Promise((resolve, reject) => {
      const timer = window.setTimeout(() => {
        cryptoMaterialWaitersRef.current.delete(requestId);
        reject(new Error("Timed out waiting for cryptographic opening material"));
      }, timeoutMs);
      cryptoMaterialWaitersRef.current.set(requestId, {
        resolve: (value) => {
          window.clearTimeout(timer);
          resolve(value);
        },
        reject: (err) => {
          window.clearTimeout(timer);
          reject(err);
        },
      });
    })
  ), []);

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
      waiter.reject(new Error(
        `${String(message.error)}: ${compactZiffleDiagnosticsJson(diagnostics)}`
      ));
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
	  }, []);

	  const privateDeckManifestForOwner = useCallback((owner, matchId = currentAuditMatchId()) => {
	    const key = `${matchId}:${Number(owner)}`;
	    const cached = privateDeckManifestsRef.current.get(key);
	    if (cached) return cached;
	    const stored = readStoredPrivateDeckManifest(matchId, owner);
	    if (stored?.slotSecrets) {
	      privateDeckManifestsRef.current.set(key, stored);
	      return stored;
	    }
	    return null;
	  }, [currentAuditMatchId]);

  const rememberZiffleOpeningPosition = useCallback((owner, originalSlot, position) => {
    ziffleOpeningPositionsRef.current.set(
      `${Number(owner)}:${Number(originalSlot)}`,
      Number(position)
    );
  }, []);

  const ziffleOpeningPositionForSlot = useCallback((owner, originalSlot) => {
    return ziffleOpeningPositionsRef.current.get(
      `${Number(owner)}:${Number(originalSlot)}`
    );
  }, []);

  const rememberLocalRevealedOpening = useCallback((opening, details = {}) => {
    if (!opening || opening.owner == null || opening.slot == null || !opening.card) return;
    const matchId = String(details.matchId || currentAuditMatchId());
    const writeEntry = (indexKey, entry) => {
      localRevealedOpeningsRef.current.set(indexKey, entry);
      writeStoredRevealedOpening(indexKey, entry);
    };
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
        details.position != null
          ? Number(details.position)
          : opening.position != null
            ? Number(opening.position)
            : null,
      positionCommitment: String(details.positionCommitment || opening.positionCommitment || ""),
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
    if (commitment) {
      candidates.push(
        readEntry(`${matchId}:owner:${owner}:commitment:${commitment}`),
        readEntry(`${matchId}:owner:${owner}:position:${commitment}`)
      );
    }
    if (objectId != null) {
      candidates.push(readEntry(`${matchId}:object:${Number(objectId)}`));
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
    const owner = Number(requirement.owner);
    const slot = requirement.slot == null ? null : Number(requirement.slot);
    const commitment = String(requirement.commitment || "");
    const positionCommitment = String(requirement.positionCommitment || requirement.position_commitment || "");
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
    if (slot != null) {
      candidates.push(readEntry(`${matchId}:owner:${owner}:slot:${slot}`));
    }
    for (const candidate of candidates) {
      if (!candidate) continue;
      if (Number(candidate.owner) !== owner) continue;
      if (slot != null && Number(candidate.slot) !== slot) continue;
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

  const ziffleCeremonyForOwner = useCallback((owner, options = {}) => {
    const normalizedOwner = Number(owner);
    const deckHash = String(options.deckHash || ziffleDeckHashFromCommitment(options.commitment) || "");
    const context = String(options.context || "");
    const live = liveZiffleCeremoniesRef.current.get(normalizedOwner);
    if (
      live
      && (!deckHash || String(live.deckHash || "") === deckHash)
      && (!context || String(live.context || "") === context)
    ) {
      return live;
    }
    const payload = options.payload || matchStartPayloadRef.current;
    return (payload?.ziffleCeremonies || []).find((entry) =>
      Number(entry.owner) === normalizedOwner
      && (!deckHash || String(entry.deckHash || "") === deckHash)
      && (!context || String(entry.context || "") === context)
    ) || null;
  }, []);

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

  const verifyAuditOpeningsAgainstManifests = useCallback(async (openings = []) => {
    for (const opening of openings || []) {
      if (!opening || opening.owner == null || opening.slot == null) continue;
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
    }
  }, [publicDeckManifestForOwner]);

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
    const object = (checkpoint?.objects || []).find(
      (entry) => Number(entry?.id) === normalized
    );
    const hidden = object?.hiddenCard || object?.hidden_card || null;
    if (!hidden) return null;
    return {
      owner: hidden.owner == null ? null : Number(hidden.owner),
      slot: hidden.slot == null ? null : Number(hidden.slot),
      commitment: String(hidden.commitment || ""),
      publicSlot: hidden.publicSlot ?? hidden.public_slot ?? null,
      publicCommitment: String(hidden.publicCommitment || hidden.public_commitment || ""),
    };
  }, []);

		  const buildLocalOpeningsForCommand = useCallback(async (command, cryptoRequirements = [], options = {}) => {
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.exportHiddenCardOpening !== "function") {
      return [];
    }
    const commandObjectIds = collectCommandObjectIds(command);
    const objectIds = new Set(commandObjectIds);
    const publicOpenRequirementByObjectId = new Map();
    for (const requirement of cryptoRequirements || []) {
      if (
        String(requirement?.type || "") === "public_open"
        && requirement.objectId != null
      ) {
        const objectId = Number(requirement.objectId);
        objectIds.add(objectId);
        publicOpenRequirementByObjectId.set(objectId, requirement);
      }
    }
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
          opening.timing = commandObjectIds.has(Number(objectId)) ? "pre" : "post";
          const zifflePosition = opening.position ?? ziffleOpeningPositionForSlot(opening.owner, opening.slot);
          if (zifflePosition != null) {
            opening.position = Number(zifflePosition);
            const ceremony = ziffleCeremonyForOwner(opening.owner);
            if (ceremony?.deckHash && !opening.positionCommitment) {
              opening.positionCommitment = ziffleRuntimeCommitment(ceremony.deckHash, zifflePosition);
            }
          }
          if (remappedFromSlot != null) {
            opening.reportedSlot = Number(remappedFromSlot);
          }
          rememberLocalRevealedOpening(opening, {
            objectId: opening.objectId,
            position: opening.position,
            positionCommitment: opening.positionCommitment,
          });
          openings.push(opening);
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
      let remappedFromSlot = null;
      let opening = cachedOpening;
      if (!opening) {
        let preferredSlot = Number(exported.slot);
        let position = null;
        let positionCommitment = "";
        const hiddenMetadata = await currentHiddenCardMetadataForObject(
          exported.object_id ?? exported.objectId ?? objectId
        );
        const exportedCommitment = String(exported.commitment || "");
        const publicPosition = hiddenMetadata?.publicCommitment && hiddenMetadata.publicSlot != null
          ? Number(hiddenMetadata.publicSlot)
          : null;
        const publicPositionCommitment = String(hiddenMetadata?.publicCommitment || "");
        const hiddenPositionCommitment =
          hiddenMetadata?.commitment && ziffleDeckHashFromCommitment(hiddenMetadata.commitment)
            ? String(hiddenMetadata.commitment)
            : "";
        let ziffleCommitment = exportedCommitment;
        let ziffleDeckHash = ziffleDeckHashFromCommitment(ziffleCommitment);
        if (!exportedCommitment || ziffleDeckHash) {
          if (!ziffleDeckHash) {
            ziffleCommitment = publicPositionCommitment || hiddenPositionCommitment;
            ziffleDeckHash = ziffleDeckHashFromCommitment(ziffleCommitment);
          }
        } else {
          if (publicPosition != null && publicPositionCommitment) {
            position = publicPosition;
            positionCommitment = publicPositionCommitment;
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
          if (typeof currentGame.ziffleRevealCard !== "function") {
            throw new Error("Ziffle opening reveal backend is not available");
          }
          position = Number(
            hiddenMetadata?.publicCommitment && hiddenMetadata.publicSlot != null
              ? hiddenMetadata.publicSlot
              : hiddenMetadata?.commitment && ziffleDeckHashFromCommitment(hiddenMetadata.commitment)
                ? hiddenMetadata.slot
                : exported.slot
          );
          const ceremony = ziffleCeremonyForOwner(exported.owner, {
            commitment: ziffleCommitment,
          });
          if (!ceremony) {
            throw new Error(`Missing ziffle ceremony for opening player ${Number(exported.owner) + 1}`);
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
          preferredSlot = Number(reveal.originalSlot);
          positionCommitment = ziffleCommitment || ziffleRuntimeCommitment(ceremony.deckHash, position);
          rememberZiffleOpeningPosition(exported.owner, preferredSlot, position);
        }
        const built = await buildDeckSlotOpeningForExport({
          manifest,
          preferredSlot,
          card: exported.card,
          exportedCommitment: ziffleDeckHash ? "" : exportedCommitment,
          label: "Local hidden card opening",
        });
        opening = built.opening;
        remappedFromSlot = built.remappedFromSlot;
        if (position != null) {
          opening.position = Number(position);
          opening.positionCommitment = positionCommitment;
        }
      }
      opening.objectId = Number(exported.object_id ?? exported.objectId ?? objectId);
      opening.timing = commandObjectIds.has(Number(objectId)) ? "pre" : "post";
      const zifflePosition = ziffleOpeningPositionForSlot(opening.owner, opening.slot);
      if (zifflePosition != null) {
        opening.position = Number(zifflePosition);
        const ceremony = ziffleCeremonyForOwner(opening.owner);
        if (ceremony?.deckHash && !opening.positionCommitment) {
          opening.positionCommitment = ziffleRuntimeCommitment(ceremony.deckHash, zifflePosition);
        }
      }
      if (remappedFromSlot != null) {
        opening.reportedSlot = Number(remappedFromSlot);
      }
      rememberLocalRevealedOpening(opening, {
        objectId: opening.objectId,
        positionCommitment: opening.positionCommitment,
      });
      openings.push(opening);
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
      rememberZiffleOpeningPosition,
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
	      const cachedOpeningForRequirement = exported
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
      const hiddenMetadata = await currentHiddenCardMetadataForObject(
        exported?.object_id ?? exported?.objectId ?? requirement?.objectId
      );
	    const exportedCommitment = String(exported?.commitment || requirement?.commitment || "");
      const publicPosition = hiddenMetadata?.publicCommitment && hiddenMetadata.publicSlot != null
        ? Number(hiddenMetadata.publicSlot)
        : null;
      const publicPositionCommitment = String(hiddenMetadata?.publicCommitment || "");
      const hiddenPositionCommitment =
        hiddenMetadata?.commitment && ziffleDeckHashFromCommitment(hiddenMetadata.commitment)
          ? String(hiddenMetadata.commitment)
          : "";
      let ziffleCommitment = exportedCommitment;
	    let ziffleDeckHash = ziffleDeckHashFromCommitment(ziffleCommitment);
      if (!exportedCommitment || ziffleDeckHash) {
        if (!ziffleDeckHash) {
          ziffleCommitment = publicPositionCommitment || hiddenPositionCommitment;
          ziffleDeckHash = ziffleDeckHashFromCommitment(ziffleCommitment);
        }
      } else {
        if (publicPosition != null && publicPositionCommitment) {
          position = publicPosition;
          positionCommitment = publicPositionCommitment;
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
	    if (ziffleDeckHash) {
	      const currentGame = gameRef.current;
	      if (!currentGame || typeof currentGame.ziffleRevealCard !== "function") {
	        throw new Error("Ziffle opening reveal backend is not available");
	      }
	      position = Number(
          hiddenMetadata?.publicCommitment && hiddenMetadata.publicSlot != null
            ? hiddenMetadata.publicSlot
            : hiddenMetadata?.commitment && ziffleDeckHashFromCommitment(hiddenMetadata.commitment)
              ? hiddenMetadata.slot
              : exported?.slot ?? requirement?.slot
        );
	      const ceremony = ziffleCeremonyForOwner(owner, { commitment: ziffleCommitment });
	      if (!ceremony) {
	        throw new Error(`Missing ziffle ceremony for opening player ${owner + 1}`);
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
	      originalSlot = Number(reveal.originalSlot);
	      positionCommitment = ziffleCommitment || ziffleRuntimeCommitment(ceremony.deckHash, position);
	      rememberZiffleOpeningPosition(owner, originalSlot, position);
	    }
      if (position == null && cachedOpeningForRequirement?.position != null) {
        position = Number(cachedOpeningForRequirement.position);
        positionCommitment = String(cachedOpeningForRequirement.positionCommitment || positionCommitment || "");
      }
      if (position == null) {
        const rememberedPosition = ziffleOpeningPositionForSlot(owner, originalSlot);
        if (rememberedPosition != null) {
          position = Number(rememberedPosition);
          const ceremony = ziffleCeremonyForOwner(owner);
          if (ceremony?.deckHash && !positionCommitment) {
            positionCommitment = ziffleRuntimeCommitment(ceremony.deckHash, position);
          }
        }
      }

		    const secret = (manifest?.slotSecrets || []).find(
	      (entry) => Number(entry.slot) === Number(originalSlot)
	    );
	    if (!secret && !cachedOpeningForRequirement && !exported?.card && !requirement?.card) {
	      throw new Error(`Missing private deck opening for slot ${Number(originalSlot)}`);
	    }
	    const card = String(exported?.card || requirement?.card || secret?.card || cachedOpeningForRequirement?.card || "");
      let cachedOpening = cachedOpeningForRequirement;
      let remappedFromSlot = null;
      let opening = cachedOpening;
      if (!opening) {
        let built = null;
        built = await buildDeckSlotOpeningForExport({
          manifest,
          preferredSlot: originalSlot,
          card,
          exportedCommitment: ziffleDeckHash ? "" : exportedCommitment,
          label: "Private deck opening",
        });
        opening = built.opening;
        remappedFromSlot = built.remappedFromSlot;
      }
      if (remappedFromSlot != null) {
        originalSlot = Number(opening.slot);
      }
      if (exported) {
        rememberLocalRevealedOpening(opening, {
          objectId: exported.object_id ?? exported.objectId,
          position,
          positionCommitment,
        });
      }
	    return {
	      opening: {
	        ...opening,
	        ...(requirement?.objectId != null
	          ? { objectId: Number(requirement.objectId) }
	          : exported?.object_id != null || exported?.objectId != null
	            ? { objectId: Number(exported.object_id ?? exported.objectId) }
	            : {}),
	        timing: "post",
	        ...(position != null ? { position } : {}),
	        ...(positionCommitment ? { positionCommitment } : {}),
          ...(remappedFromSlot != null ? { reportedSlot: Number(remappedFromSlot) } : {}),
	      },
	      owner,
	      originalSlot,
	      position,
	      positionCommitment,
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
      ziffleCeremonyForOwner,
      ziffleOpeningPositionForSlot,
	  ]);

	  const buildLocalRequirementOpeningsForRequirements = useCallback(async (requirements = [], options = {}) => {
	    const openings = [];
	    for (const requirement of requirements || []) {
	      if (String(requirement?.type || "") !== "public_open") continue;
		      const localSeat = resolveLocalCryptoPlayerIndex();
	      if (Number(requirement.owner) !== Number(localSeat)) continue;
	      openings.push((await buildLocalOpeningFromRequirement(requirement, null, options)).opening);
	    }
	    return openings;
	  }, [buildLocalOpeningFromRequirement, resolveLocalCryptoPlayerIndex]);

	  const buildLocalDeckAuditManifest = useCallback(
    async ({ matchId, owner, deck, sideboard, commanders }) => {
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
        && Number(existing.deckCount) === normalizedDeck.length
        && Number(existing.sideboardCount || 0) === normalizedSideboard.length
        && Number(existing.commanderCount || 0) === normalizedCommanders.length
        && Array.isArray(existing.slotSecrets)
        && existing.slotSecrets.length === normalizedDeck.length
      ) {
        rememberPrivateDeckManifest(existing);
        return existing;
      }
      const manifest = await buildPrivateDeckManifest({
        matchId: normalizedMatchId,
        owner: normalizedOwner,
        deck: normalizedDeck,
        sideboard: normalizedSideboard,
        commanders: normalizedCommanders,
      });
      rememberPrivateDeckManifest(manifest);
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
      await verifyAuditOpeningsAgainstManifests(audit.openings || []);
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
      if (!type || type === "hidden_move") continue;
      if (type === "public_open") {
        const match = (audit.openings || []).find((opening) =>
          openingMatchesRequirement(opening, requirement)
        );
        if (!match) {
          throw new Error(
            `Missing ${type} audit opening for player ${Number(requirement.owner) + 1}`
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
          && String(proof.positionCommitment || proof.commitment) !== String(requirement.commitment)
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
        if (matchingProofs.length < Number(requirement.count || 0)) {
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
        const combinedSeedHex = await sha256Hex(canonicalJson({
          domain: "ironsmith-combined-rng-v1",
          matchId: currentAuditMatchId(),
          seq: Number(audit.seq),
          requirementId: String(requirement.id || ""),
          commits: reveal.commits || [],
          reveals: reveal.reveals || [],
        }));
        if (combinedSeedHex !== String(reveal.combinedSeedHex || "")) {
          throw new Error("Transcripted fair-random combined seed is invalid");
        }
      }
    }
  }, [auditEncryptionPublicKeyForPlayer, currentAuditMatchId]);

  const revealAuditOpenings = useCallback(async (openings = [], options = {}) => {
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.revealHiddenSlot !== "function") return;
    const timing = options.timing || null;
    for (const opening of openings || []) {
      if (!opening || opening.owner == null || opening.slot == null || !opening.card) {
        continue;
      }
      if (timing && String(opening.timing || "pre") !== timing) {
        continue;
      }
      try {
        await verifyAuditOpeningsAgainstManifests([opening]);
        const localHiddenMetadata = opening.objectId != null
          ? await currentHiddenCardMetadataForObject(opening.objectId)
          : null;
        const localHiddenZiffleCommitment =
          localHiddenMetadata?.commitment
          && ziffleDeckHashFromCommitment(localHiddenMetadata.commitment)
            ? String(localHiddenMetadata.commitment)
            : "";
        const revealPosition =
          opening.position != null
            ? Number(opening.position)
            : localHiddenZiffleCommitment && localHiddenMetadata?.slot != null
              ? Number(localHiddenMetadata.slot)
              : null;
        const revealPositionCommitment =
          String(opening.positionCommitment || "")
          || localHiddenZiffleCommitment;
        if (
          revealPosition != null
          && typeof currentGame.revealHiddenPosition === "function"
        ) {
          const ceremony = ziffleCeremonyForOwner(opening.owner, {
            commitment: revealPositionCommitment,
          });
          await currentGame.revealHiddenPosition({
            owner: Number(opening.owner),
            position: Number(revealPosition),
            originalSlot: Number(opening.slot),
            cardName: String(opening.card),
            positionCommitment: revealPositionCommitment
              || (ceremony
                ? ziffleRuntimeCommitment(ceremony.deckHash, revealPosition)
                : undefined),
            commitment: opening.commitment || undefined,
          });
        } else {
          await currentGame.revealHiddenSlot({
            owner: Number(opening.owner),
            slot: Number(opening.slot),
            cardName: String(opening.card),
            commitment: opening.commitment || undefined,
          });
        }
      } catch (err) {
        const message = String(err?.message || err || "");
        if (!message.includes("not present") && !message.includes("not a hidden")) {
          throw err;
        }
      }
    }
  }, [currentHiddenCardMetadataForObject, verifyAuditOpeningsAgainstManifests, ziffleCeremonyForOwner]);

  async function previewRequirementsForCommand(command) {
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.previewCryptoRequirements !== "function") {
      return [];
    }
    const requirements = await currentGame.previewCryptoRequirements(command);
    return Array.isArray(requirements) ? requirements : [];
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

  async function privateOpeningFromEncryptedProof(proof, requirement = {}) {
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
    return {
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
    };
  }

  async function privateOpeningFromProof(requirement, audit = {}) {
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
    return privateOpeningFromEncryptedProof(proof, requirement);
  }

  async function revealPrivateAuditProofsForLocalViewer(audit = {}) {
    const localSeat = resolveLocalCryptoPlayerIndex();
    const openings = [];
    const seen = new Set();
    for (const proof of audit.privateViewProofs || []) {
      if (String(proof?.type || "") !== "encrypted_private_opening") continue;
      if (Number(proof.viewer) !== Number(localSeat)) continue;
      const opening = await privateOpeningFromEncryptedProof(proof, {
        owner: proof.owner,
        viewer: proof.viewer,
        objectId: proof.objectId,
      });
      if (!opening) continue;
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
      await revealAuditOpenings(openings);
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
      const ceremony = ziffleCeremonyForOwner(localSeat, { commitment: positionCommitment });
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
        const originalSlot = revealByPosition.get(Number(entry.position));
        if (!Number.isSafeInteger(originalSlot) || originalSlot < 0) {
          throw new Error(`Missing ziffle reveal for position ${Number(entry.position)}`);
        }
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
        const openingWithPosition = {
          ...opening,
          ...(Number.isSafeInteger(objectId) ? { objectId } : {}),
          timing: "post",
          position: Number(entry.position),
          positionCommitment: entry.positionCommitment,
        };
        rememberLocalRevealedOpening(openingWithPosition, {
          objectId: openingWithPosition.objectId,
          position: openingWithPosition.position,
          positionCommitment: openingWithPosition.positionCommitment,
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
      let opening = await privateOpeningFromProof(requirement, audit);
      if (!opening && Number(requirement.owner) === Number(localSeat)) {
        opening = (await buildLocalOpeningFromRequirement(requirement, null, options)).opening;
      }
      if (!opening) continue;
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

  async function injectCryptoMaterialForRequirements(requirements = [], audit = {}, options = {}) {
    const currentGame = gameRef.current;
    if (!currentGame) return;
    const seeds = [];
    for (const requirement of requirements || []) {
      const type = String(requirement?.type || "");
      if (type === "verifiable_shuffle") {
        const proof = (audit.shuffleProofs || []).find((entry) =>
          shuffleProofMatchesRequirement(entry, requirement)
        );
        if (proof?.deckHash) seeds.push(String(proof.deckHash));
      } else if (type === "fair_random") {
        const reveal = (audit.rngReveals || []).find(
          (entry) => String(entry?.requirementId || "") === String(requirement.id || "")
        );
        if (reveal?.combinedSeedHex) seeds.push(String(reveal.combinedSeedHex));
      }
    }
    if (seeds.length > 0 && typeof currentGame.injectTranscriptRandomSeeds === "function") {
      await currentGame.injectTranscriptRandomSeeds({ seeds });
    }
    const privateOpenings = await privateOpeningsForLocalViewer(requirements, audit, options);
    if (privateOpenings.length > 0) {
      await revealAuditOpenings(privateOpenings);
    }
  }

	  const buildLocalPrivateViewProofsForRequirements = useCallback(async (requirements = [], options = {}) => {
	    const currentGame = gameRef.current;
	    const proofs = [];
		    const localSeat = resolveLocalCryptoPlayerIndex();
      const privateOpenRequirements = (requirements || []).filter((requirement) =>
        String(requirement?.type || "") === "private_open"
        && !isOwnerPrivateViewRequirement(requirement)
        && Number(requirement.owner) === Number(localSeat)
      );
      const privateOpeningProofs = (await Promise.all(privateOpenRequirements.map(async (requirement) => {
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
	      const openingPayload = {
	        type: "private_view_opening",
	        matchId: currentAuditMatchId(),
	        requirementId: String(requirement.id || ""),
	        owner,
        viewer,
        zone: String(requirement.zone || ""),
        objectId: Number(requirement.objectId),
        opening: {
	          ...opening,
	          objectId: Number(requirement.objectId),
	          timing: "private",
	        },
	      };
      const encryptedOpening = await encryptPrivateAuditPayload({
        recipientPublicKey,
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
	      if (Number(requirement.owner) !== Number(localSeat)) continue;
	      const openingHashes = privateOpeningProofs
        .filter((entry) =>
          Number(entry.owner) === Number(requirement.owner)
          && Number(entry.viewer) === Number(requirement.viewer)
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

  const answerCryptoMaterialRequest = useCallback(async (conn, message) => {
    try {
	      const localSeat = resolveLocalCryptoPlayerIndex();
      const requestedRequirements = Array.isArray(message.requirements)
        ? message.requirements
        : [];
      let previewedRequirements = [];
      if (message.command) {
        try {
          previewedRequirements = await previewRequirementsForCommand(message.command);
        } catch {
          previewedRequirements = [];
        }
      }
      const requirementByKey = new Map();
      for (const requirement of [...requestedRequirements, ...previewedRequirements]) {
        const key = String(requirement?.id || canonicalMultiplayerPayload(requirement));
        if (!requirementByKey.has(key)) {
          requirementByKey.set(key, requirement);
        }
      }
      const requirements = [...requirementByKey.values()].filter((requirement) =>
        !isOwnerPrivateViewRequirement(requirement)
        && Number(requirement?.owner) === Number(localSeat)
      );
      const material = await buildLocalCryptoMaterialForRequirements(requirements, {
        cryptoMaterialRequestId: message.requestId,
        command: message.command || null,
        seq: message.seq,
        actorIndex: message.actorIndex,
        requesterIndex: message.requesterIndex,
      });
      safeSend(conn, {
        type: "crypto_material_response",
        protocolVersion: PROTOCOL_VERSION,
        requestId: message.requestId,
        openings: material.openings,
        privateViewProofs: material.privateViewProofs,
      });
    } catch (err) {
      safeSend(conn, {
        type: "crypto_material_response",
        protocolVersion: PROTOCOL_VERSION,
        requestId: message.requestId,
        error: toErrorMessage(err),
      });
    }
  }, [
    buildLocalCryptoMaterialForRequirements,
    resolveLocalCryptoPlayerIndex,
  ]);

  const collectRemoteCryptoMaterialForRequirements = useCallback(async (requirements = [], options = {}) => {
	    const localSeat = resolveLocalCryptoPlayerIndex();
    const materialByOwner = new Map();
    for (const requirement of requirements || []) {
      const type = String(requirement?.type || "");
      if (!["public_open", "private_open", "private_view_window"].includes(type)) continue;
      if (isOwnerPrivateViewRequirement(requirement)) continue;
      const owner = Number(requirement.owner);
      if (!Number.isInteger(owner) || owner === Number(localSeat)) continue;
      if (!materialByOwner.has(owner)) materialByOwner.set(owner, []);
      materialByOwner.get(owner).push(requirement);
    }
    const command = options.command || null;
    const seq = options.seq;
    const actorIndex = options.actorIndex;
    const players = reindexPlayers(matchStartPayloadRef.current?.players || multiplayerRef.current.players || []);
    if (command) {
      for (const player of players) {
        const owner = Number(player?.index);
        if (!Number.isInteger(owner) || owner === Number(localSeat)) continue;
        if (!materialByOwner.has(owner)) materialByOwner.set(owner, []);
      }
    }
    if (materialByOwner.size === 0) {
      return { openings: [], privateViewProofs: [] };
    }

    const responses = [];
    for (const [owner, ownerRequirements] of materialByOwner) {
      const player = players.find((entry) => Number(entry.index) === owner);
      if (!player?.peerId) {
        throw new Error(`Missing peer route for cryptographic material from player ${owner + 1}`);
      }
      const conn = await waitForZiffleRoute(player.peerId);
      const requestId = makeZiffleRequestId("crypto-material");
      const waiter = waitForCryptoMaterial(requestId);
      outboundCryptoMaterialRequestsRef.current.set(requestId, {
        owner,
        peerId: String(player.peerId || ""),
        requirements: cloneMultiplayerPayload(ownerRequirements),
        command: command ? cloneMultiplayerPayload(command) : null,
        seq,
        actorIndex,
        createdAt: Date.now(),
      });
      setStatus(`Waiting for cryptographic material from ${player.name || `Player ${owner + 1}`}`);
      safeSend(conn, {
        type: "crypto_material_request",
        protocolVersion: PROTOCOL_VERSION,
        requestId,
        matchId: currentAuditMatchId(),
        requesterIndex: localSeat,
        ...(seq !== null && seq !== undefined ? { seq: Number(seq) } : {}),
        ...(actorIndex !== null && actorIndex !== undefined ? { actorIndex: Number(actorIndex) } : {}),
        requirements: ownerRequirements,
        ...(command ? { command: cloneMultiplayerPayload(command) } : {}),
      });
      try {
        responses.push(await waiter);
      } finally {
        outboundCryptoMaterialRequestsRef.current.delete(requestId);
      }
    }

    return {
      openings: mergeAuditOpenings(...responses.map((response) => response.openings || [])),
      privateViewProofs: mergePrivateViewProofs(
        ...responses.map((response) => response.privateViewProofs || [])
      ),
    };
  }, [
    currentAuditMatchId,
    makeZiffleRequestId,
    resolveLocalCryptoPlayerIndex,
    setStatus,
    waitForCryptoMaterial,
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
    awaitingStateResyncRef.current = false;
    relayedActionIdsRef.current.clear();

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
      actionHistoryRef.current = [];
      actionCryptoRequirementsRef.current.clear();
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

    return {
      ...cloneMultiplayerPayload(basePayload),
      protocolVersion: PROTOCOL_VERSION,
      lobbyId: session.lobbyId || basePayload.lobbyId || "",
      hostPeerId: session.localPeerId || basePayload.hostPeerId || "",
      format: normalizeMatchFormat(basePayload.format || session.format),
      startingLife: Number(basePayload.startingLife || session.startingLife || 20),
      players: toPublicPlayers(session.players),
    };
  }, []);

  const sendHostedStateMessage = useCallback(
    async (conn, payload) => {
      const session = multiplayerRef.current;
      if (!session.players.some((entry) => entry.peerId === conn.peer)) {
        throw new Error("Cannot resync an unknown peer");
      }
      const peerPlayer = session.players.find((entry) => entry.peerId === conn.peer);
      const peerIndex = normalizePlayerIndex(peerPlayer?.index);
      const currentGame = gameRef.current;
      if (!currentGame || typeof currentGame.exportSyncCheckpoint !== "function") {
        throw new Error("Game engine cannot export a resync checkpoint");
      }
      const checkpoint =
        peerIndex != null && typeof currentGame.exportRedactedSyncCheckpoint === "function"
          ? await currentGame.exportRedactedSyncCheckpoint(peerIndex)
          : await currentGame.exportSyncCheckpoint();
      const actions = (payload.actions || actionHistoryRef.current || [])
        .map((entry) => cloneMultiplayerPayload(entry));
      const lastSequence = Number(payload.lastSequence ?? actions.at(-1)?.seq ?? 0);
      const resyncEnvelope = await buildSignedResyncEnvelope({
        keyPair: auditKeyPairRef.current,
        matchId: payload.match?.auditMatchId || currentAuditMatchId(),
        signer: resolveLocalPlayerIndex(session) ?? 0,
        lastSequence,
        finalStateHash: auditStateHashRef.current || INITIAL_AUDIT_STATE_HASH,
        checkpoint,
        actions,
      });
      safeSend(conn, {
        ...payload,
        match: redactedMatchPayloadForPeer(payload.match, conn.peer),
        checkpoint,
        actions,
        resyncEnvelope,
      });
    },
    [currentAuditMatchId]
  );

  function sequencedActionRelayKey(message) {
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
      const peerId = String(player?.peerId || "").trim();
      if (!peerId || peerId === session.localPeerId) continue;
      sendDirectPeerMessage(peerId, {
        ...cloneMultiplayerPayload(message),
        relayedBy: session.localPeerId || "",
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

  function restoreMatchClockFromTranscript(matchPayload, actionEntries, uiState) {
    const policy = matchClockPolicyFromPayload(matchPayload, matchClockConfigRef.current);
    matchClockConfigRef.current = policy;
    const playerCount = Math.max(
      matchPayload?.players?.length || 0,
      playerCountForClock(uiState)
    );
    const clock = [...(actionEntries || [])]
      .reverse()
      .map((entry) => entry?.audit?.clock)
      .find((entry) => entry && typeof entry === "object");
    matchClockRef.current = {
      policy,
      playerCount,
      baseRemainingMsByPlayer: normalizeMatchClockRemaining(
        clock?.remainingMsByPlayer,
        playerCount,
        policy.initialMs
      ),
      activePlayerIndex: null,
      epochStartedAtMs: null,
      clockHash: String(clock?.clockHash || INITIAL_MATCH_CLOCK_HASH),
      lastSequence: Number(clock?.seq || 0),
    };
    return updateMatchClockForState(uiState, { policy });
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
        : isDisconnectForfeit
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
    const elapsedMs = Math.max(0, Math.floor(Number(clock.elapsedMs || 0)));
    const expectedRemaining = debitMatchClockRemaining(baseRemaining, activePlayer, elapsedMs);
    const actualRemaining = normalizeMatchClockRemaining(
      clock.remainingMsByPlayer,
      runtime.playerCount,
      runtime.policy.initialMs
    );
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
    await verifyTimeoutCertificate(command, uiState);
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
    await verifyTimeoutCertificate(command, uiState);
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
      if (!player?.peerId) {
        throw new Error(`Missing peer route for timeout voter ${voter + 1}`);
      }
      const conn = await waitForZiffleRoute(player.peerId);
      const requestId = makeZiffleRequestId("timeout-vote");
      const waiter = waitForTimeoutVote(requestId);
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
    const disconnectedAtMs = Math.max(
      0,
      Math.floor(Number(target?.disconnectedAtMs || command?.disconnected_at_ms || 0))
    );
    if (!forfeitedPeerId) {
      throw new Error("Disconnect forfeit is missing the disconnected peer id");
    }
    if (!target || target.connected !== false) {
      throw new Error("Player is not currently marked disconnected");
    }
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
    const roster = disconnectForfeitRoster(forfeitedPlayer);
    await verifyDisconnectForfeitCertificate({
      certificate,
      command: {
        ...command,
        matchId: currentAuditMatchId(),
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
      const pending = roster
        .filter((player) => Number(player.index) !== Number(localPlayer))
        .map(async (player) => {
          if (!player?.peerId) {
            throw new Error(`Missing peer route for disconnect voter ${Number(player.index) + 1}`);
          }
          const conn = await waitForZiffleRoute(player.peerId);
          const requestId = makeZiffleRequestId("disconnect-forfeit-vote");
          const waiter = waitForTimeoutVote(requestId);
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
        if (settled.err) continue;
        await addVote(settled.vote);
      }
      for (const promise of unsettled) {
        promise.catch(() => {});
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
      const pending = players
        .filter((player) => Number(player.index) !== Number(localPlayer))
        .map(async (player) => {
          if (!player?.peerId) {
            throw new Error(`Missing peer route for quorum voter ${Number(player.index) + 1}`);
          }
          const conn = await waitForZiffleRoute(player.peerId);
          const requestId = makeZiffleRequestId("action-quorum");
          const waiter = waitForActionQuorumVote(requestId);
          safeSend(conn, {
            type: "action_quorum_vote_request",
            protocolVersion: PROTOCOL_VERSION,
            requestId,
            requesterIndex: localPlayer,
            action: cloneMultiplayerPayload(message),
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
        if (settled.err) continue;
        await addVote(settled.vote);
      }
      for (const promise of unsettled) {
        promise.catch(() => {});
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
      await applySequencedActionMessage(action, {
        relay: false,
        dryRun: true,
        skipQuorumCertificate: true,
        throwOnOrderMismatch: true,
      });
      const vote = await signActionQuorumVoteForMessage(action);
      safeSend(conn, {
        type: "action_quorum_vote_response",
        protocolVersion: PROTOCOL_VERSION,
        requestId: message.requestId,
        vote,
      });
    } catch (err) {
      const failureReason = toErrorMessage(err);
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
    auditStateHashRef.current = message.audit.nextStateHash;
    updateMultiplayer((prev) => ({
      ...prev,
      lastAppliedSequence: nextSequence,
      submittingAction: false,
    }));
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
      auditStateHash: auditStateHashRef.current,
      lastAppliedSequence: Number(multiplayerRef.current.lastAppliedSequence || 0),
      matchClock: cloneMultiplayerPayload(matchClockRef.current),
      matchClockConfig: cloneMultiplayerPayload(matchClockConfigRef.current),
      actionCryptoRequirements: new Map(
        [...actionCryptoRequirementsRef.current.entries()].map(([seq, requirements]) => [
          seq,
          cloneMultiplayerPayload(requirements),
        ])
      ),
      ziffleHandRevealKey: ziffleHandRevealKeyRef.current,
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
    auditStateHashRef.current = snapshot.auditStateHash;
    matchClockConfigRef.current = cloneMultiplayerPayload(snapshot.matchClockConfig);
    matchClockRef.current = cloneMultiplayerPayload(snapshot.matchClock);
    actionCryptoRequirementsRef.current = new Map(
      [...snapshot.actionCryptoRequirements.entries()].map(([seq, requirements]) => [
        seq,
        cloneMultiplayerPayload(requirements),
      ])
    );
    ziffleHandRevealKeyRef.current = snapshot.ziffleHandRevealKey;
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

  async function applySequencedActionMessage(message, options = {}) {
    const nextSequence = Number(message?.seq || 0);
    const session = multiplayerRef.current;
    const dryRun = Boolean(options.dryRun);
    if (awaitingStateResyncRef.current) {
      if (dryRun || options.throwOnOrderMismatch) {
        throw new Error("Cannot validate action while awaiting state resync");
      }
      return;
    }
    if (nextSequence <= session.lastAppliedSequence) {
      if (dryRun || options.throwOnOrderMismatch) {
        throw new Error("Action quorum request is not ahead of the local transcript");
      }
      await handleHistoricalSequencedAction(message);
      return;
    }
    if (nextSequence !== session.lastAppliedSequence + 1) {
      if (dryRun || options.throwOnOrderMismatch) {
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
      snapshotRestored = true;
      await restoreSequencedActionValidationSnapshot(validationSnapshot);
    };

    if (!dryRun) {
      updateMultiplayer((prev) => ({ ...prev, submittingAction: true }));
    }
    try {
      await verifySequencedActionAudit({
        audit: message.audit,
        seq: nextSequence,
        actorIndex: message.actorIndex,
        command: message.command,
      });
      if (!options.skipQuorumCertificate) {
        await verifyActionQuorumForMessage(message);
      }
      const liveStateForClock = gameRef.current ? await gameRef.current.uiState() : stateRef.current;
      if (!isUnauthorizedAddCardCommand(message.command)) {
        await verifyMatchClockAuditForAction({
          clock: message.audit?.clock,
          command: message.command,
          seq: nextSequence,
          actorIndex: message.actorIndex,
          uiState: liveStateForClock,
          skewMs: MATCH_CLOCK_CLAIM_SKEW_MS,
        });
      }
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
      const isSelfForfeit = isSelfForfeitCommand(message.command, message.actorIndex);
      if (isSelfForfeit && !isSorcerySpeedForfeitState(liveStateForClock, message.actorIndex)) {
        throw new Error("Surrender is only available at sorcery speed");
      }
      if (isForfeitCommand(message.command) && !isTimeoutForfeit && !isDisconnectForfeit) {
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
      } else if (
        expectedActor !== null
        && expectedActor !== undefined
        && Number(expectedActor) !== Number(message.actorIndex)
      ) {
        throw new Error("Sequenced action actor is not the current decision player");
      }
      await revealAuditOpenings(message.audit?.openings || [], { timing: "pre" });
      await revealPrivateAuditProofsForLocalViewer(message.audit || {});
      const localCommand = await remapPriorityCommandForLocalHiddenOpening(
        message.command,
        message.audit?.openings || []
      );
      const cryptoRequirements = filterCryptoRequirementsForCommand(
        localCommand,
        liveStateForClock,
        freshCryptoRequirementsForSequence(
          nextSequence,
          await previewRequirementsForCommand(localCommand)
        )
      );
      rememberActionCryptoRequirements(nextSequence, cryptoRequirements);
      await verifyShuffleProofsForRequirements(
        cryptoRequirements,
        message.audit?.shuffleProofs || []
      );
      await verifyAuditSatisfiesCryptoRequirements({
        requirements: cryptoRequirements,
        audit: message.audit,
      });
      await injectCryptoMaterialForRequirements(cryptoRequirements, message.audit || {}, {
        command: localCommand,
        seq: nextSequence,
        actorIndex: message.actorIndex,
        requirements: cryptoRequirements,
      });
      const appliedState = await applySyncedCommand(localCommand, message.label || "", {
        actorIndex: message.actorIndex,
        sequence: nextSequence,
      });
      await revealAuditOpenings(message.audit?.openings || [], { timing: "post" });
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
      await applyVerifiedShuffleProofs(actionShuffleProofs);
      await verifyAuditSatisfiesCryptoRequirements({
        requirements: appliedCryptoRequirements,
        audit: message.audit,
      });
      await revealLocalZiffleHand(matchStartPayloadRef.current, {
        skipIfHandUnchanged: true,
        stateHint: appliedState,
        command: localCommand,
        seq: nextSequence,
        actorIndex: message.actorIndex,
        requirements: actionCryptoRequirements,
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
      if (options.relay !== false) {
        relaySequencedAction(message);
      }
    } catch (err) {
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
      if (isUnauthorizedAddCardCommand(message?.command)) {
        const actorName = playerNameForIndex(multiplayerRef.current.players, message.actorIndex);
        const reason = err instanceof Error ? err.message : String(err);
        const status = `Rejected signed add-card cheat from ${actorName}: ${reason}`;
        emitSyncFailureNotice("Cheat detected", status);
        setStatus(status, true);
        return;
      }
      const failureReason = err instanceof Error ? err.message : String(err);
      if (isRejectedActionCheatReason(failureReason)) {
        const actorName = playerNameForIndex(multiplayerRef.current.players, message.actorIndex);
        const status = `Cheat detected from ${actorName}: ${failureReason}`;
        emitSyncFailureNotice("Cheat detected", status);
        setStatus(status, true);
        return;
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

  async function buildLocalZiffleRevealTokens(ceremony, cardPositions) {
    const positions = [...new Set((cardPositions || [])
      .map((position) => Number(position))
      .filter((position) => Number.isSafeInteger(position) && position >= 0))];
    if (positions.length === 0) return [];
    const currentGame = gameRef.current;
    if (!currentGame) {
      throw new Error("Ziffle reveal-token backend is not available");
    }
    if (typeof currentGame.ziffleBuildRevealTokens !== "function") {
      return (await Promise.all(
        positions.map((position) => buildLocalZiffleRevealToken(ceremony, position))
      )).map((token, index) => ({
        ...token,
        cardPosition: positions[index],
      }));
    }
    const localIndex = resolveLocalCryptoPlayerIndex();
    const keyContext = ziffleKeyContextForCeremony(ceremony);
    const keyPair = await ensureZiffleIdentity({
      context: keyContext,
      deckCount: ceremony.deckCount,
    });
    return currentGame.ziffleBuildRevealTokens({
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
    const blockedZones = new Set(["library", "outside_game"]);
    for (const object of checkpoint?.objects || []) {
      const hidden = object?.hiddenCard || object?.hidden_card || null;
      if (!hidden || Number(hidden.owner) !== Number(owner)) continue;
      const zone = String(object?.zone || hidden.zone || "");
      if (blockedZones.has(zone)) continue;
      const commitment = String(
        hidden.publicCommitment
        || hidden.public_commitment
        || hidden.commitment
        || ""
      );
      if (deckHash && ziffleDeckHashFromCommitment(commitment) !== String(deckHash)) continue;
      const position = hidden.publicSlot ?? hidden.public_slot ?? hidden.slot;
      if (Number.isSafeInteger(Number(position)) && Number(position) >= 0) {
        positions.add(Number(position));
      }
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
    if (Number(pending.owner) !== Number(owner)) return false;
    if (Number(pending.owner) !== Number(requester)) return false;
    const requested = new Set((positions || []).map((position) => Number(position)));
    const allowed = new Set();
    for (const requirement of pending.requirements || []) {
      if (Number(requirement?.owner) !== Number(owner)) continue;
      const position =
        zifflePositionFromCommitment(requirement.commitment)
        ?? zifflePositionFromCommitment(requirement.positionCommitment)
        ?? Number(requirement.slot);
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
    const committedPosition =
      zifflePositionFromCommitment(requirement?.commitment)
      ?? zifflePositionFromCommitment(requirement?.positionCommitment)
      ?? zifflePositionFromCommitment(requirement?.position_commitment);
    if (committedPosition != null) return committedPosition;
    const explicitPosition = Number(requirement?.position);
    if (Number.isSafeInteger(explicitPosition) && explicitPosition >= 0) return explicitPosition;
    const slot = Number(requirement?.slot);
    return Number.isSafeInteger(slot) && slot >= 0 ? slot : null;
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
        const position = zifflePositionFromRequirement(requirement);
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

  async function ziffleRevealAuthorizedByAction(message, requester, owner, positions, ceremony, debug = null) {
    const auth = message?.actionAuthorization;
    const reject = (reason) => {
      if (debug) debug.reason = reason;
      return false;
    };
    if (!auth || typeof auth !== "object") return reject("missing_action_authorization");
    if (String(auth.matchId || "") !== String(currentAuditMatchId())) return reject("match_id_mismatch");
    if (Number(auth.requesterIndex) !== Number(requester)) return reject("requester_index_mismatch");
    if (Number(requester) !== Number(owner)) return reject("requester_is_not_owner");
    const sequence = Number(auth.seq);
    if (!Number.isSafeInteger(sequence) || sequence <= 0) return reject("invalid_sequence");
    if (!auth.command || typeof auth.command !== "object") return reject("missing_command");
    const currentSequence = Number(multiplayerRef.current.lastAppliedSequence || 0);
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
      return authorized || reject("stored_requirements_do_not_authorize_positions");
    }

    if (sequence !== expectedSeq) return reject("unexpected_sequence");

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
    return authorizedByMulliganShuffle || reject("requirements_do_not_authorize_positions");
  }

  async function answerZiffleRevealTokenRequest(conn, message) {
    let diagnostics = null;
    try {
      const requester = playerIndexForPeerId(conn?.peer);
      const requestedOwner = Number(message.ceremonyOwner);
      if (requester == null || Number(requester) !== requestedOwner) {
        throw new Error("Ziffle reveal tokens can only be requested by the deck owner");
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
        || await waitForZiffleCeremony(message.ceremonyOwner, lookup)
        || attachedCeremony;
      if (!ceremony) {
        const error = new Error(
          `Unknown ziffle ceremony: ${compactZiffleDiagnosticsJson(diagnostics)}`
        );
        error.ziffleDiagnostics = diagnostics;
        throw error;
      }
      const cardPositions = Array.isArray(message.cardPositions) && message.cardPositions.length > 0
        ? message.cardPositions
        : [message.cardPosition];
      const allowedPositions = await waitForAuthorizedZiffleRevealPositions(
        requestedOwner,
        String(ceremony.deckHash || message.deckHash || ""),
        cardPositions
      );
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
      for (const position of cardPositions) {
        if (
          !allowedPositions.has(Number(position))
          && !authorizedByCryptoRequest
          && !authorizedByAction
        ) {
          throw new Error("Ziffle reveal-token request is not authorized by the visible hidden-zone state");
        }
      }
      const tokens = await buildLocalZiffleRevealTokens(ceremony, cardPositions);
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

  function openZiffleRoute(peerId) {
    const target = String(peerId || "").trim();
    const session = multiplayerRef.current;
    if (!target || target === session.localPeerId) return null;
    if (session.role === "host") {
      const conn = clientConnectionsRef.current.get(target);
      return conn && conn.open !== false ? conn : null;
    }
    if (target === session.hostPeerId) {
      const conn = hostConnectionRef.current;
      return conn && conn.open !== false ? conn : null;
    }
    const conn = peerConnectionsRef.current.get(target);
    return conn && conn.open !== false ? conn : null;
  }

  async function waitForZiffleRoute(peerId, timeoutMs = 10000) {
    const started = Date.now();
    while (Date.now() - started < timeoutMs) {
      const conn = openZiffleRoute(peerId);
      if (conn) return conn;
      await sleep(50);
    }
    throw new Error(`No direct ziffle route to peer ${peerId}`);
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
    const players = matchStartPayloadRef.current?.players || session.players || [];
    const positions = [...new Set((cardPositions || [])
      .map((position) => Number(position))
      .filter((position) => Number.isSafeInteger(position) && position >= 0))];
    if (positions.length === 0) return [];
    const actionAuthorization = options.command
      ? {
        matchId: currentAuditMatchId(),
        seq: Number(options.seq ?? Number(session.lastAppliedSequence || 0) + 1),
        requesterIndex: Number(localIndex),
        ...(options.actorIndex !== null && options.actorIndex !== undefined
          ? { actorIndex: Number(options.actorIndex) }
          : {}),
        command: cloneMultiplayerPayload(options.command),
        requirements: cloneMultiplayerPayload(options.requirements || []),
      }
      : null;
    const tokenGroups = await Promise.all((ceremony.keys || []).map(async (key) => {
      if (Number(key.player) === Number(localIndex)) {
        return buildLocalZiffleRevealTokens(ceremony, positions);
      }
      const peer = players.find((player) => Number(player.index) === Number(key.player));
      if (!peer?.peerId) {
        throw new Error(`Missing peer for ziffle reveal token player ${key.player}`);
      }
      setStatus(`Waiting for cryptographic reveal material from ${peer.name || `Player ${Number(key.player) + 1}`}`);
      const conn = await waitForZiffleRoute(peer.peerId);
      const requestId = makeZiffleRequestId("ziffle-reveal");
      const requestDiagnostics = {
        requestId,
        localPeerId: String(session.localPeerId || ""),
        localRole: String(session.role || ""),
        localMode: String(session.mode || ""),
        targetPeerId: String(peer.peerId || ""),
        targetPlayerIndex: Number(key.player),
        ceremony: compactZiffleCeremonyForDiagnostics(ceremony),
        cardPosition: positions.length === 1 ? positions[0] : null,
        cardPositions: positions,
        matchStartPayloadPresent: Boolean(matchStartPayloadRef.current),
        matchStartAuditMatchId: String(matchStartPayloadRef.current?.auditMatchId || ""),
      };
      const waiter = waitForZiffleRevealToken(requestId, 60000, requestDiagnostics);
      safeSend(conn, {
        type: "ziffle_reveal_token_request",
        protocolVersion: PROTOCOL_VERSION,
        requestId,
        ceremonyOwner: Number(ceremony.owner),
        deckHash: String(ceremony.deckHash || ""),
        ceremonyContext: String(ceremony.context || ""),
        ceremony: cloneMultiplayerPayload(ceremony),
        cardPosition: positions[0],
        cardPositions: positions,
        requesterPeerId: session.localPeerId || "",
        requesterIndex: localIndex,
        cryptoMaterialRequestId: options.cryptoMaterialRequestId || "",
        ...(actionAuthorization ? { actionAuthorization } : {}),
      });
      return waiter;
    }));
    return tokenGroups.flatMap((group) => Array.isArray(group) ? group : [group]);
  }

  async function buildLiveZiffleShuffleProof(requirement, seq) {
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.ziffleVerifyShuffle !== "function") {
      throw new Error("Ziffle mental-poker backend is not available");
    }
    const beforeOrder = normalizeShuffleOrder(requirement?.beforeOrder);
    const afterOrder = normalizeShuffleOrder(requirement?.afterOrder);
    if (beforeOrder.length > 0 && afterOrder.length > 0 && beforeOrder.length !== afterOrder.length) {
      throw new Error("Verifiable shuffle order length mismatch");
    }
    const deckCount = Number(afterOrder.length || beforeOrder.length || 0);
    if (deckCount <= 1) return null;
    if (!isSupportedZiffleDeckCount(deckCount)) {
      throw new Error(`Unsupported in-game ziffle library size ${deckCount}`);
    }
    const players = reindexPlayers(matchStartPayloadRef.current?.players || multiplayerRef.current.players || []);
    const keys = zifflePublicKeysForPlayers(players);
    const keyContext = currentAuditMatchId();
    const context = [
      keyContext,
      "action",
      Number(seq),
      "shuffle",
      String(requirement.id || ""),
      Number(requirement.owner),
      String(requirement.zone || "library"),
    ].join(":");
    const steps = [];
    for (const shuffler of players) {
      const request = {
        deckCount,
        context,
        keyContext,
        keys,
        steps: cloneMultiplayerPayload(steps),
        shuffler: Number(shuffler.index || 0),
      };
      let step;
      if (Number(shuffler.index) === Number(resolveLocalPlayerIndex(multiplayerRef.current))) {
        step = await buildLocalZiffleShuffleStep(request);
      } else {
        setStatus(`Waiting for cryptographic shuffle material from ${shuffler.name || `Player ${Number(shuffler.index) + 1}`}`);
        const conn = await waitForZiffleRoute(shuffler.peerId);
        const requestId = makeZiffleRequestId("ziffle-live-shuffle");
        const waiter = waitForZiffleShuffleStep(requestId);
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
    return {
      type: "ziffle_shuffle",
      requirementId: String(requirement.id || ""),
      owner: Number(requirement.owner),
      zone: String(requirement.zone || "library"),
      epoch: Number(seq),
      deckCount,
      context,
      keyContext,
      keys,
      steps,
      deckHash: String(verified.deckHash || ""),
      beforeOrder,
      afterOrder,
    };
  }

  async function buildLocalShuffleProofsForRequirements(cryptoRequirements = [], seq) {
    const requirements = (cryptoRequirements || []).filter(
      (requirement) => String(requirement?.type || "") === "verifiable_shuffle"
    );
    const proofs = [];
    for (const requirement of requirements) {
      const proof = await buildLiveZiffleShuffleProof(requirement, seq);
      if (proof) proofs.push(proof);
    }
    return proofs;
  }

  async function verifyShuffleProofsForRequirements(requirements = [], shuffleProofs = []) {
    const currentGame = gameRef.current;
    for (const requirement of requirements || []) {
      if (String(requirement?.type || "") !== "verifiable_shuffle") continue;
      const proof = (shuffleProofs || []).find((entry) =>
        shuffleProofMatchesRequirement(entry, requirement)
      );
      if (!proof) {
        throw new Error(`Missing verifiable shuffle proof for player ${Number(requirement.owner) + 1}`);
      }
      if (
        Array.isArray(requirement.beforeOrder)
        && !sameShuffleOrder(proof.beforeOrder, requirement.beforeOrder)
      ) {
        throw new Error(`Verifiable shuffle before-order mismatch for player ${Number(requirement.owner) + 1}`);
      }
      if (
        Array.isArray(requirement.afterOrder)
        && !sameShuffleOrder(proof.afterOrder, requirement.afterOrder)
      ) {
        throw new Error(`Verifiable shuffle after-order mismatch for player ${Number(requirement.owner) + 1}`);
      }
      if (!currentGame || typeof currentGame.ziffleVerifyShuffle !== "function") {
        throw new Error("Ziffle mental-poker backend is not available");
      }
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
    }
  }

  async function applyVerifiedShuffleProofs(shuffleProofs = []) {
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.applyVerifiedHiddenLibraryShuffle !== "function") return;
    for (const proof of shuffleProofs || []) {
      if (String(proof?.zone || "library") !== "library") continue;
      const afterOrder = normalizeShuffleOrder(proof.afterOrder);
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
      liveZiffleCeremoniesRef.current.set(Number(proof.owner), cloneMultiplayerPayload(proof));
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

  function playerIndexForPeerId(peerId) {
    const peer = String(peerId || "");
    const players = matchStartPayloadRef.current?.players || multiplayerRef.current.players || [];
    const player = players.find((entry) => String(entry?.peerId || "") === peer);
    return player?.index == null ? null : Number(player.index);
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
    const decision = gameRef.current
      ? (await gameRef.current.uiState())?.decision
      : stateRef.current?.decision;
    if (
      requester == null
      || decision?.player == null
      || Number(decision.player) !== Number(requester)
    ) {
      throw new Error(`${label} was not sent by the active decision player`);
    }
    return {
      matchId: currentAuditMatchId(),
      seq,
      requirementId: String(message.requirementId || ""),
      requesterPeerId: String(conn?.peer || ""),
      requesterPlayer: Number(requester),
    };
  }

  async function answerRngCommitRequest(conn, message) {
    try {
      const request = await validateIncomingRngRequest(conn, message, "Random commitment request");
      const nonceHex = randomAuditHex(32);
      const commitmentHex = await rngCommitmentForNonce(nonceHex);
      rngCommitNoncesRef.current.set(String(message.requestId || ""), {
        ...request,
        nonceHex,
        commitmentHex,
      });
      const localPlayer = resolveLocalPlayerIndex(multiplayerRef.current);
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
          commitmentHex,
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
      const stored = rngCommitNoncesRef.current.get(String(message.commitRequestId || ""));
      if (!stored) {
        throw new Error("Unknown random commitment request");
      }
      const request = await validateIncomingRngRequest(conn, message, "Random reveal request");
      if (
        request.matchId !== stored.matchId
        || Number(request.seq) !== Number(stored.seq)
        || request.requirementId !== stored.requirementId
        || request.requesterPeerId !== stored.requesterPeerId
      ) {
        throw new Error("Random reveal request does not match the commitment request");
      }
      rngCommitNoncesRef.current.delete(String(message.commitRequestId || ""));
      const localPlayer = resolveLocalPlayerIndex(multiplayerRef.current);
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

  async function collectFairRandomReveal(requirement, seq) {
    const players = reindexPlayers(matchStartPayloadRef.current?.players || multiplayerRef.current.players || []);
    const localIndex = resolveLocalCryptoPlayerIndex();
    const commits = [];
    const revealRequests = [];
    for (const player of players) {
      if (Number(player.index) === Number(localIndex)) {
        const nonceHex = randomAuditHex(32);
        const commitmentHex = await rngCommitmentForNonce(nonceHex);
        const commitRequestId = makeZiffleRequestId("rng-local");
        commits.push(await signRngCommitmentEntry({
          matchId: currentAuditMatchId(),
          seq: Number(seq),
          requirementId: String(requirement.id || ""),
          requestId: commitRequestId,
          requester: localIndex,
          player: Number(player.index),
          commitmentHex,
        }));
        revealRequests.push({ player, commitRequestId, local: { nonceHex, commitmentHex } });
      } else {
        const conn = await waitForZiffleRoute(player.peerId);
        const requestId = makeZiffleRequestId("rng-commit");
        const waiter = waitForRngCommit(requestId);
        setStatus(`Waiting for random commitment from ${player.name || `Player ${Number(player.index) + 1}`}`);
        safeSend(conn, {
          type: "rng_commit_request",
          protocolVersion: PROTOCOL_VERSION,
          requestId,
          matchId: currentAuditMatchId(),
          seq: Number(seq),
          requirementId: String(requirement.id || ""),
        });
        const commitment = await waiter;
        if (Number(commitment?.player) !== Number(player.index)) {
          throw new Error("Random commitment response came from the wrong player");
        }
        if (!String(commitment?.commitmentHex || "")) {
          throw new Error("Random commitment response is missing its commitment");
        }
        await verifyRngCommitmentEntry(commitment, {
          matchId: currentAuditMatchId(),
          seq: Number(seq),
          requirementId: String(requirement.id || ""),
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
    const reveals = [];
    for (const request of revealRequests) {
      if (request.local) {
        reveals.push(await signRngRevealEntry({
          matchId: currentAuditMatchId(),
          seq: Number(seq),
          requirementId: String(requirement.id || ""),
          requestId: makeZiffleRequestId("rng-local-reveal"),
          commitRequestId: request.commitRequestId,
          requester: localIndex,
          player: Number(request.player.index),
          nonceHex: request.local.nonceHex,
          commitmentHex: request.local.commitmentHex,
        }));
      } else {
        const conn = await waitForZiffleRoute(request.player.peerId);
        const requestId = makeZiffleRequestId("rng-reveal");
        const waiter = waitForRngReveal(requestId);
        setStatus(`Waiting for random reveal from ${request.player.name || `Player ${Number(request.player.index) + 1}`}`);
        safeSend(conn, {
          type: "rng_reveal_request",
          protocolVersion: PROTOCOL_VERSION,
          requestId,
          commitRequestId: request.commitRequestId,
          matchId: currentAuditMatchId(),
          seq: Number(seq),
          requirementId: String(requirement.id || ""),
        });
        const reveal = await waiter;
        if (Number(reveal?.player) !== Number(request.player.index)) {
          throw new Error("Random reveal response came from the wrong player");
        }
        await verifyRngRevealEntry(reveal, {
          matchId: currentAuditMatchId(),
          seq: Number(seq),
          requirementId: String(requirement.id || ""),
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
    const combinedSeedHex = await sha256Hex(canonicalJson({
      domain: "ironsmith-combined-rng-v1",
      matchId: currentAuditMatchId(),
      seq: Number(seq),
      requirementId: String(requirement.id || ""),
      commits,
      reveals,
    }));
    return {
      type: "commit_reveal_random",
      requirementId: String(requirement.id || ""),
      count: Number(requirement.count || 1),
      commits,
      reveals,
      combinedSeedHex,
    };
  }

  const buildLocalRngRevealsForRequirements = useCallback(async (cryptoRequirements = [], seq = 0) => {
    const fairRandomRequirements = (cryptoRequirements || []).filter(
      (requirement) => String(requirement?.type || "") === "fair_random"
    );
    const reveals = [];
    for (const requirement of fairRandomRequirements) {
      reveals.push(await collectFairRandomReveal(requirement, seq));
    }
    return reveals;
  }, [
    currentAuditMatchId,
    makeZiffleRequestId,
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
    ids.sort((left, right) => left - right);
    return [
      String(matchId || currentAuditMatchId() || ""),
      Number(localIndex),
      ids.join(","),
    ].join("|");
  }

  async function revealLocalZiffleHand(payload = matchStartPayloadRef.current, options = {}) {
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

    if (options.skipIfHandUnchanged) {
      const hintKey = localZiffleHandRevealKey(
        options.stateHint || stateRef.current,
        localIndex,
        payload.auditMatchId,
      );
      if (hintKey && hintKey === ziffleHandRevealKeyRef.current) {
        return;
      }
    }

    const checkpoint = await currentGame.exportSyncCheckpoint();
    const localPlayer = (checkpoint.players || []).find(
      (player) => Number(player.id) === Number(localIndex)
    );
    const checkpointKey = localZiffleHandRevealKey(checkpoint, localIndex, payload.auditMatchId);
    if (
      options.skipIfHandUnchanged
      && checkpointKey
      && checkpointKey === ziffleHandRevealKeyRef.current
    ) {
      return;
    }
    const handIds = new Set((localPlayer?.hand || []).map((id) => Number(id)));
    if (handIds.size === 0) {
      if (checkpointKey) ziffleHandRevealKeyRef.current = checkpointKey;
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
      const publicSlot = hidden?.publicSlot ?? hidden?.public_slot ?? null;
      const publicCommitment = String(hidden?.publicCommitment || hidden?.public_commitment || "");
      const publicZiffleDeckHash = ziffleDeckHashFromCommitment(publicCommitment);
      const knownPosition =
        publicZiffleDeckHash && publicSlot != null
          ? Number(publicSlot)
          : hiddenZiffleDeckHash
            ? Number(hidden?.slot)
            : exportedZiffleDeckHash
              ? Number(exported?.slot)
              : null;
      const knownPositionCommitment =
        publicZiffleDeckHash
          ? publicCommitment
          : hiddenZiffleDeckHash
            ? hiddenCommitment
            : exportedZiffleDeckHash
              ? exportedCommitment
              : "";
      if (exported && !exportedZiffleDeckHash) {
        const cached = localRevealedOpeningForExport(exported);
        if (cached) {
          rememberLocalRevealedOpening(cached, {
            objectId,
            position: cached.position ?? knownPosition,
            positionCommitment: cached.positionCommitment || knownPositionCommitment,
            matchId: payload.auditMatchId,
          });
          continue;
        }
        const { opening } = await buildDeckSlotOpeningForExport({
          manifest,
          preferredSlot: exported.slot,
          card: exported.card,
          exportedCommitment,
          label: "Local hidden card opening",
        });
        rememberLocalRevealedOpening(opening, {
          objectId,
          position: knownPosition,
          positionCommitment: knownPositionCommitment,
          matchId: payload.auditMatchId,
        });
        if (knownPosition != null) {
          rememberZiffleOpeningPosition(localIndex, opening.slot, knownPosition);
        }
        continue;
      }
      if (!knownPositionCommitment) continue;
      const position = Number(knownPosition);
      const positionCommitment = knownPositionCommitment;
      const ceremony = ziffleCeremonyForOwner(localIndex, {
        payload,
        commitment: positionCommitment,
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
      const positions = [...new Set(entries.map((entry) => Number(entry.position)))];
      const tokens = await collectZiffleRevealTokensBatch(ceremony, positions, options);
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
        const originalSlot = revealByPosition.get(position);
        if (!Number.isSafeInteger(originalSlot) || originalSlot < 0) {
          throw new Error(`Missing ziffle reveal for position ${position}`);
        }
        const secret = (manifest.slotSecrets || []).find(
          (candidate) => Number(candidate.slot) === originalSlot
        );
        if (!secret) {
          throw new Error(`Missing private deck opening for ziffle slot ${originalSlot}`);
        }
        const opening = await buildDeckSlotOpening({
          manifest,
          slot: originalSlot,
          card: secret.card,
        });
        const positionCommitment =
          entry.positionCommitment || ziffleRuntimeCommitment(ceremony.deckHash, position);
        await currentGame.revealHiddenPosition({
          owner: Number(localIndex),
          position,
          originalSlot,
          cardName: opening.card,
          positionCommitment,
          commitment: opening.commitment,
        }).catch((err) => {
          if (!entry.exported) throw err;
        });
        rememberLocalRevealedOpening(
          {
            ...opening,
            objectId: entry.objectId,
            position,
            positionCommitment,
          },
          {
            objectId: entry.objectId,
            position,
            positionCommitment,
            matchId: payload.auditMatchId,
          }
        );
        rememberZiffleOpeningPosition(localIndex, originalSlot, position);
        changed = true;
      }
    }
    if (changed) {
      await currentGame.setPerspective(localIndex);
      setState(await currentGame.uiState());
    }
    if (checkpointKey) {
      ziffleHandRevealKeyRef.current = checkpointKey;
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
          const waiter = waitForZiffleShuffleStep(requestId);
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

      const currentSession = multiplayerRef.current;
      const localEntry = payload.players.find(
        (player) => player.peerId === currentSession.localPeerId
      );

      if (!localEntry) {
        throw new Error("Local player is missing from the match payload");
      }
      if (!options.skipGenesisVerification) {
        await verifySignedMatchGenesis(payload);
      }
      await verifyZiffleCeremoniesForPayload(payload);
      matchStartPayloadRef.current = cloneMultiplayerPayload(payload);
      ensureDirectPeerConnections(payload.players || []);
      const localAuditPublicKey = auditPublicKeyRef.current || "";
      const localEncryptionPublicKey = auditEncryptionPublicKeyRef.current || "";
      if (
        localAuditPublicKey
        && String(localEntry.auditPublicKey || "") !== String(localAuditPublicKey)
      ) {
        throw new Error("Match genesis does not bind the local audit key");
      }
      if (
        localEncryptionPublicKey
        && String(localEntry.auditEncryptionPublicKey || "") !== String(localEncryptionPublicKey)
      ) {
        throw new Error("Match genesis does not bind the local private-view encryption key");
      }

      await currentGame.startMatch({
        playerNames: payload.players.map((player) => player.name),
        startingLife: payload.startingLife,
        seed: payload.seed,
        format: payload.format,
        decks: payload.players.map(() => []),
        sideboards: payload.sideboards,
        commanders: payload.commanders,
        hiddenDeckManifests: payload.runtimeHiddenDeckManifests,
        openingHandSize: payload.openingHandSize ?? DEFAULT_OPENING_HAND_SIZE,
      });
      await currentGame.setPerspective(localEntry.index);
      liveZiffleCeremoniesRef.current.clear();
      ziffleOpeningPositionsRef.current.clear();
      localRevealedOpeningsRef.current.clear();
      clearStoredRevealedOpeningsForMatch(payload.auditMatchId || currentAuditMatchId());
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
      auditStateHashRef.current = INITIAL_AUDIT_STATE_HASH;
      const initialPublicCheckpointHash = await publicCheckpointHash(
        await currentGame.exportPublicAuditCheckpoint()
      );
      if (
        payload.initialPublicCheckpointHash
        && String(payload.initialPublicCheckpointHash) !== initialPublicCheckpointHash
      ) {
        throw new Error("Initial public checkpoint does not match signed match genesis");
      }
      initialPublicCheckpointHashRef.current = initialPublicCheckpointHash;
      payload.initialPublicCheckpointHash = initialPublicCheckpointHash;
      matchStartPayloadRef.current = cloneMultiplayerPayload(payload);
      liveAuditTranscriptRef.current = {
        version: 1,
        kind: "ironsmith-live-browser-audit-v1",
        match: cloneMultiplayerPayload(payload),
        matchId: payload.auditMatchId || currentAuditMatchId(),
        lobbyId: payload.lobbyId || payload.hostPeerId || "",
        protocolVersion: PROTOCOL_VERSION,
        signatureAlgorithm: "ecdsa-p256-sha256",
        genesis: cloneMultiplayerPayload(payload.genesis),
        initialStateHash: INITIAL_AUDIT_STATE_HASH,
        initialPublicCheckpointHash,
        actions: [],
      };
      writeStoredPlayerIndex(payload.lobbyId || payload.hostPeerId, localEntry.index);

      updateMultiplayer((prev) => ({
        ...prev,
        role: payload.hostPeerId === localEntry.peerId ? "host" : prev.role,
        mode: "in_match",
        lobbyId: payload.lobbyId || prev.lobbyId,
        hostPeerId: payload.hostPeerId || prev.hostPeerId,
        localPlayerIndex: localEntry.index,
        desiredPlayers: payload.players.length,
        startingLife: payload.startingLife,
        format: normalizeMatchFormat(payload.format),
        localDeckCount:
          payload.deckAuditManifests?.[localEntry.index]?.deckCount
          ?? prev.localDeckCount,
        localCommanderCount:
          payload.commanders?.[localEntry.index]?.length ?? prev.localCommanderCount,
        players: payload.players,
        rematch: null,
        matchStarted: true,
        lastAppliedSequence: 0,
        submittingAction: false,
        matchClock,
        actionTimer: actionTimerSnapshotFromMatchClock(matchClock),
      }));

      if (!options.deferLocalZiffleReveal) {
        await revealLocalZiffleHand(payload);
        await currentGame.setPerspective(localEntry.index);
        setState(await currentGame.uiState());
      }

      setStatus(`Multiplayer match started as ${localEntry.name}`);
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
      const matchPayload = message?.match;
      if (!matchPayload || typeof matchPayload !== "object") {
        throw new Error("Resync payload is missing match state");
      }
      if (!message?.checkpoint || typeof message.checkpoint !== "object") {
        throw new Error("Resync payload is missing WASM checkpoint");
      }

      const currentGame = gameRef.current;
      if (!currentGame || typeof currentGame.importSyncCheckpoint !== "function") {
        throw new Error("Game engine cannot import a resync checkpoint");
      }

      const currentSession = multiplayerRef.current;
      const localEntry = matchPayload.players?.find(
        (player) => player.peerId === currentSession.localPeerId
      );

      if (!localEntry) {
        throw new Error("Local player is missing from the resync payload");
      }

      const actionEntries = Array.isArray(message?.actions)
        ? [...message.actions].sort((left, right) => Number(left?.seq || 0) - Number(right?.seq || 0))
        : [];
      await verifySignedMatchGenesis(matchPayload);
      const verifyTranscriptShuffleProof = async (proof) => {
        if (!currentGame || typeof currentGame.ziffleVerifyShuffle !== "function") {
          throw new Error("Ziffle mental-poker backend is not available");
        }
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
      }, globalThis.crypto, { verifyShuffleProof: verifyTranscriptShuffleProof });
      const payloadHostPeerId = String(matchPayload.hostPeerId || "").trim();
      const expectedHostPeerId = String(currentSession.hostPeerId || "").trim();
      if (payloadHostPeerId && expectedHostPeerId && payloadHostPeerId !== expectedHostPeerId) {
        throw new Error("Resync payload was not sent by the current match host");
      }
      const currentHostPlayer = (matchPayload.players || []).find(
        (player) => String(player?.peerId || "").trim() === (payloadHostPeerId || expectedHostPeerId)
      );
      const expectedHostSeat = normalizePlayerIndex(currentHostPlayer?.index)
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

      const rollbackCheckpoint = typeof currentGame.exportSyncCheckpoint === "function"
        ? await currentGame.exportSyncCheckpoint()
        : null;
      let nextState;
      try {
        nextState = await currentGame.importSyncCheckpoint(
          message.checkpoint,
          localEntry.index,
        );
        if (expectedPublicCheckpointHash) {
          await verifyCurrentPublicCheckpointHash(
            expectedPublicCheckpointHash,
            "Resync checkpoint public state does not match the signed action transcript"
          );
        }
      } catch (err) {
        if (rollbackCheckpoint) {
          try {
            await currentGame.importSyncCheckpoint(
              rollbackCheckpoint,
              currentSession.localPlayerIndex ?? localEntry.index
            );
          } catch {
            // Keep the resync verification error as the actionable failure.
          }
        }
        throw err;
      }
      setState(nextState);
      const matchClock = restoreMatchClockFromTranscript(matchPayload, actionEntries, nextState);

      const lastSequence = Number(
        message?.lastSequence ?? actionEntries.at(-1)?.seq ?? 0
      );
      liveZiffleCeremoniesRef.current.clear();
      ziffleOpeningPositionsRef.current.clear();
      localRevealedOpeningsRef.current.clear();
      clearStoredRevealedOpeningsForMatch(matchPayload.auditMatchId || currentAuditMatchId());
      matchStartPayloadRef.current = cloneMultiplayerPayload(matchPayload);
      actionHistoryRef.current = actionEntries.map((entry) => cloneMultiplayerPayload(entry));
      actionCryptoRequirementsRef.current.clear();
      relayedActionIdsRef.current.clear();
      auditStateHashRef.current =
        actionEntries.at(-1)?.audit?.nextStateHash || INITIAL_AUDIT_STATE_HASH;
      initialPublicCheckpointHashRef.current =
        matchPayload.initialPublicCheckpointHash
        || transcriptReport.initialPublicCheckpointHash
        || initialPublicCheckpointHashRef.current;
      liveAuditTranscriptRef.current = {
        version: 1,
        kind: "ironsmith-live-browser-audit-v1",
        match: cloneMultiplayerPayload(matchPayload),
        matchId: matchPayload.auditMatchId || currentAuditMatchId(),
        lobbyId: matchPayload.lobbyId || matchPayload.hostPeerId || "",
        protocolVersion: PROTOCOL_VERSION,
        signatureAlgorithm: "ecdsa-p256-sha256",
        genesis: cloneMultiplayerPayload(matchPayload.genesis),
        initialStateHash: INITIAL_AUDIT_STATE_HASH,
        initialPublicCheckpointHash: initialPublicCheckpointHashRef.current,
        actions: actionHistoryRef.current.map((entry) => cloneMultiplayerPayload(entry)),
      };
      writeStoredPlayerIndex(matchPayload.lobbyId || matchPayload.hostPeerId, localEntry.index);
      updateMultiplayer((prev) => ({
        ...prev,
        role: matchPayload.hostPeerId === prev.localPeerId ? "host" : prev.role,
        lobbyId: matchPayload.lobbyId || prev.lobbyId,
        hostPeerId: matchPayload.hostPeerId || prev.hostPeerId,
        desiredPlayers: matchPayload.players?.length ?? prev.desiredPlayers,
        startingLife: Number(matchPayload.startingLife || prev.startingLife || 20),
        format: normalizeMatchFormat(matchPayload.format || prev.format),
        localPlayerIndex:
          matchPayload.players?.find((player) => player.peerId === prev.localPeerId)?.index
          ?? prev.localPlayerIndex,
        players: matchPayload.players || prev.players,
        lastAppliedSequence: lastSequence,
        submittingAction: false,
        matchStarted: true,
        mode: "in_match",
        matchClock,
        actionTimer: actionTimerSnapshotFromMatchClock(matchClock),
      }));
      setStatus(
        actionEntries.length > 0
          ? `Resynced with host at action ${lastSequence}`
          : "Resynced with host",
      );
      awaitingStateResyncRef.current = false;
      await revealLocalZiffleHand(matchPayload);

      safeSend(hostConnectionRef.current, {
        type: "resync_ack",
        protocolVersion: PROTOCOL_VERSION,
        lastSequence,
      });
    },
    [
      currentAuditMatchId,
      setState,
      setStatus,
      updateMultiplayer,
      verifyCurrentPublicCheckpointHash,
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
      players: toLobbyPlayers(session.players),
      matchStarted: session.matchStarted,
    });
  }, [broadcastToClients]);

  const startHostedMatch = useCallback(async () => {
    const session = multiplayerRef.current;
    if (!canHostedMatchStart(session)) return;
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.startMatch !== "function") {
      setStatus("Game engine is not ready for multiplayer", true);
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
      lobbyId: session.lobbyId,
      hostPeerId: session.localPeerId,
      players: players.map(toPublicPlayer),
      format,
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
          decks: payload.decks,
          commanders: payload.commanders,
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
      lobbyId: session.lobbyId,
      hostPeerId: session.localPeerId,
      players: players.map((player) => ({
        ...toPublicPlayer(player),
        ready: false,
      })),
      format,
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
          decks: payload.decks,
          commanders: payload.commanders,
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
    const deckAuditManifest = await buildLocalDeckAuditManifest({
      matchId: session.lobbyId || session.hostPeerId || "pending",
      owner: localIndex,
      deck: localDeck,
      sideboard: localSideboard,
      commanders: localCommanders,
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
        deckAuditManifest: publicDeckManifest(deckAuditManifest),
        deckCount: localDeck.length,
        sideboardCount: localSideboard.length,
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
              deck: localDeck,
              sideboard: localSideboard,
              deckAuditManifest: publicDeckManifest(deckAuditManifest),
              deckCount: localDeck.length,
              sideboardCount: localSideboard.length,
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
	    setStatus,
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
    const currentZifflePlayer = normalizePlayerIndex(localEntry?.ziffleKey?.player);
    const currentSigner = normalizePlayerIndex(localEntry?.playerGenesisSignature?.signer);
    if (
      currentManifest?.owner === localPlayerIndex
      && currentManifest?.matchId === matchId
      && currentManifest?.decklistHash === expectedDecklistHash
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
        case "ziffle_reveal_token_request":
          await answerZiffleRevealTokenRequest(hostConnectionRef.current, message);
          return;
        case "ziffle_reveal_token_response":
          resolveZiffleRevealToken(message);
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
          const readyByPeer = new Map(
            (message.rematch?.players || []).map((player) => [
              player.peerId,
              Boolean(player.ready),
            ])
          );
          updateMultiplayer((prev) => {
            if (!prev.rematch) return prev;
            const players = (prev.rematch.players || []).map((player) => ({
              ...player,
              ready: readyByPeer.has(player.peerId)
                ? readyByPeer.get(player.peerId)
                : Boolean(player.ready),
            }));
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
      clientConnectionsRef.current.delete(peerId);
      finishPeerResync(peerId);
      const departed = multiplayerRef.current.players.find(
        (player) => player.peerId === peerId
      );
      const disconnectedAtMs = Date.now();
      updateMultiplayer((prev) => ({
        ...prev,
        players: markPlayerConnectionState(prev.players, peerId, false, disconnectedAtMs),
      }));
      if (departed) {
        setStatus(`${departed.name} disconnected; waiting 60s before auto-forfeit`);
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
      default:
        return;
    }
  }, [
    answerCryptoMaterialRequest,
    answerActionQuorumVoteRequest,
    answerTimeoutVoteRequest,
    answerDisconnectForfeitVoteRequest,
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
		          || message?.type === "action_quorum_vote_request"
		          || message?.type === "action_quorum_vote_response"
		        ) {
          void handlePeerMessage(conn, message).catch((err) => {
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
      const peerId = String(player?.peerId || "").trim();
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
      if (session.role === "host") {
        const conn = clientConnectionsRef.current.get(target);
        if (!conn || conn.open === false) return false;
        safeSend(conn, payload);
        return true;
      }
      if (target === session.hostPeerId) {
        const conn = hostConnectionRef.current;
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
      case "action_quorum_vote_request":
        await answerActionQuorumVoteRequest(conn, message);
        return;
      case "crypto_material_request":
        await answerCryptoMaterialRequest(conn, message);
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
            const expectedAuditKey = String(existingSlot?.auditPublicKey || "").trim();
            const presentedAuditKey = String(message.auditPublicKey || "").trim();
            const expectedEncryptionKey = String(existingSlot?.auditEncryptionPublicKey || "").trim();
            const presentedEncryptionKey = String(message.auditEncryptionPublicKey || "").trim();
            if (
              !expectedAuditKey
              || !presentedAuditKey
              || presentedAuditKey !== expectedAuditKey
              || !expectedEncryptionKey
              || !presentedEncryptionKey
              || presentedEncryptionKey !== expectedEncryptionKey
            ) {
              safeSend(conn, {
                type: "reject",
		                protocolVersion: PROTOCOL_VERSION,
		                reason: "Reconnect audit identity does not match the player slot",
	              });
	              conn.close();
	              return;
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
	                  peerId: conn.peer,
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
                  ziffleKey: message.ziffleKey || basePlayer.ziffleKey || null,
                  deck: [],
                  sideboard: [],
                  commanders: sanitizeCardList(message.commanders),
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

          const existingPlayer = session.players.find((player) => player.peerId === conn.peer);
          if (!existingPlayer) {
            safeSend(conn, {
              type: "reject",
              protocolVersion: PROTOCOL_VERSION,
              reason: "This peer is not part of the active match",
            });
            conn.close();
            return;
          }

          clientConnectionsRef.current.set(conn.peer, conn);
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
                      ziffleKey: message.ziffleKey || player.ziffleKey || null,
                      deck: [],
                      sideboard: [],
                      commanders: sanitizeCardList(message.commanders),
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

          const nextSession = updateMultiplayer((prev) => {
            if (!prev.rematch) return prev;
            const players = reindexPlayers(prev.rematch.players || []).map((player) => (
              player.peerId === conn.peer
                ? {
                    ...player,
                    deck: [],
                    sideboard: [],
                    deckAuditManifest:
                      publicDeckManifest(message.deckAuditManifest)
                      || publicDeckManifest(player.deckAuditManifest),
                    deckCount: Number(message.deckCount || 0),
                    sideboardCount: Number(message.sideboardCount || 0),
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
	          || message?.type === "action_quorum_vote_request"
	          || message?.type === "action_quorum_vote_response"
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
      const fallbackHostIndex = reclaimingOriginalHost ? null : 0;
      const playersWithHostOffline = reclaimingOriginalHost
        ? ensurePromotedLocalPlayer(session.players, session, lobbyId, localPlayerIndex)
        : markHostPeerDisconnected(
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
              lobbyId,
              hostPeerId: peerId,
              players: toPublicPlayers(nextSession.players),
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
      deckText = "",
      commanderText = "",
    }) => {
      teardownPeer();
      const {
        publicKey: auditPublicKey,
        encryptionPublicKey: auditEncryptionPublicKey,
      } = await ensureAuditIdentity();
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
        const deckAuditManifest = await buildLocalDeckAuditManifest({
          matchId: peerId,
          owner: 0,
          deck: currentDeck.deck,
          sideboard: currentDeck.sideboard,
          commanders: currentDeck.commanders,
        });
        const ziffleKeyPair = await ensureZiffleIdentity({
          context: peerId,
          deckCount: currentDeck.deckCount || 60,
        });
        const ziffleKey = publicZiffleKey(ziffleKeyPair, 0);
        const playerGenesisSignature = await signPlayerGenesis({
          matchId: peerId,
          player: {
            peerId,
            name: localName,
            index: 0,
            auditPublicKey,
            auditEncryptionPublicKey,
            deckAuditManifest: publicDeckManifest(deckAuditManifest),
            ziffleKey,
            deckCount: currentDeck.deckCount,
            sideboardCount: currentDeck.sideboard.length,
            commanderCount: currentDeck.commanderCount,
          },
        });
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
      const {
        publicKey: auditPublicKey,
        encryptionPublicKey: auditEncryptionPublicKey,
      } = await ensureAuditIdentity();
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
          startConnectionHeartbeat(heartbeatKey, conn, () => {
            handleHostConnectionLost("Lost heartbeat from lobby host.");
          });
          updateMultiplayer((prev) => ({
            ...prev,
            players: markPlayerConnectionState(prev.players, hostTarget, true),
          }));
	          const session = multiplayerRef.current;
	          const localPeerId = String(session.localPeerId || peer.id || "").trim();
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
          const deckAuditManifest = await buildLocalDeckAuditManifest({
            matchId: targetLobby,
            owner: requestedPlayerIndex ?? 0,
            deck: currentDeck.deck,
            sideboard: currentDeck.sideboard,
            commanders: currentDeck.commanders,
          });
          const ziffleKeyPair = await ensureZiffleIdentity({
            context: targetLobby,
            deckCount: currentDeck.deckCount || 60,
          });
          const zifflePlayerIndex = requestedPlayerIndex ?? 0;
          const ziffleKey = publicZiffleKey(ziffleKeyPair, zifflePlayerIndex);
	          const playerGenesisSignature = await signPlayerGenesis({
	            matchId: targetLobby,
	            player: {
	              peerId: localPeerId,
	              name: localName,
              index: zifflePlayerIndex,
              auditPublicKey,
              auditEncryptionPublicKey,
              deckAuditManifest: publicDeckManifest(deckAuditManifest),
              ziffleKey,
              deckCount: currentDeck.deckCount,
              sideboardCount: currentDeck.sideboard.length,
              commanderCount: currentDeck.commanderCount,
            },
          });
          const joinRequest = {
            type: "join_request",
            protocolVersion: PROTOCOL_VERSION,
            name: localName,
            auditPublicKey,
            auditEncryptionPublicKey,
            playerGenesisSignature,
            deckAuditManifest: publicDeckManifest(deckAuditManifest),
            ziffleKey,
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
	            || message?.type === "action_quorum_vote_request"
	            || message?.type === "action_quorum_vote_response"
	          ) {
            void handleHostMessage(message).catch((err) => {
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
      buildLocalDeckAuditManifest,
      configurePeerConnection,
      ensureAuditIdentity,
      ensureZiffleIdentity,
      emitZiffleDiagnosticNotice,
      handleHostMessage,
      handleConnectionHeartbeatMessage,
      leaveLobby,
      markConnectionAlive,
      peerOptionsRef,
      promoteLocalPlayerToHost,
      publicZiffleKey,
      setStatus,
      signPlayerGenesis,
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
      const {
        publicKey: auditPublicKey,
        encryptionPublicKey: auditEncryptionPublicKey,
      } = await ensureAuditIdentity();
      const localPlayerIndex = resolveLocalPlayerIndex(currentSession) ?? 0;
      const deckAuditManifest = await buildLocalDeckAuditManifest({
        matchId: currentSession.lobbyId || currentSession.hostPeerId || "pending",
        owner: localPlayerIndex,
        deck: deckSubmission.deck,
        sideboard: deckSubmission.sideboard,
        commanders: deckSubmission.commanders,
      });
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

      let stagedMatchClockRuntime = null;
      updateMultiplayer((prev) => ({ ...prev, submittingAction: true }));
      try {
        if (resyncingPeerIdsRef.current.size > 0) {
          setStatus("Waiting for peers to finish resyncing");
          await waitForPeerResyncs();
        }
        const preSubmitState = gameRef.current ? await gameRef.current.uiState() : stateRef.current;
        if (!isDecisionCommandCompatible(preSubmitState?.decision, command)) {
          updateMultiplayer((prev) => ({ ...prev, submittingAction: false }));
          setStatus("That action is no longer available");
          return;
        }
        const expectedActor = preSubmitState?.decision?.player;
        const isTimeoutForfeit = isActionTimeoutForfeitCommand(command);
        const isDisconnectForfeit = isDisconnectTimeoutForfeitCommand(command);
        const isSelfForfeit = isSelfForfeitCommand(command, session.localPlayerIndex);
        if (isSelfForfeit && !isSorcerySpeedForfeitState(preSubmitState, session.localPlayerIndex)) {
          throw new Error("Surrender is only available at sorcery speed");
        }
        if (isForfeitCommand(command) && !isTimeoutForfeit && !isDisconnectForfeit) {
          if (Number(command.player) !== Number(session.localPlayerIndex)) {
            throw new Error("A player can only forfeit themselves");
          }
        }
        if (isTimeoutForfeit) {
          const liveState = gameRef.current ? await gameRef.current.uiState() : stateRef.current;
          updateMatchClockForState(liveState);
          if (!timeoutCertificateFromCommand(command)) {
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
          });
        } else if (isDisconnectForfeit) {
          if (!disconnectCertificateFromCommand(command)) {
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
          });
        } else if (
          expectedActor !== null
          && expectedActor !== undefined
          && Number(expectedActor) !== Number(session.localPlayerIndex)
        ) {
          throw new Error("It is not your turn to act");
        }
        const nextSequence = Number(multiplayerRef.current.lastAppliedSequence || 0) + 1;
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
        });
        stagedMatchClockRuntime = stageLocalMatchClockAudit(clock);
        const cryptoRequirements = filterCryptoRequirementsForCommand(
          command,
          preSubmitState,
          freshCryptoRequirementsForSequence(
            nextSequence,
            await previewRequirementsForCommand(command)
          )
        );
        rememberActionCryptoRequirements(nextSequence, cryptoRequirements);
        const requestRemoteCryptoPreview = shouldRequestRemoteCryptoPreview(
          command,
          preSubmitState,
          cryptoRequirements
        );
        if (cryptoRequirements.length > 0) {
          setStatus("Preparing cryptographic material for action");
        }
        let shuffleProofs = await buildLocalShuffleProofsForRequirements(
          cryptoRequirements,
          nextSequence
        );
        const rngReveals = await buildLocalRngRevealsForRequirements(
          cryptoRequirements,
          nextSequence
        );
        const actionCryptoOptions = {
          command,
          seq: nextSequence,
          actorIndex: session.localPlayerIndex,
          requirements: cryptoRequirements,
        };
        await injectCryptoMaterialForRequirements(cryptoRequirements, {
          shuffleProofs,
          rngReveals,
        }, actionCryptoOptions);
        const remoteCryptoMaterial = await collectRemoteCryptoMaterialForRequirements(
          cryptoRequirements,
          requestRemoteCryptoPreview
            ? { command, seq: nextSequence, actorIndex: session.localPlayerIndex }
            : {}
        );
        await revealAuditOpenings(remoteCryptoMaterial.openings || [], { timing: "pre" });
        const preOpenings = await buildLocalOpeningsForCommand(command, cryptoRequirements, actionCryptoOptions);
        const appliedState = await applySyncedCommand(command, label || "", {
          actorIndex: session.localPlayerIndex,
          sequence: nextSequence,
        });
        await revealAuditOpenings(remoteCryptoMaterial.openings || [], { timing: "post" });
        const appliedRequirements = filterCryptoRequirementsForCommand(
          command,
          preSubmitState,
          freshCryptoRequirementsForSequence(
            nextSequence,
            cryptoRequirementsFromState(appliedState)
          )
        );
        rememberActionCryptoRequirements(nextSequence, appliedRequirements);
        const postShuffleRequirements = missingShuffleRequirements(
          appliedRequirements,
          shuffleProofs
        );
        if (postShuffleRequirements.length > 0) {
          const postShuffleProofs = await buildLocalShuffleProofsForRequirements(
            postShuffleRequirements,
            nextSequence
          );
          shuffleProofs = mergeShuffleProofs(shuffleProofs, postShuffleProofs);
        }
        await verifyShuffleProofsForRequirements(
          [...cryptoRequirements, ...appliedRequirements],
          shuffleProofs
        );
        await applyVerifiedShuffleProofs(shuffleProofs);
        await revealLocalZiffleHand(matchStartPayloadRef.current, {
          skipIfHandUnchanged: true,
          stateHint: appliedState,
          command,
          seq: nextSequence,
          actorIndex: session.localPlayerIndex,
          requirements: [...cryptoRequirements, ...appliedRequirements],
        });
        const openingRequirements = appliedRequirements.length > 0
          ? [...cryptoRequirements, ...appliedRequirements]
          : cryptoRequirements;
        const postOpenings = await buildLocalOpeningsForCommand(command, openingRequirements, {
          ...actionCryptoOptions,
          requirements: openingRequirements,
        });
        const openings = mergeAuditOpenings(
          preOpenings,
          postOpenings,
          remoteCryptoMaterial.openings
        );
        const localPrivateViewProofs = await buildLocalPrivateViewProofsForRequirements(
          appliedRequirements.length > 0 ? appliedRequirements : cryptoRequirements
        );
        const privateViewProofs = mergePrivateViewProofs(
          localPrivateViewProofs,
          remoteCryptoMaterial.privateViewProofs
        );
        const localPublicCheckpointHash = await currentPublicAuditCheckpointHash();
        const audit = await buildSequencedActionAudit({
          seq: nextSequence,
          actorIndex: session.localPlayerIndex,
          command,
          clock,
          openings,
          rngReveals,
          shuffleProofs,
          privateViewProofs,
          publicCheckpointHash: localPublicCheckpointHash,
        });
        const message = {
          type: "apply_action",
          protocolVersion: PROTOCOL_VERSION,
          seq: nextSequence,
          actorIndex: session.localPlayerIndex,
          command,
          label: label || "",
          audit,
        };
        await verifySequencedActionAudit({
          audit,
          seq: nextSequence,
          actorIndex: session.localPlayerIndex,
          command,
        });
        await verifyAuditSatisfiesCryptoRequirements({
          requirements: appliedRequirements.length > 0 ? appliedRequirements : cryptoRequirements,
          audit,
        });
        if (String(audit.publicCheckpointHash || "") !== localPublicCheckpointHash) {
          throw new Error("Local public checkpoint hash does not match signed action");
        }
        const quorumCertificate = await collectActionQuorumCertificate(message);
        if (quorumCertificate) {
          message.audit = {
            ...message.audit,
            quorumCertificate,
          };
          await verifyActionQuorumForMessage(message);
        }
        commitMatchClockAudit(clock, appliedState);
        await appendAppliedSequencedAction(message);
        relaySequencedAction(message);
        setStatus("Action signed and broadcast to peers");
      } catch (err) {
        if (stagedMatchClockRuntime) {
          restoreMatchClockRuntime(stagedMatchClockRuntime, stateRef.current);
        }
        updateMultiplayer((prev) => ({ ...prev, submittingAction: false }));
        const failureReason = toErrorMessage(err);
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
      buildLocalRngRevealsForRequirements,
      collectRemoteCryptoMaterialForRequirements,
      buildSequencedActionAudit,
      currentPublicAuditCheckpointHash,
      revealAuditOpenings,
      verifySequencedActionAudit,
      verifyAuditSatisfiesCryptoRequirements,
      setStatus,
      updateMultiplayer,
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
        return {
          ...prev,
          players: (prev.players || []).map((player) => {
            if (player.connected !== false || !player.disconnectedAtMs) return player;
            const autoForfeitAtMs = Number(player.autoForfeitAtMs || 0)
              || Number(player.disconnectedAtMs) + DISCONNECT_AUTO_FORFEIT_MS;
            return {
              ...player,
              autoForfeitAtMs,
              disconnectRemainingMs: Math.max(0, autoForfeitAtMs - Date.now()),
            };
          }),
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
          `${playerName} was peer-claimed disconnected for 60s`
        );
      } catch (err) {
        disconnectForfeitInFlightRef.current.delete(claimKey);
        const message = toErrorMessage(err);
        if (!message.includes("Disconnect timeout has not elapsed")) {
          setStatus(`Disconnect forfeit failed: ${message}`, true);
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

  const exportAuditTranscript = useCallback(async () => {
    if (!liveAuditTranscriptRef.current) return null;
    const transcript = cloneMultiplayerPayload(liveAuditTranscriptRef.current);
    const currentGame = gameRef.current;
    const finalPublicCheckpoint = currentGame
      && typeof currentGame.exportPublicAuditCheckpoint === "function"
      ? await currentGame.exportPublicAuditCheckpoint()
      : null;
    const actions = Array.isArray(transcript.actions) ? transcript.actions : [];
    const finalPublicCheckpointHash = finalPublicCheckpoint
      ? await publicCheckpointHash(finalPublicCheckpoint)
      : String(actions.at(-1)?.audit?.publicCheckpointHash || transcript.initialPublicCheckpointHash || "");
    const finalStateHash = auditStateHashRef.current || INITIAL_AUDIT_STATE_HASH;
    const disputes = Array.isArray(transcript.disputes) ? transcript.disputes : [];
    return {
      ...transcript,
      exportedAt: new Date().toISOString(),
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
  }, []);

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
  };
}
