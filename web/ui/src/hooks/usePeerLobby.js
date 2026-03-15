import { useCallback, useEffect, useRef, useState } from "react";
import Peer from "peerjs";
import {
  MATCH_FORMAT_COMMANDER,
  MATCH_FORMAT_NORMAL,
  evaluateLobbyDeckSubmission,
  normalizeMatchFormat,
  parseCommanderList,
  parseDeckList,
} from "@/lib/decklists";
import { emitSyncFailureNotice } from "@/lib/ui-notices";

const PROTOCOL_VERSION = 3;
const DEFAULT_OPENING_HAND_SIZE = 7;
const PEER_OPEN_TIMEOUT_MS = 10000;
const PEER_CONNECT_TIMEOUT_MS = 15000;

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
    matchStarted: false,
    lastAppliedSequence: 0,
    submittingAction: false,
  };
}

function sanitizePlayerName(raw, fallback = "Player") {
  const trimmed = String(raw || "").trim();
  return trimmed || fallback;
}

function createMatchSeed() {
  const seed = Math.floor(Math.random() * Number.MAX_SAFE_INTEGER);
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

function safeSend(conn, payload) {
  if (!conn || conn.open === false) return;
  conn.send(payload);
}

function cloneMultiplayerPayload(value) {
  if (value == null) return value;
  return JSON.parse(JSON.stringify(value));
}

function sanitizeCardList(cards) {
  if (!Array.isArray(cards)) return [];
  return cards
    .map((card) => String(card || "").trim())
    .filter(Boolean);
}

function parseDeckSubmission(format, deckText, commanderText = "") {
  const deck = sanitizeCardList(parseDeckList(deckText));
  const commanders = sanitizeCardList(parseCommanderList(commanderText));
  const status = evaluateLobbyDeckSubmission(format, deck, commanders);
  return {
    deck,
    commanders,
    deckCount: status.deckCount,
    commanderCount: status.commanderCount,
    ready: status.ready,
  };
}

function withDeckState(player, format, deck, commanders = []) {
  const normalizedDeck = sanitizeCardList(deck);
  const normalizedCommanders = sanitizeCardList(commanders);
  const status = evaluateLobbyDeckSubmission(
    format,
    normalizedDeck,
    normalizedCommanders
  );
  return {
    ...player,
    deck: normalizedDeck,
    commanders: normalizedCommanders,
    deckCount: status.deckCount,
    commanderCount: status.commanderCount,
    ready: status.ready,
  };
}

function reindexPlayers(players) {
  return players.map((player, index) => ({ ...player, index }));
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

function canHostedMatchStart(session) {
  return (
    session.role === "host" &&
    !session.matchStarted &&
    session.mode !== "starting" &&
    session.players.length === session.desiredPlayers &&
    session.players.length > 0 &&
    session.players.every((player) => player.ready)
  );
}

export function usePeerLobby({ game, setState, setStatus, applySyncedCommand }) {
  const initialPeerOptions = buildPeerOptions();
  const [multiplayer, setMultiplayer] = useState(() => createEmptyState());
  const peerRef = useRef(null);
  const hostConnectionRef = useRef(null);
  const clientConnectionsRef = useRef(new Map());
  const matchStartPayloadRef = useRef(null);
  const actionHistoryRef = useRef([]);
  const gameRef = useRef(game);
  const multiplayerRef = useRef(multiplayer);
  const peerOptionsRef = useRef(initialPeerOptions);
  const peerServerLabelRef = useRef(describePeerServer(initialPeerOptions));

  useEffect(() => {
    gameRef.current = game;
  }, [game]);

  useEffect(() => {
    multiplayerRef.current = multiplayer;
  }, [multiplayer]);

  const updateMultiplayer = useCallback((updater) => {
    const next =
      typeof updater === "function" ? updater(multiplayerRef.current) : updater;
    multiplayerRef.current = next;
    setMultiplayer(next);
    return next;
  }, [setMultiplayer]);

  const teardownPeer = useCallback(() => {
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
  }, []);

  const leaveLobby = useCallback(
    (message = "Left lobby") => {
      teardownPeer();
      matchStartPayloadRef.current = null;
      actionHistoryRef.current = [];
      updateMultiplayer(createEmptyState());
      if (message) {
        setStatus(message);
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
        commanders: payload.commanders,
        openingHandSize: payload.openingHandSize ?? DEFAULT_OPENING_HAND_SIZE,
      });
      await currentGame.setPerspective(localEntry.index);

      const nextState = await currentGame.uiState();
      setState(nextState);
      matchStartPayloadRef.current = cloneMultiplayerPayload(payload);
      actionHistoryRef.current = [];

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
    safeSend(conn, {
      type: "resync_request",
      protocolVersion: PROTOCOL_VERSION,
      lastSequence: session.lastAppliedSequence,
    });
    setStatus(reason, true);
    return true;
  }, [setStatus, updateMultiplayer]);

  const applyStateResync = useCallback(
    async (message) => {
      const matchPayload = message?.match;
      if (!matchPayload || typeof matchPayload !== "object") {
        throw new Error("Resync payload is missing match state");
      }

      const actionEntries = Array.isArray(message?.actions)
        ? [...message.actions].sort((left, right) => Number(left?.seq || 0) - Number(right?.seq || 0))
        : [];

      await applyMatchStart(matchPayload);
      for (const entry of actionEntries) {
        await applySyncedCommand(entry.command, "", {
          actorIndex: entry.actorIndex,
          sequence: entry.seq,
        });
      }

      const lastSequence = Number(
        message?.lastSequence ?? actionEntries.at(-1)?.seq ?? 0
      );
      actionHistoryRef.current = actionEntries.map((entry) => cloneMultiplayerPayload(entry));
      updateMultiplayer((prev) => ({
        ...prev,
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
    },
    [applyMatchStart, applySyncedCommand, setStatus, updateMultiplayer]
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
      players: toPublicPlayers(session.players),
      matchStarted: session.matchStarted,
    });
  }, [broadcastToClients]);

  const startHostedMatch = useCallback(async () => {
    const session = multiplayerRef.current;
    if (!canHostedMatchStart(session)) return;

    const players = reindexPlayers(session.players);

    const decks = players.map((player) => sanitizeCardList(player.deck));
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
      commanders: commanders || undefined,
      startingLife: session.startingLife,
      seed: createMatchSeed(),
      openingHandSize: DEFAULT_OPENING_HAND_SIZE,
    };

    updateMultiplayer((prev) => ({ ...prev, mode: "starting", players }));

    try {
      await applyMatchStart(payload);
      broadcastToClients(payload);
    } catch (err) {
      updateMultiplayer((prev) => ({ ...prev, mode: "lobby" }));
      setStatus(`Match start failed: ${err}`, true);
    }
  }, [applyMatchStart, broadcastToClients, setStatus, updateMultiplayer]);

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
          leaveLobby(message.reason || "Lobby join rejected");
          return;
        case "action_error":
          updateMultiplayer((prev) => ({ ...prev, submittingAction: false }));
          emitSyncFailureNotice(
            "Sync failed",
            message.reason || "Action rejected by multiplayer host"
          );
          setStatus(message.reason || "Action rejected", true);
          return;
        case "match_start":
          await applyMatchStart(message);
          return;
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
          if (nextSequence <= session.lastAppliedSequence) return;
          if (nextSequence !== session.lastAppliedSequence + 1) {
            emitSyncFailureNotice(
              "Sync failed",
              `Action order mismatch. Expected ${session.lastAppliedSequence + 1}, received ${nextSequence}.`
            );
            if (!requestResync("Multiplayer action order mismatch. Resyncing with host...")) {
              setStatus("Multiplayer action order mismatch", true);
            }
            return;
          }

          try {
            await applySyncedCommand(message.command, message.label || "", {
              actorIndex: message.actorIndex,
              sequence: nextSequence,
            });
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
            emitSyncFailureNotice(
              "Sync failed",
              err instanceof Error ? err.message : String(err)
            );
            if (!requestResync("Failed to apply synced action. Resyncing with host...")) {
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
      requestResync,
      setStatus,
      updateMultiplayer,
    ]
  );

  const handleClientDisconnect = useCallback(
    (peerId) => {
      clientConnectionsRef.current.delete(peerId);
      const departed = multiplayerRef.current.players.find(
        (player) => player.peerId === peerId
      );
      updateMultiplayer((prev) => {
        if (prev.matchStarted) {
          return {
            ...prev,
            players: prev.players.map((player) =>
              player.peerId === peerId ? { ...player, connected: false } : player
            ),
          };
        }
        return {
          ...prev,
          players: reindexPlayers(
            prev.players.filter((player) => player.peerId !== peerId)
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
    [broadcastLobbyState, broadcastMatchPresence, setStatus, updateMultiplayer]
  );

  const sequenceHostedAction = useCallback(
    async ({ actorIndex, command, label, senderPeerId = null }) => {
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
    },
    [applySyncedCommand, broadcastToClients, updateMultiplayer]
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
          if (session.matchStarted) {
            safeSend(conn, {
              type: "reject",
              protocolVersion: PROTOCOL_VERSION,
              reason: "Match already started",
            });
            conn.close();
            return;
          }
          if (session.players.length >= session.desiredPlayers) {
            safeSend(conn, {
              type: "reject",
              protocolVersion: PROTOCOL_VERSION,
              reason: "Lobby is full",
            });
            conn.close();
            return;
          }

          clientConnectionsRef.current.set(conn.peer, conn);
          const name = sanitizePlayerName(
            message.name,
            `Player ${session.players.length + 1}`
          );
          updateMultiplayer((prev) => ({
            ...prev,
            mode: "lobby",
            players: reindexPlayers([
              ...prev.players.filter((player) => player.peerId !== conn.peer),
              withDeckState(
                {
                  peerId: conn.peer,
                  name,
                  connected: true,
                },
                prev.format,
                message.deck,
                message.commanders
              ),
            ]),
          }));
          setStatus(`${name} joined lobby`);
          broadcastLobbyState();
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

          safeSend(conn, {
            type: "state_resync",
            protocolVersion: PROTOCOL_VERSION,
            match: matchPayload,
            actions: actionHistoryRef.current.map((entry) => cloneMultiplayerPayload(entry)),
            lastSequence: nextSession.lastAppliedSequence,
          });
          broadcastMatchPresence(conn.peer, true);
          setStatus(`${existingPlayer.name} reconnected`);
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
                      message.commanders
                    )
                  : player
              )
            ),
          }));
          broadcastLobbyState();
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
      sequenceHostedAction,
      setStatus,
      updateMultiplayer,
    ]
  );

  const configureHostConnection = useCallback(
    (conn) => {
      conn.on("data", (message) => {
        void handleClientMessage(conn, message).catch((err) => {
          safeSend(conn, {
            type: "action_error",
            protocolVersion: PROTOCOL_VERSION,
            reason: String(err),
          });
        });
      });
      conn.on("close", () => handleClientDisconnect(conn.peer));
      conn.on("error", () => handleClientDisconnect(conn.peer));
    },
    [handleClientDisconnect, handleClientMessage]
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
      const peer = new Peer(peerOptionsRef.current);
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
              currentDeck.commanders
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
      const peer = new Peer(peerOptionsRef.current);
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
          try {
            currentConn.close();
          } catch (err) {
            void err;
          }
        }

        const conn = peer.connect(targetLobby, {
          reliable: true,
          serialization: "json",
        });
        hostConnectionRef.current = conn;
        const connOpenTimeout = window.setTimeout(() => {
          if (hostConnectionRef.current !== conn || conn.open) return;
          hostConnectionRef.current = null;
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
        conn.on("open", () => {
          if (hostConnectionRef.current !== conn) return;
          clearJoinTimeouts();
          clearHostReconnect();
          const session = multiplayerRef.current;
          if (session.matchStarted) {
            safeSend(conn, {
              type: "resync_request",
              protocolVersion: PROTOCOL_VERSION,
              lastSequence: session.lastAppliedSequence,
            });
            setStatus(`Reconnected to match host ${targetLobby}`);
            return;
          }

          const currentDeck = parseDeckSubmission(
            session.format,
            session.localDeckText,
            session.localCommanderText
          );
          safeSend(conn, {
            type: "join_request",
            protocolVersion: PROTOCOL_VERSION,
            name: localName,
            deck: currentDeck.deck,
            commanders: currentDeck.commanders,
          });
          setStatus(`Joined lobby ${targetLobby}`);
        });
        conn.on("iceStateChanged", (state) => {
          if (hostConnectionRef.current !== conn) return;
          if (state === "checking") {
            setStatus("Negotiating direct peer connection...");
            return;
          }
          if (state === "failed") {
            clearJoinTimeouts();
            hostConnectionRef.current = null;
            scheduleHostReconnect(
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
        void handleHostMessage(message).catch((err) => {
          emitSyncFailureNotice(
            "Sync failed",
            err instanceof Error ? err.message : String(err)
          );
          setStatus(`Lobby message failed: ${err}`, true);
        });
      });
        conn.on("close", () => {
          if (hostConnectionRef.current !== conn) return;
          clearJoinTimeouts();
          hostConnectionRef.current = null;
          scheduleHostReconnect("Disconnected from lobby host.");
        });
        conn.on("error", (err) => {
          if (hostConnectionRef.current !== conn) return;
          clearJoinTimeouts();
          hostConnectionRef.current = null;
          scheduleHostReconnect(formatPeerError(err, "Lobby connection failed"));
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
        setStatus(formatPeerError(err, "Lobby error"), true);
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
    [handleHostMessage, leaveLobby, peerOptionsRef, setStatus, teardownPeer, updateMultiplayer]
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
                        deckSubmission.commanders
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
    submitMultiplayerCommand,
  };
}
