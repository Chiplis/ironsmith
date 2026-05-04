import { useCallback, useEffect, useRef, useState } from "react";
import Peer from "peerjs";
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

const PROTOCOL_VERSION = 6;
const DEFAULT_OPENING_HAND_SIZE = 7;
const PEER_OPEN_TIMEOUT_MS = 10000;
const PEER_CONNECT_TIMEOUT_MS = 15000;
const PEER_HEARTBEAT_INTERVAL_MS = 3000;
const PEER_HEARTBEAT_TIMEOUT_MS = 10000;
const CURRENT_PLAYER_STORAGE_KEY = "currentPlayer";
const CURRENT_LOBBY_STORAGE_KEY = "currentLobby";
const MATCH_SEED_OFFSET = 0xcbf29ce484222325n;
const MATCH_SEED_PRIME = 0x100000001b3n;
const MATCH_SEED_MASK = 0xffffffffffffffffn;
const matchSeedEncoder = new TextEncoder();

function createEmptyState() {
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
    rematch: null,
    matchStarted: false,
    lastAppliedSequence: 0,
    submittingAction: false,
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

function getLocalStorage() {
  if (typeof window === "undefined" || !window.localStorage) return null;
  return window.localStorage;
}

function readStoredPlayerIndex(lobbyId) {
  const storage = getLocalStorage();
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

function resolveReconnectPlayerIndex(session, lobbyId) {
  const localIndex = resolveLocalPlayerIndex(session);
  if (localIndex != null) return localIndex;
  return readStoredPlayerIndex(lobbyId);
}

function writeStoredPlayerIndex(lobbyId, playerIndex) {
  const storage = getLocalStorage();
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
  const storage = getLocalStorage();
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
    connected: player.connected !== false,
    ready: Boolean(player.ready),
    deckCount: Number(player.deckCount || 0),
    commanderCount: Number(player.commanderCount || 0),
  };
}

function toPublicPlayers(players) {
  return reindexPlayers(players).map(toPublicPlayer);
}

function toLobbyPlayer(player) {
  return {
    ...toPublicPlayer(player),
    deck: sanitizeCardList(player.deck),
    sideboard: sanitizeCardList(player.sideboard),
    commanders: sanitizeCardList(player.commanders),
  };
}

function toLobbyPlayers(players) {
  return reindexPlayers(players).map(toLobbyPlayer);
}

function canHostedMatchStart(session) {
  return (
    session.role === "host" &&
    !session.matchStarted &&
    session.mode !== "starting" &&
    session.players.length === session.desiredPlayers &&
    session.players.length > 0 &&
    session.players.every((player) => player.connected !== false && player.ready)
  );
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
  setState,
  setStatus,
  applySyncedCommand,
}) {
  const initialPeerOptions = buildPeerOptions();
  const initialHeartbeatConfig = buildPeerHeartbeatConfig();
  const [multiplayer, setMultiplayer] = useState(() => createEmptyState());
  const peerRef = useRef(null);
  const hostConnectionRef = useRef(null);
  const clientConnectionsRef = useRef(new Map());
  const connectionHeartbeatsRef = useRef(new Map());
  const matchStartPayloadRef = useRef(null);
  const actionHistoryRef = useRef([]);
  const gameRef = useRef(game);
  const multiplayerRef = useRef(multiplayer);
  const peerOptionsRef = useRef(initialPeerOptions);
  const peerHeartbeatConfigRef = useRef(initialHeartbeatConfig);
  const peerServerLabelRef = useRef(describePeerServer(initialPeerOptions));
  const hostMessageQueueRef = useRef(Promise.resolve());
  const clientMessageQueueRef = useRef(Promise.resolve());
  const hostedActionQueueRef = useRef(Promise.resolve());
  const resyncingPeerIdsRef = useRef(new Set());
  const resyncWaitersRef = useRef([]);
  const awaitingStateResyncRef = useRef(false);

  useEffect(() => {
    gameRef.current = game;
  }, [game]);

  useEffect(() => {
    multiplayerRef.current = multiplayer;
  }, [multiplayer]);

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
    if (message?.type === "peer_heartbeat_ack") return true;
    if (message?.type !== "peer_heartbeat") return false;
    safeSend(conn, {
      type: "peer_heartbeat_ack",
      protocolVersion: PROTOCOL_VERSION,
      at: message.at ?? Date.now(),
    });
    return true;
  }, []);

  const updateMultiplayer = useCallback((updater) => {
    const next =
      typeof updater === "function" ? updater(multiplayerRef.current) : updater;
    multiplayerRef.current = next;
    setMultiplayer(next);
    return next;
  }, [setMultiplayer]);

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
      actionHistoryRef.current = [];
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

  const broadcastMatchPresence = useCallback((peerId, connected) => {
    broadcastToClients({
      type: "player_presence",
      protocolVersion: PROTOCOL_VERSION,
      peerId,
      connected: Boolean(connected),
    });
  }, [broadcastToClients]);

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
      const currentGame = gameRef.current;
      if (!currentGame || typeof currentGame.exportSyncCheckpoint !== "function") {
        throw new Error("Game engine cannot export a resync checkpoint");
      }
      const checkpoint = await currentGame.exportSyncCheckpoint();
      safeSend(conn, {
        ...payload,
        checkpoint,
        actions: (payload.actions || actionHistoryRef.current || [])
          .map((entry) => cloneMultiplayerPayload(entry)),
      });
    },
    []
  );

  const applyMatchStart = useCallback(
    async (payload) => {
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

      await currentGame.startMatch({
        playerNames: payload.players.map((player) => player.name),
        startingLife: payload.startingLife,
        seed: payload.seed,
        format: payload.format,
        decks: payload.decks,
        sideboards: payload.sideboards,
        commanders: payload.commanders,
        openingHandSize: payload.openingHandSize ?? DEFAULT_OPENING_HAND_SIZE,
      });
      await currentGame.setPerspective(localEntry.index);

      const nextState = await currentGame.uiState();
      setState(nextState);
      matchStartPayloadRef.current = cloneMultiplayerPayload(payload);
      actionHistoryRef.current = [];
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
          payload.decks?.[localEntry.index]?.length ?? prev.localDeckCount,
        localCommanderCount:
          payload.commanders?.[localEntry.index]?.length ?? prev.localCommanderCount,
        players: payload.players,
        rematch: null,
        matchStarted: true,
        lastAppliedSequence: 0,
        submittingAction: false,
      }));

      setStatus(`Multiplayer match started as ${localEntry.name}`);
    },
    [setState, setStatus, updateMultiplayer]
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

      const nextState = await currentGame.importSyncCheckpoint(
        message.checkpoint,
        localEntry.index,
      );
      setState(nextState);

      const lastSequence = Number(
        message?.lastSequence ?? actionEntries.at(-1)?.seq ?? 0
      );
      matchStartPayloadRef.current = cloneMultiplayerPayload(matchPayload);
      actionHistoryRef.current = actionEntries.map((entry) => cloneMultiplayerPayload(entry));
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
      }));
      setStatus(
        actionEntries.length > 0
          ? `Resynced with host at action ${lastSequence}`
          : "Resynced with host",
      );
      awaitingStateResyncRef.current = false;

      safeSend(hostConnectionRef.current, {
        type: "resync_ack",
        protocolVersion: PROTOCOL_VERSION,
        lastSequence,
      });
    },
    [
      setState,
      setStatus,
      updateMultiplayer,
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

    const players = reindexPlayers(session.players);

    const decks = players.map((player) => sanitizeCardList(player.deck));
    const sideboards = players.map((player) => sanitizeCardList(player.sideboard));
    const format = normalizeMatchFormat(session.format);
    const commanders =
      format === MATCH_FORMAT_COMMANDER
        ? players.map((player) => sanitizeCardList(player.commanders))
        : null;

    const payload = {
      type: "match_start",
      protocolVersion: PROTOCOL_VERSION,
      lobbyId: session.lobbyId,
      hostPeerId: session.localPeerId,
      players: players.map(toPublicPlayer),
      format,
      decks,
      sideboards,
      commanders: commanders || undefined,
      startingLife: session.startingLife,
      openingHandSize: DEFAULT_OPENING_HAND_SIZE,
    };
    payload.seed = createMatchSeed(payload);

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
        });
        if (validation?.valid === false) {
          const summary = summarizeMatchValidationIssues(validation.issues);
          emitSyncFailureNotice("Match start blocked", summary.notice);
          updateMultiplayer((prev) => ({ ...prev, mode: "lobby" }));
          setStatus(summary.status, true);
          return;
        }
      }

      await applyMatchStart(payload);
      broadcastToClients(payload);
    } catch (err) {
      emitSyncFailureNotice(
        "Match start failed",
        err instanceof Error ? err.message : String(err)
      );
      updateMultiplayer((prev) => ({ ...prev, mode: "lobby" }));
      setStatus(`Match start failed: ${err}`, true);
    }
  }, [applyMatchStart, broadcastToClients, setStatus, updateMultiplayer]);

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

    const players = reindexPlayers(rematch?.players || []);
    const format = normalizeMatchFormat(session.format);
    const decks = players.map((player) => sanitizeCardList(player.deck));
    const sideboards = players.map((player) => sanitizeCardList(player.sideboard));
    const commanders =
      format === MATCH_FORMAT_COMMANDER
        ? players.map((player) => sanitizeCardList(player.commanders))
        : null;
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
      decks,
      sideboards,
      commanders: commanders || undefined,
      startingLife: session.startingLife,
      openingHandSize: DEFAULT_OPENING_HAND_SIZE,
    };
    payload.seed = createMatchSeed(payload);

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

      await applyMatchStart(payload);
      broadcastToClients(payload);
    } catch (err) {
      emitSyncFailureNotice(
        "Rematch start failed",
        err instanceof Error ? err.message : String(err)
      );
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
  }, [applyMatchStart, broadcastToClients, setStatus, updateMultiplayer]);

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
        deck: localDeck,
        sideboard: localSideboard,
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
  }, [broadcastRematchState, setStatus, startRematchFromState, updateMultiplayer]);

  const handleHostMessage = useCallback(
    async (message) => {
      if (!message || typeof message !== "object") return;
      if (message.protocolVersion && message.protocolVersion !== PROTOCOL_VERSION) {
        setStatus("Lobby protocol version mismatch", true);
        return;
      }

      switch (message.type) {
        case "lobby_state": {
          updateMultiplayer((prev) => {
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
          updateMultiplayer((prev) => ({
            ...prev,
            players: prev.players.map((player) =>
              player.peerId === message.peerId
                ? { ...player, connected: message.connected !== false }
                : player
            ),
          }));
          return;
        case "apply_action": {
          const nextSequence = Number(message.seq || 0);
          const session = multiplayerRef.current;
          if (awaitingStateResyncRef.current) return;
          if (nextSequence <= session.lastAppliedSequence) return;
          if (nextSequence !== session.lastAppliedSequence + 1) {
            reportSyncFailure(
              `Action order mismatch. Expected ${session.lastAppliedSequence + 1}, received ${nextSequence}.`,
              "Multiplayer action order mismatch. Resyncing with host...",
              "Multiplayer action order mismatch"
            );
            return;
          }

          try {
            await applySyncedCommand(message.command, message.label || "", {
              actorIndex: message.actorIndex,
              sequence: nextSequence,
            });
            actionHistoryRef.current = [
              ...actionHistoryRef.current,
              {
                seq: nextSequence,
                actorIndex: Number(message.actorIndex),
                command: cloneMultiplayerPayload(message.command),
                label: String(message.label || ""),
              },
            ];
            updateMultiplayer((prev) => ({
              ...prev,
              lastAppliedSequence: nextSequence,
              submittingAction: false,
            }));
          } catch (err) {
            updateMultiplayer((prev) => ({
              ...prev,
              submittingAction: false,
            }));
            const resynced = reportSyncFailure(
              err instanceof Error ? err.message : String(err),
              "Failed to apply synced action. Resyncing with host..."
            );
            if (!resynced) {
              throw err;
            }
          }
          return;
        }
        default:
          return;
      }
    },
    [
      applyMatchStart,
      applyStateResync,
      applySyncedCommand,
      leaveLobby,
      reportSyncFailure,
      requestResync,
      setStatus,
      updateMultiplayer,
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
      updateMultiplayer((prev) => {
        return {
          ...prev,
          players: reindexPlayers(
            prev.players.map((player) =>
              player.peerId === peerId ? { ...player, connected: false } : player
            )
          ),
        };
      });
      if (departed) {
        setStatus(`${departed.name} disconnected`);
      }
      if (multiplayerRef.current.matchStarted) {
        broadcastMatchPresence(peerId, false);
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

  const sequenceHostedAction = useCallback(
    ({ actorIndex, command, label, senderPeerId = null }) => enqueueAsync(
      hostedActionQueueRef,
      async () => {
        if (resyncingPeerIdsRef.current.size > 0) {
          setStatus("Waiting for peers to finish resyncing");
          await waitForPeerResyncs();
        }

        const session = multiplayerRef.current;
        const expectedActor = gameRef.current
          ? (await gameRef.current.uiState())?.decision?.player
          : null;
        if (
          expectedActor !== null
          && expectedActor !== undefined
          && Number(expectedActor) !== Number(actorIndex)
        ) {
          if (senderPeerId) {
            const conn = clientConnectionsRef.current.get(senderPeerId);
            safeSend(conn, {
              type: "action_error",
              protocolVersion: PROTOCOL_VERSION,
              reason: "It is not that player's turn to act",
            });
          }
          return;
        }

        const nextSequence = session.lastAppliedSequence + 1;
        try {
          await applySyncedCommand(command, label || "", {
            actorIndex,
            sequence: nextSequence,
          });
          actionHistoryRef.current = [
            ...actionHistoryRef.current,
            {
              seq: nextSequence,
              actorIndex: Number(actorIndex),
              command: cloneMultiplayerPayload(command),
              label: String(label || ""),
            },
          ];
          updateMultiplayer((prev) => ({
            ...prev,
            lastAppliedSequence: nextSequence,
            submittingAction: false,
          }));
          broadcastToClients({
            type: "apply_action",
            protocolVersion: PROTOCOL_VERSION,
            seq: nextSequence,
            actorIndex,
            command,
            label: label || "",
          });
        } catch (err) {
          if (err?.syncedRollbackApplied) {
            const rollbackCommand = { type: "cancel_decision" };
            actionHistoryRef.current = [
              ...actionHistoryRef.current,
              {
                seq: nextSequence,
                actorIndex: Number(actorIndex),
                command: rollbackCommand,
                label: "",
              },
            ];
            updateMultiplayer((prev) => ({
              ...prev,
              lastAppliedSequence: nextSequence,
              submittingAction: false,
            }));
            broadcastToClients({
              type: "apply_action",
              protocolVersion: PROTOCOL_VERSION,
              seq: nextSequence,
              actorIndex,
              command: rollbackCommand,
              label: "",
            });
            if (err && typeof err === "object") {
              err.syncedRollbackBroadcast = true;
              err.syncedRollbackSequence = nextSequence;
            }
          }
          throw err;
        }
      }
    ),
    [
      applySyncedCommand,
      broadcastToClients,
      setStatus,
      updateMultiplayer,
      waitForPeerResyncs,
    ]
  );

  const handleClientMessage = useCallback(
    async (conn, message) => {
      if (!message || typeof message !== "object") return;
      if (message.protocolVersion && message.protocolVersion !== PROTOCOL_VERSION) {
        safeSend(conn, {
          type: "reject",
          protocolVersion: PROTOCOL_VERSION,
          reason: "Protocol version mismatch",
        });
        conn.close();
        return;
      }

      switch (message.type) {
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
            const disconnectedSlot = [...indexedPlayers]
              .sort((left, right) => Number(left.index || 0) - Number(right.index || 0))
              .find((player) => player.connected === false);
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
              : withDeckState(
                  {
                    ...basePlayer,
                    peerId: conn.peer,
                    name,
                    connected: true,
                  },
                  prev.format,
                  message.deck,
                  message.commanders,
                  message.sideboard
                );
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
            players: prev.players.map((player) =>
              player.peerId === conn.peer ? { ...player, connected: true } : player
            ),
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
                  ? withDeckState(
                      player,
                      prev.format,
                      message.deck,
                      message.commanders,
                      message.sideboard
                    )
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
                    deck: sanitizeCardList(message.deck),
                    sideboard: sanitizeCardList(message.sideboard),
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
        case "player_action":
        {
          const session = multiplayerRef.current;
          const actor = session.players.find((player) => player.peerId === conn.peer);
          if (!actor) {
            safeSend(conn, {
              type: "action_error",
              protocolVersion: PROTOCOL_VERSION,
              reason: "This peer is not assigned to an active seat",
            });
            return;
          }

          const claimedActorIndex =
            message.actorIndex === null || message.actorIndex === undefined
              ? null
              : Number(message.actorIndex);
          if (
            claimedActorIndex !== null
            && Number.isFinite(claimedActorIndex)
            && claimedActorIndex !== Number(actor.index)
          ) {
            safeSend(conn, {
              type: "action_error",
              protocolVersion: PROTOCOL_VERSION,
              reason: "Seat mismatch for multiplayer action",
            });
            return;
          }

          await sequenceHostedAction({
            actorIndex: Number(actor.index),
            command: message.command,
            label: message.label || "",
            senderPeerId: conn.peer,
          });
          return;
        }
        default:
          return;
      }
    },
    [
      buildHostedResyncPayload,
      broadcastMatchPresence,
      broadcastLobbyState,
      broadcastRematchState,
      finishPeerResync,
      sendHostedStateMessage,
      sequenceHostedAction,
      setStatus,
      startRematchFromState,
      startRematchSideboarding,
      updateMultiplayer,
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
        // Acks must not wait behind actions that are blocked on those same acks.
        if (message?.type === "resync_ack") {
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
        takeoverPeer.on("connection", configureHostConnection);
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
      configureHostConnection,
      leaveLobby,
      peerOptionsRef,
      setStatus,
      updateMultiplayer,
    ]
  );

  const createLobby = useCallback(
    ({
      name,
      desiredPlayers,
      startingLife,
      format = MATCH_FORMAT_NORMAL,
      deckText = "",
      commanderText = "",
    }) => {
      teardownPeer();
      const localName = sanitizePlayerName(name, "Host");
      const targetPlayers = Math.max(2, Math.min(4, Number(desiredPlayers) || 2));
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

      peer.on("open", (peerId) => {
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
      peer.on("connection", configureHostConnection);
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
    [broadcastLobbyState, configureHostConnection, leaveLobby, peerOptionsRef, setStatus, teardownPeer, updateMultiplayer]
  );

  const joinLobby = useCallback(
    ({ name, lobbyId, deckText = "", commanderText = "" }) => {
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
          serialization: "json",
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
          if (promoteLocalPlayerToHost(reason)) return;
          scheduleHostReconnect(reason);
        };
        conn.on("open", () => {
          if (hostConnectionRef.current !== conn) return;
          clearJoinTimeouts();
          clearHostReconnect();
          startConnectionHeartbeat(heartbeatKey, conn, () => {
            handleHostConnectionLost("Lost heartbeat from lobby host.");
          });
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
          const joinRequest = {
            type: "join_request",
            protocolVersion: PROTOCOL_VERSION,
            name: localName,
            deck: currentDeck.deck,
            sideboard: currentDeck.sideboard,
            commanders: currentDeck.commanders,
          };
          const requestedPlayerIndex = resolveReconnectPlayerIndex(session, targetLobby);
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
    (updates) => {
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
                        player,
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
          deck: deckSubmission.deck,
          sideboard: deckSubmission.sideboard,
          commanders: deckSubmission.commanders,
        });
      }
    },
    [broadcastLobbyState, updateMultiplayer]
  );

  const submitMultiplayerCommand = useCallback(
    async (command, label = "") => {
      const session = multiplayerRef.current;
      if (!session.matchStarted) {
        setStatus("Match has not started yet", true);
        return;
      }
      if (session.submittingAction) {
        setStatus("Waiting for the previous action to sync");
        return;
      }
      if (session.localPlayerIndex == null) {
        setStatus("Local player seat is not assigned", true);
        return;
      }

      if (session.role === "host") {
        updateMultiplayer((prev) => ({ ...prev, submittingAction: true }));
        try {
          await sequenceHostedAction({
            actorIndex: session.localPlayerIndex,
            command,
            label,
          });
        } catch (err) {
          updateMultiplayer((prev) => ({ ...prev, submittingAction: false }));
          throw err;
        }
        return;
      }

      const conn = hostConnectionRef.current;
      if (!conn || conn.open === false) {
        setStatus("Host connection is not available", true);
        return;
      }

      updateMultiplayer((prev) => ({ ...prev, submittingAction: true }));
      safeSend(conn, {
        type: "player_action",
        protocolVersion: PROTOCOL_VERSION,
        actorIndex: session.localPlayerIndex,
        command,
        label,
      });
      setStatus("Waiting for host to sync action");
    },
    [sequenceHostedAction, setStatus, updateMultiplayer]
  );

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
  };
}
